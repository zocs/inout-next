#!/usr/bin/env python3
"""Create an access-key file exclusively; never echo its contents."""

import argparse
import os
from pathlib import Path
import secrets
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    try:
        descriptor = os.open(args.path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    except FileExistsError:
        print("The key file already exists. Choose another path to preserve the existing key.", file=sys.stderr)
        return 1
    except OSError:
        print("Cannot create the key file. Check its parent directory and permissions.", file=sys.stderr)
        return 1
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            stream.write(secrets.token_urlsafe(32) + "\n")
    except OSError:
        args.path.unlink(missing_ok=True)
        print("Could not write the key file.", file=sys.stderr)
        return 1
    print(f"Access key created at {args.path}. Read it locally when connecting a browser.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
