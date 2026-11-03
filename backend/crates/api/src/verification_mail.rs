use crate::registration_policy::Email;
use reqwest::{Client, StatusCode, header::HeaderValue, redirect::Policy};
use std::{error::Error, fmt, time::Duration};

const RESEND_ENDPOINT: &str = "https://api.resend.com/emails";
const MAX_ATTEMPTS: usize = 3;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const OVERALL_TIMEOUT: Duration = Duration::from_secs(20);
const RETRY_DELAY: Duration = Duration::from_millis(100);
const SUBJECT: &str = "Verify your email address — Cuenora";
const MAX_API_KEY_LENGTH: usize = 512;

/// Safe category for paired mail configuration errors; never includes values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidMailConfiguration;

impl fmt::Display for InvalidMailConfiguration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid email delivery configuration")
    }
}

impl Error for InvalidMailConfiguration {}

/// Safe delivery failure. Provider responses and message content are discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailUnavailable;

impl fmt::Display for MailUnavailable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("email delivery unavailable")
    }
}

impl Error for MailUnavailable {}

/// Validated sender settings. Deliberately has no `Debug` or serialization impl.
pub struct ResendConfig {
    api_key: String,
    from: String,
}

impl ResendConfig {
    pub fn new(api_key: String, from: String) -> Result<Self, InvalidMailConfiguration> {
        if !valid_bearer_token(&api_key) {
            return Err(InvalidMailConfiguration);
        }
        let email = Email::parse(&from).map_err(|_| InvalidMailConfiguration)?;
        Ok(Self {
            api_key,
            from: email.address().to_owned(),
        })
    }
}

/// Fixed-endpoint Resend sender with bounded attempts and opaque safe failures.
pub struct ResendSender {
    config: ResendConfig,
    client: Client,
    overall_timeout: Duration,
    retry_delay: Duration,
    #[cfg(test)]
    endpoint: String,
}

impl ResendSender {
    pub fn new(config: ResendConfig) -> Result<Self, MailUnavailable> {
        let client = build_client(REQUEST_TIMEOUT)?;
        Ok(Self {
            config,
            client,
            overall_timeout: OVERALL_TIMEOUT,
            retry_delay: RETRY_DELAY,
            #[cfg(test)]
            endpoint: RESEND_ENDPOINT.to_owned(),
        })
    }

    /// Send one verification message. Retries reuse exact bytes and provider key.
    pub async fn send(
        &self,
        email: &str,
        token: &str,
        idempotency_key: &str,
    ) -> Result<(), MailUnavailable> {
        let recipient = Email::parse(email).map_err(|_| MailUnavailable)?;
        if !valid_verification_token(token) || !valid_idempotency_key(idempotency_key) {
            return Err(MailUnavailable);
        }
        let content = render_verification(token);
        let payload = serde_json::to_vec(&serde_json::json!({
            "from": &self.config.from,
            "to": [recipient.original()],
            "subject": SUBJECT,
            "html": &content.html,
            "text": &content.text,
        }))
        .map_err(|_| MailUnavailable)?;

        tokio::time::timeout(
            self.overall_timeout,
            self.send_attempts(idempotency_key, payload),
        )
        .await
        .map_err(|_| MailUnavailable)?
    }

    async fn send_attempts(
        &self,
        idempotency_key: &str,
        payload: Vec<u8>,
    ) -> Result<(), MailUnavailable> {
        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .client
                .post(self.endpoint())
                .bearer_auth(&self.config.api_key)
                .header("Idempotency-Key", idempotency_key)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(payload.clone())
                .send()
                .await;

            match response {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response)
                    if response.status() != StatusCode::TOO_MANY_REQUESTS
                        && !response.status().is_server_error() =>
                {
                    return Err(MailUnavailable);
                }
                Ok(_) | Err(_) => {}
            }

            if attempt + 1 < MAX_ATTEMPTS {
                let multiplier = u32::try_from(attempt + 1).unwrap_or(2);
                tokio::time::sleep(self.retry_delay.saturating_mul(multiplier)).await;
            }
        }
        Err(MailUnavailable)
    }

    fn endpoint(&self) -> &str {
        #[cfg(test)]
        {
            &self.endpoint
        }
        #[cfg(not(test))]
        {
            RESEND_ENDPOINT
        }
    }

    #[cfg(test)]
    fn new_for_test(
        config: ResendConfig,
        endpoint: String,
        request_timeout: Duration,
        overall_timeout: Duration,
        retry_delay: Duration,
    ) -> Result<Self, MailUnavailable> {
        Ok(Self {
            config,
            client: build_client(request_timeout)?,
            overall_timeout,
            retry_delay,
            endpoint,
        })
    }
}

fn build_client(timeout: Duration) -> Result<Client, MailUnavailable> {
    Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .connect_timeout(timeout)
        .timeout(timeout)
        .build()
        .map_err(|_| MailUnavailable)
}

fn valid_bearer_token(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_API_KEY_LENGTH || !value.is_ascii() {
        return false;
    }
    let mut has_payload = false;
    let mut padding_started = false;
    for byte in value.bytes() {
        if byte == b'=' {
            padding_started = true;
        } else if byte.is_ascii_alphanumeric() || b"-._~+/".contains(&byte) {
            if padding_started {
                return false;
            }
            has_payload = true;
        } else {
            return false;
        }
    }
    has_payload
}

fn valid_idempotency_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.is_ascii()
        && value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
        && value.parse::<HeaderValue>().is_ok()
}

fn valid_verification_token(value: &str) -> bool {
    value.len() == 68
        && value.starts_with("ev1_")
        && value[4..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

struct VerificationContent {
    html: String,
    text: String,
}

fn render_verification(token: &str) -> VerificationContent {
    VerificationContent {
        html: include_str!("../templates/verification.html")
            .replace("{{TOKEN}}", &escape_html(token)),
        text: include_str!("../templates/verification.txt").replace("{{TOKEN}}", token),
    }
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::{
        InvalidMailConfiguration, MailUnavailable, ResendConfig, ResendSender, render_verification,
    };
    use serde_json::Value;
    use std::time::Duration;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::mpsc,
        task::JoinHandle,
        time::{sleep, timeout},
    };

    const API_KEY: &str = "re_test_secret_do_not_print";
    const EMAIL: &str = "User@Example.test";
    const TOKEN: &str = "ev1_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const IDEMPOTENCY_KEY: &str = "email-verification/test-id";

    fn config() -> ResendConfig {
        ResendConfig::new(API_KEY.to_owned(), "verify@cuenora.test".to_owned()).unwrap()
    }

    #[test]
    fn configuration_accepts_a_bare_sender_and_rejects_unsafe_values() {
        assert!(ResendConfig::new(API_KEY.to_owned(), "verify@cuenora.test".to_owned()).is_ok());

        for invalid_key in ["", "has space", "has\nnewline", "non-ascii-é", "bad\u{7f}"] {
            let error = ResendConfig::new(invalid_key.to_owned(), "verify@cuenora.test".to_owned())
                .err()
                .unwrap();
            assert_eq!(error, InvalidMailConfiguration);
            assert!(!error.to_string().contains("has space"));
            assert!(!error.to_string().contains("non-ascii-é"));
        }
        for invalid_sender in [
            "",
            "Cuenora <verify@cuenora.test>",
            "verify@cuenora.test\r\nBcc:attacker@example.test",
            "üser@cuenora.test",
            "verify@localhost",
        ] {
            assert_eq!(
                ResendConfig::new(API_KEY.to_owned(), invalid_sender.to_owned())
                    .err()
                    .unwrap(),
                InvalidMailConfiguration
            );
        }
    }

    #[test]
    fn verification_message_has_safe_html_and_matching_plain_text() {
        let content = render_verification(TOKEN);
        assert!(content.html.contains("Cuenora"));
        assert!(
            content
                .html
                .contains("Confirm that this email address belongs to you")
        );
        assert!(content.html.contains("24 hours"));
        assert!(content.html.contains(TOKEN));
        assert!(content.text.contains(TOKEN));
        assert!(content.text.contains("same Cuenora account"));
        assert!(content.text.contains("Do not share this token"));
        assert!(!content.html.contains("https://"));
        assert!(!content.html.contains("<script"));
        assert!(!content.html.contains("<img"));
        assert!(!content.html.to_ascii_lowercase().contains("href="));
        assert!(!content.text.contains("https://"));

        let injected = render_verification("<script>alert(1)</script>&\"'");
        assert!(!injected.html.contains("<script>"));
        assert!(
            injected
                .html
                .contains("&lt;script&gt;alert(1)&lt;/script&gt;&amp;&quot;&#39;")
        );
        assert!(injected.text.contains("<script>alert(1)</script>&\"'"));
    }

    #[tokio::test]
    async fn sends_a_bearer_request_with_expected_payload_and_idempotency_key() {
        let (endpoint, mut requests, server) = fake_server(vec![ResponseSpec::ok()]).await;
        let sender = ResendSender::new_for_test(
            config(),
            endpoint,
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::ZERO,
        )
        .unwrap();

        let result = sender.send(EMAIL, TOKEN, IDEMPOTENCY_KEY).await;
        assert!(result.is_ok(), "valid provider response should succeed");
        let request = requests.recv().await.unwrap();
        assert!(String::from_utf8_lossy(&request).starts_with("POST /emails HTTP/1.1"));
        let headers = request_headers(&request);
        assert!(headers.contains("authorization: bearer re_test_secret_do_not_print"));
        assert!(headers.contains("idempotency-key: email-verification/test-id"));
        assert!(headers.contains("content-type: application/json"));

        let payload = request_json(&request);
        assert_eq!(payload["from"], "verify@cuenora.test");
        assert_eq!(payload["to"][0], EMAIL);
        assert_eq!(payload["subject"], "Verify your email address — Cuenora");
        let html = payload["html"].as_str().unwrap();
        let text = payload["text"].as_str().unwrap();
        assert!(html.contains(TOKEN));
        assert!(text.contains(TOKEN));
        assert!(!payload["subject"].as_str().unwrap().contains(TOKEN));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn retries_transient_statuses_with_identical_payload_and_key() {
        let (endpoint, mut requests, server) = fake_server(vec![
            ResponseSpec::status(429),
            ResponseSpec::status(503),
            ResponseSpec::ok(),
        ])
        .await;
        let sender = ResendSender::new_for_test(
            config(),
            endpoint,
            Duration::from_secs(1),
            Duration::from_secs(3),
            Duration::ZERO,
        )
        .unwrap();

        assert!(sender.send(EMAIL, TOKEN, IDEMPOTENCY_KEY).await.is_ok());
        let first = requests.recv().await.unwrap();
        let second = requests.recv().await.unwrap();
        let third = requests.recv().await.unwrap();
        assert!(request_body(&first) == request_body(&second));
        assert!(request_body(&second) == request_body(&third));
        for request in [&first, &second, &third] {
            assert!(
                request_headers(request).contains("idempotency-key: email-verification/test-id")
            );
        }
        server.await.unwrap();
    }

    #[tokio::test]
    async fn does_not_retry_permanent_client_errors() {
        let (endpoint, mut requests, server) = fake_server(vec![ResponseSpec::status(400)]).await;
        let sender = ResendSender::new_for_test(
            config(),
            endpoint,
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::ZERO,
        )
        .unwrap();

        assert_eq!(
            sender.send(EMAIL, TOKEN, IDEMPOTENCY_KEY).await,
            Err(MailUnavailable)
        );
        assert!(requests.recv().await.is_some());
        assert!(matches!(
            timeout(Duration::from_millis(30), requests.recv()).await,
            Err(_) | Ok(None)
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn retries_timeouts_but_stays_within_a_bounded_total_deadline() {
        let (endpoint, mut requests, server) = fake_server(vec![
            ResponseSpec::delayed(200, Duration::from_millis(180)),
            ResponseSpec::delayed(200, Duration::from_millis(180)),
            ResponseSpec::delayed(200, Duration::from_millis(180)),
            ResponseSpec::ok(),
        ])
        .await;
        let sender = ResendSender::new_for_test(
            config(),
            endpoint,
            Duration::from_millis(25),
            Duration::from_secs(2),
            Duration::ZERO,
        )
        .unwrap();
        let start = tokio::time::Instant::now();

        assert_eq!(
            sender.send(EMAIL, TOKEN, IDEMPOTENCY_KEY).await,
            Err(MailUnavailable)
        );
        assert!(start.elapsed() < Duration::from_secs(2));
        for _ in 0..3 {
            assert!(
                timeout(Duration::from_millis(300), requests.recv())
                    .await
                    .is_ok()
            );
        }
        assert!(
            timeout(Duration::from_millis(50), requests.recv())
                .await
                .is_err()
        );
        server.abort();
    }

    #[tokio::test]
    async fn rejects_redirects_without_contacting_the_redirect_target() {
        let target_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target_endpoint = format!("http://{}", target_listener.local_addr().unwrap());
        let (target_tx, mut target_rx) = mpsc::unbounded_channel();
        let target_server = tokio::spawn(async move {
            if let Ok((mut stream, _)) = target_listener.accept().await {
                let request = read_request(&mut stream).await;
                let _ = target_tx.send(request);
            }
        });
        let (endpoint, mut requests, server) =
            fake_server(vec![ResponseSpec::redirect(&target_endpoint)]).await;
        let sender = ResendSender::new_for_test(
            config(),
            endpoint,
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::ZERO,
        )
        .unwrap();

        assert_eq!(
            sender.send(EMAIL, TOKEN, IDEMPOTENCY_KEY).await,
            Err(MailUnavailable)
        );
        assert!(requests.recv().await.is_some());
        assert!(
            timeout(Duration::from_millis(50), target_rx.recv())
                .await
                .is_err()
        );
        server.await.unwrap();
        target_server.abort();
    }

    #[tokio::test]
    async fn rejects_invalid_recipient_or_idempotency_header_without_sending() {
        let sender = ResendSender::new(config()).unwrap();
        for (email, token, key) in [
            ("not-an-email", TOKEN, IDEMPOTENCY_KEY),
            (EMAIL, TOKEN, "bad\nkey"),
            (EMAIL, TOKEN, ""),
        ] {
            assert_eq!(sender.send(email, token, key).await, Err(MailUnavailable));
        }
    }

    struct ResponseSpec {
        status: u16,
        location: Option<String>,
        delay: Duration,
    }

    impl ResponseSpec {
        fn ok() -> Self {
            Self::status(200)
        }

        fn status(status: u16) -> Self {
            Self {
                status,
                location: None,
                delay: Duration::ZERO,
            }
        }

        fn delayed(status: u16, delay: Duration) -> Self {
            Self {
                status,
                location: None,
                delay,
            }
        }

        fn redirect(location: &str) -> Self {
            Self {
                status: 307,
                location: Some(location.to_owned()),
                delay: Duration::ZERO,
            }
        }
    }

    async fn fake_server(
        responses: Vec<ResponseSpec>,
    ) -> (String, mpsc::UnboundedReceiver<Vec<u8>>, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/emails", listener.local_addr().unwrap());
        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let server = tokio::spawn(async move {
            for response in responses {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let request = read_request(&mut stream).await;
                let _ = request_tx.send(request);
                if !response.delay.is_zero() {
                    sleep(response.delay).await;
                }
                let reason = match response.status {
                    200 => "OK",
                    307 => "Temporary Redirect",
                    400 => "Bad Request",
                    429 => "Too Many Requests",
                    503 => "Service Unavailable",
                    _ => "Error",
                };
                let mut headers = format!(
                    "HTTP/1.1 {} {}\r\ncontent-length: 0\r\nconnection: close\r\n",
                    response.status, reason
                );
                if let Some(location) = response.location {
                    headers.push_str(&format!("location: {location}\r\n"));
                }
                headers.push_str("\r\n");
                let _ = stream.write_all(headers.as_bytes()).await;
            }
        });
        (endpoint, request_rx, server)
    }

    async fn read_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0u8; 2048];
        let header_end = loop {
            let read = stream.read(&mut buffer).await.unwrap();
            assert!(read > 0, "fake provider received a complete request");
            request.extend_from_slice(&buffer[..read]);
            if let Some(index) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                break index + 4;
            }
        };
        let header_text = String::from_utf8_lossy(&request[..header_end]);
        let content_length = header_text
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let read = stream.read(&mut buffer).await.unwrap();
            assert!(read > 0, "fake provider received the complete request body");
            request.extend_from_slice(&buffer[..read]);
        }
        request
    }

    fn request_headers(request: &[u8]) -> String {
        String::from_utf8_lossy(request)
            .split("\r\n\r\n")
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
    }

    fn request_body(request: &[u8]) -> &[u8] {
        let start = request
            .windows(4)
            .position(|part| part == b"\r\n\r\n")
            .map(|index| index + 4)
            .unwrap();
        &request[start..]
    }

    fn request_json(request: &[u8]) -> Value {
        serde_json::from_slice(request_body(request)).unwrap()
    }
}
