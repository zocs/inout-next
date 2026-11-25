#!/usr/bin/env python3
"""Package the opt-in native desktop trial with embedded web assets and notices."""

import argparse
import json
from pathlib import Path
import sys

from package_portable import TARGETS, write_archive
from sync_product import load_product


def native_inputs(root: Path, binary: Path, target: str) -> dict[str, tuple[bytes, int]]:
    windows = target.startswith("windows-")
    executable = "share-native-slint.exe" if windows else "share-native-slint"
    if binary.is_symlink() or not binary.is_file():
        raise ValueError("The native binary must be a regular file, not a symlink")
    inputs = {executable: (binary.read_bytes(), 0o755)}
    license_path = root / "LICENSE"
    if license_path.is_symlink() or not license_path.is_file():
        raise ValueError("The current LICENSE is required for packaging")
    inputs["LICENSE"] = (license_path.read_bytes(), 0o644)
    app = root / "apps/native-slint"
    for name in ("README.md", "THIRD_PARTY.md", "LICENSES/Slint-Royalty-free-2.0.md"):
        path = app / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"Native trial {name} is required for packaging")
        inputs[name] = (path.read_bytes(), 0o644)
    command = f".\\{executable}" if windows else f"./{executable}"
    instructions = f"""Native desktop interface trial

Extract the entire archive and run {command}. Browser assets are embedded in
the release executable; no separate web/ directory or working directory is needed.
Select any existing folder, including one containing the application package,
choose permissions, and start sharing. A LAN firewall must allow the selected port.
The access key is optional. Public upload/management can be explicitly enabled;
published deletion requires configured-key authentication. HTTP does not encrypt
files or access keys. Confirm Stop/close only when you intend to interrupt sharing.

This trial uses Slint with winit and prefers FemtoVG. Windows can fall back to
software rendering when a suitable OpenGL driver is unavailable. Compatible OS
graphics (or that fallback), fonts, clipboard and file-dialog services are required.
Linux also requires the libraries
listed in README.md. This ZIP does not establish offline/old-OS installation,
device, signing or store acceptance. Node, Python and WebView are not app runtimes.
Language, accent, appearance and sharing setup drafts are saved in the
per-user application configuration directory. The access key is never saved.
INOUT_NATIVE_PREFERENCES selects a different preferences file; an empty value
disables saving. INOUT_NATIVE_REDUCE_MOTION=1 disables startup motion;
OS motion-preference detection is a separate milestone.
Open About for Slint attribution; retain the included licences and notices.
"""
    inputs["QUICKSTART.txt"] = (instructions.encode("utf-8"), 0o644)
    return inputs


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", choices=sorted(TARGETS), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        product = load_product(args.root)
        inputs = native_inputs(args.root, args.binary, args.target)
        inputs["product.json"] = ((json.dumps(product, ensure_ascii=False, indent=2) + "\n").encode("utf-8"), 0o644)
        archive = args.output / f'{product["slug"]}-{product["version"]}-{args.target}-native-slint-trial.zip'
        digest = write_archive(archive, inputs)
        print(f"Packaged {archive.name} (SHA-256 {digest}).")
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"Native packaging error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
