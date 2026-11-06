import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  canInstall,
  escapeHtml as e,
  maskedId,
  preferences,
  signingActive,
  type SigningView,
  type Entry,
  type Ipa,
  type Page,
  type Scan,
} from "./model";
import "./style.css";
import logoUrl from "../../../assets/branding/meganugget-logo.svg";

const logo = (className: string) =>
  `<img class="${className}" src="${logoUrl}" width="64" height="64" alt="" draggable="false">`;

const root = document.querySelector<HTMLDivElement>("#app")!;
let stored: string | null = null;
try {
  stored = localStorage.getItem("meganugget.preferences");
} catch {
  /* storage may be disabled */
}
const prefs = preferences(stored);
let page: Page = "home";
let scan: Scan | null = null;
let selectedId = "";
let ipa: Ipa | null = null;
let logs: Entry[] = [];
let busy = false;
let notice = "";
let level = "all";
let signingMode = false;
let signing: SigningView = {
  status: { phase: "idle", teams: [], error: null },
  ipa: null,
};
let signingTimer: ReturnType<typeof setInterval> | undefined;
let signingPollRunning = false;
const signActive = () => signingActive(signing.status.phase);
const native = isTauri();
const t = (ru: string, en: string) => (prefs.locale === "ru" ? ru : en);
const errorHints: Record<string, [string, string]> = {
  unsupported_signing_bundle: [
    "Выбран посторонний IPA. Для подписи подходит только неподписанный MegaNugget (org.meganugget.ios) — например, MegaNugget-UNSIGNED.ipa из Загрузок. Сторонние приложения вроде GoldenNugget подписать нельзя.",
    "That IPA is not ours. Apple signing accepts only the unsigned MegaNugget (org.meganugget.ios), e.g. MegaNugget-UNSIGNED.ipa from Downloads. Third-party apps like GoldenNugget cannot be signed.",
  ],
  signing_content_unsupported: [
    "Подпись для IPA с Frameworks/PlugIns/dylib пока не реализована.",
    "Signing IPAs that contain Frameworks/PlugIns/dylibs is not implemented yet.",
  ],
  missing_signing_artifacts: [
    "Для прямой установки нужен подписанный IPA или режим Apple-подписи.",
    "Direct installation requires a signed IPA or the Apple signing mode.",
  ],
  minimum_ios_not_met: [
    "iOS на устройстве ниже минимальной версии выбранного IPA.",
    "The device iOS is older than the IPA requires.",
  ],
};
function describeError(raw: string): string {
  const [ru, en] = errorHints[raw] ?? [raw, raw];
  return t(ru, en);
}
const icons: Record<string, string> = {
  home: '<path d="m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-6v-7h-4v7H4a1 1 0 0 1-1-1z"/>',
  devices:
    '<rect x="6" y="2" width="12" height="20" rx="3"/><path d="M10 18h4M10 5h4"/>',
  installer: '<path d="M12 3v12m-5-5 5 5 5-5M4 15v5h16v-5"/>',
  logs: '<path d="M8 4h12M8 12h12M8 20h12M3 4h.01M3 12h.01M3 20h.01"/>',
  settings:
    '<path d="M4 6h16M4 12h16M4 18h16"/><circle cx="9" cy="6" r="2"/><circle cx="15" cy="12" r="2"/><circle cx="9" cy="18" r="2"/>',
  refresh: '<path d="M20 7a9 9 0 1 0 1 9M20 2v6h-6"/>',
  shield: '<path d="m12 2 9 4v6c0 5-9 10-9 10S3 17 3 12V6zM8 12l3 3 5-6"/>',
  arrow: '<path d="M4 12h16m-6-6 6 6-6 6"/>',
  usb: '<path d="M12 21V3m-3 3 3-3 3 3M12 16l-6-4V8m6 5 6-4V6"/><circle cx="6" cy="7" r="1"/><path d="M17 3h2v3h-2z"/><circle cx="12" cy="21" r="1"/>',
};
const icon = (name: string, cls = "") =>
  `<svg class="icon ${cls}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${icons[name] ?? icons.shield}</svg>`;
const names = () => ({
  home: t("Обзор", "Home"),
  devices: t("Устройства", "Device Manager"),
  installer: t("Установка IPA", "App Installer"),
  logs: t("Журнал событий", "Logs"),
  settings: t("Настройки", "Settings"),
});
const current = () => scan?.devices.find((d) => d.device.id === selectedId);
const button = (
  id: string,
  text: string,
  glyph = "arrow",
  disabled = false,
  secondary = false,
) =>
  `<button id="${id}" class="button ${secondary ? "secondary" : ""}" ${disabled || busy ? "disabled" : ""}>${icon(glyph)}${text}</button>`;
const row = (label: string, value: unknown) =>
  `<div class="detail-row"><span>${label}</span><strong>${e(value || "—")}</strong></div>`;

function status(): string {
  if (busy) return t("Выполняется операция", "Operation in progress");
  if (!native)
    return t("Предпросмотр · USB недоступен", "Preview · USB unavailable");
  if (!scan) return t("Ожидание проверки", "Awaiting scan");
  if (scan.failure) return t("Ошибка соединения", "Connection error");
  if (!scan.devices.length)
    return t("Устройство не подключено", "No device connected");
  return t("Устройство обнаружено", "Device detected");
}
function compatibility(): string {
  return current()?.compatibility?.status === "research_target"
    ? t(
        "Целевая конфигурация · твики ещё не проверены",
        "Target configuration · tweaks not validated yet",
      )
    : t("Вне первого этапа переноса", "Outside the initial porting target");
}
function deviceSelector(): string {
  if (!scan?.devices.length) return "";
  return `<label class="device-select">${t("Выбранное USB-устройство", "Selected USB device")}<select id="device" ${busy ? "disabled" : ""}>${scan.devices.map((d) => `<option value="${e(d.device.id)}" ${selectedId === d.device.id ? "selected" : ""}>${e(d.device.info?.ProductType ?? t("Неизвестная модель", "Unknown model"))} · ${e(maskedId(d.device.id, prefs.mask))}</option>`).join("")}</select></label>`;
}
function deviceDetails(): string {
  const d = current()?.device;
  return `${row(t("Модель", "Product type"), d?.info?.ProductType)}${row("iOS", d?.info?.ProductVersion)}${row(t("Сборка", "Build"), d?.info?.BuildVersion)}${row(t("Архитектура", "Architecture"), d?.info?.CPUArchitecture)}${row("UDID", d ? maskedId(d.id, prefs.mask) : "")}${row(t("Доступ lockdown", "Lockdown access"), d?.trust === "lockdown_readable" ? t("Чтение доступно", "Readable") : t("Неизвестен", "Unknown"))}`;
}
function home(): string {
  const d = current()?.device;
  return `<div class="hero card"><div class="hero-copy"><span class="eyebrow">LEGACY IOS CUSTOMIZATION</span><h2>${t("Ваш iPhone.<br>Ваш стиль.", "Your iPhone.<br>Your style.")}</h2><p>${t("Кастомизация старых iOS без jailbreak.<br>Desktop помогает подключить устройство, подписать и установить MegaNugget.", "Customize older iOS versions without a jailbreak.<br>Connect your device, sign and install MegaNugget from Desktop.")}</p>${button("scan", t("Подключить iPhone", "Connect iPhone"), "refresh", !native)}<small>${t("Проверка USB только читает данные устройства", "USB scanning only reads device information")}</small></div><div class="phone-art" aria-hidden="true"><div class="phone-island"></div><div class="phone-glow">${logo("hero-logo")}</div><span>MegaNugget</span><small>LEGACY EDITION</small><div class="phone-bar"></div></div><span class="hero-version">LEGACY PREVIEW</span></div>
  <div class="stats"><div class="stat card"><span>${icon("usb")}${t("Соединение", "Connection")}</span><strong>${status()}</strong><small>USB / libimobiledevice</small></div><div class="stat card"><span>${icon("devices")}${t("Устройство", "Device")}</span><strong>${e(d?.info?.ProductType ?? "—")}</strong><small>${d?.info ? `iOS ${e(d.info.ProductVersion)}` : t("Подключите iPhone кабелем", "Connect an iPhone via USB")}</small></div><div class="stat card"><span>${icon("settings")}${t("Кастомизация", "Customization")}</span><strong>${t("Перенос на iOS 15", "Porting to iOS 15")}</strong><small>${t("Системные твики пока недоступны", "System tweaks are not available yet")}</small></div></div>
  <div class="two-col"><section class="card panel"><div class="section-heading"><h3>${t("Ваше устройство", "Your device")}</h3><span class="pill">USB</span></div>${deviceSelector()}${deviceDetails()}</section><section class="card panel"><span class="eyebrow">MEGANUGGET IOS</span><h3 class="large-title">${t("Новый вид.<br>Знакомый iPhone.", "A fresh look.<br>A familiar iPhone.")}</h3><p>${t("Установите наш IPA через Apple Account или выберите уже подписанный файл. Первая цель переноса — iPhone 7 на iOS 15.8.8.", "Sign our IPA with your Apple Account, or select a pre-signed file. Our initial porting target is iPhone 7 on iOS 15.8.8.")}</p><div class="note">${icon("shield")}<span>${t("В новой iOS-версии доступна тема самого приложения. Строка состояния, обои и другие системные изменения ещё не перенесены.", "The new iOS version supports in-app appearance. Status bar, wallpapers and other system changes have not been ported yet.")}</span></div>${button("go-installer", t("Подписать и установить IPA", "Sign and install IPA"), "arrow", false, true)}</section></div>`;
}
function devices(): string {
  const failure = scan?.failure ?? current()?.device.failure;
  return `<section class="card panel"><div class="section-heading"><div><span class="eyebrow">USB DEVICE MANAGER</span><h2>${status()}</h2></div>${button("scan", t("Повторить проверку", "Scan again"), "refresh", !native)}</div>${deviceSelector()}<div class="two-col"><div>${deviceDetails()}</div><div class="connection-visual">${icon("usb")}<span>USB → usbmuxd → lockdown</span><small>${t("Сопряжение не запускается автоматически", "Pairing is not initiated automatically")}</small></div></div></section>
  <div class="two-col"><section class="card panel"><h3>${t("Совместимость", "Compatibility")}</h3><span class="pill warning">${compatibility()}</span><p>${t("Первая цель переноса: iPhone 7 · iPhone9,1 / iPhone9,3 · A10 · ARM64 · iOS 15.8.8. Совпадение устройства не означает, что системные твики уже работают.", "Initial porting target: iPhone 7 · iPhone9,1 / iPhone9,3 · A10 · ARM64 · iOS 15.8.8. A matching device does not mean system tweaks already work.")}</p><p>${t("Другие версии iOS и модели пока вне первого этапа.", "Other iOS versions and models are outside the initial scope.")}</p></section><section class="card panel"><h3>${t("Диагностика подключения", "Connection diagnostics")}</h3>${row(t("Результат", "Result"), failure ?? (scan ? t("Команда USB выполнена", "USB command completed") : t("Проверка ещё не запускалась", "Not scanned yet")))}<ol class="checklist"><li>${t("Разблокируйте iPhone и подтвердите «Доверять».", "Unlock iPhone and confirm “Trust”.")}</li><li>${t("Используйте кабель передачи данных и другой USB-порт.", "Use a data cable and try another USB port.")}</li><li>${t("Проверьте usbmuxd / драйвер Apple и инструменты в PATH.", "Check usbmuxd / Apple drivers and tools on PATH.")}</li></ol><small>${t("Неудача чтения не позволяет определить причину доверия или блокировки.", "A read failure does not establish trust or lock state.")}</small></section></div>`;
}
function installer(): string {
  const available = !!scan?.installer.Ok;
  const signed = ipa?.signing === "artifacts_present_not_verified";
  return `<div class="notice-banner">${icon("shield")}<div><strong>Apple Code Signing</strong><p>${t("Готовый подписанный IPA можно установить напрямую. Для нашего неподписанного MegaNugget доступна экспериментальная подпись через Apple Account без заранее созданного сертификата. Обходов защиты нет.", "Install a pre-signed IPA directly, or use experimental Apple Account signing for our unsigned MegaNugget. No pre-existing certificate is needed. No protection bypass.")}</p></div></div>
    <section class="card panel"><div class="section-heading"><h2>${t("Подпись и установка", "Sign and install")}</h2>${button("scan", t("Проверить USB", "Scan USB"), "refresh", !native || signActive(), true)}</div>
    <label class="device-select">${t("Режим", "Mode")}<select id="installer-mode" ${busy || signActive() ? "disabled" : ""}><option value="signed" ${!signingMode ? "selected" : ""}>${t("Установить подписанный IPA", "Install signed IPA")}</option><option value="apple" ${signingMode ? "selected" : ""}>${t("Подписать MegaNugget через Apple Account", "Sign MegaNugget with Apple Account")}</option></select></label>
    <div class="file-zone">${icon("installer")}<h3>${ipa ? e(ipa.bundle_id) : t("Выберите IPA-файл", "Choose an IPA file")}</h3><p>${signingMode ? t("Сейчас поддерживается только собственный org.meganugget.ios без расширений и сторонних frameworks.", "Currently limited to our own org.meganugget.ios without extensions or third-party frameworks.") : t("Локальная проверка структуры IPA.", "Local IPA structure validation.")}</p>${button("choose", t("Выбрать IPA", "Choose IPA"), "installer", !native || signActive(), true)}</div>
    ${ipa ? `${row("Bundle ID", ipa.bundle_id)}${row(t("Версия сборки", "Build version"), ipa.version)}${row(t("Минимальная iOS", "Minimum iOS"), ipa.minimum_ios)}${row(t("Состояние", "State"), signed ? t("Файлы подписи присутствуют; решение принимает iOS", "Signing artifacts present; iOS decides acceptance") : t("Требуется подпись", "Signing required"))}` : ""}
    ${deviceSelector()}${signingMode ? signingPanel() : ""}
    ${row(t("Метод установки", "Installation method"), available ? `ideviceinstaller · ${scan?.installer.Ok}` : t("Недоступен / ещё не проверен", "Unavailable / not checked"))}
    <div class="install-footer"><span>${t("Подпись не устанавливает приложение автоматически. Установка может заменить приложение с тем же Bundle ID.", "Signing does not install automatically. Installation can replace an app with the same bundle ID.")}</span>${button("install", t("Установить на iPhone", "Install on iPhone"), "installer", !native || !canInstall(scan, current(), ipa, busy || signActive()))}</div>
    ${signed ? `<div class="export-ipa">${button("save-ipa", t("Сохранить подписанный IPA", "Save signed IPA"), "installer", !native || signActive(), true)}<small>${t("Файл содержит provisioning с идентификаторами устройства. Не публикуйте его.", "Contains device provisioning identifiers. Do not publish it.")}</small></div>` : ""}</section>
    <p class="footnote">${t("После установки проверяются Bundle ID и версия. Это не подтверждает запуск. Подпись не снижает минимальную версию iOS и не даёт jailbreak.", "Bundle ID and version are checked after installation, not app launch. Signing does not lower minimum iOS or provide a jailbreak.")}</p>`;
}

function signingPanel(): string {
  const phase = signing.status.phase;
  const labels = {
    idle: t("Ожидание", "Ready"),
    authenticating: t("Авторизация Apple", "Apple authentication"),
    two_factor: t(
      "Введите код 2FA с доверенного устройства",
      "Enter 2FA code from your trusted device",
    ),
    select_team: t("Выберите команду Apple", "Select Apple Team"),
    provisioning: t("Регистрация устройства", "Registering device"),
    signing: t("Получение профиля и подпись", "Provisioning and signing"),
    signed: t(
      "IPA создан; установка пока не выполнена",
      "IPA created; not installed yet",
    ),
    failed: t("Подпись не подтверждена", "Signing not confirmed"),
    cancelled: t(
      "Отменено; изменения аккаунта не откатываются",
      "Cancelled; account changes are not rolled back",
    ),
  };
  const ready =
    native &&
    !!ipa &&
    phase === "idle" &&
    !!current()?.device.info &&
    !current()?.device.failure;
  const form =
    phase === "idle"
      ? `<form id="apple-form" autocomplete="off"><label>Apple Account<input id="apple-email" type="email" autocomplete="off" required maxlength="254" ${!native ? "disabled" : ""}></label><label>${t("Пароль Apple Account", "Apple Account password")}<input id="apple-password" type="password" autocomplete="new-password" required maxlength="1024" ${!native ? "disabled" : ""}></label><label class="consent"><input id="sign-consent" type="checkbox" required ${!native ? "disabled" : ""}><span>${t("Разрешаю сетевую авторизацию и создание сертификата, App ID, группы и профиля у Apple. Согласен на сторонний anisette-сервис https://ani.iloader.app и хранение ключа в системном keyring. Автоматического отзыва сертификатов нет.", "I consent to Apple authentication/account provisioning, third-party anisette at https://ani.iloader.app and private-key storage in the OS keyring. Certificates are never revoked automatically.")}</span></label><button type="submit" class="button" ${!ready || busy ? "disabled" : ""}>${icon("shield")}${t("Подписать через Apple", "Sign with Apple")}</button></form>`
      : "";
  const challenge =
    phase === "two_factor"
      ? `<form id="two-factor-form" autocomplete="off"><label>${t("Код 2FA (6 цифр)", "2FA code (6 digits)")}<input id="two-factor-code" inputmode="numeric" pattern="[0-9]{6}" maxlength="6" required autocomplete="off"></label><button type="submit" class="button">${t("Продолжить", "Continue")}</button></form>`
      : "";
  const teams =
    phase === "select_team"
      ? `<form id="team-form"><label>Apple Team<select id="apple-team">${signing.status.teams.map((team) => `<option value="${e(team.id)}">${e(team.name)} · ${e(team.id)}</option>`).join("")}</select></label><button type="submit" class="button">${t("Выбрать", "Select")}</button></form>`
      : "";
  return `<section class="signing-panel"><span class="eyebrow">EXPERIMENTAL · ISIDELOAD BACKEND</span><h3 role="status">${e(labels[phase])}</h3>${signing.status.error ? `<p class="warning-text">${e(signing.status.error)}</p>` : ""}<p>${t("Пароль и код не сохраняются в настройках или журналах. Данные обрабатываются локальным backend и сервисами авторизации. Бесплатная подпись обычно действует 7 дней. Требуется системный keyring; проблемы Apple/2FA могут прервать процесс. Только код с доверенного устройства; выбор SMS пока не реализован.", "Password and code are not saved in settings or logs. Authentication uses the native backend and network services. Free signing typically lasts 7 days. OS keyring required; Apple/2FA failures may interrupt the process. Trusted-device codes only; SMS selection is not implemented.")}</p>${form}${challenge}${teams}${signActive() ? button("cancel-sign", t("Отменить подпись", "Cancel signing"), "arrow", false, true) : ""}${phase === "failed" || phase === "cancelled" ? `<p>${t("Для повторной попытки снова выберите исходный IPA. Ранее созданный сертификат остаётся в аккаунте.", "Select the original IPA again to retry. Any created certificate remains in your account.")}</p>` : ""}</section>`;
}

async function pollSigning(): Promise<void> {
  if (!native || signingPollRunning) return;
  signingPollRunning = true;
  try {
    const next = await invoke<SigningView>("signing_status");
    const changed = JSON.stringify(next) !== JSON.stringify(signing);
    signing = next;
    if (next.ipa) ipa = next.ipa;
    if (!signActive()) {
      clearInterval(signingTimer);
      signingTimer = undefined;
      await refreshLogs();
    }
    if (changed && !busy) render();
  } catch {
    clearInterval(signingTimer);
    signingTimer = undefined;
    notice = t(
      "Связь со статусом подписи потеряна. Результат неизвестен.",
      "Signing status connection lost. Result unknown.",
    );
    if (!busy) render();
  } finally {
    signingPollRunning = false;
  }
}
function journal(): string {
  const entries = logs.filter((x) => level === "all" || x.level === level);
  return `<section class="card panel"><div class="section-heading"><div><h3>${t("Диагностический журнал", "Diagnostic journal")}</h3><p>${t("Только разрешённые коды событий. Без UDID, серийных номеров, путей и вывода команд.", "Allowlisted event codes only. No UDIDs, serial numbers, paths or command output.")}</p></div>${button("export", t("Экспорт JSON", "Export JSON"), "installer", !native, true)}</div><label>${t("Уровень", "Level")} <select id="level">${["all", "info", "warning", "error"].map((x) => `<option ${x === level ? "selected" : ""} value="${x}">${e(x)}</option>`).join("")}</select></label><div class="log-table"><div class="log-row log-header"><span>${t("Время", "Time")}</span><span>${t("Уровень", "Level")}</span><span>${t("Событие", "Event")}</span></div>${entries.length ? entries.map((x) => `<div class="log-row"><time>${e(new Date(x.timestamp * 1000).toLocaleTimeString(prefs.locale))}</time><span class="log-level ${e(x.level)}">${e(x.level)}</span><code>${e(x.event)}</code></div>`).join("") : `<div class="empty-state">${icon("logs")}<h3>${t("Пока тихо", "All quiet for now")}</h3><p>${t("Запустите проверку устройства, чтобы увидеть события.", "Run a device scan to see diagnostic events.")}</p></div>`}</div></section>`;
}
function settings(): string {
  return `<section class="card panel settings-panel"><h3>${t("Внешний вид и язык", "Appearance and language")}</h3><label class="setting-row"><span>${t("Тема интерфейса", "Interface theme")}<small>${t("Тёмная по умолчанию", "Dark by default")}</small></span><select id="theme"><option value="dark" ${prefs.theme === "dark" ? "selected" : ""}>${t("Тёмная", "Dark")}</option><option value="light" ${prefs.theme === "light" ? "selected" : ""}>${t("Светлая", "Light")}</option></select></label><label class="setting-row"><span>${t("Язык", "Language")}</span><select id="locale"><option value="ru" ${prefs.locale === "ru" ? "selected" : ""}>Русский</option><option value="en" ${prefs.locale === "en" ? "selected" : ""}>English</option></select></label><h3>${t("Приватность и диагностика", "Privacy and diagnostics")}</h3><label class="setting-row"><span>${t("Скрывать UDID на экране", "Mask UDID on screen")}<small>${t("Экспорт всегда исключает идентификаторы", "Exports always exclude identifiers")}</small></span><input id="mask" type="checkbox" ${prefs.mask ? "checked" : ""}></label><label class="setting-row"><span>${t("Проверка USB при запуске", "Scan USB on launch")}<small>${t("Только чтение. По умолчанию выключено.", "Read-only. Disabled by default.")}</small></span><input id="startup" type="checkbox" ${prefs.scanOnLaunch ? "checked" : ""}></label>${row(t("Телеметрия", "Telemetry"), t("Отсутствует", "None"))}${row(t("Хранение журнала", "Journal retention"), t("В памяти, до 500 событий", "In memory, up to 500 events"))}<h3>${t("О проекте", "About")}</h3>${row(t("Версия", "Version"), "0.1.0 · legacy customization preview")}${row(t("Лицензия проекта", "Project license"), "AGPL-3.0-only")}<p>Copyright © 2026 MegaNugget contributors. ${t("Без гарантий. Разрешено распространение и изменение на условиях AGPL-3.0; полный текст в LICENSE. Первоначальное MIT-уведомление сохранено в LICENSES. Зависимости имеют собственные лицензии; см. docs/THIRD_PARTY.md.", "No warranty. Redistribution and modification permitted under AGPL-3.0; full terms in LICENSE. Initial MIT notice retained in LICENSES. Dependencies retain their licenses; see docs/THIRD_PARTY.md.")}</p></section>`;
}
function render(): void {
  document.documentElement.lang = prefs.locale;
  document.documentElement.dataset.theme = prefs.theme;
  const titles = names();
  root.innerHTML = `<aside class="sidebar"><a class="brand" href="#home" aria-label="MegaNugget Home">${logo("brand-mark")}<span>MegaNugget<small>LEGACY IOS CUSTOMIZATION</small></span></a><span class="nav-label">WORKSPACE</span><nav>${(Object.keys(titles) as Page[]).map((key) => `<button data-page="${key}" class="nav-item ${key === page ? "selected" : ""}" ${key === page ? 'aria-current="page"' : ""}>${icon(key)}${titles[key]}${key === "devices" && scan?.devices.length ? '<span class="nav-dot"></span>' : ""}</button>`).join("")}</nav><div class="sidebar-bottom"><div class="research-badge">${icon("shield")}<span>${t("Прозрачно по замыслу", "Transparent by design")}<small>${t("Без jailbreak", "No jailbreak")}</small></span></div><div class="version"><span class="tiny-dot"></span> LEGACY PREVIEW <span>v0.1.0</span></div></div></aside><main><header class="topbar"><span>MegaNugget <span class="slash">/</span> <strong>${titles[page]}</strong></span><span class="top-status"><span class="tiny-dot ${scan?.failure ? "amber" : ""}"></span>${status()}</span></header><div class="content"><div class="page-heading"><div><span class="eyebrow">YOUR DEVICE. YOUR STYLE.</span><h1>${titles[page]}</h1><p>${t("Знакомый iPhone. Новые возможности.", "A familiar iPhone. New possibilities.")}</p></div><span class="pill outline">${icon("shield")}${t("Без скрытых действий", "No hidden operations")}</span></div>${!native ? `<div class="notice-banner compact">${t("Веб-предпросмотр: нативный backend отсутствует. USB, выбор IPA и установка недоступны. Данные устройства не симулируются.", "Web preview: native backend absent. USB, IPA selection and installation are unavailable. No device data is simulated.")}</div>` : ""}${notice ? `<div role="status" class="notice-banner result">${e(notice)}</div>` : ""}${busy ? `<div role="status" class="busy"><span class="spinner"></span>${t("Ожидание реальной операции. Не отключайте устройство при установке.", "Waiting for the real operation. Do not disconnect during installation.")}</div>` : ""}${page === "settings" ? `<div class="product-identity">${logo("about-logo")}<div><strong>MegaNugget</strong><span>Legacy iOS Customization</span></div></div>` : ""}${{ home, devices, installer, logs: journal, settings }[page]()}<footer>MegaNugget <span>Open source. Open possibilities.</span><span>${t("Кастомизация старых iOS.", "Customization for older iOS.")}</span></footer></div></main>`;
  bind();
}
async function refreshLogs(): Promise<void> {
  if (native) logs = await invoke<Entry[]>("get_logs");
}
async function action(work: () => Promise<void>): Promise<void> {
  if (busy) return;
  busy = true;
  notice = "";
  render();
  try {
    await work();
  } catch (error) {
    notice = `${t("Операция не подтверждена", "Operation not confirmed")}: ${describeError(
      String(error),
    )}`;
  } finally {
    try {
      await refreshLogs();
    } catch {
      /* retain the original operation status */
    }
    busy = false;
    render();
  }
}
async function scanDevices(): Promise<void> {
  await action(async () => {
    scan = await invoke<Scan>("scan_devices");
    if (!scan.devices.some((d) => d.device.id === selectedId))
      selectedId = scan.devices[0]?.device.id ?? "";
    if (scan.failure)
      notice = `${t("Диагностика USB завершилась ошибкой", "USB diagnostics failed")}: ${scan.failure}`;
  });
}
function save(): void {
  try {
    localStorage.setItem("meganugget.preferences", JSON.stringify(prefs));
  } catch {
    notice = t(
      "Не удалось сохранить настройки.",
      "Could not persist settings.",
    );
  }
  render();
}
function on(id: string, callback: () => void): void {
  document.getElementById(id)?.addEventListener("click", callback);
}
function select(id: string, callback: (value: string) => void): void {
  document
    .getElementById(id)
    ?.addEventListener("change", (event) =>
      callback((event.target as HTMLSelectElement).value),
    );
}
function bind(): void {
  document
    .querySelectorAll<HTMLButtonElement>("[data-page]")
    .forEach((button) =>
      button.addEventListener("click", () => {
        page = button.dataset.page as Page;
        render();
        if (page === "logs" && !busy)
          void action(async () => {
            await refreshLogs();
          });
      }),
    );
  document.querySelector(".brand")?.addEventListener("click", (event) => {
    event.preventDefault();
    page = "home";
    render();
  });
  on("scan", () => {
    if (native) void scanDevices();
  });
  on("go-installer", () => {
    page = "installer";
    render();
  });
  on("choose", () => {
    if (native && !signActive())
      void action(async () => {
        ipa = null;
        signing = {
          status: { phase: "idle", teams: [], error: null },
          ipa: null,
        };
        ipa = await invoke<Ipa | null>("choose_ipa", {
          forSigning: signingMode,
        });
      });
  });
  on("install", () => {
    if (native && canInstall(scan, current(), ipa, busy || signActive()))
      void action(async () => {
        const result = await invoke<string>("install_ipa", { id: selectedId });
        notice =
          result === "installed_verified"
            ? t(
                "Bundle ID и версия найдены на устройстве. Откройте приложение на iPhone.",
                "Bundle ID and build found on device. Open the app on iPhone.",
              )
            : t("Установка отменена.", "Installation cancelled.");
      });
  });
  select("installer-mode", (value) => {
    if (signActive()) return;
    signingMode = value === "apple";
    ipa = null;
    signing = { status: { phase: "idle", teams: [], error: null }, ipa: null };
    notice = "";
    render();
  });
  document.getElementById("apple-form")?.addEventListener("submit", (event) => {
    event.preventDefault();
    if (!native || busy || signActive()) return;
    const emailInput =
      document.querySelector<HTMLInputElement>("#apple-email")!;
    const passwordInput =
      document.querySelector<HTMLInputElement>("#apple-password")!;
    const consent = document.querySelector<HTMLInputElement>("#sign-consent")!;
    if (!consent.checked || !ipa) return;
    const email = emailInput.value;
    let password = passwordInput.value;
    emailInput.value = "";
    passwordInput.value = "";
    void action(async () => {
      const pending = invoke<boolean>("begin_signing", {
        id: selectedId,
        email,
        password,
      });
      password = "";
      const started = await pending;
      if (started) {
        signing = {
          status: { phase: "authenticating", teams: [], error: null },
          ipa: null,
        };
        clearInterval(signingTimer);
        signingTimer = setInterval(() => {
          void pollSigning();
        }, 1000);
      } else {
        notice = t(
          "Подпись отменена в системном диалоге.",
          "Signing cancelled in the native dialog.",
        );
      }
    });
  });
  for (const [form, input] of [
    ["two-factor-form", "two-factor-code"],
    ["team-form", "apple-team"],
  ]) {
    document.getElementById(form)?.addEventListener("submit", (event) => {
      event.preventDefault();
      if (!native || busy) return;
      const element = document.getElementById(input) as
        HTMLInputElement | HTMLSelectElement;
      const value = element.value;
      element.value = "";
      void action(async () => {
        await invoke("signing_respond", { value });
      });
    });
  }
  on("cancel-sign", () => {
    if (native && signActive())
      void action(async () => {
        await invoke("cancel_signing");
        await pollSigning();
      });
  });
  on("save-ipa", () => {
    if (native && !signActive())
      void action(async () => {
        const saved = await invoke<boolean>("export_signed_ipa");
        notice = saved
          ? t(
              "IPA сохранён. Не публикуйте provisioning-профиль.",
              "IPA saved. Do not publish its provisioning profile.",
            )
          : t("Сохранение отменено.", "Save cancelled.");
      });
  });
  if (signActive()) {
    const device = document.getElementById(
      "device",
    ) as HTMLSelectElement | null;
    if (device) device.disabled = true;
  }
  on("export", () => {
    if (native)
      void action(async () => {
        const exported = await invoke<boolean>("export_report");
        notice = exported
          ? t("Обезличенный отчёт сохранён.", "De-identified report saved.")
          : t("Экспорт отменён.", "Export cancelled.");
      });
  });
  select("device", (value) => {
    selectedId = value;
    render();
  });
  select("level", (value) => {
    level = value;
    render();
  });
  select("theme", (value) => {
    prefs.theme = value === "light" ? "light" : "dark";
    save();
  });
  select("locale", (value) => {
    prefs.locale = value === "en" ? "en" : "ru";
    save();
  });
  for (const [id, key] of [
    ["mask", "mask"],
    ["startup", "scanOnLaunch"],
  ] as const) {
    document.getElementById(id)?.addEventListener("change", (event) => {
      prefs[key] = (event.target as HTMLInputElement).checked;
      save();
    });
  }
}
render();
if (native && prefs.scanOnLaunch) void scanDevices();
