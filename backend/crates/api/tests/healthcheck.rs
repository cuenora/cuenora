use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    process::{Command, Output},
    thread,
    time::{Duration, Instant},
};

fn run(address: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cuenora-healthcheck"))
        .env_clear()
        .env("CUENORA_LISTEN_ADDR", address)
        .env("CUENORA_DATABASE_URL", "DO_NOT_PRINT_DATABASE_SECRET")
        .env("CUENORA_RESEND_API_KEY", "DO_NOT_PRINT_MAIL_SECRET")
        .env("HTTP_PROXY", "http://127.0.0.1:1")
        .env("http_proxy", "http://127.0.0.1:1")
        .env("ALL_PROXY", "http://127.0.0.1:1")
        .output()
        .unwrap()
}

fn server(
    response: &'static str,
    delay: Duration,
    delay_body: bool,
) -> (SocketAddr, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            if let Ok((stream, _)) = listener.accept() {
                break stream;
            }
            assert!(Instant::now() < deadline, "probe did not connect");
            thread::sleep(Duration::from_millis(5));
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = [0; 2048];
        let read = stream.read(&mut request).unwrap();
        assert!(
            String::from_utf8_lossy(&request[..read]).starts_with("GET /v1/health/live HTTP/1.1")
        );
        if delay_body {
            let (headers, body) = response.split_once("\r\n\r\n").unwrap();
            stream
                .write_all(format!("{headers}\r\n\r\n").as_bytes())
                .unwrap();
            thread::sleep(delay);
            let _ = stream.write_all(body.as_bytes());
        } else {
            thread::sleep(delay);
            let _ = stream.write_all(response.as_bytes());
        }
    });
    (address, handle)
}

#[test]
fn wildcard_listener_is_probed_locally_without_dependency_configuration_or_proxy() {
    let (address, server) = server(
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK",
        Duration::ZERO,
        false,
    );
    let result = run(&format!("0.0.0.0:{}", address.port()));
    server.join().unwrap();
    assert!(result.status.success(), "healthcheck failed");
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
}

#[test]
fn wrong_status_body_or_redirect_fails_with_fixed_secret_safe_output() {
    for response in [
        "HTTP/1.1 503 Unavailable\r\nContent-Length: 6\r\n\r\nSECRET",
        "HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nSECRET",
        "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/SECRET\r\nContent-Length: 0\r\n\r\n",
    ] {
        let (address, server) = server(response, Duration::ZERO, false);
        let result = run(&address.to_string());
        server.join().unwrap();
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, b"Health check failed.\n");
    }
}

#[test]
fn invalid_address_and_deadline_fail_without_raw_diagnostics() {
    let refused = run("127.0.0.1:1");
    assert!(!refused.status.success());
    assert_eq!(refused.stderr, b"Health check failed.\n");
    let invalid = run("DO_NOT_PRINT_ADDRESS_SECRET");
    assert!(!invalid.status.success());
    assert_eq!(invalid.stderr, b"Health check failed.\n");
    let (address, server) = server(
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK",
        Duration::from_millis(2500),
        false,
    );
    let start = Instant::now();
    let result = run(&address.to_string());
    assert!(start.elapsed() < Duration::from_secs(3));
    server.join().unwrap();
    assert!(!result.status.success());
    assert_eq!(result.stderr, b"Health check failed.\n");
}

#[test]
fn deadline_includes_reading_the_response_body() {
    let (address, server) = server(
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK",
        Duration::from_millis(2500),
        true,
    );
    let start = Instant::now();
    let result = run(&address.to_string());
    assert!(start.elapsed() < Duration::from_secs(3));
    server.join().unwrap();
    assert!(!result.status.success());
    assert_eq!(result.stderr, b"Health check failed.\n");
}
