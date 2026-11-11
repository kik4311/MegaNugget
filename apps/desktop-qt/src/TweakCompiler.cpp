#include "TweakCompiler.h"

#include <QSet>
#include <QTextStream>
#include <cstring>

namespace {

struct Drafted {
  const TweakSpec* spec = nullptr;
  bool on = false;
  std::optional<TweakValue> stored;
};

QString escapeXml(const QString& value) {
  QString out;
  out.reserve(value.size());
  for (const QChar ch : value) {
    switch (ch.unicode()) {
      case '&':
        out += QStringLiteral("&amp;");
        break;
      case '<':
        out += QStringLiteral("&lt;");
        break;
      case '>':
        out += QStringLiteral("&gt;");
        break;
      case '"':
        out += QStringLiteral("&quot;");
        break;
      default:
        out += ch;
        break;
    }
  }
  return out;
}

QString valueXml(const TweakValue& value) {
  switch (value.type) {
    case TweakValue::Type::Bool:
      return value.boolValue ? QStringLiteral("<true/>") : QStringLiteral("<false/>");
    case TweakValue::Type::Integer:
      return QStringLiteral("<integer>%1</integer>").arg(value.intValue);
    case TweakValue::Type::Double:
      return QStringLiteral("<real>%1</real>").arg(value.doubleValue, 0, 'g', 17);
    case TweakValue::Type::Text:
      return QStringLiteral("<string>%1</string>").arg(escapeXml(value.textValue));
  }
  return QStringLiteral("<string></string>");
}

QString serializeDict(const QList<QPair<QString, TweakValue>>& entries) {
  QString xml;
  QTextStream stream(&xml);
  stream << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
         << "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" "
         << "\"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n"
         << "<plist version=\"1.0\">\n<dict>\n";
  for (const auto& entry : entries) {
    stream << "\t<key>" << escapeXml(entry.first) << "</key>\n\t" << valueXml(entry.second)
           << "\n";
  }
  stream << "</dict>\n</plist>\n";
  return xml;
}

using Table = QList<QPair<QString, TweakValue>>;

Table* location(const QString& path, QList<QPair<QString, Table>>& tables) {
  for (auto& table : tables) {
    if (table.first == path) {
      return &table.second;
    }
  }
  tables.push_back({path, {}});
  return &tables.last().second;
}

void upsert(Table* table, const QString& key, const TweakValue& value) {
  for (auto& entry : *table) {
    if (entry.first == key) {
      entry.second = value;
      return;
    }
  }
  table->push_back({key, value});
}

}  // namespace

QString TweakCompiler::errorCode(DraftError error) {
  switch (error) {
    case DraftError::EmptySelection:
      return QStringLiteral("tweaks_empty_selection");
    case DraftError::TooManySelections:
      return QStringLiteral("tweaks_too_many_selections");
    case DraftError::UnknownTweak:
      return QStringLiteral("tweaks_unknown_tweak");
    case DraftError::BlockedTweak:
      return QStringLiteral("tweaks_blocked_tweak");
    case DraftError::InvalidValue:
      return QStringLiteral("tweaks_invalid_value");
    case DraftError::Conflict:
      return QStringLiteral("tweaks_conflict");
    case DraftError::SerializationFailed:
      return QStringLiteral("tweaks_compile_failed");
  }
  return QStringLiteral("tweaks_compile_failed");
}

std::optional<std::pair<QString, QString>> TweakCompiler::splitDomain(const QString& path) {
  struct Rule {
    const char* prefix;
    const char* domain;
    bool container;
  };
  static const Rule rules[] = {
      {"/var/Managed Preferences/", "ManagedPreferencesDomain", false},
      {"/var/root/", "RootDomain", false},
      {"/var/preferences/", "SystemPreferencesDomain", false},
      {"/var/MobileDevice/", "MobileDeviceDomain", false},
      {"/var/mobile/", "HomeDomain", false},
      {"/var/db/", "DatabaseDomain", false},
      {"/var/containers/Shared/SystemGroup/", "SysSharedContainerDomain-", true},
      {"/var/containers/Data/SystemGroup/", "SysContainerDomain-", true},
  };
  for (const auto& rule : rules) {
    if (!path.startsWith(QLatin1String(rule.prefix))) {
      continue;
    }
    const QString rest = path.mid(static_cast<int>(std::strlen(rule.prefix)));
    if (!rule.container) {
      return std::make_pair(QString::fromLatin1(rule.domain), rest);
    }
    const int slash = rest.indexOf(QLatin1Char('/'));
    if (slash < 0) {
      return std::make_pair(QString::fromLatin1(rule.domain) + rest, QString());
    }
    return std::make_pair(QString::fromLatin1(rule.domain) + rest.left(slash),
                          rest.mid(slash + 1));
  }
  return std::nullopt;
}

std::optional<DraftPreview> TweakCompiler::preview(const QMap<QString, TweakValue>& choices,
                                                   const TweakDevice& device, DraftError* error,
                                                   QString* conflictDetail) {
  auto fail = [&](DraftError code) -> std::optional<DraftPreview> {
    if (error) {
      *error = code;
    }
    return std::nullopt;
  };

  if (choices.isEmpty()) {
    return fail(DraftError::EmptySelection);
  }
  if (choices.size() > TweakCatalog::kPreviewLimit) {
    return fail(DraftError::TooManySelections);
  }

  auto& catalog = TweakCatalog::instance();
  for (auto it = choices.begin(); it != choices.end(); ++it) {
    if (!catalog.spec(it.key())) {
      return fail(DraftError::UnknownTweak);
    }
  }

  QVector<Drafted> drafted;
  QMap<QString, TweakValue> writes;
  QSet<QString> staged;

  for (const auto& spec : catalog.specs()) {
    if (!choices.contains(spec.id)) {
      continue;
    }
    const TweakValue value = choices.value(spec.id);
    if (TweakCatalog::status(spec, device) != TweakStatus::DraftCandidate ||
        TweakCatalog::deviceStatus(device)) {
      return fail(DraftError::BlockedTweak);
    }
    if (spec.writesWholeDict() && !(value == spec.value)) {
      return fail(DraftError::InvalidValue);
    }
    if (value.type == TweakValue::Type::Bool && value.boolValue && spec.exclusiveWith) {
      const auto peer = choices.value(*spec.exclusiveWith);
      if (peer.type == TweakValue::Type::Bool && peer.boolValue) {
        if (conflictDetail) {
          *conflictDetail = spec.id + QStringLiteral(" / ") + *spec.exclusiveWith;
        }
        return fail(DraftError::Conflict);
      }
    }

    switch (spec.kind) {
      case TweakKind::Toggle: {
        if (value.type != TweakValue::Type::Bool) {
          return fail(DraftError::InvalidValue);
        }
        if (spec.offValue) {
          if (value.boolValue) {
            writes.insert(spec.id, spec.value);
            staged.insert(spec.id);
          } else {
            writes.insert(spec.id, *spec.offValue);
            staged.remove(spec.id);
          }
        } else if (spec.reversibleBoolean) {
          if (value.boolValue) {
            staged.insert(spec.id);
          } else {
            staged.remove(spec.id);
          }
        } else {
          writes.insert(spec.id, value);
          staged.insert(spec.id);
        }
        break;
      }
      case TweakKind::Text: {
        if (value.type != TweakValue::Type::Text ||
            value.textValue.toUtf8().size() > TweakCatalog::kMaxTextBytes ||
            value.textValue.contains(QChar(0)) || spec.writesWholeDict()) {
          return fail(DraftError::InvalidValue);
        }
        writes.insert(spec.id, value);
        staged.insert(spec.id);
        break;
      }
      case TweakKind::Number: {
        if (value.type == TweakValue::Type::Integer) {
          if (spec.value.type != TweakValue::Type::Integer ||
              static_cast<double>(value.intValue) < spec.minValue ||
              static_cast<double>(value.intValue) > spec.maxValue) {
            return fail(DraftError::InvalidValue);
          }
        } else if (value.type == TweakValue::Type::Double) {
          if (spec.value.type != TweakValue::Type::Double || !qIsFinite(value.doubleValue) ||
              value.doubleValue < spec.minValue || value.doubleValue > spec.maxValue) {
            return fail(DraftError::InvalidValue);
          }
        } else {
          return fail(DraftError::InvalidValue);
        }
        writes.insert(spec.id, value);
        staged.insert(spec.id);
        break;
      }
    }

    drafted.push_back(Drafted{&spec, staged.contains(spec.id),
                              writes.contains(spec.id) ? std::optional(writes.value(spec.id))
                                                       : std::nullopt});
  }

  if (choices.contains(QStringLiteral("RTL")) && choices.contains(QStringLiteral("LTR"))) {
    const auto rtl = choices.value(QStringLiteral("RTL"));
    const auto ltr = choices.value(QStringLiteral("LTR"));
    if (rtl.type == TweakValue::Type::Bool && rtl.boolValue &&
        ltr.type == TweakValue::Type::Bool && ltr.boolValue) {
      if (conflictDetail) {
        *conflictDetail = QStringLiteral("RTL/LTR");
      }
      return fail(DraftError::Conflict);
    }
  }

  QList<QPair<QString, Table>> tables;
  for (const auto& entry : drafted) {
    const TweakSpec* spec = entry.spec;
    if (spec->writesWholeDict()) {
      Table* dict = location(spec->path, tables);
      for (auto it = spec->multiValues.begin(); it != spec->multiValues.end(); ++it) {
        upsert(dict, it.key(), it.value());
      }
      continue;
    }
    if (spec->key.isEmpty()) {
      if (error) {
        *error = DraftError::SerializationFailed;
      }
      return std::nullopt;
    }
    Table* dict = location(spec->path, tables);
    const TweakValue chosen = writes.value(spec->id, spec->value);
    if (spec->offValue) {
      upsert(dict, spec->key, entry.on ? spec->value : *spec->offValue);
    } else if (spec->reversibleBoolean) {
      TweakValue encoded;
      if (spec->integerBoolean) {
        encoded.type = TweakValue::Type::Integer;
        encoded.intValue = entry.on ? 1 : 0;
      } else {
        encoded.type = TweakValue::Type::Bool;
        encoded.boolValue = entry.on;
      }
      upsert(dict, spec->key, encoded);
    } else {
      upsert(dict, spec->key, chosen);
    }
  }

  DraftPreview preview;
  preview.selectedCount = drafted.size();
  preview.appliedToDevice = false;
  for (const auto& table : tables) {
    const auto mapped = splitDomain(table.first);
    if (!mapped || table.second.isEmpty()) {
      return fail(DraftError::SerializationFailed);
    }
    preview.files.push_back(PlistFileDraft{mapped->first, mapped->second,
                                           serializeDict(table.second)});
  }
  if (preview.files.isEmpty()) {
    return fail(DraftError::SerializationFailed);
  }
  return preview;
}
