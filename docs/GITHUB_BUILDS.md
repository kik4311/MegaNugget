# Сборка iOS на GitHub

Репозиторий: https://github.com/kik4311/MegaNugget (приватный; нужна авторизация).
Workflow **MegaNugget checks** запускается по push/PR и вручную (Actions → Run workflow).

Job `ios` выполняет Swift unit tests, генерацию проекта XcodeGen, simulator build,
затем отдельную Release-сборку для физического ARM64 iPhone с deployment target 15.0.
Зависимости/проверки могут потребовать времени платных минут приватного репозитория;
доступность runner/квоты определяет GitHub.

После успешной сборки в Artifacts появляется `MegaNugget-unsigned-device`, срок хранения 7 дней:

- `MegaNugget-UNSIGNED.ipa` — **НЕПОДПИСАННЫЙ**, не устанавливается на обычную iOS;
- `build-info.json` — commit, версия, архитектура и SHA-256;
- `README.txt` — ограничения артефакта.

Для скачивания из терминала:

```sh
gh run list -R kik4311/MegaNugget --workflow ci.yml
gh run download RUN_ID -R kik4311/MegaNugget -n MegaNugget-unsigned-device -D ./unsigned-ios
```

Этот файл **не является проверкой успешной установки**. Desktop должен отклонять его
с `MissingSigningArtifacts`; workflow не подделывает CodeResources/provisioning.
Для положительного теста установки нужен законно подписанный IPA с подходящим профилем,
например экспортированный из Xcode своей командой Apple. См. CODE_SIGNING.md.

Apple ID, сертификаты и профили в этот workflow не загружаются. Не добавляйте ключи
в git, issue или артефакты. Подписанную CI-сборку следует проектировать отдельно,
с отдельным согласованием секретов, разрешений и аккаунта Apple.
