#!/usr/bin/env python3
"""Generate all app branding from one SVG. Requires rsvg-convert and Pillow.

No network access, upstream artwork or runtime dependencies are added to the apps.
Run from any directory: python3 scripts/generate_branding.py
"""

import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/branding/meganugget-logo.svg"
DESKTOP = ROOT / "apps/desktop/src-tauri/icons"
WEB = ROOT / "apps/desktop/public/branding"
IOS = ROOT / "apps/ios/Sources/App/Assets.xcassets"


def generate():
    renderer = shutil.which("rsvg-convert")
    if not renderer:
        raise SystemExit("rsvg-convert is required (librsvg2-bin on Debian/Ubuntu).")
    source = SOURCE.read_bytes()
    outputs = {}

    def write(path, data):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        outputs[path.relative_to(ROOT).as_posix()] = hashlib.sha256(data).hexdigest()

    def render(size, opaque=False):
        data = subprocess.run(
            [renderer, "--format", "png", "--width", str(size), "--height", str(size)],
            input=source, capture_output=True, check=True,
        ).stdout
        image = Image.open(io.BytesIO(data)).convert("RGBA")
        if opaque:
            # iOS applies its own icon mask; never ship alpha or transparent corners.
            canvas = Image.new("RGB", image.size, "#111017")
            canvas.paste(image, mask=image.getchannel("A"))
            return canvas
        return image

    def save_image(path, image, fmt="PNG", **kwargs):
        stream = io.BytesIO()
        image.save(stream, format=fmt, **kwargs)
        write(path, stream.getvalue())

    write(WEB / "meganugget-logo.svg", source)
    write(DESKTOP / "source.svg", source)
    for name, size in [("32x32.png", 32), ("128x128.png", 128),
                       ("128x128@2x.png", 256), ("icon.png", 512)]:
        save_image(DESKTOP / name, render(size))
    master = render(1024)
    save_image(DESKTOP / "icon.ico", master, "ICO", sizes=[(n, n) for n in [16, 24, 32, 48, 64, 128, 256]])
    save_image(DESKTOP / "icon.icns", master, "ICNS")
    save_image(WEB / "favicon.ico", master, "ICO", sizes=[(n, n) for n in [16, 32, 48]])
    save_image(WEB / "apple-touch-icon.png", render(180, opaque=True))

    icons = []
    # Explicit iPhone slots work with the iOS 15 deployment target too.
    for points in [20, 29, 40, 60]:
        for scale in [2, 3]:
            filename = f"AppIcon-{points}@{scale}x.png"
            save_image(IOS / "AppIcon.appiconset" / filename, render(points * scale, opaque=True))
            icons.append({"filename": filename, "idiom": "iphone", "size": f"{points}x{points}", "scale": f"{scale}x"})
    save_image(IOS / "AppIcon.appiconset/AppIcon.png", render(1024, opaque=True))
    icons.append({"filename": "AppIcon.png", "idiom": "ios-marketing", "size": "1024x1024", "scale": "1x"})
    info = {"author": "xcode", "version": 1}
    write(IOS / "AppIcon.appiconset/Contents.json", (json.dumps({"images": icons, "info": info}, indent=2) + "\n").encode())

    images = []
    for scale in [1, 2, 3]:
        filename = f"AppLogo@{scale}x.png"
        save_image(IOS / "AppLogo.imageset" / filename, render(64 * scale))
        images.append({"filename": filename, "idiom": "universal", "scale": f"{scale}x"})
    write(IOS / "AppLogo.imageset/Contents.json", (json.dumps({"images": images, "info": info}, indent=2) + "\n").encode())

    manifest = {"source": SOURCE.relative_to(ROOT).as_posix(),
                "source_sha256": hashlib.sha256(source).hexdigest(), "outputs": outputs}
    (ROOT / "assets/branding/generated.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Generated {len(outputs)} branding files from {manifest['source']}")


if __name__ == "__main__":
    generate()
