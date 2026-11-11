#include "ApplyChannel.h"

#include "CliRunner.h"
#include "Compatibility.h"

#include <QDir>
#include <QFileInfo>
#include <QFile>
#include <QXmlStreamReader>

QString ApplyChannel::errorCode(ApplyError error) {
  switch (error) {
    case ApplyError::UsbFailed:
      return QStringLiteral("usb_failed");
    case ApplyError::LockdownUnreadable:
      return QStringLiteral("lockdown_unreadable");
    case ApplyError::DeviceNotFound:
      return QStringLiteral("device_not_found");
    case ApplyError::TargetMismatch:
      return QStringLiteral("target_mismatch");
    case ApplyError::BackupToolMissing:
      return QStringLiteral("backup_tool_missing");
    case ApplyError::BackupRequiresConsent:
      return QStringLiteral("backup_requires_consent");
    case ApplyError::BackupFailed:
      return QStringLiteral("backup_failed");
    case ApplyError::BackupUnfinished:
      return QStringLiteral("backup_unfinished");
    case ApplyError::BackupLayoutUnsupported:
      return QStringLiteral("backup_layout_unsupported");
  }
  return QStringLiteral("backup_failed");
}

bool ApplyChannel::targetMatches(const QString& product, const QString& ios) {
  return evaluateCompatibility(product, ios).matchesTarget;
}

ApplyDetection ApplyChannel::detect() {
  ApplyDetection out;
  const DeviceScan scan = DeviceScanner::scan();
  if (scan.devices.isEmpty()) {
    out.failures.push_back(scan.failure ? CliRunner::failureCode(*scan.failure)
                                        : QStringLiteral("device_not_found"));
    return out;
  }
  for (const auto& device : scan.devices) {
    ApplyDetectedDevice row;
    row.udid = device.id;
    row.udidMasked = device.idMasked;
    if (device.info) {
      row.product = device.info->product;
      row.ios = device.info->ios;
      row.lockdownReadable = true;
      row.matchesTarget = targetMatches(row.product, row.ios);
    } else {
      row.lockdownReadable = false;
      out.failures.push_back(QStringLiteral("lockdown_unreadable"));
    }
    out.devices.push_back(row);
  }
  return out;
}

std::optional<QString> ApplyChannel::singleTargetUdid(ApplyError* error) {
  const DeviceScan scan = DeviceScanner::scan();
  if (scan.devices.isEmpty()) {
    if (error) {
      *error = scan.failure ? ApplyError::UsbFailed : ApplyError::DeviceNotFound;
    }
    return std::nullopt;
  }
  QStringList matching;
  for (const auto& device : scan.devices) {
    if (!device.info) {
      continue;
    }
    if (targetMatches(device.info->product, device.info->ios)) {
      matching.push_back(device.id);
    }
  }
  if (matching.size() != 1) {
    if (error) {
      *error = matching.isEmpty() ? ApplyError::TargetMismatch : ApplyError::DeviceNotFound;
    }
    // Two+ matching devices also refuse (ambiguous).
    if (matching.size() > 1 && error) {
      *error = ApplyError::DeviceNotFound;
    }
    return std::nullopt;
  }
  return matching.first();
}

QStringList ApplyChannel::backupArgs(const QString& udid, const QString& targetDir) {
  return {QStringLiteral("-u"), udid, QStringLiteral("backup"),
          QDir::toNativeSeparators(targetDir)};
}

namespace {

std::optional<QString> readStatusField(const QByteArray& xml, const QString& key) {
  QXmlStreamReader reader(xml);
  QString pending;
  while (!reader.atEnd()) {
    const auto token = reader.readNext();
    if (token == QXmlStreamReader::StartElement) {
      if (reader.name() == QLatin1String("key")) {
        pending = reader.readElementText();
      } else if (reader.name() == QLatin1String("string") && pending == key) {
        return reader.readElementText();
      } else if ((reader.name() == QLatin1String("true") ||
                  reader.name() == QLatin1String("false")) &&
                 pending == key) {
        return reader.name().toString();
      }
    }
  }
  return std::nullopt;
}

QString findStatusPlist(const QString& dir) {
  const QDir root(dir);
  const auto entries = root.entryList(QDir::Dirs | QDir::NoDotAndDotDot);
  for (const QString& name : entries) {
    const QString candidate = root.filePath(name + QStringLiteral("/Status.plist"));
    if (QFile::exists(candidate)) {
      return candidate;
    }
  }
  const QString direct = root.filePath(QStringLiteral("Status.plist"));
  return QFile::exists(direct) ? direct : QString();
}

}  // namespace

std::optional<BackupSnapshot> ApplyChannel::verifyBackup(const QString& dir,
                                                         ApplyError* error) {
  const QString statusPath = findStatusPlist(dir);
  if (statusPath.isEmpty()) {
    if (error) {
      *error = ApplyError::BackupLayoutUnsupported;
    }
    return std::nullopt;
  }
  QFile file(statusPath);
  if (!file.open(QIODevice::ReadOnly)) {
    if (error) {
      *error = ApplyError::BackupFailed;
    }
    return std::nullopt;
  }
  const QByteArray bytes = file.readAll();
  QString state;
  if (bytes.startsWith("bplist")) {
    // Minimal: require XML Status.plist for Qt verify; binary falls back to unfinished.
    if (error) {
      *error = ApplyError::BackupUnfinished;
    }
    return std::nullopt;
  }
  const auto snapshotState = readStatusField(bytes, QStringLiteral("SnapshotState"));
  if (!snapshotState) {
    if (error) {
      *error = ApplyError::BackupLayoutUnsupported;
    }
    return std::nullopt;
  }
  state = *snapshotState;
  if (state != QLatin1String("finished")) {
    if (error) {
      *error = ApplyError::BackupUnfinished;
    }
    return std::nullopt;
  }

  // Info.plist next to Status for product/ios when present.
  BackupSnapshot snap;
  snap.snapshotState = state;
  const QFileInfo statusInfo(statusPath);
  const QString infoPath = statusInfo.dir().filePath(QStringLiteral("Info.plist"));
  if (QFile::exists(infoPath)) {
    QFile infoFile(infoPath);
    if (infoFile.open(QIODevice::ReadOnly)) {
      const QByteArray infoBytes = infoFile.readAll();
      if (!infoBytes.startsWith("bplist")) {
        snap.product =
            readStatusField(infoBytes, QStringLiteral("Product Type"))
                .value_or(readStatusField(infoBytes, QStringLiteral("ProductType"))
                              .value_or(QString()));
        snap.ios = readStatusField(infoBytes, QStringLiteral("Product Version"))
                       .value_or(readStatusField(infoBytes, QStringLiteral("ProductVersion"))
                                     .value_or(QString()));
      }
    }
  }
  const QString udidDir = statusInfo.dir().dirName();
  snap.udidMasked = DeviceScanner::maskUdid(udidDir);
  if (!snap.product.isEmpty() && !snap.ios.isEmpty() &&
      !targetMatches(snap.product, snap.ios)) {
    if (error) {
      *error = ApplyError::TargetMismatch;
    }
    return std::nullopt;
  }
  return snap;
}
