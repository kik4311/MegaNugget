#include "Journal.h"

#include <QDateTime>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

void Journal::record(JournalLevel level, JournalEvent event) {
  if (entries_.size() == 500) {
    entries_.pop_front();
  }
  entries_.push_back(JournalEntry{
      QDateTime::currentSecsSinceEpoch(),
      level,
      event,
  });
}

QVector<JournalEntry> Journal::entries() const {
  return QVector<JournalEntry>(entries_.begin(), entries_.end());
}

QString Journal::eventCode(JournalEvent event) {
  switch (event) {
    case JournalEvent::ScanCompleted:
      return QStringLiteral("scan_completed");
    case JournalEvent::ScanFailed:
      return QStringLiteral("scan_failed");
    case JournalEvent::IpaValidated:
      return QStringLiteral("ipa_validated");
    case JournalEvent::IpaRejected:
      return QStringLiteral("ipa_rejected");
    case JournalEvent::InstallStarted:
      return QStringLiteral("install_started");
    case JournalEvent::InstallVerified:
      return QStringLiteral("install_verified");
    case JournalEvent::InstallFailed:
      return QStringLiteral("install_failed");
    case JournalEvent::InstallUnverified:
      return QStringLiteral("install_unverified");
    case JournalEvent::DraftCompiled:
      return QStringLiteral("draft_compiled");
    case JournalEvent::BackupStarted:
      return QStringLiteral("backup_started");
    case JournalEvent::BackupFinished:
      return QStringLiteral("backup_finished");
    case JournalEvent::BackupFailed:
      return QStringLiteral("backup_failed");
  }
  return QStringLiteral("unknown");
}

QString Journal::levelCode(JournalLevel level) {
  switch (level) {
    case JournalLevel::Info:
      return QStringLiteral("info");
    case JournalLevel::Warning:
      return QStringLiteral("warning");
    case JournalLevel::Error:
      return QStringLiteral("error");
  }
  return QStringLiteral("info");
}

QString Journal::exportJson() const {
  QJsonArray events;
  for (const auto& entry : entries_) {
    events.append(QJsonObject{
        {QStringLiteral("timestamp"), entry.timestamp},
        {QStringLiteral("level"), levelCode(entry.level)},
        {QStringLiteral("event"), eventCode(entry.event)},
    });
  }
  QJsonObject root{
      {QStringLiteral("schema"), 1},
      {QStringLiteral("version"), QStringLiteral("0.1.0")},
      {QStringLiteral("privacy"), QStringLiteral("allowlisted-events-only")},
      {QStringLiteral("events"), events},
  };
  return QString::fromUtf8(QJsonDocument(root).toJson(QJsonDocument::Indented));
}
