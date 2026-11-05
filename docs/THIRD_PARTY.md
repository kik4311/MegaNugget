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
| Dopamine 2.5b4 | официальный upstream jailbreak-компонент | https://github.com/opa334/Dopamine — GPL-3.0; входит в MegaNugget **только как проверенный загрузочный артефакт**, не как код |
| TrollStore 2.x / TrollRestore | канал установки на устройстве | https://github.com/opa334/TrollStore, https://github.com/JJTech0130/TrollRestore — лицензии upstream |
| isideload 0.5.2 | Apple auth, provisioning, signing; pinned Git revision | https://github.com/nab138/isideload — MIT; notice в LICENSES/isideload-MIT.txt |
| apple-codesign-quick 0.1.0 | транзитивный Mach-O signer | https://github.com/Dadoum/apple-crates — LGPL-2.1-or-later; см. LICENSES/LGPL-2.1.txt |

isideload — компонент подписи, **не jailbreak-компонент**. Его использование не означает
аффилированность или одобрение авторами isideload/iloader. Документация брендинга iloader
и его ресурсы не копируются. Добавление backend существенно расширяет криптографический
граф зависимостей; успешные тесты не являются полным аудитом этого графа.

Dopamine/TrollStore используются как внешние артефакты (не vendored-code), проверяемые
по SHA-256 перед использованием пользователем; GPL-3.0 не «заражает» MegaNugget, потому
что мы не изменяем и не линкуем их исходники. Практика: любая правка этих проектов
остаётся в их репозиториях; отчёты о проблемах отправляются upstream.

Важно: библиотека apple-codesign-quick **не MIT**. LGPL-текст поставляется отдельно.
Исходники неизменённой версии: https://crates.io/api/v1/crates/apple-codesign-quick/0.1.0/download
(точная версия/checksum зафиксированы Cargo.lock). При распространении линкованного
бинарника необходимо выполнить LGPL-обязательства, включая возможность пересборки/
перелинковки с изменённой библиотекой; исходники MegaNugget и lock-файл предоставляются
для пересборки. Не запрещается reverse engineering для отладки таких изменений.
Это уведомление не заменяет полного лицензионного аудита будущего production-дистрибутива.

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
