#!/usr/bin/env python3
"""Synchronize product-facing metadata while preserving explicit app identity."""

import argparse
import json
from pathlib import Path
import re
import sys
from urllib.parse import urlsplit
from uuid import UUID


FIELDS = {"name", "slug", "description", "version", "application_id", "repository"}


def load_product(root: Path) -> dict:
    product = json.loads((root / "product.json").read_text(encoding="utf-8"))
    if not isinstance(product, dict) or set(product) != FIELDS:
        raise ValueError("product.json must contain exactly the six documented product fields")
    for key, value in product.items():
        if not isinstance(value, str) or not value.strip() or any(ord(c) < 32 for c in value):
            raise ValueError(f"{key} must be a nonempty string without control characters")
    if len(product["name"]) > 80 or len(product["description"]) > 240:
        raise ValueError("name or description exceeds its documented display limit")
    if not re.fullmatch(r"[a-z][a-z0-9]*(?:-[a-z0-9]+)*", product["slug"]) or len(product["slug"]) > 64:
        raise ValueError("slug must be a lowercase portable name separated with hyphens")
    if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", product["version"]):
        raise ValueError("version must be a semantic version")
    if not re.fullmatch(r"[A-Za-z][A-Za-z0-9-]*(?:\.[A-Za-z][A-Za-z0-9-]*)+", product["application_id"]):
        raise ValueError("application_id must be an explicit reverse-domain identifier")
    repository = urlsplit(product["repository"])
    if repository.scheme != "https" or not repository.netloc or repository.username or repository.password:
        raise ValueError("repository must be an HTTPS URL without credentials")
    return product


def load_native_identity(root: Path, product: dict) -> dict:
    path = root / "config/native-identity.json"
    try:
        identity = json.loads(path.read_text(encoding="utf-8"))
    except ValueError as error:
        raise ValueError("native-identity.json must contain valid JSON") from error
    if not isinstance(identity, dict) or set(identity) != {
        "application_id", "product_name", "wix_upgrade_code",
    }:
        raise ValueError("native-identity.json must contain exactly the three native identity fields")
    if any(not isinstance(value, str) or not value.strip() for value in identity.values()):
        raise ValueError("native identity fields must be nonempty strings")
    if identity["application_id"] != product["application_id"]:
        raise ValueError("native identity application_id must match product.json; changes require an explicit migration")
    name = identity["product_name"]
    stem = name.split(".")[0].upper()
    if (len(name) > 80 or name != name.strip() or name.endswith(".")
            or any(ord(c) < 32 or ord(c) == 127 or c in '\\/:<>"|?*' for c in name)
            or stem in {"CON", "PRN", "AUX", "NUL"}
            or re.fullmatch(r"(?:COM|LPT)[1-9¹²³]", stem)):
        raise ValueError("native product_name must be a portable installation name of at most 80 characters")
    code = identity["wix_upgrade_code"]
    try:
        upgrade = UUID(code)
    except ValueError as error:
        raise ValueError("native wix_upgrade_code must be an explicit nonzero canonical UUID") from error
    if not upgrade.int or str(upgrade) != code.lower():
        raise ValueError("native wix_upgrade_code must be an explicit nonzero canonical UUID")
    return identity


def generated_files(product: dict, identity: dict) -> dict[str, dict]:
    native = {
        "$schema": "https://schema.tauri.app/config/2",
        "productName": identity["product_name"],
        "version": product["version"],
        "identifier": product["application_id"],
        "build": {
            "beforeDevCommand": "npm --prefix ../web run dev -- --port 3100",
            "devUrl": "http://localhost:3100",
            "beforeBuildCommand": "npm --prefix ../web run build",
            "frontendDist": "../../web/out",
        },
        "app": {
            "windows": [{
                "label": "main", "title": product["name"],
                "width": 1060, "height": 760, "minWidth": 360, "minHeight": 560,
            }],
            "security": {
                "csp": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost; img-src 'self' data:; object-src 'none'; base-uri 'none'",
            },
        },
        "bundle": {
            "active": True,
            "targets": "all",
            "shortDescription": product["description"],
            "category": "Utility",
            "resources": {"../../web/out/": "web/"},
            "icon": ["icons/icon.png", "icons/icon.ico", "icons/icon.icns"],
            "windows": {
                "webviewInstallMode": {"type": "downloadBootstrapper"},
                "wix": {"upgradeCode": identity["wix_upgrade_code"]},
            },
        },
    }
    distribution = {
        "product": product,
        "archive_prefix": product["slug"],
        "server_binary": "share-server",
        "desktop_binary": "share-desktop",
        "web_directory": "web",
        "runtime_node_required": False,
    }
    return {
        "apps/web/src/generated/product.json": product,
        "apps/desktop/src-tauri/tauri.conf.json": native,
        "config/distribution.json": distribution,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--check", action="store_true", help="Report stale generated files without changing them")
    args = parser.parse_args()
    try:
        product = load_product(args.root)
        files = generated_files(product, load_native_identity(args.root, product))
        stale = []
        for relative, value in files.items():
            path = args.root / relative
            expected = json.dumps(value, ensure_ascii=False, indent=2) + "\n"
            if args.check:
                if not path.is_file() or path.read_text(encoding="utf-8") != expected:
                    stale.append(relative)
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(expected, encoding="utf-8")
        if stale:
            print("Stale product metadata: " + ", ".join(stale), file=sys.stderr)
            return 1
        print("Product metadata checked." if args.check else "Product metadata synchronized.")
        return 0
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"Product configuration error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
