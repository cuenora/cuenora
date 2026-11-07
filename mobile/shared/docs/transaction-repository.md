# Transaction repository

`TransactionRepository` is the shared client boundary for recorded transaction history and the existing transaction resource endpoints. It maps transport JSON to domain values; it does not calculate balances, emulate server validation, cache pages, retry mutations, or own credentials and retry-key lifetimes.

## Use

Construct `HttpTransactionRepository` with the application-owned `ApiClient` and a `RepositoryContext` for the current session. Supply a fresh explicit `IdempotencyKey` for each logical create, edit, or void operation, and reuse that same key only when resolving an ambiguous retry of that operation. The repository does not close the client; its caller owns client/session disposal.

Use `TransactionQuery` for bounded pages and inclusive business-date, category, and account filters. `TransactionAccountFilter.Accountless` sends the API's literal `account_id=none`; a linked account filter sends its UUID. Keep filters unchanged when continuing with `Page.nextCursor`.

Use one of the typed create commands for income, expense, interest income, or a two-leg transfer. Money remains exact and carries its currency. A transfer's optional `quotedRate` is sent as the caller-provided decimal string; the recorded received amount remains authoritative. Both listing and creation return `TransactionRecord`, which preserves optional server quote-comparison information separately from the recorded `Transaction`, including for voided transfer history.

`TransactionPatch` uses `PatchField.Absent`, `Null`, or `Value` to preserve, clear, or replace nullable API fields. Amount and currency are separate fields: use `DecimalAmount("2.50")` for an amount-only change, and explicitly supply `currency` to change it. This currencyless transport field validates recorded decimal scale/range; it cannot accidentally discard a `Money` currency. Creation and recorded responses still use currency-bearing `Money`. The API does not support editing the quoted rate. Edit and void return a `TransactionAcknowledgement` (`id` and `state`), not a current transaction snapshot.

## Fakes and failures

`FakeTransactionRepository` captures calls and lets tests configure each operation's result or suspend/cancel before a call. It deliberately does not simulate server-side financial rules. HTTP and malformed-success responses remain `RepositoryResult.Failure(ApiFailure...)`; coroutine cancellation propagates.
