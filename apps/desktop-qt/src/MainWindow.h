#pragma once

#include "ApplyChannel.h"
#include "DeviceScanner.h"
#include "IpaInstaller.h"
#include "Journal.h"
#include "TweakCatalog.h"
#include "TweakCompiler.h"

#include <QMainWindow>
#include <QMap>
#include <QProcess>
#include <optional>

class QCheckBox;
class QComboBox;
class QLabel;
class QLineEdit;
class QListWidget;
class QListWidgetItem;
class QProcess;
class QPushButton;
class QTabWidget;
class QTextEdit;

class MainWindow final : public QMainWindow {
  Q_OBJECT

 public:
  explicit MainWindow(QWidget* parent = nullptr);
  ~MainWindow() override;

 private slots:
  void onScan();
  void onLocaleChanged(int index);
  void onThemeChanged(int index);
  void onExportJournal();
  void onChooseIpa();
  void onInstallIpa();
  void onDeviceSelectionChanged();
  void onTweakFilterChanged(const QString& text);
  void onCandidatesOnlyToggled(bool checked);
  void onCompileDraft();
  void onTweakItemChanged(QListWidgetItem* item);
  void onApplyDetect();
  void onChooseBackupDir();
  void onStartBackup();
  void onBackupFinished(int exitCode, QProcess::ExitStatus status);

 private:
  enum class Locale { Ru, En };
  enum class Theme { Dark, Light };

  [[nodiscard]] QString trRuEn(const QString& ru, const QString& en) const;
  void applyTheme();
  void refreshChrome();
  void renderScan();
  void renderJournal();
  void renderIpa();
  void renderTweaks();
  void clearIpa();
  [[nodiscard]] std::optional<QString> selectedUdid() const;
  [[nodiscard]] TweakDevice currentTweakDevice() const;

  Locale locale_ = Locale::Ru;
  Theme theme_ = Theme::Dark;
  DeviceScan scan_{};
  Journal journal_;
  std::optional<ValidatedIpa> ipa_;
  QMap<QString, TweakValue> draft_;
  bool candidatesOnly_ = false;
  QString backupDir_;
  QString backupUdid_;

  QLabel* brand_ = nullptr;
  QLabel* subtitle_ = nullptr;
  QComboBox* localeBox_ = nullptr;
  QComboBox* themeBox_ = nullptr;
  QTabWidget* tabs_ = nullptr;

  QPushButton* scanButton_ = nullptr;
  QLabel* statusLabel_ = nullptr;
  QListWidget* deviceList_ = nullptr;
  QLabel* ipaLabel_ = nullptr;
  QPushButton* chooseIpaButton_ = nullptr;
  QPushButton* installButton_ = nullptr;

  QLabel* tweaksMeta_ = nullptr;
  QLineEdit* tweakFilter_ = nullptr;
  QCheckBox* candidatesOnlyBox_ = nullptr;
  QListWidget* tweakList_ = nullptr;
  QPushButton* compileButton_ = nullptr;
  QTextEdit* draftPreview_ = nullptr;

  QLabel* applyStatus_ = nullptr;
  QPushButton* applyDetectButton_ = nullptr;
  QLineEdit* backupPathEdit_ = nullptr;
  QPushButton* chooseBackupDirButton_ = nullptr;
  QCheckBox* backupConsentBox_ = nullptr;
  QPushButton* backupButton_ = nullptr;
  QTextEdit* applyLog_ = nullptr;
  QProcess* backupProcess_ = nullptr;

  QLabel* signingLabel_ = nullptr;

  QTextEdit* journalView_ = nullptr;
  QPushButton* exportButton_ = nullptr;
};
