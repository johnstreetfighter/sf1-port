//! Gen's Alpha stances and commands through the native fight loop.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod alpha;

use alpha::{local_fight, local_select_with_fighter, pad, DOWN, LEFT, LP, RIGHT, UP};
use sf_engine::data_bank::GROUND;
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::era_fight::SUB_HP_PER_POINT;
use sf_engine::fight_reader::FIGHTER_SLOTS;
use sf_engine::fighter::{P2_STRUCT, WORK_BASE};
use sf_engine::front::fight::{Fight, FightFrame};
use sf_engine::front::{GameSession, PlayerInputs};
use sf_engine::roster::TournamentFighter;
use sf_engine::tasks::TIME_DIGITS;
use sf_video::announcer::AnnouncerLine;
use sf_video::callouts::{CalloutKind, Side};
use sf_video::romset::Assets;

const HP: usize = 2;
const HK: usize = 5;
const QCF_X2: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];
const QCB_X2: &[u8] = &[DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT];
const QCF_DP: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT];
const DP: &[u8] = &[RIGHT, DOWN, DOWN | RIGHT];
const CRANE_GUARD: u16 = 0x212c;
const MANTIS_GUARD: u16 = 0x2000;
const FULL_A_ISM: u8 = 144;

fn local_gen(loadout: Loadout) -> Option<(GameSession, Fight)> {
    let select = local_select_with_fighter(
        GaugeMode::EraDefault,
        TournamentFighter::Gen,
        Revision::Sfz3,
        loadout,
    )?;
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let fight = session.fight().expect("the local fight").clone();
    assert_eq!(fight.machine().era_fighters()[0].style.loadout, loadout);
    assert_eq!(
        fight.machine().era_fighters()[0].fighter,
        Some(TournamentFighter::Gen)
    );
    Some((session, fight))
}

fn step(fight: &mut Fight, assets: &Assets, p1: PlayerInputs) -> FightFrame {
    fight.step(assets, p1, PlayerInputs::default(), false)
}

fn rest(fight: &mut Fight, assets: &Assets, frames: usize) {
    for _ in 0..frames {
        step(fight, assets, PlayerInputs::default());
    }
}

fn switch(fight: &mut Fight, assets: &Assets, crane: bool) -> u8 {
    let mut p1 = PlayerInputs::default();
    let first = if crane { 3 } else { 0 };
    p1.buttons[first..first + 3].fill(true);
    step(fight, assets, p1);
    let transition = move_id(fight);
    rest(fight, assets, 12);
    transition
}

fn command(fight: &mut Fight, assets: &Assets, directions: &[u8], button: usize) -> FightFrame {
    let (&last, before) = directions.split_last().expect("a direction sequence");
    for &direction in before {
        step(fight, assets, pad(direction, None));
    }
    step(fight, assets, pad(last, Some(button)))
}

fn set_meter(fight: &mut Fight, amount: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = amount;
    fight.restore_netplay_snapshot(&snapshot);
}

fn move_id(fight: &Fight) -> u8 {
    fight.fight_read().fighters[0].move_id
}

fn grid(fight: &Fight) -> (u16, i32) {
    let ram = &fight.machine().state.objectram;
    let entity = usize::from(FIGHTER_SLOTS[0]);
    let word = |offset| u16::from_be_bytes([ram[entity + offset], ram[entity + offset + 1]]);
    let cell = if word(2) & 0x400 != 0 { 32 } else { 16 };
    let bottom = i32::from(word(4) as i16) + (i32::from(ram[entity + 0xc]) + 1) * cell;
    (word(0), bottom)
}

fn assert_move_floor(fight: &mut Fight, assets: &Assets, slot: u8, flies: bool) {
    assert_eq!(move_id(fight), slot, "move {slot:#04x} starts");
    let mut rose = false;
    let mut active = false;
    let mut final_bottom = GROUND;
    for _ in 0..150 {
        if move_id(fight) != slot {
            break;
        }
        let bottom = grid(fight).1;
        assert!(bottom <= GROUND, "move {slot:#04x} draws down to {bottom}");
        final_bottom = bottom;
        rose |= bottom < GROUND;
        active |= fight.fight_read().fighters[0].attack_active;
        step(fight, assets, PlayerInputs::default());
    }
    assert_ne!(move_id(fight), slot, "move {slot:#04x} recovers");
    assert!(active, "move {slot:#04x} has an active strike");
    assert_eq!(rose, flies, "move {slot:#04x} altitude");
    assert_eq!(
        final_bottom, GROUND,
        "move {slot:#04x} recovers on the floor"
    );
    rest(fight, assets, 2);
}

fn assert_super(fight: &Fight, frame: &FightFrame, slot: u8, name: &'static str) {
    assert_eq!(move_id(fight), slot);
    assert!(fight.machine().era_super_freeze() > 0);
    assert!(frame.announcements.contains(&AnnouncerLine::Super));
    assert!(fight
        .callouts()
        .frame()
        .side(Side::P1)
        .words
        .contains(&Some(CalloutKind::Super { name })));
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
}

#[test]
fn stance_chords_and_crane_charge_specials_use_the_native_bank() {
    let Some((session, mut fight)) = local_gen(Loadout::AIsm) else {
        return;
    };
    assert_eq!(grid(&fight).0, MANTIS_GUARD);
    assert_eq!(switch(&mut fight, session.assets(), true), 0x1f);
    assert_eq!(grid(&fight).0, CRANE_GUARD);

    for (charge, release, button, slot, flies) in
        [(LEFT, RIGHT, LP, 0x2d, false), (DOWN, UP, HK, 0x2f, true)]
    {
        for _ in 0..48 {
            step(&mut fight, session.assets(), pad(charge, None));
        }
        command(&mut fight, session.assets(), &[release], button);
        assert_move_floor(&mut fight, session.assets(), slot, flies);
    }

    assert_eq!(switch(&mut fight, session.assets(), false), 0x1f);
    assert_eq!(grid(&fight).0, MANTIS_GUARD);
    for _ in 0..3 {
        step(&mut fight, session.assets(), pad(0, Some(LP)));
        rest(&mut fight, session.assets(), 1);
    }
    step(&mut fight, session.assets(), pad(0, Some(LP)));
    assert_eq!(move_id(&fight), 0x1a, "Hyaku Renkou takes repeated punches");
}

#[test]
fn gekirou_continues_only_with_fresh_kick_taps_and_lands_at_each_height() {
    let Some((session, base)) = local_gen(Loadout::AIsm) else {
        return;
    };
    let assets = session.assets();
    let mut peaks = [GROUND; 2];
    for (index, tapped) in [false, true].into_iter().enumerate() {
        let mut fight = base.clone();
        command(&mut fight, assets, DP, HK);
        assert_eq!(move_id(&fight), 0x1b, "Gekirou uses kicks in Mantis");
        let mut final_bottom = GROUND;
        for frame in 0..150 {
            if move_id(&fight) != 0x1b {
                break;
            }
            final_bottom = grid(&fight).1;
            assert!(
                final_bottom <= GROUND,
                "Gekirou draws down to {final_bottom}"
            );
            peaks[index] = peaks[index].min(final_bottom);
            step(
                &mut fight,
                assets,
                pad(0, (tapped && frame % 3 == 1).then_some(HK)),
            );
        }
        assert_ne!(move_id(&fight), 0x1b, "Gekirou recovers");
        assert_eq!(final_bottom, GROUND, "Gekirou recovers on the floor");
    }
    assert_eq!(peaks, [GROUND - 24, GROUND - 100]);
}

#[test]
fn both_mantis_and_crane_supers_announce_spend_and_land_at_the_floor() {
    let Some((session, base)) = local_gen(Loadout::AIsm) else {
        return;
    };
    for (crane, motion, button, slot, name, airborne) in [
        (false, QCF_X2, HP, 0x29, "ZAN'EI", false),
        (false, QCB_X2, HP, 0x2a, "SHITENSHUU", false),
        (true, QCF_DP, HK, 0x2b, "JAKOUHA", false),
        (true, QCB_X2, HK, 0x2c, "KOUGA", true),
    ] {
        let mut fight = base.clone();
        if crane {
            switch(&mut fight, session.assets(), true);
        }
        set_meter(&mut fight, FULL_A_ISM);
        if airborne {
            let ground = command(&mut fight, session.assets(), motion, button);
            assert!(!ground.announcements.contains(&AnnouncerLine::Super));
            rest(&mut fight, session.assets(), 45);
            step(&mut fight, session.assets(), pad(UP, None));
            rest(&mut fight, session.assets(), 8);
        }
        let result = command(&mut fight, session.assets(), motion, button);
        assert_super(&fight, &result, slot, name);
        assert_move_floor(&mut fight, session.assets(), slot, crane);
    }
}

#[test]
fn a_clean_shitenshuu_hit_leaves_a_delayed_dizzy() {
    let Some((session, mut fight)) = local_gen(Loadout::AIsm) else {
        return;
    };
    let assets = session.assets();
    for _ in 0..100 {
        let positions = fight.machine().diagnostic_telemetry();
        if positions.p2.center_x - positions.p1.center_x <= 38 {
            break;
        }
        fight.step(assets, PlayerInputs::default(), pad(LEFT, None), false);
    }
    rest(&mut fight, assets, 12);
    let health = fight.machine().era_fighters()[1].sub_hp;
    set_meter(&mut fight, FULL_A_ISM);
    command(&mut fight, assets, QCB_X2, HP);
    for _ in 0..60 {
        if fight.machine().era_fighters()[1].sub_hp < health {
            break;
        }
        step(&mut fight, assets, PlayerInputs::default());
    }
    assert!(
        fight.machine().era_fighters()[1].sub_hp < health,
        "Shitenshuu connects"
    );
    rest(&mut fight, assets, 65);
    fight.step(assets, PlayerInputs::default(), pad(0, Some(LP)), false);
    assert!(
        !fight.fight_read().fighters[1].attacking(),
        "the delayed dizzy locks the pad"
    );
    rest(&mut fight, assets, 65);
    fight.step(assets, PlayerInputs::default(), pad(0, Some(LP)), false);
    assert!(
        fight.fight_read().fighters[1].attacking(),
        "the dizzy wears off"
    );
}

#[test]
fn x_ism_stays_in_mantis_and_only_has_zanei() {
    let Some((session, mut fight)) = local_gen(Loadout::XIsm) else {
        return;
    };
    switch(&mut fight, session.assets(), true);
    rest(&mut fight, session.assets(), 70);
    assert_eq!(grid(&fight).0, MANTIS_GUARD);
    set_meter(&mut fight, 48);
    let wrong = command(&mut fight, session.assets(), QCB_X2, HP);
    assert!(!wrong.announcements.contains(&AnnouncerLine::Super));
    rest(&mut fight, session.assets(), 70);
    let right = command(&mut fight, session.assets(), QCF_X2, HP);
    assert_super(&fight, &right, 0x29, "ZAN'EI");
}

#[test]
fn a_real_next_round_returns_gen_to_mantis() {
    let Some((session, mut fight)) = local_gen(Loadout::AIsm) else {
        return;
    };
    switch(&mut fight, session.assets(), true);
    assert_eq!(grid(&fight).0, CRANE_GUARD);
    let mut saved = fight.save_netplay_snapshot(0);
    let health = (P2_STRUCT + 6 - WORK_BASE) as usize;
    saved.machine.work[health] = 1;
    saved.machine.work[health + 1] = 0;
    saved.machine.toggle_state.era.fighters[1].sub_hp = SUB_HP_PER_POINT;
    let timer = (TIME_DIGITS - WORK_BASE) as usize;
    saved.machine.work[timer..timer + 3].fill(0);
    fight.restore_netplay_snapshot(&saved);
    for _ in 0..900 {
        step(&mut fight, session.assets(), PlayerInputs::default());
        if fight.machine().work_ram()[0x400a] == 1
            && fight.fight_read().fighters[1].health > 1
            && move_id(&fight) == 1
        {
            assert_eq!(grid(&fight).0, MANTIS_GUARD);
            return;
        }
    }
    panic!("round two did not reach idle");
}
