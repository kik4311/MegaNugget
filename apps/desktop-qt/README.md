# MegaNugget Desktop (Qt 6)

C++/Qt 6 rewrite of the Tauri desktop host.

- Phase 0–1: shell UI (RU/EN, themes) and USB device scan
- Phase 2: IPA choose / validate / install via `ideviceinstaller`
- Phase 3: offline tweak catalog + local XML draft compile (`applied_to_device=false`)
- Phase 4: Apply channel detect/consent backup; Signing tab is an honest stub (isideload still Tauri)

The existing `apps/desktop` Tauri app remains until feature parity.

## Requirements

- CMake 3.21+
- Qt 6 Widgets (MSVC on Windows)
- Visual Studio Build Tools with C++
- `idevice_id` / `ideviceinfo` on `PATH`

## Configure & build (Windows)

```powershell
$env:CMAKE_PREFIX_PATH = "C:\Users\levzh\Qt\6.8.3\msvc2022_64"
cmake -S apps/desktop-qt -B apps/desktop-qt/build -G "Ninja" `
  -DCMAKE_BUILD_TYPE=Release
cmake --build apps/desktop-qt/build
```

If Ninja is missing, use `"Visual Studio 18 2026"` or your installed VS generator.

## Run

```powershell
$env:Path = "C:\msys64\ucrt64\bin;$env:Path"
.\apps\desktop-qt\build\meganugget-desktop-qt.exe
```
