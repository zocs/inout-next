"""Exercise the raw Android asset index using actual exported filesystem trees."""

import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "scripts/prepare_android_assets.py"


class AndroidAssetIndexTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.web = self.root / "web"
        (self.web / "_next/static").mkdir(parents=True)
        self.files = {
            "index.html": b"<!doctype html><title>Android asset fixture</title>",
            "_next/static/app.js": b"window.assetFixture = 'raw UTF-8 bytes';\n",
        }
        for name, content in self.files.items():
            (self.web / name).write_bytes(content)
        self.output = self.root / "index.tsv"

    def run_index(self):
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--web-dir", str(self.web), "--output", str(self.output)],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_index_identifies_every_original_file_and_is_deterministic(self):
        result = self.run_index()
        self.assertEqual(result.returncode, 0, result.stderr)
        first = self.output.read_bytes()
        header, entries = first.split(b"\n", 1)
        self.assertEqual(
            header.decode().split("\t"),
            ["INOUTWEB1", hashlib.sha256(entries).hexdigest(), "2", str(sum(map(len, self.files.values())))],
        )
        rows = [line.split("\t") for line in entries.decode().splitlines()]
        self.assertEqual([row[2] for row in rows], sorted(self.files))
        for digest, size, name in rows:
            self.assertEqual(digest, hashlib.sha256(self.files[name]).hexdigest())
            self.assertEqual(int(size), len(self.files[name]))
        result = self.run_index()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.output.read_bytes(), first)
        self.assertFalse(list(self.root.glob("*.tmp")))

    def test_missing_entrypoint_does_not_replace_an_existing_completed_index(self):
        self.output.write_bytes(b"previous completed index")
        (self.web / "index.html").unlink()
        result = self.run_index()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertEqual(self.output.read_bytes(), b"previous completed index")

    def test_symbolic_link_cannot_publish_private_sibling_bytes(self):
        secret = self.root / "private-sibling.txt"
        secret.write_bytes(b"private resource must never be published")
        (self.web / "_next/static/alias.js").symlink_to(secret)
        result = self.run_index()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertFalse(self.output.exists())
        self.assertEqual(secret.read_bytes(), b"private resource must never be published")

    def test_backslash_and_table_control_names_are_rejected(self):
        for name in ["back\\slash.js", "table\tcolumn.js", "line\nbreak.js"]:
            with self.subTest(name=name):
                path = self.web / name
                path.write_bytes(b"invalid resource name")
                result = self.run_index()
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertFalse(self.output.exists())
                path.unlink()

    def test_sparse_oversized_resource_is_rejected_before_publication(self):
        with (self.web / "_next/static/oversized.js").open("wb") as stream:
            stream.truncate(128 * 1024 * 1024 + 1)
        result = self.run_index()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertFalse(self.output.exists())

    def test_excessive_directory_depth_is_rejected(self):
        directory = self.web / "_next"
        for _ in range(128):
            directory /= "d"
        directory.mkdir(parents=True)
        (directory / "app.js").write_bytes(b"too deep")
        result = self.run_index()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
