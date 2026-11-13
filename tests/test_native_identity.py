"""NSIS inspection parser regressions; fixtures are not Windows bundle evidence."""

from pathlib import Path
import sys
import tempfile
import unittest


sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from verify_native_identity import inspect_nsis, sha256


class NsisInspectionTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.resource = self.root / "index.html"
        self.resource.write_text("existing exported page", encoding="utf-8")
        self.web = {"index.html": sha256(self.resource)}
        self.text = r'''
!define PRODUCTNAME "Inout Next"
!define BUNDLEID "io.github.zocs.inoutnext"
!define MANUFACTURER "github"
!define MAINBINARYNAME "share-desktop"
!define INSTALLMODE "currentUser"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCTNAME}"
!define MANUKEY "Software\${MANUFACTURER}"
!define MANUPRODUCTKEY "${MANUKEY}\${PRODUCTNAME}"
!define PLACEHOLDER_INSTALL_DIR "placeholder\${PRODUCTNAME}"
!define STARTMENUFOLDER ""
StrCpy $INSTDIR "$LOCALAPPDATA\${PRODUCTNAME}"
ReadRegStr $R0 SHCTX "${UNINSTKEY}" "UninstallString"
WriteRegStr SHCTX "${MANUPRODUCTKEY}" "" $INSTDIR
DeleteRegKey SHCTX "${UNINSTKEY}"
'''
        self.text += f'File /a "/oname=web\\index.html" "{self.resource}"\n'
        self.template = self.root / "installer.nsi"

    def inspect(self, text):
        self.template.write_text(text, encoding="utf-8")
        return inspect_nsis(self.template, self.web)

    def test_resolves_install_and_registry_keys_from_inspected_template(self):
        identity = self.inspect(self.text)
        self.assertEqual(identity["PRODUCTNAME"], "Inout Next")
        self.assertEqual(identity["MANUPRODUCTKEY"], r"Software\github\Inout Next")
        self.assertEqual(identity["UNINSTKEY"],
                         r"Software\Microsoft\Windows\CurrentVersion\Uninstall\Inout Next")
        self.assertEqual(identity["install_directories"], [r"$LOCALAPPDATA\Inout Next"])
        self.assertEqual(identity["web_files_verified"], 1)
        renamed = self.inspect(self.text.replace('PRODUCTNAME "Inout Next"', 'PRODUCTNAME "Harbor"'))
        self.assertNotEqual(renamed["MANUPRODUCTKEY"], identity["MANUPRODUCTKEY"])
        self.assertNotEqual(renamed["UNINSTKEY"], identity["UNINSTKEY"])
        self.assertNotEqual(renamed["install_directories"], identity["install_directories"])

    def test_rejects_template_without_uninstall_registry_operation(self):
        with self.assertRaises(ValueError):
            self.inspect(self.text.replace('DeleteRegKey SHCTX "${UNINSTKEY}"', ""))

    def test_rejects_missing_install_directory_operation(self):
        with self.assertRaises(ValueError):
            self.inspect(self.text.replace('StrCpy $INSTDIR "$LOCALAPPDATA\\${PRODUCTNAME}"', ""))

    def test_rejects_wrong_packaged_web_destination(self):
        with self.assertRaises(ValueError):
            self.inspect(self.text.replace("web\\index.html", "other\\index.html"))


if __name__ == "__main__":
    unittest.main()
