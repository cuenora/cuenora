# Check account isolation between two staging users

This interactive smoke test checks **account metadata isolation**, not the entire
financial API. Use two different, already verified users on an explicitly selected
staging API. Never use personal financial data or production accounts.

## Run from your local repository

```bash
python3 scripts/check_account_isolation.py --api https://cuenora.carf.dev
```

Review the side-effect warning, type `yes`, then enter each user's email and
password at the prompts. Passwords are hidden; never pass credentials as command
arguments or share them, access tokens or full response bodies in chat.

## What changes

- Two new synthetic SEK savings accounts, with no opening balance or postings.
- Each owner renames only their new test account as a positive control.
- The helper attempts cross-user reads, renames and foreign pagination cursors.
  A correct server rejects these without changing account snapshots.
- The helper logs out its newly created test sessions; it does not revoke other
  sessions, delete accounts, change profile zones or request real-money transfers.
- Test accounts remain visible afterward. Do not automatically repeat a failed or
  interrupted run: a failed response can have an unknown mutation outcome.

Stop other account edits during the test so snapshot comparisons are meaningful.
The helper reads account pages in memory with a bounded 500-account window per
user. It never prints or saves account lists, credentials or tokens. It reuses the
verification helper's HTTPS, no-redirect, no-environment-proxy transport and
application user agent.

## Expected proof

Success requires both logins and owner controls to pass, then in both directions:

1. Each owner's listing includes their account and excludes the other's.
2. A direct foreign savings-history read returns 404 without account metadata.
3. A foreign rename returns 404 without account metadata.
4. A cursor referencing the other owner's account returns 400.
5. Public account snapshots remain identical after the rejected requests.
6. Logout of the test sessions is acknowledged using each login's retained
   refresh credential in the request body, without an access bearer.

Only then does the helper print `Account isolation checks passed`.
This is public-interface smoke evidence, not a database hash comparison or proof
of isolation for transactions, transfers, recurrence, reports, all preexisting
resources or concurrent writes. It does not test load, backup restoration or
origin network isolation.

Local regression tests run without a network, real accounts or real mail:

```bash
python3 -m unittest discover -s tests/infrastructure -p 'test_*helper*.py'
```

A corrected invocation can close only sessions created by that invocation. It
cannot recover tokens from an earlier exited process or prove those older
sessions were revoked. Each repeated full run creates two additional accounts.
