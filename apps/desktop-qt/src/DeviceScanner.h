#pragma once

#include "CliRunner.h"
#include "Compatibility.h"

#include <QString>
#include <QVector>
#include <optional>

struct DeviceInfo {
  QString product;
  QString ios;
  QString build;
  QString architecture = QStringLiteral("arm64");
};

struct DeviceRow {
  QString id;
  QString idMasked;
  std::optional<DeviceInfo> info;
  QString trust;
  std::optional<CliFailure> failure;
  Compatibility compatibility;
};

struct DeviceScan {
  QString transport = QStringLiteral("usbmuxd_usb");
  QVector<DeviceRow> devices;
  std::optional<CliFailure> failure;
};

class DeviceScanner {
 public:
  [[nodiscard]] static bool validUdid(const QString& id);
  [[nodiscard]] static QString maskUdid(const QString& id);
  [[nodiscard]] static DeviceScan scan();
};
