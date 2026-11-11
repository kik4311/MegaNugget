#pragma once

#include <QByteArray>
#include <QString>
#include <QStringList>
#include <functional>
#include <optional>

struct ApplyChannelRunResult {
  bool success = false;
  QByteArray stdoutJson;
  QString errorCode;  // stable snake_case from stderr when !success
  bool toolMissing = false;
  bool timedOut = false;
};

/// Locates and invokes the Rust `apply-channel` binary (plan / apply / restore / effect).
class ApplyChannelCli {
 public:
  using InvokeHook =
      std::function<ApplyChannelRunResult(const QString& binary, const QStringList& args,
                                          int timeoutMs)>;

  static void setInvokeHookForTests(InvokeHook hook);
  static void clearInvokeHookForTests();

  /// Search order: QSettings path, `APPLY_CHANNEL_BIN`, next to the Qt exe, repo `target/*`.
  [[nodiscard]] static std::optional<QString> resolveBinary(QString* detail = nullptr);

  [[nodiscard]] static ApplyChannelRunResult run(const QStringList& args, int timeoutMs = 120000);

  [[nodiscard]] static QString parseErrorFromStderr(const QByteArray& stderrBytes);
  [[nodiscard]] static QString hideLowPowerAlertsDraftJson();
  [[nodiscard]] static QString suggestedAppliedDir(const QString& verifiedBackupDir);
  [[nodiscard]] static QStringList planArgs(const QString& backupDir, const QString& draftPath,
                                            const QString& planOutPath);
  [[nodiscard]] static QStringList applyArgs(const QString& backupDir, const QString& draftPath,
                                             const QString& appliedDir);
  [[nodiscard]] static QStringList restoreArgs(const QString& appliedDir);
  [[nodiscard]] static QStringList effectArgs(const QString& draftPath, const QString& preBackup,
                                              const QString& postBackup);

 private:
  [[nodiscard]] static ApplyChannelRunResult invokeDefault(const QString& binary,
                                                           const QStringList& args, int timeoutMs);
  static InvokeHook testHook_;
};
