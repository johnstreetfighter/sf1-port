//! NP5: the approved twelve-fighter bank survives the public online flow.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::data_bank::table_address;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, TypedKey};
use sf_engine::netplay::bus::{VirtualNetBus, VirtualNetConfig};
use sf_engine::netplay::protocol::LobbyPhase;
use sf_engine::netplay::{NetPacket, NetTransportCommand};
use sf_engine::roster::{MenuInput, StagePick, TournamentFighter};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::background_bank::BackgroundBank;

macro_rules! fighter_files {
    ($folder:literal) => {
        [
            include_str!(concat!("../../../art/", $folder, "/ryu.json")),
            include_str!(concat!("../../../art/", $folder, "/ken.json")),
            include_str!(concat!("../../../art/", $folder, "/retsu.json")),
            include_str!(concat!("../../../art/", $folder, "/geki.json")),
            include_str!(concat!("../../../art/", $folder, "/joe.json")),
            include_str!(concat!("../../../art/", $folder, "/mike.json")),
            include_str!(concat!("../../../art/", $folder, "/lee.json")),
            include_str!(concat!("../../../art/", $folder, "/gen.json")),
            include_str!(concat!("../../../art/", $folder, "/birdie.json")),
            include_str!(concat!("../../../art/", $folder, "/eagle.json")),
            include_str!(concat!("../../../art/", $folder, "/adon.json")),
            include_str!(concat!("../../../art/", $folder, "/sagat.json")),
        ]
    };
}

fn session() -> GameSession {
    static LOADED: OnceLock<GameSession> = OnceLock::new();
    LOADED
        .get_or_init(|| {
            let assets = sf_video::romset::load_assets(
                "../../tools/reference/local/roms/sf",
                "../../tools/reference/rom-manifest.json",
            )
            .expect("NP5 roster coverage requires the verified local ROMs");
            let pack = ArtPack::open(Path::new("../../assets/sf1-port-art"))
                .expect("NP5 roster coverage requires the approved local art pack");
            let art = load_art_from(
                &assets,
                &ArtSources {
                    recipes: &fighter_files!("recipes"),
                    reviews: Some(&fighter_files!("review")),
                    pack: Ok(&pack),
                },
            )
            .expect("the approved tournament fighter catalogues load");
            assert_eq!(
                art.catalogues.len(),
                12,
                "no fighter may fall back to missing art"
            );
            assert!(
                art.kept.is_empty(),
                "every approved fighter must load: {:?}",
                art.kept
            );
            let mut backgrounds = BackgroundBank::new();
            backgrounds
                .add_rom_stages(&assets)
                .expect("the ten native stages load");
            backgrounds
                .add_vs_screens(&assets)
                .expect("boss-vs-boss VS screens build");
            let session = GameSession::new(
                assets,
                MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul),
            )
            .with_sprite_bank(&art.bank)
            .with_player_tables(&art.catalogues)
            .with_background_bank(backgrounds);
            let bank = session
                .data_bank()
                .expect("the actual extended data bank builds");
            assert_eq!(
                bank.fighters().len(),
                10,
                "all ten bosses have player tables"
            );
            for fighter in TournamentFighter::ALL {
                assert!(
                    fighter.is_shoto() || bank.table(fighter).is_some(),
                    "{fighter:?} is playable"
                );
            }
            session
        })
        .clone()
}

fn key(key: MenuInput) -> FrameInputs {
    let mut input = FrameInputs::default();
    match key {
        MenuInput::Up => input.p1.directions[3] = true,
        MenuInput::Down => input.p1.directions[2] = true,
        MenuInput::Left => input.p1.directions[1] = true,
        MenuInput::Right => input.p1.directions[0] = true,
        MenuInput::Confirm => input.menu.confirm = true,
        MenuInput::Start => input.menu.start = true,
        _ => panic!("unsupported roster test key"),
    }
    input
}

fn pulse(peer: &mut GameSession, input: MenuInput) {
    peer.step(key(input));
    peer.step(FrameInputs::default());
}

fn online(peer: &mut GameSession) {
    peer.step(FrameInputs::default());
    for input in [
        MenuInput::Start,
        MenuInput::Down,
        MenuInput::Confirm,
        MenuInput::Down,
        MenuInput::Confirm,
    ] {
        pulse(peer, input);
    }
    assert_eq!(peer.screen(), "online");
    peer.drain_net_commands();
}

struct Pair {
    peers: [GameSession; 2],
    bus: VirtualNetBus,
    packets: Vec<(usize, NetPacket)>,
    latches: [Vec<u8>; 2],
}

impl Pair {
    fn connected() -> Self {
        let mut client = session();
        client.set_fight_dips(sf_engine::front::FightDips {
            difficulty: 4,
            round_time: 250,
        });
        let mut pair = Self {
            peers: [session(), client],
            bus: VirtualNetBus::new(VirtualNetConfig {
                latency_ms: 17,
                ..VirtualNetConfig::default()
            }),
            packets: Vec::new(),
            latches: std::array::from_fn(|_| Vec::new()),
        };
        online(&mut pair.peers[0]);
        pulse(&mut pair.peers[0], MenuInput::Right);
        for _ in 0..4 {
            pulse(&mut pair.peers[0], MenuInput::Down);
        }
        pulse(&mut pair.peers[0], MenuInput::Confirm);
        pair.bus.submit(0, &pair.peers[0].drain_net_commands());
        online(&mut pair.peers[1]);
        pulse(&mut pair.peers[1], MenuInput::Left);
        pulse(&mut pair.peers[1], MenuInput::Down);
        pulse(&mut pair.peers[1], MenuInput::Confirm);
        pair.peers[1].type_keys(
            "SF1-7F00-0001"
                .chars()
                .map(TypedKey::Char)
                .chain([TypedKey::Enter]),
        );
        pair.peers[1].step(FrameInputs::default());
        pair.bus.submit(1, &pair.peers[1].drain_net_commands());
        for _ in 0..180 {
            pair.idle();
            if pair.peers.iter().all(|peer| peer.screen() == "select") {
                return pair;
            }
        }
        panic!("both approved-art peers must complete the handshake");
    }

    fn step(&mut self, input: [FrameInputs; 2]) {
        for side in 0..2 {
            let peer = &mut self.peers[side];
            peer.set_net_time_ms(self.bus.now_ms() as u32);
            peer.push_net_events(&self.bus.drain_events(side as u8));
            self.latches[side].push(peer.step(input[side]).sound_latch);
            let commands = peer.drain_net_commands();
            for command in &commands {
                if let NetTransportCommand::SendPacket { bytes, .. } = command {
                    self.packets.push((
                        side,
                        NetPacket::from_bytes(bytes).expect("the public queue emits valid packets"),
                    ));
                }
            }
            self.bus.submit(side as u8, &commands);
        }
        self.bus.step_ms(17);
    }

    fn idle(&mut self) {
        self.step([FrameInputs::default(); 2]);
    }

    fn pulse(&mut self, side: usize, input: MenuInput) {
        let mut inputs = [FrameInputs::default(); 2];
        inputs[side] = key(input);
        self.step(inputs);
        self.idle();
    }
}

#[test]
fn all_twelve_online_picks_use_the_approved_bank_and_boss_fights_converge() {
    let mut pair = Pair::connected();
    // Traverse every tile through each player's physical P1 controls, and
    // observe encoded public LobbyPick packets rather than private cursors.
    pair.pulse(1, MenuInput::Left);
    for side in 0..2 {
        for _ in 0..5 {
            pair.pulse(side, MenuInput::Right);
        }
        pair.pulse(side, MenuInput::Down);
        for _ in 0..5 {
            pair.pulse(side, MenuInput::Left);
        }
        pair.pulse(side, MenuInput::Up);
        for fighter in TournamentFighter::ALL {
            assert!(pair.packets.iter().any(|(sender, packet)| matches!(packet,
                NetPacket::LobbyPick { side: assigned, fighter: picked, phase: LobbyPhase::Fighters, .. }
                if *sender == side && usize::from(*assigned) == side && *picked == fighter
            )), "side {side} can select and send {fighter:?}");
        }
    }
    pair.pulse(0, MenuInput::Down); // Lee, below Ryu.
    for _ in 0..5 {
        pair.pulse(1, MenuInput::Right);
    }
    pair.pulse(1, MenuInput::Down); // Sagat, below Mike.
    for side in [1, 0] {
        pair.pulse(side, MenuInput::Confirm); // Open the fighter's Style Card.
        pair.pulse(side, MenuInput::Confirm); // Lock the default style.
        for _ in 0..5 {
            pair.idle();
        }
    }
    for _ in 0..60 {
        pair.idle();
        if pair
            .peers
            .iter()
            .all(|peer| peer.screen() == "stage select")
        {
            break;
        }
    }
    assert!(pair
        .peers
        .iter()
        .all(|peer| peer.screen() == "stage select"));
    let bank = pair.peers[0].background_bank().unwrap();
    assert_eq!(bank.listed_stages().len(), 10);
    let sagat = bank
        .listed_stages()
        .iter()
        .find(|stage| stage.owner == "Sagat")
        .unwrap()
        .index;
    let stage = bank.stage(sagat).unwrap();
    // The handshake's shared initial stage is bank 0. Choose Sagat's stage
    // explicitly so the test verifies the stage cursor's online routing.
    for _ in 0..9 {
        pair.pulse(0, MenuInput::Down);
    }
    pair.pulse(0, MenuInput::Confirm);
    assert!(
        pair.packets.iter().any(|(sender, packet)| matches!(packet,
            NetPacket::LobbyPick { phase: LobbyPhase::Stage, stage: picked, locked: true, .. }
            if *sender == 0 && *picked == StagePick::Bank(sagat)
        )),
        "the host locks Sagat's native stage through the public queue"
    );
    for _ in 0..3_000 {
        pair.idle();
        if pair.peers.iter().all(|peer| {
            peer.fight()
                .is_some_and(|fight| !fight.driving() && flow_position(fight.machine()) == 0x3eafa)
        }) {
            break;
        }
    }
    for side in 0..2 {
        let peer = &pair.peers[side];
        let fight = peer
            .fight()
            .expect("both peers start a real boss-vs-boss fight");
        assert!(!fight.driving() && flow_position(fight.machine()) == 0x3eafa);
        let config = fight.setup().config;
        assert_eq!(
            (config.roster.p1, config.roster.p2),
            (TournamentFighter::Lee, TournamentFighter::Sagat)
        );
        assert_eq!(config.roster.stage, Some(StagePick::Bank(sagat)));
        assert_eq!(config, pair.peers[0].fight().unwrap().setup().config);
        assert_eq!(
            (config.difficulty, config.round_time),
            (
                session().fight_dips().difficulty,
                session().fight_dips().round_time
            )
        );
        assert!(config.toggles.true_combos && config.toggles.cancels && config.toggles.throws);
        let machine = fight.machine();
        for (player, fighter) in [
            (P1_STRUCT, TournamentFighter::Lee),
            (P2_STRUCT, TournamentFighter::Sagat),
        ] {
            let at = (player + 0x12 - 0xff8000) as usize;
            let table = u32::from_be_bytes(machine.work_ram()[at..at + 4].try_into().unwrap());
            assert_eq!(
                table,
                table_address(fighter),
                "the native player uses {fighter:?}'s extended table"
            );
        }
        assert!(peer.fight_sprite_tiles().unwrap() > peer.assets().sprite_tiles());
        assert_eq!(machine.state.fg_scroll, stage.fg.scroll + 0xc0);
        assert!((stage.bg.scroll..=stage.bg.scroll + 0x200).contains(&machine.state.bg_scroll));
        let scene = machine.state.scene();
        assert_eq!((scene.fg_page, scene.bg_page), (0, 0));
        for (offset, palette) in [(0, stage.bg.palette), (0x200, stage.fg.palette)] {
            let at = palette as usize;
            assert!(
                scene.paletteram[offset..offset + 0x200] == peer.assets().maincpu[at..at + 0x200]
            );
        }
        assert!(
            pair.latches[side].contains(&(stage.theme as u8)),
            "Sagat's native theme plays on side {side}"
        );
    }
    let before = pair
        .peers
        .each_ref()
        .map(|peer| peer.fight_machine().unwrap().state.frame);
    let wire_before = pair
        .packets
        .iter()
        .filter_map(|(_, packet)| match packet {
            NetPacket::Input { checksum_frame, .. } if *checksum_frame != u32::MAX => {
                Some(*checksum_frame)
            }
            _ => None,
        })
        .max()
        .expect("the native intro has confirmed input frames");
    for frame in 0..180 {
        pair.step(std::array::from_fn(|side| FrameInputs {
            p1: PlayerInputs {
                directions: [
                    side == 0 && frame < 30,
                    side == 1 && frame < 30,
                    false,
                    false,
                ],
                buttons: [frame % 20 < 3, false, false, false, false, false],
            },
            ..FrameInputs::default()
        }));
    }
    let checksums: [BTreeMap<u32, u64>; 2] = std::array::from_fn(|side| {
        pair.packets
            .iter()
            .filter_map(|(sender, packet)| match packet {
                NetPacket::Input {
                    checksum_frame,
                    checksum,
                    ..
                } if *sender == side && *checksum_frame != u32::MAX => {
                    Some((*checksum_frame, *checksum))
                }
                _ => None,
            })
            .collect()
    });
    let common: Vec<_> = checksums[0]
        .iter()
        .filter(|(frame, _)| **frame > wire_before && checksums[1].contains_key(frame))
        .collect();
    assert!(
        common.len() > 100,
        "compare confirmed wire frames, never speculative tips"
    );
    for (frame, checksum) in common {
        assert_eq!(
            Some(checksum),
            checksums[1].get(frame),
            "confirmed frame {frame}"
        );
    }
    for (side, peer) in pair.peers.iter().enumerate() {
        assert_eq!(peer.screen(), "fight", "side {side} stays connected");
        assert!(peer.fight_machine().unwrap().state.frame > before[side] + 100);
        assert!(peer.fight_machine().unwrap().unrecovered().is_empty());
    }
}
