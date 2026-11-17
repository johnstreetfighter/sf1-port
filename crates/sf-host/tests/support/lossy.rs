//! A faulty UDP link between game transports and a relay on localhost. It
//! adds latency and drops, duplicates and delays datagrams on a fixed
//! schedule, so the QUIC connection and the rollback session above it see a
//! bad Internet path. It can also cut its first client off without a word.
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Counts {
    pub forwarded: AtomicU32,
    pub dropped: AtomicU32,
    pub duplicated: AtomicU32,
    pub delayed: AtomicU32,
}

pub struct LossyLink {
    address: SocketAddr,
    pub counts: Arc<Counts>,
    stop: Arc<AtomicBool>,
    cut_first: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// One datagram waiting to leave, towards the relay or back to its client.
struct Queued {
    due: Instant,
    client: SocketAddr,
    to_relay: bool,
    bytes: Vec<u8>,
}

impl LossyLink {
    /// Stands in front of `relay`. Every datagram takes 10 ms each way;
    /// every 23rd is dropped, every 17th sent twice and every 7th held back
    /// a further 35 ms, which reorders it.
    pub fn start(relay: SocketAddr) -> Self {
        let front = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("the lossy link binds");
        front.set_nonblocking(true).unwrap();
        let address = front.local_addr().unwrap();
        let counts = Arc::new(Counts::default());
        let stop = Arc::new(AtomicBool::new(false));
        let cut_first = Arc::new(AtomicBool::new(false));
        let (tally, stopped, cut) = (counts.clone(), stop.clone(), cut_first.clone());
        let thread = std::thread::spawn(move || {
            let mut upstream: HashMap<SocketAddr, UdpSocket> = HashMap::new();
            let mut queue: Vec<Queued> = Vec::new();
            let mut serial = 0_u32;
            let mut buffer = [0_u8; 2048];
            let mut first: Option<SocketAddr> = None;
            let mut schedule = |queue: &mut Vec<Queued>, client: SocketAddr, to_relay: bool, bytes: &[u8]| {
                serial += 1;
                if serial % 23 == 0 {
                    tally.dropped.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                let held = serial % 7 == 0;
                if held {
                    tally.delayed.fetch_add(1, Ordering::Relaxed);
                }
                let due = Instant::now() + Duration::from_millis(if held { 45 } else { 10 });
                let copies = if serial % 17 == 0 {
                    tally.duplicated.fetch_add(1, Ordering::Relaxed);
                    2
                } else {
                    1
                };
                for _ in 0..copies {
                    queue.push(Queued { due, client, to_relay, bytes: bytes.to_vec() });
                }
            };
            while !stopped.load(Ordering::Relaxed) {
                let mut idle = true;
                while let Ok((count, client)) = front.recv_from(&mut buffer) {
                    idle = false;
                    first.get_or_insert(client);
                    upstream.entry(client).or_insert_with(|| {
                        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("an upstream socket binds");
                        socket.connect(relay).unwrap();
                        socket.set_nonblocking(true).unwrap();
                        socket
                    });
                    schedule(&mut queue, client, true, &buffer[..count]);
                }
                for (client, socket) in &upstream {
                    while let Ok(count) = socket.recv(&mut buffer) {
                        idle = false;
                        schedule(&mut queue, *client, false, &buffer[..count]);
                    }
                }
                let now = Instant::now();
                let mut waiting = Vec::new();
                for item in queue.drain(..) {
                    if item.due > now {
                        waiting.push(item);
                        continue;
                    }
                    if cut.load(Ordering::Relaxed) && first == Some(item.client) {
                        continue;
                    }
                    tally.forwarded.fetch_add(1, Ordering::Relaxed);
                    if item.to_relay {
                        if let Some(socket) = upstream.get(&item.client) {
                            let _ = socket.send(&item.bytes);
                        }
                    } else {
                        let _ = front.send_to(&item.bytes, item.client);
                    }
                }
                queue = waiting;
                if idle {
                    std::thread::sleep(Duration::from_micros(200));
                }
            }
        });
        Self { address, counts, stop, cut_first, thread: Some(thread) }
    }

    /// From now on nothing passes to or from the first client that spoke:
    /// its machine has vanished without closing anything.
    #[allow(dead_code)]
    pub fn cut_first_client(&self) {
        self.cut_first.store(true, Ordering::Relaxed);
    }

    pub fn address(&self) -> String {
        self.address.to_string()
    }
}

impl Drop for LossyLink {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
