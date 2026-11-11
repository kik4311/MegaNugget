#pragma once

#include "TweakCatalog.h"

#include <QMap>
#include <QString>
#include <QVector>
#include <optional>

struct PlistFileDraft {
  QString domain;
  QString relativePath;
  QString xml;
};

struct DraftPreview {
  QVector<PlistFileDraft> files;
  int selectedCount = 0;
  bool appliedToDevice = false;
};

enum class DraftError {
  EmptySelection,
  TooManySelections,
  UnknownTweak,
  BlockedTweak,
  InvalidValue,
  Conflict,
  SerializationFailed,
};

class TweakCompiler {
 public:
  [[nodiscard]] static QString errorCode(DraftError error);
  [[nodiscard]] static std::optional<std::pair<QString, QString>> splitDomain(
      const QString& path);
  [[nodiscard]] static std::optional<DraftPreview> preview(
      const QMap<QString, TweakValue>& choices, const TweakDevice& device,
      DraftError* error = nullptr, QString* conflictDetail = nullptr);
};
