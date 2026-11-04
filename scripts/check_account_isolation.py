#!/usr/bin/env python3
"""Interactive staging account isolation check; creates two zero-balance savings accounts."""
import argparse
import getpass
import json
import sys
import uuid
import warnings
from urllib.parse import urlencode

from verify_email import Api, VerificationError, require


def account_id(value):
    try:
        return str(uuid.UUID(value))
    except (ValueError, TypeError, AttributeError):
        raise VerificationError('Unexpected account identifier.') from None


def listing(api, bearer):
    accounts = {}
    cursor = None
    seen = set()
    for _ in range(100):
        query = {'limit': 5}
        if cursor:
            query['cursor'] = cursor
        status, page = api.request('GET', '/v1/accounts?' + urlencode(query), bearer=bearer)
        require(status, 200, 'Account listing failed; no isolation verdict is available.')
        if not isinstance(page, dict) or not isinstance(page.get('accounts'), list):
            raise VerificationError('Unexpected account listing response.')
        for item in page['accounts']:
            if not isinstance(item, dict):
                raise VerificationError('Unexpected account listing item.')
            key = account_id(item.get('id'))
            if key in accounts:
                raise VerificationError('Duplicate account in listing.')
            accounts[key] = item
        cursor = page.get('next_cursor')
        if cursor is None:
            return accounts
        cursor = account_id(cursor)
        if cursor in seen:
            raise VerificationError('Repeated account cursor.')
        seen.add(cursor)
    raise VerificationError('Account listing exceeded the bounded test window.')


def reject(status, expected, body, foreign):
    require(status, expected, 'Cross-user request was not rejected as expected. Stop testing.')
    if not isinstance(body, dict) or not isinstance(body.get('code'), str):
        raise VerificationError('Unexpected rejection response.')
    if set(body) - {'code', 'message', 'details', 'request_id'} or body.get('details') is not None:
        raise VerificationError('Cross-user rejection returned unexpected details.')
    rendered = json.dumps(body)
    if foreign['id'] in rendered or foreign['name'] in rendered:
        raise VerificationError('Cross-user rejection disclosed test account metadata.')


def check_accounts(api, sessions):
    accounts = []
    for index, bearer in enumerate(sessions):
        body = {'name': 'Isolation test ' + str(uuid.uuid4()), 'account_type': 'savings', 'currency': 'SEK'}
        status, result = api.request('POST', '/v1/accounts', body, bearer,
                                     idempotency_key=uuid.uuid4().hex)
        require(status, 201, 'Test account creation failed or is uncertain; do not retry automatically.')
        if not isinstance(result, dict):
            raise VerificationError('Unexpected account creation response; a test account may remain.')
        account_id(result.get('id'))
        if any(result.get(key) != value for key, value in body.items()) or result.get('status') != 'active':
            raise VerificationError('Unexpected test account metadata.')
        accounts.append(result)
        other = listing(api, sessions[1 - index])
        if result['id'] in other:
            raise VerificationError('Account listing exposed another user account, or both logins are the same user.')

    # Positive controls prove that the tested routes work for each owner.
    for bearer, own in zip(sessions, accounts):
        own['name'] += ' owner control'
        status, result = api.request('PATCH', '/v1/accounts/' + own['id'], {'name': own['name']}, bearer)
        require(status, 200, 'Owner rename control failed.')
        if not isinstance(result, dict) or result.get('id') != own['id'] or result.get('name') != own['name']:
            raise VerificationError('Owner rename control returned unexpected metadata.')
        status, terms = api.request('GET', '/v1/accounts/' + own['id'] + '/savings-terms', bearer=bearer)
        require(status, 200, 'Owner savings-history control failed.')
        if terms != {'versions': [], 'next_cursor': None}:
            raise VerificationError('New test account has unexpected savings configuration.')

    before = [listing(api, bearer) for bearer in sessions]
    for index, bearer in enumerate(sessions):
        own, foreign = accounts[index], accounts[1 - index]
        if own['id'] not in before[index] or foreign['id'] in before[index]:
            raise VerificationError('Account listing isolation failed.')
        if before[index][own['id']].get('balance') != '0.00':
            raise VerificationError('Test account does not have the expected zero balance.')
        status, body = api.request('GET', '/v1/accounts/' + foreign['id'] + '/savings-terms', bearer=bearer)
        reject(status, 404, body, foreign)
        status, body = api.request('PATCH', '/v1/accounts/' + foreign['id'],
                                   {'name': 'Unauthorized isolation probe'}, bearer)
        reject(status, 404, body, foreign)
        status, body = api.request('GET', '/v1/accounts?' + urlencode({'limit': 5, 'cursor': foreign['id']}),
                                   bearer=bearer)
        reject(status, 400, body, foreign)
    if [listing(api, bearer) for bearer in sessions] != before:
        raise VerificationError('Public account snapshots changed after rejected requests.')


def flow(api, credentials):
    sessions = []
    refresh_tokens = []
    cleanup_failed = False
    try:
        for email, password in credentials:
            status, pair = api.post('/v1/auth/login', {'email': email, 'password': password})
            require(status, 200, 'Test-user login failed; no account checks were completed.')
            if (not isinstance(pair, dict) or not isinstance(pair.get('access_token'), str)
                    or not isinstance(pair.get('refresh_token'), str)):
                raise VerificationError('Unexpected login response.')
            sessions.append(pair['access_token'])
            refresh_tokens.append(pair['refresh_token'])
        if len(sessions) != 2:
            raise VerificationError('Exactly two test users are required.')
        check_accounts(api, sessions)
    finally:
        for refresh_token in refresh_tokens:
            try:
                status, _ = api.post('/v1/auth/logout', {'refresh_token': refresh_token})
                cleanup_failed |= status != 204
            except VerificationError:
                cleanup_failed = True
        if cleanup_failed:
            print('Warning: test session logout was not confirmed.', file=sys.stderr)
    if cleanup_failed:
        raise VerificationError('Account checks completed, but test session cleanup was not confirmed.')
    print('Account isolation checks passed: owner controls, cross-user reads/writes and cursors, unchanged public snapshots.')
    print('Two synthetic zero-balance savings accounts remain. This is not a complete financial security audit.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--api', required=True, help='Explicit staging API URL; HTTPS required except loopback.')
    args = parser.parse_args()
    if not sys.stdin.isatty():
        parser.error('Use an interactive terminal; never pipe credentials into this helper.')
    warnings.simplefilter('error', getpass.GetPassWarning)
    try:
        api = Api(args.api)
        print('Creates two synthetic zero-balance savings accounts and renames only those accounts.')
        print('Tests rejection of cross-user requests; logs out test sessions. Accounts are not deleted.')
        if input('Proceed on this staging API? Type yes: ').strip().lower() != 'yes':
            return 0
        credentials = []
        for label in ('A', 'B'):
            email = input('Verified test email for user ' + label + ': ')
            password = getpass.getpass('Test-account password (hidden): ')
            credentials.append((email, password))
        if credentials[0][0].strip().casefold() == credentials[1][0].strip().casefold():
            raise VerificationError('Use two different verified test users.')
        flow(api, credentials)
    except VerificationError as error:
        print('Isolation test could not complete: ' + str(error), file=sys.stderr)
        print('Test accounts may remain; no automatic mutation retry was attempted.', file=sys.stderr)
        return 1
    except getpass.GetPassWarning:
        print('Secure hidden input is unavailable; test stopped.', file=sys.stderr)
        return 1
    except (KeyboardInterrupt, EOFError):
        print('\nIsolation test cancelled; test accounts may remain.', file=sys.stderr)
        return 130
    return 0


if __name__ == '__main__':
    sys.exit(main())
