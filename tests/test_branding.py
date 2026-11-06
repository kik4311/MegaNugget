"""Check committed branding without installing a renderer/Pillow in CI."""
import hashlib
import json
from pathlib import Path
import struct
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
IOS = ROOT / "apps/ios/Sources/App/Assets.xcassets"


class BrandingTests(unittest.TestCase):
    def test_source_and_generated_files_match_manifest(self):
        manifest = json.loads((ROOT / "assets/branding/generated.json").read_text())
        self.assertEqual(hashlib.sha256((ROOT / manifest["source"]).read_bytes()).hexdigest(), manifest["source_sha256"])
        for name, digest in manifest["outputs"].items():
            with self.subTest(name=name):
                self.assertEqual(hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), digest)

    def test_svg_copies_and_accessibility(self):
        source = (ROOT / "assets/branding/meganugget-logo.svg").read_bytes()
        tree = ET.fromstring(source)
        self.assertEqual(tree.attrib["viewBox"], "0 0 512 512")
        self.assertIsNotNone(tree.find("{http://www.w3.org/2000/svg}title"))
        for name in ["apps/desktop/public/branding/meganugget-logo.svg", "apps/desktop/src-tauri/icons/source.svg"]:
            self.assertEqual((ROOT / name).read_bytes(), source)

    def test_iphone_icons_are_opaque_and_correct_size(self):
        catalog = json.loads((IOS / "AppIcon.appiconset/Contents.json").read_text())
        self.assertEqual(len(catalog["images"]), 9)
        for entry in catalog["images"]:
            data = (IOS / "AppIcon.appiconset" / entry["filename"]).read_bytes()
            self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")
            width, height, depth, color = struct.unpack(">IIBB", data[16:26])
            expected = int(entry["size"].split("x")[0]) * int(entry["scale"][0])
            self.assertEqual((width, height, depth, color), (expected, expected, 8, 2))

    def test_native_icon_containers(self):
        icons = ROOT / "apps/desktop/src-tauri/icons"
        self.assertEqual((icons / "icon.icns").read_bytes()[:4], b"icns")
        self.assertEqual((icons / "icon.ico").read_bytes()[:4], b"\x00\x00\x01\x00")


if __name__ == "__main__":
    unittest.main()
