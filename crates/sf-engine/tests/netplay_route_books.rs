//! Ticket #206 (`SC14`, spec #192, ADR-0026): the route books in use are
//! part of what both sides of a netplay session agree on.
//!
//! A peer's engine identity (`protocol_hash`, sent in its Hello and in the
//! synchronized rules) folds in a hash of every checked-in route book's
//! text, so two builds whose books differ are refused as a version
//! mismatch, the same way two releases are. A build with other books is
//! stood in for here by the identity it would send,
//! `netplay::protocol_identity` of its books' texts.
//!
//! The handshake and rollback tests need the local ROM set (and the
//! rollback test the art pack); they fail without them rather than skip.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::era::{listed_families, listed_revisions, loadouts, EraFamily, Loadout, Revision};
use sf_engine::fair_cpu::{perceive, Percept};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::{Fight, FightDips};
use sf_engine::front::menu::MenuItem;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, Transition, TypedKey};
use sf_engine::netplay::bus::{VirtualNetBus, VirtualNetConfig};
use sf_engine::netplay::protocol::{tag_from_str, HandshakePolicy, LobbyPhase};
use sf_engine::netplay::rollback::RollbackEngine;
use sf_engine::netplay::{
    encode_inputs, protocol_hash, protocol_identity, Link, NetPacket, NetTransportCommand, RejectReason, RoomKind,
    SyncedMatchRules,
};
use sf_engine::roster::{MenuInput, StagePick, TournamentFighter};
use sf_engine::route_book::{book_texts, books_hash, books_hash_of};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::Side;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const PACK: &str = "../../assets/sf1-port-art";
const RECIPES: [&str; 11] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/retsu.json"),
    include_str!("../../../art/recipes/geki.json"),
    include_str!("../../../art/recipes/joe.json"),
    include_str!("../../../art/recipes/mike.json"),
    include_str!("../../../art/recipes/lee.json"),
    include_str!("../../../art/recipes/gen.json"),
    include_str!("../../../art/recipes/birdie.json"),
    include_str!("../../../art/recipes/eagle.json"),
    include_str!("../../../art/recipes/adon.json"),
    include_str!("../../../art/recipes/sagat.json"),
];
const REVIEWS: [&str; 11] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/retsu.json"),
    include_str!("../../../art/review/geki.json"),
    include_str!("../../../art/review/joe.json"),
    include_str!("../../../art/review/mike.json"),
    include_str!("../../../art/review/lee.json"),
    include_str!("../../../art/review/gen.json"),
    include_str!("../../../art/review/birdie.json"),
    include_str!("../../../art/review/eagle.json"),
    include_str!("../../../art/review/adon.json"),
    include_str!("../../../art/review/sagat.json"),
];

/// The books' texts with one number in a route changed: the first digit
/// of book `index`'s first route (past its `route-book 1`, `fighter` and
/// `style` lines, whose digits are no route's) raised by one, as a book
/// searched again would differ.
fn with_one_book_changed(index: usize) -> Vec<String> {
    let mut texts: Vec<String> = book_texts().iter().map(|t| t.to_string()).collect();
    let text = &mut texts[index];
    let route = text.find("\nroute ").expect("a route book holds routes");
    let at = route + text[route..].find(|c: char| c.is_ascii_digit()).expect("a route holds numbers");
    let digit = text.as_bytes()[at] - b'0';
    text.replace_range(at..at + 1, &((digit + 1) % 10).to_string());
    texts
}

fn identity_of(texts: &[String]) -> u32 {
    protocol_identity(&texts.iter().map(String::as_str).collect::<Vec<_>>())
}

fn rules() -> SyncedMatchRules {
    SyncedMatchRules::new(
        MatchConfig::definitive(),
        [TournamentFighter::Ryu, TournamentFighter::Ken],
        FightDips::default(),
        StagePick::Bank(0),
        42,
        123,
        456,
    )
}

#[test]
fn the_agreed_configuration_covers_every_route_books_text() {
    // Every book on disk is one the build holds, so none is left out of
    // the hash.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("route-books");
    let mut on_disk = 0;
    for entry in std::fs::read_dir(&dir).expect("the route book folder") {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "routes") {
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(book_texts().contains(&text.as_str()), "{} is compiled in", path.display());
            on_disk += 1;
        }
    }
    assert_eq!(book_texts().len(), on_disk, "the build holds exactly the books on disk");
    assert!(on_disk >= 290, "the whole cast's books: {on_disk}");

    // The identity a session agrees on is this build's books' identity.
    let base = rules();
    assert_eq!(base.protocol_hash, protocol_hash());
    assert_eq!(protocol_hash(), protocol_identity(book_texts()));
    assert_eq!(books_hash(), books_hash_of(book_texts()));
    assert_ne!(protocol_identity(book_texts()), protocol_identity(&[]), "the books are in the identity");
    // A Windows checkout (`\r\n`) and a Linux one (`\n`) of the same books
    // agree.
    let lf: Vec<String> = book_texts().iter().map(|t| t.replace("\r\n", "\n")).collect();
    let crlf: Vec<String> = lf.iter().map(|t| t.replace('\n', "\r\n")).collect();
    assert_ne!(lf, crlf);
    assert_eq!(identity_of(&lf), base.protocol_hash, "line endings are not the books' content");
    assert_eq!(identity_of(&crlf), base.protocol_hash, "line endings are not the books' content");

    // A change to any one book changes the identity and so the agreed
    // configuration.
    let mut seen = std::collections::HashSet::from([base.protocol_hash]);
    for index in 0..book_texts().len() {
        let identity = identity_of(&with_one_book_changed(index));
        assert_ne!(identity, base.protocol_hash, "book {index} changed");
        let mut other = base;
        other.protocol_hash = identity;
        assert_ne!(other.config_hash(), base.config_hash(), "book {index} changed: the configuration differs");
        seen.insert(identity);
    }
    assert_eq!(seen.len(), book_texts().len() + 1, "each change gives an identity of its own");

    // A book dropped, or two books' texts swapped, changes it too.
    let texts: Vec<&str> = book_texts().to_vec();
    assert_ne!(protocol_identity(&texts[1..]), base.protocol_hash, "a book dropped");
    let mut swapped = texts.clone();
    swapped.swap(0, 1);
    assert_ne!(protocol_identity(&swapped), base.protocol_hash, "two books swapped");
}

fn session() -> GameSession {
    let assets = load_assets(ROMS, MANIFEST).expect("netplay session tests require the verified local ROMs");
    GameSession::new(assets, MatchConfig::definitive())
}

fn pulse(session: &mut GameSession, key: MenuInput) {
    let mut input = FrameInputs::default();
    match key {
        MenuInput::Down => input.p1.directions[2] = true,
        MenuInput::Left => input.p1.directions[1] = true,
        MenuInput::Right => input.p1.directions[0] = true,
        MenuInput::Confirm => input.menu.confirm = true,
        MenuInput::Start => input.menu.start = true,
        _ => panic!("unsupported test key"),
    }
    session.step(input);
    session.step(FrameInputs::default());
}

fn online(session: &mut GameSession) {
    session.step(FrameInputs::default());
    pulse(session, MenuInput::Start);
    pulse(session, MenuInput::Down);
    pulse(session, MenuInput::Confirm);
    pulse(session, MenuInput::Down);
    pulse(session, MenuInput::Confirm);
    assert_eq!(session.screen(), "online");
}

/// Which peer runs the other build.
#[derive(Clone, Copy, Debug)]
enum Other {
    /// The client's Hello carries this identity.
    Client(u32),
    /// The host's Welcome carries this identity in its rules.
    Host(u32),
}

/// Two real sessions over the virtual bus, host and client, one of them
/// sending the given identity in place of its own. The screens of both
/// ends and the client's notice when it settles.
fn handshake(other: Other) -> ([&'static str; 2], Option<String>) {
    let mut peers = [session(), session()];
    let mut bus = VirtualNetBus::new(VirtualNetConfig { latency_ms: 15, ..VirtualNetConfig::default() });
    online(&mut peers[0]);
    peers[0].drain_net_commands();
    pulse(&mut peers[0], MenuInput::Right);
    // VISIBILITY one step right of INTERNET CODE is PUBLIC, a local room.
    for key in [MenuInput::Down, MenuInput::Down, MenuInput::Right, MenuInput::Down, MenuInput::Down] {
        pulse(&mut peers[0], key);
    }
    pulse(&mut peers[0], MenuInput::Confirm);
    bus.submit(0, &peers[0].drain_net_commands());
    online(&mut peers[1]);
    peers[1].drain_net_commands();
    pulse(&mut peers[1], MenuInput::Left);
    pulse(&mut peers[1], MenuInput::Down);
    pulse(&mut peers[1], MenuInput::Confirm);
    peers[1].type_keys("SF1-7F00-0001".chars().map(TypedKey::Char).chain([TypedKey::Enter]));
    peers[1].step(FrameInputs::default());
    bus.submit(1, &peers[1].drain_net_commands());
    let mut rewritten = 0;
    for _ in 0..180 {
        for (id, peer) in peers.iter_mut().enumerate() {
            peer.set_net_time_ms(bus.now_ms() as u32);
            peer.push_net_events(&bus.drain_events(id as u8));
            peer.step(FrameInputs::default());
            let mut commands = peer.drain_net_commands();
            for command in &mut commands {
                let NetTransportCommand::SendPacket { bytes, .. } = command else { continue };
                let Some(mut packet) = NetPacket::from_bytes(bytes) else { continue };
                match (other, id, &mut packet) {
                    (Other::Client(identity), 1, NetPacket::Hello { protocol_hash, .. }) => {
                        *protocol_hash = identity;
                    }
                    (Other::Host(identity), 0, NetPacket::Welcome { config, .. }) => {
                        config.protocol_hash = identity;
                    }
                    _ => continue,
                }
                rewritten += 1;
                *bytes = packet.to_bytes();
            }
            bus.submit(id as u8, &commands);
        }
        bus.step_ms(17);
        if peers[1].notice().is_some_and(|n| n.contains("MISMATCH")) || peers.iter().all(|p| p.screen() == "select") {
            break;
        }
    }
    assert!(rewritten > 0, "the other build's identity was sent ({other:?})");
    (peers.each_ref().map(|p| p.screen()), peers[1].notice().map(str::to_string))
}

#[test]
fn a_session_between_builds_with_different_route_books_is_refused_as_a_version_mismatch() {
    let ours = protocol_identity(book_texts());
    let other_books = identity_of(&with_one_book_changed(book_texts().len() / 2));
    let other_release = ours ^ 1;

    // The host's own policy gives a peer with other books the reason it
    // gives one from another release.
    let base = rules();
    let policy = HandshakePolicy {
        rules: base,
        wired_only: false,
        host_tag: tag_from_str("HOST").unwrap(),
        max_rollback: 8,
        generation: 1,
        frame_base: 0,
        phase: LobbyPhase::Fighters,
        picker: 0,
        input_delay: None,
    };
    let hello = |protocol_hash| NetPacket::Hello {
        request_id: 9,
        tag: tag_from_str("CLIENT").unwrap(),
        kind: RoomKind::Versus,
        link: Link::Ethernet,
        protocol_hash,
        config_hash: 0,
        bank_hash: base.bank_hash,
        art_pack_hash: base.art_pack_hash,
    };
    let reason = |identity| match policy.welcome(&hello(identity), 0, false) {
        Some(NetPacket::Welcome { accepted, reason, .. }) => (accepted, reason),
        other => panic!("a Welcome: {other:?}"),
    };
    assert_eq!(reason(ours), (true, RejectReason::None));
    assert_eq!(reason(other_release), (false, RejectReason::VersionMismatch));
    assert_eq!(reason(other_books), (false, RejectReason::VersionMismatch));

    // Real sessions: the same books meet in Fighter Select. Other books are
    // refused as another release is, by the host on the client's Hello and
    // by the client on the host's Welcome (the host's own check passes in
    // that case, since only its Welcome is rewritten).
    let (screens, notice) = handshake(Other::Client(ours));
    assert_eq!(screens, ["select", "select"], "the same books agree: {notice:?}");
    let (screens, notice) = handshake(Other::Host(ours));
    assert_eq!(screens, ["select", "select"], "the same books agree: {notice:?}");
    for (books, release) in [
        (Other::Client(other_books), Other::Client(other_release)),
        (Other::Host(other_books), Other::Host(other_release)),
    ] {
        let refused = handshake(books);
        assert_eq!(refused, handshake(release), "{books:?} ends as {release:?} does");
        let (screens, notice) = refused;
        assert_eq!(screens[1], "online", "{books:?}: the client is not let in");
        assert!(
            notice.as_deref().is_some_and(|n| n.contains("VERSION MISMATCH")),
            "{books:?} is refused as a version mismatch: {notice:?}"
        );
        if let Other::Client(_) = books {
            assert_eq!(screens[0], "online", "the host refuses the client and stays open");
        }
    }
}

// The rollback test: Ryu against the stronger Fair CPU's Ryu, both in the
// SF6 era style, so the CPU plays from Ryu's SF6 route book. No netplay
// session can build this fight yet (a session keeps the stronger CPU's
// switch off and does not send it), so a Training fight stands in for one:
// the rollback engine a session runs plays it with the human's pad arriving
// late, as the CPU's peer would see it.

const FIGHT_LOOP: u32 = 0x3eafa;
const R: u8 = 1;
const L: u8 = 2;
const D: u8 = 4;
const U: u8 = 8;
const LP: u8 = 1;
const MK: u8 = 16;

fn p1(dirs: u8, buttons: u8) -> FrameInputs {
    FrameInputs { p1: pad(dirs, buttons), ..FrameInputs::default() }
}

fn pad(dirs: u8, buttons: u8) -> PlayerInputs {
    PlayerInputs {
        directions: [R, L, D, U].map(|bit| dirs & bit != 0),
        buttons: std::array::from_fn(|i| buttons & (1 << i) != 0),
    }
}

fn tap(s: &mut GameSession, input: FrameInputs) -> Option<Transition> {
    let t = s.step(input).transition;
    s.step(FrameInputs::default());
    t
}

fn choose(s: &mut GameSession, item: MenuItem) {
    for _ in 0..8 {
        if s.menu_item() == Some(item) {
            tap(s, p1(0, LP));
            return;
        }
        tap(s, p1(D, 0));
    }
    panic!("{item:?} missing from {}", s.screen());
}

fn lock_sf6(s: &mut GameSession, fighter: TournamentFighter) {
    assert_eq!(tap(s, p1(0, LP)), Some(Transition::StyleOpened), "the Style card opens");
    let order = [
        EraFamily::Sf1Plus,
        EraFamily::Sf2,
        EraFamily::SfZ,
        EraFamily::Sf3,
        EraFamily::Sf4,
        EraFamily::Sf5,
        EraFamily::Sf6,
        EraFamily::Sf1987,
    ];
    let available = listed_families(fighter, true);
    let steps = order.into_iter().filter(|f| available.contains(f)).position(|f| f == EraFamily::Sf6).expect("SF6 is listed");
    for _ in 0..steps {
        tap(s, p1(R, 0));
    }
    assert_eq!(listed_revisions(fighter, EraFamily::Sf6, true), [Revision::Sf6]);
    assert_eq!(loadouts(Revision::Sf6)[0], Loadout::Standard);
    tap(s, p1(0, LP));
}

/// Training against the stronger Fair CPU at Very Difficult, Ryu against
/// Ryu in SF6, from the Main Menu to the fight loop: the fight, lifted out.
fn fair_cpu_fight() -> Fight {
    let assets = load_assets(ROMS, MANIFEST).expect("the rollback test requires the verified local ROMs");
    let pack = ArtPack::open(Path::new(PACK)).expect("the rollback test requires the art pack");
    let art = load_art_from(&assets, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) })
        .expect("the art loads");
    let mut config = MatchConfig::definitive();
    config.roster.what_if_styles = true;
    let mut s = GameSession::new(assets, config)
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
        .with_stronger_cpu(true);
    s.set_fight_dips(FightDips { difficulty: 4, round_time: 100 });
    for _ in 0..200 {
        s.step(FrameInputs::default());
    }
    let mut start = FrameInputs::default();
    start.menu.start = true;
    tap(&mut s, start);
    choose(&mut s, MenuItem::SinglePlayer);
    choose(&mut s, MenuItem::Training);
    choose(&mut s, MenuItem::TrainingCpu);
    lock_sf6(&mut s, TournamentFighter::Ryu);
    for input in [U, U, L, L, L, L, L, L] {
        tap(&mut s, p1(input, 0));
    }
    lock_sf6(&mut s, TournamentFighter::Ryu);
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        match s.screen() {
            "stage select" | "country select" => {
                tap(&mut s, p1(0, LP));
            }
            _ => {
                s.step(FrameInputs::default());
            }
        }
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    let machine = s.fight_machine().unwrap();
    assert_eq!(machine.era_fighters().map(|e| e.style.revision), [Revision::Sf6; 2]);
    assert!(machine.match_config().stronger_cpu, "the stronger Fair CPU plays");
    assert!(machine.fair_cpu_pad().is_some(), "the Fair CPU plays Player 2");
    s.fight().unwrap().clone()
}

/// What Player 1 does, from what the frame shows: walk in, then poke with
/// crouching medium kick now and then (hits, guards and whiffs the CPU
/// answers with routes), and guard low for a while every so often.
fn player_one(frame: u32, v: &Percept) -> PlayerInputs {
    let toward = if v.me.x >= v.foe.x { R } else { L };
    let away = R + L - toward;
    let gap = if v.me.x >= v.foe.x { v.me.hurt.0 - v.foe.x } else { v.foe.x - v.me.hurt.1 };
    if frame % 240 >= 200 {
        return pad(D | away, 0);
    }
    if gap > 75 {
        return pad(toward, 0);
    }
    if frame % 45 == 0 {
        return pad(D, MK);
    }
    pad(0, 0)
}

#[test]
fn a_fair_cpu_fight_played_from_its_route_book_stays_in_step_under_rollback() {
    const FRAMES: u32 = 3_000;
    let assets = load_assets(ROMS, MANIFEST).expect("the rollback test requires the verified local ROMs");
    let initial = fair_cpu_fight();
    // The straight run: Player 1's pad from what each frame shows, kept as
    // the stream the late peer will send.
    let mut reference = initial.clone();
    let mut stream = Vec::new();
    let mut checksums = Vec::new();
    let mut routes = 0;
    let mut was_in_route = false;
    for frame in 0..FRAMES {
        let m = reference.machine();
        let view = perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P2);
        let input = player_one(frame, &view);
        reference.step(&assets, input, PlayerInputs::default(), false);
        stream.push(input);
        checksums.push(reference.state_checksum());
        let in_route = reference.machine().fair_cpu_entering_route();
        routes += u32::from(in_route && !was_in_route);
        was_in_route = in_route;
    }
    assert!(routes >= 3, "the CPU took routes from its book while the session ran: {routes}");

    // The CPU's peer runs the same fight with Player 1's pad arriving late,
    // rolling back and playing again every frame it guessed wrong.
    for late in [2u32, 7] {
        let mut fight = initial.clone();
        let mut net = RollbackEngine::new(1, 0, 8);
        let mut deepest = 0;
        let mut checked = vec![false; FRAMES as usize];
        for tick in 0..FRAMES + late {
            if tick >= late {
                let remote = tick - late;
                net.apply_remote_inputs(&mut fight, &assets, remote, &[encode_inputs(stream[remote as usize])]);
                deepest = deepest.max(net.last_rollback_frames());
            }
            if net.next_frame() < FRAMES {
                net.step(&mut fight, &assets, PlayerInputs::default()).expect("the session advances");
            }
            if let Some((frame, checksum)) = net.latest_confirmed_checksum() {
                assert_eq!(checksum, checksums[frame as usize], "{late} frames late, confirmed frame {frame} agrees");
                checked[frame as usize] = true;
            }
        }
        assert_eq!(net.confirmed_frame(), i64::from(FRAMES) - 1);
        assert_eq!(deepest, late as u8, "rollbacks of {late} frames were played again");
        assert!(checked.iter().all(|&c| c), "every frame was checked as it was confirmed");
        assert!(
            fight.machine().save_snapshot() == reference.machine().save_snapshot(),
            "{late} frames late, the two ends finish in the same state"
        );
        assert_eq!(fight.state_checksum(), reference.state_checksum());
    }
}
