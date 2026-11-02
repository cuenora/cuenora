"""Explicit Docker bootstrap smoke test; never requests mail or contacts Resend."""
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("verification_helper_local", ROOT / "scripts" / "verify_email.py")
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)

@unittest.skipUnless(os.environ.get("CUENORA_TEST_LOCAL_HELPER") == "1", "explicit disposable Docker test only")
class LocalVerificationHelperTest(unittest.TestCase):
    def test_bootstrap_register_login_and_cleanup_without_requesting_mail(self):
        calls = []
        def no_mail_flow(api, email, password, register):
            self.assertTrue(register)
            self.assertTrue(api.base.startswith("http://127.0.0.1:"))
            self.assertEqual(api.post("/v1/auth/register", {"email": email, "password": password})[0], 202)
            status, pair = api.post("/v1/auth/login", {"email": email, "password": password})
            self.assertEqual(status, 200)
            self.assertTrue(pair["access_token"].startswith("ca1_"))
            calls.append(True)
        output = io.StringIO()
        with patch.dict(os.environ, {"CUENORA_RESEND_API_KEY": "re_synthetic_never_sent", "CUENORA_EMAIL_FROM": "verify@example.test"}), \
             patch.object(helper, "flow", no_mail_flow), contextlib.redirect_stdout(output):
            helper.local_flow("smoke@example.test", "violet meadows beside winter 水", True)
        self.assertEqual(calls, [True])
        self.assertIn("test account was not retained", output.getvalue())
        self.assertNotIn("re_synthetic_never_sent", output.getvalue())

if __name__ == "__main__": unittest.main()
