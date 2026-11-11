#pragma once

#include <QByteArray>
#include <QString>
#include <QStringList>
#include <optional>

struct AppPlistInfo {
  QString bundleId;
  QString version;
  QString executable;
  QString packageType;
  QString minimumIos;
  QStringList platforms;
};

std::optional<AppPlistInfo> parseAppPlist(const QByteArray& bytes);
