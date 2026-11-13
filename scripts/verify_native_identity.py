#!/usr/bin/env python3
"""Rebundle an existing executable in temporary targets; inspect rename identity.

Checks emitted Debian packages or rendered/compiled NSIS installers, not installed
upgrades, runtime rebranding, signing, or MSI behavior. Never builds Rust or web.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


PROJECT = Path(__file__).resolve().parents[1]


def run(command, **kwargs):
    return subprocess.run(command, check=True, **kwargs)


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def web_hashes(directory):
    return {path.relative_to(directory).as_posix(): sha256(path)
            for path in sorted(directory.rglob("*")) if path.is_file()}


def inspect_deb(artifact, directory, web, binary_hash):
    package = run(["dpkg-deb", "--field", str(artifact), "Package"],
                  capture_output=True, text=True).stdout.strip()
    run(["dpkg-deb", "--extract", str(artifact), str(directory)])
    resources = list((directory / "usr/lib").glob("*/web"))
    if len(resources) != 1:
        raise ValueError("Debian package must contain one native web resource directory")
    if web_hashes(resources[0]) != web:
        raise ValueError("Debian web resource hashes differ from the existing export")
    if sha256(directory / "usr/bin/share-desktop") != binary_hash:
        raise ValueError("Debian executable differs from the supplied executable")
    return {"package": package,
            "resource_directory": resources[0].relative_to(directory).as_posix(),
            "binary": "usr/bin/share-desktop", "web_files_verified": len(web)}


def inspect_nsis(template, web):
    text = template.read_text(encoding="utf-8-sig")
    defines = dict(re.findall(r'^!define (\w+) "(.*)"$', text, re.MULTILINE))

    def expand(value):
        for _ in range(8):
            updated = re.sub(r"\$\{(\w+)\}",
                             lambda match: defines.get(match[1], match[0]), value)
            if updated == value:
                return value
            value = updated
        raise ValueError("NSIS identity definitions did not resolve")

    keys = ("PRODUCTNAME", "BUNDLEID", "MANUFACTURER", "MAINBINARYNAME",
            "INSTALLMODE", "UNINSTKEY", "MANUKEY", "MANUPRODUCTKEY",
            "PLACEHOLDER_INSTALL_DIR", "STARTMENUFOLDER")
    identity = {key: expand(defines[key]) for key in keys}
    identity["install_directories"] = sorted(set(expand(value) for value in re.findall(
        r'StrCpy \$INSTDIR "([^"]+)"', text)))
    identity["registry_operations"] = sorted(set(expand(line.strip())
        for line in text.splitlines()
        if re.match(r"\s*(ReadRegStr|WriteRegStr|WriteRegDWORD|DeleteRegKey|DeleteRegValue)\b", line)
        and re.search(r"\$\{(?:UNINSTKEY|MANUPRODUCTKEY|MANUKEY)\}", line)))
    if not identity["install_directories"] or not identity["registry_operations"]:
        raise ValueError("Rendered NSIS template lacks install/registry identity operations")
    if not re.search(r'^\s*DeleteRegKey(?:\s+/ifempty)?\s+\w+\s+"\$\{UNINSTKEY\}"',
                     text, re.MULTILINE):
        raise ValueError("Rendered NSIS template lacks uninstall-key removal")
    resources = re.findall(r'File /a "/oname=([^"]+)" "([^"]+)"', text)
    packaged_web = {name.replace("\\", "/")[4:]: sha256(Path(source))
                    for name, source in resources if name.replace("\\", "/").startswith("web/")}
    if packaged_web != web:
        raise ValueError("Rendered NSIS resource destinations/hashes differ from the web export")
    identity["web_files_verified"] = len(packaged_web)
    return identity


def verify(binary, target, bundle):
    product = json.loads((PROJECT / "product.json").read_text(encoding="utf-8"))
    identity = json.loads((PROJECT / "config/native-identity.json").read_text(encoding="utf-8"))
    web = web_hashes(PROJECT / "apps/web/out")
    if "index.html" not in web:
        raise ValueError("An existing complete web export is required")
    binary_hash = sha256(binary)
    alternate_name = "Harbor" if product["name"] != "Harbor" else "Harbor Identity Check"
    alternate_slug = "harbor" if product["slug"] != "harbor" else "harbor-identity-check"
    fixtures = []
    with tempfile.TemporaryDirectory(prefix="native-identity-") as temporary:
        for label, branding in (("baseline", product), ("renamed", {
            **product, "name": alternate_name, "slug": alternate_slug,
            "description": "Files on your network",
        })):
            fixture = Path(temporary) / label
            (fixture / "config").mkdir(parents=True)
            (fixture / "product.json").write_text(json.dumps(branding), encoding="utf-8")
            shutil.copyfile(PROJECT / "config/native-identity.json", fixture / "config/native-identity.json")
            run([sys.executable, str(PROJECT / "scripts/sync_product.py"), "--root", str(fixture)])
            config = fixture / "apps/desktop/src-tauri/tauri.conf.json"
            native = json.loads(config.read_text(encoding="utf-8"))
            # Bundle reads config/resources in the real crate, but only writes to
            # this temporary target. Copy, never link, the existing executable.
            output = fixture / "target" / target / "release"
            output.mkdir(parents=True)
            shutil.copy2(binary, output / binary.name)
            environment = {**os.environ, "CARGO_TARGET_DIR": str(fixture / "target")}
            run(["npm.cmd" if os.name == "nt" else "npm", "run", "tauri", "--",
                 "bundle", "--ci", "--target", target, "--bundles", bundle,
                 "--no-sign", "--no-binary-patching", "--config", str(config),
                 "--config", '{"build":{"beforeBundleCommand":""}}'],
                cwd=PROJECT / "apps/desktop", env=environment)
            artifacts = list((output / "bundle" / bundle).glob("*.deb" if bundle == "deb" else "*.exe"))
            if len(artifacts) != 1:
                raise ValueError(f"Expected one emitted {bundle} artifact, found {len(artifacts)}")
            if bundle == "deb":
                emitted = inspect_deb(artifacts[0], fixture / "extracted", web, binary_hash)
            else:
                templates = list((output / "nsis").glob("*/installer.nsi"))
                if len(templates) != 1:
                    raise ValueError("Expected one actual rendered NSIS installer.nsi")
                emitted = inspect_nsis(templates[0], web)
            fixtures.append({"fixture": label, "display_name": branding["name"],
                             "window_title": native["app"]["windows"][0]["title"],
                             "identifier": native["identifier"],
                             "wix_upgrade_code": native["bundle"]["windows"].get("wix", {}).get("upgradeCode"),
                             "artifact": artifacts[0].name, "artifact_sha256": sha256(artifacts[0]),
                             "emitted_identity": emitted})
    checks = {
        "emitted_identity_invariant": fixtures[0]["emitted_identity"] == fixtures[1]["emitted_identity"],
        "display_title_updated": fixtures[0]["window_title"] == product["name"] and fixtures[1]["window_title"] == alternate_name,
        "explicit_identifier_invariant": all(item["identifier"] == identity["application_id"] for item in fixtures),
        "explicit_wix_uuid_invariant_config_only": all(item["wix_upgrade_code"] == identity["wix_upgrade_code"] for item in fixtures),
        "source_executable_unchanged": sha256(binary) == binary_hash,
        "source_export_unchanged": web_hashes(PROJECT / "apps/web/out") == web,
    }
    if bundle == "deb":
        checks["stable_resource_directory"] = all(item["emitted_identity"]["resource_directory"] == f'usr/lib/{identity["product_name"]}/web' for item in fixtures)
    else:
        checks["stable_native_names"] = all(
            item["emitted_identity"]["PRODUCTNAME"] == identity["product_name"]
            and item["emitted_identity"]["BUNDLEID"] == identity["application_id"]
            and item["emitted_identity"]["MAINBINARYNAME"] == "share-desktop"
            for item in fixtures)
    return {"bundle": bundle, "target": target, "executable_sha256": binary_hash,
            "method": "Tauri bundle only; existing executable copied into temporary targets",
            "fixtures": fixtures, "checks": checks, "passed": all(checks.values()),
            "installed_or_upgraded": False, "msi_verified": False, "runtime_rebranding_verified": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--target", required=True)
    parser.add_argument("--bundle", required=True, choices=("deb", "nsis"))
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    try:
        report = verify(args.binary.resolve(), args.target, args.bundle)
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(report, indent=2))
        return 0 if report["passed"] else 1
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"Native identity verification failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
