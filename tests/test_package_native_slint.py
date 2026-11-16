import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile


SCRIPT = Path(__file__).resolve().parents[1] / "scripts/package_native_slint.py"


class NativeTrialPackagingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "source"
        self.root.mkdir()
        self.product = {
            "name": "newname", "slug": "renamed", "version": "0.1.0",
            "description": "A local share", "application_id": "io.example.stable",
            "repository": "https://github.com/example/project",
        }
        (self.root / "product.json").write_text(json.dumps(self.product))
        (self.root / "LICENSE").write_text("Application source licence fixture\n")
        (self.root / "README.md").write_text("Project readme fixture\n")
        web = self.root / "apps/web/out"
        (web / "_next/static").mkdir(parents=True)
        (web / "index.html").write_bytes(b"<!doctype html><title>Exact export</title>")
        (web / "_next/static/app.js").write_bytes(b"console.log('fixture');")
        app = self.root / "apps/native-slint"
        (app / "LICENSES").mkdir(parents=True)
        (app / "README.md").write_text("Native desktop trial usage\n")
        (app / "THIRD_PARTY.md").write_text("Native framework attribution\n")
        self.notice = app / "LICENSES/Slint-Royalty-free-2.0.md"
        self.notice.write_text("Unmodified dependency licence fixture\n")
        (self.root / "private-key.txt").write_text("this must stay private")
        self.binary = self.root / "built-client"
        self.binary.write_bytes(b"Native executable fixture")

    def package(self, output, target="linux-x64"):
        return subprocess.run([
            sys.executable, str(SCRIPT), "--root", str(self.root),
            "--binary", str(self.binary), "--target", target, "--output", str(output),
        ], capture_output=True, text=True, timeout=10)

    def test_complete_native_archive_keeps_resources_notices_identity_and_executable_mode(self):
        first = Path(self.temporary.name) / "first"
        second = Path(self.temporary.name) / "second"
        result = self.package(first)
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        other = self.package(second)
        self.assertEqual(other.returncode, 0, other.stderr + other.stdout)
        archive = first / "renamed-0.1.0-linux-x64-native-slint-trial.zip"
        with zipfile.ZipFile(archive) as bundle:
            self.assertEqual(set(bundle.namelist()), {
                "share-native-slint", "web/index.html", "web/_next/static/app.js",
                "LICENSE", "README.md", "QUICKSTART.txt", "product.json",
                "THIRD_PARTY.md", "LICENSES/Slint-Royalty-free-2.0.md",
            })
            self.assertEqual(bundle.read("web/index.html"), b"<!doctype html><title>Exact export</title>")
            self.assertEqual(bundle.read("share-native-slint"), b"Native executable fixture")
            self.assertEqual(json.loads(bundle.read("product.json")), self.product)
            self.assertEqual(bundle.read("README.md"), b"Native desktop trial usage\n")
            self.assertEqual(bundle.read("LICENSES/Slint-Royalty-free-2.0.md"), self.notice.read_bytes())
            self.assertEqual((bundle.getinfo("share-native-slint").external_attr >> 16) & 0o777, 0o755)
            self.assertEqual((bundle.getinfo("web/index.html").external_attr >> 16) & 0o777, 0o644)
        contents = archive.read_bytes()
        self.assertEqual(contents, (second / archive.name).read_bytes())
        self.assertEqual(archive.with_suffix(".zip.sha256").read_text().split()[0], hashlib.sha256(contents).hexdigest())

    def test_windows_package_uses_the_fixed_gui_binary_name(self):
        output = Path(self.temporary.name) / "windows"
        result = self.package(output, "windows-x86")
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        with zipfile.ZipFile(output / "renamed-0.1.0-windows-x86-native-slint-trial.zip") as bundle:
            self.assertIn("share-native-slint.exe", bundle.namelist())
            self.assertNotIn("share-server.exe", bundle.namelist())

    def test_missing_framework_notice_prevents_a_partial_archive(self):
        self.notice.unlink()
        output = Path(self.temporary.name) / "incomplete"
        result = self.package(output)
        self.assertEqual(result.returncode, 1)
        self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
