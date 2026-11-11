#include "DeviceScanner.h"

bool DeviceScanner::validUdid(const QString& id) {
  if (id.size() == 40) {
    for (const QChar ch : id) {
      if (!ch.isDigit() && (ch < QLatin1Char('a') || ch > QLatin1Char('f')) &&
          (ch < QLatin1Char('A') || ch > QLatin1Char('F'))) {
        return false;
      }
    }
    return true;
  }
  if (id.size() == 25 && id.at(8) == QLatin1Char('-')) {
    for (int i = 0; i < id.size(); ++i) {
      if (i == 8) {
        continue;
      }
      const QChar ch = id.at(i);
      if (!ch.isDigit() && (ch < QLatin1Char('a') || ch > QLatin1Char('f')) &&
          (ch < QLatin1Char('A') || ch > QLatin1Char('F'))) {
        return false;
      }
    }
    return true;
  }
  return false;
}

QString DeviceScanner::maskUdid(const QString& id) {
  if (id.size() < 8) {
    return QStringLiteral("********");
  }
  return id.left(4) + QString(id.size() - 8, QLatin1Char('*')) + id.right(4);
}

static std::optional<QString> readKey(const QString& id, const QString& key,
                                      CliFailure* failure) {
  const auto bytes =
      CliRunner::checked(QStringLiteral("ideviceinfo"),
                         {QStringLiteral("-u"), id, QStringLiteral("-k"), key}, 15000,
                         failure);
  if (!bytes) {
    return std::nullopt;
  }
  return QString::fromUtf8(*bytes).trimmed();
}

DeviceScan DeviceScanner::scan() {
  DeviceScan result;
  CliFailure failure = CliFailure::CommandFailed;
  const auto listBytes = CliRunner::checked(QStringLiteral("idevice_id"),
                                            {QStringLiteral("-l")}, 10000, &failure);
  if (!listBytes) {
    result.failure = failure;
    return result;
  }

  const QString text = QString::fromUtf8(*listBytes);
  const QStringList lines = text.split(QLatin1Char('\n'), Qt::SkipEmptyParts);
  for (QString line : lines) {
    line = line.trimmed();
    if (line.isEmpty()) {
      continue;
    }
    if (!validUdid(line) || result.devices.size() >= 32) {
      result.devices.clear();
      result.failure = CliFailure::InvalidOutput;
      return result;
    }

    DeviceRow row;
    row.id = line;
    row.idMasked = maskUdid(line);

    CliFailure infoFailure = CliFailure::CommandFailed;
    const auto product = readKey(line, QStringLiteral("ProductType"), &infoFailure);
    const auto ios = readKey(line, QStringLiteral("ProductVersion"), &infoFailure);
    const auto build = readKey(line, QStringLiteral("BuildVersion"), &infoFailure);
    const auto arch = readKey(line, QStringLiteral("CPUArchitecture"), &infoFailure);
    if (product && ios && build) {
      DeviceInfo info{*product, *ios, *build,
                      arch && !arch->isEmpty() ? *arch : QStringLiteral("arm64")};
      row.info = info;
      row.trust = QStringLiteral("lockdown_readable");
      row.compatibility = evaluateCompatibility(info.product, info.ios);
    } else {
      row.trust = QStringLiteral("unknown");
      row.failure = infoFailure;
      row.compatibility = evaluateCompatibility(QString(), QString());
    }
    result.devices.push_back(row);
  }
  return result;
}
