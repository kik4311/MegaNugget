"""Provenance checks complement Swift unit tests; no device/transport is used."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]


class GoldenNuggetImportTests(unittest.TestCase):
    def test_pinned_import_and_modified_file_hashes(self):
        manifest = json.loads((ROOT / "LICENSES/GoldenNuggetMobile-import.json").read_text())
        self.assertEqual(manifest["revision"], "e4be86f050f4980ccc58de3b74f9de58b070b705")
        for name, digest in manifest["outputs"].items():
            with self.subTest(file=name):
                data = (ROOT / name).read_bytes()
                self.assertEqual(hashlib.sha256(data).hexdigest(), digest)
                if name.endswith(".swift"):
                    self.assertIn(b"Modifications by MegaNugget contributors, 2026-11-06", data)

    def test_license_and_complete_credits_retained(self):
        license_text = (ROOT / "LICENSES/GoldenNuggetMobile-AGPL-3.0.txt").read_text()
        self.assertIn("GNU AFFERO GENERAL PUBLIC LICENSE", license_text)
        readme = (ROOT / "LICENSES/GoldenNuggetMobile-README.upstream.md").read_text()
        self.assertIn("awesomenull", readme)
        self.assertIn("## Credits", readme)

    def test_desktop_catalog_matches_pinned_swift(self):
        """The committed Rust catalog must equal a fresh export; never stale."""
        catalog = json.loads((ROOT / "crates/customization/catalog.json").read_text())
        self.assertEqual(catalog["revision"], "e4be86f050f4980ccc58de3b74f9de58b070b705")
        self.assertEqual(len(catalog["specs"]), 258)
        completed = subprocess.run(
            [sys.executable, str(ROOT / "scripts/export_desktop_catalog.py"), "--check"],
            capture_output=True,
            text=True,
            cwd=ROOT,
        )
        self.assertEqual(
            completed.returncode, 0, completed.stdout + completed.stderr
        )
        self.assertIn("matches the pinned Swift source", completed.stdout)


if __name__ == "__main__":
    unittest.main()
