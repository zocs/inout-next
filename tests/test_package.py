"""Check delivered portable contents rather than packaging source text."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from zipfile import ZipFile

PROJECT = Path(__file__).resolve().parents[1]


class PortablePackageTests(unittest.TestCase):
    def test_portable_zip_uses_brand_and_contains_only_runtime_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            product = json.loads((PROJECT / "product.json").read_text())
            product.update(name="Harbor", slug="harbor")
            (root / "product.json").write_text(json.dumps(product))
            (root / "LICENSE").write_text("License fixture")
            (root / "README.md").write_text("Usage fixture")
            (root / "private.key").write_text("must not ship")
            (root / "apps/web/out/_next/static").mkdir(parents=True)
            (root / "apps/web/out/index.html").write_text("<title>Harbor</title>")
            (root / "apps/web/out/_next/static/app.js").write_text("console.info('fixture')")
            binary = root / "source-binary"
            binary.write_bytes(b"executable fixture")
            output = root / "packages"
            result = subprocess.run([
                sys.executable, str(PROJECT / "scripts/package_portable.py"),
                "--root", str(root), "--binary", str(binary), "--target", "linux-x64",
                "--output", str(output),
            ], capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            archives = list(output.glob("harbor-0.1.0-linux-x64-server.zip"))
            self.assertEqual(len(archives), 1)
            with ZipFile(archives[0]) as archive:
                self.assertEqual(archive.read("share-server"), b"executable fixture")
                self.assertEqual(json.loads(archive.read("product.json"))["application_id"], product["application_id"])
                self.assertIn("web/index.html", archive.namelist())
                self.assertIn("web/_next/static/app.js", archive.namelist())
                self.assertNotIn("private.key", archive.namelist())
                self.assertTrue(all(not name.startswith("/") and ".." not in name.split("/") for name in archive.namelist()))

    def test_missing_assets_fail_without_creating_a_misleading_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "product.json").write_bytes((PROJECT / "product.json").read_bytes())
            binary = root / "server"
            binary.write_bytes(b"fixture")
            output = root / "packages"
            result = subprocess.run([
                sys.executable, str(PROJECT / "scripts/package_portable.py"),
                "--root", str(root), "--binary", str(binary), "--target", "linux-x64",
                "--output", str(output),
            ], capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("web assets", result.stderr.lower())
            self.assertFalse(list(output.glob("*.zip")))

    @unittest.skipUnless(hasattr(Path, "symlink_to"), "symlinks unavailable")
    def test_linked_assets_are_not_included_in_a_portable_package(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "product.json").write_bytes((PROJECT / "product.json").read_bytes())
            (root / "LICENSE").write_text("License fixture")
            (root / "README.md").write_text("Usage fixture")
            (root / "apps/web/out/_next").mkdir(parents=True)
            (root / "apps/web/out/index.html").write_text("<title>Fixture</title>")
            private = root / "private.key"
            private.write_text("must not ship")
            try:
                (root / "apps/web/out/key.txt").symlink_to(private)
            except OSError:
                self.skipTest("host does not permit creating symlinks")
            binary = root / "server"
            binary.write_bytes(b"fixture")
            output = root / "packages"
            result = subprocess.run([
                sys.executable, str(PROJECT / "scripts/package_portable.py"),
                "--root", str(root), "--binary", str(binary), "--target", "linux-x64",
                "--output", str(output),
            ], capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("symlink", result.stderr.lower())
            self.assertFalse(list(output.glob("*.zip")))


if __name__ == "__main__":
    unittest.main()
