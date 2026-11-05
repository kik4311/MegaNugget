#!/usr/bin/env python3
"""Package an Xcode device build, never claim it is signed or installable."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import subprocess
import zipfile


def package(app: Path, output: Path) -> Path:
    if not app.is_dir() or app.suffix != ".app":
        raise ValueError("Expected a built .app directory")
    with (app / "Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    if "iPhoneOS" not in info.get("CFBundleSupportedPlatforms", []):
        raise ValueError("A simulator build cannot be packaged as a device IPA")
    minimum = info.get("MinimumOSVersion", "")
    if minimum != "15.0":
        raise ValueError(f"Unexpected deployment target: {minimum!r}; expected 15.0")
    executable = info.get("CFBundleExecutable", "")
    if not executable or Path(executable).name != executable or executable in (".", ".."):
        raise ValueError("Invalid executable name")
    architectures = subprocess.check_output(
        ["lipo", "-archs", str(app / executable)], text=True
    ).strip().split()
    if "arm64" not in architectures:
        raise ValueError("Device build must contain arm64")
    if (app / "embedded.mobileprovision").exists():
        raise ValueError("Do not upload provisioning profiles in unsigned CI artifacts")
    files = sorted(path for path in app.rglob("*") if path.is_file())
    if any(path.is_symlink() for path in app.rglob("*")):
        raise ValueError("Unexpected symlink in app bundle")
    if any(path.suffix.lower() in (".mobileprovision", ".p12", ".p8", ".key") for path in files):
        raise ValueError("Signing material must never be included in CI artifacts")
    output.mkdir(parents=True, exist_ok=True)
    ipa = output / "MegaNugget-UNSIGNED.ipa"
    with zipfile.ZipFile(ipa, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for path in files:
            archive.write(path, str(Path("Payload") / app.name / path.relative_to(app)))
    manifest = {
        "schema": 1,
        "artifact": ipa.name,
        "signing": "unsigned",
        "installable_on_stock_ios": False,
        "minimum_ios": minimum,
        "architectures": architectures,
        "bundle_id": info["CFBundleIdentifier"],
        "build": info["CFBundleVersion"],
        "commit": os.environ.get("GITHUB_SHA", "local"),
        "sha256": hashlib.sha256(ipa.read_bytes()).hexdigest(),
    }
    (output / "build-info.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (output / "README.txt").write_text(
        "UNSIGNED DEVICE BUILD — NOT INSTALLABLE ON STOCK IOS\n"
        "This IPA has no Apple signing identity or provisioning profile.\n"
        "MegaNugget Desktop is expected to reject it with MissingSigningArtifacts.\n"
        "Use Xcode with your own authorized signing/provisioning for real installation.\n"
        "Deployment target 15.0 is not hardware validation or jailbreak support.\n"
        "JailbreakEngine returns not_implemented.\n"
    )
    return ipa


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    print(package(args.app, args.output))
