#include "ApplyChannelCli.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QProcess>
#include <QRegularExpression>
#include <QSettings>

ApplyChannelCli::InvokeHook ApplyChannelCli::testHook_;

void ApplyChannelCli::setInvokeHookForTests(InvokeHook hook) {
  testHook_ = std::move(hook);
}

void ApplyChannelCli::clearInvokeHookForTests() {
  testHook_ = nullptr;
}

QString ApplyChannelCli::hideLowPowerAlertsDraftJson() {
  return QStringLiteral(
      "{\"items\":[{\"id\":\"SBHideLowPowerAlerts\",\"kind\":\"toggle\","
      "\"key\":\"SBHideLowPowerAlerts\",\"domain\":\"HomeDomain\","
      "\"relative_path\":\"Library/Preferences/com.apple.springboard.plist\","
      "\"value\":true,\"accept_case_variants\":true}]}");
}

QString ApplyChannelCli::suggestedAppliedDir(const QString& verifiedBackupDir) {
  const QFileInfo info(verifiedBackupDir);
  const QString base = info.fileName().isEmpty() ? QStringLiteral("backup") : info.fileName();
  return info.dir().filePath(base + QStringLiteral("-applied"));
}

QStringList ApplyChannelCli::planArgs(const QString& backupDir, const QString& draftPath,
                                      const QString& planOutPath) {
  return {QStringLiteral("plan"), QDir::toNativeSeparators(backupDir),
          QDir::toNativeSeparators(draftPath), QDir::toNativeSeparators(planOutPath)};
}

QStringList ApplyChannelCli::applyArgs(const QString& backupDir, const QString& draftPath,
                                       const QString& appliedDir) {
  return {QStringLiteral("apply"), QDir::toNativeSeparators(backupDir),
          QDir::toNativeSeparators(draftPath), QDir::toNativeSeparators(appliedDir)};
}

QStringList ApplyChannelCli::restoreArgs(const QString& appliedDir) {
  return {QStringLiteral("restore"), QDir::toNativeSeparators(appliedDir),
          QStringLiteral("--consent-restore")};
}

QStringList ApplyChannelCli::effectArgs(const QString& draftPath, const QString& preBackup,
                                        const QString& postBackup) {
  return {QStringLiteral("effect"), QDir::toNativeSeparators(draftPath),
          QDir::toNativeSeparators(preBackup), QDir::toNativeSeparators(postBackup)};
}

QString ApplyChannelCli::parseErrorFromStderr(const QByteArray& stderrBytes) {
  const QString text = QString::fromUtf8(stderrBytes).trimmed();
  static const QRegularExpression pattern(
      QStringLiteral("^error:\\s*([a-z0-9_]+)\\s*$"),
      QRegularExpression::MultilineOption);
  const auto match = pattern.match(text);
  if (match.hasMatch()) {
    return match.captured(1);
  }
  return QString();
}

std::optional<QString> ApplyChannelCli::resolveBinary(QString* detail) {
  auto note = [&](const QString& line) {
    if (detail) {
      if (!detail->isEmpty()) {
        *detail += QLatin1Char('\n');
      }
      *detail += line;
    }
  };

  QSettings settings(QStringLiteral("MegaNugget"), QStringLiteral("DesktopQt"));
  const QString configured = settings.value(QStringLiteral("apply_channel_path")).toString().trimmed();
  if (!configured.isEmpty()) {
    if (QFileInfo::exists(configured)) {
      note(QStringLiteral("settings: %1").arg(configured));
      return configured;
    }
    note(QStringLiteral("settings path missing: %1").arg(configured));
  }

  const QString envPath = qEnvironmentVariable("APPLY_CHANNEL_BIN");
  if (!envPath.isEmpty()) {
    if (QFileInfo::exists(envPath)) {
      note(QStringLiteral("APPLY_CHANNEL_BIN: %1").arg(envPath));
      return envPath;
    }
    note(QStringLiteral("APPLY_CHANNEL_BIN missing: %1").arg(envPath));
  }

#ifdef Q_OS_WIN
  const QString exeName = QStringLiteral("apply-channel.exe");
#else
  const QString exeName = QStringLiteral("apply-channel");
#endif
  const QString besideExe =
      QDir(QCoreApplication::applicationDirPath()).filePath(exeName);
  if (QFileInfo::exists(besideExe)) {
    note(QStringLiteral("beside exe: %1").arg(besideExe));
    return besideExe;
  }
  note(QStringLiteral("not beside exe: %1").arg(besideExe));

  const QStringList cargoNames = {
#ifdef Q_OS_WIN
      QStringLiteral("target/release/apply-channel.exe"),
      QStringLiteral("target/debug/apply-channel.exe"),
#else
      QStringLiteral("target/release/apply-channel"),
      QStringLiteral("target/debug/apply-channel"),
#endif
  };

  QDir walk(QCoreApplication::applicationDirPath());
  for (int depth = 0; depth < 8; ++depth) {
    for (const QString& rel : cargoNames) {
      const QString candidate = walk.filePath(rel);
      if (QFileInfo::exists(candidate)) {
        note(QStringLiteral("cargo target: %1").arg(candidate));
        return candidate;
      }
    }
    if (!walk.cdUp()) {
      break;
    }
  }

  const QString cwd = QDir::currentPath();
  for (const QString& rel : cargoNames) {
    const QString candidate = QDir(cwd).filePath(rel);
    if (QFileInfo::exists(candidate)) {
      note(QStringLiteral("cwd cargo: %1").arg(candidate));
      return candidate;
    }
  }

  return std::nullopt;
}

ApplyChannelRunResult ApplyChannelCli::invokeDefault(const QString& binary, const QStringList& args,
                                                     int timeoutMs) {
  ApplyChannelRunResult out;
  QProcess process;
  process.setProgram(binary);
  process.setArguments(args);
  process.setProcessChannelMode(QProcess::SeparateChannels);
  process.start();
  if (!process.waitForStarted(5000)) {
    out.toolMissing = true;
    out.errorCode = QStringLiteral("apply_channel_missing");
    return out;
  }
  const bool finished =
      timeoutMs <= 0 ? process.waitForFinished(-1)
                     : process.waitForFinished(timeoutMs);
  if (!finished) {
    process.kill();
    process.waitForFinished(5000);
    out.timedOut = true;
    out.errorCode = QStringLiteral("timeout");
    return out;
  }
  out.stdoutJson = process.readAllStandardOutput();
  const QByteArray err = process.readAllStandardError();
  out.success =
      process.exitStatus() == QProcess::NormalExit && process.exitCode() == 0;
  if (!out.success) {
    out.errorCode = parseErrorFromStderr(err);
    if (out.errorCode.isEmpty()) {
      out.errorCode = QStringLiteral("command_failed");
    }
  }
  return out;
}

ApplyChannelRunResult ApplyChannelCli::run(const QStringList& args, int timeoutMs) {
  if (testHook_) {
    return testHook_(QStringLiteral("test-hook-binary"), args, timeoutMs);
  }
  QString detail;
  const auto binary = resolveBinary(&detail);
  if (!binary) {
    ApplyChannelRunResult out;
    out.toolMissing = true;
    out.errorCode = QStringLiteral("apply_channel_missing");
    out.stdoutJson = detail.toUtf8();
    return out;
  }
  return invokeDefault(*binary, args, timeoutMs);
}
