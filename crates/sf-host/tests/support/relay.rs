//! A real relay on localhost with a throwaway self-signed certificate.
use std::net::{Ipv4Addr, SocketAddr};

use sf_engine::netplay::Link;
use sf_host::net::{AdapterProbe, HostNetDriver, NetDriverConfig};
use sf_relay::client::{ClientConfig, ClientEvent, RelayClient, Roots};
use sf_relay::wire::Request;
use sf_relay::RoomCode;
use sf_relay::rooms::Limits;
use sf_relay::service::{RelayService, ServiceConfig};

pub struct TestRelay {
    pub service: RelayService,
    certificate: Vec<u8>,
}

impl TestRelay {
    pub fn start() -> Self {
        Self::start_limited(Limits::default())
    }

    #[allow(dead_code)]
    pub fn start_limited(limits: Limits) -> Self {
        let signed = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_owned()]).unwrap();
        let certificate = signed.cert.der().to_vec();
        let service = RelayService::start(ServiceConfig {
            listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            certificates: vec![certificate.clone()],
            private_key: signed.signing_key.serialize_der(),
            limits,
        })
        .expect("the test relay starts");
        Self { service, certificate }
    }

    pub fn address(&self) -> String {
        self.service.local_addr().to_string()
    }

    /// A localhost driver that trusts only this relay's certificate.
    #[allow(dead_code)]
    pub fn driver(&self) -> HostNetDriver {
        self.driver_trusting(&[self])
    }

    /// A driver set to this relay that trusts only `trusted`' certificates.
    #[allow(dead_code)]
    pub fn driver_trusting(&self, trusted: &[&TestRelay]) -> HostNetDriver {
        self.driver_with(trusted, Some(Link::Ethernet), None)
    }

    /// A driver whose link to this relay is reported as `link`.
    #[allow(dead_code)]
    pub fn driver_linked(&self, link: Link) -> HostNetDriver {
        self.driver_with(&[self], Some(link), None)
    }

    /// A driver that classifies its link to this relay by asking `probe`.
    #[allow(dead_code)]
    pub fn driver_probing(&self, probe: AdapterProbe) -> HostNetDriver {
        self.driver_with(&[self], None, Some(probe))
    }

    fn driver_with(&self, trusted: &[&TestRelay], link: Option<Link>, probe: Option<AdapterProbe>) -> HostNetDriver {
        let defaults = NetDriverConfig::default();
        let mut driver = HostNetDriver::with_config(NetDriverConfig {
            bind_ip: Ipv4Addr::LOCALHOST,
            link_override: link,
            adapter_probe: probe.unwrap_or(defaults.adapter_probe),
            discovery_port: 0,
            discovery_targets: Vec::new(),
            relay_roots: Roots::Only(trusted.iter().map(|relay| relay.certificate.clone()).collect()),
            ..defaults
        });
        driver.set_relay(Some(self.address()));
        driver
    }

    /// Whether a fresh client can join `code` right now. It leaves at once.
    #[allow(dead_code)]
    pub fn can_join(&self, code: &str) -> bool {
        let code = RoomCode::parse(code).expect("an Internet room code");
        let client = RelayClient::open(
            ClientConfig { relay: self.address(), roots: Roots::Only(vec![self.certificate.clone()]) },
            Request::Join(code),
        )
        .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(6);
        while std::time::Instant::now() < deadline {
            match client.poll() {
                Some(ClientEvent::Ready { .. }) => return true,
                Some(_) => return false,
                None => std::thread::sleep(std::time::Duration::from_millis(2)),
            }
        }
        panic!("the relay did not answer a join");
    }
}
