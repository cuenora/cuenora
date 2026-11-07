# Shared account repository

`AccountRepository` is the shared boundary for listing, creating and updating
account metadata. `HttpAccountRepository` adapts the existing `/v1/accounts`
endpoints; hosts supply an `ApiClient` and a session-scoped `RepositoryContext`.
The repository does not own or close the client.

## Use

```kotlin
val repository = HttpAccountRepository(apiClient, RepositoryContext(ownerId, accessToken))
val page = repository.list(AccountListQuery(limit = 50))
val created = repository.create(
    CreateAccountCommand(AccountName("Daily"), AccountType.NORMAL, Currency.SEK),
    IdempotencyKey(callerGeneratedKey),
)
```

Listing returns `Page<AccountWithBalance>`: archived accounts are retained and
each exact balance is separate from `Account` metadata. Creation accepts an
optional signed `OpeningBalance` with its explicit `BusinessDate`; it returns
the server's metadata-only response. Updates use `PatchField.Absent` to retain a
field and support name/status assignments only. An empty update or explicit null
is rejected locally.

`RepositoryContext.ownerId` supplies domain identity because account responses
omit an owner. It is not authorization evidence; the server derives ownership
from the bearer token. The context sends the caller's credential, and create
requires a caller-provided stable `IdempotencyKey`. Updates do not send a key.
The adapter adds no retries, optimistic cache, authentication flow or server
financial-rule simulation. `RepositoryResult.Failure` preserves typed transport
and HTTP failures; malformed successful responses become redacted
`ApiFailure.InvalidResponse` values.

## Tests

Common tests use Ktor `MockEngine` through the public repository API. Hosts can
use `FakeAccountRepository` from `commonTest` to script results and inspect
captured calls; the fake deliberately does not emulate authorization, balance,
archive or idempotency rules enforced by the backend.
