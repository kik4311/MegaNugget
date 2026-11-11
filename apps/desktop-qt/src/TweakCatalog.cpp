#include "TweakCatalog.h"

#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSet>

namespace {

const char* kPortedFactories[] = {
    "BasicPlistTweak",
    "AdvancedPlistTweak",
    "ReversibleBooleanPlistTweak",
};

const char* kOs15Only[] = {
    "iPhone8,1", "iPhone8,2", "iPhone8,4", "iPhone9,1", "iPhone9,2", "iPhone9,3", "iPhone9,4",
};
const char* kThroughOs16[] = {
    "iPhone10,1", "iPhone10,2", "iPhone10,3", "iPhone10,4", "iPhone10,5", "iPhone10,6",
};
const char* kFromOs15[] = {
    "iPhone11,2", "iPhone11,4", "iPhone11,6", "iPhone11,8", "iPhone12,1", "iPhone12,3",
    "iPhone12,5", "iPhone12,8", "iPhone13,1", "iPhone13,2", "iPhone13,3", "iPhone13,4",
    "iPhone14,2", "iPhone14,3", "iPhone14,4", "iPhone14,5", "iPhone14,6",
};
const char* kFromOs16[] = {"iPhone14,7", "iPhone14,8", "iPhone15,2", "iPhone15,3"};
const char* kFromOs17[] = {"iPhone15,4", "iPhone15,5", "iPhone16,1", "iPhone16,2"};

bool inList(const char* const* items, int count, const QString& model) {
  for (int i = 0; i < count; ++i) {
    if (model == QLatin1String(items[i])) {
      return true;
    }
  }
  return false;
}

bool hasDynamicIsland(const QString& model) {
  static const char* models[] = {
      "iPhone15,2", "iPhone15,3", "iPhone15,4", "iPhone15,5", "iPhone16,1", "iPhone16,2",
      "iPhone17,1", "iPhone17,2", "iPhone17,3", "iPhone17,4", "iPhone18,1", "iPhone18,2",
      "iPhone18,3", "iPhone18,4"};
  return inList(models, std::size(models), model.trimmed());
}

bool hasCameraControl(const QString& model) {
  static const char* models[] = {
      "iPhone17,1", "iPhone17,2", "iPhone17,3", "iPhone17,4", "iPhone18,1", "iPhone18,2",
      "iPhone18,3", "iPhone18,4", "iPhone19,2", "iPhone19,3", "iPhone19,7"};
  return inList(models, std::size(models), model.trimmed());
}

bool hasActionButton(const QString& model) {
  static const char* models[] = {
      "iPhone16,1", "iPhone16,2", "iPhone17,1", "iPhone17,2", "iPhone17,3", "iPhone17,4",
      "iPhone17,5", "iPhone18,1", "iPhone18,2", "iPhone18,3", "iPhone18,4", "iPhone18,5",
      "iPhone19,2", "iPhone19,3", "iPhone19,7"};
  return inList(models, std::size(models), model.trimmed());
}

bool hasHomeButton(const QString& model) {
  static const char* models[] = {
      "iPad11,1",  "iPad11,2",  "iPad11,3",  "iPad11,4",  "iPad11,6",  "iPad11,7",  "iPad12,1",
      "iPad12,2",  "iPad5,1",   "iPad5,2",   "iPad5,3",   "iPad5,4",   "iPad6,11",  "iPad6,12",
      "iPad6,3",   "iPad6,4",   "iPad6,7",   "iPad6,8",   "iPad7,1",   "iPad7,11",  "iPad7,12",
      "iPad7,2",   "iPad7,3",   "iPad7,4",   "iPad7,5",   "iPad7,6",   "iPhone10,1", "iPhone10,2",
      "iPhone10,4","iPhone10,5","iPhone12,8","iPhone14,6","iPhone8,1", "iPhone8,2", "iPhone8,4",
      "iPhone9,1", "iPhone9,2", "iPhone9,3", "iPhone9,4"};
  return inList(models, std::size(models), model.trimmed());
}

bool hasAlwaysOn(const QString& model) {
  static const char* models[] = {
      "iPhone15,2", "iPhone15,3", "iPhone16,1", "iPhone16,2", "iPhone17,1", "iPhone17,2",
      "iPhone18,1", "iPhone18,2", "iPhone18,3", "iPhone18,4", "iPhone19,2", "iPhone19,3",
      "iPhone19,7"};
  return inList(models, std::size(models), model.trimmed());
}

std::optional<TweakValue> parseValue(const QJsonObject& object) {
  const QString type = object.value(QStringLiteral("type")).toString();
  const QJsonValue value = object.value(QStringLiteral("value"));
  TweakValue out;
  if (type == QLatin1String("bool")) {
    out.type = TweakValue::Type::Bool;
    out.boolValue = value.toBool();
    return out;
  }
  if (type == QLatin1String("int")) {
    out.type = TweakValue::Type::Integer;
    out.intValue = static_cast<qint64>(value.toDouble());
    return out;
  }
  if (type == QLatin1String("double")) {
    out.type = TweakValue::Type::Double;
    out.doubleValue = value.toDouble();
    return out;
  }
  if (type == QLatin1String("string")) {
    out.type = TweakValue::Type::Text;
    out.textValue = value.toString();
    return out;
  }
  return std::nullopt;
}

TweakSection parseSection(const QString& value) {
  if (value == QLatin1String("liquidGlass")) {
    return TweakSection::LiquidGlass;
  }
  if (value == QLatin1String("springboard")) {
    return TweakSection::Springboard;
  }
  if (value == QLatin1String("daemons")) {
    return TweakSection::Daemons;
  }
  return TweakSection::InternalOptions;
}

TweakKind parseKind(const QString& value) {
  if (value == QLatin1String("text")) {
    return TweakKind::Text;
  }
  if (value == QLatin1String("number")) {
    return TweakKind::Number;
  }
  return TweakKind::Toggle;
}

}  // namespace

QString TweakValue::display() const {
  switch (type) {
    case Type::Bool:
      return boolValue ? QStringLiteral("true") : QStringLiteral("false");
    case Type::Integer:
      return QString::number(intValue);
    case Type::Double:
      return QString::number(doubleValue, 'g', 17);
    case Type::Text:
      return textValue;
  }
  return {};
}

bool TweakValue::operator==(const TweakValue& other) const {
  if (type != other.type) {
    return false;
  }
  switch (type) {
    case Type::Bool:
      return boolValue == other.boolValue;
    case Type::Integer:
      return intValue == other.intValue;
    case Type::Double:
      return doubleValue == other.doubleValue;
    case Type::Text:
      return textValue == other.textValue;
  }
  return false;
}

bool TweakSpec::isCompatible(const TweakDevice& device) const {
  if (!TweakCatalog::compareVersion(device.systemVersion, device.systemVersion)) {
    return false;
  }
  if (disabled || unavailableReason) {
    return false;
  }
  if (minVersion) {
    const auto cmp = TweakCatalog::compareVersion(device.systemVersion, *minVersion);
    if (!cmp || *cmp < 0) {
      return false;
    }
  }
  if (maxVersion) {
    const auto cmp = TweakCatalog::compareVersion(device.systemVersion, *maxVersion);
    if (!cmp || *cmp > 0) {
      return false;
    }
  }
  if (factory == QLatin1String("SiriAITweak") || ipadOnly) {
    return false;
  }
  if (!supportedModels.isEmpty() && !supportedModels.contains(device.model)) {
    return false;
  }
  if (requiresDynamicIsland && !hasDynamicIsland(device.model)) {
    return false;
  }
  if (requiresCameraControl && !hasCameraControl(device.model)) {
    return false;
  }
  if (requiresActionButton && !hasActionButton(device.model)) {
    return false;
  }
  if (requiresHomeButton && !hasHomeButton(device.model)) {
    return false;
  }
  if (requiresAlwaysOn && !hasAlwaysOn(device.model)) {
    return false;
  }
  return true;
}

TweakCatalog& TweakCatalog::instance() {
  static TweakCatalog catalog;
  return catalog;
}

bool TweakCatalog::load(QString* error) {
  if (loaded_) {
    return true;
  }
  QFile file(QStringLiteral(":/catalog.json"));
  if (!file.open(QIODevice::ReadOnly)) {
    if (error) {
      *error = QStringLiteral("catalog_missing");
    }
    return false;
  }
  const auto doc = QJsonDocument::fromJson(file.readAll());
  if (!doc.isObject()) {
    if (error) {
      *error = QStringLiteral("catalog_invalid");
    }
    return false;
  }
  const QJsonObject root = doc.object();
  repository_ = root.value(QStringLiteral("repository")).toString();
  revision_ = root.value(QStringLiteral("revision")).toString();
  if (revision_ != QLatin1String("e4be86f050f4980ccc58de3b74f9de58b070b705")) {
    if (error) {
      *error = QStringLiteral("catalog_revision_mismatch");
    }
    return false;
  }
  const QJsonArray specs = root.value(QStringLiteral("specs")).toArray();
  if (specs.size() != kRegistryCount) {
    if (error) {
      *error = QStringLiteral("catalog_count_mismatch");
    }
    return false;
  }
  QSet<QString> seen;
  QVector<TweakSpec> parsed;
  parsed.reserve(specs.size());
  for (const QJsonValue& item : specs) {
    const QJsonObject object = item.toObject();
    TweakSpec spec;
    spec.id = object.value(QStringLiteral("id")).toString();
    if (spec.id.isEmpty() || seen.contains(spec.id)) {
      if (error) {
        *error = QStringLiteral("catalog_duplicate_id");
      }
      return false;
    }
    seen.insert(spec.id);
    spec.section = parseSection(object.value(QStringLiteral("section")).toString());
    spec.title = object.value(QStringLiteral("title")).toString();
    spec.path = object.value(QStringLiteral("path")).toString();
    spec.key = object.value(QStringLiteral("key")).toString();
    const auto value = parseValue(object.value(QStringLiteral("value")).toObject());
    if (!value) {
      if (error) {
        *error = QStringLiteral("catalog_invalid_value");
      }
      return false;
    }
    spec.value = *value;
    spec.kind = parseKind(object.value(QStringLiteral("kind")).toString());
    spec.minValue = object.value(QStringLiteral("minValue")).toDouble();
    spec.maxValue = object.value(QStringLiteral("maxValue")).toDouble();
    spec.step = object.value(QStringLiteral("step")).toDouble();
    if (!object.value(QStringLiteral("minVersion")).isNull()) {
      spec.minVersion = object.value(QStringLiteral("minVersion")).toString();
    }
    if (!object.value(QStringLiteral("maxVersion")).isNull()) {
      spec.maxVersion = object.value(QStringLiteral("maxVersion")).toString();
    }
    spec.iphoneOnly = object.value(QStringLiteral("iphoneOnly")).toBool();
    spec.ipadOnly = object.value(QStringLiteral("ipadOnly")).toBool();
    spec.disabled = object.value(QStringLiteral("disabled")).toBool();
    if (!object.value(QStringLiteral("detail")).isNull()) {
      spec.detail = object.value(QStringLiteral("detail")).toString();
    }
    if (object.value(QStringLiteral("multiValues")).isObject()) {
      const QJsonObject multi = object.value(QStringLiteral("multiValues")).toObject();
      for (auto it = multi.begin(); it != multi.end(); ++it) {
        const auto parsed = parseValue(it.value().toObject());
        if (!parsed) {
          if (error) {
            *error = QStringLiteral("catalog_invalid_multivalue");
          }
          return false;
        }
        spec.multiValues.insert(it.key(), *parsed);
      }
    }
    spec.reversibleBoolean = object.value(QStringLiteral("reversibleBoolean")).toBool();
    spec.requiresDynamicIsland = object.value(QStringLiteral("requiresDynamicIsland")).toBool();
    spec.factory = object.value(QStringLiteral("factory")).toString();
    if (object.value(QStringLiteral("offValue")).isObject()) {
      spec.offValue = parseValue(object.value(QStringLiteral("offValue")).toObject());
    }
    spec.integerBoolean = object.value(QStringLiteral("integerBoolean")).toBool();
    spec.requiresCameraControl = object.value(QStringLiteral("requiresCameraControl")).toBool();
    spec.requiresActionButton = object.value(QStringLiteral("requiresActionButton")).toBool();
    spec.requiresHomeButton = object.value(QStringLiteral("requiresHomeButton")).toBool();
    spec.requiresAlwaysOn = object.value(QStringLiteral("requiresAlwaysOn")).toBool();
    if (!object.value(QStringLiteral("warning")).isNull()) {
      spec.warning = object.value(QStringLiteral("warning")).toString();
    }
    for (const QJsonValue& req : object.value(QStringLiteral("requires")).toArray()) {
      spec.requiredIds.push_back(req.toString());
    }
    if (!object.value(QStringLiteral("exclusiveWith")).isNull()) {
      spec.exclusiveWith = object.value(QStringLiteral("exclusiveWith")).toString();
    }
    if (!object.value(QStringLiteral("unavailableReason")).isNull()) {
      spec.unavailableReason = object.value(QStringLiteral("unavailableReason")).toString();
    }
    for (const QJsonValue& model : object.value(QStringLiteral("supportedModels")).toArray()) {
      spec.supportedModels.push_back(model.toString());
    }
    parsed.push_back(spec);
  }
  specs_ = std::move(parsed);
  loaded_ = true;
  return true;
}

const TweakSpec* TweakCatalog::spec(const QString& id) const {
  for (const auto& item : specs_) {
    if (item.id == id) {
      return &item;
    }
  }
  return nullptr;
}

std::optional<int> TweakCatalog::compareVersion(const QString& lhs, const QString& rhs) {
  const auto parse = [](const QString& version) -> std::optional<QVector<qint64>> {
    const QStringList parts = version.split(QLatin1Char('.'));
    if (parts.isEmpty() || parts.size() > 3) {
      return std::nullopt;
    }
    QVector<qint64> out;
    for (const QString& part : parts) {
      if (part.isEmpty()) {
        return std::nullopt;
      }
      for (const QChar ch : part) {
        if (!ch.isDigit()) {
          return std::nullopt;
        }
      }
      bool ok = false;
      const qint64 value = part.toLongLong(&ok);
      if (!ok) {
        return std::nullopt;
      }
      out.push_back(value);
    }
    return out;
  };
  auto left = parse(lhs);
  auto right = parse(rhs);
  if (!left || !right) {
    return std::nullopt;
  }
  const int count = qMax(left->size(), right->size());
  left->resize(count);
  right->resize(count);
  for (int i = 0; i < count; ++i) {
    if (left->at(i) < right->at(i)) {
      return -1;
    }
    if (left->at(i) > right->at(i)) {
      return 1;
    }
  }
  return 0;
}

std::optional<TweakStatus> TweakCatalog::deviceStatus(const TweakDevice& device) {
  const QString majorText = device.systemVersion.section(QLatin1Char('.'), 0, 0);
  bool ok = false;
  const qint64 major = majorText.toLongLong(&ok);
  if (!ok || !compareVersion(device.systemVersion, device.systemVersion) ||
      (device.architecture != QLatin1String("arm64") &&
       device.architecture != QLatin1String("arm64e")) ||
      device.isSimulator) {
    return TweakStatus::InvalidDevice;
  }
  if (major < 15 || major > 17) {
    return TweakStatus::OutsideLegacyScope;
  }
  if (inList(kOs15Only, std::size(kOs15Only), device.model)) {
    return major == 15 ? std::nullopt : std::optional(TweakStatus::HardwareMismatch);
  }
  if (inList(kThroughOs16, std::size(kThroughOs16), device.model)) {
    return (major == 15 || major == 16) ? std::nullopt
                                        : std::optional(TweakStatus::HardwareMismatch);
  }
  if (inList(kFromOs15, std::size(kFromOs15), device.model)) {
    return std::nullopt;
  }
  if (inList(kFromOs16, std::size(kFromOs16), device.model)) {
    return (major == 16 || major == 17) ? std::nullopt
                                        : std::optional(TweakStatus::HardwareMismatch);
  }
  if (inList(kFromOs17, std::size(kFromOs17), device.model)) {
    return major == 17 ? std::nullopt : std::optional(TweakStatus::HardwareMismatch);
  }
  return TweakStatus::InvalidDevice;
}

TweakStatus TweakCatalog::status(const TweakSpec& spec, const TweakDevice& device) {
  if (const auto blocked = deviceStatus(device)) {
    return *blocked;
  }
  if (spec.disabled || spec.unavailableReason) {
    return TweakStatus::UpstreamUnavailable;
  }
  if (spec.minVersion) {
    const auto cmp = compareVersion(device.systemVersion, *spec.minVersion);
    if (!cmp) {
      return TweakStatus::UpstreamUnavailable;
    }
    if (*cmp < 0) {
      return TweakStatus::RequiresNewerIos;
    }
  }
  if (!spec.isCompatible(device)) {
    return TweakStatus::HardwareMismatch;
  }
  if (spec.section != TweakSection::Daemons) {
    for (const char* factory : kPortedFactories) {
      if (spec.factory == QLatin1String(factory)) {
        return TweakStatus::DraftCandidate;
      }
    }
  }
  return TweakStatus::MechanismNotPorted;
}

QString TweakCatalog::statusCode(TweakStatus status) {
  switch (status) {
    case TweakStatus::DraftCandidate:
      return QStringLiteral("draft_candidate");
    case TweakStatus::RequiresNewerIos:
      return QStringLiteral("requires_newer_ios");
    case TweakStatus::UpstreamUnavailable:
      return QStringLiteral("upstream_unavailable");
    case TweakStatus::HardwareMismatch:
      return QStringLiteral("hardware_mismatch");
    case TweakStatus::OutsideLegacyScope:
      return QStringLiteral("outside_legacy_scope");
    case TweakStatus::InvalidDevice:
      return QStringLiteral("invalid_device");
    case TweakStatus::MechanismNotPorted:
      return QStringLiteral("mechanism_not_ported");
  }
  return QStringLiteral("mechanism_not_ported");
}

QVector<TweakSummary> TweakCatalog::summarize(const TweakDevice& device) const {
  QVector<TweakSummary> out;
  out.reserve(specs_.size());
  for (const auto& spec : specs_) {
    TweakSummary summary;
    summary.id = spec.id;
    summary.title = spec.title;
    summary.section = spec.section;
    summary.detail = spec.detail;
    summary.warning = spec.warning;
    summary.minimumIos = spec.minVersion;
    summary.status = status(spec, device);
    summary.defaultValue = spec.value;
    summary.destination = spec.path;
    summary.key = spec.key;
    summary.kind = spec.kind;
    summary.writesWholeDict = spec.writesWholeDict();
    out.push_back(summary);
  }
  return out;
}

TweakCounts TweakCatalog::counts(const TweakDevice& device) const {
  TweakCounts counts;
  for (const auto& summary : summarize(device)) {
    ++counts.total;
    switch (summary.status) {
      case TweakStatus::DraftCandidate:
        ++counts.candidates;
        break;
      case TweakStatus::RequiresNewerIos:
        ++counts.requiresNewerIos;
        break;
      case TweakStatus::UpstreamUnavailable:
        ++counts.upstreamUnavailable;
        break;
      case TweakStatus::HardwareMismatch:
        ++counts.hardwareMismatch;
        break;
      case TweakStatus::OutsideLegacyScope:
        ++counts.outsideLegacyScope;
        break;
      case TweakStatus::InvalidDevice:
        ++counts.invalidDevice;
        break;
      case TweakStatus::MechanismNotPorted:
        ++counts.mechanismNotPorted;
        break;
    }
  }
  return counts;
}
