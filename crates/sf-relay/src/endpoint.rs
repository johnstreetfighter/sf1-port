//! Which relay a client uses (#128): the shipped default and the form of a
//! custom `host` or `host:port`.

use std::net::{IpAddr, SocketAddr};

use crate::wire::DEFAULT_PORT;

/// The project relay that official builds use when the player sets none.
/// `None` until the project relay runs at a stable hostname (#130).
pub const DEFAULT_RELAY: Option<&str> = None;

/// The longest relay a player can enter: a full DNS name and a port.
pub const MAX_LEN: usize = 253 + 6;

/// The host and port of `relay`, written as `host`, `host:port`, an IP
/// address or `[v6]:port`. The port defaults to [`DEFAULT_PORT`].
pub fn split(relay: &str) -> Option<(String, u16)> {
    let relay = relay.trim();
    if relay.is_empty() {
        return None;
    }
    if let Ok(address) = relay.parse::<SocketAddr>() {
        return Some((address.ip().to_string(), address.port()));
    }
    match relay.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => Some((host.to_owned(), port.parse().ok()?)),
        _ => Some((relay.trim_matches(['[', ']']).to_owned(), DEFAULT_PORT)),
    }
}

/// A relay a player typed, in the form the settings keep: lower case, with
/// the port only when it was given. `None` unless it is a DNS name or an IP
/// address with an optional port from 1 to 65535.
pub fn normalize(text: &str) -> Option<String> {
    let text = text.trim().to_ascii_lowercase();
    if text.len() > MAX_LEN {
        return None;
    }
    let (host, port) = split(&text)?;
    // Brackets belong to an IPv6 address only.
    let known = match host.parse::<IpAddr>() {
        Ok(address) => address.is_ipv6() || !text.contains(['[', ']']),
        Err(_) => !text.contains(['[', ']']) && dns_name(&host),
    };
    (port != 0 && known).then_some(text)
}

fn dns_name(host: &str) -> bool {
    let label = |label: &str| {
        (1..=63).contains(&label.len())
            && label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    };
    // A last label of digits only would read as a mistyped IPv4 address.
    host.len() <= 253
        && host.split('.').all(label)
        && host.rsplit('.').next().is_some_and(|last| !last.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::{normalize, split};

    #[test]
    fn relay_addresses_take_an_optional_port() {
        assert_eq!(split("relay.example.net"), Some(("relay.example.net".into(), 7443)));
        assert_eq!(split(" relay.example.net:9000 "), Some(("relay.example.net".into(), 9000)));
        assert_eq!(split("127.0.0.1:7443"), Some(("127.0.0.1".into(), 7443)));
        assert_eq!(split("[::1]:7443"), Some(("::1".into(), 7443)));
        assert_eq!(split("relay.example.net:port"), None);
        assert_eq!(split(""), None);
    }

    #[test]
    fn a_typed_relay_is_kept_in_lower_case_with_its_optional_port() {
        assert_eq!(normalize(" RELAY.Example.NET "), Some("relay.example.net".into()));
        assert_eq!(normalize("relay.example.net:9000"), Some("relay.example.net:9000".into()));
        assert_eq!(normalize("my-relay"), Some("my-relay".into()));
        assert_eq!(normalize("203.0.113.9:7443"), Some("203.0.113.9:7443".into()));
        assert_eq!(normalize("[2001:db8::1]:7443"), Some("[2001:db8::1]:7443".into()));
    }

    #[test]
    fn a_malformed_relay_is_refused() {
        for text in [
            "", "   ", "relay..example.net", "-relay.example.net", "relay-.example.net", ".example.net",
            "relay.example.net.", "relay.example.net:", "relay.example.net:0", "relay.example.net:65536",
            "relay.example.net:port", "relay example.net", "relay_host.example.net", "https://relay.example.net",
            "relay.example.net/rooms", "203.0.113", "K7Q-MPX:1:2", "[relay.example.net]",
            "[relay.example.net]:9000", "[203.0.113.9]",
        ] {
            assert_eq!(normalize(text), None, "{text:?}");
        }
        assert_eq!(normalize(&format!("{}.net", "a".repeat(64))), None);
    }
}
