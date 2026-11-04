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


class ApiHttpTransportTest(unittest.TestCase):
    @staticmethod
    def response(status, body, content_type="application/json"):
        response = io.BytesIO(body)
        response.code = status
        response.headers = {"Content-Type": content_type}
        return response

    def test_requests_identify_helper_without_changing_json_or_bearer(self):
        from unittest.mock import Mock
        import json
        api = helper.Api("https://api.example.test")
        api.opener = Mock()
        api.opener.open.return_value = self.response(202, b'{"status":"pending"}')
        self.assertEqual(api.post("/v1/auth/register", {"password": "secret"}, "bearer-secret"),
                         (202, {"status": "pending"}))
        request = api.opener.open.call_args.args[0]
        self.assertEqual(request.get_header("User-agent"), "Cuenora-Identity-Helper/0.1")
        self.assertEqual(request.get_header("Content-type"), "application/json")
        self.assertEqual(request.get_header("Authorization"), "Bearer bearer-secret")
        self.assertEqual(json.loads(request.data), {"password": "secret"})
        self.assertEqual(request.get_method(), "POST")

    def test_non_json_errors_report_only_status_and_safe_content_type(self):
        from unittest.mock import Mock
        import urllib.error
        for content_type, expected in [("text/plain; charset=UTF-8", "text/plain"),
                                       ("text/html", "text/html"),
                                       ("application/x-DO_NOT_PRINT_SECRET", "other")]:
            with self.subTest(content_type=content_type):
                api = helper.Api("https://api.example.test")
                api.opener = Mock()
                api.opener.open.side_effect = urllib.error.HTTPError(
                    "https://api.example.test/private-url", 403, "DO_NOT_PRINT_SECRET",
                    {"Content-Type": content_type}, io.BytesIO(b"DO_NOT_PRINT_SECRET"))
                with self.assertRaises(helper.VerificationError) as error:
                    api.post("/v1/auth/login", {"password": "DO_NOT_PRINT_SECRET"})
                self.assertEqual(str(error.exception),
                                 f"Unexpected API response format (HTTP 403; content type: {expected}).")
                self.assertNotIn("DO_NOT_PRINT_SECRET", str(error.exception))
                self.assertNotIn("private-url", str(error.exception))

    def test_post_null_and_bodyless_get_preserve_distinct_wire_payloads(self):
        from unittest.mock import Mock
        api = helper.Api("https://api.example.test")
        api.opener = Mock()
        api.opener.open.side_effect = [self.response(204, b""), self.response(200, b"{}")]
        api.post("/v1/auth/logout", None, "synthetic")
        self.assertEqual(api.opener.open.call_args.args[0].data, b"null")
        api.request("GET", "/v1/accounts", bearer="synthetic")
        self.assertIsNone(api.opener.open.call_args.args[0].data)
        self.assertEqual(api.opener.open.call_args.args[0].get_method(), "GET")

    def test_existing_json_errors_and_empty_success_are_preserved(self):
        from unittest.mock import Mock
        import urllib.error
        api = helper.Api("https://api.example.test")
        api.opener = Mock()
        api.opener.open.side_effect = urllib.error.HTTPError(
            "https://api.example.test", 400, "Bad request", {},
            io.BytesIO(b'{"code":"invalid_challenge"}'))
        self.assertEqual(api.post("/v1/auth/password-recovery/confirm", {}),
                         (400, {"code": "invalid_challenge"}))
        api.opener.open.side_effect = None
        api.opener.open.return_value = self.response(204, b"")
        self.assertEqual(api.post("/v1/auth/email-verification/confirm", {}), (204, None))

if __name__ == "__main__": unittest.main()
