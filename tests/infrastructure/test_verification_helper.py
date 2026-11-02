"""Secret-safe terminal verification workflow, without sending real mail."""
import contextlib
import importlib.util
import io
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("verification_helper", ROOT / "scripts" / "verify_email.py")
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)

class VerificationHelperTest(unittest.TestCase):
    def test_terminal_flow_uses_body_tokens_and_rejects_replay_without_printing_credentials(self):
        token = "ev1_" + "a" * 64
        class Api:
            def __init__(self): self.calls = []
            def post(self, path, body, bearer=None):
                self.calls.append((path, body, bearer))
                if path.endswith("register"): return 202, {"status": "pending"}
                if path.endswith("login"): return 200, {"access_token": "access-secret", "refresh_token": "refresh-secret"}
                if path.endswith("request"): return 202, {"status": "accepted"}
                if path.endswith("refresh"): return 200, {"access_token": "new-access-secret"}
                return (204, None) if len(self.calls) == 5 else (400, {"code": "invalid_challenge"})
        api = Api()
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            helper.flow(api, "person@example.test", "private-password", True, lambda _: token)
        self.assertEqual(len(api.calls), 6)
        self.assertEqual(api.calls[4][1], {"token": token})
        self.assertEqual(api.calls[4][2], "new-access-secret")
        self.assertTrue(all("?" not in call[0] and token not in call[0] for call in api.calls))
        for secret in [token, "person@example.test", "private-password", "access-secret", "refresh-secret"]:
            self.assertNotIn(secret, output.getvalue())
        self.assertIn("Email verified (204)", output.getvalue())

    def test_api_destinations_reject_credentials_remote_http_and_bad_ports(self):
        for url in ["http://example.test", "https://user:secret@example.test", "https://example.test/?x=secret", "https://example.test/#secret", "http://localhost:99999"]:
            with self.assertRaises(helper.VerificationError): helper.Api(url)
        for url in ["http://127.0.0.1:3000", "http://[::1]:3000", "https://api.example.test"]:
            helper.Api(url)

    def test_rejected_confirmation_never_reports_success(self):
        class Api:
            def post(self, path, body, bearer=None):
                if path.endswith("login"): return 200, {"access_token": "a", "refresh_token": "r"}
                if path.endswith("request"): return 202, {}
                if path.endswith("refresh"): return 200, {"access_token": "a"}
                return 400, {"code": "invalid_challenge"}
        output=io.StringIO()
        with contextlib.redirect_stdout(output), self.assertRaises(helper.VerificationError):
            helper.flow(Api(),"person@example.test","password",False,lambda _: "ev1_"+"a"*64)
        self.assertNotIn("Email verified (204)",output.getvalue())


class VerificationDiagnosticsTest(unittest.TestCase):
    def test_main_reports_safe_failure_reason_instead_of_generic_message(self):
        from unittest.mock import patch
        safe_reason = "Registration was not accepted; check password policy and server availability."
        error = io.StringIO()
        with patch.object(helper.sys, "argv", ["verify_email.py", "--local", "--register"]), \
             patch.object(helper.sys.stdin, "isatty", return_value=True), \
             patch("builtins.input", return_value="synthetic@example.test"), \
             patch.object(helper.getpass, "getpass", return_value="synthetic-password"), \
             patch.object(helper, "local_flow", side_effect=helper.VerificationError(safe_reason)), \
             contextlib.redirect_stderr(error):
            self.assertEqual(helper.main(), 1)
        self.assertIn(safe_reason, error.getvalue())
        self.assertNotIn("synthetic-password", error.getvalue())
        self.assertNotIn("synthetic@example.test", error.getvalue())

    def test_startup_failure_only_exposes_known_variable_names(self):
        from types import SimpleNamespace
        marker = "DO_NOT_PRINT_SECRET_OR_ADDRESS"
        process = SimpleNamespace(stderr=io.BytesIO(("Invalid CUENORA_RESEND_API_KEY/CUENORA_EMAIL_FROM " + marker).encode()))
        reason = helper.startup_failure(process)
        self.assertIn("CUENORA_RESEND_API_KEY", reason)
        self.assertIn("CUENORA_EMAIL_FROM", reason)
        self.assertNotIn(marker, reason)
        process = SimpleNamespace(stderr=io.BytesIO((marker + " CUENORA_UNKNOWN_SECRET").encode()))
        reason = helper.startup_failure(process)
        self.assertNotIn(marker, reason)
        self.assertNotIn("CUENORA_UNKNOWN_SECRET", reason)

if __name__ == "__main__": unittest.main()
