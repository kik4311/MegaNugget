"""Provenance checks complement Swift unit tests; no device/transport is used."""
import hashlib
import json
from pathlib import Path
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


if __name__ == "__main__":
    unittest.main()
