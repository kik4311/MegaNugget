#include "MainWindow.h"

#include "CliRunner.h"

#include <QCheckBox>
#include <QComboBox>
#include <QFile>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMessageBox>
#include <QPushButton>
#include <QSettings>
#include <QTabWidget>
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

  QString catalogError;
  if (!TweakCatalog::instance().load(&catalogError)) {
    tweaksMeta_->setText(QStringLiteral("catalog error: %1").arg(catalogError));
  }

  applyTheme();
  refreshChrome();
  statusLabel_->setText(
      trRuEn(QStringLiteral("Подключите iPhone по USB и нажмите «Сканировать»."),
             QStringLiteral("Connect an iPhone over USB and press Scan.")));
  renderTweaks();
}

MainWindow::~MainWindow() {
  if (backupProcess_) {
    backupProcess_->kill();
    backupProcess_->waitForFinished(3000);
  }
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
          "Кастомизация старых iOS без jailbreak. Desktop на Qt 6 (фаза 0–4)."),
      QStringLiteral(
          "Old-iOS customization without jailbreak. Qt 6 desktop (phase 0–4).")));
  themeBox_->setItemText(0, trRuEn(QStringLiteral("Тёмная"), QStringLiteral("Dark")));
  themeBox_->setItemText(1, trRuEn(QStringLiteral("Светлая"), QStringLiteral("Light")));
  tabs_->setTabText(0, trRuEn(QStringLiteral("Устройства"), QStringLiteral("Devices")));
  tabs_->setTabText(1, trRuEn(QStringLiteral("Твики"), QStringLiteral("Tweaks")));
  tabs_->setTabText(2, trRuEn(QStringLiteral("Канал"), QStringLiteral("Apply")));
  tabs_->setTabText(3, trRuEn(QStringLiteral("Подпись"), QStringLiteral("Signing")));
  scanButton_->setText(trRuEn(QStringLiteral("Сканировать"), QStringLiteral("Scan")));
  exportButton_->setText(
      trRuEn(QStringLiteral("Экспорт журнала"), QStringLiteral("Export journal")));
  chooseIpaButton_->setText(
      trRuEn(QStringLiteral("Выбрать IPA"), QStringLiteral("Choose IPA")));
  installButton_->setText(trRuEn(QStringLiteral("Установить"), QStringLiteral("Install")));
  tweakFilter_->setPlaceholderText(
      trRuEn(QStringLiteral("Поиск по названию/id"), QStringLiteral("Search title/id")));
  candidatesOnlyBox_->setText(
      trRuEn(QStringLiteral("Только кандидаты"), QStringLiteral("Candidates only")));
  compileButton_->setText(
      trRuEn(QStringLiteral("Скомпилировать черновик"), QStringLiteral("Compile draft")));
  applyDetectButton_->setText(
      trRuEn(QStringLiteral("Detect (цель)"), QStringLiteral("Detect (target)")));
  chooseBackupDirButton_->setText(
      trRuEn(QStringLiteral("Папка backup"), QStringLiteral("Backup folder")));
  backupConsentBox_->setText(trRuEn(
      QStringLiteral("Я согласен создать полный локальный backup (consent)"),
      QStringLiteral("I consent to create a full local backup")));
  backupButton_->setText(
      trRuEn(QStringLiteral("Запустить backup"), QStringLiteral("Start backup")));
  applyStatus_->setText(trRuEn(
      QStringLiteral(
          "Apply channel: только iPhone9,1/9,3 + 15.8.8. Restore пока не из UI."),
      QStringLiteral(
          "Apply channel: iPhone9,1/9,3 + 15.8.8 only. Restore is not in the UI yet.")));
  signingLabel_->setText(trRuEn(
      QStringLiteral(
          "Apple Account signing ещё не портирован на Qt/C++ (isideload остаётся в "
          "Rust/Tauri Desktop). Здесь заглушка фазы 4 — без сети и без ввода пароля."),
      QStringLiteral(
          "Apple Account signing is not ported to Qt/C++ yet (isideload remains in "
          "the Rust/Tauri Desktop). This is a phase-4 stub — no network, no password UI.")));
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
      trRuEn(QStringLiteral("Установить %1 (build %2) на устройство %3?")
                 .arg(ipa_->info.bundleId, ipa_->info.version, DeviceScanner::maskUdid(*udid)),
             QStringLiteral("Install %1 (build %2) on device %3?")
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
  QString text = trRuEn(QStringLiteral("Файлов: %1 · selected=%2 · applied_to_device=false\n\n")
                            .arg(preview->files.size())
                            .arg(preview->selectedCount),
                        QStringLiteral("Files: %1 · selected=%2 · applied_to_device=false\n\n")
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
        QStringLiteral("Скан не удался: %1").arg(CliRunner::failureCode(*scan_.failure)),
        QStringLiteral("Scan failed: %1").arg(CliRunner::failureCode(*scan_.failure))));
    return;
  }
  if (scan_.devices.isEmpty()) {
    statusLabel_->setText(trRuEn(
        QStringLiteral("Устройства не найдены (USB)."),
        QStringLiteral("No devices found (USB).")));
    return;
  }
  statusLabel_->setText(trRuEn(QStringLiteral("Найдено устройств: %1").arg(scan_.devices.size()),
                               QStringLiteral("Devices found: %1").arg(scan_.devices.size())));
  for (const auto& device : scan_.devices) {
    if (!device.info) {
      deviceList_->addItem(QStringLiteral("%1 · unknown").arg(device.idMasked));
      continue;
    }
    QString line = QStringLiteral("%1 · %2 · %3 · build %4")
                       .arg(device.idMasked, device.info->product, device.info->ios,
                            device.info->build);
    if (device.compatibility.matchesTarget) {
      line += QStringLiteral(" · matches_target=true");
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
        QStringLiteral("IPA не выбран. Нужен подписанный IPA."),
        QStringLiteral("No IPA selected. A signed IPA is required.")));
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
      trRuEn(QStringLiteral("Цель: %1 / %2 · кандидаты %3 / %4 · ревизия %5")
                 .arg(device.model, device.systemVersion)
                 .arg(counts.candidates)
                 .arg(counts.total)
                 .arg(TweakCatalog::instance().revision().left(12)),
             QStringLiteral("Target: %1 / %2 · candidates %3 / %4 · revision %5")
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
    lines << trRuEn(QStringLiteral("Устройств нет / ошибка: %1")
                        .arg(detection.failures.join(QLatin1Char(','))),
                    QStringLiteral("No devices / failure: %1")
                        .arg(detection.failures.join(QLatin1Char(','))));
  }
  for (const auto& device : detection.devices) {
    lines << QStringLiteral("%1 · %2 · %3 · matches_target=%4")
                 .arg(device.udidMasked,
                      device.product.isEmpty() ? QStringLiteral("?") : device.product,
                      device.ios.isEmpty() ? QStringLiteral("?") : device.ios)
                 .arg(device.matchesTarget ? QStringLiteral("true")
                                           : QStringLiteral("false"));
  }
  if (!detection.failures.isEmpty()) {
    lines << QStringLiteral("failures: %1").arg(detection.failures.join(QLatin1Char(',')));
  }
  applyLog_->setPlainText(lines.join(QLatin1Char('\n')));
}

void MainWindow::onChooseBackupDir() {
  const QString dir = QFileDialog::getExistingDirectory(
      this, trRuEn(QStringLiteral("Папка для backup"), QStringLiteral("Backup folder")));
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
        trRuEn(QStringLiteral("Сначала выберите папку backup."),
               QStringLiteral("Choose a backup folder first.")));
    return;
  }
  if (backupProcess_ && backupProcess_->state() != QProcess::NotRunning) {
    applyLog_->setPlainText(
        trRuEn(QStringLiteral("Backup уже выполняется."),
               QStringLiteral("A backup is already running.")));
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
          "Backup запущен для %1…\nЭто может занять много времени. Не отключайте кабель.")
          .arg(DeviceScanner::maskUdid(*udid)),
      QStringLiteral(
          "Backup started for %1…\nThis can take a long time. Keep the cable connected.")
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
              "\n\nПодсказка: MSYS2 idevicebackup2 1.3.0 слишком старый для iOS 15+. "
              "Нужна сборка libimobiledevice из git (1.4+/master)."),
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
  renderJournal();
}
