#pragma once

#include <QString>
#include <QVector>
#include <deque>

enum class JournalLevel { Info, Warning, Error };

enum class JournalEvent {
  ScanCompleted,
  ScanFailed,
  IpaValidated,
  IpaRejected,
  InstallStarted,
  InstallVerified,
  InstallFailed,
  InstallUnverified,
  DraftCompiled,
  BackupStarted,
  BackupFinished,
  BackupFailed,
  ChannelPlanStarted,
  ChannelPlanFinished,
  ChannelPlanFailed,
  ChannelMergeStarted,
  ChannelMergeFinished,
  ChannelMergeFailed,
  ChannelRestoreStarted,
  ChannelRestoreFinished,
  ChannelRestoreFailed,
  ChannelEffectStarted,
  ChannelEffectFinished,
  ChannelEffectFailed,
  ChannelVerifyFinished,
  ChannelVerifyFailed,
};

struct JournalEntry {
  qint64 timestamp = 0;
  JournalLevel level = JournalLevel::Info;
  JournalEvent event = JournalEvent::ScanCompleted;
};

class Journal {
 public:
  void record(JournalLevel level, JournalEvent event);
  [[nodiscard]] QVector<JournalEntry> entries() const;
  [[nodiscard]] QString exportJson() const;
  [[nodiscard]] static QString eventCode(JournalEvent event);
  [[nodiscard]] static QString levelCode(JournalLevel level);

 private:
  std::deque<JournalEntry> entries_;
};
