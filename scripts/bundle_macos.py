#!/usr/bin/env python3
"""Retry only macOS DMG creation EBUSY, reusing an already compiled application.

Pass a Tauri bundle-only command after --. Each attempt retains its output; the
CI job timeout owns command lifetime. This helper never builds Rust or web assets.
"""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import time


CREATE_BUSY = b"hdiutil: create failed - Resource busy"
DELAYS = (2, 4)


def bundle(command, report):
    logs = report.with_suffix("")
    logs.mkdir(parents=True)
    attempts = []
    for number in range(1, 4):
        try:
            result = subprocess.run(command, capture_output=True, check=False)
        except OSError as error:
            result = subprocess.CompletedProcess(command, 1, b"", str(error).encode())
        stdout = logs / f"attempt-{number:02}.stdout"
        stderr = logs / f"attempt-{number:02}.stderr"
        stdout.write_bytes(result.stdout)
        stderr.write_bytes(result.stderr)
        sys.stdout.buffer.write(result.stdout)
        sys.stderr.buffer.write(result.stderr)
        sys.stdout.flush()
        sys.stderr.flush()
        retryable = result.returncode > 0 and CREATE_BUSY in result.stdout + result.stderr
        attempts.append({"number": number, "exit_code": result.returncode,
                         "creation_busy": retryable, "stdout_file": str(stdout),
                         "stderr_file": str(stderr)})
        report.write_text(json.dumps({"command": command, "maximum_attempts": 3,
                                      "passed": result.returncode == 0,
                                      "attempts": attempts}, indent=2) + "\n")
        if result.returncode == 0:
            return 0
        if not retryable or number == 3:
            return result.returncode if result.returncode > 0 else 1
        delay = DELAYS[number - 1]
        print(f"DMG creation reported EBUSY; retrying bundle only in {delay}s "
              f"(attempt {number + 1}/3).", file=sys.stderr, flush=True)
        time.sleep(delay)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("a bundle-only command is required after --")
    report = args.report.resolve()
    if report.exists() or report.with_suffix("").exists():
        parser.error("the report or attempt directory already exists; preserve its evidence")
    return bundle(command, report)


if __name__ == "__main__":
    raise SystemExit(main())
