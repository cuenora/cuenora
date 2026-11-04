"""Offline regression checks for bounded staging transfer concurrency."""
import contextlib
from decimal import Decimal
import importlib.util
import io
from pathlib import Path
import sys
import threading
import time
import unittest
from urllib.parse import parse_qs, urlsplit
import uuid

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'scripts'))
spec = importlib.util.spec_from_file_location('transfer_helper', ROOT / 'scripts/check_transfer_concurrency.py')
helper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper)


class FakeApi:
    base = 'https://api.example.test'

    def __init__(self, duplicate=False, partial=False, lost_credit=False, wrong_metadata=False):
        self.accounts, self.events, self.keys = {}, {}, {}
        self.lock = threading.Lock()
        self.active = self.peak = 0
        self.duplicate, self.partial, self.lost_credit = duplicate, partial, lost_credit
        self.logout = []
        self.wrong_metadata = wrong_metadata

    def post(self, path, body, bearer=None):
        if path.endswith('/login'):
            return 200, {'access_token': 'access-secret', 'refresh_token': 'refresh-secret'}
        if path.endswith('/logout'):
            self.logout.append((body, bearer))
            return (204, None) if body == {'refresh_token': 'refresh-secret'} and bearer is None else (400, {})
        raise AssertionError('Unexpected endpoint')

    def request(self, method, path, body=None, bearer=None, *, idempotency_key=None):
        if path == '/v1/me':
            return 200, dict(body)
        if path == '/v1/accounts' and method == 'POST':
            key = str(uuid.uuid4())
            record = dict(body)
            record.pop('opening_balance', None)
            record.update(id=key, status='active', balance=body.get('opening_balance', {}).get('amount', '0.00'))
            self.accounts[key] = record
            result = {k: v for k, v in record.items() if k != 'balance'}
            if self.wrong_metadata:
                result['account_type'] = 'savings'
            return 201, result
        if path.startswith('/v1/accounts?'):
            return 200, {'accounts': [dict(a) for a in self.accounts.values()], 'next_cursor': None}
        if method == 'GET' and path.startswith('/v1/transactions?'):
            account = parse_qs(urlsplit(path).query)['account_id'][0]
            return 200, {'transactions': [dict(e) for e in self.events.values()
                                         if account in (e['account_id'], e['destination_account_id'])],
                         'next_cursor': None}
        if path == '/v1/transactions' and method == 'POST':
            with self.lock:
                self.active += 1
                self.peak = max(self.peak, self.active)
            try:
                time.sleep(0.01)
                with self.lock:
                    return self.transfer(body, idempotency_key)
            finally:
                with self.lock:
                    self.active -= 1
        raise AssertionError('Unexpected request')

    def transfer(self, body, key):
        source = self.accounts[body['account_id']]
        destination = self.accounts[body['destination_account_id']]
        if source is destination or destination['currency'] != body['received_currency']:
            if self.partial:
                source['balance'] = format(Decimal(source['balance']) - Decimal('1.00'), '.2f')
            return 400, {'code': 'invalid_request'}
        if key in self.keys and not self.duplicate:
            previous, result = self.keys[key]
            return (201, dict(result)) if previous == body else (409, {'code': 'idempotency_conflict'})
        event = dict(body, id=str(uuid.uuid4()), state='posted', category_id=None)
        source['balance'] = format(Decimal(source['balance']) - Decimal(body['amount']), '.2f')
        if not self.lost_credit:
            destination['balance'] = format(Decimal(destination['balance']) + Decimal(body['received_amount']), '.2f')
        self.events[event['id']] = event
        self.keys[key] = (dict(body), event)
        return 201, dict(event)


class TransferConcurrencyTest(unittest.TestCase):
    def run_flow(self, api):
        return helper.flow(api, 'person@example.test', 'password-secret', '2026-11-04', factory=lambda _: api)

    def test_same_key_once_unique_keys_all_and_safe_cleanup(self):
        api = FakeApi()
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            self.run_flow(api)
        self.assertEqual(len(api.events), 5)
        self.assertEqual(sorted(a['balance'] for a in api.accounts.values()), ['14.00', '86.00'])
        self.assertGreaterEqual(api.peak, 2)
        self.assertLessEqual(api.peak, 4)
        self.assertEqual(api.logout, [({'refresh_token': 'refresh-secret'}, None)])
        self.assertIn('Transfer concurrency checks passed', output.getvalue())
        for secret in ('person@example.test', 'password-secret', 'access-secret', 'refresh-secret'):
            self.assertNotIn(secret, output.getvalue())

    def test_wrong_creation_metadata_stops_before_transfers(self):
        api = FakeApi(wrong_metadata=True)
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(helper.VerificationError):
            self.run_flow(api)
        self.assertEqual(api.events, {})
        self.assertEqual(len(api.logout), 1)

    def test_duplicate_postings_lost_credit_or_partial_rejection_cannot_pass(self):
        for mode in ('duplicate', 'partial', 'lost_credit'):
            with self.subTest(mode=mode):
                api = FakeApi(**{mode: True})
                output = io.StringIO()
                with contextlib.redirect_stdout(output), self.assertRaises(helper.VerificationError):
                    self.run_flow(api)
                self.assertNotIn('checks passed', output.getvalue())
                self.assertEqual(len(api.logout), 1)


if __name__ == '__main__':
    unittest.main()
