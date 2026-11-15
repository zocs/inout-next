"""Inspect real ZIP bytes, indexed resources and ELF program headers."""

import hashlib
import io
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
import warnings
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
import inspect_android as inspect
from prepare_android_assets import build_index


def elf(align=16384, machine=183, offset=0, virtual=0, filesz=128, memsz=128):
    ident = b"\x7fELF\x02\x01\x01" + bytes(9)
    header = struct.pack("<HHIQQQIHHHHHH", 3, machine, 1, 0, 64, 0, 0, 64, 56, 1, 0, 0, 0)
    program = struct.pack("<IIQQQQQQ", 1, 5, offset, virtual, 0, filesz, memsz, align)
    return (ident + header + program).ljust(128, b"\x00")


class AndroidElfTests(unittest.TestCase):
    def test_load_alignment_and_machine_are_measured_from_bytes(self):
        result = inspect.inspect_elf(elf(), "arm64-v8a")
        self.assertEqual(result["machine"], 183)
        self.assertEqual(result["load_segments"][0]["alignment"], 16384)

    def test_small_alignment_or_invalid_mapping_fails(self):
        for data in [elf(4096), elf(offset=1), elf(align=24000), elf(filesz=129), elf(memsz=127)]:
            with self.subTest(data=data), self.assertRaises(ValueError):
                inspect.inspect_elf(data, "arm64-v8a")

    def test_truncated_headers_wrong_machine_and_non_elf_fail(self):
        for data in [b"not ELF", elf()[:100], elf(machine=62), elf()[:63]]:
            with self.subTest(data=data), self.assertRaises(ValueError):
                inspect.inspect_elf(data, "arm64-v8a")


class AndroidArchiveTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.web = self.root / "web"
        (self.web / "_next").mkdir(parents=True)
        (self.web / "index.html").write_bytes(b"<title>Raw preview</title>")
        (self.web / "_next/app.js").write_bytes(b"console.log('raw')")
        self.index = build_index(self.web)

    def archive(self, extras=(), changed=None, abi="arm64-v8a", native=None, aligned=True, omitted=()):
        contents = {"AndroidManifest.xml": b"binary manifest fixture",
                    "assets/inout-web-index.tsv": self.index,
                    "assets/web/index.html": (self.web / "index.html").read_bytes(),
                    "assets/web/_next/app.js": (self.web / "_next/app.js").read_bytes(),
                    f"lib/{abi}/libapp.so": elf() if native is None else native}
        if changed:
            contents.update(changed)
        for name in omitted:
            contents.pop(name)
        buffer = io.BytesIO()
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            with zipfile.ZipFile(buffer, "w") as archive:
                for name, data in list(contents.items()) + list(extras):
                    info = zipfile.ZipInfo(name)
                    if name.endswith(".so") and aligned:
                        padding = -(buffer.tell() + 30 + len(name.encode())) % 16384
                        if 0 < padding < 4:
                            padding += 16384
                        if padding:
                            info.extra = struct.pack("<HH", 0xcafe, padding - 4) + bytes(padding - 4)
                    archive.writestr(info, data)
        return buffer.getvalue()

    def check(self, data):
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            return inspect.inspect_archive(archive, "arm64-v8a", self.index)

    def test_complete_raw_resource_snapshot_and_abi_are_verified(self):
        result = self.check(self.archive())
        self.assertEqual(result["resource_count"], 2)
        self.assertEqual(result["abis"], ["arm64-v8a"])
        self.assertEqual(result["libraries"][0]["sha256"], hashlib.sha256(elf()).hexdigest())

    def test_changed_missing_extra_or_duplicate_resources_fail(self):
        fixtures = [self.archive(changed={"assets/web/index.html": b"different"}),
                    self.archive(extras=[("assets/web/extra.js", b"extra")]),
                    self.archive(extras=[("assets/web/index.html", b"duplicate")]),
                    self.archive(changed={"assets/inout-web-index.tsv": b"wrong index"})]
        for data in fixtures:
            with self.subTest(data=data), self.assertRaises(ValueError):
                self.check(data)

    def test_archive_traversal_absolute_backslash_and_duplicate_nonresources_fail(self):
        for member in ["../escape", "/absolute", "x/../escape", "x\\escape", "assets//x", "AndroidManifest.xml"]:
            with self.subTest(member=member), self.assertRaises(ValueError):
                self.check(self.archive(extras=[(member, b"bad")]))

    def test_extra_abi_missing_native_or_malformed_second_library_fails(self):
        fixtures = [self.archive(abi="x86_64"),
                    self.archive(extras=[("lib/x86_64/libsecond.so", elf(machine=62))]),
                    self.archive(extras=[("lib/arm64-v8a/libsecond.so", b"not ELF")]),
                    self.archive(native=elf(4096))]
        for data in fixtures:
            with self.subTest(data=data), self.assertRaises(ValueError):
                self.check(data)

    def test_unaligned_uncompressed_native_zip_data_fails(self):
        with self.assertRaises(ValueError):
            self.check(self.archive(aligned=False))

    def test_missing_resource_or_native_library_fails(self):
        for name in ["assets/web/index.html", "lib/arm64-v8a/libapp.so"]:
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.check(self.archive(omitted=[name]))

    def fake_tool(self, name, stdout="", exit_code=0):
        path = self.root / name
        path.write_text("#!" + sys.executable + "\nimport sys\n"
                        f"print({stdout!r})\nraise SystemExit({exit_code})\n")
        path.chmod(0o755)
        return path

    def test_unsigned_inspection_is_explicit_and_signature_gate_rejects_it(self):
        apk = self.root / "candidate.apk"
        apk.write_bytes(self.archive())
        index = self.root / "index.tsv"
        index.write_bytes(self.index)
        product = self.root / "product.json"
        product.write_text(json.dumps({"application_id": "io.github.zocs.inoutnext", "version": "0.1.0", "name": "inout"}))
        aapt = self.fake_tool("aapt", AndroidManifestTests().badging())
        align = self.fake_tool("zipalign")
        signer = self.fake_tool("apksigner", "No verified signer", 1)
        result = inspect.inspect_apk(apk, "arm64-v8a", self.web, index, product, aapt, align, signer)
        self.assertTrue(result["passed"])
        self.assertFalse(result["signature"]["verified"])
        self.assertFalse(result["installable_by_signature"])
        with self.assertRaises(ValueError):
            inspect.inspect_apk(apk, "arm64-v8a", self.web, index, product, aapt, align, signer, True)

    def test_actual_tool_failure_cannot_be_reported_as_inspection_success(self):
        apk = self.root / "candidate.apk"
        apk.write_bytes(self.archive())
        index = self.root / "index.tsv"
        index.write_bytes(self.index)
        product = self.root / "product.json"
        product.write_text(json.dumps({"application_id": "io.github.zocs.inoutnext", "version": "0.1.0", "name": "inout"}))
        aapt = self.fake_tool("aapt", "manifest unavailable", 2)
        align = self.fake_tool("zipalign")
        with self.assertRaises(ValueError):
            inspect.inspect_apk(apk, "arm64-v8a", self.web, index, product, aapt, align)

    def test_index_checksum_count_and_file_directory_conflict_fail(self):
        bad = self.index.replace(b"INOUTWEB1", b"OTHERWEB1")
        with self.assertRaises(ValueError):
            inspect.parse_index(bad)
        header, table = self.index.split(b"\n", 1)
        fields = header.split(b"\t")
        fields[2] = b"3"
        with self.assertRaises(ValueError):
            inspect.parse_index(b"\t".join(fields) + b"\n" + table)
        table = b"0" * 64 + b"\t0\t_next\n" + b"0" * 64 + b"\t0\t_next/app.js\n" + b"0" * 64 + b"\t0\tindex.html\n"
        header = f"INOUTWEB1\t{hashlib.sha256(table).hexdigest()}\t3\t0\n".encode()
        with self.assertRaises(ValueError):
            inspect.parse_index(header + table)


class AndroidManifestTests(unittest.TestCase):
    def badging(self, identity="io.github.zocs.inoutnext.preview", target=36, permission="android.permission.INTERNET"):
        return (f"package: name='{identity}' versionCode='100001' versionName='0.1.0'\n"
                f"sdkVersion:'24'\ntargetSdkVersion:'{target}'\napplication-label:'inout'\n"
                f"uses-permission: name='{permission}'\nnative-code: 'arm64-v8a'\n")

    def test_production_identity_wrong_target_and_unexpected_permissions_fail(self):
        for text in [self.badging(identity="io.github.zocs.inoutnext"), self.badging(target=37),
                     self.badging(permission="android.permission.MANAGE_EXTERNAL_STORAGE")]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                inspect.parse_badging(text, "inout", "arm64-v8a")

    def test_unsigned_candidate_has_explicit_noninstallable_status(self):
        result = inspect.parse_badging(self.badging(), "inout", "arm64-v8a")
        self.assertEqual(result["application_id"], "io.github.zocs.inoutnext.preview")

    def test_pinned_aapt2_min_sdk_spelling_and_androidx_internal_permission(self):
        text = self.badging().replace("sdkVersion:'24'", "minSdkVersion:'24'")
        text += "uses-permission: name='io.github.zocs.inoutnext.preview.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION'\n"
        result = inspect.parse_badging(text, "inout", "arm64-v8a")
        self.assertEqual(result["min_sdk"], 24)
        self.assertEqual(len(result["permissions"]), 2)
        for permission in ["android.permission.READ_EXTERNAL_STORAGE",
                           "other.app.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION"]:
            with self.subTest(permission=permission), self.assertRaises(ValueError):
                inspect.parse_badging(text + f"uses-permission: name='{permission}'\n", "inout", "arm64-v8a")


if __name__ == "__main__":
    unittest.main()
