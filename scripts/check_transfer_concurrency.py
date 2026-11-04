#!/usr/bin/env python3
"""Bounded staging transfer smoke test: synthetic records, at most four concurrent requests."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
from decimal import Decimal, InvalidOperation
import getpass
import sys
import threading
from urllib.parse import urlencode
import uuid
import warnings
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

from check_account_isolation import account_id, listing
from verify_email import Api, VerificationError, require


def history(api, bearer, account):
    records, seen = {}, set()
    cursor = None
    for _ in range(100):
        query = {'account_id': account, 'limit': 5}
        if cursor is not None:
            query['cursor'] = cursor
        status, page = api.request('GET', '/v1/transactions?' + urlencode(query), bearer=bearer)
        require(status, 200, 'Transaction history failed; no verdict is available.')
        if not isinstance(page, dict) or not isinstance(page.get('transactions'), list):
            raise VerificationError('Unexpected transaction history response.')
        for record in page['transactions']:
            if not isinstance(record, dict):
                raise VerificationError('Unexpected transaction history item.')
            key = account_id(record.get('id'))
            if key in records:
                raise VerificationError('Duplicate event in paginated history.')
            records[key] = record
        cursor = page.get('next_cursor')
        if cursor is None:
            return records
        if not isinstance(cursor, str) or not cursor or len(cursor) > 1024 or cursor in seen:
            raise VerificationError('Invalid or repeated transaction cursor.')
        seen.add(cursor)
    raise VerificationError('Transaction history exceeded the bounded test window.')


def snapshot(api, bearer, accounts):
    return (listing(api, bearer), *(history(api, bearer, key) for key in accounts))


def balances(state, accounts, expected):
    try:
        values = [state[0][key]['balance'] for key in accounts]
        if not all(isinstance(value, str) for value in values):
            raise VerificationError('Synthetic balances must be decimal strings.')
        actual = tuple(Decimal(value) for value in values)
    except (KeyError, InvalidOperation, TypeError):
        raise VerificationError('Unexpected synthetic account balance response.') from None
    if actual != tuple(Decimal(value) for value in expected):
        raise VerificationError('Synthetic balances do not match exact expected postings.')


def validate_transfer(value, payload):
    if not isinstance(value, dict):
        raise VerificationError('Unexpected transfer response.')
    key = account_id(value.get('id'))
    if value.get('state') != 'posted' or any(value.get(k) != v for k, v in payload.items()):
        raise VerificationError('Transfer fields do not match the intended synthetic operation.')
    return key


def parallel(api, bearer, payload, keys, factory):
    barrier = threading.Barrier(4, timeout=10)

    def send(key):
        # Separate opener/connection state per worker; no credentials in arguments or logs.
        client = factory(api.base)
        try:
            barrier.wait()
        except threading.BrokenBarrierError:
            raise VerificationError('Concurrent request start failed; do not retry automatically.') from None
        return client.request('POST', '/v1/transactions', payload, bearer, idempotency_key=key)

    with ThreadPoolExecutor(max_workers=4) as pool:
        futures = [pool.submit(send, key) for key in keys]
        responses = [future.result() for future in futures]
    for status, _ in responses:
        require(status, 201, 'A concurrent mutation failed or is uncertain; do not retry automatically.')
    return [value for _, value in responses]


def check_history(state, original, payload, expected_ids, first_id):
    for current, baseline in zip(state[1:], original[1:]):
        if any(current.get(key) != value for key, value in baseline.items()):
            raise VerificationError('Preexisting synthetic history changed.')
        if set(current) - set(baseline) != expected_ids:
            raise VerificationError('Transfer history contains missing or duplicate events.')
        for key in expected_ids:
            # The payload amount varies in the unique-key wave; check its shared fields here.
            shared = {k: v for k, v in payload.items() if k not in ('amount', 'received_amount')}
            validate_transfer(current[key], shared)
            expected_amount = '10.00' if key == first_id else '1.00'
            if current[key].get('amount') != expected_amount or current[key].get('received_amount') != expected_amount:
                raise VerificationError('Transfer history has unexpected exact amounts.')


def check_transfers(api, bearer, day, factory):
    status, result = api.request('PATCH', '/v1/me', {'time_zone': 'Europe/Stockholm'}, bearer)
    require(status, 200, 'Profile time-zone confirmation failed.')
    if result != {'time_zone': 'Europe/Stockholm'}:
        raise VerificationError('Unexpected profile time-zone response.')
    accounts = []
    for opening in ('100.00', None):
        body = {'name': 'Concurrency test ' + str(uuid.uuid4()), 'account_type': 'normal', 'currency': 'SEK'}
        if opening is not None:
            body['opening_balance'] = {'amount': opening, 'occurred_on': day}
        status, result = api.request('POST', '/v1/accounts', body, bearer, idempotency_key=uuid.uuid4().hex)
        require(status, 201, 'Synthetic account creation failed or is uncertain; do not retry automatically.')
        if not isinstance(result, dict):
            raise VerificationError('Unexpected synthetic account creation response.')
        if (any(result.get(key) != body[key] for key in ('name', 'account_type', 'currency'))
                or result.get('status') != 'active'):
            raise VerificationError('Synthetic account creation returned unexpected metadata.')
        accounts.append(account_id(result.get('id')))
    if accounts[0] == accounts[1]:
        raise VerificationError('Distinct synthetic accounts were not created.')
    initial = snapshot(api, bearer, accounts)
    balances(initial, accounts, ('100.00', '0.00'))
    payload = {'kind': 'transfer', 'amount': '10.00', 'currency': 'SEK',
               'account_id': accounts[0], 'destination_account_id': accounts[1],
               'received_amount': '10.00', 'received_currency': 'SEK', 'occurred_on': day}
    same_key = uuid.uuid4().hex
    responses = parallel(api, bearer, payload, [same_key] * 4, factory)
    first_id = validate_transfer(responses[0], payload)
    if any(validate_transfer(value, payload) != first_id or value != responses[0] for value in responses):
        raise VerificationError('Same-key concurrent requests did not replay one identical transfer.')
    after_replay = snapshot(api, bearer, accounts)
    balances(after_replay, accounts, ('90.00', '10.00'))
    check_history(after_replay, initial, payload, {first_id}, first_id)

    cases = [(dict(payload, amount='11.00', received_amount='11.00'), same_key, 409, 'idempotency_conflict'),
             (dict(payload, destination_account_id=accounts[0]), uuid.uuid4().hex, 400, 'invalid_request'),
             (dict(payload, received_currency='EUR'), uuid.uuid4().hex, 400, 'invalid_request')]
    for rejected, key, expected, code in cases:
        status, error = api.request('POST', '/v1/transactions', rejected, bearer, idempotency_key=key)
        require(status, expected, 'An invalid or conflicting transfer was not rejected.')
        if not isinstance(error, dict) or error.get('code') != code:
            raise VerificationError('Unexpected transfer rejection response.')
        if snapshot(api, bearer, accounts) != after_replay:
            raise VerificationError('A rejected transfer changed public balances or history.')

    unique_payload = dict(payload, amount='1.00', received_amount='1.00')
    responses = parallel(api, bearer, unique_payload, [uuid.uuid4().hex for _ in range(4)], factory)
    ids = {validate_transfer(value, unique_payload) for value in responses}
    if len(ids) != 4 or first_id in ids:
        raise VerificationError('Distinct idempotency keys did not create four distinct operations.')
    final = snapshot(api, bearer, accounts)
    balances(final, accounts, ('86.00', '14.00'))
    check_history(final, initial, payload, ids | {first_id}, first_id)


def flow(api, email, password, day, *, factory=Api):
    status, pair = api.post('/v1/auth/login', {'email': email, 'password': password})
    require(status, 200, 'Test-user login failed.')
    if not isinstance(pair, dict) or not all(isinstance(pair.get(k), str) for k in ('access_token', 'refresh_token')):
        raise VerificationError('Unexpected login response.')
    cleanup_failed = False
    try:
        check_transfers(api, pair['access_token'], day, factory)
    finally:
        try:
            status, _ = api.post('/v1/auth/logout', {'refresh_token': pair['refresh_token']})
            cleanup_failed = status != 204
        except VerificationError:
            cleanup_failed = True
        if cleanup_failed:
            print('Warning: test session logout was not confirmed.', file=sys.stderr)
    if cleanup_failed:
        raise VerificationError('Transfer checks completed, but session cleanup was not confirmed.')
    print('Transfer concurrency checks passed: one same-key event, four distinct events, exact balances and rejected-write snapshots.')
    print('Synthetic balances: 86.00 SEK and 14.00 SEK. Accounts, opening and five transfers remain.')
    print('No database failure injection or load/security certification was performed.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', required=True, help='Explicit staging API URL; HTTPS required except loopback.')
    args = parser.parse_args()
    if not sys.stdin.isatty():
        parser.error('Use an interactive terminal; never pipe credentials into this helper.')
    warnings.simplefilter('error', getpass.GetPassWarning)
    try:
        api = Api(args.api)
        day = datetime.now(ZoneInfo('Europe/Stockholm')).date().isoformat()
        print('Confirms Europe/Stockholm on one test profile; creates two normal SEK accounts, one opening of 100.00 and five recorded transfers.')
        print('At most four concurrent requests. Synthetic records remain; failures are never retried automatically.')
        if input('Proceed on this staging API? Type yes: ').strip().lower() != 'yes':
            return 0
        email = input('Verified test email: ')
        password = getpass.getpass('Test-account password (hidden): ')
        flow(api, email, password, day)
    except VerificationError as error:
        print('Transfer test could not complete: ' + str(error), file=sys.stderr)
        print('Synthetic records may remain; inspect before rerunning.', file=sys.stderr)
        return 1
    except ZoneInfoNotFoundError:
        print('Europe/Stockholm zone data is unavailable; no test was started.', file=sys.stderr)
        return 1
    except getpass.GetPassWarning:
        print('Secure hidden input is unavailable; test stopped.', file=sys.stderr)
        return 1
    except (KeyboardInterrupt, EOFError):
        print('\nTransfer test cancelled; synthetic records may remain.', file=sys.stderr)
        return 130
    return 0


if __name__ == '__main__':
    sys.exit(main())
