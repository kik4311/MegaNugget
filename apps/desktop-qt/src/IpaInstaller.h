#pragma once

#include "CliRunner.h"
#include "DeviceScanner.h"

#include <QString>
#include <optional>

enum class InstallError {
  InvalidIpa,
  UnsafeArchive,
  SizeLimit,
  MissingSigningArtifacts,
  Io,
  InvalidDevice,
  UnsupportedTool,
  ToolFailure,
  VerificationFailed,
};

struct IpaInfo {
  QString bundleId;
  QString version;
  QString minimumIos;
  QString signing;
};

struct ValidatedIpa {
  IpaInfo info;
  QString snapshotPath;
};

enum class InstallerCli { Legacy, Modern };

class IpaInstaller {
 public:
  static constexpr qint64 kMaxIpa = 256LL * 1024 * 1024;
  static constexpr qint64 kMaxExpanded = 1024LL * 1024 * 1024;

  [[nodiscard]] static QString errorCode(InstallError error);
  [[nodiscard]] static std::optional<ValidatedIpa> inspect(const QString& path,
                                                           InstallError* error = nullptr);
  [[nodiscard]] static std::optional<InstallerCli> detectCli(InstallError* error = nullptr);
  [[nodiscard]] static std::optional<QString> install(const ValidatedIpa& ipa,
                                                      const QString& udid,
                                                      InstallError* error = nullptr);
};
