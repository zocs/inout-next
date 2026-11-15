#!/usr/bin/env python3
"""Index original static-export bytes for Android AssetManager installation."""

import argparse
import hashlib
import os
from pathlib import Path
import re
import stat
import sys
import tempfile


MAX_FILES = 10_000
MAX_BYTES = 128 * 1024 * 1024
MAX_DEPTH = 128
MAX_INDEX_BYTES = 2 * 1024 * 1024
RESOURCE_PATH = re.compile(r"[A-Za-z0-9_.@/-]+")


def checked_name(relative: str) -> None:
    parts = relative.split("/")
    if (not RESOURCE_PATH.fullmatch(relative) or len(parts) > MAX_DEPTH
            or any(part in {"", ".", ".."} for part in parts)):
        raise ValueError("The export contains an invalid or excessively deep resource path")


def build_index(web_dir: Path) -> bytes:
    if web_dir.is_symlink() or not web_dir.is_dir():
        raise ValueError("The web export must be a real directory")
    if not (web_dir / "index.html").is_file() or not (web_dir / "_next").is_dir():
        raise ValueError("The web export is missing index.html or _next")
    web_dir = web_dir.resolve(strict=True)
    pending = [(web_dir, 0)]
    files = []
    total = 0
    scanned = 0
    while pending:
        directory, depth = pending.pop()
        if depth > MAX_DEPTH:
            raise ValueError("The web export exceeds the directory depth limit")
        with os.scandir(directory) as entries:
            for entry in entries:
                scanned += 1
                if scanned > MAX_FILES * MAX_DEPTH:
                    raise ValueError("The web export exceeds the entry scan limit")
                if entry.is_symlink():
                    raise ValueError("Symbolic links are not allowed in Android web resources")
                path = Path(entry.path)
                name = path.relative_to(web_dir).as_posix()
                checked_name(name)
                metadata = entry.stat(follow_symlinks=False)
                if stat.S_ISDIR(metadata.st_mode):
                    pending.append((path, depth + 1))
                elif stat.S_ISREG(metadata.st_mode):
                    total += metadata.st_size
                    files.append((name, path, metadata))
                    if len(files) > MAX_FILES or total > MAX_BYTES:
                        raise ValueError("The web export exceeds the file count or byte limit")
                else:
                    raise ValueError("Android web resources must be regular files")

    rows = []
    for name, path, metadata in sorted(files):
        digest = hashlib.sha256()
        read = 0
        descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
        with os.fdopen(descriptor, "rb") as stream:
            opened = os.fstat(stream.fileno())
            if (opened.st_dev, opened.st_ino, opened.st_size) != (
                    metadata.st_dev, metadata.st_ino, metadata.st_size):
                raise ValueError("The web export changed while preparing its index")
            while chunk := stream.read(128 * 1024):
                read += len(chunk)
                if read > metadata.st_size:
                    raise ValueError("A web resource grew while preparing its index")
                digest.update(chunk)
        if read != metadata.st_size:
            raise ValueError("A web resource changed while preparing its index")
        rows.append(f"{digest.hexdigest()}\t{read}\t{name}\n")
    table = "".join(rows).encode("ascii")
    header = f"INOUTWEB1\t{hashlib.sha256(table).hexdigest()}\t{len(files)}\t{total}\n".encode("ascii")
    index = header + table
    if len(index) > MAX_INDEX_BYTES:
        raise ValueError("The Android web index exceeds its byte limit")
    return index


def write_index(output: Path, index: bytes) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="wb", dir=output.parent,
                                         prefix=output.name + ".", suffix=".tmp", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(index)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, output)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main() -> int:
    project = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--web-dir", type=Path, default=project / "apps/web/out")
    parser.add_argument("--output", type=Path, default=project / ".local/android-assets/web-index.tsv")
    arguments = parser.parse_args()
    try:
        index = build_index(arguments.web_dir)
        write_index(arguments.output, index)
        header = index.split(b"\n", 1)[0].decode().split("\t")
        print(f"Android web index: {header[2]} files, {header[3]} bytes, snapshot {header[1]}")
        return 0
    except (OSError, ValueError) as error:
        print(f"Android assets error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
