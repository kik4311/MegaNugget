# Сторонние компоненты

Основная лицензия MegaNugget — AGPL-3.0-only; первоначальное MIT-уведомление
сохранено в LICENSES/MIT-initial-code.txt (см. LICENSING.md).
Лицензия проекта не заменяет лицензий зависимостей.
Никакие чужие jailbreak-компоненты, payloads или CLI-бинарники не включены.

| Компонент | Использование | Upstream / лицензия |
|---|---|---|
| Tauri 2 | host, IPC, WebView | https://github.com/tauri-apps/tauri — MIT / Apache-2.0 |
| Rust serde / serde_json | сериализация | https://github.com/serde-rs — MIT / Apache-2.0 |
| plist | XML/binary plist | https://github.com/ebarnard/rust-plist — MIT |
| zip-rs | ограниченное чтение ZIP | https://github.com/zip-rs/zip2 — MIT |
| rfd | системные диалоги | https://github.com/PolyMeilex/rfd — MIT |
| tempfile / wait-timeout | snapshot / контроль процессов | MIT / Apache-2.0 |
| TypeScript / Vite / Vitest | frontend build/tests | Apache-2.0 / MIT / MIT |
| libimobiledevice | внешний CLI/протокол lockdown | https://github.com/libimobiledevice/libimobiledevice — LGPL-2.1 (library), GPL-2.0 (tools); уточнять по файлам |
| ideviceinstaller | внешняя программа установки | https://github.com/libimobiledevice/ideviceinstaller — GPL-2.0 |
| usbmuxd | внешний системный сервис | https://github.com/libimobiledevice/usbmuxd — GPL / LGPL, см. upstream |
| XcodeGen | генератор Xcode project | https://github.com/yonaskolb/XcodeGen — MIT |

Системные WebKitGTK/GTK, WebView2 и Apple frameworks имеют отдельные условия.
Cargo.lock и package-lock.json фиксируют полный граф зависимостей; таблица выше
не является исчерпывающим SBOM. До распространения бинарников создайте полный
лицензионный отчёт и notices для точного target/графа (например cargo-about и
эквивалентный npm inventory), проверьте обязательства поставляемых библиотек.
Если решите поставлять GPL CLI, соблюдайте GPL, включая требования исходников.
Сам по себе запуск отдельного процесса не отменяет требований его распространения.

Изучены upstream README/CLI ideviceinstaller и prerequisites Tauri 2.
В среде разработки обнаружен legacy CLI. Проверены справки команд и ошибка USB;
это не аппаратная валидация установки. Современный CLI покрыт контрактным тестом,
а не физическим устройством. Источники:

- https://libimobiledevice.org/
- https://github.com/libimobiledevice/ideviceinstaller/blob/master/README.md
- https://github.com/libimobiledevice/ideviceinstaller/blob/master/src/ideviceinstaller.c
- https://v2.tauri.app/start/prerequisites/
