//! Optional HTTP(S) room directory work, isolated from the frame thread.

use std::io;
use std::net::SocketAddrV4;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::time::Duration;

use sf_engine::netplay::{Room, RoomAnnouncement};

/// A bounded unit of optional directory work. The caller owns epoch policy.
pub struct DirectoryJob {
    pub epoch: u64,
    pub url: String,
    pub publish: Option<(u16, RoomAnnouncement)>,
    pub withdraw_port: Option<u16>,
    pub fetch: bool,
}

/// A completed request, including the caller's unchanged epoch.
pub struct DirectoryResult {
    pub epoch: u64,
    pub result: Result<Vec<Room>, String>,
}

/// One HTTP worker with one queued job and one queued result at most.
///
/// Dropping this handle never joins an in-flight request on the frame thread.
/// That request remains bounded by the worker's HTTP timeout.
pub struct DirectoryWorker {
    jobs: SyncSender<DirectoryJob>,
    results: Receiver<DirectoryResult>,
}

impl DirectoryWorker {
    pub fn new(timeout: Duration) -> io::Result<Self> {
        let timeout = timeout.clamp(Duration::from_millis(1), Duration::from_secs(5));
        let (jobs, pending) = mpsc::sync_channel::<DirectoryJob>(1);
        let (completed, results) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("sf-directory".into())
            .spawn(move || {
                let config = ureq::Agent::config_builder()
                    .timeout_global(Some(timeout))
                    .timeout_connect(Some(timeout))
                    .timeout_resolve(Some(timeout))
                    .timeout_recv_body(Some(timeout))
                    .max_redirects(0)
                    .max_response_header_size(8192)
                    .max_idle_connections(1)
                    .max_idle_connections_per_host(1)
                    .proxy(None)
                    .build();
                let agent: ureq::Agent = config.into();
                while let Ok(job) = pending.recv() {
                    let result = execute(&agent, &job);
                    if completed
                        .send(DirectoryResult {
                            epoch: job.epoch,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self { jobs, results })
    }

    /// Queue work without waiting for HTTP, DNS, or an occupied worker.
    pub fn try_submit(&self, job: DirectoryJob) -> Result<(), DirectoryJob> {
        self.jobs.try_send(job).map_err(|error| match error {
            TrySendError::Full(job) | TrySendError::Disconnected(job) => job,
        })
    }

    pub fn try_result(&self) -> Option<DirectoryResult> {
        self.results.try_recv().ok()
    }
}

fn execute(agent: &ureq::Agent, job: &DirectoryJob) -> Result<Vec<Room>, String> {
    validate_url(&job.url)?;
    if let Some(port) = job.withdraw_port {
        if port == 0 {
            return Err("Directory withdrawal port is invalid".into());
        }
        let payload = serde_json::json!({"version": 1, "port": port, "beacon": null}).to_string();
        let response = agent
            .post(&job.url)
            .header("Content-Type", "application/json")
            .send(payload.as_str())
            .map_err(|_| "Directory withdrawal failed")?;
        response_body(response)?;
    }
    if let Some((port, room)) = &job.publish {
        if room.public {
            let beacon = super::encode_room_announcement(room, *port)
                .ok_or("Directory announcement is invalid")?;
            let beacon: String = beacon.iter().map(|byte| format!("{byte:02x}")).collect();
            let payload =
                serde_json::json!({"version": 1, "port": port, "beacon": beacon}).to_string();
            let response = agent
                .post(&job.url)
                .header("Content-Type", "application/json")
                .send(payload.as_str())
                .map_err(|_| "Directory publication failed")?;
            response_body(response)?;
        }
    }
    if !job.fetch {
        return Ok(Vec::new());
    }
    let response = agent
        .get(&job.url)
        .call()
        .map_err(|_| "Directory request failed")?;
    let bytes = response_body(response)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "Directory response is invalid JSON")?;
    if value.get("version").and_then(|version| version.as_u64()) != Some(1) {
        return Err("Directory version is unsupported".into());
    }
    let rows = value
        .get("rooms")
        .and_then(|rooms| rooms.as_array())
        .ok_or("Directory room list is invalid")?;
    if rows.len() > 128 {
        return Err("Directory room list exceeds 128 rows".into());
    }
    let mut rooms = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(address) = row
            .get("address")
            .and_then(|address| address.as_str())
            .and_then(|address| address.parse::<SocketAddrV4>().ok())
        else {
            continue;
        };
        let first_octet = address.ip().octets()[0];
        if first_octet == 0 || first_octet >= 224 || address.port() == 0 {
            continue;
        }
        let Some(beacon) = row
            .get("beacon")
            .and_then(|beacon| beacon.as_str())
            .and_then(decode_hex_beacon)
        else {
            continue;
        };
        if u16::from_le_bytes([beacon[6], beacon[7]]) != address.port() {
            continue;
        }
        if let Some(room) = super::decode_room_announcement(&beacon, *address.ip()) {
            rooms.push(room);
        }
    }
    Ok(rooms)
}

fn validate_url(value: &str) -> Result<(), String> {
    let invalid =
        || "Directory URL must be bounded HTTP(S) without credentials or fragments".to_string();
    if value.len() > 2048
        || value.contains('#')
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(invalid());
    }
    let uri: ureq::http::Uri = value.parse().map_err(|_| invalid())?;
    let scheme = uri.scheme_str().ok_or_else(invalid)?;
    let authority = uri.authority().ok_or_else(invalid)?;
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        || authority.as_str().contains('@')
        || uri.host().is_none_or(str::is_empty)
        || uri.port_u16() == Some(0)
    {
        return Err(invalid());
    }
    Ok(())
}

fn response_body(mut response: ureq::http::Response<ureq::Body>) -> Result<Vec<u8>, String> {
    if !response.status().is_success() {
        return Err("Directory response status is invalid".into());
    }
    response
        .body_mut()
        .with_config()
        .limit(64 * 1024)
        .read_to_vec()
        .map_err(|_| "Directory response exceeded its body or time limit".into())
}

fn decode_hex_beacon(value: &str) -> Option<[u8; 56]> {
    if value.len() != 112 {
        return None;
    }
    let mut beacon = [0; 56];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = (pair[0] as char).to_digit(16)? as u8;
        let low = (pair[1] as char).to_digit(16)? as u8;
        beacon[index] = (high << 4) | low;
    }
    Some(beacon)
}
