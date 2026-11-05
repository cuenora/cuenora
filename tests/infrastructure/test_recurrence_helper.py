"""Offline models for the interactive staging recurrence helper."""
import contextlib
import copy
import importlib.util
import io
from pathlib import Path
import sys
import unittest
from unittest.mock import patch
from urllib.parse import urlsplit, parse_qs
import uuid

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
spec = importlib.util.spec_from_file_location('recurrence_helper', ROOT / 'scripts/check_recurrence_generation.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)


class FakeApi:
    def __init__(self, mode='normal'):
        self.mode = mode
        self.accounts, self.events = {}, []
        if mode == 'history_change':
            self.events = [{'id': str(uuid.uuid4()), 'description': 'Existing synthetic record'}]
        self.rule = None
        self.pending = '0.00'
        self.generated = False
        self.rotation = 0
        self.logout = []
        self.mutations = []
        self.waits = 0

    def post(self, path, body, bearer=None):
        if path == '/v1/auth/login':
            assert body == {'email': 'person@example.test', 'password': 'password-secret'}
            return 200, {'access_token': 'access-0', 'refresh_token': 'refresh-0'}
        if path == '/v1/auth/refresh':
            assert bearer is None and body == {'refresh_token': 'refresh-' + str(self.rotation)}
            if self.mode == 'refresh_failure':
                return 503, {}
            self.rotation += 1
            return 200, {'access_token': 'access-' + str(self.rotation),
                         'refresh_token': 'refresh-' + str(self.rotation)}
        if path == '/v1/auth/logout':
            self.logout.append((body, bearer))
            assert body == {'refresh_token': 'refresh-' + str(self.rotation)} and bearer is None
            return (503, {}) if self.mode == 'logout_failure' else (204, None)
        raise AssertionError('Unexpected authentication endpoint')

    def request(self, method, path, body=None, bearer=None, *, idempotency_key=None):
        assert bearer == 'access-' + str(self.rotation)
        parsed = urlsplit(path)
        if parsed.path == '/v1/accounts' and method == 'POST':
            assert 'opening_balance' not in body and idempotency_key
            self.mutations.append(path)
            key = str(uuid.uuid4())
            record = dict(body, id=key, status='active', balance='0.00')
            self.accounts[key] = record
            if self.mode == 'metadata':
                return 201, dict(record, account_type='savings')
            return 201, {k: v for k, v in record.items() if k != 'balance'}
        if parsed.path == '/v1/accounts' and method == 'GET':
            return 200, {'accounts': copy.deepcopy(list(self.accounts.values())), 'next_cursor': None}
        if parsed.path == '/v1/transactions' and method == 'GET':
            assert 'account_id' not in parse_qs(parsed.query)
            if self.mode == 'cursor':
                return 200, {'transactions': [], 'next_cursor': 'repeated'}
            return 200, {'transactions': copy.deepcopy(self.events), 'next_cursor': None}
        if path == '/v1/recurring-rules' and method == 'POST':
            assert idempotency_key and body['starts_on'] == body['ends_on'] == '2026-11-05'
            assert body['amount'] == '10.00' and body['cadence'] == 'monthly'
            assert body['time_zone'] == 'Europe/Stockholm'
            self.mutations.append(path)
            if self.mode == 'create_failure':
                return 503, {}
            self.rule = dict(body, id=str(uuid.uuid4()), state='active',
                             paused_on=None, generated_through=None)
            if self.mode == 'wrong_rule':
                self.rule['amount'] = '11.00'
            return 201, copy.deepcopy(self.rule)
        if parsed.path == '/v1/reports/account-planning':
            assert parse_qs(parsed.query)['through'] == ['2026-11-05']
            record = next(iter(self.accounts.values()))
            row = dict(account_id=record['id'], name=record['name'], account_type='normal',
                       status='active', currency='SEK', current_balance=record['balance'],
                       pending_total=self.pending, projected_total='0.00',
                       planning_difference='-10.00' if self.generated else None,
                       complete=self.generated)
            return 200, {'through': '2026-11-05', 'accounts': [row], 'complete': False}
        raise AssertionError('Unexpected public API request')

    def operator(self, message):
        self.waits += 1
        if self.mode == 'cancel' or (self.mode == 'cancel_second' and self.waits == 2):
            raise KeyboardInterrupt()
        self.generated = self.mode != 'not_generated'
        self.pending = '10.00' if self.generated else '0.00'
        if self.mode == 'duplicate' and self.waits == 2:
            self.pending = '20.00'
        if self.mode == 'balance':
            next(iter(self.accounts.values()))['balance'] = '-10.00'
        if self.mode == 'history_change':
            self.events[0]['description'] = 'Unexpected change'
        if self.mode == 'posting':
            self.events.append({'id': str(uuid.uuid4()), 'kind': 'expense'})
        return ''


class RecurrenceHelperTest(unittest.TestCase):
    def run_flow(self, api):
        return helper.flow(api, 'person@example.test', 'password-secret',
                           today=lambda: '2026-11-05', prompt=api.operator)

    def test_generated_totals_repeat_and_financial_snapshots_stay_unchanged(self):
        api, output = FakeApi(), io.StringIO()
        with contextlib.redirect_stdout(output):
            self.run_flow(api)
        self.assertIn('Recurrence checks passed', output.getvalue())
        self.assertEqual(api.waits, 2)
        self.assertEqual(api.rotation, 2)
        self.assertEqual(api.events, [])
        self.assertEqual(api.pending, '10.00')
        self.assertEqual(api.logout, [({'refresh_token': 'refresh-2'}, None)])
        self.assertEqual(api.mutations, ['/v1/accounts', '/v1/recurring-rules'])
        for secret in ('person@example.test', 'password-secret', 'access-', 'refresh-', api.rule['id']):
            self.assertNotIn(secret, output.getvalue())

    def test_failures_never_report_success_or_retry_mutations(self):
        for mode in ('not_generated', 'duplicate', 'balance', 'posting', 'metadata',
                     'wrong_rule', 'refresh_failure', 'logout_failure', 'create_failure', 'cursor', 'history_change'):
            with self.subTest(mode=mode):
                api, output = FakeApi(mode), io.StringIO()
                with contextlib.redirect_stdout(output), contextlib.redirect_stderr(io.StringIO()):
                    with self.assertRaises(helper.VerificationError):
                        self.run_flow(api)
                self.assertNotIn('checks passed', output.getvalue())
                self.assertEqual(len(api.logout), 1)
                self.assertLessEqual(api.mutations.count('/v1/recurring-rules'), 1)

    def test_cli_reports_exit_and_hides_credentials(self):
        for mode, expected in (('normal', 0), ('not_generated', 1), ('cancel', 130)):
            with self.subTest(mode=mode):
                api, output, errors = FakeApi(mode), io.StringIO(), io.StringIO()
                answers = iter(('yes', 'person@example.test'))

                def answer(message):
                    if message.startswith('After a successful'):
                        return api.operator(message)
                    return next(answers)

                with patch.object(sys, 'argv', ['check_recurrence_generation.py', '--api', 'https://api.example.test']), \
                        patch.object(sys.stdin, 'isatty', return_value=True), \
                        patch.object(helper, 'Api', return_value=api), \
                        patch.object(helper.getpass, 'getpass', return_value='password-secret'), \
                        patch('builtins.input', side_effect=answer), \
                        patch.object(helper, 'datetime') as clock, \
                        contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
                    clock.now.return_value.date.return_value.isoformat.return_value = '2026-11-05'
                    self.assertEqual(helper.main(), expected)
                self.assertEqual('Recurrence checks passed' in output.getvalue(), expected == 0)
                self.assertEqual(len(api.logout), 1)
                for secret in ('person@example.test', 'password-secret', 'access-', 'refresh-'):
                    self.assertNotIn(secret, output.getvalue() + errors.getvalue())

    def test_second_pause_cancellation_revokes_rotated_session(self):
        api = FakeApi('cancel_second')
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(KeyboardInterrupt):
            self.run_flow(api)
        self.assertEqual(api.rotation, 1)
        self.assertEqual(api.logout, [({'refresh_token': 'refresh-1'}, None)])

    def test_cancellation_revokes_session_and_preserves_synthetic_records(self):
        api = FakeApi('cancel')
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(KeyboardInterrupt):
            self.run_flow(api)
        self.assertEqual(len(api.accounts), 1)
        self.assertIsNotNone(api.rule)
        self.assertEqual(api.logout, [({'refresh_token': 'refresh-0'}, None)])


if __name__ == '__main__':
    unittest.main()
