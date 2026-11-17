//! World Super Turbo Ken's ordinary Shoryuken through Local Versus controllers.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::era::{Loadout, Revision};
use sf_engine::era_fight::FULL_SUB_HP;
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::Fight;
use sf_engine::front::menu::MenuItem;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, Transition};
use sf_video::callouts::Side;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const SHORYUKEN: u8 = 0x19;
/// Hit reactions start at this move id.
const REACTIONS: u8 = 0x30;
/// Room behind Ryu for two consecutive 78 px recoils and a knockdown.
const ROOM: u16 = 60;

fn assets() -> Assets {
    load_assets(ROMS, MANIFEST).expect("local ROMs assemble")
}

fn tap(session: &mut GameSession, input: FrameInputs) -> Option<Transition> {
    let output = session.step(input);
    session.step(FrameInputs::default());
    output.transition
}

fn pad(dir: u8, button: Option<usize>) -> PlayerInputs {
    let mut input = PlayerInputs {
        directions: [1, 2, 4, 8].map(|bit| dir & bit != 0),
        ..PlayerInputs::default()
    };
    if let Some(button) = button {
        input.buttons[button] = true;
    }
    input
}

fn side_tap(session: &mut GameSession, side: usize, dir: u8, punch: bool) -> Option<Transition> {
    let mut input = FrameInputs::default();
    let pad = pad(dir, punch.then_some(0));
    if side == 0 {
        input.p1 = pad;
    } else {
        input.p2 = pad;
    }
    tap(session, input)
}

fn choose(session: &mut GameSession, item: MenuItem) {
    for _ in 0..8 {
        if session.menu_item() == Some(item) {
            assert_eq!(side_tap(session, 0, 0, true), Some(Transition::Choose(item)));
            return;
        }
        side_tap(session, 0, 4, false);
    }
    panic!("no {item:?} on {}", session.screen());
}

/// Picks the default 1987 style, or an SFII revision and loadout.
fn select(session: &mut GameSession, side: usize, style: Option<(Revision, Loadout)>) {
    assert_eq!(side_tap(session, side, 0, true), Some(Transition::StyleOpened));
    if let Some((revision, loadout)) = style {
        side_tap(session, side, 1, false);
        if revision != Revision::St {
            side_tap(session, side, 4, false);
            let count = [Revision::Ww, Revision::Ce, Revision::Hf, Revision::Super]
                .iter()
                .position(|&r| r == revision)
                .unwrap()
                + 1;
            for _ in 0..count {
                side_tap(session, side, 1, false);
            }
        }
        if loadout == Loadout::Old {
            side_tap(session, side, 8, false);
            side_tap(session, side, 1, false);
        }
    }
    assert!(matches!(
        side_tap(session, side, 0, true),
        Some(Transition::Picked(_) | Transition::Fighters(_, _))
    ));
}

/// Local Versus with Ryu on the left and Ken on the right.
fn versus(styles: [Option<(Revision, Loadout)>; 2]) -> (Assets, Fight) {
    let mut session = GameSession::new(assets(), MatchConfig::default()).with_settings(Settings::default());
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }
    let mut input = FrameInputs::default();
    input.menu.start = true;
    assert_eq!(tap(&mut session, input), Some(Transition::OpenMainMenu));
    choose(&mut session, MenuItem::MultiPlayer);
    choose(&mut session, MenuItem::Local);
    select(&mut session, 0, styles[0]);
    select(&mut session, 1, styles[1]);
    for _ in 0..40 {
        if session.screen() == "country select" {
            break;
        }
        session.step(FrameInputs::default());
    }
    assert_eq!(session.screen(), "country select");
    side_tap(&mut session, 0, 0, true);
    for _ in 0..1500 {
        if session.fight_machine().map(flow_position) == Some(FIGHT_LOOP) {
            let mut fight = session.fight().unwrap().clone();
            let assets = assets();
            for _ in 0..12 {
                fight.step(&assets, pad(0, None), pad(0, None), false);
            }
            return (assets, fight);
        }
        session.step(FrameInputs::default());
    }
    panic!("the native fight loop did not open");
}

fn entity(fight: &Fight, ps: u32) -> usize {
    let pointer = (ps - 0xff8000) as usize + 0x10;
    let work = fight.machine().work_ram();
    usize::from(u16::from_be_bytes([work[pointer], work[pointer + 1]]))
}

/// The fighter's source axis in world pixels.
fn axis(fight: &Fight, side: Side) -> i32 {
    let machine = fight.machine();
    let telemetry = machine.diagnostic_telemetry();
    let fighter = if side == Side::P1 { telemetry.p1 } else { telemetry.p2 };
    let offset = machine.state.objectram[usize::from(fighter.entity_offset) + 0x26] as i8;
    let camera = u16::from_be_bytes(machine.work_ram()[0x400c..0x400e].try_into().unwrap());
    fighter.center_x - i32::from(offset) + i32::from(camera)
}

fn height(fight: &Fight, ps: u32) -> i32 {
    let e = entity(fight, ps);
    let o = &fight.machine().state.objectram[e..e + 0x40];
    let top = i32::from(i16::from_be_bytes([o[4], o[5]]));
    let cell = if o[2] & 4 == 0 { 16 } else { 32 };
    sf_engine::data_bank::GROUND - top - (i32::from(o[0xc]) + 1) * cell
}

fn shift(snapshot: &mut sf_engine::front::fight::FightSnapshot, ps: u32, shift: u16) {
    let offset = (ps - 0xff8000) as usize + 0x10;
    let e = usize::from(u16::from_be_bytes(snapshot.machine.work[offset..offset + 2].try_into().unwrap()));
    let objects = &mut snapshot.machine.state.objectram;
    let cells = (usize::from(objects[e + 0xc]) + 1) * (usize::from(objects[e + 0xd]) + 1);
    for sprite in (e..).step_by(0x40).take(cells) {
        let x = &mut objects[sprite + 6..sprite + 8];
        let moved = u16::from_be_bytes(x.try_into().unwrap()).wrapping_add(shift);
        x.copy_from_slice(&moved.to_be_bytes());
    }
}

/// Moves both fighters `room` pixels right, then puts Ken's axis `gap` pixels from Ryu's.
fn place(fight: &mut Fight, gap: u16, room: u16) {
    let current = (axis(fight, Side::P2) - axis(fight, Side::P1)) as u16;
    let mut snapshot = fight.save_netplay_snapshot(0);
    shift(&mut snapshot, P1_STRUCT, room);
    shift(&mut snapshot, P2_STRUCT, room.wrapping_add(gap.wrapping_sub(current)));
    fight.restore_netplay_snapshot(&snapshot);
    assert_eq!(axis(fight, Side::P2) - axis(fight, Side::P1), i32::from(gap));
}

/// Ken's forward, down, down-forward and punch, with the punch tick as tick zero.
/// As in the arcade placement controls, the gap is set before the last input.
fn shoryuken(fight: &mut Fight, assets: &Assets, defense: PlayerInputs, strength: usize, gap: u16, room: u16) {
    for dir in [2, 4] {
        fight.step(assets, defense, pad(dir, None), false);
    }
    place(fight, gap, room);
    fight.step(assets, defense, pad(6, Some(strength - 1)), false);
    assert_eq!(fight.fight_read().fighters[1].move_id, SHORYUKEN);
}

/// What one Shoryuken did, in ticks from Ken's action entry.
#[derive(Debug, Default, PartialEq, Eq)]
struct Trace {
    /// Contact tick and the arcade-bar health it took.
    contacts: Vec<(i32, u32)>,
    /// Every contact was guarded, or none was.
    guarded: Option<bool>,
    /// Ken's first tick back in neutral.
    ready: i32,
    /// Ryu's first tick back in his neutral or guard-ready stance after the last contact.
    ryu_ready: Option<i32>,
    /// How far Ryu's axis moved by the end of the trace.
    ryu_recoil: i32,
    /// Ryu's greatest height above the floor.
    ryu_peak: i32,
}

fn arcade_hp(sub_hp: u32) -> u32 {
    (1..=144)
        .find(|&hp| (hp * u32::from(FULL_SUB_HP) + 72) / 144 == sub_hp)
        .unwrap_or_else(|| panic!("{sub_hp} Sub-HP is not a whole arcade point"))
}

/// Runs a Shoryuken from `gap` pixels with room behind Ryu, for `ticks` ticks.
fn replay(base: &Fight, assets: &Assets, strength: usize, gap: u16, defense: u8, ticks: i32) -> (Trace, Fight) {
    let defense = pad(defense, None);
    let mut fight = base.clone();
    for _ in 0..8 {
        fight.step(assets, defense, pad(0, None), false);
    }
    let mut health = u32::from(fight.machine().era_fighters()[0].sub_hp);
    shoryuken(&mut fight, assets, defense, strength, gap, ROOM);
    let ryu_start = axis(&fight, Side::P1);
    let mut trace = Trace::default();
    for tick in 0..=ticks {
        if tick != 0 {
            fight.step(assets, defense, pad(0, None), false);
        }
        let read = fight.fight_read();
        if read.connected(Side::P2) {
            let left = u32::from(fight.machine().era_fighters()[0].sub_hp);
            trace.contacts.push((tick, arcade_hp(health - left)));
            health = left;
            let guarded = read.blocked[1];
            assert_eq!(*trace.guarded.get_or_insert(guarded), guarded, "contact at {tick}");
            assert_eq!(fight.machine().toggle_state().combo.contact_freeze[..2], [14, 14], "contact at {tick}");
            trace.ryu_ready = None;
        } else if !trace.contacts.is_empty() && trace.ryu_ready.is_none() {
            let combo = &fight.machine().toggle_state().combo;
            let reacting = combo.hitstun[0] != 0 || combo.blockstun[0] != 0 || combo.contact_freeze[0] != 0;
            if !reacting && read.fighters[0].move_id < REACTIONS {
                trace.ryu_ready = Some(tick);
            }
        }
        if trace.ready == 0 && read.fighters[1].move_id != SHORYUKEN {
            trace.ready = tick;
        }
        trace.ryu_peak = trace.ryu_peak.max(height(&fight, P1_STRUCT));
    }
    trace.ryu_recoil = ryu_start - axis(&fight, Side::P1);
    (trace, fight)
}

fn st_fight(ken: Loadout) -> (Assets, Fight) {
    versus([Some((Revision::St, Loadout::Standard)), Some((Revision::St, ken))])
}

#[test]
fn st_ken_shoryuken_whiffs_along_the_world_turbo_two_flight() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, base) = st_fight(loadout);
        let opener = if loadout == Loadout::Old { 1 } else { 0 };
        for (strength, first_active, last_active, floor, ready, peak, travel) in [
            (1, 2, 19, 36, 39, 35, 18),
            (2, 2, 27, 52, 55, 89, 68),
            // The arcade travels 126.5 px. The port rounds to its pixel grid.
            (3, opener, 29, 61, 64, 138, 127),
        ] {
            let tag = format!("{loadout:?}, punch {strength}");
            let mut fight = base.clone();
            shoryuken(&mut fight, &assets, pad(0, None), strength, 171, 0);
            let start = axis(&fight, Side::P2);
            let mut active = Vec::new();
            let (mut top, mut landed, mut exit) = (0, None, None);
            for tick in 0..=ready {
                if tick != 0 {
                    fight.step(&assets, pad(0, None), pad(0, None), false);
                }
                let read = fight.fight_read();
                assert!(!read.connected(Side::P2), "{tag}, tick {tick}");
                if read.fighters[1].attack_active {
                    active.push(tick);
                }
                let height = height(&fight, P2_STRUCT);
                top = top.max(height);
                if height == 0 && top > 0 {
                    landed.get_or_insert(tick);
                }
                if read.fighters[1].move_id != SHORYUKEN {
                    exit.get_or_insert(tick);
                }
            }
            assert_eq!(active, (first_active..=last_active).collect::<Vec<_>>(), "{tag}");
            assert_eq!((top, landed, exit), (peak, Some(floor), Some(ready)), "{tag}");
            assert_eq!(start - axis(&fight, Side::P2), travel, "{tag}");
        }
    }
}

#[test]
fn st_ken_shoryuken_clean_hits_follow_the_world_strike_phases() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, base) = st_fight(loadout);
        let opener = if loadout == Loadout::Old { 1 } else { 0 };
        // Strength, gap, contacts with arcade damage, Ken ready, Ryu idle, Ryu launched.
        let cases: [(usize, u16, &[(i32, u32)], i32, i32, bool); 6] = [
            (1, 55, &[(2, 30)], 53, 136, true),
            (2, 55, &[(2, 29)], 69, 38, false),
            (2, 32, &[(2, 29), (20, 12)], 83, 153, true),
            (3, 32, &[(opener, 4), (16, 27), (32, 11)], 106, 158, true),
            (3, 55, &[(2, 27), (18, 11)], 92, 144, true),
            (3, 72, &[(2, 27)], 78, 38, false),
        ];
        // At the natural 39 px entry gap, New Ken's third strike misses and Old Ken's lands.
        let natural: (&[(i32, u32)], i32) = if loadout == Loadout::Old {
            (&[(1, 4), (16, 27), (32, 11)], 106)
        } else {
            (&[(0, 4), (16, 27)], 92)
        };
        let (trace, _) = replay(&base, &assets, 3, 39, 0, 110);
        assert_eq!((&trace.contacts[..], trace.ready), natural, "{loadout:?}, natural gap");
        // A crouching Ryu at the same gap takes the same heavy strikes, and one light strike.
        let (trace, _) = replay(&base, &assets, 3, 39, 4, 110);
        assert_eq!((&trace.contacts[..], trace.ready), natural, "{loadout:?}, crouching");
        let (trace, _) = replay(&base, &assets, 1, 39, 4, 110);
        assert_eq!((&trace.contacts[..], trace.ready), (&[(2, 30)][..], 53), "{loadout:?}, crouching");
        for (strength, gap, contacts, ready, ryu_idle, launched) in cases {
            let tag = format!("{loadout:?}, punch {strength}, gap {gap}");
            let (trace, fight) = replay(&base, &assets, strength, gap, 0, 170);
            assert_eq!(trace.contacts, contacts, "{tag}");
            assert_eq!(trace.guarded, Some(false), "{tag}");
            assert_eq!(trace.ready, ready, "{tag}");
            assert_eq!(trace.ryu_ready, Some(ryu_idle), "{tag}");
            if launched {
                // The heavy punch launches Ryu lower than the other two.
                assert_eq!(trace.ryu_peak, if strength == 3 { 57 } else { 77 }, "{tag}");
            } else {
                // The heavy grounded recoil covers 78 px.
                assert_eq!((trace.ryu_recoil, trace.ryu_peak), (78, 0), "{tag}");
            }
            assert_eq!(fight.fight_read().fighters[1].move_id, 1, "{tag}");
        }
    }
}

#[test]
fn st_ken_shoryuken_chips_a_crouch_guard_on_each_world_strike() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, base) = st_fight(loadout);
        let opener = if loadout == Loadout::Old { 1 } else { 0 };
        // Old Ken's later opener leaves Ryu frozen when the second strike lands,
        // so he recoils one 8 px step less before the last 78 px recoil.
        let heavy_recoil = if loadout == Loadout::Old { 94 } else { 102 };
        // Strength, contacts with arcade chip, Ken ready, Ryu guard-ready, Ryu's recoil.
        let cases: [(usize, &[(i32, u32)], i32, i32, i32); 3] = [
            (1, &[(2, 8), (20, 4)], 67, 55, 102),
            (2, &[(2, 7), (20, 3)], 83, 55, 102),
            (3, &[(opener, 1), (16, 6), (32, 2)], 106, 67, heavy_recoil),
        ];
        for (strength, contacts, ready, guard_ready, recoil) in cases {
            let tag = format!("{loadout:?}, punch {strength}");
            let (trace, _) = replay(&base, &assets, strength, 55, 6, 110);
            assert_eq!(trace.contacts, contacts, "{tag}");
            assert_eq!(trace.guarded, Some(true), "{tag}");
            assert_eq!(trace.ready, ready, "{tag}");
            assert_eq!(trace.ryu_ready, Some(guard_ready), "{tag}");
            // Each guarded strike restarts the 78 px recoil from where the last one stopped.
            assert_eq!(trace.ryu_recoil, recoil, "{tag}");
        }
    }
}

#[test]
fn st_ken_shoryuken_follows_the_world_standing_guard_and_far_controls() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, base) = st_fight(loadout);
        // Ryu holds Back at 57 px: strength, contacts with chip, Ken ready, Ryu guard-ready.
        let guarded: [(usize, &[(i32, u32)], i32, i32); 3] = [
            (1, &[(2, 8)], 53, 38),
            (2, &[(2, 7)], 69, 38),
            (3, &[(2, 6), (18, 2)], 92, 53),
        ];
        for (strength, contacts, ready, guard_ready) in guarded {
            let tag = format!("{loadout:?}, punch {strength}, standing guard");
            let (trace, _) = replay(&base, &assets, strength, 57, 2, 110);
            assert_eq!(trace.contacts, contacts, "{tag}");
            assert_eq!(trace.guarded, Some(true), "{tag}");
            assert_eq!((trace.ready, trace.ryu_ready), (ready, Some(guard_ready)), "{tag}");
        }
        // A standing Ryu at 95 px: the light punch falls short and the others
        // reach him with the upper strike once Ken has travelled.
        let far: [(usize, &[(i32, u32)], i32, Option<i32>); 3] = [
            (1, &[], 39, None),
            (2, &[(11, 12)], 69, Some(145)),
            (3, &[(8, 11)], 78, Some(135)),
        ];
        for (strength, contacts, ready, ryu_idle) in far {
            let tag = format!("{loadout:?}, punch {strength}, 95 px");
            let (trace, _) = replay(&base, &assets, strength, 95, 0, 170);
            assert_eq!(trace.contacts, contacts, "{tag}");
            assert_eq!((trace.ready, trace.ryu_ready), (ready, ryu_idle), "{tag}");
        }
    }
}

#[test]
fn st_ken_shoryuken_replays_from_a_snapshot_between_strikes() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let (assets, base) = st_fight(Loadout::Standard);
    let (_, played) = replay(&base, &assets, 3, 32, 0, 170);
    let (_, mut resumed) = replay(&base, &assets, 3, 32, 0, 24);
    let saved = resumed.save_netplay_snapshot(24);
    for _ in 0..40 {
        resumed.step(&assets, pad(0, None), pad(1, None), false);
    }
    resumed.restore_netplay_snapshot(&saved);
    for _ in 24..170 {
        resumed.step(&assets, pad(0, None), pad(0, None), false);
    }
    assert_eq!(resumed.state_checksum(), played.state_checksum());
}

/// One punch at 32 px: its contact ticks and the attacker's first neutral tick.
fn close_contacts(styles: [Option<(Revision, Loadout)>; 2], strength: usize) -> (Vec<i32>, i32) {
    let (assets, mut fight) = versus(styles);
    for _ in 0..8 {
        fight.step(&assets, pad(0, None), pad(0, None), false);
    }
    shoryuken(&mut fight, &assets, pad(0, None), strength, 32, ROOM);
    let (mut contacts, mut ready) = (Vec::new(), 0);
    for tick in 0..=120 {
        if tick != 0 {
            fight.step(&assets, pad(0, None), pad(0, None), false);
        }
        let read = fight.fight_read();
        if read.connected(Side::P2) {
            contacts.push(tick);
        }
        if ready == 0 && read.fighters[1].move_id != SHORYUKEN {
            ready = tick;
        }
    }
    (contacts, ready)
}

#[test]
fn earlier_revisions_and_faithful_1987_keep_their_shoryuken() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let ryu = Some((Revision::St, Loadout::Standard));
    for (styles, contact, ready) in [
        ([ryu, Some((Revision::Ce, Loadout::Standard))], 4, 58),
        ([ryu, Some((Revision::Hf, Loadout::Standard))], 3, 54),
        ([None, None], 4, 57),
    ] {
        for strength in 1..=3 {
            assert_eq!(close_contacts(styles, strength), (vec![contact], ready), "{styles:?}, punch {strength}");
        }
    }
}

#[test]
fn st_ken_keeps_the_world_shoryuken_strikes_against_other_defenders() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    // A Faithful 1987 Ryu and an Old Super Turbo Ryu at 32 px take every
    // strike: the grounding strikes leave them standing for the next one.
    // Their own reactions decide when the later strikes reach them.
    for ryu in [None, Some((Revision::St, Loadout::Old))] {
        for (strength, strikes, ready) in [(1, 1, 39), (2, 2, 55), (3, 3, 64)] {
            let (contacts, exit) = close_contacts([ryu, Some((Revision::St, Loadout::Standard))], strength);
            assert_eq!(contacts.len(), strikes, "{ryu:?}, punch {strength}: {contacts:?}");
            assert_eq!(contacts[0], if strength == 3 { 0 } else { 2 }, "{ryu:?}, punch {strength}");
            // Each contact holds Ken for the 14-tick freeze.
            assert_eq!(exit, ready + 14 * contacts.len() as i32, "{ryu:?}, punch {strength}");
        }
    }
}
