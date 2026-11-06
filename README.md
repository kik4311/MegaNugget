# MegaNugget

**Кастомизация старых iOS без jailbreak**: SwiftUI-клиент и Tauri Desktop
для подписи/установки. Первая цель переноса — **iPhone 7 (iPhone9,1/9,3), iOS 15.8.8**.
Это цель разработки, а не заявление о работающих системных твиках.

Ориентир — [GoldenNugget Mobile](https://github.com/GoldenNugget-Team/GoldenNugget-mobile)
и его авторы. Это не официальный клиент и не готовый backport GoldenNugget.
Его исходники и брендинг пока не копировались; собственный интерфейс MegaNugget
сохраняется. Аудит и границы переноса: [LEGACY_CUSTOMIZATION.md](docs/LEGACY_CUSTOMIZATION.md).

Направление Dopamine остановлено. Старый `jb-orchestrator` оставлен для истории,
исключён из Cargo default-members и не подключён к приложению. Старый
`JailbreakEngine` остаётся заглушкой; в новом интерфейсе его кнопки нет.

Не используйте проект на чужих устройствах без разрешения владельца.

## Что реализовано

- Desktop: Home, Device Manager, App Installer, Logs, Settings; RU/EN, две темы.
- Реальная USB-проверка `idevice_id -l`, чтение plist через `ideviceinfo`.
  Ошибки инструментов не маскируются под отсутствие устройства.
- Точная классификация `iPhone9,1` / `iPhone9,3` + iOS `15.8.8` как **цели исследования**.
- iOS: каталог кастомизации, настройки и журнал вместо проверки jailbreak.
  Реально работает выбор темы **самого приложения** (системная/тёмная/светлая),
  сохранённый локально. Системные функции отмечены как недоступные:
  status bar, SpringBoard, обои и службы ещё не перенесены.
- Локальный выбор IPA, ограниченная проверка структуры/артефактов подписи,
  временная копия проверенного файла, явное системное подтверждение установки.
- Вызов внешнего `ideviceinstaller` (старый/новый CLI), проверка Bundle ID и build.
- Экспериментальная подпись собственного MegaNugget через Apple Account.
  Подробнее в [APPLE_SIGNING.md](docs/APPLE_SIGNING.md).
- Обезличенный JSON-журнал: закрытый набор кодов событий, без вывода команд.
- Исходники SwiftUI, тестируемая Swift-библиотека, генерация проекта Xcode через XcodeGen.
- Rust/TypeScript/Swift-тесты и конфигурация CI для Linux, Windows, macOS и iOS build.

## Быстрый старт Desktop

Установите **Rust stable**, **Node.js 22.12+** (рекомендуется актуальная LTS),
платформенные зависимости Tauri и инструменты libimobiledevice в `PATH`.
Подробные инструкции для каждой ОС: [DEVELOPMENT.md](docs/DEVELOPMENT.md).

```sh
cd apps/desktop
npm ci
npm run tauri dev
```

Сборка исполняемого файла: `npm run tauri build -- --no-bundle`.
Дистрибутивы/подпись Desktop пока не настроены; `bundle.active = false`.

`npm run dev` запускает **только веб-предпросмотр**: никаких выдуманных USB-данных,
выбор файла и установка в браузере отключены.

## Сборка iOS

Только macOS с Xcode и XcodeGen (`brew install xcodegen`):

```sh
cd apps/ios
swift test
xcodegen generate
open MegaNugget.xcodeproj
```

Выберите свою Signing Team и уникальный Bundle ID, deployment target iOS 15.0.
Для реального устройства используйте Xcode Run либо Archive → Distribute App
с доступным вам методом provisioning. Экспорт IPA доступен не для всех типов
аккаунтов/профилей; Personal Team не гарантирует экспортируемый IPA.
Desktop не решает эти ограничения. См. [CODE_SIGNING.md](docs/CODE_SIGNING.md).

## Проверки

```sh
cargo test --locked
cargo run -p device-manager --example probe
cd apps/desktop
npm test
npm run build
```

`probe` выводит **реальные UDID** в терминал при подключении. Не публикуйте его
необработанный вывод; для отчётов используйте Logs → Export JSON.
Фактически выполненные проверки: [TESTING.md](docs/TESTING.md).

## Документация

- [Архитектура и границы привилегий](docs/ARCHITECTURE.md)
- [Совместимость и отсутствие jailbreak](docs/COMPATIBILITY.md)
- [Разработка и запуск на трёх ОС](docs/DEVELOPMENT.md)
- [Подпись и установка](docs/CODE_SIGNING.md)
- [GitHub Actions и неподписанный iOS-артефакт](docs/GITHUB_BUILDS.md)
- [Восстановление](docs/RECOVERY.md)
- [Дорожная карта](docs/ROADMAP.md)
- [Сторонние зависимости](docs/THIRD_PARTY.md)

Основная лицензия проекта — AGPL-3.0 (см. LICENSE и LICENSING.md).
MIT-уведомление первоначального кода сохранено в LICENSES/MIT-initial-code.txt.
Зависимости сохраняют собственные лицензии.
Проект не связан с Apple и не одобрен Apple. Телеметрии, фоновой отправки данных,
автоматических обновлений и изменения системных разделов нет.
