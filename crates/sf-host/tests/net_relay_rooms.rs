//! Every room role through a real relay: Dramatic Battle allies and
//! spectators each hold their own seat in one Internet room.
use std::time::{Duration, Instant};

use sf_engine::config::MatchConfig;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::netplay::{NetPacket, NetTransportCommand as Command, NetTransportEvent as Event, RoomKind};
use sf_engine::roster::MenuInput;
use sf_host::net::HostNetDriver;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::background_bank::BackgroundBank;

#[path = "support/relay.rs"]
mod relay;

fn session(kind: RoomKind) -> GameSession {
    let assets = sf_video::romset::load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    )
    .expect("relayed room tests require the verified local ROMs");
    let mut backgrounds = BackgroundBank::new();
    backgrounds.add_rom_stages(&assets).unwrap();
    backgrounds.add_vs_screens(&assets).unwrap();
    let art = (kind == RoomKind::Dramatic).then(|| {
        let pack = ArtPack::open(std::path::Path::new("../../assets/sf1-port-art"))
            .expect("Dramatic Battle requires the approved art pack");
        load_art_from(&assets, &ArtSources {
            recipes: &[include_str!("../../../art/recipes/sagat.json")],
            reviews: Some(&[include_str!("../../../art/review/sagat.json")]),
            pack: Ok(&pack),
        })
        .expect("Sagat's approved poses load")
    });
    let session = GameSession::new(assets, MatchConfig::definitive()).with_background_bank(backgrounds);
    match art {
        Some(art) => session.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues),
        None => session,
    }
}

fn key(key: MenuInput) -> FrameInputs {
    let mut input = FrameInputs::default();
    match key {
        MenuInput::Down => input.p1.directions[2] = true,
        MenuInput::Up => input.p1.directions[3] = true,
        MenuInput::Left => input.p1.directions[1] = true,
        MenuInput::Right => input.p1.directions[0] = true,
        MenuInput::Confirm => input.menu.confirm = true,
        MenuInput::Start => input.menu.start = true,
        _ => panic!("unsupported test key"),
    }
    input
}

fn pulse(session: &mut GameSession, input: MenuInput) {
    session.step(key(input));
    session.step(FrameInputs::default());
}

fn online(session: &mut GameSession) {
    session.step(FrameInputs::default());
    for input in [MenuInput::Start, MenuInput::Down, MenuInput::Confirm, MenuInput::Down, MenuInput::Confirm] {
        pulse(session, input);
    }
    assert_eq!(session.screen(), "online");
    session.drain_net_commands();
}

/// How a guest enters the room from Direct Join.
#[derive(Clone, Copy, PartialEq)]
enum Role {
    Player,
    Spectator,
    Ally,
}

/// One Internet room on a real relay; peer 0 hosts it.
struct Room {
    relay: relay::TestRelay,
    template: GameSession,
    peers: Vec<GameSession>,
    drivers: Vec<HostNetDriver>,
    /// Every packet a peer handed its transport, with the sender's index.
    sent: Vec<(usize, NetPacket)>,
    /// What each peer's transport reported as failed.
    errors: Vec<Vec<String>>,
    code: String,
}

impl Room {
    fn host(kind: RoomKind) -> Self {
        let relay = relay::TestRelay::start();
        let settings = Settings { relay: Some(relay.address()), ..Settings::default() };
        let template = session(kind).with_settings(settings);
        let mut room = Self {
            peers: vec![template.clone()],
            drivers: vec![relay.driver()],
            template,
            relay,
            sent: Vec::new(),
            errors: vec![Vec::new()],
            code: String::new(),
        };
        let host = &mut room.peers[0];
        online(host);
        pulse(host, MenuInput::Right);
        pulse(host, MenuInput::Down);
        if kind == RoomKind::Dramatic {
            pulse(host, MenuInput::Right);
        }
        // VISIBILITY opens on INTERNET CODE once a relay is set.
        for _ in 0..3 {
            pulse(host, MenuInput::Down);
        }
        pulse(host, MenuInput::Confirm);
        let deadline = Instant::now() + Duration::from_secs(6);
        while room.peers[0].room_code_for_copy().is_none() {
            assert!(Instant::now() < deadline, "the host should show its Internet room code: {:?}", room.errors);
            room.step(None);
        }
        room.code = room.peers[0].room_code_for_copy().unwrap().to_owned();
        room
    }

    /// A new peer joins by the room's code; returns its index.
    fn join(&mut self, role: Role) -> usize {
        let id = self.peers.len();
        self.peers.push(self.template.clone());
        self.drivers.push(self.relay.driver());
        self.errors.push(Vec::new());
        let peer = &mut self.peers[id];
        online(peer);
        pulse(peer, MenuInput::Left);
        pulse(peer, MenuInput::Down);
        if role != Role::Player {
            pulse(peer, MenuInput::Down);
            pulse(peer, if role == Role::Spectator { MenuInput::Right } else { MenuInput::Left });
            pulse(peer, MenuInput::Up);
        }
        peer.paste_room_code(Some(&self.code));
        pulse(peer, MenuInput::Confirm);
        self.exchange(id);
        id
    }

    fn exchange(&mut self, id: usize) {
        let commands = self.peers[id].drain_net_commands();
        for command in &commands {
            if let Command::SendPacket { bytes, .. } = command {
                self.sent.push((id, NetPacket::from_bytes(bytes).expect("engine emitted malformed packet")));
            }
        }
        let events = self.drivers[id].poll(&commands);
        for event in &events {
            if let Event::TransportError { message } = event {
                self.errors[id].push(message.clone());
            }
        }
        self.peers[id].push_net_events(&events);
    }

    /// One frame for every peer; `pressed` gives one peer its input.
    fn step(&mut self, pressed: Option<(usize, FrameInputs)>) {
        self.step_all(|id| pressed.filter(|(side, _)| *side == id).map_or_else(FrameInputs::default, |(_, input)| input));
    }

    /// One frame for every peer, each with the input `inputs` gives it.
    fn step_all(&mut self, inputs: impl Fn(usize) -> FrameInputs) {
        for id in 0..self.peers.len() {
            self.peers[id].set_net_time_ms(self.drivers[id].now_ms());
            self.peers[id].step(inputs(id));
            self.exchange(id);
        }
        std::thread::sleep(Duration::from_millis(1));
    }

    fn press(&mut self, id: usize, input: MenuInput) {
        self.step(Some((id, key(input))));
        self.step(None);
    }

    fn wait(&mut self, what: &str, limit: usize, mut done: impl FnMut(&Self) -> bool) {
        for _ in 0..limit {
            if done(self) {
                return;
            }
            self.step(None);
        }
        panic!(
            "{what}: {:?} {:?}",
            self.peers.iter().map(|peer| (peer.screen(), peer.notice())).collect::<Vec<_>>(),
            self.errors
        );
    }

    /// Both combatants lock fighters and the host picks the stage.
    fn start_versus(&mut self) {
        self.wait("both players reach fighter select", 3_000, |room| {
            room.peers[..2].iter().all(|peer| peer.screen() == "select")
        });
        for id in [1, 0] {
            self.press(id, MenuInput::Confirm);
            self.press(id, MenuInput::Confirm);
        }
        self.wait("both players reach stage select", 3_000, |room| {
            room.peers[..2].iter().all(|peer| peer.screen() == "stage select")
        });
        self.press(0, MenuInput::Confirm);
        self.wait("the fight starts for both players", 3_000, |room| {
            room.peers[..2].iter().all(|peer| peer.fight().is_some_and(|fight| !fight.driving()))
        });
    }

    fn sent_by(&self, id: usize, matches: impl Fn(&NetPacket) -> bool) -> bool {
        self.sent.iter().any(|(from, packet)| *from == id && matches(packet))
    }
}

/// Walk forward and attack on a repeating two-second pattern.
fn attack(frame: u32) -> FrameInputs {
    let phase = frame % 120;
    let mut input = FrameInputs::default();
    input.p1.directions[0] = phase < 40;
    if phase >= 40 && phase % 10 < 3 {
        input.p1.buttons[((frame / 10) % 6) as usize] = true;
    }
    input
}

#[test]
fn spectators_join_an_internet_room_before_and_during_the_match() {
    let mut room = Room::host(RoomKind::Versus);
    let challenger = room.join(Role::Player);
    let early = room.join(Role::Spectator);
    room.start_versus();
    room.wait("the early spectator watches the fight", 3_000, |room| {
        room.peers[early].spectator_frame().is_some_and(|frame| frame >= 60)
    });

    let late = room.join(Role::Spectator);
    room.wait("the late spectator catches up with the fight", 6_000, |room| {
        room.peers[late].spectator_frame().is_some_and(|frame| frame >= 60)
    });

    for spectator in [early, late] {
        assert!(room.errors[spectator].is_empty(), "{:?}", room.errors);
        assert!(!room.sent_by(spectator, |packet| matches!(packet, NetPacket::Input { .. })), "a spectator sends no inputs");
        assert!(room.sent_by(spectator, |packet| matches!(packet, NetPacket::SpectatorAck { .. })));
    }
    assert!(room.sent_by(challenger, |packet| matches!(packet, NetPacket::Input { .. })));
    assert_eq!(room.relay.service.open_rooms(), 1, "every role shares the one room");
    assert_eq!(room.relay.service.connections(), 4, "each participant has its own route to the relay");
    assert!(room.peers.iter().all(|peer| !peer.notice().is_some_and(|notice| notice.contains("DESYNC"))));
}

#[test]
fn four_spectators_fill_an_internet_room_and_the_fifth_is_refused() {
    let mut room = Room::host(RoomKind::Versus);
    room.join(Role::Player);
    room.wait("both players reach fighter select", 3_000, |room| {
        room.peers[..2].iter().all(|peer| peer.screen() == "select")
    });
    let spectators: Vec<usize> = (0..4).map(|_| room.join(Role::Spectator)).collect();
    room.wait("four spectators are seated", 3_000, |room| {
        spectators.iter().all(|&id| room.peers[id].screen() != "online")
    });

    let fifth = room.join(Role::Spectator);
    room.wait("the fifth spectator is turned away", 3_000, |room| room.peers[fifth].notice() == Some("ROOM IS FULL"));
    assert_eq!(room.peers[fifth].screen(), "online", "the refused spectator keeps a usable lobby");
    assert!(spectators.iter().all(|&id| room.errors[id].is_empty()), "{:?}", room.errors);

    // A second challenger has no seat either: the room has its two players.
    let extra = room.join(Role::Player);
    room.wait("a third player is turned away", 3_000, |room| room.peers[extra].notice() == Some("ROOM IS FULL"));
    assert_eq!(room.relay.service.open_rooms(), 1);
}

#[test]
fn a_dramatic_battle_runs_to_its_end_through_an_internet_room_with_a_late_spectator() {
    let mut room = Room::host(RoomKind::Dramatic);
    let ally = room.join(Role::Ally);
    room.wait("the host reaches Dramatic Battle setup", 3_000, |room| room.peers[0].screen() == "dramatic setup");
    for input in [MenuInput::Down, MenuInput::Down, MenuInput::Right, MenuInput::Down, MenuInput::Right,
        MenuInput::Down, MenuInput::Confirm] {
        room.press(0, input);
    }
    room.wait("Ally 2 is asked for a fighter", 3_000, |room| room.peers[ally].screen() == "dramatic guest");
    room.press(ally, MenuInput::Right);
    room.press(ally, MenuInput::Confirm);
    room.wait("both allies start the same fight", 6_000, |room| {
        room.peers[..2].iter().all(|peer| peer.fight().is_some_and(|fight| !fight.driving()))
    });
    let configs: Vec<_> = room.peers[..2].iter().map(|peer| peer.fight().unwrap().setup().config).collect();
    assert_eq!(configs[0], configs[1]);
    assert!(configs[0].dramatic.is_some(), "the co-op rules crossed the relay");

    let watcher = room.join(Role::Spectator);
    let mut ended = false;
    for frame in 0..40_000_u32 {
        if room.peers[..2].iter().all(|peer| peer.screen() == "after match") {
            ended = true;
            break;
        }
        // Both allies walk in and attack; each drives its own fighter.
        room.step_all(|id| if id < 2 { attack(frame + 30 * id as u32) } else { FrameInputs::default() });
    }
    assert!(ended, "the relayed Dramatic Battle should finish: {:?} {:?}",
        room.peers.iter().map(|peer| (peer.screen(), peer.notice())).collect::<Vec<_>>(), room.errors);
    assert!(room.peers[watcher].spectator_frame().is_some_and(|frame| frame > 0), "the late spectator watched it");
    assert!(room.errors.iter().all(Vec::is_empty), "{:?}", room.errors);
    assert!(room.sent_by(ally, |packet| matches!(packet, NetPacket::Input { .. })));
    assert!(!room.sent_by(watcher, |packet| matches!(packet, NetPacket::Input { .. })));
    assert_eq!(room.relay.service.connections(), 3, "host, ally and spectator each hold a route");
    assert!(room.peers.iter().all(|peer| !peer.notice().is_some_and(|notice| notice.contains("DESYNC"))));
}
