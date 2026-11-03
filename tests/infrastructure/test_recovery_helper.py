"""Recovery terminal workflow without a real provider or credentials."""
import contextlib
import importlib.util
import io
from pathlib import Path
import sys
import unittest
from unittest.mock import patch
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
spec = importlib.util.spec_from_file_location("recovery_helper", ROOT / "scripts/recover_password.py")
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)

class FakeApi:
    def __init__(self, reject=False):
        self.calls = []
        self.reject = reject
    def post(self, path, body, bearer=None):
        self.calls.append((path, body, bearer))
        n = len(self.calls)
        if path.endswith("register"): return 202, {}
        if path.endswith("request"): return 202, {"status": "accepted"}
        if path.endswith("confirm"):
            if self.reject or n == 5: return 400, {"code": "invalid_challenge"}
            return 204, None
        if path.endswith("refresh"): return 401, {"code": "invalid_refresh_token"}
        if path.endswith("login"):
            if n == 6: return 401, {"code": "invalid_credentials"}
            return 200, {"access_token": "secret-access", "refresh_token": "secret-refresh"}
        raise AssertionError("Unexpected operation")

class RecoveryHelperTest(unittest.TestCase):
    def test_flow_checks_replay_old_password_revocation_and_new_login_without_leaks(self):
        api=FakeApi(); output=io.StringIO(); token="pr1_"+"a"*64
        prompts=iter([token,"New-private!password-456","New-private!password-456",""])
        with contextlib.redirect_stdout(output):
            helper.flow(api,"synthetic@example.test","Old-private!password-123",True,lambda _:next(prompts))
        self.assertEqual(len(api.calls),8)
        self.assertEqual(api.calls[3][1],{"token":token,"new_password":"New-private!password-456"})
        self.assertTrue(all(bearer is None for _,_,bearer in api.calls))
        self.assertIn("Password reset (204)",output.getvalue())
        for secret in [token,"synthetic@example.test","Old-private!password-123","New-private!password-456","secret-access","secret-refresh"]:
            self.assertNotIn(secret,output.getvalue())
    def test_rejected_reset_never_reports_success(self):
        prompts=iter(["pr1_"+"b"*64,"New-private!password-456","New-private!password-456"])
        output=io.StringIO()
        with contextlib.redirect_stdout(output),self.assertRaises(helper.VerificationError):
            helper.flow(FakeApi(reject=True),"synthetic@example.test","old-password",True,lambda _:next(prompts))
        self.assertNotIn("Password reset (204)",output.getvalue())
    def test_mismatched_new_password_never_submits_confirmation(self):
        api=FakeApi(); prompts=iter(["pr1_"+"b"*64,"new-password","different-password"])
        with contextlib.redirect_stdout(io.StringIO()),self.assertRaises(helper.VerificationError):
            helper.flow(api,"synthetic@example.test","old-password",True,lambda _:next(prompts))
        self.assertFalse(any(p.endswith("confirm") for p,_,_ in api.calls))
    def test_main_rejects_noninteractive_input_before_reading_secrets(self):
        with patch.object(helper.sys,"argv",["recover_password.py"]),patch.object(helper.sys.stdin,"isatty",return_value=False),patch("builtins.input") as read,contextlib.redirect_stderr(io.StringIO()),self.assertRaises(SystemExit):
            helper.main()
        read.assert_not_called()
    def test_hidden_input_failure_never_falls_back_to_plaintext(self):
        import getpass
        with patch.object(helper.sys,"argv",["recover_password.py"]),patch.object(helper.sys.stdin,"isatty",return_value=True),patch("builtins.input",return_value="synthetic@example.test"),patch.object(helper.getpass,"getpass",side_effect=getpass.GetPassWarning),contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(helper.main(),1)
