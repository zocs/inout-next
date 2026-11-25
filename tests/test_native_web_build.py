"""Run the real build script's export boundary without compiling Slint's UI.

CARGO_FEATURE_GUI is set as it is by Cargo. The standalone compilation omits
the UI compiler dependency; these tests cover the embedding/build failure only.
"""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

PROJECT = Path(__file__).resolve().parents[1]


@unittest.skipUnless(shutil.which("rustc"), "Rust is required to run the native build script")
class NativeWebBuildTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.compilation = tempfile.TemporaryDirectory(prefix="native-build-script-")
        cls.addClassCleanup(cls.compilation.cleanup)
        cls.script = Path(cls.compilation.name) / "build-script"
        subprocess.run(["rustc", "--edition=2024", str(PROJECT / "apps/native-slint/build.rs"),
                        "-o", str(cls.script)], check=True, capture_output=True)

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="native web build ")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.manifest = self.root / "apps/native-slint"
        self.manifest.mkdir(parents=True)
        self.output = self.root / "output"
        self.output.mkdir()
        self.web = self.root / "apps/web/out"

    def run_build(self, profile="release", gui=True):
        env = dict(os.environ, CARGO_MANIFEST_DIR=str(self.manifest),
                   OUT_DIR=str(self.output), PROFILE=profile)
        env.pop("CARGO_FEATURE_GUI", None)
        if gui:
            env["CARGO_FEATURE_GUI"] = "1"
        return subprocess.run([str(self.script)], env=env, capture_output=True, text=True, timeout=10)

    def export(self):
        (self.web / "_next/static").mkdir(parents=True)
        (self.web / "index.html").write_bytes(b"trusted index")
        (self.web / "_next/static/app.js").write_bytes(b"trusted script")

    def test_release_gui_rejects_a_missing_or_partial_export(self):
        result = self.run_build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("build apps/web", result.stderr)
        self.web.mkdir(parents=True)
        (self.web / "index.html").write_bytes(b"partial index")
        self.assertNotEqual(self.run_build().returncode, 0)

    def test_debug_and_headless_builds_can_omit_the_export(self):
        self.assertEqual(self.run_build(profile="debug").returncode, 0)
        self.assertEqual(self.run_build(gui=False).returncode, 0)

    def test_complete_export_is_embedded_as_the_original_bytes(self):
        self.export()
        result = self.run_build()
        self.assertEqual(result.returncode, 0, result.stderr)
        consumer = self.output / "consumer.rs"
        consumer.write_text('include!("web_bundle.rs");\nfn main() {\n'
                            'assert_eq!(FILES, &[("_next/static/app.js", b"trusted script".as_slice()),'
                            '("index.html", b"trusted index".as_slice())]);\n}\n')
        binary = self.output / "consumer"
        subprocess.run(["rustc", "--edition=2024", str(consumer), "-o", str(binary)],
                       check=True, capture_output=True)
        subprocess.run([str(binary)], check=True, capture_output=True)

    def test_an_oversized_export_fails_during_build(self):
        self.export()
        with (self.web / "_next/static/huge.js").open("wb") as stream:
            stream.truncate(128 * 1024 * 1024 + 1)
        result = self.run_build()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("limit", result.stderr.lower())


if __name__ == "__main__":
    unittest.main()
