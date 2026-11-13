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
        (self.root / "config").mkdir()
        self.identity = {
            "application_id": "io.github.zocs.inoutnext",
            "product_name": "Inout Next",
            "wix_upgrade_code": "ccf507b8-b478-5af7-855b-a5e8fb9bc2d1",
        }
        (self.root / "config/native-identity.json").write_text(json.dumps(self.identity))

    def sync(self, *args):
        return subprocess.run(
            [sys.executable, str(PROJECT / "scripts/sync_product.py"),
             "--root", str(self.root), *args],
            capture_output=True, text=True, check=False,
        )

    def test_alternate_brand_updates_consumers_without_changing_identity(self):
        self.assertEqual(self.sync().returncode, 0)
        native_path = self.root / "apps/desktop/src-tauri/tauri.conf.json"
        baseline = json.loads(native_path.read_text())
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
        self.assertEqual(native["productName"], "Inout Next")
        self.assertEqual(native["identifier"], identity)
        self.assertEqual(native["productName"], baseline["productName"])
        self.assertEqual(native["identifier"], baseline["identifier"])
        self.assertEqual(native["bundle"]["windows"]["wix"]["upgradeCode"],
                         "ccf507b8-b478-5af7-855b-a5e8fb9bc2d1")
        self.assertEqual(native["bundle"]["windows"]["wix"], baseline["bundle"]["windows"]["wix"])
        self.assertEqual(native["app"]["windows"][0]["title"], "Harbor")
        self.assertEqual(baseline["app"]["windows"][0]["title"], "Inout Next")
        self.assertEqual(native["bundle"]["shortDescription"], "Files on your network")
        self.assertEqual(distribution["archive_prefix"], "harbor")
        self.assertEqual(native["build"]["frontendDist"], "../../web/out")
        self.assertEqual(self.sync("--check").returncode, 0)

    def test_invalid_native_identity_does_not_mutate_any_consumer(self):
        self.assertEqual(self.sync().returncode, 0)
        consumers = [self.root / relative for relative in (
            "apps/web/src/generated/product.json",
            "apps/desktop/src-tauri/tauri.conf.json",
            "config/distribution.json",
        )]
        before = [path.read_bytes() for path in consumers]
        product = json.loads((self.root / "product.json").read_text())
        product["name"] = "Changed before invalid identity"
        (self.root / "product.json").write_text(json.dumps(product))
        invalid = [None, {}, {**self.identity, "extra": "field"},
                   {**self.identity, "application_id": "invalid"},
                   {**self.identity, "application_id": "io.github.other.application"},
                   {**self.identity, "product_name": ""},
                   {**self.identity, "product_name": "../outside"},
                   {**self.identity, "product_name": "Bad\nName"},
                   {**self.identity, "product_name": "Name."},
                   {**self.identity, "product_name": "CON"},
                   {**self.identity, "wix_upgrade_code": 123},
                   {**self.identity, "wix_upgrade_code": "not-a-uuid"},
                   {**self.identity, "wix_upgrade_code": "00000000-0000-0000-0000-000000000000"}]
        for identity in invalid:
            with self.subTest(identity=identity):
                (self.root / "config/native-identity.json").write_text(json.dumps(identity))
                result = self.sync()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("native", result.stderr.lower())
                self.assertEqual([path.read_bytes() for path in consumers], before)

    def test_missing_native_identity_does_not_create_consumers(self):
        (self.root / "config/native-identity.json").unlink()
        result = self.sync()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "apps").exists())
        self.assertFalse((self.root / "config/distribution.json").exists())

    def test_application_id_change_requires_matching_explicit_native_migration(self):
        product = json.loads((self.root / "product.json").read_text())
        product["application_id"] = "io.github.zocs.newidentity"
        (self.root / "product.json").write_text(json.dumps(product))
        result = self.sync()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "apps").exists())
        self.identity.update(application_id=product["application_id"],
                             product_name="New Install Identity",
                             wix_upgrade_code="12345678-1234-4234-8234-123456789abc")
        (self.root / "config/native-identity.json").write_text(json.dumps(self.identity))
        result = self.sync()
        self.assertEqual(result.returncode, 0, result.stderr)
        native = json.loads((self.root / "apps/desktop/src-tauri/tauri.conf.json").read_text())
        self.assertEqual(native["identifier"], "io.github.zocs.newidentity")
        self.assertEqual(native["productName"], "New Install Identity")
        self.assertEqual(native["bundle"]["windows"]["wix"]["upgradeCode"],
                         "12345678-1234-4234-8234-123456789abc")

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
        self.assertFalse((self.root / "config/distribution.json").exists())


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
