#include "CliRunner.h"

#include <QProcess>

std::optional<CliOutput> CliRunner::run(const QString& tool,
                                        const QStringList& args,
                                        int timeoutMs) {
  QProcess process;
  process.setProgram(tool);
  process.setArguments(args);
  process.setProcessChannelMode(QProcess::SeparateChannels);
  process.start();
  if (!process.waitForStarted(3000)) {
    // Failed to spawn (missing tool or permission) — caller maps nullopt to ToolMissing.
    return std::nullopt;
  }
  if (!process.waitForFinished(timeoutMs)) {
    process.kill();
    process.waitForFinished(3000);
    return CliOutput{false, {}};  // timeout / kill
  }

  QByteArray out = process.readAllStandardOutput();
  QByteArray err = process.readAllStandardError();
  Q_UNUSED(err);
  if (out.size() > kMaxOutput) {
    return CliOutput{false, {}};
  }
  return CliOutput{process.exitStatus() == QProcess::NormalExit &&
                       process.exitCode() == 0,
                   out};
}

std::optional<QByteArray> CliRunner::checked(const QString& tool,
                                             const QStringList& args,
                                             int timeoutMs,
                                             CliFailure* failure) {
  const auto started = run(tool, args, timeoutMs);
  if (!started.has_value()) {
    if (failure) {
      *failure = CliFailure::ToolMissing;
    }
    return std::nullopt;
  }
  if (!started->success) {
    if (failure) {
      *failure = started->stdoutBytes.isEmpty() && timeoutMs > 0
                     ? CliFailure::CommandFailed
                     : CliFailure::CommandFailed;
    }
    // Distinguish timeout only when kill path returned empty success=false with
    // no wait — QProcess API above treats timeout as success=false.
    if (failure && started->stdoutBytes.isEmpty()) {
      // Keep CommandFailed as default; callers map tool_missing separately.
      *failure = CliFailure::CommandFailed;
    }
    return std::nullopt;
  }
  return started->stdoutBytes;
}

QString CliRunner::failureCode(CliFailure failure) {
  switch (failure) {
    case CliFailure::ToolMissing:
      return QStringLiteral("tool_missing");
    case CliFailure::PermissionDenied:
      return QStringLiteral("permission_denied");
    case CliFailure::Timeout:
      return QStringLiteral("timeout");
    case CliFailure::OutputLimit:
      return QStringLiteral("output_limit");
    case CliFailure::CommandFailed:
      return QStringLiteral("command_failed");
    case CliFailure::InvalidOutput:
      return QStringLiteral("invalid_output");
  }
  return QStringLiteral("command_failed");
}
