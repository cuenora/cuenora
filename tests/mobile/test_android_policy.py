"""Static regression checks for the native Android distribution/privacy policy."""
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
APP = ROOT / "mobile/androidApp"
ANDROID = "{http://schemas.android.com/apk/res/android}"


class AndroidPolicyTest(unittest.TestCase):
    def test_supported_sdk_and_navigation(self):
        build = (APP / "build.gradle.kts").read_text()
        self.assertRegex(build, r"minSdk\s*=\s*24\b")
        self.assertRegex(build, r"targetSdk\s*=\s*37\b")
        self.assertIn('androidx.navigation:navigation-compose:2.10.2', build)

    def test_manifest_disables_legacy_backup_and_links_modern_rules(self):
        app = ET.parse(APP / "src/main/AndroidManifest.xml").getroot().find("application")
        self.assertEqual("false", app.get(ANDROID + "allowBackup"))
        self.assertEqual("false", app.get(ANDROID + "fullBackupContent"))
        self.assertEqual("@xml/data_extraction_rules", app.get(ANDROID + "dataExtractionRules"))

    def test_cloud_and_device_transfer_exclude_all_storage_domains(self):
        rules = ET.parse(APP / "src/main/res/xml/data_extraction_rules.xml").getroot()
        self.assertEqual("data-extraction-rules", rules.tag)
        domains = {"root", "file", "database", "sharedpref", "external",
                   "device_root", "device_file", "device_database", "device_sharedpref"}
        self.assertEqual({"cloud-backup", "device-transfer"}, {section.tag for section in rules})
        for section in rules:
            self.assertEqual(domains, {entry.get("domain") for entry in section})
            self.assertEqual(len(domains), len(section))
            for entry in section:
                self.assertEqual("exclude", entry.tag)
                self.assertEqual(".", entry.get("path"))


if __name__ == "__main__":
    unittest.main()
