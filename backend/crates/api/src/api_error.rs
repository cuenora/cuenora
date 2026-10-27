//! Safe public errors: closed codes and messages, never arbitrary diagnostics.
use crate::request_id::RequestId;
use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub enum ApiError {
    InvalidRequest,
    PayloadTooLarge,
    UnsupportedMediaType,
    InvalidCredentials,
    RegistrationUnavailable,
    LoginUnavailable,
    InvalidRefreshToken,
    RefreshUnavailable,
    InvalidAccessToken { missing: bool },
    AuthenticationUnavailable,
    NotFound,
    MethodNotAllowed,
    RequestIdUnavailable,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'static str,
    message: &'static str,
    details: Option<()>,
    request_id: Option<&'a str>,
}

impl ApiError {
    pub(crate) fn invalid_json(status: StatusCode) -> Self {
        match status {
            StatusCode::PAYLOAD_TOO_LARGE => Self::PayloadTooLarge,
            StatusCode::UNSUPPORTED_MEDIA_TYPE => Self::UnsupportedMediaType,
            _ => Self::InvalidRequest,
        }
    }

    fn parts(self) -> (StatusCode, &'static str, &'static str) {
        use ApiError::*;
        match self {
            InvalidRequest => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "The request is invalid.",
            ),
            PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "invalid_request",
                "The request is invalid.",
            ),
            UnsupportedMediaType => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "invalid_request",
                "The request is invalid.",
            ),
            InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                "The credentials are invalid.",
            ),
            RegistrationUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "registration_unavailable",
                "Registration is temporarily unavailable.",
            ),
            LoginUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "login_unavailable",
                "Login is temporarily unavailable.",
            ),
            InvalidRefreshToken => (
                StatusCode::UNAUTHORIZED,
                "invalid_refresh_token",
                "The refresh token is invalid.",
            ),
            RefreshUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "refresh_unavailable",
                "Refresh is temporarily unavailable.",
            ),
            InvalidAccessToken { .. } => (
                StatusCode::UNAUTHORIZED,
                "invalid_access_token",
                "The access token is invalid.",
            ),
            AuthenticationUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "authentication_unavailable",
                "Authentication is temporarily unavailable.",
            ),
            NotFound => (
                StatusCode::NOT_FOUND,
                "not_found",
                "The resource was not found.",
            ),
            MethodNotAllowed => (
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "The method is not allowed.",
            ),
            RequestIdUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "request_id_unavailable",
                "The request is temporarily unavailable.",
            ),
        }
    }

    pub fn response(self, request_id: &RequestId) -> Response {
        self.render(Some(request_id))
    }

    // Only entropy failure may omit an ID; callers cannot pass arbitrary strings.
    pub(crate) fn without_request_id() -> Response {
        Self::RequestIdUnavailable.render(None)
    }

    fn render(self, request_id: Option<&RequestId>) -> Response {
        let (status, code, message) = self.parts();
        let mut response = (
            status,
            Json(ErrorBody {
                code,
                message,
                details: None,
                request_id: request_id.map(RequestId::as_str),
            }),
        )
            .into_response();
        if let Some(id) = request_id {
            id.attach(&mut response);
        }
        if matches!(
            self,
            Self::InvalidAccessToken { .. }
                | Self::AuthenticationUnavailable
                | Self::RequestIdUnavailable
        ) {
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        }
        if let Self::InvalidAccessToken { missing } = self {
            response.headers_mut().insert(
                header::WWW_AUTHENTICATE,
                HeaderValue::from_static(if missing {
                    "Bearer"
                } else {
                    "Bearer error=\"invalid_token\""
                }),
            );
        }
        response
    }
}
