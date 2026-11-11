#pragma once

#include "DeviceScanner.h"

#include <QString>
#include <QVector>
#include <optional>

enum class ApplyError {
  UsbFailed,
  LockdownUnreadable,
  DeviceNotFound,
  TargetMismatch,
  BackupToolMissing,
  BackupRequiresConsent,
  BackupFailed,
  BackupUnfinished,
  BackupLayoutUnsupported,
};

struct ApplyDetectedDevice {
  QString udid;
  QString udidMasked;
  QString product;
  QString ios;
  bool matchesTarget = false;
  bool lockdownReadable = false;
};

struct ApplyDetection {
  QVector<ApplyDetectedDevice> devices;
  QStringList failures;
};

struct BackupSnapshot {
  QString udidMasked;
  QString product;
  QString ios;
  QString snapshotState;
};

class ApplyChannel {
 public:
  [[nodiscard]] static QString errorCode(ApplyError error);
  [[nodiscard]] static bool targetMatches(const QString& product, const QString& ios);
  [[nodiscard]] static ApplyDetection detect();
  [[nodiscard]] static std::optional<QString> singleTargetUdid(ApplyError* error = nullptr);
  [[nodiscard]] static std::optional<BackupSnapshot> verifyBackup(const QString& dir,
                                                                  ApplyError* error = nullptr);
  /// Builds argv for idevicebackup2; never includes free-form options.
  [[nodiscard]] static QStringList backupArgs(const QString& udid, const QString& targetDir);
};
