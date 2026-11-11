#pragma once

#include <QByteArray>
#include <QString>
#include <QStringList>
#include <optional>

enum class CliFailure {
  ToolMissing,
  PermissionDenied,
  Timeout,
  OutputLimit,
  CommandFailed,
  InvalidOutput,
};

struct CliOutput {
  bool success = false;
  QByteArray stdoutBytes;
};

class CliRunner {
 public:
  static constexpr qint64 kMaxOutput = 4 * 1024 * 1024;

  [[nodiscard]] static std::optional<CliOutput> run(const QString& tool,
                                                    const QStringList& args,
                                                    int timeoutMs);
  [[nodiscard]] static std::optional<QByteArray> checked(const QString& tool,
                                                         const QStringList& args,
                                                         int timeoutMs,
                                                         CliFailure* failure = nullptr);
  [[nodiscard]] static QString failureCode(CliFailure failure);
};
