//! Global reference reads do not grant financial authority.
use crate::{api_error::ApiError, auth_identity::require_identity, request_id::RequestId};
use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State, rejection::BytesRejection},
    http::Uri,
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Serialize;
use sqlx::PgPool;

pub(crate) fn routes(pool: Option<PgPool>) -> Router {
    Router::new()
        .route("/v1/categories", get(list))
        .route_layer(DefaultBodyLimit::max(8 * 1024))
        .route_layer(middleware::from_fn_with_state(
            pool.clone(),
            require_identity,
        ))
        .with_state(pool)
}

#[derive(Serialize)]
struct Category {
    id: String,
    code: String,
    kind: String,
    name: String,
}

#[derive(Serialize)]
struct Catalog {
    categories: Vec<Category>,
}

async fn catalog(pool: &PgPool) -> Result<Catalog, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION READ ONLY")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL lock_timeout = '2s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    // The immutable, migration-managed catalog has ten entries. Never copy seeds
    // into the HTTP layer or filter by user ownership: categories are global.
    let rows: Vec<(String, String, String, String)> =
        sqlx::query_as("SELECT id::text, code, kind, name FROM categories ORDER BY id")
            .fetch_all(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(Catalog {
        categories: rows
            .into_iter()
            .map(|(id, code, kind, name)| Category {
                id,
                code,
                kind,
                name,
            })
            .collect(),
    })
}

async fn list(
    State(pool): State<Option<PgPool>>,
    Extension(id): Extension<RequestId>,
    uri: Uri,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let body = match body {
        Ok(body) => body,
        Err(error) => return ApiError::invalid_json(error.status()).response(&id),
    };
    if !body.is_empty() || uri.query().is_some_and(|query| !query.is_empty()) {
        return ApiError::InvalidRequest.response(&id);
    }
    let Some(pool) = pool else {
        return ApiError::CategoryListingUnavailable.response(&id);
    };
    match catalog(&pool).await {
        Ok(catalog) => Json(catalog).into_response(),
        Err(_) => ApiError::CategoryListingUnavailable.response(&id),
    }
}
