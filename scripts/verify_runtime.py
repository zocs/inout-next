#!/usr/bin/env python3
"""Exercise the real CLI with streamed HTTP transfers and independent file hashes."""

import argparse
import hashlib
import http.client
import json
import os
from pathlib import Path
import queue
import random
import re
import secrets
import signal
import subprocess
import sys
import tempfile
import threading
import time
from urllib.parse import urlsplit


CHUNK = 64 * 1024
MIB = 1024 * 1024


def rss_bytes(pid: int) -> int | None:
    try:
        text = Path(f"/proc/{pid}/status").read_text()
        match = re.search(r"^VmRSS:\s+(\d+) kB$", text, re.MULTILINE)
        return int(match[1]) * 1024 if match else None
    except OSError:
        return None


def digest_file(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify(binary: Path, web_dir: Path, size: int) -> dict:
    if not binary.is_file() or not (web_dir / "index.html").is_file():
        raise ValueError("Build the server binary and exported web assets before the runtime probe")
    with tempfile.TemporaryDirectory(prefix="share-runtime-") as directory:
        temporary = Path(directory)
        root = temporary / "shared"
        root.mkdir()
        source = temporary / "source.bin"
        generator = random.Random(491_927)
        with source.open("wb") as stream:
            remaining = size
            while remaining:
                chunk = generator.randbytes(min(CHUNK, remaining))
                stream.write(chunk)
                remaining -= len(chunk)
        expected = digest_file(source)
        key = secrets.token_urlsafe(32)
        key_file = temporary / "access.key"
        with key_file.open("x", encoding="ascii") as stream:
            stream.write(key + "\n")
        if os.name != "nt":
            key_file.chmod(0o600)
        process = subprocess.Popen([
            str(binary.resolve()), "--root", str(root), "--listen", "127.0.0.1:0",
            "--web-dir", str(web_dir.resolve()), "--writable", "--key-file", str(key_file),
        ], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        address = queue.Queue()

        def ready():
            assert process.stdout is not None
            for line in process.stdout:
                match = re.search(r"Listening on (http://127\.0\.0\.1:\d+)", line)
                if match:
                    address.put(match[1])

        reader = threading.Thread(target=ready, daemon=True)
        reader.start()
        stop_sampling = threading.Event()
        sampler = None
        peak = [0]
        connection = None
        clean_stop = False
        started = time.monotonic()
        try:
            try:
                url = address.get(timeout=15)
            except queue.Empty as error:
                raise RuntimeError("The Rust host did not announce readiness within 15 seconds") from error
            baseline = rss_bytes(process.pid)
            peak[0] = baseline or 0

            def sample():
                while not stop_sampling.is_set():
                    current = rss_bytes(process.pid)
                    if current is not None:
                        peak[0] = max(peak[0], current)
                    stop_sampling.wait(0.025)

            sampler = threading.Thread(target=sample, daemon=True)
            sampler.start()
            origin = urlsplit(url)
            connection = http.client.HTTPConnection(origin.hostname, origin.port, timeout=60)
            headers = {"Authorization": "Bearer " + key}
            connection.request("GET", "/")
            response = connection.getresponse()
            if response.status != 200 or b"<html" not in response.read():
                raise RuntimeError("The host did not serve the exported index page")
            connection.putrequest("PUT", "/api/v1/content?path=transfer.bin")
            connection.putheader("Authorization", headers["Authorization"])
            connection.putheader("Content-Type", "application/octet-stream")
            connection.putheader("Content-Length", str(size))
            connection.endheaders()
            with source.open("rb") as stream:
                while chunk := stream.read(CHUNK):
                    connection.send(chunk)
            response = connection.getresponse()
            response.read()
            if response.status != 201:
                raise RuntimeError(f"Streamed upload returned HTTP {response.status}")
            if digest_file(root / "transfer.bin") != expected:
                raise RuntimeError("Published filesystem bytes differ from the source")
            connection.request("GET", "/api/v1/content?path=transfer.bin", headers=headers)
            response = connection.getresponse()
            if response.status != 200:
                raise RuntimeError(f"Streamed download returned HTTP {response.status}")
            received = hashlib.sha256()
            downloaded = 0
            while chunk := response.read(CHUNK):
                downloaded += len(chunk)
                received.update(chunk)
            if downloaded != size or received.hexdigest() != expected:
                raise RuntimeError("Downloaded length or SHA-256 differs from the independent source")
            connection.request("GET", "/api/v1/content?path=transfer.bin", headers={**headers, "Range": "bytes=-64"})
            response = connection.getresponse()
            with source.open("rb") as stream:
                stream.seek(-64, os.SEEK_END)
                tail = stream.read(64)
            if response.status != 206 or response.read() != tail:
                raise RuntimeError("The suffix range differs from the source tail")
            connection.close()
            connection = None
            container = root / "INOUTTMP"
            if sorted(path.name for path in root.iterdir()) != ["INOUTTMP", "transfer.bin"] or not container.is_dir() or any(container.iterdir()):
                raise RuntimeError("The completed transfer left unpublished staging files in shared storage")
            connection = http.client.HTTPConnection(origin.hostname, origin.port, timeout=60)
            connection.request("GET", "/api/v1/entries?path=", headers=headers)
            response = connection.getresponse()
            listing = json.loads(response.read())
            if response.status != 200 or [entry["name"] for entry in listing["entries"]] != ["transfer.bin"]:
                raise RuntimeError("Public file listing exposes internal state or omits the published file")
            connection.close()
            connection = None
            growth = peak[0] - baseline if baseline is not None else None
            if growth is not None and growth > max(24 * MIB, size // 2):
                raise RuntimeError("Server RSS growth exceeded the bounded-stream probe threshold")
            if os.name == "nt":
                process.terminate()
            else:
                process.send_signal(signal.SIGINT)
            process.wait(timeout=10)
            clean_stop = os.name != "nt" and process.returncode == 0
            if os.name != "nt" and not clean_stop:
                raise RuntimeError("The CLI did not exit cleanly after Ctrl-C")
            return {
                "payload_bytes": size,
                "sha256": expected,
                "upload_download_and_suffix": "passed",
                "public_listing_and_empty_staging_container": "passed",
                "baseline_rss_bytes": baseline,
                "peak_rss_bytes": peak[0] if baseline is not None else None,
                "rss_growth_bytes": growth,
                "rss_source": "Linux /proc VmRSS" if baseline is not None else "unavailable on this host",
                "ctrl_c_clean_stop": clean_stop,
                "elapsed_seconds": round(time.monotonic() - started, 3),
            }
        finally:
            if connection is not None:
                connection.close()
            stop_sampling.set()
            if sampler is not None:
                sampler.join(timeout=1)
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            reader.join(timeout=1)
            if process.stdout is not None:
                process.stdout.close()


def main() -> int:
    project = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=project / "target/debug" / ("share-server.exe" if os.name == "nt" else "share-server"))
    parser.add_argument("--web-dir", type=Path, default=project / "apps/web/out")
    parser.add_argument("--mib", type=int, default=64)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    if not 1 <= args.mib <= 4096:
        parser.error("--mib must be between 1 and 4096")
    try:
        report = verify(args.binary, args.web_dir, args.mib * MIB)
        rendered = json.dumps(report, indent=2) + "\n"
        if args.report is not None:
            args.report.parent.mkdir(parents=True, exist_ok=True)
            args.report.write_text(rendered, encoding="utf-8")
        print(rendered, end="")
        return 0
    except (OSError, ValueError, RuntimeError, subprocess.TimeoutExpired, http.client.HTTPException) as error:
        print(f"Runtime verification failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
