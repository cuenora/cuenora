"""Keep the system splash artwork separate from the unchanged launcher icon."""
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
RES = ROOT / "mobile/androidApp/src/main/res"
ANDROID = "{http://schemas.android.com/apk/res/android}"


class AndroidSplashTest(unittest.TestCase):
    def test_system_splash_uses_dedicated_safe_size_artwork(self):
        splash = ET.parse(RES / "drawable/ic_splash.xml").getroot()
        self.assertEqual("animated-vector", splash.tag)
        vector = splash.find("{http://schemas.android.com/aapt}attr/vector")
        self.assertEqual("432dp", vector.get(ANDROID + "width"))
        self.assertEqual("432dp", vector.get(ANDROID + "height"))
        self.assertEqual("432", vector.get(ANDROID + "viewportWidth"))
        self.assertEqual("432", vector.get(ANDROID + "viewportHeight"))
        paths = vector.findall(".//path")
        self.assertEqual(3, len(paths))
        self.assertTrue(all(path.get(ANDROID + "fillColor") == "#2466D1" for path in paths))
        self.assertTrue(all(path.get(ANDROID + "name") for path in paths))
        self.assertEqual(3, len(splash.findall("target")))
        mark = vector.find("group")
        self.assertEqual("0.9", mark.get(ANDROID + "scaleX"))
        self.assertEqual("0.9", mark.get(ANDROID + "scaleY"))
        self.assertEqual("72", mark.get(ANDROID + "translateX"))
        self.assertEqual("72", mark.get(ANDROID + "translateY"))
        launcher = ET.parse(RES / "drawable/ic_launcher.xml").getroot()
        self.assertEqual("48dp", launcher.get(ANDROID + "width"))
        self.assertEqual("48dp", launcher.get(ANDROID + "height"))
        launcher_groups = launcher.findall("group")
        splash_groups = vector.findall("group/group")
        self.assertEqual([group.get(ANDROID + "translateX") for group in launcher_groups],
                         [group.get(ANDROID + "translateX") for group in splash_groups])
        self.assertEqual([group.get(ANDROID + "translateY") for group in launcher_groups],
                         [group.get(ANDROID + "translateY") for group in splash_groups])
        self.assertEqual([group.find("path").get(ANDROID + "pathData") for group in launcher_groups],
                         [group.find("path").get(ANDROID + "pathData") for group in splash_groups])


    def test_reveal_finishes_at_declared_duration_with_complete_static_fallback(self):
        splash = ET.parse(RES / "drawable/ic_splash.xml").getroot()
        paths = splash.findall(".//path")
        self.assertTrue(all(path.get(ANDROID + "fillAlpha", "1") == "1" for path in paths))
        ends = []
        for target in splash.findall("target"):
            animator = ET.parse(RES / (target.get(ANDROID + "animation")[1:] + ".xml")).getroot()
            self.assertEqual("fillAlpha", animator.get(ANDROID + "propertyName"))
            self.assertEqual("0", animator.get(ANDROID + "valueFrom"))
            self.assertEqual("1", animator.get(ANDROID + "valueTo"))
            ends.append(int(animator.get(ANDROID + "startOffset", "0")) +
                        int(animator.get(ANDROID + "duration")))
        self.assertEqual([200, 280, 360], ends)
        theme = ET.parse(RES / "values-v31/styles.xml").getroot()
        duration = theme.find("style[@name='Theme.Cuenora.Starting']/item[@name='android:windowSplashScreenAnimationDuration']")
        self.assertEqual(max(ends), int(duration.text))


if __name__ == "__main__":
    unittest.main()
