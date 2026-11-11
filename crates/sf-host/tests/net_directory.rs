use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use sf_engine::config::RulesetPreset;
use sf_engine::netplay::{Link, RoomAnnouncement, RoomKind};
use sf_host::net::directory::{DirectoryJob, DirectoryWorker};

#[test]
fn an_optional_directory_fetch_runs_on_a_worker_and_preserves_the_callers_epoch() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            if let Ok((stream, _)) = listener.accept() {
                break stream;
            }
            assert!(Instant::now() < until, "no localhost directory request");
            std::thread::sleep(Duration::from_millis(1));
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
            assert!(request.len() < 8192);
        }
        assert!(request.starts_with(b"GET /rooms HTTP/1.1\r\n"));
        let body = br#"{"version":1,"rooms":[]}"#;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
        stream.write_all(body).unwrap();
    });
    let worker = DirectoryWorker::new(Duration::from_secs(1)).unwrap();
    let started = Instant::now();
    assert!(worker
        .try_submit(DirectoryJob {
            epoch: 7,
            url: format!("http://{address}/rooms"),
            publish: None,
            withdraw_port: None,
            fetch: true
        })
        .is_ok());
    assert!(started.elapsed() < Duration::from_millis(100));
    let until = Instant::now() + Duration::from_secs(3);
    let result = loop {
        if let Some(result) = worker.try_result() {
            break result;
        }
        assert!(Instant::now() < until, "no directory worker response");
        std::thread::sleep(Duration::from_millis(1));
    };
    assert_eq!(result.epoch, 7);
    assert!(result.result.unwrap().is_empty());
    server.join().unwrap();
}

const BEACON: &str = concat!(
    "534631440102581b484f53540000000000000000",
    "0000010100000008",
    "08070605040302011817161514131211f8f7f6f5f4f3f2f1a4a3a2a1",
);

fn public_room() -> RoomAnnouncement {
    RoomAnnouncement {
        host: *b"HOST\0\0\0\0\0\0\0\0",
        kind: RoomKind::Versus,
        link: Link::Ethernet,
        wired_only: true,
        public: true,
        preset: RulesetPreset::DefinitiveOverhaul,
        in_match: false,
        spectators: 0,
        max_rollback: 8,
        config_hash: 0x0102_0304_0506_0708,
        bank_hash: 0x1112_1314_1516_1718,
        art_pack_hash: 0xf1f2_f3f4_f5f6_f7f8,
        protocol_hash: 0xa1a2_a3a4,
    }
}

fn accept_request(listener: &TcpListener) -> (TcpStream, String, Vec<u8>) {
    let until = Instant::now() + Duration::from_secs(3);
    let mut stream = loop {
        if let Ok((stream, _)) = listener.accept() {
            break stream;
        }
        assert!(Instant::now() < until, "no localhost directory request");
        std::thread::sleep(Duration::from_millis(1));
    };
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut header = Vec::new();
    while !header.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
        assert!(header.len() < 8192);
    }
    let header = String::from_utf8(header).unwrap();
    let content_length = header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    assert!(content_length < 8192);
    let mut body = vec![0; content_length];
    stream.read_exact(&mut body).unwrap();
    (stream, header, body)
}

fn respond(stream: &mut TcpStream, body: &[u8]) {
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
    stream.write_all(body).unwrap();
}

fn result(worker: &DirectoryWorker) -> sf_host::net::directory::DirectoryResult {
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(result) = worker.try_result() {
            return result;
        }
        assert!(Instant::now() < until, "no directory worker response");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn public_room_publication_and_fetch_preserve_literal_beacon_hashes_and_use_the_listed_source_address(
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut post, header, body) = accept_request(&listener);
        assert!(header.starts_with("POST /rooms HTTP/1.1\r\n"));
        assert!(header
            .lines()
            .any(|line| line.eq_ignore_ascii_case("content-type: application/json")));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({"version": 1, "port": 7000, "beacon": BEACON})
        );
        respond(&mut post, b"{}");
        let (mut get, header, body) = accept_request(&listener);
        assert!(header.starts_with("GET /rooms HTTP/1.1\r\n"));
        assert!(body.is_empty());
        let listing = serde_json::json!({"version": 1, "rooms": [{"address": "127.0.0.2:7000", "beacon": BEACON}]}).to_string();
        respond(&mut get, listing.as_bytes());
    });
    let worker = DirectoryWorker::new(Duration::from_secs(1)).unwrap();
    assert!(worker
        .try_submit(DirectoryJob {
            epoch: 19,
            url: format!("http://{address}/rooms"),
            publish: Some((7000, public_room())),
            withdraw_port: None,
            fetch: true
        })
        .is_ok());
    let completed = result(&worker);
    assert_eq!(completed.epoch, 19);
    let rooms = completed.result.unwrap();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0].code, "SF1-7F00-0002");
    assert_eq!(rooms[0].host, "HOST");
    assert_eq!(rooms[0].link, Link::Ethernet);
    assert_eq!(rooms[0].rollback_frames, 8);
    assert_eq!(rooms[0].preset, RulesetPreset::DefinitiveOverhaul);
    server.join().unwrap();
}

#[test]
fn closing_a_public_room_posts_a_null_beacon_and_a_private_room_is_never_published() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let requests = listener.try_clone().unwrap();
    let server = std::thread::spawn(move || {
        let (mut post, header, body) = accept_request(&requests);
        assert!(header.starts_with("POST /rooms HTTP/1.1\r\n"));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({"version": 1, "port": 7000, "beacon": null})
        );
        respond(&mut post, b"{}");
    });
    let worker = DirectoryWorker::new(Duration::from_millis(200)).unwrap();
    let mut private = public_room();
    private.public = false;
    assert!(worker
        .try_submit(DirectoryJob {
            epoch: 20,
            url: format!("http://{address}/rooms"),
            publish: Some((7001, private)),
            withdraw_port: Some(7000),
            fetch: false
        })
        .is_ok());
    assert!(result(&worker).result.unwrap().is_empty());
    server.join().unwrap();
    assert!(
        listener.accept().is_err(),
        "a private room must not send a publication request"
    );
}

#[test]
fn untrusted_directory_urls_are_rejected_before_network_contact_and_diagnostics_hide_credentials() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let worker = DirectoryWorker::new(Duration::from_millis(50)).unwrap();
    let urls = [
        format!("http://{address}/{}", "a".repeat(2049)),
        format!("http://user:secret-directory-token@{address}/rooms"),
        format!("ftp://{address}/rooms"),
        format!("http://{address}/rooms#secret-directory-token"),
        "/rooms".into(),
        "http:///rooms".into(),
        "file:///rooms".into(),
    ];
    for (epoch, url) in urls.into_iter().enumerate() {
        assert!(worker
            .try_submit(DirectoryJob {
                epoch: epoch as u64,
                url,
                publish: None,
                withdraw_port: None,
                fetch: true
            })
            .is_ok());
        let error = result(&worker)
            .result
            .expect_err("untrusted directory URL was accepted");
        assert!(!error.contains("secret-directory-token"));
        assert!(error.len() <= 120);
        assert!(
            listener.accept().is_err(),
            "invalid directory URL contacted the network"
        );
    }
}

#[test]
fn directory_listings_accept_only_valid_unicast_rooms_and_at_most_128_rows() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let mut private = BEACON.to_string();
        private.replace_range(46..48, "00");
        let mut dramatic = BEACON.to_string();
        dramatic.replace_range(40..42, "02");
        let mut unknown_kind = BEACON.to_string();
        unknown_kind.replace_range(40..42, "03");
        let mixed = serde_json::json!({"version": 1, "rooms": [
            {"address": "127.0.0.2:7000", "beacon": BEACON},
            {"address": "0.0.0.0:7000", "beacon": BEACON},
            {"address": "0.1.2.3:7000", "beacon": BEACON},
            {"address": "224.1.2.3:7000", "beacon": BEACON},
            {"address": "240.1.2.3:7000", "beacon": BEACON},
            {"address": "255.255.255.255:7000", "beacon": BEACON},
            {"address": "localhost:7000", "beacon": BEACON},
            {"address": "[::1]:7000", "beacon": BEACON},
            {"address": "127.0.0.2:0", "beacon": BEACON},
            {"address": "127.0.0.2:7001", "beacon": BEACON},
            {"address": "127.0.0.2:7000", "beacon": private},
            {"address": "127.0.0.3:7000", "beacon": dramatic},
            {"address": "127.0.0.2:7000", "beacon": unknown_kind},
            {"address": "127.0.0.2:7000", "beacon": "bad"},
            {"address": 123, "beacon": BEACON}
        ]})
        .to_string();
        let (mut first, _, _) = accept_request(&listener);
        respond(&mut first, mixed.as_bytes());
        for count in [129, 128] {
            let rows =
                vec![serde_json::json!({"address": "127.0.0.2:7000", "beacon": BEACON}); count];
            let body = serde_json::json!({"version": 1, "rooms": rows}).to_string();
            let (mut request, _, _) = accept_request(&listener);
            respond(&mut request, body.as_bytes());
        }
    });
    let worker = DirectoryWorker::new(Duration::from_secs(1)).unwrap();
    let fetch = |epoch| DirectoryJob {
        epoch,
        url: format!("http://{address}/rooms"),
        publish: None,
        withdraw_port: None,
        fetch: true,
    };
    assert!(worker.try_submit(fetch(1)).is_ok());
    let rooms = result(&worker).result.unwrap();
    assert_eq!(
        rooms.len(),
        2,
        "invalid or private rooms entered the listing"
    );
    assert_eq!(rooms[0].code, "SF1-7F00-0002");
    assert_eq!(rooms[0].kind, RoomKind::Versus);
    assert_eq!(rooms[1].code, "SF1-7F00-0003");
    assert_eq!(rooms[1].kind, RoomKind::Dramatic);
    assert!(worker.try_submit(fetch(2)).is_ok());
    assert!(
        result(&worker).result.is_err(),
        "more than 128 directory rows were accepted"
    );
    assert!(worker.try_submit(fetch(3)).is_ok());
    assert_eq!(result(&worker).result.unwrap().len(), 128);
    server.join().unwrap();
}

#[test]
fn malformed_oversized_and_redirected_directory_responses_fail_without_exposing_the_url() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let redirect = TcpListener::bind("127.0.0.1:0").unwrap();
    redirect.set_nonblocking(true).unwrap();
    let redirect_address = redirect.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let responses = [
            ("200 OK", b"{".to_vec()),
            ("200 OK", br#"{"version":2,"rooms":[]}"#.to_vec()),
            ("200 OK", br#"{"version":1,"rooms":{}}"#.to_vec()),
            ("200 OK", vec![b'x'; 64 * 1024 + 1]),
            ("302 Found", Vec::new()),
            ("500 Internal Server Error", Vec::new()),
        ];
        for (status, body) in responses {
            let (mut request, _, _) = accept_request(&listener);
            write!(request, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nLocation: http://{redirect_address}/secret-directory-token\r\nConnection: close\r\n\r\n", body.len()).unwrap();
            // The bounded client may close an oversized response before its final byte.
            let _ = request.write_all(&body);
        }
    });
    let worker = DirectoryWorker::new(Duration::from_secs(1)).unwrap();
    for epoch in 0..6 {
        assert!(worker
            .try_submit(DirectoryJob {
                epoch,
                url: format!("http://{address}/rooms?token=secret-directory-token"),
                publish: None,
                withdraw_port: None,
                fetch: true
            })
            .is_ok());
        let completed = result(&worker);
        assert_eq!(completed.epoch, epoch);
        let message = completed
            .result
            .expect_err("untrusted directory response was accepted");
        assert!(!message.contains("secret-directory-token"));
        assert!(message.len() <= 120);
    }
    server.join().unwrap();
    assert!(
        redirect.accept().is_err(),
        "the optional directory followed an HTTP redirect"
    );
}

#[test]
fn a_stalled_response_times_out_while_submission_and_result_polling_stay_bounded() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (started, accepted) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut request, _, _) = accept_request(&listener);
        let body = br#"{"version":1,"rooms":[]}"#;
        write!(
            request,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{{",
            body.len()
        )
        .unwrap();
        started.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        let _ = request.write_all(&body[1..]);
    });
    let worker = DirectoryWorker::new(Duration::from_millis(80)).unwrap();
    let job = |epoch, fetch| DirectoryJob {
        epoch,
        url: format!("http://{address}/rooms"),
        publish: None,
        withdraw_port: None,
        fetch,
    };
    assert!(worker.try_submit(job(30, true)).is_ok());
    accepted.recv_timeout(Duration::from_secs(1)).unwrap();
    let immediate = Instant::now();
    assert!(worker.try_submit(job(31, false)).is_ok());
    assert_eq!(
        worker.try_submit(job(32, false)).err().unwrap().epoch,
        32,
        "the worker accepted more than one queued job"
    );
    assert!(worker.try_result().is_none());
    assert!(
        immediate.elapsed() < Duration::from_millis(100),
        "frame-thread methods waited for HTTP"
    );
    let completed = result(&worker);
    assert_eq!(completed.epoch, 30);
    assert!(
        completed.result.is_err(),
        "an incomplete HTTP body outlived its configured timeout"
    );
    assert!(immediate.elapsed() < Duration::from_millis(400));
    let completed = result(&worker);
    assert_eq!(completed.epoch, 31);
    assert!(completed.result.unwrap().is_empty());
    server.join().unwrap();
}

#[test]
fn dropping_a_directory_worker_does_not_join_an_in_flight_http_request() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (started, accepted) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut request, _, _) = accept_request(&listener);
        started.send(()).unwrap();
        std::thread::sleep(Duration::from_millis(200));
        respond(&mut request, br#"{"version":1,"rooms":[]}"#);
    });
    let worker = DirectoryWorker::new(Duration::from_secs(1)).unwrap();
    assert!(worker
        .try_submit(DirectoryJob {
            epoch: 99,
            url: format!("http://{address}/rooms"),
            publish: None,
            withdraw_port: None,
            fetch: true
        })
        .is_ok());
    accepted.recv_timeout(Duration::from_secs(1)).unwrap();
    let immediate = Instant::now();
    drop(worker);
    assert!(immediate.elapsed() < Duration::from_millis(100));
    server.join().unwrap();
}
