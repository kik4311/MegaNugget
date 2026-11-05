import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  canInstall,
  escapeHtml as e,
  maskedId,
  preferences,
  type Entry,
  type Ipa,
  type Page,
  type Scan,
} from "./model";
import "./style.css";

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
const native = isTauri();
const t = (ru: string, en: string) => (prefs.locale === "ru" ? ru : en);
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
    ? t("Исследовательская конфигурация", "Research target")
    : t("Совместимость не подтверждена", "Compatibility unverified");
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
  return `<div class="hero card"><div class="hero-copy"><span class="eyebrow">IOS SECURITY RESEARCH</span><h2>${t("Большие идеи.<br>Честные возможности.", "Big ideas.<br>Honest capabilities.")}</h2><p>${t("Открытая платформа для исследования iOS.<br>Ваше устройство. Ваш контроль. Никаких скрытых действий.", "An open platform for iOS security research.<br>Your device. Your control. No hidden operations.")}</p>${button("scan", t("Проверить подключение", "Check connection"), "refresh", !native)}<small>${t("Только чтение · без изменения системы", "Read-only · no system changes")}</small></div><div class="phone-art" aria-hidden="true"><div class="phone-island"></div><div class="phone-glow">${icon("shield")}</div><span>MegaNugget</span><small>RESEARCH EDITION</small><div class="phone-bar"></div></div><span class="hero-version">PREVIEW 0.1</span></div>
  <div class="stats"><div class="stat card"><span>${icon("usb")}${t("Соединение", "Connection")}</span><strong>${status()}</strong><small>USB / libimobiledevice</small></div><div class="stat card"><span>${icon("devices")}${t("Устройство", "Device")}</span><strong>${e(d?.info?.ProductType ?? "—")}</strong><small>${d?.info ? `iOS ${e(d.info.ProductVersion)}` : t("Подключите iPhone кабелем", "Connect an iPhone via USB")}</small></div><div class="stat card"><span>${icon("shield")}Jailbreak</span><strong>${t("Не реализован", "Not implemented")}</strong><small>${t("Проверенной цепочки пока нет", "No verified exploit chain")}</small></div></div>
  <div class="two-col"><section class="card panel"><div class="section-heading"><h3>${t("Ваше устройство", "Your device")}</h3><span class="pill">USB</span></div>${deviceSelector()}${deviceDetails()}</section><section class="card panel"><span class="eyebrow">MEGANUGGET IOS</span><h3 class="large-title">${t("Исследование начинается<br>на устройстве.", "Research starts<br>on your device.")}</h3><p>${t("Соберите приложение в Xcode, подпишите его своей командой Apple и выберите экспортированный IPA.", "Build the app in Xcode, sign it with your Apple team, then select the exported IPA.")}</p><div class="note">${icon("shield")}<span>${t("Наличие приложения не даёт привилегий ядра. Установка не выполняет jailbreak.", "Installing the app grants no kernel privileges and does not jailbreak the device.")}</span></div>${button("go-installer", t("Открыть установщик", "Open installer"), "arrow", false, true)}</section></div>`;
}
function devices(): string {
  const failure = scan?.failure ?? current()?.device.failure;
  return `<section class="card panel"><div class="section-heading"><div><span class="eyebrow">USB DEVICE MANAGER</span><h2>${status()}</h2></div>${button("scan", t("Повторить проверку", "Scan again"), "refresh", !native)}</div>${deviceSelector()}<div class="two-col"><div>${deviceDetails()}</div><div class="connection-visual">${icon("usb")}<span>USB → usbmuxd → lockdown</span><small>${t("Сопряжение не запускается автоматически", "Pairing is not initiated automatically")}</small></div></div></section>
  <div class="two-col"><section class="card panel"><h3>${t("Совместимость", "Compatibility")}</h3><span class="pill warning">${compatibility()}</span><p>${t("Точная цель: iPhone9,1 / iPhone9,3 · A10 · ARM64 · iOS 15.8.8. Это цель исследования, не заявление о рабочем jailbreak.", "Exact target: iPhone9,1 / iPhone9,3 · A10 · ARM64 · iOS 15.8.8. This is a research target, not a working jailbreak claim.")}</p><p>${t("iOS 15–17 на других конфигурациях не проверены.", "Other iOS 15–17 configurations are unverified.")}</p></section><section class="card panel"><h3>${t("Диагностика подключения", "Connection diagnostics")}</h3>${row(t("Результат", "Result"), failure ?? (scan ? t("Команда USB выполнена", "USB command completed") : t("Проверка ещё не запускалась", "Not scanned yet")))}<ol class="checklist"><li>${t("Разблокируйте iPhone и подтвердите «Доверять».", "Unlock iPhone and confirm “Trust”.")}</li><li>${t("Используйте кабель передачи данных и другой USB-порт.", "Use a data cable and try another USB port.")}</li><li>${t("Проверьте usbmuxd / драйвер Apple и инструменты в PATH.", "Check usbmuxd / Apple drivers and tools on PATH.")}</li></ol><small>${t("Неудача чтения не позволяет определить причину доверия или блокировки.", "A read failure does not establish trust or lock state.")}</small></section></div>`;
}
function installer(): string {
  const available = !!scan?.installer.Ok;
  return `<div class="notice-banner">${icon("shield")}<div><strong>Apple Code Signing</strong><p>${t("Требуется корректно подписанный IPA с профилем для вашего устройства. MegaNugget не подписывает приложения и не обходит проверки Apple.", "A valid signed IPA provisioned for your device is required. MegaNugget neither signs apps nor bypasses Apple checks.")}</p></div></div><section class="card panel"><span class="eyebrow">APP INSTALLER</span><h2>${t("Установка, без магии.", "Installation, not magic.")}</h2><div class="steps"><span class="active">01 ${t("Выберите IPA", "Choose IPA")}</span><span class="${ipa ? "active" : ""}">02 ${t("Проверьте условия", "Review prerequisites")}</span><span>03 ${t("Подтвердите установку", "Confirm installation")}</span></div><div class="file-zone">${icon("installer")}<h3>${ipa ? e(ipa.bundle_id) : t("Выберите подписанный IPA-файл", "Select a signed IPA file")}</h3><p>${t("Локальная проверка структуры · файл никуда не отправляется до установки", "Local structure validation · no upload until installation")}</p>${button("choose", t("Выбрать файл", "Choose file"), "installer", !native, true)}</div>${ipa ? `${row("Bundle ID", ipa.bundle_id)}${row(t("Версия сборки", "Build version"), ipa.version)}${row(t("Минимальная iOS", "Minimum iOS"), ipa.minimum_ios)}<p class="warning-text">${t("Файлы подписи найдены. Криптографическая корректность, срок профиля, entitlements и разрешённый UDID локально не проверены — окончательное решение принимает iOS.", "Signing artifacts found. Signature validity, profile expiry, entitlements and allowed UDID are not validated locally; iOS is authoritative.")}</p>` : ""}${deviceSelector()}${row(t("Метод установки", "Installation method"), available ? `ideviceinstaller · ${scan?.installer.Ok}` : t("Недоступен / ещё не проверен", "Unavailable / not checked"))}<div class="install-footer"><span>${t("Установка может заменить приложение с тем же Bundle ID.", "Installation may replace an app with the same bundle ID.")}</span>${button("install", busy ? t("Операция выполняется…", "Operation in progress…") : t("Установить на iPhone", "Install on iPhone"), "installer", !native || !canInstall(scan, current(), ipa, busy))}</div></section><p class="footnote">${t("Процесс показывает реальное ожидание команды, без вымышленного процента. После команды проверяются Bundle ID и версия в списке установленных приложений. Это не подтверждает запуск приложения.", "Progress is indeterminate, not a fabricated percentage. Bundle ID and build version are checked after installation. This does not verify app launch.")}</p>`;
}
function journal(): string {
  const entries = logs.filter((x) => level === "all" || x.level === level);
  return `<section class="card panel"><div class="section-heading"><div><h3>${t("Диагностический журнал", "Diagnostic journal")}</h3><p>${t("Только разрешённые коды событий. Без UDID, серийных номеров, путей и вывода команд.", "Allowlisted event codes only. No UDIDs, serial numbers, paths or command output.")}</p></div>${button("export", t("Экспорт JSON", "Export JSON"), "installer", !native, true)}</div><label>${t("Уровень", "Level")} <select id="level">${["all", "info", "warning", "error"].map((x) => `<option ${x === level ? "selected" : ""} value="${x}">${e(x)}</option>`).join("")}</select></label><div class="log-table"><div class="log-row log-header"><span>${t("Время", "Time")}</span><span>${t("Уровень", "Level")}</span><span>${t("Событие", "Event")}</span></div>${entries.length ? entries.map((x) => `<div class="log-row"><time>${e(new Date(x.timestamp * 1000).toLocaleTimeString(prefs.locale))}</time><span class="log-level ${e(x.level)}">${e(x.level)}</span><code>${e(x.event)}</code></div>`).join("") : `<div class="empty-state">${icon("logs")}<h3>${t("Пока тихо", "All quiet for now")}</h3><p>${t("Запустите проверку устройства, чтобы увидеть события.", "Run a device scan to see diagnostic events.")}</p></div>`}</div></section>`;
}
function settings(): string {
  return `<section class="card panel settings-panel"><h3>${t("Внешний вид и язык", "Appearance and language")}</h3><label class="setting-row"><span>${t("Тема интерфейса", "Interface theme")}<small>${t("Тёмная по умолчанию", "Dark by default")}</small></span><select id="theme"><option value="dark" ${prefs.theme === "dark" ? "selected" : ""}>${t("Тёмная", "Dark")}</option><option value="light" ${prefs.theme === "light" ? "selected" : ""}>${t("Светлая", "Light")}</option></select></label><label class="setting-row"><span>${t("Язык", "Language")}</span><select id="locale"><option value="ru" ${prefs.locale === "ru" ? "selected" : ""}>Русский</option><option value="en" ${prefs.locale === "en" ? "selected" : ""}>English</option></select></label><h3>${t("Приватность и диагностика", "Privacy and diagnostics")}</h3><label class="setting-row"><span>${t("Скрывать UDID на экране", "Mask UDID on screen")}<small>${t("Экспорт всегда исключает идентификаторы", "Exports always exclude identifiers")}</small></span><input id="mask" type="checkbox" ${prefs.mask ? "checked" : ""}></label><label class="setting-row"><span>${t("Проверка USB при запуске", "Scan USB on launch")}<small>${t("Только чтение. По умолчанию выключено.", "Read-only. Disabled by default.")}</small></span><input id="startup" type="checkbox" ${prefs.scanOnLaunch ? "checked" : ""}></label>${row(t("Телеметрия", "Telemetry"), t("Отсутствует", "None"))}${row(t("Хранение журнала", "Journal retention"), t("В памяти, до 500 событий", "In memory, up to 500 events"))}<h3>${t("О проекте", "About")}</h3>${row(t("Версия", "Version"), "0.1.0 · research preview")}${row(t("Лицензия проекта", "Project license"), "AGPL-3.0-only")}<p>Copyright © 2026 MegaNugget contributors. ${t("Без гарантий. Разрешено распространение и изменение на условиях AGPL-3.0; полный текст в LICENSE. Первоначальное MIT-уведомление сохранено в LICENSES. Зависимости имеют собственные лицензии; см. docs/THIRD_PARTY.md. JailbreakEngine: not_implemented.", "No warranty. Redistribution and modification permitted under AGPL-3.0; full terms in LICENSE. Initial MIT notice retained in LICENSES. Dependencies retain their licenses; see docs/THIRD_PARTY.md. JailbreakEngine: not_implemented.")}</p></section>`;
}
function render(): void {
  document.documentElement.lang = prefs.locale;
  document.documentElement.dataset.theme = prefs.theme;
  const titles = names();
  root.innerHTML = `<aside class="sidebar"><a class="brand" href="#home" aria-label="MegaNugget Home"><span class="brand-mark">M<span>·</span></span><span>MegaNugget<small>DEVICE RESEARCH PLATFORM</small></span></a><span class="nav-label">WORKSPACE</span><nav>${(Object.keys(titles) as Page[]).map((key) => `<button data-page="${key}" class="nav-item ${key === page ? "selected" : ""}" ${key === page ? 'aria-current="page"' : ""}>${icon(key)}${titles[key]}${key === "devices" && scan?.devices.length ? '<span class="nav-dot"></span>' : ""}</button>`).join("")}</nav><div class="sidebar-bottom"><div class="research-badge">${icon("shield")}<span>${t("Прозрачно по замыслу", "Transparent by design")}<small>${t("Только проверенные возможности", "Only verified capabilities")}</small></span></div><div class="version"><span class="tiny-dot"></span> RESEARCH PREVIEW <span>v0.1.0</span></div></div></aside><main><header class="topbar"><span>MegaNugget <span class="slash">/</span> <strong>${titles[page]}</strong></span><span class="top-status"><span class="tiny-dot ${scan?.failure ? "amber" : ""}"></span>${status()}</span></header><div class="content"><div class="page-heading"><div><span class="eyebrow">YOUR DEVICE. YOUR RESEARCH.</span><h1>${titles[page]}</h1><p>${t("Контроль начинается с понимания.", "Control starts with understanding.")}</p></div><span class="pill outline">${icon("shield")}${t("Без скрытых действий", "No hidden operations")}</span></div>${!native ? `<div class="notice-banner compact">${t("Веб-предпросмотр: нативный backend отсутствует. USB, выбор IPA и установка недоступны. Данные устройства не симулируются.", "Web preview: native backend absent. USB, IPA selection and installation are unavailable. No device data is simulated.")}</div>` : ""}${notice ? `<div role="status" class="notice-banner result">${e(notice)}</div>` : ""}${busy ? `<div role="status" class="busy"><span class="spinner"></span>${t("Ожидание реальной операции. Не отключайте устройство при установке.", "Waiting for the real operation. Do not disconnect during installation.")}</div>` : ""}${{ home, devices, installer, logs: journal, settings }[page]()}<footer>MegaNugget <span>Open source. Open possibilities.</span><span>${t("Исследуйте ответственно.", "Research responsibly.")}</span></footer></div></main>`;
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
    notice = `${t("Операция не подтверждена", "Operation not confirmed")}: ${String(error)}`;
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
    if (native)
      void action(async () => {
        ipa = null;
        ipa = await invoke<Ipa | null>("choose_ipa");
      });
  });
  on("install", () => {
    if (native && canInstall(scan, current(), ipa, busy))
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
