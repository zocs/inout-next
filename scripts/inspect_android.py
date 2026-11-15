#!/usr/bin/env python3
"""Inspect preview APK bytes; signing and target-device runtime are separate gates."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import struct
import subprocess
import sys
import zipfile

from prepare_android_assets import MAX_BYTES, MAX_FILES, MAX_INDEX_BYTES, checked_name, build_index

ABI_TARGETS = {"aarch64": "arm64-v8a", "x86_64": "x86_64"}
MACHINES = {"arm64-v8a": 183, "x86_64": 62}
PREVIEW_ID = "io.github.zocs.inoutnext.preview"
MAX_APK_BYTES = 512 * 1024 * 1024
MAX_LIBRARY_BYTES = 128 * 1024 * 1024


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def parse_index(data):
    if len(data) > MAX_INDEX_BYTES or not data.endswith(b"\n"):
        raise ValueError("The packaged resource index is oversized or incomplete")
    try:
        header, table = data.decode("ascii").split("\n", 1)
        kind, snapshot, count, total = header.split("\t")
        if (kind != "INOUTWEB1" or not re.fullmatch(r"[0-9a-f]{64}", snapshot)
                or not count.isdecimal() or not total.isdecimal()
                or not 1 <= int(count) <= MAX_FILES or int(total) > MAX_BYTES
                or sha256(table.encode("ascii")) != snapshot):
            raise ValueError("The resource index header or table checksum is invalid")
        entries = {}
        previous = ""
        summed = 0
        for row in table[:-1].split("\n"):
            digest, size, name = row.split("\t")
            checked_name(name)
            if (not re.fullmatch(r"[0-9a-f]{64}", digest) or not size.isdecimal()
                    or int(size) > MAX_BYTES or name <= previous):
                raise ValueError("Resource index entries must have valid sizes/hashes and sorted unique paths")
            entries[name] = {"bytes": int(size), "sha256": digest}
            previous = name
            summed += int(size)
        if len(entries) != int(count) or summed != int(total) or summed > MAX_BYTES:
            raise ValueError("The resource index count or total differs")
        directories = {"_next"}
        for name in entries:
            parts = name.split("/")
            directories.update("/".join(parts[:depth]) for depth in range(1, len(parts)))
        if "index.html" not in entries or directories.intersection(entries):
            raise ValueError("The entrypoint is missing or resource files conflict with directories")
        return {"snapshot": snapshot, "entries": entries, "total_bytes": summed}
    except (UnicodeError, TypeError) as error:
        raise ValueError("The resource index must be valid ASCII") from error


def inspect_elf(data, abi):
    if len(data) < 64 or data[:7] != b"\x7fELF\x02\x01\x01":
        raise ValueError("A native library is not a complete little-endian ELF64")
    fields = struct.unpack_from("<HHIQQQIHHHHHH", data, 16)
    kind, machine, version, _, phoff, _, _, ehsize, phsize, phcount, _, _, _ = fields
    if (kind != 3 or machine != MACHINES[abi] or version != 1 or ehsize != 64
            or phsize != 56 or not 1 <= phcount <= 4096 or phoff < 64
            or phoff + phsize * phcount > len(data)):
        raise ValueError("Native ELF architecture, type or program headers are invalid")
    loads = []
    relro = []
    for number in range(phcount):
        ptype, flags, offset, virtual, _, filesz, memsz, alignment = struct.unpack_from(
            "<IIQQQQQQ", data, phoff + number * phsize)
        if ptype not in (1, 0x6474e552):
            continue
        if filesz > memsz or offset + filesz > len(data):
            raise ValueError("Native ELF segment extends beyond its file or memory mapping")
        segment = {"offset": offset, "virtual_address": virtual, "file_bytes": filesz,
                   "memory_bytes": memsz, "alignment": alignment, "flags": flags}
        if ptype == 1:
            if (alignment < 16384 or alignment & (alignment - 1)
                    or offset % alignment != virtual % alignment):
                raise ValueError("A native ELF LOAD segment is insufficiently aligned for 16 KB pages")
            loads.append(segment)
        else:
            relro.append(segment)
    if not loads:
        raise ValueError("Native ELF has no LOAD segments")
    return {"machine": machine, "elf_class": 64, "load_segments": loads, "relro_segments": relro,
            "page_size_runtime_verified": False}


def checked_member(info):
    name = info.filename
    parts = name.rstrip("/").split("/")
    if (info.orig_filename != name or "\x00" in name or "\\" in name or name.startswith("/")
            or any(part in {"", ".", ".."} for part in parts) or ":" in parts[0]
            or info.flag_bits & 1 or info.file_size < 0):
        raise ValueError(f"Unsafe or encrypted APK archive member: {name!r}")
    mode = info.external_attr >> 16
    if stat.S_IFMT(mode) not in (0, stat.S_IFREG, stat.S_IFDIR):
        raise ValueError(f"APK archive contains a nonregular member: {name!r}")


def native_zip_offset(archive, info):
    archive.fp.seek(info.header_offset)
    header = archive.fp.read(30)
    if len(header) != 30 or header[:4] != b"PK\x03\x04":
        raise ValueError("A native library ZIP local header is malformed")
    name_length, extra_length = struct.unpack_from("<HH", header, 26)
    return info.header_offset + 30 + name_length + extra_length


def inspect_archive(archive, abi, index):
    indexed = parse_index(index)
    infos = archive.infolist()
    if len(infos) > 50_000 or sum(info.file_size for info in infos) > MAX_APK_BYTES:
        raise ValueError("APK archive exceeds inspection bounds")
    names = set()
    for info in infos:
        checked_member(info)
        if info.filename in names:
            raise ValueError(f"Duplicate APK archive member: {info.filename}")
        names.add(info.filename)
    if "AndroidManifest.xml" not in names or "assets/inout-web-index.tsv" not in names:
        raise ValueError("APK lacks its manifest or indexed resource contract")
    if archive.read("assets/inout-web-index.tsv") != index:
        raise ValueError("The APK resource index differs from the prepared export index")
    expected = {"assets/web/" + name for name in indexed["entries"]}
    actual = {info.filename for info in infos if info.filename.startswith("assets/web/") and not info.is_dir()}
    if expected != actual:
        raise ValueError("APK web resource names/count differ from the indexed export")
    for name, entry in indexed["entries"].items():
        info = archive.getinfo("assets/web/" + name)
        if info.file_size != entry["bytes"] or sha256(archive.read(info)) != entry["sha256"]:
            raise ValueError(f"APK web resource bytes/hash differ: {name}")
    abis = set()
    libraries = []
    for info in infos:
        if info.filename.startswith("lib/") and not info.is_dir():
            parts = info.filename.split("/")
            if len(parts) != 3 or not parts[2].endswith(".so"):
                raise ValueError("APK contains an unexpected native-library path")
            abis.add(parts[1])
        if not info.filename.endswith(".so") or info.is_dir():
            continue
        parts = info.filename.split("/")
        if len(parts) != 3 or parts[:2] != ["lib", abi] or info.file_size > MAX_LIBRARY_BYTES:
            raise ValueError("APK contains a missing/unexpected ABI or native-library path")
        data = archive.read(info)
        native = inspect_elf(data, abi)
        offset = native_zip_offset(archive, info)
        if info.compress_type == zipfile.ZIP_STORED and offset % 16384:
            raise ValueError("An uncompressed native library is not ZIP aligned to 16 KB")
        native.update({"path": info.filename, "sha256": sha256(data), "bytes": len(data),
                       "zip_data_offset": offset, "zip_compression": info.compress_type,
                       "zip_16kb_aligned": offset % 16384 == 0 if info.compress_type == zipfile.ZIP_STORED else None})
        libraries.append(native)
    if abis != {abi} or not libraries:
        raise ValueError("APK must contain exactly the requested ABI and at least one native library")
    return {"resource_count": len(expected), "resource_bytes": indexed["total_bytes"],
            "resource_snapshot": indexed["snapshot"], "abis": sorted(abis), "libraries": libraries}


def parse_badging(text, label, abi):
    package = re.search(r"^package: name='([^']+)' versionCode='([^']+)' versionName='([^']+)'", text, re.M)
    minimum = re.search(r"^(?:minSdkVersion|sdkVersion):'(\d+)'", text, re.M)
    target = re.search(r"^targetSdkVersion:'(\d+)'", text, re.M)
    display = re.search(r"^application-label:'([^']*)'", text, re.M)
    native = re.search(r"^native-code:(.*)$", text, re.M)
    permissions = sorted(set(re.findall(r"^uses-permission(?:-sdk-\d+)?: name='([^']+)'", text, re.M)))
    if (not all((package, minimum, target, display, native))
            or package.groups() != (PREVIEW_ID, "100001", "0.1.0")
            or minimum.group(1) != "24" or target.group(1) != "36"
            or display.group(1) != label or re.findall(r"'([^']+)'", native.group(1)) != [abi]
            or permissions not in (["android.permission.INTERNET"],
                                   ["android.permission.INTERNET",
                                    PREVIEW_ID + ".DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION"])):
        raise ValueError("APK manifest does not match the scoped preview identity/version/SDK/label/ABI/permissions")
    return {"application_id": package.group(1), "version_code": 100001, "version_name": "0.1.0",
            "min_sdk": 24, "target_sdk": 36, "label": label, "permissions": permissions}


def run_tool(command):
    result = subprocess.run([str(part) for part in command], capture_output=True, text=True, timeout=90)
    return {"command": [str(part) for part in command], "exit_code": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}


def inspect_apk(apk, abi, web_dir, index_path, product_path, aapt2, zipalign, apksigner=None, require_signature=False):
    index = index_path.read_bytes()
    if build_index(web_dir) != index:
        raise ValueError("Prepared index does not match the current static export")
    product = json.loads(product_path.read_text())
    if product.get("application_id") != "io.github.zocs.inoutnext" or product.get("version") != "0.1.0":
        raise ValueError("Product identity/version differs from this preview profile")
    if apk.stat().st_size > MAX_APK_BYTES:
        raise ValueError("APK exceeds inspection size bounds")
    with zipfile.ZipFile(apk) as archive:
        archive_result = inspect_archive(archive, abi, index)
    badging = run_tool([aapt2, "dump", "badging", apk])
    if badging["exit_code"]:
        raise ValueError("aapt2 could not inspect the actual APK manifest: " + badging["stderr"])
    manifest = parse_badging(badging["stdout"], product["name"], abi)
    alignment = run_tool([zipalign, "-c", "-P", "16", "-v", "4", apk])
    if alignment["exit_code"]:
        raise ValueError("Android zipalign rejected the APK alignment: " + alignment["stderr"])
    signature = {"status": "not_checked", "verified": False}
    if require_signature and not apksigner:
        raise ValueError("--require-signature requires an apksigner executable")
    if apksigner:
        verification = run_tool([apksigner, "verify", "--verbose", "--print-certs", apk])
        signature = {"status": "verified" if verification["exit_code"] == 0 else "unverified_or_unsigned",
                     "verified": verification["exit_code"] == 0, "tool": verification}
        if require_signature and not signature["verified"]:
            raise ValueError("APK signature verification failed")
    return {"passed": True, "apk": str(apk.resolve()), "sha256": sha256(apk.read_bytes()),
            "manifest": manifest, **archive_result, "tools": [badging, alignment],
            "signature": signature, "installable_by_signature": signature["verified"],
            "target_device_verified": False, "page_size_runtime_verified": False}


def main():
    project = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("apk", type=Path)
    parser.add_argument("--abi", choices=ABI_TARGETS, default="aarch64")
    parser.add_argument("--web-dir", type=Path, default=project / "apps/web/out")
    parser.add_argument("--index", type=Path, default=project / ".local/android-assets/web-index.tsv")
    parser.add_argument("--product", type=Path, default=project / "product.json")
    sdk = Path(os.environ.get("ANDROID_HOME", os.environ.get("ANDROID_SDK_ROOT", "/opt/android-sdk")))
    parser.add_argument("--aapt2", type=Path, default=sdk / "build-tools/36.0.0/aapt2")
    parser.add_argument("--zipalign", type=Path, default=sdk / "build-tools/36.0.0/zipalign")
    parser.add_argument("--apksigner", type=Path)
    parser.add_argument("--require-signature", action="store_true")
    parser.add_argument("--report", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        result = inspect_apk(arguments.apk, ABI_TARGETS[arguments.abi], arguments.web_dir, arguments.index,
                             arguments.product, arguments.aapt2, arguments.zipalign, arguments.apksigner,
                             arguments.require_signature)
    except (OSError, ValueError, zipfile.BadZipFile, struct.error, subprocess.SubprocessError) as error:
        result = {"passed": False, "error": str(error), "target_device_verified": False}
    arguments.report.parent.mkdir(parents=True, exist_ok=True)
    arguments.report.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"passed": result["passed"], "report": str(arguments.report),
                      "error": result.get("error")}, indent=2))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
