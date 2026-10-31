//! Owner-serialized corrections: inverse original postings, never rewrite history.
use crate::{
    account_creation::today_at, api_error::ApiError, auth_identity::AuthenticatedIdentity,
    income_http::normalize_uuid, profile_zone::parse_zone, request_id::RequestId,
};
use axum::{
    Extension, Json,
    extract::{
        Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::{HeaderMap, Uri},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use cuenora_domain::{BusinessDate, BusinessDateError, Currency, Money, QuotedRate};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};

#[derive(Clone, Default)]
enum Field {
    #[default]
    Absent,
    Null,
    Value(String),
}
impl<'de> Deserialize<'de> for Field {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<String>::deserialize(deserializer)? {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}
impl Field {
    fn apply(&self, target: &mut Option<String>) {
        match self {
            Self::Absent => {}
            Self::Null => *target = None,
            Self::Value(value) => *target = Some(value.clone()),
        }
    }
    fn required(&self, target: &mut String) {
        if let Self::Value(value) = self {
            *target = value.clone();
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Patch {
    amount: Field,
    currency: Field,
    occurred_on: Field,
    account_id: Field,
    destination_account_id: Field,
    received_amount: Field,
    received_currency: Field,
    category_id: Field,
    description: Field,
}
impl Patch {
    fn fields(&self) -> [&Field; 9] {
        [
            &self.amount,
            &self.currency,
            &self.occurred_on,
            &self.account_id,
            &self.destination_account_id,
            &self.received_amount,
            &self.received_currency,
            &self.category_id,
            &self.description,
        ]
    }
    fn validate(&mut self) -> Result<(), ApiError> {
        if self.fields().iter().all(|f| matches!(f, Field::Absent)) {
            return Err(ApiError::InvalidRequest);
        }
        for field in [&self.amount, &self.currency, &self.occurred_on] {
            if matches!(field, Field::Null) {
                return Err(ApiError::InvalidRequest);
            }
        }
        for field in [
            &mut self.account_id,
            &mut self.destination_account_id,
            &mut self.category_id,
        ] {
            if let Field::Value(value) = field {
                *value = normalize_uuid(value.clone())?;
            }
        }
        // All supported currencies use hundredths. Canonicalize decimal spelling
        // before hashing, independently of the latest mutable event currency.
        for field in [&mut self.amount, &mut self.received_amount] {
            if let Field::Value(value) = field {
                *value = Money::parse(value, Currency::Sek)
                    .map_err(|_| ApiError::InvalidRequest)?
                    .to_string();
            }
        }
        for field in [&self.currency, &self.received_currency] {
            if let Field::Value(value) = field {
                value
                    .parse::<Currency>()
                    .map_err(|_| ApiError::InvalidRequest)?;
            }
        }
        if let Field::Value(value) = &self.occurred_on {
            parse_date(value)?;
        }
        if let Field::Value(value) = &self.description
            && (value.chars().count() > 1000 || value.chars().any(char::is_control))
        {
            return Err(ApiError::InvalidRequest);
        }
        Ok(())
    }
    fn fingerprint(&self, id: &str, action: &str) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(id.as_bytes());
        hash.update(action.as_bytes());
        for field in self.fields() {
            match field {
                Field::Absent => hash.update([0]),
                Field::Null => hash.update([1]),
                Field::Value(value) => {
                    hash.update([2]);
                    hash.update((value.len() as u64).to_be_bytes());
                    hash.update(value.as_bytes());
                }
            }
        }
        hash.finalize().into()
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Empty {}
#[derive(Serialize)]
struct Acknowledgement {
    id: String,
    state: &'static str,
}

fn parse_date(value: &str) -> Result<BusinessDate, ApiError> {
    BusinessDate::parse(value).map_err(|error| match error {
        BusinessDateError::InvalidDate => ApiError::InvalidBusinessDate,
        BusinessDateError::OutOfRange => ApiError::DateOutOfRange,
        BusinessDateError::FutureDate => ApiError::FutureBusinessDate,
    })
}
fn key(headers: &HeaderMap) -> Result<[u8; 32], ApiError> {
    let mut keys = headers.get_all("idempotency-key").iter();
    let value = keys
        .next()
        .and_then(|key| key.to_str().ok())
        .ok_or(ApiError::InvalidRequest)?;
    if keys.next().is_some()
        || value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        return Err(ApiError::InvalidRequest);
    }
    Ok(Sha256::digest(value.as_bytes()).into())
}

pub(crate) async fn edit(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(request_id): Extension<RequestId>,
    uri: Uri,
    headers: HeaderMap,
    path: Result<Path<String>, PathRejection>,
    payload: Result<Json<Patch>, JsonRejection>,
) -> Response {
    let result = async {
        if uri.query().is_some() {
            return Err(ApiError::InvalidRequest);
        }
        let Path(id) = path.map_err(|_| ApiError::InvalidRequest)?;
        let id = normalize_uuid(id)?;
        let Json(mut patch) = payload.map_err(|error| ApiError::invalid_json(error.status()))?;
        patch.validate()?;
        let key = key(&headers)?;
        mutate(
            pool.as_ref()
                .ok_or(ApiError::TransactionUpdateUnavailable)?,
            &identity,
            &id,
            &key,
            &patch,
            false,
        )
        .await
    }
    .await;
    match result {
        Ok(output) => Json(output).into_response(),
        Err(error) => error.response(&request_id),
    }
}
pub(crate) async fn void(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(request_id): Extension<RequestId>,
    uri: Uri,
    headers: HeaderMap,
    path: Result<Path<String>, PathRejection>,
    payload: Result<Json<Empty>, JsonRejection>,
) -> Response {
    let result = async {
        if uri.query().is_some() {
            return Err(ApiError::InvalidRequest);
        }
        let Path(id) = path.map_err(|_| ApiError::InvalidRequest)?;
        let id = normalize_uuid(id)?;
        let Json(_) = payload.map_err(|error| ApiError::invalid_json(error.status()))?;
        let key = key(&headers)?;
        mutate(
            pool.as_ref()
                .ok_or(ApiError::TransactionUpdateUnavailable)?,
            &identity,
            &id,
            &key,
            &Patch::default(),
            true,
        )
        .await
    }
    .await;
    match result {
        Ok(output) => Json(output).into_response(),
        Err(error) => error.response(&request_id),
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Event {
    kind: String,
    state: String,
    amount: i64,
    currency: String,
    date: String,
    account: Option<String>,
    destination: Option<String>,
    received: Option<i64>,
    received_currency: Option<String>,
    category: Option<String>,
    description: Option<String>,
    quoted_rate: Option<String>,
}
impl Event {
    fn patched(&self, patch: &Patch) -> Result<Self, ApiError> {
        let mut next = self.clone();
        patch.currency.required(&mut next.currency);
        patch.occurred_on.required(&mut next.date);
        patch.account_id.apply(&mut next.account);
        patch.destination_account_id.apply(&mut next.destination);
        patch.received_currency.apply(&mut next.received_currency);
        patch.category_id.apply(&mut next.category);
        patch.description.apply(&mut next.description);
        if let Field::Value(value) = &patch.amount {
            next.amount = Money::parse(
                value,
                next.currency
                    .parse()
                    .map_err(|_| ApiError::InvalidRequest)?,
            )
            .map_err(|_| ApiError::InvalidRequest)?
            .minor_units();
        }
        match &patch.received_amount {
            Field::Absent => {}
            Field::Null => next.received = None,
            Field::Value(value) => {
                let currency = next
                    .received_currency
                    .as_deref()
                    .ok_or(ApiError::InvalidRequest)?
                    .parse()
                    .map_err(|_| ApiError::InvalidRequest)?;
                next.received = Some(
                    Money::parse(value, currency)
                        .map_err(|_| ApiError::InvalidRequest)?
                        .minor_units(),
                );
            }
        }
        let invalid = ApiError::InvalidRequest;
        if next.kind != "opening_balance" && next.amount <= 0 {
            return Err(invalid);
        }
        if matches!(next.kind.as_str(), "opening_balance" | "interest_income")
            && next.account.is_none()
        {
            return Err(invalid);
        }
        if next.kind == "transfer" {
            if next.account.is_none()
                || next.destination.is_none()
                || next.account == next.destination
                || next.received.is_none_or(|value| value <= 0)
                || next.received_currency.is_none()
                || (next.received_currency.as_deref() == Some(&next.currency)
                    && next.received != Some(next.amount))
            {
                return Err(invalid);
            }
        } else if next.destination.is_some()
            || next.received.is_some()
            || next.received_currency.is_some()
        {
            return Err(invalid);
        }
        if matches!(next.kind.as_str(), "transfer" | "opening_balance") && next.category.is_some() {
            return Err(invalid);
        }
        if !matches!(
            next.kind.as_str(),
            "income" | "expense" | "transfer" | "opening_balance" | "interest_income"
        ) {
            return Err(invalid);
        }
        if let Some(value) = &next.quoted_rate {
            let rate = QuotedRate::parse(value).map_err(|_| ApiError::InvalidRequest)?;
            let sent = Money::from_minor_units(
                next.amount,
                next.currency
                    .parse()
                    .map_err(|_| ApiError::InvalidRequest)?,
            )
            .map_err(|_| ApiError::InvalidRequest)?;
            let received = Money::from_minor_units(
                next.received.ok_or(ApiError::InvalidRequest)?,
                next.received_currency
                    .as_deref()
                    .ok_or(ApiError::InvalidRequest)?
                    .parse()
                    .map_err(|_| ApiError::InvalidRequest)?,
            )
            .map_err(|_| ApiError::InvalidRequest)?;
            crate::transfer_quote::TransferQuote::new(rate, sent, received)
                .map_err(|_| ApiError::InvalidRequest)?;
        }
        Ok(next)
    }
    fn changed(&self, next: &Self) -> Vec<&'static str> {
        [
            (self.state != next.state, "state"),
            (self.amount != next.amount, "amount_minor"),
            (self.currency != next.currency, "currency"),
            (self.date != next.date, "occurred_on"),
            (self.account != next.account, "account_id"),
            (
                self.destination != next.destination,
                "destination_account_id",
            ),
            (self.received != next.received, "received_amount_minor"),
            (
                self.received_currency != next.received_currency,
                "received_currency",
            ),
            (self.category != next.category, "category_id"),
            (self.description != next.description, "description"),
        ]
        .into_iter()
        .filter_map(|(changed, name)| changed.then_some(name))
        .collect()
    }
    fn postings(&self) -> Vec<Posting> {
        if self.state == "voided" {
            return vec![];
        }
        let mut postings = vec![];
        if let Some(account) = &self.account {
            postings.push(Posting {
                account: account.clone(),
                amount: if matches!(self.kind.as_str(), "expense" | "transfer") {
                    -self.amount
                } else {
                    self.amount
                },
                currency: self.currency.clone(),
                date: self.date.clone(),
            });
        }
        if let (Some(account), Some(amount), Some(currency)) =
            (&self.destination, self.received, &self.received_currency)
        {
            postings.push(Posting {
                account: account.clone(),
                amount,
                currency: currency.clone(),
                date: self.date.clone(),
            });
        }
        postings
    }
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Posting {
    account: String,
    amount: i64,
    currency: String,
    date: String,
}
fn db(_: sqlx::Error) -> ApiError {
    ApiError::TransactionUpdateUnavailable
}

async fn mutate(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    id: &str,
    key: &[u8; 32],
    patch: &Patch,
    void: bool,
) -> Result<Acknowledgement, ApiError> {
    let captured: DateTime<Utc> = std::time::SystemTime::now().into();
    let action = if void { "void" } else { "edit" };
    let acknowledgement = || Acknowledgement {
        id: id.to_owned(),
        state: if void { "voided" } else { "posted" },
    };
    let fingerprint = patch.fingerprint(id, action);
    let mut tx = pool.begin().await.map_err(db)?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(db)?;
    // Same owner serialization boundary as creation, archive and profile updates.
    let owner: Option<(bool,Option<String>)>=sqlx::query_as("SELECT email_verified_at IS NOT NULL,profile_time_zone FROM users WHERE id=$1::uuid FOR UPDATE").bind(identity.user_id()).fetch_optional(&mut *tx).await.map_err(db)?;
    let (true, zone) = owner.ok_or(ApiError::EmailNotVerified)? else {
        return Err(ApiError::EmailNotVerified);
    };
    let previous: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT request_hash FROM transaction_mutation_keys WHERE user_id=$1::uuid AND key_hash=$2",
    )
    .bind(identity.user_id())
    .bind(key.as_slice())
    .fetch_optional(&mut *tx)
    .await
    .map_err(db)?;
    if let Some(previous) = previous {
        if previous != fingerprint {
            return Err(ApiError::IdempotencyConflict);
        }
        tx.commit().await.map_err(db)?;
        return Ok(acknowledgement());
    }
    let row=sqlx::query("SELECT kind,state,amount_minor,currency,occurred_on::text,account_id::text,destination_account_id::text,received_amount_minor,received_currency,category_id::text,description,quoted_rate FROM transactions WHERE id=$1::uuid AND user_id=$2::uuid FOR UPDATE").bind(id).bind(identity.user_id()).fetch_optional(&mut *tx).await.map_err(db)?.ok_or(ApiError::NotFound)?;
    let current = Event {
        kind: row.try_get("kind").map_err(db)?,
        state: row.try_get("state").map_err(db)?,
        amount: row.try_get("amount_minor").map_err(db)?,
        currency: row.try_get("currency").map_err(db)?,
        date: row.try_get("occurred_on").map_err(db)?,
        account: row.try_get("account_id").map_err(db)?,
        destination: row.try_get("destination_account_id").map_err(db)?,
        received: row.try_get("received_amount_minor").map_err(db)?,
        received_currency: row.try_get("received_currency").map_err(db)?,
        category: row.try_get("category_id").map_err(db)?,
        description: row.try_get("description").map_err(db)?,
        quoted_rate: row.try_get("quoted_rate").map_err(db)?,
    };
    if !void && current.state == "voided" {
        return Err(ApiError::TransactionVoided);
    }
    let mut next = if void {
        current.clone()
    } else {
        current.patched(patch)?
    };
    if void {
        next.state = "voided".into();
    }
    let changed = current.changed(&next);
    if let Field::Value(date) = &patch.occurred_on {
        // Explicit dates are validated; unrelated corrections never revalidate a
        // historical date against a subsequently changed profile zone.
        let zone = parse_zone(zone.as_deref().ok_or(ApiError::ProfileZoneUnconfirmed)?)?;
        let today = today_at(captured, zone).map_err(|_| ApiError::DateOutOfRange)?;
        parse_date(date)?
            .ensure_not_future(today)
            .map_err(|_| ApiError::FutureBusinessDate)?;
    }
    if !changed.is_empty() {
        if let Some(category) = &next.category {
            let kind = if next.kind == "expense" {
                "expense"
            } else {
                "income"
            };
            let compatible: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE id=$1::uuid AND kind=$2)",
            )
            .bind(category)
            .bind(kind)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            if !compatible {
                return Err(ApiError::InvalidRequest);
            }
        }
        let old_postings = current.postings();
        let new_postings = next.postings();
        if old_postings != new_postings {
            correct(
                &mut tx,
                identity.user_id(),
                id,
                &next,
                &old_postings,
                &new_postings,
            )
            .await?;
        }
        sqlx::query("UPDATE transactions SET state=$3,amount_minor=$4,currency=$5,occurred_on=$6::date,account_id=$7::uuid,destination_account_id=$8::uuid,received_amount_minor=$9,received_currency=$10,category_id=$11::uuid,description=$12,updated_at=statement_timestamp() WHERE id=$1::uuid AND user_id=$2::uuid")
            .bind(id).bind(identity.user_id()).bind(&next.state).bind(next.amount).bind(&next.currency).bind(&next.date).bind(&next.account).bind(&next.destination).bind(next.received).bind(&next.received_currency).bind(&next.category).bind(&next.description).execute(&mut *tx).await.map_err(db)?;
        sqlx::query("INSERT INTO transaction_audit(user_id,actor_id,transaction_id,action,changed_fields) VALUES($1::uuid,$1::uuid,$2::uuid,$3,$4)")
            .bind(identity.user_id()).bind(id).bind(action).bind(changed).execute(&mut *tx).await.map_err(db)?;
    }
    sqlx::query("INSERT INTO transaction_mutation_keys(user_id,key_hash,request_hash,transaction_id,action) VALUES($1::uuid,$2,$3,$4::uuid,$5)").bind(identity.user_id()).bind(key.as_slice()).bind(fingerprint.as_slice()).bind(id).bind(action).execute(&mut *tx).await.map_err(db)?;
    tx.commit().await.map_err(db)?;
    Ok(acknowledgement())
}

async fn correct(
    tx: &mut Transaction<'_, Postgres>,
    owner: &str,
    id: &str,
    next: &Event,
    old: &[Posting],
    new: &[Posting],
) -> Result<(), ApiError> {
    let mut accounts: Vec<&str> = old.iter().chain(new).map(|p| p.account.as_str()).collect();
    accounts.sort_unstable();
    accounts.dedup();
    for account in accounts {
        let row: Option<(String,String,bool)>=sqlx::query_as("SELECT account_type,currency,archived_at IS NULL FROM accounts WHERE id=$1::uuid AND user_id=$2::uuid FOR UPDATE").bind(account).bind(owner).fetch_optional(&mut **tx).await.map_err(db)?;
        let (kind, currency, active) = row.ok_or(ApiError::NotFound)?;
        if !active {
            return Err(ApiError::AccountArchived);
        }
        for posting in new.iter().filter(|p| p.account == account) {
            if posting.currency != currency || (next.kind == "income" && kind != "normal") {
                return Err(ApiError::InvalidRequest);
            }
        }
    }
    let live=sqlx::query("SELECT m.id::text,m.account_id::text,m.signed_amount_minor,m.currency,m.occurred_on::text FROM account_movements m WHERE m.transaction_id=$1::uuid AND m.user_id=$2::uuid AND m.reverses_movement_id IS NULL AND NOT EXISTS(SELECT 1 FROM account_movements r WHERE r.reverses_movement_id=m.id) ORDER BY m.id").bind(id).bind(owner).fetch_all(&mut **tx).await.map_err(db)?;
    let mut originals = vec![];
    for row in live {
        let posting = Posting {
            account: row.try_get("account_id").map_err(db)?,
            amount: row.try_get("signed_amount_minor").map_err(db)?,
            currency: row.try_get("currency").map_err(db)?,
            date: row.try_get("occurred_on").map_err(db)?,
        };
        originals.push((row.try_get::<String, _>("id").map_err(db)?, posting));
    }
    let mut actual: Vec<Posting> = originals.iter().map(|(_, p)| p.clone()).collect();
    actual.sort();
    let mut expected = old.to_vec();
    expected.sort();
    // Legacy malformed posting sets are not permission to invent a correction.
    if actual != expected {
        return Err(ApiError::TransactionUpdateUnavailable);
    }
    for (original, posting) in originals {
        append(
            tx,
            owner,
            id,
            &Posting {
                amount: -posting.amount,
                ..posting
            },
            Some(&original),
        )
        .await?;
    }
    for posting in new {
        append(tx, owner, id, posting, None).await?;
    }
    Ok(())
}
async fn append(
    tx: &mut Transaction<'_, Postgres>,
    owner: &str,
    id: &str,
    posting: &Posting,
    reverses: Option<&str>,
) -> Result<(), ApiError> {
    sqlx::query("INSERT INTO account_movements(user_id,transaction_id,account_id,signed_amount_minor,currency,occurred_on,reverses_movement_id) VALUES($1::uuid,$2::uuid,$3::uuid,$4,$5,$6::date,$7::uuid)").bind(owner).bind(id).bind(&posting.account).bind(posting.amount).bind(&posting.currency).bind(&posting.date).bind(reverses).execute(&mut **tx).await.map_err(db)?;
    Ok(())
}
