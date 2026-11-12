//! Native SF2 commands from real Character Select and two-player pads.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::data_bank::{SF2_SPECIAL_SLOT, SF2_SUPER_SLOT};
use sf_engine::era::Revision;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, PlayerInputs, Transition};
use sf_engine::front::settings::Settings;
use sf_engine::roster::TournamentFighter::{self, Ken, Ryu, Sagat};
use sf_engine::front::cabinet::flow_position;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::Side;
use sf_video::romset::load_assets;
use sf_video::sprite_bank::{sprite_slot, ROM_SPRITE_TILES};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const LP: usize = 0;
const MP: usize = 1;
const HP: usize = 2;
const LK: usize = 3;

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn pad(direction: u8, facing_right: bool, button: Option<usize>) -> PlayerInputs {
    let mut input = PlayerInputs::default();
    let forward = usize::from(!facing_right);
    let back = 1 - forward;
    input.directions[forward] = matches!(direction, 3 | 6 | 9);
    input.directions[back] = matches!(direction, 1 | 4 | 7);
    input.directions[2] = matches!(direction, 1..=3);
    input.directions[3] = matches!(direction, 7..=9);
    if let Some(button) = button {
        input.buttons[button] = true;
    }
    input
}

fn tap(session: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let output = session.step(input);
    session.step(idle());
    output
}

fn p1(direction: u8, button: Option<usize>) -> FrameInputs {
    FrameInputs { p1: pad(direction, true, button), ..idle() }
}

fn p2(direction: u8, button: Option<usize>) -> FrameInputs {
    FrameInputs { p2: pad(direction, false, button), ..idle() }
}

fn select_st(session: &mut GameSession, confirm: FrameInputs, right: FrameInputs) {
    assert_eq!(tap(session, confirm).transition, Some(Transition::StyleOpened));
    assert_eq!(tap(session, right).transition, Some(Transition::CursorMoved));
    tap(session, confirm);
}

fn until(session: &mut GameSession, ready: impl Fn(&GameSession) -> bool, label: &str) {
    for _ in 0..3000 {
        if ready(session) {
            return;
        }
        session.step(idle());
    }
    panic!("did not reach {label} from {}", session.screen());
}

fn fight(first: TournamentFighter, training: bool) -> Option<GameSession> {
    if !Path::new(ROMS).is_dir() || (first == Sagat && !Path::new(PACK).is_dir()) {
        eprintln!("skipping: local ROM set or Sagat's art pack not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let mut session = GameSession::new(assets, MatchConfig::definitive())
        .with_settings(Settings { fill_meters: training, ..Settings::default() });
    if first == Sagat {
        let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
        let art = load_art_from(
            session.assets(),
            &ArtSources {
                recipes: &[include_str!("../../../art/recipes/sagat.json")],
                reviews: Some(&[include_str!("../../../art/review/sagat.json")]),
                pack: Ok(&pack),
            },
        )
        .expect("Sagat's approved poses load");
        session = session.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues);
    }

    for _ in 0..200 {
        session.step(idle());
    }
    let mut start = idle();
    start.menu.start = true;
    tap(&mut session, start);
    let confirm = p1(5, Some(LP));
    if training {
        tap(&mut session, confirm); // Single Player
        tap(&mut session, p1(2, None));
        tap(&mut session, confirm); // Training
        tap(&mut session, p1(2, None));
        assert_eq!(tap(&mut session, confirm).screen, "select"); // Dummy
    } else {
        tap(&mut session, p1(2, None));
        tap(&mut session, confirm); // Multi Player
        assert_eq!(tap(&mut session, confirm).screen, "select"); // Local
    }
    if first == Sagat {
        tap(&mut session, p1(2, None));
        for _ in 0..5 {
            tap(&mut session, p1(6, None));
        }
    } else if first == Ken {
        tap(&mut session, p1(6, None));
    }
    select_st(&mut session, confirm, p1(6, None));
    if training {
        tap(&mut session, confirm); // Ken
        tap(&mut session, confirm); // SF1+
    } else {
        select_st(&mut session, p2(5, Some(LP)), p2(4, None));
    }
    until(&mut session, |s| s.screen() == "country select", "country select");
    tap(&mut session, confirm); // Japan
    until(
        &mut session,
        |s| s.fight_machine().map(flow_position) == Some(FIGHT_LOOP),
        "fight loop",
    );
    for _ in 0..240 {
        session.step(idle());
    }
    let bout = session.fight().unwrap();
    assert_eq!([bout.player_fighter(0), bout.player_fighter(1)], [first, Ken]);
    let styles = bout.machine().era_fighters();
    assert_eq!(styles[0].style.revision, Revision::St);
    if !training {
        assert_eq!(styles[1].style.revision, Revision::St);
    }
    Some(session)
}

fn close(session: &mut GameSession) {
    for _ in 0..180 {
        let read = session.fight_machine().unwrap().diagnostic_telemetry();
        if read.p2.center_x - read.p1.center_x <= 48 {
            for _ in 0..4 {
                session.step(idle());
            }
            return;
        }
        session.step(p1(6, None));
    }
    panic!("failed to walk into close range");
}

fn command(session: &mut GameSession, directions: &[u8], button: usize) {
    for (index, &direction) in directions.iter().enumerate() {
        let attack = (index + 1 == directions.len()).then_some(button);
        for _ in 0..3 {
            session.step(p1(direction, attack));
        }
    }
}

fn p1_meter(session: &GameSession) -> u8 {
    session.fight_machine().unwrap().era_fighters()[0].super_meter
}

#[test]
fn st_normal_meter_depends_on_strength_and_contact() {
    let Some(base) = fight(Ryu, false) else { return };
    let mut near = base.clone();
    close(&mut near);

    for (button, hit_gain, block_gain) in [(LP, 2, 1), (MP, 4, 3), (HP, 5, 4)] {
        for (start, guard, hit, blocked, gain) in [
            (&base, 5, false, false, 0),
            (&near, 5, true, false, hit_gain),
            (&near, 1, false, true, block_gain),
        ] {
            let mut session = start.clone();
            assert_eq!(p1_meter(&session), 0);
            let mut attacked = false;
            let mut landed = false;
            let mut blockstun = false;
            for frame in 0..48 {
                session.step(FrameInputs {
                    p1: pad(2, true, (frame == 0).then_some(button)),
                    p2: pad(guard, false, None),
                    ..idle()
                });
                let read = session.fight().unwrap().fight_read();
                attacked |= read.fighter(Side::P1).attacking();
                landed |= read.landed_by(Side::P1).is_some_and(|hit| !hit.chip);
                blockstun |= read.fighter(Side::P2).blockstun;
            }
            assert!(attacked, "crouching punch button {button} must start");
            assert_eq!(
                (landed, blockstun),
                (hit, blocked),
                "button {button}, guard {guard}"
            );
            assert_eq!(
                p1_meter(&session),
                gain,
                "button {button}, hit {hit}, blocked {blocked}"
            );
        }
    }
}

#[test]
fn st_special_meter_is_awarded_at_startup_by_move() {
    const QCF: &[u8] = &[2, 3, 6];
    const DP: &[u8] = &[6, 2, 3];
    const QCB: &[u8] = &[2, 1, 4];
    const KNEE: &[u8] = &[2, 3, 6, 9];

    for (fighter, specials) in [
        (Ryu, &[(QCF, LP, 0x1a, 2), (DP, LP, 0x19, 7), (QCB, LK, 0x1b, 6)][..]),
        (Ken, &[(QCF, LP, 0x1a, 2), (DP, LP, 0x19, 8), (QCB, LK, 0x1b, 6)][..]),
        (Sagat, &[
            (QCF, LP, 0x1a, 3),
            (QCF, LK, 0x1b, 3),
            (DP, LP, SF2_SPECIAL_SLOT, 7),
            (KNEE, LK, 0x19, 6),
        ][..]),
    ] {
        let Some(base) = fight(fighter, false) else { continue };
        assert_eq!(p1_meter(&base), 0);
        for &(directions, button, move_id, gain) in specials {
            let mut session = base.clone();
            command(&mut session, directions, button);
            let read = session.fight().unwrap().fight_read();
            assert_eq!(
                read.fighter(Side::P1).move_id,
                move_id,
                "{fighter:?} command {directions:?}, button {button}"
            );
            assert!(read.landed_by(Side::P1).is_none());
            assert_eq!(p1_meter(&session), gain, "{fighter:?} move {move_id:#04x}");
        }
    }
}

#[test]
fn st_hadoken_hit_and_chip_do_not_add_meter() {
    let Some(base) = fight(Ryu, false) else { return };
    for (guard, chip) in [(5, false), (1, true)] {
        let mut session = base.clone();
        command(&mut session, &[2, 3, 6], LP);
        assert_eq!(p1_meter(&session), 2);
        let mut contacts = Vec::new();
        for _ in 0..120 {
            session.step(p2(guard, None));
            if let Some(hit) = session.fight().unwrap().fight_read().landed_by(Side::P1) {
                contacts.push(hit.chip);
            }
        }
        assert_eq!(contacts, [chip]);
        assert_eq!(p1_meter(&session), 2);
    }
}

fn observe_rising_move(mut session: GameSession, id: u8) -> (usize, bool, usize) {
    let mut started = false;
    let mut hits = 0;
    let mut frozen = false;
    let mut launches = 0;
    let mut rising = false;
    for frame in 0..150 {
        let Some(bout) = session.fight() else {
            panic!("move {id:#04x} left Training at frame {frame}: {:?}", session.take_log());
        };
        let read = bout.fight_read();
        let attacker = read.fighter(Side::P1);
        if attacker.move_id == id {
            started = true;
            let just_rising = attacker.state & 3 == 1;
            launches += usize::from(just_rising && !rising);
            rising = just_rising;
            hits += usize::from(read.landed_by(Side::P1).is_some_and(|hit| !hit.chip));
        }
        frozen |= bout.machine().era_super_freeze() != 0;
        session.step(idle());
    }
    assert!(started, "the normal pad input starts {id:#04x}");
    assert!(session.fight_machine().unwrap().unrecovered().is_empty());
    (hits, frozen, launches)
}

#[test]
fn sagat_uses_the_native_tiger_uppercut_and_tiger_genocide_streams() {
    let Some(mut base) = fight(Sagat, true) else { return };
    close(&mut base);
    let mut uppercut = base.clone();
    command(&mut uppercut, &[6, 2, 3], LP);
    let (uppercut_hits, uppercut_freeze, uppercut_launches) = observe_rising_move(uppercut, SF2_SPECIAL_SLOT);
    command(&mut base, &[2, 3, 6, 2, 3, 6], LK);
    let (super_hits, super_freeze, super_launches) = observe_rising_move(base, SF2_SUPER_SLOT);

    assert!(uppercut_hits >= 1, "Tiger Uppercut connects at close range");
    assert!(!uppercut_freeze);
    assert_eq!(uppercut_launches, 1);
    assert!(super_hits >= 2, "Tiger Genocide's knee and uppercut contacts: {super_hits}");
    assert!(super_freeze);
    assert_eq!(super_launches, 2);
}

#[test]
fn tiger_genocide_draws_sagats_own_poses() {
    let Some(mut session) = fight(Sagat, true) else { return };
    close(&mut session);

    let bank = session.data_bank().unwrap();
    let crouch_header = bank.move_header(Sagat, 5).unwrap();
    let crouch = u16::from_be_bytes([bank.rb(crouch_header), bank.rb(crouch_header + 1)]);
    let crouch_slot = sprite_slot(crouch);
    assert!(crouch_slot >= ROM_SPRITE_TILES);
    assert!(crouch_slot < session.fight_sprite_tiles().unwrap());

    let drawn_pose = |session: &GameSession| {
        let machine = session.fight_machine().unwrap();
        let offset = usize::from(machine.diagnostic_telemetry().p1.entity_offset);
        let grid = &machine.state.objectram[offset..];
        u16::from_be_bytes([grid[0], grid[1]])
    };
    for (index, direction) in [2, 3, 6, 2, 3, 6].into_iter().enumerate() {
        for _ in 0..3 {
            session.step(p1(direction, (index == 5).then_some(LK)));
            if direction == 2 {
                assert_eq!(drawn_pose(&session), crouch, "quarter-circle crouch");
            }
        }
    }

    let mut poses = Vec::new();
    for _ in 0..150 {
        if session.fight().unwrap().fight_read().fighter(Side::P1).move_id == SF2_SUPER_SLOT {
            let pose = drawn_pose(&session);
            if poses.last() != Some(&pose) {
                poses.push(pose);
            }
        } else if !poses.is_empty() {
            break;
        }
        session.step(idle());
    }
    assert_eq!(poses, [0x2d80, 0x2d0a, 0x2c8c, 0x2d80, 0x2d00, 0x2c8c, 0x2d80]);
}

#[test]
fn kens_shoryureppa_advances_into_a_higher_finishing_uppercut() {
    let Some(base) = fight(Ken, true) else { return };
    for at_close_range in [false, true] {
        let mut session = base.clone();
        if at_close_range {
            close(&mut session);
        } else {
            for _ in 0..30 {
                session.step(p1(4, None));
            }
            for _ in 0..4 {
                session.step(idle());
            }
        }
        command(&mut session, &[2, 3, 6, 2, 3, 6], LP);
        let machine = session.fight_machine().unwrap();
        let start = machine.diagnostic_telemetry().p1;
        let start_scroll = machine.state.fg_scroll;

        let mut launches = 0;
        let mut was_rising = false;
        let mut frozen = false;
        let mut height = [0; 2];
        let mut airborne_frames = [0; 2];
        let mut hits = [0; 2];
        let mut travel = 0;
        for _ in 0..150 {
            let bout = session.fight().unwrap();
            let read = bout.fight_read();
            let attacker = read.fighter(Side::P1);
            frozen |= bout.machine().era_super_freeze() != 0;
            if attacker.move_id == SF2_SUPER_SLOT {
                let posture = attacker.state & 3;
                let rising = posture == 1;
                launches += usize::from(rising && !was_rising);
                was_rising = rising;
                let machine = bout.machine();
                let position = machine.diagnostic_telemetry().p1;
                let camera_delta = machine.state.fg_scroll.wrapping_sub(start_scroll) as i16;
                travel = position.center_x - start.center_x + i32::from(camera_delta);
                if launches > 0 {
                    let arc = launches - 1;
                    let entity = &machine.state.objectram[usize::from(position.entity_offset)..];
                    let grid_top = i32::from(u16::from_be_bytes([entity[4], entity[5]]));
                    let tile_height = if entity[2] & 4 != 0 { 32 } else { 16 };
                    let grid_bottom = grid_top + (i32::from(entity[0xc]) + 1) * tile_height;
                    height[arc] = height[arc].max(sf_engine::data_bank::GROUND - grid_bottom);
                    airborne_frames[arc] += usize::from(matches!(posture, 1 | 2));
                    hits[arc] += usize::from(read.landed_by(Side::P1).is_some_and(|hit| !hit.chip));
                }
            }
            session.step(idle());
        }

        assert!(frozen);
        assert_eq!(launches, 2);
        assert!(height[0] >= 35 && height[1] >= height[0] + 35, "apex heights: {height:?}");
        assert!(airborne_frames[1] >= 40, "airborne frames: {airborne_frames:?}");
        if at_close_range {
            assert!(hits.iter().all(|&n| n > 0), "hits by arc: {hits:?}");
        } else {
            assert_eq!(hits, [0, 0]);
            assert!(travel >= 75, "horizontal travel on whiff: {travel}");
        }
        assert!(session.fight_machine().unwrap().unrecovered().is_empty());
    }
}

#[derive(Debug, Default)]
struct RedTrace {
    saw_three: bool,
    saw_ordinary: bool,
    punched_through: bool,
    hits: usize,
    meter: u8,
}

fn observe_red(mut session: GameSession, clash: bool) -> RedTrace {
    let mut result = RedTrace::default();
    let mut last_hp = session.fight_machine().unwrap().era_fighters()[1].sub_hp;
    for frame in 0..150 {
        let phase = frame / 3;
        let dir1 = [4, 1, 2, 3, 6].get(phase).copied().unwrap_or(5);
        let dir2 = [5, 5, 2, 3, 6].get(phase).copied().unwrap_or(5);
        let attack = (phase == 4).then_some(LP);
        session.step(FrameInputs {
            p1: pad(dir1, true, attack),
            p2: pad(dir2, false, if clash { attack } else { None }),
            ..idle()
        });
        let machine = session.fight_machine().unwrap();
        let effects = machine.toggle_state().era.projectiles();
        let red = effects.iter().find(|p| p.crimson && p.hits_remaining != 0);
        let ordinary = effects.iter().any(|p| !p.crimson && p.move_id == 0x1a && p.hits_remaining == 1);
        let hp = machine.era_fighters()[1].sub_hp;
        result.hits += usize::from(hp < last_hp);
        last_hp = hp;
        result.saw_three |= red.is_some_and(|p| p.hits_remaining == 3);
        // Both effects can meet on the frame Ken's ordinary projectile spawns.
        // His move ID and the red projectile's spent hit before any HP loss
        // establish that collision even if the ordinary effect ends at once.
        result.saw_ordinary |= session.fight().unwrap().fight_read().fighter(Side::P2).move_id == 0x1a;
        result.punched_through |= result.saw_three && result.saw_ordinary && !ordinary
            && red.is_some_and(|p| p.hits_remaining == 2) && result.hits == 0;
    }
    assert!(session.fight_machine().unwrap().unrecovered().is_empty());
    result.meter = p1_meter(&session);
    result
}

#[test]
fn red_hadoken_hits_three_times_and_punches_through_an_ordinary_fireball() {
    let Some(base) = fight(Ryu, false) else { return };
    let alone = observe_red(base.clone(), false);
    assert!(alone.saw_three, "the native red Hadoken spawns with three hits");
    assert_eq!(alone.hits, 3, "all three contacts land on neutral Ken");
    assert_eq!(alone.meter, 2, "the three contacts add no meter after startup");
    let clash = observe_red(base, true);
    assert!(clash.punched_through, "the red projectile still has two hits after the clash: {clash:?}");
    assert_eq!(clash.hits, 2, "the surviving projectile spends its remaining hits");
    assert_eq!(clash.meter, 2, "the clash adds no meter after startup");
}
