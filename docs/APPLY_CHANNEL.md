# Канал применения твиков (backup/restore)

Дата: 2026-11-06. Статус: **read-only прототип, ничего не пишется в устройство.**
См. также: [GOLDENNUGGET_PORT.md](GOLDENNUGGET_PORT.md) (черновики) и
[TESTING.md](TESTING.md) (результаты). Этот документ описывает единственный
спроектированный путь, которым XML-черновик может когда-либо доехать до
подключённого iPhone **без jailbreak**: локальная защитная копия
(idevicebackup2) → проверка → управляемое слияние → restore.

## Принципы (fail closed)

1. **Сначала защитная копия.** Любая работа начинается с полного локального
   backup устройства. Канал не обязан оперировать, пока backup не собран и
   не проверен (`SnapshotState == "finished"`).
2. **Явное согласие на каждый шаг.** Backup и всё последующее выполняются
   только с явным флагом/кнопкой в интерфейсе. Автоматических запусков нет.
3. **Никаких удалений.** Инструмент никогда не удаляет backup и временные
   копии: только создаёт рядом отчёт/план. Ручная уборка — прерогатива человека.
4. **Минимальные привилегии pairing.** Читаем состояние только уже
   сопряжённого устройства (`lockdown_readable`). Регулярные домены не запрашиваются.
5. **Проверяемое слияние.** В backup изменяются только ключи из явно выбранного
   черновика; каждый изменяемый plist проходит пофайловой diff и валидацию
   checksum Manifest; результат `plan` человек читает глазами до применения.
6. **Право писать появляется только после проверок.** `DRAFT_CANDIDATE` остаётся
   «кандидатом» до фактического подтверждения эффекта на устройстве; success
   restore ≠ эффект твика (нужно чтение после restore).
7. **Приватность.** Backup остаётся локально; вывод команд — только белый
   список полей (Product Type, Product Version, замаскированный UDID). Серийные
   номера/IMEI/телефон в логи и stdout не попадают.

## Ограничения, известные честно

- Поведение restore по доменам не одинаково: часть системных plists могла быть
  сброшена iOS после restore; какой путь переживёт restore на iOS 15.8.8 —
  неизвестно и проверяется только на устройстве (per ключ/домен).
- `idevicebackup2 restore` может требовать стадию "Settings" и игнорировать
  отдельные домены; план B — обновление backup (`-b`) + restore_domains.
- Расписание этого этапа не обещает дату: каждое связующее звено проверяется
  отдельно, начиная с read-only.

## Этапы

| Этап | Содержимое | Статус |
|---|---|---|
| 1 | Read-only: `detect`, `inspect`, `verify`, защитный `backup` (с consent) | реализовано; **реальный backup выполнен** (SnapshotState finished, 3,9 ГБ) |
| 2 | `plan`: слияние draft ↔ backup (bplist diff, checksum Manifest) | реализовано; **реальный план построен** на живом backup (checksum OK, `add_key`) |
| 3 | Собственно apply (restore) + per-key чтение эффекта + rollback | не начат |
| 4 | Desktop UI + журнал-события (`backup_started`…) | не начат |

## CLI прототипа (`crates/apply-channel`)

```sh
apply-channel detect                  # устройство, гейт цели, `matches_target`
apply-channel inspect <dir>           # разбор структуры backup (Info/Status/макет)
apply-channel verify <dir>            # SnapshotState == finished, гейт модели/версии
apply-channel backup <dir> --consent-backup   # только с явным согласием
```

Вывод — однострочный JSON из белых списков (UDID только замаскированный), ошибки
— `error: <код>` в stderr с exit 1. Backup работает строго с единственным
подключённым устройством: ноль — `device_not_found`, два и больше — тоже отказ
вместо угадывания.

Кодов ошибок (стабильные snake_case):

- `usb_failed`, `lockdown_unreadable`, `device_not_found` — read-only read.
- `target_mismatch` — подключённое устройство не совпало с целевым iPhone 7/15.8.8 (или внутри backup).
- `backup_tool_missing`, `backup_requires_consent`, `backup_failed`, `backup_unfinished`
- `backup_layout_unsupported` — макет не распознан; **никакой записи** по неизвестному макету.

Restore-команды в прототипе **нет и не будет**, пока не пройдёт этап 2/3:
нарушение этого правила делает канал несуществующим, что бы ни декларировалось.

## Этап 2: план слияния (реализовано 2026-11-06)

```sh
apply-channel plan <backup_dir> <draft.json> <out.json>
```

Команда только читает backup и черновик; единственная запись — сам план
(`out.json`, существующий файл не перезаписывается — `plan_output_exists`).
Новые коды ошибок: `plan_item_not_found`, `plan_ambiguous_target`,
`plan_digest_mismatch`, `plan_unsupported_kind`, `plan_bad_draft`,
`plan_output_exists`.

Формат черновика (desktop-этап 4 будет строить его из каталога и явных выборов):

```json
{"items": [{
  "id": "SBHideLowPowerAlerts", "kind": "toggle",
  "key": "SBHideLowPowerAlerts", "domain": "HomeDomain",
  "relative_path": "Library/Preferences/com.apple.springboard.plist",
  "value": true
}]}
```

Политика целевого файла: каталог GoldenNugget хранит путь твика
(`/var/Managed Preferences/mobile/...`), а в backup такие файлы отсутствуют
(IsFullBackup=false), поэтому этап 4 обязан явно задавать перекомпоновку
спецификации → домен/путь backup — без доверия по умолчанию.

Проверяемые шаги планировщика:

1. verify-гейты (готовность backup + точная модель/версия);
2. в Manifest.db ищется ровно одна запись (домен + относительный путь)
   с `flags=1`; если по NOCASE найдены ещё варианты того же пути — отказ
   `plan_ambiguous_target`, а при явном `accept_case_variants: true` в
   элементах черновика план строится по точному регистру, сосед остаётся
   нетронутым и попадает в отчёт (`case_variants`);
3. блоб метаданных `file` (NSKeyedArchiver: `Digest` — SHA-1, `Size`,
   `RelativePath`) расшифровывается; Digest может быть ссылкой UID на объект
   bytes (так пишет реальный idevicebackup2) или inline data;
4. SHA-1 реального файла из hash-бакета сверяется с `Digest` и `Size` —
   при отказе `plan_digest_mismatch` ничего не планируется;
5. читается целевой plist (bplist00), для каждого ключа вычисляется
   `add_key`/`replace_value` и старое значение; в план печатается только
   отчёт, файлы не изменяются.

Живой прогон по настоящему backup (draft из примера — тумблер
`SBHideLowPowerAlerts = true`): план построен без ошибок; в отчёте
`digest_verified: true`, файл 3244 байта, 26 ключей, `case_variants: 2`
(в манифесте блокирует оба регистра: настоящий
`com.apple.springboard.plist` и 42-байтовый огрызок
`com.apple.SpringBoard.plist`), действие `add_key` (ключа в plist нет).

## Что делает прототип НЕ делает

- Не запускает backup автоматически, не «предваряет» согласие молчаливым
  флагом и ничего не подменяет.
- Не пишет в backup, не изменяет Manifest, не сжимает/не распаковывает файлы.
- Не производит «предварительный» restore доменов, не «тестирует» запись.
- Не хранит UDID и серийные номера в файлах отчётов; в stdout — только белый список.

## Реальный backup (2026-11-06, первый консент-гейтованный прогон)

Одна команда: `apply-channel backup <home>/.local/share/meganugget/backups/2026-11-06-iphone7-first --consent-backup`.

- Время ~0–10 минут, объём **3,9 ГБ**, SnapshotState **finished**; IsFullBackup
  зафиксирован `false` (протокол idevicebackup2 1.3.0 первого прохода; состав
  проверяется на этапе 2 по содержимому Manifest.db, домены и их счётчики уже
  там).
- Макет подтверждён `inspect`: **manifest_sqlite** — `Manifest.db` (SQLite,
  ~27 МБ), `Manifest.plist`, `Info.plist`, `Status.plist` и подкаталоги-бакеты
  `00…ff` (256 шт., по одному на префикс хеша файла).
- Первая версия verify/inspect отказала с `backup_layout_unsupported`: ключи
  Info.plist в backup записаны как «Product Type»/«Product Version» с пробелами
  (формат Apple/libimobiledevice), а код искал `ProductType`. Исправлено:
  принимаются оба варианта; тесты обновлены.
- Вывод CLI — только белые списки: `{"udid_masked":"cb01…7637",
  "product":"iPhone9,3","ios":"15.8.8","snapshot_state":"finished",
  "is_full_backup":false,"layout":"manifest_sqlite","bucket_dirs":256,
  "domain_files":0,"domain_dirs":0,"entries":260}`.
- Ничего не удалено, ничего не записано в устройство; restore-команд по-прежнему
  нет.

## Критерий успеха этапа 1

- На реальном iPhone 9,3/9,1 (iOS 15.8.8) получится: `detect` видит устройство;
  `backup --consent-backup` собирает полную защитную копию за приемлемое время;
  `verify` подтверждает finished; `inspect` печатает безопасный разбор макета.
  После этого канал считается готовым к этапу 2 (слияние draft ↔ backup).
