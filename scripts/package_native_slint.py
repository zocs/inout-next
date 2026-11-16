#!/usr/bin/env python3
"""Package the opt-in native desktop trial with its browser assets and notices."""

import argparse
import json
from pathlib import Path
import sys

from package_portable import TARGETS, runtime_inputs, write_archive
from sync_product import load_product


def native_inputs(root: Path, binary: Path, target: str) -> dict[str, tuple[bytes, int]]:
    inputs = runtime_inputs(root, binary, target)
    windows = target.startswith("windows-")
    server = "share-server.exe" if windows else "share-server"
    executable = "share-native-slint.exe" if windows else "share-native-slint"
    inputs[executable] = inputs.pop(server)
    app = root / "apps/native-slint"
    for name in ("README.md", "THIRD_PARTY.md", "LICENSES/Slint-Royalty-free-2.0.md"):
        path = app / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"Native trial {name} is required for packaging")
        inputs[name] = (path.read_bytes(), 0o644)
    command = f".\\{executable}" if windows else f"./{executable}"
    instructions = f"""Native desktop interface trial

Extract the entire archive and run {command}. Keep web/ beside the executable;
the application resolves its browser assets from that directory, independently
of the current working directory. Select an existing folder outside this package,
choose permissions, and start sharing. A LAN firewall must allow the selected port.
The access key is optional. Public upload/management can be explicitly enabled;
published deletion requires configured-key authentication. HTTP does not encrypt
files or access keys. Confirm Stop/close only when you intend to interrupt sharing.

This trial uses Slint with winit/FemtoVG. It requires compatible OS graphics,
fonts, clipboard and file-dialog services. Linux also requires the libraries
listed in README.md. This ZIP does not establish offline/old-OS installation,
device, signing or store acceptance. Node, Python and WebView are not app runtimes.
Preferences are temporary. INOUT_NATIVE_REDUCE_MOTION=1 disables startup motion;
OS motion-preference detection and saved settings are separate milestones.
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
