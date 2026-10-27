//! Assert dynamic IDs before normalizing only that field for exact comparisons.
use axum::http::HeaderMap;
use serde_json::{Value, json};

pub fn checked_body(headers: &HeaderMap, body: String) -> String {
    let id = headers["x-request-id"].to_str().unwrap();
    assert_eq!(headers.get_all("x-request-id").iter().count(), 1);
    assert_eq!(id.len(), 32);
    assert!(
        id.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    if let Ok(mut value) = serde_json::from_str::<Value>(&body)
        && value.get("code").is_some()
    {
        assert_eq!(value["request_id"], id);
        value["request_id"] = json!("<request-id>");
        return value.to_string();
    }
    body
}

pub fn error_body(code: &str) -> String {
    let message = match code {
        "invalid_request" => "The request is invalid.",
        "invalid_credentials" => "The credentials are invalid.",
        "registration_unavailable" => "Registration is temporarily unavailable.",
        "login_unavailable" => "Login is temporarily unavailable.",
        "invalid_refresh_token" => "The refresh token is invalid.",
        "refresh_unavailable" => "Refresh is temporarily unavailable.",
        "invalid_access_token" => "The access token is invalid.",
        "authentication_unavailable" => "Authentication is temporarily unavailable.",
        _ => panic!("unexpected auth error code"),
    };
    json!({"code":code, "message":message, "details":null, "request_id":"<request-id>"}).to_string()
}
