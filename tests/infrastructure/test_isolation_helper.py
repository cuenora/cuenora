"""Test the interactive account isolation checker without credentials or network."""
import contextlib
import importlib.util
import io
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
spec = importlib.util.spec_from_file_location('isolation_helper', ROOT / 'scripts/check_account_isolation.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)


class FakeApi:
    def __init__(self, leak=False, allow_foreign=False):
        self.accounts = {}
        self.leak = leak
        self.allow_foreign = allow_foreign
        self.logged_out = []
        self.logout_calls = []
        self.requests = []

    def post(self, path, body, bearer=None):
        if path.endswith('/login'):
            return 200, {'access_token': 'token-' + body['email'], 'refresh_token': 'refresh-' + body['email']}
        if path.endswith('/logout'):
            self.logout_calls.append((body, bearer))
            if not isinstance(body.get('refresh_token'), str):
                return 400, {'code': 'invalid_request'}
            self.logged_out.append(body['refresh_token'])
            return 204, None
        raise AssertionError('Unexpected POST')

    def request(self, method, path, body=None, bearer=None, *, idempotency_key=None):
        self.requests.append((method, path, body, bearer, idempotency_key))
        if path == '/v1/accounts' and method == 'POST':
            assert idempotency_key
            assert 'opening_balance' not in body
            account = dict(body, id=f'00000000-0000-0000-0000-{len(self.accounts)+1:012d}',
                           status='active', balance='0.00')
            self.accounts[account['id']] = (bearer, account)
            return 201, {k: v for k, v in account.items() if k != 'balance'}
        if path.startswith('/v1/accounts?'):
            if 'cursor=' in path:
                return 400, {'code': 'invalid_request'}
            return 200, {'accounts': [dict(a) for owner, a in self.accounts.values()
                                     if owner == bearer or self.leak], 'next_cursor': None}
        account_id = path.split('/')[3]
        owner, account = self.accounts[account_id]
        if path.endswith('/savings-terms'):
            return (200, {'versions': [], 'next_cursor': None}) if owner == bearer else (404, {'code': 'not_found'})
        if method == 'PATCH':
            if owner != bearer and not self.allow_foreign:
                return 404, {'code': 'not_found'}
            account.update(body)
            return 200, {k: v for k, v in account.items() if k != 'balance'}
        raise AssertionError('Unexpected request')


class IsolationHelperTest(unittest.TestCase):
    def test_owned_controls_and_cross_user_rejections_do_not_disclose_secrets(self):
        api = FakeApi()
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            helper.flow(api, [('a@example.test', 'secret-a'), ('b@example.test', 'secret-b')])
        self.assertEqual(len(api.accounts), 2)
        self.assertEqual(len(api.logged_out), 2)
        self.assertTrue(all(a['balance'] == '0.00' for _, a in api.accounts.values()))
        self.assertIn('Account isolation checks passed', output.getvalue())
        for secret in ['a@example.test', 'b@example.test', 'secret-a', 'secret-b', 'token-', '00000000-']:
            self.assertNotIn(secret, output.getvalue())

    def test_logout_uses_each_retained_refresh_without_access_bearer(self):
        api = FakeApi()
        with contextlib.redirect_stdout(io.StringIO()):
            helper.flow(api, [('a@example.test', 'secret-a'), ('b@example.test', 'secret-b')])
        self.assertEqual(api.logout_calls, [({'refresh_token': 'refresh-a@example.test'}, None),
                                           ({'refresh_token': 'refresh-b@example.test'}, None)])

    def test_list_leak_is_not_reported_as_success_and_sessions_are_closed(self):
        output = io.StringIO()
        api = FakeApi(leak=True)
        with contextlib.redirect_stdout(output), self.assertRaises(helper.VerificationError):
            helper.flow(api, [('a@example.test', 'secret-a'), ('b@example.test', 'secret-b')])
        self.assertNotIn('checks passed', output.getvalue())
        self.assertEqual(len(api.logged_out), 2)

    def test_foreign_write_is_not_reported_as_success_or_retried(self):
        output = io.StringIO()
        api = FakeApi(allow_foreign=True)
        with contextlib.redirect_stdout(output), self.assertRaises(helper.VerificationError):
            helper.flow(api, [('a@example.test', 'secret-a'), ('b@example.test', 'secret-b')])
        self.assertNotIn('checks passed', output.getvalue())
        foreign = [r for r in api.requests if r[0] == 'PATCH' and r[3] != api.accounts[r[1].split('/')[3]][0]]
        self.assertEqual(len(foreign), 1)
        self.assertEqual(len(api.logged_out), 2)


if __name__ == '__main__':
    unittest.main()
