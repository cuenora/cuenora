//! Route-scoped authentication, separate from financial authorization.

use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

/// Server-derived identity; fields cannot be supplied by downstream callers.
#[derive(Clone)]
pub struct AuthenticatedIdentity {
    user_id: String,
    session_id: String,
}

impl AuthenticatedIdentity {
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

fn access_digest(headers: &HeaderMap) -> Option<[u8; 32]> {
    let mut values = headers.get_all(header::AUTHORIZATION).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    let token = token.trim_start_matches(' ');
    let secret = token.strip_prefix("ca1_")?;
    if secret.len() != 64
        || !secret
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    Some(Sha256::digest(token.as_bytes()).into())
}

#[derive(Serialize)]
struct AuthenticationError {
    error: &'static str,
}

fn failure(status: StatusCode, error: &'static str, challenge: Option<&'static str>) -> Response {
    let mut response = (status, Json(AuthenticationError { error })).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Some(challenge) = challenge {
        response.headers_mut().insert(
            header::WWW_AUTHENTICATE,
            HeaderValue::from_static(challenge),
        );
    }
    response
}

/// Apply with `from_fn_with_state(pool, require_identity)` to protected routes
/// only. Handlers extract `Extension<AuthenticatedIdentity>`. This proves identity,
/// not email verification, ownership or financial permission. No session is renewed.
/// Revocation committed before the lookup snapshot is enforced; already admitted
/// requests are not cancelled by a later revocation. Never cache this lookup.
///
/// ```
/// use axum::{Extension, Router, middleware, routing::get};
/// use cuenora_api::auth_identity::{AuthenticatedIdentity, require_identity};
///
/// fn protected_routes(pool: Option<sqlx::PgPool>) -> Router {
///     Router::new()
///         .route("/protected", get(|Extension(id): Extension<AuthenticatedIdentity>| async move {
///             id.user_id().to_owned()
///         }))
///         .route_layer(middleware::from_fn_with_state(pool, require_identity))
/// }
/// ```
pub async fn require_identity(
    State(pool): State<Option<PgPool>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(digest) = access_digest(request.headers()) else {
        let challenge = if request.headers().contains_key(header::AUTHORIZATION) {
            "Bearer error=\"invalid_token\""
        } else {
            "Bearer"
        };
        return failure(
            StatusCode::UNAUTHORIZED,
            "invalid_access_token",
            Some(challenge),
        );
    };
    let Some(pool) = pool else {
        return failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "authentication_unavailable",
            None,
        );
    };
    // One read-only statement checks token and family in the same snapshot. Only
    // the complete credential digest enters SQL; request-supplied IDs never do.
    let identity: Result<Option<(String, String)>, sqlx::Error> = sqlx::query_as(
        "SELECT a.user_id::text, a.session_id::text
         FROM session_access_tokens a
         JOIN user_sessions s ON s.id=a.session_id AND s.user_id=a.user_id
         WHERE a.token_hash=$1 AND a.expires_at > statement_timestamp()
           AND s.revoked_at IS NULL",
    )
    .bind(digest.as_slice())
    .fetch_optional(&pool)
    .await;
    let (user_id, session_id) = match identity {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            return failure(
                StatusCode::UNAUTHORIZED,
                "invalid_access_token",
                Some("Bearer error=\"invalid_token\""),
            );
        }
        Err(_) => {
            return failure(
                StatusCode::SERVICE_UNAVAILABLE,
                "authentication_unavailable",
                None,
            );
        }
    };
    request.extensions_mut().insert(AuthenticatedIdentity {
        user_id,
        session_id,
    });
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
