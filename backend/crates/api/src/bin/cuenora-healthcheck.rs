//! Local-only process liveness probe; never loads database or mail credentials.
use cuenora_api::config::{AppConfig, InvalidConfiguration};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    process::ExitCode,
    time::Duration,
};

fn address() -> Result<SocketAddr, ()> {
    // Reuse the application's listen validation, but supply defaults for every
    // other setting instead of reading unrelated dependency credentials.
    let config = AppConfig::from_lookup(|name| {
        if name != "CUENORA_LISTEN_ADDR" {
            return Ok(None);
        }
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(_) => Err(InvalidConfiguration(name)),
        }
    })
    .map_err(|_| ())?;
    Ok(local_address(config.listen_addr))
}

fn local_address(mut address: SocketAddr) -> SocketAddr {
    if address.ip().is_unspecified() {
        address.set_ip(match address.ip() {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
        });
    }
    address
}

async fn probe() -> Result<(), ()> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|_| ())?;
    let mut response = client
        .get(format!("http://{}/v1/health/live", address()?))
        .send()
        .await
        .map_err(|_| ())?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(());
    }
    let mut body = Vec::with_capacity(2);
    while let Some(chunk) = response.chunk().await.map_err(|_| ())? {
        if body.len() + chunk.len() > 2 {
            return Err(());
        }
        body.extend_from_slice(&chunk);
    }
    if body == b"OK" { Ok(()) } else { Err(()) }
}

#[tokio::main]
async fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() == 1 && arguments[0] == "--help" {
        println!(
            "cuenora-healthcheck: check local /v1/health/live using CUENORA_LISTEN_ADDR; no dependency checks or credentials."
        );
        return ExitCode::SUCCESS;
    }
    if arguments.is_empty() && probe().await.is_ok() {
        return ExitCode::SUCCESS;
    }
    eprintln!("Health check failed.");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::local_address;

    #[test]
    fn wildcard_addresses_map_to_the_corresponding_loopback() {
        assert_eq!(
            local_address("0.0.0.0:3000".parse().unwrap()),
            "127.0.0.1:3000".parse().unwrap()
        );
        assert_eq!(
            local_address("[::]:3000".parse().unwrap()),
            "[::1]:3000".parse().unwrap()
        );
        assert_eq!(
            local_address("127.0.0.2:9000".parse().unwrap()),
            "127.0.0.2:9000".parse().unwrap()
        );
    }
}
