#include "IpaInstaller.h"

#include "PlistReader.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QProcess>
#include <QSet>
#include <QTemporaryFile>

namespace {

QStringList tarList(const QString& ipaPath, InstallError* error) {
  QProcess process;
  process.start(QStringLiteral("tar"),
                {QStringLiteral("-tf"), QDir::toNativeSeparators(ipaPath)});
  if (!process.waitForStarted(3000)) {
    if (error) {
      *error = InstallError::Io;
    }
    return {};
  }
  if (!process.waitForFinished(30000)) {
    process.kill();
    if (error) {
      *error = InstallError::Io;
    }
    return {};
  }
  if (process.exitCode() != 0) {
    if (error) {
      *error = InstallError::InvalidIpa;
    }
    return {};
  }
  const QString text = QString::fromUtf8(process.readAllStandardOutput());
  return text.split(QLatin1Char('\n'), Qt::SkipEmptyParts);
}

QByteArray tarExtract(const QString& ipaPath, const QString& member, InstallError* error) {
  QProcess process;
  process.start(QStringLiteral("tar"),
                {QStringLiteral("-xOf"), QDir::toNativeSeparators(ipaPath), member});
  if (!process.waitForStarted(3000)) {
    if (error) {
      *error = InstallError::Io;
    }
    return {};
  }
  if (!process.waitForFinished(30000)) {
    process.kill();
    if (error) {
      *error = InstallError::Io;
    }
    return {};
  }
  if (process.exitCode() != 0) {
    if (error) {
      *error = InstallError::InvalidIpa;
    }
    return {};
  }
  QByteArray bytes = process.readAllStandardOutput();
  if (bytes.size() > 1024 * 1024) {
    if (error) {
      *error = InstallError::SizeLimit;
    }
    return {};
  }
  return bytes;
}

bool validBundleId(const QString& id) {
  if (id.isEmpty() || id.size() > 255 || !id.contains(QLatin1Char('.'))) {
    return false;
  }
  for (const QChar ch : id) {
    if (!ch.isLetterOrNumber() && ch != QLatin1Char('.') && ch != QLatin1Char('-')) {
      return false;
    }
  }
  return true;
}

bool memberExists(const QStringList& names, const QString& member) {
  return names.contains(member) || names.contains(member + QLatin1Char('/'));
}

}  // namespace

QString IpaInstaller::errorCode(InstallError error) {
  switch (error) {
    case InstallError::InvalidIpa:
      return QStringLiteral("invalid_ipa");
    case InstallError::UnsafeArchive:
      return QStringLiteral("unsafe_archive");
    case InstallError::SizeLimit:
      return QStringLiteral("size_limit");
    case InstallError::MissingSigningArtifacts:
      return QStringLiteral("missing_signing_artifacts");
    case InstallError::Io:
      return QStringLiteral("io");
    case InstallError::InvalidDevice:
      return QStringLiteral("invalid_device");
    case InstallError::UnsupportedTool:
      return QStringLiteral("unsupported_tool");
    case InstallError::ToolFailure:
      return QStringLiteral("tool_failure");
    case InstallError::VerificationFailed:
      return QStringLiteral("verification_failed");
  }
  return QStringLiteral("invalid_ipa");
}

std::optional<ValidatedIpa> IpaInstaller::inspect(const QString& path, InstallError* error) {
  const QFileInfo info(path);
  if (!info.exists() || !info.isFile() ||
      info.suffix().compare(QLatin1String("ipa"), Qt::CaseInsensitive) != 0) {
    if (error) {
      *error = InstallError::InvalidIpa;
    }
    return std::nullopt;
  }
  if (info.size() > kMaxIpa) {
    if (error) {
      *error = InstallError::SizeLimit;
    }
    return std::nullopt;
  }

  QTemporaryFile snapshot(QDir::temp().filePath(QStringLiteral("meganugget-XXXXXX.ipa")));
  snapshot.setAutoRemove(false);
  if (!snapshot.open()) {
    if (error) {
      *error = InstallError::Io;
    }
    return std::nullopt;
  }
  QFile input(path);
  if (!input.open(QIODevice::ReadOnly)) {
    if (error) {
      *error = InstallError::Io;
    }
    return std::nullopt;
  }
  if (snapshot.write(input.readAll()) != info.size()) {
    if (error) {
      *error = InstallError::Io;
    }
    return std::nullopt;
  }
  snapshot.close();
  const QString snapshotPath = snapshot.fileName();

  InstallError localError = InstallError::InvalidIpa;
  const QStringList names = tarList(snapshotPath, &localError);
  if (names.isEmpty()) {
    QFile::remove(snapshotPath);
    if (error) {
      *error = localError;
    }
    return std::nullopt;
  }
  if (names.size() > 20000) {
    QFile::remove(snapshotPath);
    if (error) {
      *error = InstallError::SizeLimit;
    }
    return std::nullopt;
  }

  QSet<QString> unique;
  QSet<QString> roots;
  for (QString name : names) {
    name = name.trimmed();
    if (name.endsWith(QLatin1Char('/'))) {
      name.chop(1);
    }
    if (name.isEmpty()) {
      continue;
    }
    if (name.contains(QLatin1Char('\\')) || name.contains(QLatin1Char(':')) ||
        name.split(QLatin1Char('/')).contains(QLatin1String("..")) ||
        name.split(QLatin1Char('/')).contains(QLatin1String(".")) ||
        unique.contains(name)) {
      QFile::remove(snapshotPath);
      if (error) {
        *error = InstallError::UnsafeArchive;
      }
      return std::nullopt;
    }
    unique.insert(name);
    const QStringList parts = name.split(QLatin1Char('/'));
    if (parts.size() >= 2 && parts[0] == QLatin1String("Payload") &&
        parts[1].endsWith(QLatin1String(".app"))) {
      roots.insert(QStringLiteral("Payload/%1").arg(parts[1]));
    }
  }
  if (roots.size() != 1) {
    QFile::remove(snapshotPath);
    if (error) {
      *error = InstallError::InvalidIpa;
    }
    return std::nullopt;
  }
  const QString root = *roots.constBegin();
  const QByteArray plistBytes =
      tarExtract(snapshotPath, root + QStringLiteral("/Info.plist"), &localError);
  if (plistBytes.isEmpty()) {
    QFile::remove(snapshotPath);
    if (error) {
      *error = localError;
    }
    return std::nullopt;
  }
  const auto app = parseAppPlist(plistBytes);
  if (!app || !validBundleId(app->bundleId) || app->executable.isEmpty() ||
      app->executable.contains(QLatin1Char('/')) ||
      app->executable.contains(QLatin1Char('\\')) ||
      app->executable == QLatin1String(".") || app->executable == QLatin1String("..") ||
      app->packageType != QLatin1String("APPL") ||
      !app->platforms.contains(QLatin1String("iPhoneOS")) || app->version.isEmpty() ||
      app->version.size() > 100 || app->minimumIos.size() > 32) {
    QFile::remove(snapshotPath);
    if (error) {
      *error = InstallError::InvalidIpa;
    }
    return std::nullopt;
  }

  const QString executableMember = root + QLatin1Char('/') + app->executable;
  const QString codeResources =
      root + QStringLiteral("/_CodeSignature/CodeResources");
  const QString provision = root + QStringLiteral("/embedded.mobileprovision");
  if (!memberExists(names, executableMember)) {
    QFile::remove(snapshotPath);
    if (error) {
      *error = InstallError::MissingSigningArtifacts;
    }
    return std::nullopt;
  }
  const bool signingArtifacts =
      memberExists(names, codeResources) && memberExists(names, provision);
  if (!signingArtifacts) {
    QFile::remove(snapshotPath);
    if (error) {
      *error = InstallError::MissingSigningArtifacts;
    }
    return std::nullopt;
  }

  ValidatedIpa validated;
  validated.info.bundleId = app->bundleId;
  validated.info.version = app->version;
  validated.info.minimumIos = app->minimumIos;
  validated.info.signing = QStringLiteral("artifacts_present_not_verified");
  validated.snapshotPath = snapshotPath;
  return validated;
}

std::optional<InstallerCli> IpaInstaller::detectCli(InstallError* error) {
  CliFailure failure = CliFailure::CommandFailed;
  const auto help =
      CliRunner::checked(QStringLiteral("ideviceinstaller"), {QStringLiteral("--help")},
                         10000, &failure);
  if (!help) {
    if (error) {
      *error = failure == CliFailure::ToolMissing ? InstallError::UnsupportedTool
                                                  : InstallError::ToolFailure;
    }
    return std::nullopt;
  }
  const QString text = QString::fromUtf8(*help);
  if (text.contains(QLatin1String("--install")) &&
      text.contains(QLatin1String("--list-apps"))) {
    return InstallerCli::Legacy;
  }
  if (text.contains(QLatin1String("install PATH")) &&
      text.contains(QLatin1String("--xml"))) {
    return InstallerCli::Modern;
  }
  if (error) {
    *error = InstallError::UnsupportedTool;
  }
  return std::nullopt;
}

std::optional<QString> IpaInstaller::install(const ValidatedIpa& ipa, const QString& udid,
                                             InstallError* error) {
  if (!DeviceScanner::validUdid(udid)) {
    if (error) {
      *error = InstallError::InvalidDevice;
    }
    return std::nullopt;
  }
  InstallError localError = InstallError::ToolFailure;
  const auto cli = detectCli(&localError);
  if (!cli) {
    if (error) {
      *error = localError;
    }
    return std::nullopt;
  }

  // Confirm device is still readable before install.
  CliFailure failure = CliFailure::CommandFailed;
  if (!CliRunner::checked(QStringLiteral("ideviceinfo"),
                          {QStringLiteral("-u"), udid, QStringLiteral("-k"),
                           QStringLiteral("ProductType")},
                          15000, &failure)) {
    if (error) {
      *error = InstallError::ToolFailure;
    }
    return std::nullopt;
  }

  QStringList installArgs;
  if (*cli == InstallerCli::Legacy) {
    installArgs = {QStringLiteral("-u"), udid, QStringLiteral("-i"), ipa.snapshotPath};
  } else {
    installArgs = {QStringLiteral("-u"), udid, QStringLiteral("install"), ipa.snapshotPath};
  }
  if (!CliRunner::checked(QStringLiteral("ideviceinstaller"), installArgs, 180000,
                          &failure)) {
    if (error) {
      *error = InstallError::ToolFailure;
    }
    return std::nullopt;
  }

  QStringList listArgs;
  if (*cli == InstallerCli::Legacy) {
    listArgs = {QStringLiteral("-u"), udid, QStringLiteral("-l"), QStringLiteral("-o"),
                QStringLiteral("xml")};
  } else {
    listArgs = {QStringLiteral("-u"), udid, QStringLiteral("list"),
                QStringLiteral("--xml")};
  }
  const auto listed =
      CliRunner::checked(QStringLiteral("ideviceinstaller"), listArgs, 30000, &failure);
  if (!listed) {
    if (error) {
      *error = InstallError::VerificationFailed;
    }
    return std::nullopt;
  }

  // Lightweight verification: bundle id + version appear in XML listing text.
  const QString xml = QString::fromUtf8(*listed);
  if (!xml.contains(ipa.info.bundleId) || !xml.contains(ipa.info.version)) {
    if (error) {
      *error = InstallError::VerificationFailed;
    }
    return QStringLiteral("installed_unverified");
  }
  return QStringLiteral("installed_verified");
}
