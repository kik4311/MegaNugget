#pragma once

#include <QMap>
#include <QString>
#include <QVector>
#include <optional>

struct TweakDevice {
  QString model;
  QString systemVersion;
  QString architecture = QStringLiteral("arm64");
  bool isSimulator = false;
};

enum class TweakKind { Toggle, Text, Number };
enum class TweakSection { LiquidGlass, Springboard, InternalOptions, Daemons };

enum class TweakStatus {
  DraftCandidate,
  RequiresNewerIos,
  UpstreamUnavailable,
  HardwareMismatch,
  OutsideLegacyScope,
  InvalidDevice,
  MechanismNotPorted,
};

struct TweakValue {
  enum class Type { Bool, Integer, Double, Text };
  Type type = Type::Bool;
  bool boolValue = false;
  qint64 intValue = 0;
  double doubleValue = 0.0;
  QString textValue;

  [[nodiscard]] QString display() const;
  [[nodiscard]] bool operator==(const TweakValue& other) const;
};

struct TweakSpec {
  QString id;
  TweakSection section = TweakSection::InternalOptions;
  QString title;
  QString path;
  QString key;
  TweakValue value;
  TweakKind kind = TweakKind::Toggle;
  double minValue = 0;
  double maxValue = 999;
  double step = 1;
  std::optional<QString> minVersion;
  std::optional<QString> maxVersion;
  bool iphoneOnly = false;
  bool ipadOnly = false;
  bool disabled = false;
  std::optional<QString> detail;
  QMap<QString, TweakValue> multiValues;  // empty => not whole-dict
  bool reversibleBoolean = false;
  bool requiresDynamicIsland = false;
  QString factory;
  std::optional<TweakValue> offValue;
  bool integerBoolean = false;
  bool requiresCameraControl = false;
  bool requiresActionButton = false;
  bool requiresHomeButton = false;
  bool requiresAlwaysOn = false;
  std::optional<QString> warning;
  QStringList requiredIds;
  std::optional<QString> exclusiveWith;
  std::optional<QString> unavailableReason;
  QStringList supportedModels;

  [[nodiscard]] bool writesWholeDict() const { return !multiValues.isEmpty(); }
  [[nodiscard]] bool isCompatible(const TweakDevice& device) const;
};

struct TweakSummary {
  QString id;
  QString title;
  TweakSection section = TweakSection::InternalOptions;
  std::optional<QString> detail;
  std::optional<QString> warning;
  std::optional<QString> minimumIos;
  TweakStatus status = TweakStatus::MechanismNotPorted;
  TweakValue defaultValue;
  QString destination;
  QString key;
  TweakKind kind = TweakKind::Toggle;
  bool writesWholeDict = false;
};

struct TweakCounts {
  int total = 0;
  int candidates = 0;
  int requiresNewerIos = 0;
  int upstreamUnavailable = 0;
  int hardwareMismatch = 0;
  int outsideLegacyScope = 0;
  int invalidDevice = 0;
  int mechanismNotPorted = 0;
};

class TweakCatalog {
 public:
  static constexpr int kRegistryCount = 258;
  static constexpr int kPreviewLimit = 64;
  static constexpr int kMaxTextBytes = 1024;

  [[nodiscard]] static TweakCatalog& instance();
  [[nodiscard]] bool load(QString* error = nullptr);
  [[nodiscard]] const QString& repository() const { return repository_; }
  [[nodiscard]] const QString& revision() const { return revision_; }
  [[nodiscard]] const QVector<TweakSpec>& specs() const { return specs_; }
  [[nodiscard]] const TweakSpec* spec(const QString& id) const;
  [[nodiscard]] QVector<TweakSummary> summarize(const TweakDevice& device) const;
  [[nodiscard]] TweakCounts counts(const TweakDevice& device) const;
  [[nodiscard]] static TweakStatus status(const TweakSpec& spec, const TweakDevice& device);
  [[nodiscard]] static std::optional<TweakStatus> deviceStatus(const TweakDevice& device);
  [[nodiscard]] static QString statusCode(TweakStatus status);
  [[nodiscard]] static std::optional<int> compareVersion(const QString& lhs, const QString& rhs);

 private:
  TweakCatalog() = default;
  bool loaded_ = false;
  QString repository_;
  QString revision_;
  QVector<TweakSpec> specs_;
};
