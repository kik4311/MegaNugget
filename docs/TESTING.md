# Результаты проверок первого этапа

Дата среды: 2026-11-05. ОС: Linux Mint 22.3, x86_64.
Rust stable 1.99.1, Node 22.16.0. Инструменты разработки и отсутствовавшие
нативные SDK подготовлены локально в `/tmp/opencode`, без установки системных пакетов.
В обычной среде используйте пакетные зависимости из DEVELOPMENT.md.

## Выполнено

| Проверка | Результат |
|---|---|
| `cargo test --locked` (default core members) | **16 passed**, 0 failed |
| `cargo clippy --all-targets -- -D warnings` | успешно |
| `cargo clippy -p meganugget-desktop --all-targets -- -D warnings` | успешно |
| `cargo fmt --all` / проверка оформления | применено; последующая проверка пройдена |
| `npm test` | **4 passed**, 0 failed |
| `npm run build` | TypeScript check + Vite production build успешно |
| `npm run format:check` | успешно |
| `npm audit` | 0 найденных уязвимостей в разрешённом npm-графе на момент проверки |
| `cargo build -p meganugget-desktop` | успешно, debug executable |
| `npm run tauri build -- --no-bundle` | успешно, оптимизированный Linux executable |
| Debug runtime smoke | процесс работал 7 секунд, затем штатно завершён тестом; stdout/stderr пусты |
| Standalone release runtime smoke | процесс работал 5 секунд; X11 подтвердил окно `MegaNugget` 1240×840; stdout/stderr пусты |
| Browser UI | все 5 разделов открываются; RU/EN и light/dark сохраняются; overflow отсутствовал в проверенном viewport 1296×863; console errors отсутствуют |
| Browser native gating | USB scan, выбор IPA, установка и export отключены; указано отсутствие native backend, никаких фиктивных device data |
| `cargo run -p device-manager --example probe` | реальный результат: `devices: []`, `failure: command_failed` |
| `idevice_id -l` | exit 255, `Unable to retrieve device list` |
| `lsusb` | устройство Apple не обнаружено |
| `ideviceinstaller --help` | в среде есть legacy CLI; справка прочитана |

До успешной нативной сборки исправлены реальные ошибки: отсутствовал Wayland/GTK/WebKit
development SDK, затем требуемая Tauri иконка. SDK подготовлен отдельно, иконки добавлены.
Первоначальный npm-граф включал уязвимые dev-зависимости Vitest; обновлён до текущего
Vite/Vitest, повторный audit вернул 0. Это не полный security-аудит Rust/системных библиотек.

## Покрытие автоматических тестов

- Точные модель/версия цели, отсутствие jailbreak-обещаний.
- USB: пустой список, plist сведений, неизвестный trust при ошибке, некорректный UDID,
  отсутствующий executable, распространение таймаута, ограничение объёма вывода.
- IPA: отсутствие signing artifacts, traversal, несколько корневых приложений,
  snapshot при изменении исходного файла, явное отличие artifacts от валидной подписи.
- Installer: контракт legacy/modern CLI, положительная проверка bundle/build и
  отказ при отсутствии приложения. **Это Fake Runner, не аппаратная установка.**
- Диагностика: ограниченный размер, закрытая схема export.
- Frontend: HTML escaping, маскирование UDID, валидация настроек и prerequisites установки.

## Не проверено

- Swift tests / SwiftUI compilation: Swift и Xcode в текущем Linux-окружении отсутствуют.
  Исходники и macOS CI подготовлены; успешная Xcode-сборка **не утверждается**.
- Физический iPhone 7, заявленная iOS 15.8.8/BuildVersion, реальное доверие и USB.
- Установка/upgrade IPA, подпись, запуск iOS-приложения, потери соединения при install.
- Windows и macOS сборки/драйверы/GUI. Матрица GitHub Actions добавлена, но удалённые
  workflow здесь не запускались и результатов GitHub CI пока нет.
- Native GUI взаимодействие с IPA/file picker/confirmation. Runtime smoke доказывает
  запуск и создание окна, а не полную end-to-end проверку всех IPC-команд.
- Jailbreak, rootless-bootstrap, kernel privileges и recovery restore: не реализованы.

Ручная матрица следующего этапа: [../tests/HARDWARE.md](../tests/HARDWARE.md).
