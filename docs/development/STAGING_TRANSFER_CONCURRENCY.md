# Check bounded transfer concurrency in staging

Use one **verified staging user** and synthetic records only. This checks final
public balance/history invariants, idempotency and rejected-write effects. It is
not a load test or proof of atomic behavior under injected database failure.

## Run from your local repository

```bash
python3 scripts/check_transfer_concurrency.py --api https://cuenora.carf.dev
```

The maintainer approved `Europe/Stockholm` for the first test user's profile.
The helper confirms that zone, selects today's date in that zone, then asks for
the verified test email and a hidden password. Review the side-effect warning
and explicitly type `yes`. Never share passwords, tokens or full responses.

## Persistent effects and limits

- Confirms or updates that user's profile zone to `Europe/Stockholm`.
- Creates two new normal SEK accounts, with a synthetic opening of `100.00` in
  the source and zero in the destination. The opening is not ordinary income.
- Sends two waves of four requests, with independent HTTP clients per worker.
  Maximum concurrency is four; no automatic mutation retries are performed.
- A successful test leaves the two accounts, opening posting and five transfers.
  This records synthetic application data; it does not execute bank payments.
- Logs out only its newly created session using the retained refresh credential.
- Uses the shared identity helper's HTTPS/no-redirect/no-environment-proxy transport.
  It keeps credentials/responses in memory, without printing account IDs or lists.

Do not perform other account edits or postings for this user while testing.
Account listings and transaction histories are bounded to 500 records per scope.
A failure or interruption can leave records or have an unknown mutation outcome;
inspect before rerunning. A new run creates additional synthetic data, not a retry.

## Required assertions

| Probe | Expected result |
| --- | --- |
| Four identical `10.00` transfers with one idempotency key | All 201; identical result and one event ID; balances `90.00` / `10.00` |
| Same key with changed amounts | 409 `idempotency_conflict`; balances/history unchanged |
| Transfer to the same account | 400 `invalid_request`; balances/history unchanged |
| Destination currency mismatch | 400 `invalid_request`; balances/history unchanged |
| Four `1.00` transfers with distinct keys | Four distinct events; final balances `86.00` / `14.00` |
| Both account histories | Exactly five new transfers, each listed once per account; exact expected fields/amounts and unchanged baseline events |
| Session logout | 204 using `refresh_token` in the body, without an access bearer |

Only after these checks does the helper print `Transfer concurrency checks passed`.
No direct SQL movement-count/hash probe, database crash, connection termination,
load, rate-limit exhaustion or cross-user transfer test is included. The check
cannot certify the whole backend or its deployment as production-ready.

## Local regression checks

```bash
python3 -m unittest discover -s tests/infrastructure -p 'test_*helper*.py'
```

These tests use fake APIs, not staging credentials, real mail or financial data.
They include duplicate postings, lost credits and rejected partial-write models
that must never produce a success verdict. Actual staging evidence is separate.
