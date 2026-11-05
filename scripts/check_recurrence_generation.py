#!/usr/bin/env python3
"""Interactive staging recurrence check; creates persistent synthetic planning records."""
import argparse
from datetime import datetime
import getpass
import sys
from urllib.parse import urlencode
import uuid
import warnings
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

from check_account_isolation import account_id, listing
from verify_email import Api, VerificationError, require


def transactions(api, bearer):
    records, seen = {}, set()
    cursor = None
    for _ in range(100):
        query = {'limit': 5}
        if cursor is not None:
            query['cursor'] = cursor
        status, page = api.request('GET', '/v1/transactions?' + urlencode(query), bearer=bearer)
        require(status, 200, 'Transaction listing failed; no financial verdict is available.')
        if not isinstance(page, dict) or not isinstance(page.get('transactions'), list):
            raise VerificationError('Unexpected transaction listing response.')
        for record in page['transactions']:
            if not isinstance(record, dict):
                raise VerificationError('Unexpected transaction item.')
            key = account_id(record.get('id'))
            if key in records:
                raise VerificationError('Duplicate event in transaction listing.')
            records[key] = record
        cursor = page.get('next_cursor')
        if cursor is None:
            return records
        if not isinstance(cursor, str) or not cursor or len(cursor) > 1024 or cursor in seen:
            raise VerificationError('Invalid or repeated transaction cursor.')
        seen.add(cursor)
    raise VerificationError('Transaction listing exceeded the bounded test window.')


def financial_snapshot(api, bearer):
    return listing(api, bearer), transactions(api, bearer)


def tokens(value):
    if not isinstance(value, dict) or not all(isinstance(value.get(k), str) and value[k]
                                            for k in ('access_token', 'refresh_token')):
        raise VerificationError('Unexpected session response; session revocation may be needed.')
    return value['access_token'], value['refresh_token']


def planning(api, bearer, day, account, generated):
    status, report = api.request('GET', '/v1/reports/account-planning?' + urlencode({'through': day}),
                                 bearer=bearer)
    require(status, 200, 'Account planning failed; no generation verdict is available.')
    if not isinstance(report, dict) or report.get('through') != day or not isinstance(report.get('accounts'), list):
        raise VerificationError('Unexpected account planning response.')
    rows, seen = [], set()
    for row in report['accounts']:
        if not isinstance(row, dict):
            raise VerificationError('Unexpected planning account row.')
        key = account_id(row.get('account_id'))
        if key in seen:
            raise VerificationError('Duplicate planning account row.')
        seen.add(key)
        if key == account['id']:
            rows.append(row)
    expected = dict(account_id=account['id'], name=account['name'], account_type='normal',
                    status='active', currency='SEK', current_balance='0.00',
                    pending_total='10.00' if generated else '0.00', projected_total='0.00',
                    planning_difference='-10.00' if generated else None)
    if (len(rows) != 1 or any(k not in rows[0] or rows[0][k] != v for k, v in expected.items())
            or rows[0].get('complete') is not generated):
        raise VerificationError('Synthetic planning totals or completeness do not match the expected generation state.')


def flow(api, email, password, *, today=None, prompt=input):
    status, pair = api.post('/v1/auth/login', {'email': email, 'password': password})
    require(status, 200, 'Verified test-user login failed.')
    bearer, refresh = tokens(pair)
    cleanup_failed = False
    try:
        day = (today or (lambda: datetime.now(ZoneInfo('Europe/Stockholm')).date().isoformat()))()
        body = {'name': 'Recurrence test ' + str(uuid.uuid4()), 'account_type': 'normal', 'currency': 'SEK'}
        status, account = api.request('POST', '/v1/accounts', body, bearer,
                                      idempotency_key=uuid.uuid4().hex)
        require(status, 201, 'Synthetic account creation failed or is uncertain; do not retry automatically.')
        if (not isinstance(account, dict) or any(account.get(k) != v for k, v in body.items())
                or account.get('status') != 'active'):
            raise VerificationError('Unexpected synthetic account metadata.')
        key = account_id(account.get('id'))
        initial = financial_snapshot(api, bearer)
        if initial[0].get(key) != dict(account, balance='0.00'):
            raise VerificationError('New synthetic account is missing or does not have a zero balance.')
        rule = {'amount': '10.00', 'currency': 'SEK', 'cadence': 'monthly',
                'starts_on': day, 'ends_on': day, 'time_zone': 'Europe/Stockholm',
                'account_id': key, 'category_id': None, 'description': 'Synthetic recurrence check'}
        status, result = api.request('POST', '/v1/recurring-rules', rule, bearer,
                                     idempotency_key=uuid.uuid4().hex)
        require(status, 201, 'Synthetic rule creation failed or is uncertain; do not retry automatically.')
        if (not isinstance(result, dict) or any(k not in result or result[k] != v for k, v in rule.items())
                or result.get('state') != 'active' or result.get('paused_on', False) is not None
                or result.get('generated_through', False) is not None):
            raise VerificationError('Unexpected synthetic rule metadata.')
        account_id(result.get('id'))
        planning(api, bearer, day, account, False)
        if financial_snapshot(api, bearer) != initial:
            raise VerificationError('Creating a planning rule changed public financial records.')
        print('Synthetic rule created with one eligible date; no future recurring slots.')
        print('Run /usr/local/bin/cuenora-recurrence-job in the API container, not the PostgreSQL terminal.')
        for iteration in range(2):
            if iteration:
                print('Run the same job again to check that planning totals do not duplicate.')
            prompt('After a successful job run (rules_failed=0), press Enter; Ctrl+C cancels: ')
            # The operator pause can exceed the access-token lifetime. Rotate once, never retry.
            status, pair = api.post('/v1/auth/refresh', {'refresh_token': refresh})
            require(status, 200, 'Session refresh failed or is uncertain; do not retry automatically.')
            bearer, refresh = tokens(pair)
            planning(api, bearer, day, account, True)
            if financial_snapshot(api, bearer) != initial:
                raise VerificationError('Generation changed public account snapshots or transaction history.')
    finally:
        try:
            status, _ = api.post('/v1/auth/logout', {'refresh_token': refresh})
            cleanup_failed = status != 204
        except VerificationError:
            cleanup_failed = True
        if cleanup_failed:
            print('Warning: test-session logout was not confirmed.', file=sys.stderr)
    if cleanup_failed:
        raise VerificationError('Planning checks completed, but session cleanup was not confirmed.')
    print('Recurrence checks passed: pending 10.00 SEK after both runs; zero synthetic balance and unchanged public financial snapshots.')
    print('Synthetic account, bounded rule and pending planning records remain. No payment was confirmed.')
    print('Aggregate API checks do not prove raw occurrence counts, database audit hashes or automatic scheduling.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', required=True, help='Explicit staging API URL; HTTPS required except loopback.')
    args = parser.parse_args()
    if not sys.stdin.isatty():
        parser.error('Use an interactive terminal; never pipe credentials into this helper.')
    warnings.simplefilter('error', getpass.GetPassWarning)
    try:
        api = Api(args.api)
        print('Creates a zero-balance normal SEK account and a 10.00 monthly rule starting and ending today in Europe/Stockholm.')
        print('You will run the job twice in Dokploy. It processes ALL eligible owners, not only this account.')
        print('Synthetic records remain. No profile-zone change, payment confirmation, deletion or automatic mutation retry.')
        if input('Proceed on this staging API? Type yes: ').strip().lower() != 'yes':
            return 0
        email = input('Verified test email: ')
        password = getpass.getpass('Test-account password (hidden): ')
        flow(api, email, password, prompt=input)
    except VerificationError as error:
        print('Recurrence test could not complete: ' + str(error), file=sys.stderr)
        print('Synthetic records may remain; inspect before rerunning.', file=sys.stderr)
        return 1
    except ZoneInfoNotFoundError:
        print('Europe/Stockholm zone data is unavailable; no planning records were created.', file=sys.stderr)
        return 1
    except getpass.GetPassWarning:
        print('Secure hidden input is unavailable; test stopped.', file=sys.stderr)
        return 1
    except (KeyboardInterrupt, EOFError):
        print('\nRecurrence test cancelled; synthetic records may remain.', file=sys.stderr)
        return 130
    return 0


if __name__ == '__main__':
    sys.exit(main())
