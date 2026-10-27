//! Route-scoped authentication, separate from financial authorization.

use crate::{api_error::ApiError, request_id};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, header},
    middleware::Next,
    response::Response,
};
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
    // Standalone use also receives a server ID; outer context reuses the extension.
    let Ok(request_id) = request_id::ensure(&mut request) else {
        return ApiError::without_request_id();
    };
    let Some(digest) = access_digest(request.headers()) else {
        return ApiError::InvalidAccessToken {
            missing: !request.headers().contains_key(header::AUTHORIZATION),
        }
        .response(&request_id);
    };
    let Some(pool) = pool else {
        return ApiError::AuthenticationUnavailable.response(&request_id);
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
        Ok(None) => return ApiError::InvalidAccessToken { missing: false }.response(&request_id),
        Err(_) => return ApiError::AuthenticationUnavailable.response(&request_id),
    };
    request.extensions_mut().insert(AuthenticatedIdentity {
        user_id,
        session_id,
    });
    let mut response = next.run(request).await;
    request_id.attach(&mut response);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
