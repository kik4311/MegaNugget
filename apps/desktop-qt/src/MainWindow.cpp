#include "MainWindow.h"

#include "ApplyChannelCli.h"
#include "CliRunner.h"

#include <QCheckBox>
#include <QComboBox>
#include <QFile>
#include <QDir>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMessageBox>
#include <QPushButton>
#include <QSettings>
#include <QStandardPaths>
#include <QTabWidget>
#include <QTemporaryFile>
#include <QTextEdit>
#include <QVBoxLayout>
#include <QWidget>

MainWindow::MainWindow(QWidget* parent) : QMainWindow(parent) {
  setWindowTitle(QStringLiteral("MegaNugget"));
  resize(1000, 760);

  QSettings settings(QStringLiteral("MegaNugget"), QStringLiteral("DesktopQt"));
  locale_ = settings.value(QStringLiteral("locale"), QStringLiteral("ru")) ==
                    QLatin1String("en")
                ? Locale::En
                : Locale::Ru;
  theme_ = settings.value(QStringLiteral("theme"), QStringLiteral("dark")) ==
                   QLatin1String("light")
               ? Theme::Light
               : Theme::Dark;

  auto* central = new QWidget(this);
  setCentralWidget(central);
  auto* root = new QVBoxLayout(central);
  root->setContentsMargins(20, 20, 20, 20);
  root->setSpacing(12);

  auto* header = new QHBoxLayout();
  brand_ = new QLabel(QStringLiteral("MegaNugget"), this);
  QFont brandFont = brand_->font();
  brandFont.setPointSize(22);
  brandFont.setBold(true);
  brand_->setFont(brandFont);
  subtitle_ = new QLabel(this);
  subtitle_->setWordWrap(true);
  auto* headerText = new QVBoxLayout();
  headerText->addWidget(brand_);
  headerText->addWidget(subtitle_);
  header->addLayout(headerText, 1);
  localeBox_ = new QComboBox(this);
  localeBox_->addItem(QStringLiteral("RU"), QStringLiteral("ru"));
  localeBox_->addItem(QStringLiteral("EN"), QStringLiteral("en"));
  localeBox_->setCurrentIndex(locale_ == Locale::En ? 1 : 0);
  themeBox_ = new QComboBox(this);
  themeBox_->addItem(QString(), QStringLiteral("dark"));
  themeBox_->addItem(QString(), QStringLiteral("light"));
  themeBox_->setCurrentIndex(theme_ == Theme::Light ? 1 : 0);
  header->addWidget(localeBox_, 0, Qt::AlignTop);
  header->addWidget(themeBox_, 0, Qt::AlignTop);
  root->addLayout(header);

  tabs_ = new QTabWidget(this);
  root->addWidget(tabs_, 1);

  auto* devicesPage = new QWidget(this);
  auto* devicesLayout = new QVBoxLayout(devicesPage);
  auto* actions = new QHBoxLayout();
  scanButton_ = new QPushButton(this);
  exportButton_ = new QPushButton(this);
  actions->addWidget(scanButton_);
  actions->addWidget(exportButton_);
  actions->addStretch(1);
  devicesLayout->addLayout(actions);
  statusLabel_ = new QLabel(this);
  statusLabel_->setWordWrap(true);
  devicesLayout->addWidget(statusLabel_);
  deviceList_ = new QListWidget(this);
  devicesLayout->addWidget(deviceList_, 2);
  ipaLabel_ = new QLabel(this);
  ipaLabel_->setWordWrap(true);
  devicesLayout->addWidget(ipaLabel_);
  auto* ipaActions = new QHBoxLayout();
  chooseIpaButton_ = new QPushButton(this);
  installButton_ = new QPushButton(this);
  installButton_->setEnabled(false);
  ipaActions->addWidget(chooseIpaButton_);
  ipaActions->addWidget(installButton_);
  ipaActions->addStretch(1);
  devicesLayout->addLayout(ipaActions);
  journalView_ = new QTextEdit(this);
  journalView_->setReadOnly(true);
  journalView_->setMinimumHeight(110);
  devicesLayout->addWidget(journalView_, 1);
  tabs_->addTab(devicesPage, QStringLiteral("Devices"));

  auto* tweaksPage = new QWidget(this);
  auto* tweaksLayout = new QVBoxLayout(tweaksPage);
  tweaksMeta_ = new QLabel(this);
  tweaksMeta_->setWordWrap(true);
  tweaksLayout->addWidget(tweaksMeta_);
  auto* filterRow = new QHBoxLayout();
  tweakFilter_ = new QLineEdit(this);
  candidatesOnlyBox_ = new QCheckBox(this);
  filterRow->addWidget(tweakFilter_, 1);
  filterRow->addWidget(candidatesOnlyBox_);
  tweaksLayout->addLayout(filterRow);
  tweakList_ = new QListWidget(this);
  tweaksLayout->addWidget(tweakList_, 3);
  compileButton_ = new QPushButton(this);
  tweaksLayout->addWidget(compileButton_);
  draftPreview_ = new QTextEdit(this);
  draftPreview_->setReadOnly(true);
  tweaksLayout->addWidget(draftPreview_, 2);
  tabs_->addTab(tweaksPage, QStringLiteral("Tweaks"));

  auto* applyPage = new QWidget(this);
  auto* applyLayout = new QVBoxLayout(applyPage);
  applyStatus_ = new QLabel(this);
  applyStatus_->setWordWrap(true);
  applyLayout->addWidget(applyStatus_);
  applyDetectButton_ = new QPushButton(this);
  applyLayout->addWidget(applyDetectButton_);
  auto* backupRow = new QHBoxLayout();
  backupPathEdit_ = new QLineEdit(this);
  backupPathEdit_->setReadOnly(true);
  chooseBackupDirButton_ = new QPushButton(this);
  backupRow->addWidget(backupPathEdit_, 1);
  backupRow->addWidget(chooseBackupDirButton_);
  applyLayout->addLayout(backupRow);
  backupConsentBox_ = new QCheckBox(this);
  applyLayout->addWidget(backupConsentBox_);
  backupButton_ = new QPushButton(this);
  applyLayout->addWidget(backupButton_);

  auto* verifiedRow = new QHBoxLayout();
  verifiedBackupEdit_ = new QLineEdit(this);
  verifiedBackupEdit_->setReadOnly(true);
  chooseVerifiedBackupButton_ = new QPushButton(this);
  verifyBackupButton_ = new QPushButton(this);
  verifiedRow->addWidget(verifiedBackupEdit_, 1);
  verifiedRow->addWidget(chooseVerifiedBackupButton_);
  verifiedRow->addWidget(verifyBackupButton_);
  applyLayout->addLayout(verifiedRow);
  planButton_ = new QPushButton(this);
  applyLayout->addWidget(planButton_);

  auto* appliedRow = new QHBoxLayout();
  appliedDirEdit_ = new QLineEdit(this);
  appliedDirEdit_->setReadOnly(true);
  chooseAppliedDirButton_ = new QPushButton(this);
  mergeButton_ = new QPushButton(this);
  appliedRow->addWidget(appliedDirEdit_, 1);
  appliedRow->addWidget(chooseAppliedDirButton_);
  appliedRow->addWidget(mergeButton_);
  applyLayout->addLayout(appliedRow);

  restoreConsentBox_ = new QCheckBox(this);
  applyLayout->addWidget(restoreConsentBox_);
  restoreButton_ = new QPushButton(this);
  applyLayout->addWidget(restoreButton_);

  auto* postRow = new QHBoxLayout();
  postBackupEdit_ = new QLineEdit(this);
  postBackupEdit_->setReadOnly(true);
  choosePostBackupButton_ = new QPushButton(this);
  effectButton_ = new QPushButton(this);
  postRow->addWidget(postBackupEdit_, 1);
  postRow->addWidget(choosePostBackupButton_);
  postRow->addWidget(effectButton_);
  applyLayout->addLayout(postRow);

  applyLog_ = new QTextEdit(this);
  applyLog_->setReadOnly(true);
  applyLayout->addWidget(applyLog_, 1);
  tabs_->addTab(applyPage, QStringLiteral("Apply"));

  auto* signingPage = new QWidget(this);
  auto* signingLayout = new QVBoxLayout(signingPage);
  signingLabel_ = new QLabel(this);
  signingLabel_->setWordWrap(true);
  signingLayout->addWidget(signingLabel_);
  signingLayout->addStretch(1);
  tabs_->addTab(signingPage, QStringLiteral("Signing"));

  connect(scanButton_, &QPushButton::clicked, this, &MainWindow::onScan);
  connect(exportButton_, &QPushButton::clicked, this, &MainWindow::onExportJournal);
  connect(chooseIpaButton_, &QPushButton::clicked, this, &MainWindow::onChooseIpa);
  connect(installButton_, &QPushButton::clicked, this, &MainWindow::onInstallIpa);
  connect(deviceList_, &QListWidget::currentRowChanged, this,
          &MainWindow::onDeviceSelectionChanged);
  connect(localeBox_, &QComboBox::currentIndexChanged, this, &MainWindow::onLocaleChanged);
  connect(themeBox_, &QComboBox::currentIndexChanged, this, &MainWindow::onThemeChanged);
  connect(tweakFilter_, &QLineEdit::textChanged, this, &MainWindow::onTweakFilterChanged);
  connect(candidatesOnlyBox_, &QCheckBox::toggled, this, &MainWindow::onCandidatesOnlyToggled);
  connect(compileButton_, &QPushButton::clicked, this, &MainWindow::onCompileDraft);
  connect(tweakList_, &QListWidget::itemChanged, this, &MainWindow::onTweakItemChanged);
  connect(applyDetectButton_, &QPushButton::clicked, this, &MainWindow::onApplyDetect);
  connect(chooseBackupDirButton_, &QPushButton::clicked, this, &MainWindow::onChooseBackupDir);
  connect(backupButton_, &QPushButton::clicked, this, &MainWindow::onStartBackup);
  connect(chooseVerifiedBackupButton_, &QPushButton::clicked, this,
          &MainWindow::onChooseVerifiedBackup);
  connect(verifyBackupButton_, &QPushButton::clicked, this, &MainWindow::onVerifyVerifiedBackup);
  connect(planButton_, &QPushButton::clicked, this, &MainWindow::onRunPlan);
  connect(chooseAppliedDirButton_, &QPushButton::clicked, this, &MainWindow::onChooseAppliedDir);
  connect(mergeButton_, &QPushButton::clicked, this, &MainWindow::onRunMerge);
  connect(restoreButton_, &QPushButton::clicked, this, &MainWindow::onRunRestore);
  connect(choosePostBackupButton_, &QPushButton::clicked, this, &MainWindow::onChoosePostBackup);
  connect(effectButton_, &QPushButton::clicked, this, &MainWindow::onRunEffect);

  QString catalogError;
  if (!TweakCatalog::instance().load(&catalogError)) {
    tweaksMeta_->setText(QStringLiteral("catalog error: %1").arg(catalogError));
  }

  applyTheme();
  refreshChrome();
  statusLabel_->setText(
      trRuEn(QStringLiteral("Подключите iPhone по USB и нажмите «Найти iPhone»."),
             QStringLiteral("Connect an iPhone over USB and press Find iPhone.")));
  renderTweaks();
}

MainWindow::~MainWindow() {
  if (backupProcess_) {
    backupProcess_->kill();
    backupProcess_->waitForFinished(3000);
  }
  stopRestoreProcess();
  clearIpa();
}

void MainWindow::clearIpa() {
  if (ipa_) {
    QFile::remove(ipa_->snapshotPath);
    ipa_.reset();
  }
}

QString MainWindow::trRuEn(const QString& ru, const QString& en) const {
  return locale_ == Locale::Ru ? ru : en;
}

void MainWindow::applyTheme() {
  if (theme_ == Theme::Dark) {
    setStyleSheet(QStringLiteral(
        "QWidget { background: #101014; color: #eeedf4; }"
        "QComboBox, QPushButton, QListWidget, QTextEdit, QLineEdit, QTabWidget::pane {"
        "  background: #19191f; color: #eeedf4; border: 1px solid #2c2b35;"
        "  border-radius: 8px; padding: 8px; }"
        "QPushButton { background: #292238; }"
        "QPushButton:hover { background: #3a3150; }"
        "QPushButton:disabled { color: #96949f; background: #1a1a20; }"
        "QListWidget::item { padding: 8px; }"
        "QListWidget::item:selected { background: #3a3150; }"
        "QTabBar::tab { background: #19191f; padding: 10px 16px; margin-right: 4px; }"
        "QTabBar::tab:selected { background: #292238; }"
        "QLabel { background: transparent; }"));
  } else {
    setStyleSheet(QStringLiteral(
        "QWidget { background: #f5f4f9; color: #22202c; }"
        "QComboBox, QPushButton, QListWidget, QTextEdit, QLineEdit, QTabWidget::pane {"
        "  background: #ffffff; color: #22202c; border: 1px solid #dfdce7;"
        "  border-radius: 8px; padding: 8px; }"
        "QPushButton { background: #eee7ff; }"
        "QPushButton:hover { background: #e0d6ff; }"
        "QPushButton:disabled { color: #706a7d; background: #f0eef6; }"
        "QListWidget::item { padding: 8px; }"
        "QListWidget::item:selected { background: #e0d6ff; }"
        "QTabBar::tab { background: #ffffff; padding: 10px 16px; margin-right: 4px; }"
        "QTabBar::tab:selected { background: #eee7ff; }"
        "QLabel { background: transparent; }"));
  }
}

void MainWindow::refreshChrome() {
  subtitle_->setText(trRuEn(
      QStringLiteral(
          "Твики для старых iPhone без джейлбрейка."),
      QStringLiteral(
          "Tweaks for older iPhones without a jailbreak.")));
  themeBox_->setItemText(0, trRuEn(QStringLiteral("Тёмная"), QStringLiteral("Dark")));
  themeBox_->setItemText(1, trRuEn(QStringLiteral("Светлая"), QStringLiteral("Light")));
  tabs_->setTabText(0, trRuEn(QStringLiteral("Устройства"), QStringLiteral("Devices")));
  tabs_->setTabText(1, trRuEn(QStringLiteral("Твики"), QStringLiteral("Tweaks")));
  tabs_->setTabText(2, trRuEn(QStringLiteral("Применение"), QStringLiteral("Apply")));
  tabs_->setTabText(3, trRuEn(QStringLiteral("Подпись"), QStringLiteral("Signing")));
  scanButton_->setText(trRuEn(QStringLiteral("Найти iPhone"), QStringLiteral("Find iPhone")));
  exportButton_->setText(
      trRuEn(QStringLiteral("Экспорт журнала"), QStringLiteral("Export journal")));
  chooseIpaButton_->setText(
      trRuEn(QStringLiteral("Выбрать IPA"), QStringLiteral("Choose IPA")));
  installButton_->setText(trRuEn(QStringLiteral("Установить"), QStringLiteral("Install")));
  tweakFilter_->setPlaceholderText(
      trRuEn(QStringLiteral("Поиск твика"), QStringLiteral("Search tweaks")));
  candidatesOnlyBox_->setText(
      trRuEn(QStringLiteral("Только кандидаты"), QStringLiteral("Candidates only")));
  compileButton_->setText(
      trRuEn(QStringLiteral("Собрать черновик"), QStringLiteral("Build draft")));
  applyDetectButton_->setText(
      trRuEn(QStringLiteral("Проверить iPhone"), QStringLiteral("Check iPhone")));
  chooseBackupDirButton_->setText(
      trRuEn(QStringLiteral("Папка для копии"), QStringLiteral("Backup folder")));
  backupConsentBox_->setText(trRuEn(
      QStringLiteral("Я согласен создать полную резервную копию iPhone на этом компьютере"),
      QStringLiteral("I agree to create a full backup of the iPhone on this computer")));
  backupButton_->setText(
      trRuEn(QStringLiteral("Создать резервную копию"), QStringLiteral("Create backup")));
  applyStatus_->setText(trRuEn(
      QStringLiteral(
          "Для iPhone 7 на iOS 15.8.8. Шаги: проверка копии, план, добавление твика в копию, "
          "восстановление на iPhone (с отдельным согласием) и проверка результата."),
      QStringLiteral(
          "For iPhone 7 on iOS 15.8.8. Steps: check the backup, plan, add the tweak to the copy, "
          "restore to the iPhone (with a separate consent) and check the result.")));
  chooseVerifiedBackupButton_->setText(
      trRuEn(QStringLiteral("Выбрать копию"), QStringLiteral("Choose backup")));
  verifyBackupButton_->setText(trRuEn(QStringLiteral("Проверить копию"), QStringLiteral("Check backup")));
  planButton_->setText(
      trRuEn(QStringLiteral("План: без предупреждений о низком заряде"),
             QStringLiteral("Plan: no low battery alerts")));
  chooseAppliedDirButton_->setText(
      trRuEn(QStringLiteral("Папка для новой копии"), QStringLiteral("Folder for new copy")));
  mergeButton_->setText(trRuEn(QStringLiteral("Добавить твик в копию"), QStringLiteral("Add tweak to copy")));
  restoreConsentBox_->setText(trRuEn(
      QStringLiteral(
          "Я согласен восстановить копию на iPhone. Телефон перезагрузится."),
      QStringLiteral(
          "I agree to restore the copy to the iPhone. The phone will reboot.")));
  restoreButton_->setText(
      trRuEn(QStringLiteral("Восстановить на iPhone"), QStringLiteral("Restore to iPhone")));
  choosePostBackupButton_->setText(
      trRuEn(QStringLiteral("Копия после восстановления"), QStringLiteral("Backup after restore")));
  effectButton_->setText(trRuEn(QStringLiteral("Проверить результат"), QStringLiteral("Check result")));
  signingLabel_->setText(trRuEn(
      QStringLiteral(
          "Подпись через Apple Account в этой версии пока не работает: она есть только "
          "в версии на Tauri. Здесь ничего не отправляется в сеть и пароль не запрашивается."),
      QStringLiteral(
          "Signing with an Apple Account does not work in this version yet: it is only in "
          "the Tauri version. Nothing is sent over the network and no password is asked.")));
  renderScan();
  renderIpa();
  renderTweaks();
  renderJournal();
}

TweakDevice MainWindow::currentTweakDevice() const {
  TweakDevice device;
  device.model = QStringLiteral("iPhone9,3");
  device.systemVersion = QStringLiteral("15.8.8");
  device.architecture = QStringLiteral("arm64");
  if (const auto udid = selectedUdid()) {
    for (const auto& row : scan_.devices) {
      if (row.id == *udid && row.info) {
        device.model = row.info->product;
        device.systemVersion = row.info->ios;
        device.architecture = row.info->architecture;
        break;
      }
    }
  } else if (!scan_.devices.isEmpty() && scan_.devices.first().info) {
    device.model = scan_.devices.first().info->product;
    device.systemVersion = scan_.devices.first().info->ios;
    device.architecture = scan_.devices.first().info->architecture;
  }
  return device;
}

void MainWindow::onLocaleChanged(int index) {
  locale_ = localeBox_->itemData(index).toString() == QLatin1String("en") ? Locale::En
                                                                          : Locale::Ru;
  QSettings settings(QStringLiteral("MegaNugget"), QStringLiteral("DesktopQt"));
  settings.setValue(QStringLiteral("locale"),
                    locale_ == Locale::En ? QStringLiteral("en") : QStringLiteral("ru"));
  refreshChrome();
}

void MainWindow::onThemeChanged(int index) {
  theme_ = themeBox_->itemData(index).toString() == QLatin1String("light") ? Theme::Light
                                                                           : Theme::Dark;
  QSettings settings(QStringLiteral("MegaNugget"), QStringLiteral("DesktopQt"));
  settings.setValue(QStringLiteral("theme"),
                    theme_ == Theme::Light ? QStringLiteral("light") : QStringLiteral("dark"));
  applyTheme();
  refreshChrome();
}

void MainWindow::onScan() {
  scanButton_->setEnabled(false);
  statusLabel_->setText(trRuEn(QStringLiteral("Сканирование…"), QStringLiteral("Scanning…")));
  scan_ = DeviceScanner::scan();
  journal_.record(scan_.failure ? JournalLevel::Error : JournalLevel::Info,
                  scan_.failure ? JournalEvent::ScanFailed : JournalEvent::ScanCompleted);
  renderScan();
  renderIpa();
  renderTweaks();
  renderJournal();
  scanButton_->setEnabled(true);
}

void MainWindow::onExportJournal() {
  const QString path = QFileDialog::getSaveFileName(
      this, trRuEn(QStringLiteral("Сохранить журнал"), QStringLiteral("Save journal")),
      QStringLiteral("meganugget-journal.json"), QStringLiteral("JSON (*.json)"));
  if (path.isEmpty()) {
    return;
  }
  QFile file(path);
  if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
    QMessageBox::warning(this, QStringLiteral("MegaNugget"),
                         trRuEn(QStringLiteral("Не удалось сохранить файл."),
                                QStringLiteral("Could not save the file.")));
    return;
  }
  file.write(journal_.exportJson().toUtf8());
}

void MainWindow::onChooseIpa() {
  const QString path = QFileDialog::getOpenFileName(
      this, trRuEn(QStringLiteral("Выбрать IPA"), QStringLiteral("Choose IPA")), QString(),
      QStringLiteral("IPA (*.ipa)"));
  if (path.isEmpty()) {
    return;
  }
  clearIpa();
  InstallError error = InstallError::InvalidIpa;
  auto validated = IpaInstaller::inspect(path, &error);
  if (!validated) {
    journal_.record(JournalLevel::Error, JournalEvent::IpaRejected);
    ipaLabel_->setText(trRuEn(QStringLiteral("IPA отклонён: %1").arg(IpaInstaller::errorCode(error)),
                              QStringLiteral("IPA rejected: %1")
                                  .arg(IpaInstaller::errorCode(error))));
    renderJournal();
    renderIpa();
    return;
  }
  ipa_ = std::move(validated);
  journal_.record(JournalLevel::Info, JournalEvent::IpaValidated);
  renderIpa();
  renderJournal();
}

void MainWindow::onInstallIpa() {
  const auto udid = selectedUdid();
  if (!ipa_ || !udid) {
    return;
  }
  const auto reply = QMessageBox::question(
      this, QStringLiteral("MegaNugget"),
      trRuEn(QStringLiteral("Установить %1 (версия %2) на iPhone %3?")
                 .arg(ipa_->info.bundleId, ipa_->info.version, DeviceScanner::maskUdid(*udid)),
             QStringLiteral("Install %1 (version %2) on iPhone %3?")
                 .arg(ipa_->info.bundleId, ipa_->info.version, DeviceScanner::maskUdid(*udid))));
  if (reply != QMessageBox::Yes) {
    return;
  }
  installButton_->setEnabled(false);
  journal_.record(JournalLevel::Info, JournalEvent::InstallStarted);
  InstallError error = InstallError::ToolFailure;
  const auto result = IpaInstaller::install(*ipa_, *udid, &error);
  if (!result) {
    journal_.record(error == InstallError::VerificationFailed ? JournalLevel::Warning
                                                              : JournalLevel::Error,
                    error == InstallError::VerificationFailed ? JournalEvent::InstallUnverified
                                                              : JournalEvent::InstallFailed);
    statusLabel_->setText(trRuEn(
        QStringLiteral("Установка: %1").arg(IpaInstaller::errorCode(error)),
        QStringLiteral("Install: %1").arg(IpaInstaller::errorCode(error))));
  } else {
    journal_.record(JournalLevel::Info, JournalEvent::InstallVerified);
    statusLabel_->setText(
        trRuEn(QStringLiteral("Установка подтверждена."), QStringLiteral("Install verified.")));
  }
  renderIpa();
  renderJournal();
}

void MainWindow::onDeviceSelectionChanged() {
  renderIpa();
  renderTweaks();
}

void MainWindow::onTweakFilterChanged(const QString&) {
  renderTweaks();
}

void MainWindow::onCandidatesOnlyToggled(bool checked) {
  candidatesOnly_ = checked;
  renderTweaks();
}

void MainWindow::onTweakItemChanged(QListWidgetItem* item) {
  if (!item) {
    return;
  }
  const QString id = item->data(Qt::UserRole).toString();
  if (item->checkState() == Qt::Checked) {
    const auto* spec = TweakCatalog::instance().spec(id);
    if (!spec) {
      return;
    }
    TweakValue value = spec->value;
    if (spec->kind == TweakKind::Toggle) {
      value.type = TweakValue::Type::Bool;
      value.boolValue = true;
    }
    draft_.insert(id, value);
  } else {
    draft_.remove(id);
  }
}

void MainWindow::onCompileDraft() {
  DraftError error = DraftError::EmptySelection;
  QString detail;
  const auto preview =
      TweakCompiler::preview(draft_, currentTweakDevice(), &error, &detail);
  if (!preview) {
    draftPreview_->setPlainText(
        trRuEn(QStringLiteral("Ошибка: %1 %2")
                   .arg(TweakCompiler::errorCode(error), detail),
               QStringLiteral("Error: %1 %2")
                   .arg(TweakCompiler::errorCode(error), detail)));
    return;
  }
  journal_.record(JournalLevel::Info, JournalEvent::DraftCompiled);
  QString text = trRuEn(QStringLiteral("Файлов: %1 · выбрано твиков: %2 · на iPhone не применено\n\n")
                            .arg(preview->files.size())
                            .arg(preview->selectedCount),
                        QStringLiteral("Files: %1 · tweaks selected: %2 · not applied to the iPhone\n\n")
                            .arg(preview->files.size())
                            .arg(preview->selectedCount));
  for (const auto& file : preview->files) {
    text += QStringLiteral("## %1 / %2\n%3\n")
                .arg(file.domain, file.relativePath, file.xml);
  }
  draftPreview_->setPlainText(text);
  renderJournal();
}

std::optional<QString> MainWindow::selectedUdid() const {
  const int row = deviceList_->currentRow();
  if (row < 0 || row >= scan_.devices.size()) {
    return std::nullopt;
  }
  return scan_.devices.at(row).id;
}

void MainWindow::renderScan() {
  const int previous = deviceList_->currentRow();
  deviceList_->clear();
  if (scan_.failure) {
    statusLabel_->setText(trRuEn(
        QStringLiteral("Поиск не удался: %1").arg(CliRunner::failureCode(*scan_.failure)),
        QStringLiteral("Scan failed: %1").arg(CliRunner::failureCode(*scan_.failure))));
    return;
  }
  if (scan_.devices.isEmpty()) {
    statusLabel_->setText(trRuEn(
        QStringLiteral("iPhone не найден. Проверьте USB-кабель."),
        QStringLiteral("No iPhone found. Check the USB cable.")));
    return;
  }
  statusLabel_->setText(trRuEn(QStringLiteral("Найдено iPhone: %1").arg(scan_.devices.size()),
                               QStringLiteral("Devices found: %1").arg(scan_.devices.size())));
  for (const auto& device : scan_.devices) {
    if (!device.info) {
      deviceList_->addItem(QStringLiteral("%1 · unknown").arg(device.idMasked));
      continue;
    }
    QString line = QStringLiteral("%1 · %2 · iOS %3 (%4)")
                       .arg(device.idMasked, device.info->product, device.info->ios,
                            device.info->build);
    if (device.compatibility.matchesTarget) {
      line += trRuEn(QStringLiteral(" · поддерживается"), QStringLiteral(" · supported"));
    }
    deviceList_->addItem(line);
  }
  if (previous >= 0 && previous < deviceList_->count()) {
    deviceList_->setCurrentRow(previous);
  } else if (deviceList_->count() == 1) {
    deviceList_->setCurrentRow(0);
  }
}

void MainWindow::renderIpa() {
  if (!ipa_) {
    ipaLabel_->setText(trRuEn(
        QStringLiteral("Файл приложения не выбран. Выберите подписанный файл MegaNugget (.ipa)."),
        QStringLiteral("No app file selected. Choose a signed MegaNugget file (.ipa).")));
    installButton_->setEnabled(false);
    return;
  }
  ipaLabel_->setText(QStringLiteral("IPA: %1 · build %2 · %3")
                         .arg(ipa_->info.bundleId, ipa_->info.version, ipa_->info.signing));
  installButton_->setEnabled(selectedUdid().has_value());
}

void MainWindow::renderTweaks() {
  const TweakDevice device = currentTweakDevice();
  const auto counts = TweakCatalog::instance().counts(device);
  tweaksMeta_->setText(
      trRuEn(QStringLiteral("Для %1 на iOS %2 · кандидатов %3 из %4 · версия каталога %5")
                 .arg(device.model, device.systemVersion)
                 .arg(counts.candidates)
                 .arg(counts.total)
                 .arg(TweakCatalog::instance().revision().left(12)),
             QStringLiteral("For %1 on iOS %2 · candidates %3 of %4 · catalog version %5")
                 .arg(device.model, device.systemVersion)
                 .arg(counts.candidates)
                 .arg(counts.total)
                 .arg(TweakCatalog::instance().revision().left(12))));

  const QString filter = tweakFilter_->text().trimmed();
  tweakList_->blockSignals(true);
  tweakList_->clear();
  for (const auto& summary : TweakCatalog::instance().summarize(device)) {
    if (candidatesOnly_ && summary.status != TweakStatus::DraftCandidate) {
      continue;
    }
    if (!filter.isEmpty() &&
        !summary.title.contains(filter, Qt::CaseInsensitive) &&
        !summary.id.contains(filter, Qt::CaseInsensitive)) {
      continue;
    }
    auto* item = new QListWidgetItem(
        QStringLiteral("%1 · %2 · %3")
            .arg(summary.title, summary.id, TweakCatalog::statusCode(summary.status)),
        tweakList_);
    item->setData(Qt::UserRole, summary.id);
    const bool candidate = summary.status == TweakStatus::DraftCandidate;
    item->setFlags(candidate ? (item->flags() | Qt::ItemIsUserCheckable)
                             : (item->flags() & ~Qt::ItemIsUserCheckable));
    if (candidate) {
      item->setCheckState(draft_.contains(summary.id) ? Qt::Checked : Qt::Unchecked);
    }
  }
  tweakList_->blockSignals(false);
}

void MainWindow::renderJournal() {
  QStringList lines;
  for (const auto& entry : journal_.entries()) {
    lines << QStringLiteral("%1 %2 %3")
                 .arg(entry.timestamp)
                 .arg(Journal::levelCode(entry.level), Journal::eventCode(entry.event));
  }
  journalView_->setPlainText(lines.join(QLatin1Char('\n')));
}

void MainWindow::onApplyDetect() {
  const ApplyDetection detection = ApplyChannel::detect();
  QStringList lines;
  if (detection.devices.isEmpty()) {
    lines << trRuEn(QStringLiteral("iPhone не найден или произошла ошибка: %1")
                        .arg(detection.failures.join(QLatin1Char(','))),
                    QStringLiteral("No iPhone found, or an error occurred: %1")
                        .arg(detection.failures.join(QLatin1Char(','))));
  }
  for (const auto& device : detection.devices) {
    lines << QStringLiteral("%1 · %2 · iOS %3 · %4")
                 .arg(device.udidMasked,
                      device.product.isEmpty() ? QStringLiteral("?") : device.product,
                      device.ios.isEmpty() ? QStringLiteral("?") : device.ios)
                 .arg(device.matchesTarget
                          ? trRuEn(QStringLiteral("поддерживается"), QStringLiteral("supported"))
                          : trRuEn(QStringLiteral("не поддерживается"),
                                   QStringLiteral("not supported")));
  }
  if (!detection.failures.isEmpty()) {
    lines << QStringLiteral("failures: %1").arg(detection.failures.join(QLatin1Char(',')));
  }
  applyLog_->setPlainText(lines.join(QLatin1Char('\n')));
}

void MainWindow::onChooseBackupDir() {
  const QString dir = QFileDialog::getExistingDirectory(
      this, trRuEn(QStringLiteral("Папка для резервной копии"), QStringLiteral("Backup folder")));
  if (dir.isEmpty()) {
    return;
  }
  backupDir_ = dir;
  backupPathEdit_->setText(dir);
}

void MainWindow::onStartBackup() {
  if (!backupConsentBox_->isChecked()) {
    applyLog_->setPlainText(ApplyChannel::errorCode(ApplyError::BackupRequiresConsent));
    return;
  }
  if (backupDir_.isEmpty()) {
    applyLog_->setPlainText(
        trRuEn(QStringLiteral("Сначала выберите папку для резервной копии."),
               QStringLiteral("Choose a backup folder first.")));
    return;
  }
  if (backupProcess_ && backupProcess_->state() != QProcess::NotRunning) {
    applyLog_->setPlainText(
        trRuEn(QStringLiteral("Резервная копия уже создаётся."),
               QStringLiteral("A backup is already in progress.")));
    return;
  }

  ApplyError error = ApplyError::DeviceNotFound;
  const auto udid = ApplyChannel::singleTargetUdid(&error);
  if (!udid) {
    applyLog_->setPlainText(ApplyChannel::errorCode(error));
    journal_.record(JournalLevel::Error, JournalEvent::BackupFailed);
    renderJournal();
    return;
  }

  QDir().mkpath(backupDir_);
  backupUdid_ = *udid;
  if (!backupProcess_) {
    backupProcess_ = new QProcess(this);
    connect(backupProcess_,
            QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
            &MainWindow::onBackupFinished);
  }
  backupProcess_->setProgram(QStringLiteral("idevicebackup2"));
  backupProcess_->setArguments(ApplyChannel::backupArgs(*udid, backupDir_));
  journal_.record(JournalLevel::Info, JournalEvent::BackupStarted);
  renderJournal();
  applyLog_->setPlainText(trRuEn(
      QStringLiteral(
          "Создаю резервную копию iPhone %1…\nЭто может занять много времени. Не отключайте кабель.")
          .arg(DeviceScanner::maskUdid(*udid)),
      QStringLiteral(
          "Creating a backup of iPhone %1…\nThis can take a long time. Keep the cable connected.")
          .arg(DeviceScanner::maskUdid(*udid))));
  backupButton_->setEnabled(false);
  backupConsentBox_->setEnabled(false);
  backupProcess_->start();
  if (!backupProcess_->waitForStarted(3000)) {
    backupButton_->setEnabled(true);
    backupConsentBox_->setEnabled(true);
    journal_.record(JournalLevel::Error, JournalEvent::BackupFailed);
    applyLog_->setPlainText(ApplyChannel::errorCode(ApplyError::BackupToolMissing));
    renderJournal();
  }
}

void MainWindow::onBackupFinished(int exitCode, QProcess::ExitStatus status) {
  backupButton_->setEnabled(true);
  backupConsentBox_->setEnabled(true);
  if (status != QProcess::NormalExit || exitCode != 0) {
    journal_.record(JournalLevel::Error, JournalEvent::BackupFailed);
    const QString out = QString::fromUtf8(backupProcess_->readAllStandardOutput()).trimmed();
    const QString err = QString::fromUtf8(backupProcess_->readAllStandardError()).trimmed();
    QString message = ApplyChannel::errorCode(ApplyError::BackupFailed);
    message += QStringLiteral("\nexit=%1").arg(exitCode);
    if (!out.isEmpty()) {
      message += QStringLiteral("\n") + out;
    }
    if (!err.isEmpty()) {
      message += QStringLiteral("\n") + err;
    }
    if (out.contains(QStringLiteral("protocol version exchange"), Qt::CaseInsensitive) ||
        err.contains(QStringLiteral("protocol version exchange"), Qt::CaseInsensitive)) {
      message += trRuEn(
          QStringLiteral(
              "\n\nПодсказка: idevicebackup2 1.3.0 из MSYS2 слишком старый для iOS 15 и новее. "
              "Нужна сборка libimobiledevice из git, версия 1.4 или новее."),
          QStringLiteral(
              "\n\nHint: MSYS2 idevicebackup2 1.3.0 is too old for iOS 15+. "
              "Build libimobiledevice from git (1.4+/master)."));
    }
    applyLog_->setPlainText(message);
    renderJournal();
    return;
  }
  ApplyError error = ApplyError::BackupFailed;
  const auto snap = ApplyChannel::verifyBackup(backupDir_, &error);
  if (!snap) {
    journal_.record(JournalLevel::Error, JournalEvent::BackupFailed);
    applyLog_->setPlainText(ApplyChannel::errorCode(error));
    renderJournal();
    return;
  }
  journal_.record(JournalLevel::Info, JournalEvent::BackupFinished);
  applyLog_->setPlainText(
      QStringLiteral("udid=%1\nproduct=%2\nios=%3\nSnapshotState=%4")
          .arg(snap->udidMasked, snap->product, snap->ios, snap->snapshotState));
  verifiedBackupDir_ = backupDir_;
  verifiedBackupEdit_->setText(backupDir_);
  appliedDir_ = ApplyChannelCli::suggestedAppliedDir(backupDir_);
  appliedDirEdit_->setText(appliedDir_);
  renderJournal();
}

void MainWindow::appendApplyLog(const QString& line) {
  const QString prev = applyLog_->toPlainText().trimmed();
  applyLog_->setPlainText(prev.isEmpty() ? line : prev + QLatin1Char('\n') + line);
}

bool MainWindow::ensureApplyChannelTool(QString* message) const {
  QString detail;
  const auto binary = ApplyChannelCli::resolveBinary(&detail);
  if (!binary) {
    if (message) {
      *message = trRuEn(
          QStringLiteral("apply-channel не найден. Соберите: cargo build -p apply-channel\n%1")
              .arg(detail),
          QStringLiteral("apply-channel not found. Build: cargo build -p apply-channel\n%1")
              .arg(detail));
    }
    return false;
  }
  return true;
}

std::optional<QString> MainWindow::writeDraftToTemp(QString* error) const {
  const QString cache =
      QStandardPaths::writableLocation(QStandardPaths::CacheLocation);
  QDir().mkpath(cache);
  const QString path = QDir(cache).filePath(QStringLiteral("sb-hide-low-power-draft.json"));
  QFile file(path);
  if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
    if (error) {
      *error = trRuEn(QStringLiteral("Не удалось сохранить черновик."),
                      QStringLiteral("Could not write draft."));
    }
    return std::nullopt;
  }
  file.write(ApplyChannelCli::hideLowPowerAlertsDraftJson().toUtf8());
  return path;
}

QString MainWindow::formatPlanJson(const QByteArray& json) const {
  QJsonParseError parseError;
  const QJsonDocument doc = QJsonDocument::fromJson(json, &parseError);
  if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
    return QString::fromUtf8(json);
  }
  QStringList lines;
  const QJsonObject root = doc.object();
  const QJsonArray files = root.value(QStringLiteral("files")).toArray();
  for (const QJsonValue& fileVal : files) {
    const QJsonObject file = fileVal.toObject();
      const QString rel = file.value(QStringLiteral("relative_path")).toString();
    const bool digest = file.value(QStringLiteral("digest_verified")).toBool();
    lines << QStringLiteral("file=%1 digest_verified=%2")
                 .arg(rel, digest ? QStringLiteral("true") : QStringLiteral("false"));
    const QJsonArray entries = file.value(QStringLiteral("entries")).toArray();
    for (const QJsonValue& entryVal : entries) {
      const QJsonObject entry = entryVal.toObject();
      const QString key = entry.value(QStringLiteral("key")).toString();
      const QString action = entry.value(QStringLiteral("action")).toString();
      const QString oldV = entry.value(QStringLiteral("old")).toVariant().toString();
      const QString newV = entry.value(QStringLiteral("new")).toString();
      lines << QStringLiteral("  key=%1 action=%2 before=%3 after=%4")
                   .arg(key, action, oldV.isEmpty() ? QStringLiteral("null") : oldV, newV);
    }
  }
  return lines.isEmpty() ? QString::fromUtf8(json) : lines.join(QLatin1Char('\n'));
}

QString MainWindow::formatEffectJson(const QByteArray& json) const {
  QJsonParseError parseError;
  const QJsonDocument doc = QJsonDocument::fromJson(json, &parseError);
  if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
    return QString::fromUtf8(json);
  }
  QStringList lines;
  const QJsonArray files = doc.object().value(QStringLiteral("files")).toArray();
  for (const QJsonValue& fileVal : files) {
    const QJsonArray keys = fileVal.toObject().value(QStringLiteral("keys")).toArray();
    for (const QJsonValue& keyVal : keys) {
      const QJsonObject key = keyVal.toObject();
      lines << QStringLiteral("%1 · %2 · status=%3 · before=%4 · after=%5")
                   .arg(key.value(QStringLiteral("id")).toString(),
                        key.value(QStringLiteral("key")).toString(),
                        key.value(QStringLiteral("status")).toString(),
                        key.value(QStringLiteral("before"))
                            .toVariant()
                            .toString()
                            .isEmpty()
                            ? QStringLiteral("null")
                            : key.value(QStringLiteral("before")).toVariant().toString(),
                        key.value(QStringLiteral("after"))
                            .toVariant()
                            .toString()
                            .isEmpty()
                            ? QStringLiteral("null")
                            : key.value(QStringLiteral("after")).toVariant().toString());
    }
  }
  return lines.isEmpty() ? QString::fromUtf8(json) : lines.join(QLatin1Char('\n'));
}

void MainWindow::stopRestoreProcess() {
  if (!restoreProcess_) {
    return;
  }
  if (restoreProcess_->state() != QProcess::NotRunning) {
    restoreProcess_->kill();
    restoreProcess_->waitForFinished(3000);
  }
}

void MainWindow::onChooseVerifiedBackup() {
  const QString dir = QFileDialog::getExistingDirectory(
      this, trRuEn(QStringLiteral("Проверенная резервная копия"), QStringLiteral("Verified backup")));
  if (dir.isEmpty()) {
    return;
  }
  verifiedBackupDir_ = dir;
  verifiedBackupEdit_->setText(dir);
  if (appliedDir_.isEmpty()) {
    appliedDir_ = ApplyChannelCli::suggestedAppliedDir(dir);
    appliedDirEdit_->setText(appliedDir_);
  }
}

void MainWindow::onVerifyVerifiedBackup() {
  if (verifiedBackupDir_.isEmpty()) {
    appendApplyLog(trRuEn(QStringLiteral("Сначала выберите папку для резервной копии."),
                          QStringLiteral("Choose a backup folder first.")));
    return;
  }
  QString message;
  if (!ensureApplyChannelTool(&message)) {
    appendApplyLog(message);
    journal_.record(JournalLevel::Error, JournalEvent::ChannelVerifyFailed);
    renderJournal();
    return;
  }
  const auto result =
      ApplyChannelCli::run({QStringLiteral("verify"), QDir::toNativeSeparators(verifiedBackupDir_)});
  if (!result.success) {
    journal_.record(JournalLevel::Error, JournalEvent::ChannelVerifyFailed);
    appendApplyLog(result.errorCode);
    renderJournal();
    return;
  }
  journal_.record(JournalLevel::Info, JournalEvent::ChannelVerifyFinished);
  appendApplyLog(QString::fromUtf8(result.stdoutJson));
  renderJournal();
}

void MainWindow::onRunPlan() {
  if (verifiedBackupDir_.isEmpty()) {
    appendApplyLog(trRuEn(QStringLiteral("Сначала выберите проверенную резервную копию."),
                          QStringLiteral("Choose a verified backup first.")));
    return;
  }
  QString toolMessage;
  if (!ensureApplyChannelTool(&toolMessage)) {
    appendApplyLog(toolMessage);
    journal_.record(JournalLevel::Error, JournalEvent::ChannelPlanFailed);
    renderJournal();
    return;
  }
  QString draftError;
  const auto draftPath = writeDraftToTemp(&draftError);
  if (!draftPath) {
    appendApplyLog(draftError);
    journal_.record(JournalLevel::Error, JournalEvent::ChannelPlanFailed);
    renderJournal();
    return;
  }
  draftPath_ = *draftPath;
  const QString cache =
      QStandardPaths::writableLocation(QStandardPaths::CacheLocation);
  planOutPath_ = QDir(cache).filePath(QStringLiteral("plan-report.json"));
  if (QFile::exists(planOutPath_)) {
    QFile::remove(planOutPath_);
  }

  journal_.record(JournalLevel::Info, JournalEvent::ChannelPlanStarted);
  renderJournal();
  const auto result = ApplyChannelCli::run(
      ApplyChannelCli::planArgs(verifiedBackupDir_, draftPath_, planOutPath_));
  if (!result.success) {
    journal_.record(JournalLevel::Error, JournalEvent::ChannelPlanFailed);
    appendApplyLog(QStringLiteral("plan: %1").arg(result.errorCode));
    renderJournal();
    return;
  }
  journal_.record(JournalLevel::Info, JournalEvent::ChannelPlanFinished);
  appendApplyLog(trRuEn(QStringLiteral("План готов:"), QStringLiteral("Plan ready:")));
  appendApplyLog(formatPlanJson(result.stdoutJson));
  renderJournal();
}

void MainWindow::onChooseAppliedDir() {
  const QString dir = QFileDialog::getExistingDirectory(
      this,
      trRuEn(QStringLiteral("Папка для новой копии"), QStringLiteral("Folder for new copy")));
  if (dir.isEmpty()) {
    return;
  }
  appliedDir_ = dir;
  appliedDirEdit_->setText(dir);
}

void MainWindow::onRunMerge() {
  if (verifiedBackupDir_.isEmpty()) {
    appendApplyLog(trRuEn(QStringLiteral("Нужна проверенная резервная копия."),
                          QStringLiteral("A verified backup is required.")));
    return;
  }
  if (appliedDir_.isEmpty()) {
    appliedDir_ = ApplyChannelCli::suggestedAppliedDir(verifiedBackupDir_);
    appliedDirEdit_->setText(appliedDir_);
  }
  if (QFileInfo::exists(appliedDir_)) {
    appendApplyLog(QStringLiteral("merge_output_exists"));
    journal_.record(JournalLevel::Error, JournalEvent::ChannelMergeFailed);
    renderJournal();
    return;
  }
  QString toolMessage;
  if (!ensureApplyChannelTool(&toolMessage)) {
    appendApplyLog(toolMessage);
    journal_.record(JournalLevel::Error, JournalEvent::ChannelMergeFailed);
    renderJournal();
    return;
  }
  QString draftError;
  const auto draftPath = draftPath_.isEmpty() ? writeDraftToTemp(&draftError) : std::optional(draftPath_);
  if (!draftPath) {
    appendApplyLog(draftError);
    journal_.record(JournalLevel::Error, JournalEvent::ChannelMergeFailed);
    renderJournal();
    return;
  }
  journal_.record(JournalLevel::Info, JournalEvent::ChannelMergeStarted);
  renderJournal();
  const auto result = ApplyChannelCli::run(
      ApplyChannelCli::applyArgs(verifiedBackupDir_, *draftPath, appliedDir_), 600000);
  if (!result.success) {
    journal_.record(JournalLevel::Error, JournalEvent::ChannelMergeFailed);
    appendApplyLog(QStringLiteral("apply: %1").arg(result.errorCode));
    renderJournal();
    return;
  }
  journal_.record(JournalLevel::Info, JournalEvent::ChannelMergeFinished);
  appendApplyLog(trRuEn(QStringLiteral("Твик добавлен в копию:"), QStringLiteral("Tweak added to the copy:")));
  appendApplyLog(QString::fromUtf8(result.stdoutJson));
  renderJournal();
}

void MainWindow::onRunRestore() {
  if (!restoreConsentBox_->isChecked()) {
    appendApplyLog(QStringLiteral("restore_requires_consent"));
    return;
  }
  if (appliedDir_.isEmpty() || !QFileInfo::exists(appliedDir_)) {
    appendApplyLog(trRuEn(QStringLiteral("Сначала добавьте твик в копию в новой папке."),
                          QStringLiteral("Add the tweak to a copy in a new folder first.")));
    return;
  }
  ApplyError error = ApplyError::DeviceNotFound;
  const auto udid = ApplyChannel::singleTargetUdid(&error);
  if (!udid) {
    appendApplyLog(ApplyChannel::errorCode(error));
    journal_.record(JournalLevel::Error, JournalEvent::ChannelRestoreFailed);
    renderJournal();
    return;
  }

  const auto reply = QMessageBox::warning(
      this, QStringLiteral("MegaNugget"),
      trRuEn(QStringLiteral(
                 "Копия с твиком будет восстановлена на iPhone %1, после чего он перезагрузится. "
                 "Всё, что изменилось на телефоне после резервной копии, пропадёт. Продолжить?")
                 .arg(DeviceScanner::maskUdid(*udid)),
             QStringLiteral(
                 "The copy with the tweak will be restored to iPhone %1, and then it will reboot. "
                 "Anything changed on the phone since the backup will be lost. Continue?")
                 .arg(DeviceScanner::maskUdid(*udid))),
      QMessageBox::Yes | QMessageBox::No, QMessageBox::No);
  if (reply != QMessageBox::Yes) {
    return;
  }

  QString toolMessage;
  if (!ensureApplyChannelTool(&toolMessage)) {
    appendApplyLog(toolMessage);
    journal_.record(JournalLevel::Error, JournalEvent::ChannelRestoreFailed);
    renderJournal();
    return;
  }
  const auto binary = ApplyChannelCli::resolveBinary();
  if (!binary) {
    appendApplyLog(QStringLiteral("apply_channel_missing"));
    return;
  }

  if (restoreProcess_ && restoreProcess_->state() != QProcess::NotRunning) {
    appendApplyLog(trRuEn(QStringLiteral("Восстановление уже идёт."),
                          QStringLiteral("A restore is already in progress.")));
    return;
  }

  if (!restoreProcess_) {
    restoreProcess_ = new QProcess(this);
    connect(restoreProcess_,
            QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
            &MainWindow::onRestoreFinished);
  }

  restoreProcess_->setProgram(*binary);
  restoreProcess_->setArguments(ApplyChannelCli::restoreArgs(appliedDir_));
  journal_.record(JournalLevel::Info, JournalEvent::ChannelRestoreStarted);
  renderJournal();
  appendApplyLog(trRuEn(QStringLiteral("Восстанавливаю копию на iPhone %1… Не отключайте кабель.")
                            .arg(DeviceScanner::maskUdid(*udid)),
                        QStringLiteral("Restoring the copy to iPhone %1… Keep the cable connected.")
                            .arg(DeviceScanner::maskUdid(*udid))));
  restoreButton_->setEnabled(false);
  restoreConsentBox_->setEnabled(false);
  restoreProcess_->start();
  if (!restoreProcess_->waitForStarted(5000)) {
    restoreButton_->setEnabled(true);
    restoreConsentBox_->setEnabled(true);
    journal_.record(JournalLevel::Error, JournalEvent::ChannelRestoreFailed);
    appendApplyLog(QStringLiteral("restore_tool_missing"));
    renderJournal();
  }
}

void MainWindow::onRestoreFinished(int exitCode, QProcess::ExitStatus status) {
  restoreButton_->setEnabled(true);
  restoreConsentBox_->setEnabled(true);
  const QByteArray out = restoreProcess_ ? restoreProcess_->readAllStandardOutput() : QByteArray();
  const QByteArray err = restoreProcess_ ? restoreProcess_->readAllStandardError() : QByteArray();
  if (status != QProcess::NormalExit || exitCode != 0) {
    journal_.record(JournalLevel::Error, JournalEvent::ChannelRestoreFailed);
    QString code = ApplyChannelCli::parseErrorFromStderr(err);
    if (code.isEmpty()) {
      code = QStringLiteral("restore_failed");
    }
    appendApplyLog(QStringLiteral("restore: %1 (exit=%2)").arg(code).arg(exitCode));
    renderJournal();
    return;
  }
  journal_.record(JournalLevel::Info, JournalEvent::ChannelRestoreFinished);
  appendApplyLog(trRuEn(QStringLiteral("Восстановление завершено. Сделайте новую резервную копию и нажмите «Проверить результат»."),
                        QStringLiteral("Restore finished. Create a new backup and press Check result.")));
  if (!out.isEmpty()) {
    appendApplyLog(QString::fromUtf8(out));
  }
  renderJournal();
}

void MainWindow::onChoosePostBackup() {
  const QString dir = QFileDialog::getExistingDirectory(
      this, trRuEn(QStringLiteral("Копия после восстановления"), QStringLiteral("Backup after restore")));
  if (dir.isEmpty()) {
    return;
  }
  postBackupDir_ = dir;
  postBackupEdit_->setText(dir);
}

void MainWindow::onRunEffect() {
  if (verifiedBackupDir_.isEmpty() || postBackupDir_.isEmpty()) {
    appendApplyLog(trRuEn(QStringLiteral("Нужны две копии: проверенная исходная и сделанная после восстановления."),
                          QStringLiteral("Two backups are needed: the verified original and one made after the restore.")));
    return;
  }
  QString toolMessage;
  if (!ensureApplyChannelTool(&toolMessage)) {
    appendApplyLog(toolMessage);
    journal_.record(JournalLevel::Error, JournalEvent::ChannelEffectFailed);
    renderJournal();
    return;
  }
  QString draftError;
  const auto draftPath = draftPath_.isEmpty() ? writeDraftToTemp(&draftError) : std::optional(draftPath_);
  if (!draftPath) {
    appendApplyLog(draftError);
    journal_.record(JournalLevel::Error, JournalEvent::ChannelEffectFailed);
    renderJournal();
    return;
  }
  journal_.record(JournalLevel::Info, JournalEvent::ChannelEffectStarted);
  renderJournal();
  const auto result = ApplyChannelCli::run(
      ApplyChannelCli::effectArgs(*draftPath, verifiedBackupDir_, postBackupDir_), 120000);
  if (!result.success) {
    journal_.record(JournalLevel::Error, JournalEvent::ChannelEffectFailed);
    appendApplyLog(QStringLiteral("effect: %1").arg(result.errorCode));
    renderJournal();
    return;
  }
  journal_.record(JournalLevel::Info, JournalEvent::ChannelEffectFinished);
  appendApplyLog(trRuEn(QStringLiteral("Результат:"), QStringLiteral("Result:")));
  appendApplyLog(formatEffectJson(result.stdoutJson));
  renderJournal();
}
