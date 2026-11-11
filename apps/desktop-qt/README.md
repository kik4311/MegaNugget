# MegaNugget Desktop (Qt 6)

C++/Qt 6 rewrite of the Tauri desktop host.

- Phase 0–1: shell UI (RU/EN, themes) and USB device scan
- Phase 2: IPA choose / validate / install via `ideviceinstaller`
- Phase 3: offline tweak catalog + local XML draft compile (`applied_to_device=false`)
- Phase 4: Apply channel tab drives the Rust `apply-channel` CLI: detect, consent backup, verify, **plan**, **merge (`apply`)**, **restore (`--consent-restore`)**, **effect**; journal events `channel_*`
- Signing tab is an honest stub (isideload still Tauri)

The existing `apps/desktop` Tauri app remains until feature parity.

## `apply-channel` binary

The **Применение / Apply** tab shells out to `apply-channel` (crate `crates/apply-channel`). It must be on disk before plan/merge/restore/effect run.

Resolve order (first hit wins):

1. `MegaNugget` / `DesktopQt` setting `apply_channel_path` (Qt `QSettings`)
2. Environment variable `APPLY_CHANNEL_BIN`
3. Next to the Qt executable (`apply-channel` or `apply-channel.exe`)
4. `target/release/apply-channel` or `target/debug/apply-channel` walking up from the executable directory (typical after `cargo build -p apply-channel` in the repo root)

Build from the repository root:

```sh
cargo build -p apply-channel
```

On Windows, copy `target\release\apply-channel.exe` beside `meganugget-desktop-qt.exe`, or set `APPLY_CHANNEL_BIN`.

If the binary is missing, the UI shows `apply_channel_missing` with search hints in the log.

## Apply channel workflow (SBHideLowPowerAlerts)

Target device only: **iPhone9,1** or **iPhone9,3**, **iOS 15.8.8**, no jailbreak.

1. **Detect** — one connected device, `matches_target=true`
2. **Backup** — first consent checkbox → `idevicebackup2` (unchanged); on success the path is offered as the verified backup
3. **Verified backup** — pick folder; **Verify** runs `apply-channel verify <dir>`
4. **Plan** — draft JSON for `SBHideLowPowerAlerts=true` (HomeDomain springboard plist, `accept_case_variants`) → `apply-channel plan`; log shows key, before/after, `digest_verified`
5. **Merge** — `apply-channel apply <backup> <draft> <new_dir>` (default sibling `*-applied`; refuses if the folder already exists)
6. **Restore** — **second** consent + confirmation dialog → `apply-channel restore <applied> --consent-restore` (async, masked UDID only)
7. **Effect** — after a fresh post-restore backup, `apply-channel effect <draft> <pre> <post>` → `matched` / `missing` / `changed` per key

Journal codes include `channel_plan_started`, `channel_merge_finished`, `channel_restore_started`, `channel_effect_finished`, etc.

## Requirements

- CMake 3.21+
- Qt 6 Widgets (+ Qt Test for unit tests)
- Visual Studio Build Tools with C++ (Windows) or GCC/Clang (Linux)
- `idevice_id` / `ideviceinfo` / `idevicebackup2` on `PATH` for device steps
- Built `apply-channel` for plan/merge/restore/effect

## Configure & build (Windows)

```powershell
$env:CMAKE_PREFIX_PATH = "C:\Users\levzh\Qt\6.8.3\msvc2022_64"
cargo build -p apply-channel --release
cmake -S apps/desktop-qt -B apps/desktop-qt/build -G "Ninja" `
  -DCMAKE_BUILD_TYPE=Release
cmake --build apps/desktop-qt/build
ctest --test-dir apps/desktop-qt/build
Copy-Item target\release\apply-channel.exe apps\desktop-qt\build\
```

If Ninja is missing, use `"Visual Studio 18 2026"` or your installed VS generator.

## Configure & build (Linux)

```sh
cargo build -p apply-channel
cmake -S apps/desktop-qt -B apps/desktop-qt/build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build apps/desktop-qt/build
ctest --test-dir apps/desktop-qt/build
cp target/debug/apply-channel apps/desktop-qt/build/
```

## Run

```powershell
$env:Path = "C:\msys64\ucrt64\bin;$env:Path"
.\apps\desktop-qt\build\meganugget-desktop-qt.exe
```

## Unit tests (no device)

`ApplyChannelCli_test` covers draft JSON shape, stable stderr parsing, argv whitelists, and restore consent gating via an injectable CLI hook.
