"""Exercise bundle retry policy with real children, not hdiutil or macOS artifacts."""

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


PROJECT = Path(__file__).resolve().parents[1]
CHILD = """
import json
from pathlib import Path
import sys
scenario = json.loads(sys.argv[1])
counter = Path(sys.argv[2])
calls = counter.read_text().splitlines() if counter.exists() else []
with counter.open('a') as stream:
    stream.write('attempt\\n')
result = scenario[min(len(calls), len(scenario) - 1)]
print(result.get('stdout', ''), end='')
print(result.get('stderr', ''), end='', file=sys.stderr)
if result['exit'] == 0:
    Path(sys.argv[3]).write_bytes(b'disk-image fixture')
raise SystemExit(result['exit'])
"""


class MacosBundleRetryTests(unittest.TestCase):
    def run_scenario(self, scenario):
        temporary = tempfile.TemporaryDirectory(prefix="dmg policy ")
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        child = root / "bundler.py"
        child.write_text(CHILD)
        report = root / "attempts.json"
        counter = root / "calls"
        artifact = root / "candidate.dmg"
        result = subprocess.run([
            sys.executable, str(PROJECT / "scripts/bundle_macos.py"),
            "--report", str(report), "--", sys.executable, str(child),
            json.dumps(scenario), str(counter), str(artifact),
        ], capture_output=True, timeout=20, check=False)
        return result, report, counter, artifact

    def test_creation_busy_recovers_without_losing_successful_artifact(self):
        # A one-attempt or overbroad abort policy would leave no candidate.
        error = "hdiutil: create failed - Resource busy\n"
        result, report, counter, artifact = self.run_scenario([
            {"exit": 16, "stderr": error}, {"exit": 0, "stdout": "Bundled\n"},
        ])
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(counter.read_text().splitlines(), ["attempt", "attempt"])
        self.assertEqual(artifact.read_bytes(), b"disk-image fixture")
        data = json.loads(report.read_text())
        self.assertTrue(data["passed"])
        self.assertIn(error.encode(), result.stderr)
        self.assertEqual(Path(data["attempts"][0]["stderr_file"]).read_text(), error)

    def test_other_errors_stop_and_preserve_their_exit_status(self):
        # Retrying every failure would repeat signing or non-create disk errors.
        for status, message in (
            (9, "codesign failed: invalid identity\n"),
            (16, "hdiutil: attach failed - Resource busy\n"),
            (13, "hdiutil: create failed - Permission denied\n"),
        ):
            with self.subTest(error=message):
                result, report, counter, artifact = self.run_scenario([
                    {"exit": status, "stderr": message}, {"exit": 0},
                ])
                self.assertEqual(result.returncode, status, result.stderr.decode())
                self.assertEqual(counter.read_text().splitlines(), ["attempt"])
                self.assertFalse(artifact.exists())
                self.assertFalse(json.loads(report.read_text())["passed"])
                self.assertIn(message.encode(), result.stderr)

    def test_persistent_creation_busy_stops_after_three_attempts(self):
        # An unbounded loop or false success would hide a broken disk-image job.
        result, report, counter, artifact = self.run_scenario([
            {"exit": 16, "stderr": "hdiutil: create failed - Resource busy\n"},
        ])
        self.assertEqual(result.returncode, 16, result.stderr.decode())
        self.assertEqual(counter.read_text().splitlines(), ["attempt"] * 3)
        self.assertFalse(artifact.exists())
        self.assertFalse(json.loads(report.read_text())["passed"])

    def test_success_does_not_run_the_packager_again(self):
        # Continuing after success could replace a valid candidate or fail later.
        result, report, counter, artifact = self.run_scenario([
            {"exit": 0}, {"exit": 9, "stderr": "should not run\n"},
        ])
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(counter.read_text().splitlines(), ["attempt"])
        self.assertEqual(artifact.read_bytes(), b"disk-image fixture")
        self.assertTrue(json.loads(report.read_text())["passed"])


if __name__ == "__main__":
    unittest.main()
