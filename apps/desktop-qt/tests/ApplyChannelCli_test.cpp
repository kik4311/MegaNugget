#include "ApplyChannelCli.h"

#include <QCoreApplication>
#include <QTest>

class ApplyChannelCliTest : public QObject {
  Q_OBJECT

 private slots:
  void hideLowPowerDraftContainsCaseVariants();
  void parseStableErrorCode();
  void planArgsWhitelisted();
  void restoreArgsRequireConsentFlag();
  void runUsesHookForRefusals();
  void suggestedAppliedSibling();
};

void ApplyChannelCliTest::hideLowPowerDraftContainsCaseVariants() {
  const QByteArray json = ApplyChannelCli::hideLowPowerAlertsDraftJson().toUtf8();
  QVERIFY(json.contains("SBHideLowPowerAlerts"));
  QVERIFY(json.contains("accept_case_variants"));
  QVERIFY(json.contains("com.apple.springboard.plist"));
}

void ApplyChannelCliTest::parseStableErrorCode() {
  QCOMPARE(ApplyChannelCli::parseErrorFromStderr("error: restore_requires_consent\n"),
           QStringLiteral("restore_requires_consent"));
  QCOMPARE(ApplyChannelCli::parseErrorFromStderr("noise\nerror: target_mismatch\n"),
           QStringLiteral("target_mismatch"));
  QVERIFY(ApplyChannelCli::parseErrorFromStderr("garbage").isEmpty());
}

void ApplyChannelCliTest::planArgsWhitelisted() {
  const QStringList args = ApplyChannelCli::planArgs(QStringLiteral("/b"), QStringLiteral("/d"),
                                                     QStringLiteral("/o"));
  QCOMPARE(args.size(), 4);
  QCOMPARE(args.at(0), QStringLiteral("plan"));
}

void ApplyChannelCliTest::restoreArgsRequireConsentFlag() {
  const QStringList args = ApplyChannelCli::restoreArgs(QStringLiteral("/applied"));
  QCOMPARE(args.at(0), QStringLiteral("restore"));
  QCOMPARE(args.last(), QStringLiteral("--consent-restore"));
}

void ApplyChannelCliTest::runUsesHookForRefusals() {
  ApplyChannelCli::setInvokeHookForTests([](const QString&, const QStringList& args, int) {
    ApplyChannelRunResult out;
    if (args.contains(QStringLiteral("--consent-restore"))) {
      out.success = true;
      out.stdoutJson = QByteArrayLiteral("{\"ok\":true}");
      return out;
    }
    out.success = false;
    out.errorCode = QStringLiteral("restore_requires_consent");
    return out;
  });
  const auto bad = ApplyChannelCli::run({QStringLiteral("restore"), QStringLiteral("/x")});
  QVERIFY(!bad.success);
  QCOMPARE(bad.errorCode, QStringLiteral("restore_requires_consent"));
  const auto good = ApplyChannelCli::run(
      {QStringLiteral("restore"), QStringLiteral("/x"), QStringLiteral("--consent-restore")});
  QVERIFY(good.success);
  ApplyChannelCli::clearInvokeHookForTests();
}

void ApplyChannelCliTest::suggestedAppliedSibling() {
  QCOMPARE(ApplyChannelCli::suggestedAppliedDir(QStringLiteral("/home/b/2026-11-06-first")),
           QStringLiteral("/home/b/2026-11-06-first-applied"));
}

QTEST_MAIN(ApplyChannelCliTest)
#include "ApplyChannelCli_test.moc"
