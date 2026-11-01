//! Planning changes only: never call financial posting or derive today's date.
use crate::{
    api_error::ApiError, auth_identity::AuthenticatedIdentity, income_http::normalize_uuid,
    recurring_rule_http::key, recurring_rules, request_id::RequestId,
};
use axum::{
    Extension, Json,
    extract::{
        Path, RawQuery, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use cuenora_domain::{BusinessDate, BusinessDateError};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{MapAccess, Visitor, value::MapAccessDeserializer},
};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

// Derived structs also accept positional JSON arrays. Require an object while
// delegating field/duplicate validation to the original derived deserializer.
pub(crate) struct Object<T>(T);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
            type Value = Object<T>;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a JSON object")
            }
            fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
                T::deserialize(MapAccessDeserializer::new(map)).map(Object)
            }
        }
        deserializer.deserialize_map(ObjectVisitor(std::marker::PhantomData))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Skip {}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Postpone {
    due_on: String,
}

#[derive(Clone, Copy)]
enum Action {
    Skip,
    Postpone(BusinessDate),
}
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Skip => "skip",
            Self::Postpone(_) => "postpone",
        }
    }
    fn state(self) -> &'static str {
        match self {
            Self::Skip => "skipped",
            Self::Postpone(_) => "planned",
        }
    }
    fn fingerprint(self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"recurring-occurrence-action-v1:");
        hash.update(self.name().as_bytes());
        if let Self::Postpone(date) = self {
            hash.update(date.to_string().as_bytes());
        }
        hash.finalize().into()
    }
}
#[derive(Serialize)]
struct Output {
    id: String,
    scheduled_on: String,
    due_on: String,
    state: &'static str,
}

pub(crate) async fn skip(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    path: Result<Path<String>, PathRejection>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    payload: Result<Json<Object<Skip>>, JsonRejection>,
) -> Response {
    let result = async {
        let (target, key) = target(path, query, &headers)?;
        let _ = payload.map_err(|e| ApiError::invalid_json(e.status()))?;
        apply(
            &pool.ok_or(ApiError::RecurrenceUnavailable)?,
            &identity,
            target,
            &key,
            Action::Skip,
        )
        .await
    }
    .await;
    reply(result, &id)
}
pub(crate) async fn postpone(
    State(pool): State<Option<PgPool>>,
    Extension(identity): Extension<AuthenticatedIdentity>,
    Extension(id): Extension<RequestId>,
    path: Result<Path<String>, PathRejection>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    payload: Result<Json<Object<Postpone>>, JsonRejection>,
) -> Response {
    let result = async {
        let (target, key) = target(path, query, &headers)?;
        let input = payload.map_err(|e| ApiError::invalid_json(e.status()))?.0.0;
        let date = BusinessDate::parse(&input.due_on).map_err(|error| match error {
            BusinessDateError::OutOfRange => ApiError::DateOutOfRange,
            _ => ApiError::InvalidBusinessDate,
        })?;
        apply(
            &pool.ok_or(ApiError::RecurrenceUnavailable)?,
            &identity,
            target,
            &key,
            Action::Postpone(date),
        )
        .await
    }
    .await;
    reply(result, &id)
}
fn reply(result: Result<Output, ApiError>, id: &RequestId) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => error.response(id),
    }
}
fn target(
    path: Result<Path<String>, PathRejection>,
    query: Option<String>,
    headers: &HeaderMap,
) -> Result<(String, [u8; 32]), ApiError> {
    if query.is_some() {
        return Err(ApiError::InvalidRequest);
    }
    Ok((
        normalize_uuid(path.map_err(|_| ApiError::InvalidRequest)?.0)?,
        key(headers)?,
    ))
}
fn storage(_: sqlx::Error) -> ApiError {
    ApiError::RecurrenceUnavailable
}

async fn apply(
    pool: &PgPool,
    identity: &AuthenticatedIdentity,
    id: String,
    key: &[u8; 32],
    action: Action,
) -> Result<Output, ApiError> {
    // Share owner-first authorization with generation/rules. No rule-state or account gate:
    // paused/ended rules and archived planned accounts retain actionable existing items.
    let mut tx = recurring_rules::begin(pool, identity).await?;
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT state,to_char(scheduled_on,'YYYY-MM-DD'),to_char(due_on,'YYYY-MM-DD')
         FROM recurring_occurrences WHERE id=$1::uuid AND user_id=$2::uuid FOR UPDATE",
    )
    .bind(&id)
    .bind(identity.user_id())
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage)?;
    let (state, scheduled_on, current_due) = row.ok_or(ApiError::NotFound)?;
    let fingerprint = action.fingerprint();
    let prior: Option<(String, Vec<u8>, String)> = sqlx::query_as(
        "SELECT occurrence_id::text,request_hash,to_char(result_due_on,'YYYY-MM-DD')
         FROM recurring_occurrence_action_keys WHERE user_id=$1::uuid AND key_hash=$2",
    )
    .bind(identity.user_id())
    .bind(key.as_slice())
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage)?;
    if let Some((target, previous, due_on)) = prior {
        if target != id || previous != fingerprint {
            return Err(ApiError::IdempotencyConflict);
        }
        tx.commit().await.map_err(storage)?;
        return Ok(Output {
            id,
            scheduled_on,
            due_on,
            state: action.state(),
        });
    }
    if state != "planned" {
        return Err(ApiError::OccurrenceActionConflict);
    }
    let due_on = match action {
        Action::Skip => current_due,
        Action::Postpone(date) => {
            let current =
                BusinessDate::parse(&current_due).map_err(|_| ApiError::RecurrenceUnavailable)?;
            if date <= current {
                return Err(ApiError::InvalidRequest);
            }
            date.to_string()
        }
    };
    sqlx::query(
        "UPDATE recurring_occurrences SET state=$3,due_on=$4::date,updated_at=statement_timestamp()
                 WHERE id=$1::uuid AND user_id=$2::uuid",
    )
    .bind(&id)
    .bind(identity.user_id())
    .bind(action.state())
    .bind(&due_on)
    .execute(&mut *tx)
    .await
    .map_err(storage)?;
    sqlx::query("INSERT INTO recurring_occurrence_action_keys(user_id,key_hash,request_hash,occurrence_id,action,result_due_on)
                 VALUES($1::uuid,$2,$3,$4::uuid,$5,$6::date)")
        .bind(identity.user_id()).bind(key.as_slice()).bind(fingerprint.as_slice()).bind(&id)
        .bind(action.name()).bind(&due_on).execute(&mut *tx).await.map_err(storage)?;
    tx.commit().await.map_err(storage)?;
    Ok(Output {
        id,
        scheduled_on,
        due_on,
        state: action.state(),
    })
}
