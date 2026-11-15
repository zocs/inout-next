"""Exercise Android lock enforcement with real child processes and changed files."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import build_android as build


class AndroidBuildTests(unittest.TestCase):
    def test_locked_argument_precedes_rustc_separator_and_preserves_global_toolchain(self):
        args = ["+1.99.1", "rustc", "--target", "aarch64-linux-android", "--", "--cfg", "example"]
        changed = build.locked_argv(args, "aarch64-linux-android")
        self.assertEqual(changed, args[:4] + ["--locked"] + args[4:])

    def test_existing_lock_after_separator_is_not_a_cargo_lock(self):
        args = ["build", "--target=aarch64-linux-android", "--", "--locked"]
        changed = build.locked_argv(args, "aarch64-linux-android")
        self.assertEqual(changed, args[:2] + ["--locked"] + args[2:])

    def test_other_abi_or_missing_build_target_fails_before_child(self):
        for args in [["build"], ["build", "--target", "x86_64-linux-android"], ["update"]]:
            with self.subTest(args=args), self.assertRaises(ValueError):
                build.locked_argv(args, "aarch64-linux-android")

    def test_cargo_tauri_external_command_is_not_given_cargo_lock_flag(self):
        args = ["tauri", "android", "android-studio-script", "--target", "aarch64-linux-android"]
        self.assertEqual(build.locked_argv(args, "aarch64-linux-android"), args)

    def test_metadata_is_locked_without_requiring_target(self):
        self.assertEqual(build.locked_argv(["metadata", "--format-version", "1"], "aarch64-linux-android"),
                         ["metadata", "--format-version", "1", "--locked"])

    def run_child(self, mutate=False, command="rustc", mutation="Cargo.lock"):
        temporary = tempfile.TemporaryDirectory(prefix="Android build argv ")
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        (root / "apps/desktop/src-tauri").mkdir(parents=True)
        for name in build.LOCKFILES:
            (root / name).write_text("original lock\n")
        actual = root / "real-cargo"
        actual.write_text("#!" + sys.executable + "\n"
                          "import json,os,pathlib,sys\n"
                          "pathlib.Path(os.environ['FIXTURE_ARGV']).write_text(json.dumps(sys.argv[1:]))\n"
                          + (f"pathlib.Path({mutation!r}).write_text('changed lock')\n" if mutate else ""))
        actual.chmod(0o755)
        driver = root / "driver.py"
        args = ["+1.99.1", command, "--target", "aarch64-linux-android", "--", "--cfg", "fixture"]
        driver.write_text("import subprocess,sys\n"
                          f"raise SystemExit(subprocess.call(['cargo']+{args!r}))\n")
        env = dict(os.environ, FIXTURE_ARGV=str(root / "argv.json"))
        result = build.run_locked([sys.executable, str(driver)], root, root / ".local/run",
                                  "aarch64-linux-android", actual, env)
        return result, root

    def test_generated_wrapper_changes_actual_child_argv_and_records_both(self):
        result, root = self.run_child()
        self.assertEqual(result["exit_code"], 0, result)
        actual = json.loads((root / "argv.json").read_text())
        self.assertLess(actual.index("--locked"), actual.index("--"))
        events = [json.loads(row) for row in Path(result["cargo_events"]).read_text().splitlines()]
        invocation = next(row for row in events if row["event"] == "invoke")
        self.assertNotIn("--locked", invocation["original_argv"])
        self.assertEqual(actual, invocation["effective_argv"])
        self.assertEqual(result["locked_target_commands"], 1)

    def test_lockfile_mutation_is_failure_even_when_child_exits_zero(self):
        result, root = self.run_child(mutate=True)
        self.assertNotEqual(result["exit_code"], 0)
        self.assertIn("Cargo.lock", result["changed_lockfiles"])
        self.assertEqual((root / "Cargo.lock").read_text(), "changed lock")

    def test_native_lock_mutation_is_detected_and_preserved(self):
        native = "apps/desktop/src-tauri/Cargo.lock"
        result, root = self.run_child(mutate=True, mutation=native)
        self.assertNotEqual(result["exit_code"], 0)
        self.assertIn(native, result["changed_lockfiles"])
        self.assertEqual((root / native).read_text(), "changed lock")

    def test_external_tauri_child_is_passed_through_and_its_nested_build_is_locked(self):
        with tempfile.TemporaryDirectory(prefix="nested Cargo ") as directory:
            root = Path(directory)
            (root / "apps/desktop/src-tauri").mkdir(parents=True)
            for name in build.LOCKFILES:
                (root / name).write_text("lock")
            cargo = root / "real-cargo"
            cargo.write_text("#!" + sys.executable + "\nimport json,subprocess,sys\n"
                             "if sys.argv[1] == 'tauri':\n"
                             "    assert '--locked' not in sys.argv\n"
                             "    raise SystemExit(subprocess.call(['cargo','build','--target','aarch64-linux-android']))\n"
                             "assert sys.argv[-1] == '--locked'\n")
            cargo.chmod(0o755)
            driver = root / "driver.py"
            driver.write_text("import subprocess\nraise SystemExit(subprocess.call(['cargo','tauri']))\n")
            result = build.run_locked([sys.executable, str(driver)], root, root / ".local/run",
                                      "aarch64-linux-android", cargo, dict(os.environ))
            self.assertEqual(result["exit_code"], 0, result)
            events = [json.loads(row) for row in Path(result["cargo_events"]).read_text().splitlines()]
            invocations = [row for row in events if row["event"] == "invoke"]
            self.assertEqual([row["subcommand"] for row in invocations], ["tauri", "build"])
            self.assertFalse(invocations[0]["locked"])
            self.assertTrue(invocations[1]["locked"])

    def test_bad_wrapper_bytes_are_never_installed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "download.jar"
            source.write_bytes(b"untrusted wrapper")
            with self.assertRaises(ValueError):
                build.ensure_wrapper(root, source)
            self.assertFalse((root / "apps/desktop/src-tauri/gen/android/gradle/wrapper/gradle-wrapper.jar").exists())

    def test_preview_preflight_rejects_removed_mapping_or_changed_release_suffix(self):
        project = Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = ["product.json", "apps/desktop/src-tauri/tauri.conf.json",
                     "apps/desktop/src-tauri/tauri.android.conf.json",
                     "apps/desktop/src-tauri/gen/android/app/build.gradle.kts",
                     "apps/desktop/src-tauri/gen/android/build.gradle.kts",
                     "apps/desktop/src-tauri/gen/android/gradle/wrapper/gradle-wrapper.properties"]
            for name in files:
                destination = root / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(project / name, destination)
            build.validate_preview_project(root)
            base = root / files[1]
            original = base.read_text()
            data = json.loads(original)
            data["bundle"]["resources"] = {}
            base.write_text(json.dumps(data))
            with self.assertRaises(ValueError):
                build.validate_preview_project(root)
            base.write_text(original)
            gradle = root / files[3]
            gradle.write_text(gradle.read_text().replace('applicationIdSuffix = ".preview"', 'applicationIdSuffix = ".wrong"'))
            with self.assertRaises(ValueError):
                build.validate_preview_project(root)

    def test_clean_source_icons_are_generated_instead_of_using_old_mipmap_cache(self):
        with tempfile.TemporaryDirectory(prefix="Android icon recipe ") as directory:
            root = Path(directory)
            source = root / "apps/desktop/src-tauri/icons/icon.png"
            source.parent.mkdir(parents=True)
            source.write_bytes(b"existing artwork fixture")
            npm = root / "npm"
            npm.write_text("#!" + sys.executable + "\nimport pathlib,sys\n"
                           "output=pathlib.Path(sys.argv[sys.argv.index('--output')+1])\n"
                           "for density in ['mdpi','hdpi','xhdpi','xxhdpi','xxxhdpi']:\n"
                           " for name in ['ic_launcher','ic_launcher_round','ic_launcher_foreground']:\n"
                           "  path=output/'android'/('mipmap-'+density)/(name+'.png')\n"
                           "  path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(b'fresh icon fixture')\n")
            npm.chmod(0o755)
            result = build.prepare_icons(root, npm, root / ".local/generated-icons")
            self.assertEqual(len(result["resources"]), 15)
            self.assertEqual(source.read_bytes(), b"existing artwork fixture")
            destination = root / "apps/desktop/src-tauri/gen/android/app/src/main/res/mipmap-mdpi/ic_launcher.png"
            self.assertEqual(destination.read_bytes(), b"fresh icon fixture")

    def test_success_without_target_compilation_evidence_is_not_a_build(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "apps/desktop/src-tauri").mkdir(parents=True)
            for name in build.LOCKFILES:
                (root / name).write_text("lock")
            result = build.run_locked([sys.executable, "-c", "pass"], root, root / ".local/run",
                                      "aarch64-linux-android", Path(sys.executable), dict(os.environ))
            self.assertNotEqual(result["exit_code"], 0)


if __name__ == "__main__":
    unittest.main()
