# Разработка

## Общие зависимости

- Rust stable + `rustfmt`, `clippy` (см. `rust-toolchain.toml`).
- Node.js >=22.12, рекомендуемая поддерживаемая LTS; npm.
- Tauri 2 системные зависимости: https://v2.tauri.app/start/prerequisites/
- `idevice_id`, `ideviceinfo`, `ideviceinstaller` в доверенном `PATH`.
  Они не входят в репозиторий и не загружаются приложением автоматически.
- Интернет для первоначального разрешения Cargo/npm зависимостей.

## Linux x86_64

Для Ubuntu 24.04 / Mint 22 (другим дистрибутивам нужны эквивалентные пакеты):

```sh
sudo apt update
sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev libwayland-dev libdbus-1-dev \
  usbmuxd libimobiledevice-utils ideviceinstaller
```

Запуск от обычного пользователя, не от root. Проверяйте usbmuxd и штатные udev rules
дистрибутива; не меняйте USB-права на `777`. Разблокируйте телефон и подтвердите доверие.

```sh
cd apps/desktop
npm ci
npm run tauri dev
```

## macOS Intel / Apple Silicon

Установите Xcode/Command Line Tools, Rust подходящей архитектуры, Node LTS.

```sh
xcode-select --install
brew install libimobiledevice ideviceinstaller
cd apps/desktop
npm ci
npm run tauri dev
```

GUI-приложения Finder не всегда наследуют Homebrew PATH. До настройки распространения
запускайте из Terminal, где доступны `/opt/homebrew/bin` (ARM) или `/usr/local/bin` (Intel).
Не используйте Rosetta как замену проверке целевой архитектуры.

## Windows 10/11 x64

1. Visual Studio Build Tools: **Desktop development with C++**, Windows SDK.
2. WebView2 Evergreen Runtime, Rust stable MSVC x64, Node LTS x64.
3. Установите официальное Apple Devices / соответствующее ПО Apple с USB-драйвером.
4. Соберите CLI из upstream libimobiledevice/ideviceinstaller либо получите
   проверенную доверенную сборку отдельно. Приложение не предлагает случайные DLL.
   Точное окружение сборки зависит от версии upstream; см. ссылки в THIRD_PARTY.md.
5. Добавьте директорию CLI и необходимых DLL в PATH. Сначала проверьте
   `idevice_id -l`, `ideviceinfo --help`, `ideviceinstaller --help` в PowerShell.

```powershell
cd apps/desktop
npm ci
npm run tauri dev
```

Без CLI Desktop запускается, но показывает `tool_missing`; установка недоступна.
Windows-драйверы и связка Apple Devices/libimobiledevice требуют проверки на железе.

## Сборка Desktop (любая из трёх ОС)

```sh
cd apps/desktop
npm run tauri build -- --no-bundle
```

Результат: `target/release/meganugget-desktop` (Windows: `.exe`) в корне workspace.
Это несопровождаемый подписью исполняемый файл, не production installer.
Для распространения нужны отдельные code signing, macOS notarization,
Windows signing и аудит лицензий runtime/CLI.

## iOS (только macOS + полноценный Xcode)

```sh
brew install xcodegen
cd apps/ios
swift test
xcodegen generate
xcodebuild -project MegaNugget.xcodeproj -scheme MegaNugget \
  -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' \
  CODE_SIGNING_ALLOWED=NO build
open MegaNugget.xcodeproj
```

Swift Package содержит только Core; приложение собирается Xcode. Генерируемый проект
не хранится в git, источник настройки — `project.yml`. Для устройства выберите Signing
Team/Bundle ID в Xcode и действующий provisioning. Не добавляйте jailbreak entitlements.
`swift-tools-version: 5.9` в Package.swift — версия инструментов SwiftPM;
`SWIFT_VERSION: 5.0` в Xcode — режим языка Swift 5, не требование старого компилятора.
Сведения о подписи: [CODE_SIGNING.md](CODE_SIGNING.md).

## Проверки перед PR

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cd apps/desktop
npm ci
npm test
npm run format:check
npm run build
npm audit
```

Корневые Cargo default-members — четыре core-crate: эти тесты не требуют WebKit.
`cargo check -p meganugget-desktop` проверяет нативный host и требует SDK Tauri.
Web-preview не проверяет Rust IPC и не считается запуском Desktop.
