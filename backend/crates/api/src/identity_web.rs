//! First-party identity pages.
use axum::{
    Router,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
    routing::get,
};

pub(crate) fn routes() -> Router {
    Router::new()
        .route(
            "/auth/assets/recovery.js",
            get(|| async {
                asset(
                    "text/javascript; charset=utf-8",
                    include_str!("identity_web/recovery.js"),
                )
            }),
        )
        .route(
            "/auth/assets/fonts/bodoni-moda-license.txt",
            get(|| async {
                asset(
                    "text/plain; charset=utf-8",
                    include_str!("identity_web/fonts/BodoniModa-OFL.txt"),
                )
            }),
        )
        .route(
            "/auth/assets/fonts/ibm-plex-mono-license.txt",
            get(|| async {
                asset(
                    "text/plain; charset=utf-8",
                    include_str!("identity_web/fonts/IBMPlexMono-OFL.txt"),
                )
            }),
        )
        .route(
            "/auth/assets/fonts/bodoni-moda.ttf",
            get(|| async {
                asset(
                    "font/ttf",
                    include_bytes!("identity_web/fonts/bodoni-moda.ttf").as_slice(),
                )
            }),
        )
        .route(
            "/auth/assets/fonts/ibm-plex-mono-regular.ttf",
            get(|| async {
                asset(
                    "font/ttf",
                    include_bytes!("identity_web/fonts/ibm-plex-mono-regular.ttf").as_slice(),
                )
            }),
        )
        .route(
            "/auth/assets/fonts/ibm-plex-mono-medium.ttf",
            get(|| async {
                asset(
                    "font/ttf",
                    include_bytes!("identity_web/fonts/ibm-plex-mono-medium.ttf").as_slice(),
                )
            }),
        )
        .route(
            "/auth/assets/logo.svg",
            get(|| async { asset("image/svg+xml", include_str!("identity_web/logo.svg")) }),
        )
        .route(
            "/auth/reset-password",
            get(|| async { recovery_page(true) }),
        )
        .route(
            "/auth/assets/login.js",
            get(|| async {
                asset(
                    "text/javascript; charset=utf-8",
                    include_str!("identity_web/login.js"),
                )
            }),
        )
        .route("/auth/verify-email", get(|| async { login_page() }))
        .route("/auth/sign-in", get(|| async { login_page() }))
        .route(
            "/auth/forgot-password",
            get(|| async { recovery_page(false) }),
        )
        .route("/auth/preview", get(|| async { page("preview") }))
        .route(
            "/auth/assets/identity.css",
            get(|| async {
                asset(
                    "text/css; charset=utf-8",
                    include_str!("identity_web/identity.css"),
                )
            }),
        )
        .route(
            "/auth/assets/identity.js",
            get(|| async {
                asset(
                    "text/javascript; charset=utf-8",
                    include_str!("identity_web/identity.js"),
                )
            }),
        )
}

fn login_page() -> Response {
    let mut response = asset(
        "text/html; charset=utf-8",
        include_str!("identity_web/login.html"),
    );
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; style-src 'self'; script-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self'; form-action 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );
    response
}

fn asset(content_type: &'static str, body: impl IntoResponse) -> Response {
    let mut response = body.into_response();
    let headers = response.headers_mut();
    for (key, value) in [
        (header::CONTENT_TYPE, content_type),
        (header::CACHE_CONTROL, "no-store"),
        (header::REFERRER_POLICY, "no-referrer"),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (
            header::CONTENT_SECURITY_POLICY,
            "default-src 'none'; style-src 'self'; script-src 'self'; img-src 'self'; font-src 'self'; connect-src 'none'; form-action 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    ] {
        headers.insert(key, HeaderValue::from_static(value));
    }
    response
}

fn field(label: &str, id: &str, password: bool, helper: &str) -> String {
    let kind = if password { "password" } else { "email" };
    let toggle = if password {
        format!(
            "<button class=\"toggle\" type=\"button\" data-toggle=\"{id}\" aria-controls=\"{id}\" aria-pressed=\"false\" disabled>Show</button>"
        )
    } else {
        String::new()
    };
    format!(
        "<div class=\"field\"><label for=\"{id}\">{label}</label><div class=\"input-row\"><input id=\"{id}\" type=\"{kind}\" autocomplete=\"off\" aria-describedby=\"{id}-help\" disabled>{toggle}</div><small id=\"{id}-help\">{helper}</small></div>"
    )
}

fn page(kind: &str) -> Response {
    let (title, purpose, description, fields, action) = match kind {
        "reset" => ("Choose a new password", "PASSWORD RECOVERY", "Use a unique password you do not use for another account.", format!("{}{}<p class=\"notice\">Changing your password will sign you out on every device. You will need to sign in again.</p>", field("New password", "new-password", true, "15–128 characters. Spaces and Unicode are welcome."), field("Confirm password", "confirm-password", true, "Enter your new password again.")), "Change password"),
        "verify" => ("Verify your email", "EMAIL VERIFICATION", "Confirm that this email address belongs to your Cuenora account.", "<p class=\"notice\">Verification will require a signed-in session of the same account that received the email.</p><p>Your email will be verified only after an explicit confirmation. Opening this page does not verify it.</p>".into(), "Verify email"),
        "login" => ("Sign in to verify", "EMAIL VERIFICATION", "Sign in to the Cuenora account that received this email. Then confirm verification.", format!("{}{}<p class=\"notice\">Your app session is not automatically shared with this browser.</p>", field("Email address", "email", false, "Use the account that received the verification email."), field("Password", "password", true, "Your existing Cuenora password.")), "Sign in and continue"),
        "request" => ("Forgot your password?", "PASSWORD RECOVERY", "Enter your account email to request instructions for choosing a new password.", field("Email address", "email", false, "No email will be sent in this preview."), "Send recovery email"),
        _ => ("Identity screen previews", "DESIGN REVIEW", "Visual examples only. None of these examples represents your account status.", examples(), "Not connected"),
    };
    let html = include_str!("identity_web/page.html")
        .replace("{{title}}", title)
        .replace("{{purpose}}", purpose)
        .replace("{{description}}", description)
        .replace("{{fields}}", &fields)
        .replace("{{action}}", action);
    asset("text/html; charset=utf-8", html)
}

fn examples() -> String {
    [
        ("Check your inbox", "If an eligible account matches that address, we will send recovery instructions. Recovery links expire after 30 minutes."),
        ("Password changed", "Success example: signed out on every device; normal sign-in required. Email verification is unchanged."),
        ("Email verified", "Success example: return to Cuenora to continue. No automatic password change or sign-in."),
        ("This link cannot be used", "It may have expired or already been used. Request a fresh link; verification resend requires sign-in."),
        ("Use the matching account", "Verification requires the account that received the email. Do not reveal another account's address."),
        ("Passwords do not match", "Enter the same password in both fields. Common passwords are not accepted."),
        ("Please try again later", "Service-unavailable example, only when submission is known not to have happened."),
        ("We could not confirm the result", "Recovery may have completed. Try ordinary sign-in with the new password, or request a fresh recovery email."),
        ("Verification outcome unknown", "Check authenticated verification status before attempting confirmation again."),
        ("Request a fresh email", "Signed-in verification resend example. No email is sent by this preview."),
        ("Changing password… / Verifying email…", "Busy examples: prevent duplicate activation. Do not display success before confirmation."),
        ("Too many attempts", "Wait before trying again. Display a retry time only when the server supplies one."),
    ].into_iter().map(|(title, body)| format!("<article class=\"state\"><span class=\"eyebrow\">STATIC EXAMPLE</span><h2>{title}</h2><p>{body}</p></article>")).collect()
}

fn recovery_page(reset: bool) -> Response {
    let (title, description, mode, action, endpoint, fields) = if reset {
        (
            "Choose a new password",
            "Use a unique password you do not use for another account.",
            "reset",
            "Change password",
            "/v1/auth/password-recovery/confirm",
            format!(
                "{}{}<p class=\"notice\">Common passwords are not accepted. Changing your password signs you out on every device. Email verification is unchanged.</p>",
                recovery_field(
                    "New password",
                    "new-password",
                    "password",
                    "new-password",
                    "15–128 characters. Spaces and Unicode are welcome."
                ),
                recovery_field(
                    "Confirm password",
                    "confirm-password",
                    "password",
                    "new-password",
                    "Enter your new password again."
                )
            ),
        )
    } else {
        (
            "Forgot your password?",
            "Enter your account email to request instructions for choosing a new password.",
            "request",
            "Send recovery email",
            "/v1/auth/password-recovery/request",
            recovery_field(
                "Email address",
                "email",
                "email",
                "email",
                "We will not reveal whether an account exists.",
            ),
        )
    };
    let html = include_str!("identity_web/recovery.html")
        .replace("{{title}}", title)
        .replace("{{description}}", description)
        .replace("{{mode}}", mode)
        .replace("{{action}}", action)
        .replace("{{endpoint}}", endpoint)
        .replace("{{fields}}", &fields);
    let mut response = asset("text/html; charset=utf-8", html);
    response.headers_mut().insert(header::CONTENT_SECURITY_POLICY, HeaderValue::from_static(
        "default-src 'none'; style-src 'self'; script-src 'self'; img-src 'self'; font-src 'self'; connect-src 'self'; form-action 'none'; base-uri 'none'; frame-ancestors 'none'"));
    response
}

fn recovery_field(label: &str, id: &str, kind: &str, autocomplete: &str, helper: &str) -> String {
    let toggle = if kind == "password" {
        format!(
            "<button class=\"toggle\" type=\"button\" data-toggle=\"{id}\" aria-controls=\"{id}\" aria-pressed=\"false\" aria-label=\"Show {label}\" disabled>Show</button>"
        )
    } else {
        String::new()
    };
    format!(
        "<div class=\"field\"><label for=\"{id}\">{label}</label><div class=\"input-row\"><input id=\"{id}\" type=\"{kind}\" autocomplete=\"{autocomplete}\" aria-describedby=\"{id}-help\" aria-errormessage=\"status\" required disabled>{toggle}</div><small id=\"{id}-help\">{helper}</small></div>"
    )
}
