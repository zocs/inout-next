"""Check observable brand substitution and private key-file behavior."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


PROJECT = Path(__file__).resolve().parents[1]


class ProductSyncTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        shutil.copyfile(PROJECT / "product.json", self.root / "product.json")

    def sync(self, *args):
        return subprocess.run(
            [sys.executable, str(PROJECT / "scripts/sync_product.py"),
             "--root", str(self.root), *args],
            capture_output=True, text=True, check=False,
        )

    def test_alternate_brand_updates_consumers_without_changing_identity(self):
        product = json.loads((self.root / "product.json").read_text())
        identity = product["application_id"]
        product.update(name="Harbor", slug="harbor", description="Files on your network")
        (self.root / "product.json").write_text(json.dumps(product))
        result = self.sync()
        self.assertEqual(result.returncode, 0, result.stderr)
        web = json.loads((self.root / "apps/web/src/generated/product.json").read_text())
        native = json.loads((self.root / "apps/desktop/src-tauri/tauri.conf.json").read_text())
        distribution = json.loads((self.root / "config/distribution.json").read_text())
        self.assertEqual(web["name"], "Harbor")
        self.assertEqual(web["application_id"], identity)
        self.assertEqual(native["productName"], "Harbor")
        self.assertEqual(native["identifier"], identity)
        self.assertEqual(distribution["archive_prefix"], "harbor")
        self.assertEqual(native["build"]["frontendDist"], "../../web/out")
        self.assertEqual(self.sync("--check").returncode, 0)

    def test_check_detects_stale_generated_metadata_without_rewriting_it(self):
        self.assertEqual(self.sync().returncode, 0)
        generated = self.root / "apps/web/src/generated/product.json"
        generated.write_text('{"name":"stale"}\n')
        before = generated.read_bytes()
        result = self.sync("--check")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("stale", result.stderr.lower())
        self.assertEqual(generated.read_bytes(), before)

    def test_invalid_slug_does_not_create_distribution_files(self):
        product = json.loads((self.root / "product.json").read_text())
        product["slug"] = "../outside"
        (self.root / "product.json").write_text(json.dumps(product))
        result = self.sync()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("slug", result.stderr.lower())
        self.assertFalse((self.root / "config").exists())


class PrivateKeyTests(unittest.TestCase):
    def test_creates_private_key_without_disclosing_or_overwriting_it(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "access.key"
            command = [sys.executable, str(PROJECT / "scripts/create_key.py"), str(path)]
            result = subprocess.run(command, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            key = path.read_text().strip()
            self.assertGreaterEqual(len(key), 32)
            self.assertNotIn(key, result.stdout + result.stderr)
            if os.name != "nt":
                self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            duplicate = subprocess.run(command, capture_output=True, text=True, check=False)
            self.assertNotEqual(duplicate.returncode, 0)
            self.assertEqual(path.read_text().strip(), key)
            self.assertNotIn(key, duplicate.stdout + duplicate.stderr)


if __name__ == "__main__":
    unittest.main()
