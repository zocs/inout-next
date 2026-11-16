#!/usr/bin/env python3
"""Package the standalone server and exported local UI without build tools or keys."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

from sync_product import load_product


TARGETS = {"linux-x64", "linux-arm64", "windows-x86", "windows-x64", "macos-x64", "macos-arm64"}
MAX_WEB_BYTES = 128 * 1024 * 1024
MAX_WEB_FILES = 10_000


def runtime_inputs(root: Path, binary: Path, target: str) -> dict[str, tuple[bytes, int]]:
    web = root / "apps/web/out"
    if not web.is_dir() or not (web / "index.html").is_file() or not (web / "_next").is_dir():
        raise ValueError("Exported web assets are missing; build apps/web before packaging")
    if web.is_symlink():
        raise ValueError("Web assets must not contain a symlink")
    inputs = {}
    total = 0
    for path in sorted(web.rglob("*")):
        if path.is_symlink():
            raise ValueError("Web assets must not contain a symlink")
        if path.is_dir():
            continue
        if not path.is_file():
            raise ValueError("Web assets must contain only regular files")
        total += path.stat().st_size
        if total > MAX_WEB_BYTES or len(inputs) >= MAX_WEB_FILES:
            raise ValueError("Web assets exceed the service snapshot limits")
        inputs["web/" + path.relative_to(web).as_posix()] = (path.read_bytes(), 0o644)
    if binary.is_symlink() or not binary.is_file():
        raise ValueError("The server binary must be a regular file, not a symlink")
    name = "share-server.exe" if target.startswith("windows-") else "share-server"
    inputs[name] = (binary.read_bytes(), 0o755)
    for name in ("LICENSE", "README.md"):
        path = root / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"The current {name} is required for packaging")
        inputs[name] = (path.read_bytes(), 0o644)
    command = ".\\share-server.exe" if target.startswith("windows-") else "./share-server"
    instructions = f"""Standalone file service

Extract the entire archive. Run the command from the extracted directory:

{command} --root /path/to/your/shared-directory --web-dir ./web --product-file ./product.json --listen 127.0.0.1:5000

Replace the shared-directory path with an existing folder. Keep it separate from
the extracted web/ folder: public UI assets cannot overlap shared storage.
Open http://127.0.0.1:5000 in a browser. Ctrl-C stops the service.
This default is read-only. For access from other devices, bind to 0.0.0.0:5000
and use this computer's LAN address; the firewall must allow the selected port.
For write operations use --writable --key-file /path/to/a/private-key-file.
Keep the key file outside shared storage and use at least 32 random characters.
HTTP on a LAN does not encrypt transferred files or access keys.

The web UI is already built. Node and Python are not required to run this server.
This archive is the CLI host, not the Tauri desktop installer. See README.md
for filesystem/platform limitations and validated targets.
"""
    inputs["QUICKSTART.txt"] = (instructions.encode("utf-8"), 0o644)
    return inputs


def write_archive(archive_path: Path, inputs: dict[str, tuple[bytes, int]]) -> str:
    """Publish a deterministic runtime ZIP and checksum after input validation."""
    temporary = None
    try:
        archive_path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(prefix=".package-", suffix=".tmp", dir=archive_path.parent, delete=False) as stream:
            temporary = Path(stream.name)
        with ZipFile(temporary, "w", compression=ZIP_DEFLATED) as archive:
            for name, (contents, mode) in sorted(inputs.items()):
                info = ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
                info.create_system = 3
                info.external_attr = (stat.S_IFREG | mode) << 16
                info.compress_type = ZIP_DEFLATED
                archive.writestr(info, contents)
        with temporary.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        os.replace(temporary, archive_path)
        temporary = None
        archive_path.with_suffix(".zip.sha256").write_text(f"{digest}  {archive_path.name}\n", encoding="ascii")
        return digest
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", choices=sorted(TARGETS), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        product = load_product(args.root)
        inputs = runtime_inputs(args.root, args.binary, args.target)
        inputs["product.json"] = ((json.dumps(product, ensure_ascii=False, indent=2) + "\n").encode("utf-8"), 0o644)
        archive_path = args.output / f'{product["slug"]}-{product["version"]}-{args.target}-server.zip'
        digest = write_archive(archive_path, inputs)
        print(f"Packaged {archive_path.name} (SHA-256 {digest}).")
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"Packaging error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
