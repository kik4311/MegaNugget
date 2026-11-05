import json
from pathlib import Path
import plistlib
import tempfile
import unittest
from unittest.mock import patch
import zipfile

from scripts.package_unsigned_ios import package


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.app = self.root / "MegaNugget.app"
        self.app.mkdir()
        self.info = {
            "CFBundleIdentifier": "org.meganugget.ios",
            "CFBundleVersion": "1",
            "CFBundleExecutable": "MegaNugget",
            "CFBundleSupportedPlatforms": ["iPhoneOS"],
            "MinimumOSVersion": "15.0",
        }
        self.write_info()
        (self.app / "MegaNugget").write_bytes(b"fixture-not-a-real-executable")

    def write_info(self):
        (self.app / "Info.plist").write_bytes(plistlib.dumps(self.info))

    @patch("scripts.package_unsigned_ios.subprocess.check_output", return_value="arm64\n")
    def test_unsigned_archive_is_honestly_labelled(self, _lipo):
        ipa = package(self.app, self.root / "output")
        with zipfile.ZipFile(ipa) as archive:
            self.assertIn("Payload/MegaNugget.app/Info.plist", archive.namelist())
            self.assertFalse(any("mobileprovision" in name for name in archive.namelist()))
        manifest = json.loads((ipa.parent / "build-info.json").read_text())
        self.assertFalse(manifest["installable_on_stock_ios"])
        self.assertEqual(manifest["signing"], "unsigned")
        self.assertEqual(len(manifest["sha256"]), 64)

    def test_reject_simulator(self):
        self.info["CFBundleSupportedPlatforms"] = ["iPhoneSimulator"]
        self.write_info()
        with self.assertRaisesRegex(ValueError, "simulator"):
            package(self.app, self.root / "output")

    def test_reject_wrong_minimum_ios(self):
        self.info["MinimumOSVersion"] = "26.0"
        self.write_info()
        with self.assertRaisesRegex(ValueError, "deployment target"):
            package(self.app, self.root / "output")

    @patch("scripts.package_unsigned_ios.subprocess.check_output", return_value="x86_64\n")
    def test_reject_non_arm64(self, _lipo):
        with self.assertRaisesRegex(ValueError, "arm64"):
            package(self.app, self.root / "output")

    @patch("scripts.package_unsigned_ios.subprocess.check_output", return_value="arm64\n")
    def test_reject_profile_upload(self, _lipo):
        (self.app / "embedded.mobileprovision").write_bytes(b"fixture")
        with self.assertRaisesRegex(ValueError, "provisioning profiles"):
            package(self.app, self.root / "output")

    @patch("scripts.package_unsigned_ios.subprocess.check_output", return_value="arm64\n")
    def test_reject_private_key_upload(self, _lipo):
        (self.app / "accidental.key").write_bytes(b"fixture")
        with self.assertRaisesRegex(ValueError, "Signing material"):
            package(self.app, self.root / "output")


if __name__ == "__main__":
    unittest.main()
