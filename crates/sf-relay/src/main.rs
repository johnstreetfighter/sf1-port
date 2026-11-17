//! The Internet room relay.
//!
//! ```text
//! sf-relay --listen 0.0.0.0:7443 --cert fullchain.pem --key privkey.pem
//! ```
//!
//! The key must be PKCS #8 (`BEGIN PRIVATE KEY`). The relay logs only its
//! address and its room and connection counts, never codes or payloads.
//!
//! ```text
//! sf-relay --check relay.example.net
//! ```
//!
//! A health check: connect the way the game does, verify the certificate
//! against the public web authorities, and exit 0 if the relay answered.

use std::net::SocketAddr;
use std::process::ExitCode;
use std::time::Duration;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use sf_relay::client::{ClientConfig, Roots};
use sf_relay::rooms::Limits;
use sf_relay::service::{RelayService, ServiceConfig};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("sf-relay: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut listen: SocketAddr = ([0, 0, 0, 0], sf_relay::wire::DEFAULT_PORT).into();
    let (mut cert, mut key) = (None, None);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--listen" => listen = value()?.parse().map_err(|error| format!("--listen: {error}"))?,
            "--cert" => cert = Some(value()?),
            "--key" => key = Some(value()?),
            "--check" => return check(value()?),
            _ => return Err(format!("unknown argument {arg}; use --listen, --cert and --key, or --check")),
        }
    }
    let cert = cert.ok_or("--cert is required")?;
    let key = key.ok_or("--key is required")?;
    let certificates = CertificateDer::pem_file_iter(&cert)
        .and_then(|certs| certs.collect::<Result<Vec<_>, _>>())
        .map_err(|error| format!("{cert}: {error}"))?
        .into_iter()
        .map(|certificate| certificate.to_vec())
        .collect();
    let private_key = match PrivateKeyDer::from_pem_file(&key).map_err(|error| format!("{key}: {error}"))? {
        PrivateKeyDer::Pkcs8(key) => key.secret_pkcs8_der().to_vec(),
        _ => return Err(format!("{key}: the key must be PKCS #8 (BEGIN PRIVATE KEY)")),
    };
    let relay = RelayService::start(ServiceConfig { listen, certificates, private_key, limits: Limits::default() })
        .map_err(|error| format!("cannot start on {listen}: {error}"))?;
    println!("sf-relay listening on {} (UDP)", relay.local_addr());
    loop {
        std::thread::sleep(Duration::from_secs(60));
        println!("rooms {} connections {}", relay.open_rooms(), relay.connections());
    }
}

fn check(relay: String) -> Result<(), String> {
    let address = sf_relay::client::check(&ClientConfig { relay: relay.clone(), roots: Roots::Web })?;
    println!("sf-relay: {} answered at {address} with a trusted certificate", relay.trim());
    Ok(())
}
