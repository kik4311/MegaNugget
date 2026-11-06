# Результаты проверок первого этапа

## Текущее направление: Legacy customization (2026-11-06)

- Частичный порт GoldenNugget Mobile: пять реальных Foundation Swift-файлов,
  закреплённая ревизия, неизменённые лицензия/полные Credits, хэши и notices.
  Добавлены восемь Swift-тестов адаптера/компилятора и два Python-теста происхождения.
  До нового macOS CI Swift-компиляция и эти новые Swift-тесты не считались проверенными;
  macOS CI run `42234299147` (коммит `d9e9d3c`): все 5 jobs зелёные, ios — `swift test`
  и сборки SwiftUI прошли. Системное применение и аппаратная матрица iOS 15–17
  по-прежнему не выполнялись.

- Desktop-черновики твиков (та же сессия): модель/компилятор портированы в Rust-крейт
  `crates/customization` (catalog.json, strict-версии, гейты, компилятор выборки),
  IPC `tweaks_catalog`/`tweaks_preview`, страница «Твики · черновики». Все локальные
  проверки зелёные: cargo fmt/clippy/test по всему workspace, prettier/Vitest (10)/
  tsc+Vite build, 3 Python-теста (включая `--check` снепшота). Черновик — только
  локальный XML; применение на устройство не реализовано. Запуск страницы в
  реальном GTK-окне не проверялся; веб-предпросмотр показывает заглушку без IPC.

- Добавлен статический сайт `apps/website`: русскоязычный лендинг без внешних
  ресурсов, аналитики и форм аккаунта. Иллюстрация помечена как концепция,
  неперенесённые системные твики не объявлены доступными. Локально в Chromium
  проверены desktop (1296px), узкие viewport 316/356/386/764px через same-origin
  frames, загрузка логотипов, отсутствие горизонтального переполнения,
  одиночное раскрытие FAQ. Ошибок консоли нет. Это не тест Safari на iPhone.
  Добавлены 4 теста сайта; вместе с branding/упаковщиком — 14 Python-тестов.
  Публичный хостинг и домен не настроены, сайт доступен только локально.

- Общий логотип: источник SVG, 24 генерируемых файла (Desktop/web/iOS), manifest
  хэшей и 4 новых Python-теста. Всего 10 Python-тестов и 6 frontend-тестов прошли;
  TypeScript/Vite и нативная release-сборка успешны. В браузере логотипы загрузились
  в боковой панели, на главной и в настройках, в светлой и тёмной теме;
  горизонтального переполнения в проверенном viewport нет.
  iOS AppIcon проверен по размерам и отсутствию alpha; SwiftUI и actool для этого
  изменения должны пройти macOS CI. Внешний вид иконок Windows/macOS не проверен.
  См. [BRANDING.md](BRANDING.md).

- Desktop переоформлен под кастомизацию: удалена карточка Jailbreak, обновлены
  главный экран, сведения о цели переноса, брендовые надписи и текст о проекте RU/EN.
  USB, подпись и установка не менялись. TypeScript/Vite build и 6 frontend-тестов
  прошли. В браузере проверены RU/EN, отсутствие старой карточки и горизонтального
  переполнения в текущем viewport. Веб-проверка не проверяет native IPC.
  Временные Node/Rust/SDK восстановлены вне системных пакетов; новая нативная
  release-сборка через `tauri build --no-bundle` прошла и запущена отдельно.

- Новый iOS UI: каталог / настройки / журнал; реальная локальная тема через AppStorage.
- Добавлены 3 Swift-теста: локальное оформление, точный аппаратный target и отказ
  всех системных функций. Swift/Xcode локально недоступны; до нового macOS CI
  эти тесты и компиляция SwiftUI не считаются выполненными.
- Подпись/установку прежней версии пользователь подтвердил словами «Все работает»;
  это пользовательский отчёт, не независимое аппаратное тестирование новой версии.
- Старый jb-orchestrator исключён из default-members и не используется UI.
- Локально: 6 Python-тестов упаковщика и `git diff --check` прошли.
  Rust повторно не запускался: временный toolchain предыдущей сессии отсутствует.
- Предыдущие отчёты ниже относятся к своим этапам, не доказывают перенос твиков.

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

## Дополнение: первый завершённый GitHub CI

Run https://github.com/kik4311/MegaNugget/actions/runs/42080206506,
commit `a0796a531d8e08995722511ccd40d8e904ca30d1`:

- iOS: Swift tests, SwiftUI simulator build, Release ARM64 device build и выгрузка
  **неподписанного** IPA прошли. Это заменяет прежний статус «Xcode не проверен»
  для данного коммита, но не доказывает установку/запуск на физическом iPhone.
- Desktop: Linux x86_64, macOS ARM и macOS Intel native builds прошли.
- Windows: Rust и TypeScript tests прошли; проверка Prettier остановила job из-за
  переводов строк при Windows checkout. Компиляция Windows host в этом запуске
  не выполнялась. Добавлен `.gitattributes` с LF для text-файлов; нужен новый CI.
- 6 новых Python-тестов упаковщика прошли локально и в iOS job. `lipo` в unit tests
  подменяется; реальная архитектура проверяется отдельно скриптом в CI.
- Артефакт `MegaNugget-unsigned-device` содержит `MegaNugget-UNSIGNED.ipa`, manifest
  и предупреждение. Provisioning/сертификатов нет; установка на обычную iOS недоступна.

Предыдущий запуск `42080021825` отменён после исправления Xcode language mode;
его отметки отмены не используются как доказательство ошибок тестов.

## Дополнение: экспериментальный Apple Account signer

Добавлены `apple-signing`, отдельный тип неподписанного IPA, native consent,
локальная форма Apple Account/2FA/Team, отмена и экспорт результата.

- Linux: обновлённый host прошёл `cargo check`, Clippy с `-D warnings`, release build
  через Tauri и runtime smoke (5 секунд, stdout/stderr без ошибок).
- Rust: **24 unit tests passed**, включая передачу/отмену 2FA challenge, gate версии iOS,
  запрет прямой установки unsigned IPA, безопасное извлечение и запрет frameworks/PlugIns.
- Ошибки IPC выбора/установки IPA переведены на стабильные snake_case-коды
  (`unsupported_signing_bundle`, `signing_content_unsupported` и др.), которые фронтенд
  переводит в понятные RU/EN-подсказки; Debug-имена вариантов больше не показываются.
- TypeScript: **6 tests passed**, включая запрет установки до подписи, состояния операции
  и отсутствие credential-полей в сериализуемых настройках. TypeScript/Vite build успешен.
- Web-preview: новая форма видна, без native backend её поля/подпись заблокированы;
  consent присутствует, переполнения в проверенном viewport нет, credentials в storage нет.
- Реальные Apple Account login/2FA, сетевые изменения аккаунта, выдача сертификата,
  сохранение сертификата в keyring, подпись и установка на физический iPhone **не выполнены**.
  Для этого требуется участие пользователя, его согласие и аккаунт в локальном приложении.
- Зелёный CI `42082367307` относится к коду **до** Apple signer: он подтверждает предыдущий
  этап на всех desktop ОС и iOS, но не новую интеграцию.
- CI для самой интеграции прошёл зелёным: `42091396498` (коммит `370d04a`, signer, все 5 jobs)
  и `42093526574` (коммит `9401705`, стабильные коды ошибок + локализованные подсказки).
  Зелёный CI по-прежнему **не означает** реальной Apple-авторизации/подписи/установки.

Без Apple Account не производится никаких «тестовых» фиктивных выдач профиля/подписи.
Старые пункты «не реализовано Apple signing» выше относятся к baseline 0.1.0 до этого дополнения.

## Дополнение 0.2.0: jb-orchestrator

Добавлен Rust-кран `jb-orchestrator` (матрица + проверенные загрузки) для направления
«реальный jailbreak» — см. docs/JAILBREAK_ORCHESTRATION.md.

- Linux: `cargo fmt`, `cargo clippy -D warnings`, `cargo test` — чисто.
- Rust: **16 новых unit tests** — матрица честно отвечает для iPhone 7/15.8.8
  (Dopamine ≥ 2.5b4 + DarkSword + TrollStore + предупреждение bootloops), для 15.8.7,
  ниже 15.8.7 (landa), 16.6.1; отказывается для A11/DarkSword, неизвестных устройств
  и испорченных версий; разбор релизов, точный тег, `.ipa`, лимиты, проверка
  размеров и SHA-256-дайджестов, `sha256_hex` на известных векторах.
- **Не выполнено**: реальная загрузка релиза по сети, подключение к Desktop IPC/UI,
  любой run на реальном iPhone 7, TrollStore/TrollRestore на устройстве, сам
  jailbreak. Всё это требует участия пользователя и его устройства.
- Матрица — **документированные публичные данные** (сверено 2026-11), не гарантия.

## Дополнение: Desktop-черновики твиков (2026-11-06)

Порт модели/компилятора GoldenNugget в Desktop без изменения системы устройства:

| Проверка | Результат |
|---|---|
| `cargo test -p customization` | **12 passed** (3 unit + 9 integration, зеркалируют Swift-тесты) |
| `cargo test --workspace` | все наборы passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | успешно (пустые/непокрытые ветки fail closed) |
| `cargo fmt --all -- --check` | чисто |
| `cargo check -p meganugget-desktop` | успешно; IPC-команды зарегистрированы |
| `npm run format:check` (prettier) | чисто |
| `npm test` (Vitest) | **10 passed** (6 прошлых + 4 новых про твики) |
| `npm run build` | tsc --noEmit + Vite build успешны |
| `python3 -m unittest tests.test_goldennugget_import` | **3 passed**, включая свежесть `catalog.json` через `export_desktop_catalog.py --check` |

Новая страница не способна применить что-либо к устройству структурно: в API нет
результата-канала, `DraftPreview.applied_to_device` константно `false`, журнал
получает только событие `DraftCompiled`. Смена цели перезагружает каталог и удаляет
из черновика потерявшие статус пункты, компиляция повторяет проверки Swift-порта.

Дополнено после записи выше: страница проверена в реальном GTK-окне (нативная
debug-сборка, X11): каталог загрузился через IPC для подключённого устройства,
редактор тумблера (`SBHideLowPowerAlerts` = true) — «Сохранить в черновик»
(1/64), компиляция вернула XML plist с `<true/>`, журнал получил
`draft_compiled`; скан увидел реальное устройство, UDID замаскирован. Штатный
выход; черновик в памяти и исчез после закрытия, на устройстве ничего не
менялось.

**Не проверено:** ввод текстовых значений с клавиатуры (в тесте только через
латинскую раскладку; поле открывается корректно), числовой редактор, смена цели
через селект, настоящий restore-канал, iOS 15–17 на устройстве, поведение
Swift-UI вкладки «Твики» с iOS. CI направления: run `42234299147` (Swift-порт,
ios job) и run `42250519670` (Desktop-черновики, все 5 jobs вкл. ios и windows) —
зелёные; это компиляция и тесты, не работа твиков на устройстве.
