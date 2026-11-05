# Check recurrence generation in staging

Use one verified synthetic staging user and the deployed recurrence job. The
interactive helper checks aggregate planning results across two manual job runs;
it never invokes Docker, SSH or a scheduler on your behalf.

## Run from your local repository

```bash
python3 scripts/check_recurrence_generation.py --api https://cuenora.carf.dev
```

Review the warning and type `yes`. Enter the test email and hidden current
password only into the helper. Keep it running while you use Dokploy's **API
container terminal** to run this command at each of its two prompts:

```sh
/usr/local/bin/cuenora-recurrence-job
```

Wait for completion and require `rules_failed=0`; an overlap/no-op alone does not
prove the active job succeeded. Return to the helper and press Enter. Run the
job again at the second prompt to check unchanged planning totals after a retry.
The job processes **all eligible owners**, not only the helper's account.

Do not modify this test user's accounts or transactions concurrently. If a
scheduled generator is already enabled, coordinate the test to avoid it generating
the new rule before the helper captures the expected incomplete baseline.

## Persistent effects

- Creates one normal SEK account with no opening balance.
- Creates a monthly rule for `10.00` SEK, linked to that account, with start and
  inclusive end both today in `Europe/Stockholm`. There is one eligible calendar
  slot, with no future obligations. The existing profile zone is not changed.
- Manual generation leaves pending planning records; it does not post an expense.
- Records remain on success, failure or cancellation. There is no deletion or
  automatic mutation retry; another invocation creates new synthetic records.
- The helper refreshes its session after each operator pause and logs out using
  the latest retained refresh token. A refresh failure stops without retry; if
  logout is unconfirmed, inspect/revoke the test sessions rather than assuming cleanup.

## Required checks and boundaries

| Stage | New account planning result |
| --- | --- |
| Rule created, before job | Balance/pending/projected `0.00`, incomplete, difference null |
| After first job | Balance `0.00`, pending `10.00`, projected `0.00`, complete, difference `-10.00` |
| After second job | Same totals and completeness; no doubled pending amount |

At all stages, account metadata/balance snapshots and the user's public transaction
history must remain unchanged from the post-account-creation baseline. The helper
validates creation metadata and its targeted account row, not global report
completeness or the changing `as_of` timestamp. It paginates accounts/transactions
within 100 pages of five records; larger histories or responses exceeding the
shared transport's 16 KiB bound fail without a success verdict.

The API has no public recurrence occurrence listing. This checks a one-slot
rule's aggregate result and retry stability, **not raw occurrence counts/IDs,
database movement/audit hashes, overlap exclusion, crash behavior or automatic
scheduler timing**. Zero-rule job output does not establish generation writes.
Only successful checks and confirmed session logout print `Recurrence checks passed`.
Never share passwords, tokens, full API responses or private record identifiers.

## Offline regression tests

```bash
python3 -m unittest discover -s tests/infrastructure -p 'test_*helper*.py'
```

The fake API models missing generation, duplicated totals, changed balances,
premature transactions, malformed metadata, cursor failure, refresh/logout
failure and cancellation. These tests do not access staging or prove live database
permissions. Record actual operator evidence separately after the smoke.

See [Dokploy scheduling](DOKPLOY_RECURRENCE_JOB.md) and
[account planning semantics](../product/account-planning.md).
