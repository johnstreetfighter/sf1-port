//! Birdie and Eagle's native Alpha kits through the Local Style Card.

#[path = "support/alpha.rs"]
#[allow(dead_code)]
mod alpha;

use alpha::{
    load_fighter_art, local_fight, local_select_with_fighter, pad, DOWN, LEFT, LP, RIGHT, UP,
};
use sf_engine::combo::throw::{THROWN_MOVE, THROW_MOVE};
use sf_engine::data_bank::GROUND;
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_engine::front::PlayerInputs;
use sf_engine::impact::{ImpactDirection, ImpactFighter, ImpactKind};
use sf_engine::moves::{ButtonClass, Dir, MoveInput, Strength};
use sf_engine::roster::TournamentFighter::{self, Birdie, Eagle};
use sf_video::decode::decode_sprite_tile;
use sf_video::romset::Assets;
use sf_video::sprite_bank::sprite_slot;

const MP: usize = 1;
const HP: usize = 2;
const LK: usize = 3;
const QCF: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT];
const QCB: &[u8] = &[DOWN, DOWN | LEFT, LEFT];
const HCF: &[u8] = &[LEFT, DOWN | LEFT, DOWN, DOWN | RIGHT, RIGHT];
const DP: &[u8] = &[RIGHT, DOWN, DOWN | RIGHT];
const QCB_X2: &[u8] = &[DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT];
const QCF_X2: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];
const QCF_DP: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT];
const SPIN: &[u8] = &[RIGHT, DOWN, LEFT, UP];

fn bout(
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
) -> Option<(Fight, Assets)> {
    let select = local_select_with_fighter(GaugeMode::EraDefault, fighter, revision, loadout)?;
    let slots: &[u8] = match fighter {
        Birdie => &[0x1a, 0x1b, 0x29, 0x2a, 0x2c, 0x2d],
        Eagle => &[0x1a, 0x1f, 0x29, 0x2a, 0x2c, 0x2d, 0x2f],
        _ => unreachable!(),
    };
    let bank = select.data_bank().expect("a fighter move bank");
    for &slot in slots {
        assert!(
            bank.move_header(fighter, slot).is_some(),
            "{fighter:?} slot {slot:#04x}"
        );
    }
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let mut fight = session.fight().expect("a local bout").clone();
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter =
        if loadout == Loadout::XIsm { 48 } else { 144 };
    fight.restore_netplay_snapshot(&snapshot);
    let art = load_fighter_art(session.assets(), fighter);
    Some((fight, session.assets().with_sprite_bank(&art.bank)))
}

fn step(fight: &mut Fight, assets: &Assets, input: PlayerInputs) {
    fight.step(assets, input, PlayerInputs::default(), false);
}

fn command(fight: &mut Fight, assets: &Assets, motion: &[u8], button: usize) {
    for (index, &direction) in motion.iter().enumerate() {
        step(
            fight,
            assets,
            pad(direction, (index + 1 == motion.len()).then_some(button)),
        );
    }
}

fn approach(fight: &mut Fight, assets: &Assets) {
    for _ in 0..120 {
        let read = fight.machine().diagnostic_telemetry();
        if read.p1.center_x.abs_diff(read.p2.center_x) <= 48 {
            step(fight, assets, PlayerInputs::default());
            return;
        }
        step(fight, assets, pad(RIGHT, None));
    }
    panic!("fighters did not reach close range");
}

fn check_floor(fight: &Fight, assets: &Assets, slot: u8, frame: usize) {
    let machine = fight.machine();
    let read = machine.diagnostic_telemetry().p1;
    let ram = &machine.state.objectram;
    let word = |at| u16::from_be_bytes([ram[at], ram[at + 1]]);
    let e = usize::from(read.entity_offset);
    let attr = word(e + 2);
    let rows = i32::from(ram[e + 0xc]) + 1;
    let cell = if attr & 0x400 != 0 { 32 } else { 16 };
    if i32::from(word(e + 4) as i16) + rows * cell <= GROUND {
        return;
    }

    // The new catalogue poses include transparent padding in their last row.
    // Check the actual pens when the tile rectangle extends beyond the floor.
    let entries = usize::try_from(rows).unwrap() * (usize::from(ram[e + 0xd]) + 1);
    assert!(
        entries <= 24,
        "move {slot:#04x} frame {frame} has an invalid sprite grid"
    );
    for index in 0..entries {
        let at = e + index * 0x40;
        let code = word(at);
        let y = i32::from(word(at + 4) as i16);
        let flip_y = word(at + 2) & 0x200 != 0;
        let tiles: &[u16] = if cell == 32 { &[0, 1, 16, 17] } else { &[0] };
        for (quadrant, &offset) in tiles.iter().enumerate() {
            let tile = decode_sprite_tile(&assets.gfx3, sprite_slot(code + offset));
            for (row, pens) in tile.iter().enumerate() {
                let py = (quadrant / 2 * 16 + row) as i32;
                let pen_y = y + if flip_y { cell - 1 - py } else { py };
                assert!(
                    pen_y < GROUND || pens.iter().all(|&pen| pen == 15),
                    "move {slot:#04x} frame {frame} draws below the floor at {pen_y}"
                );
            }
        }
    }
}

#[derive(Debug, Default)]
struct Outcome {
    damage: u16,
    hits: usize,
    caught: bool,
    mat_slams: usize,
    blocked: bool,
    airborne: bool,
    airborne_strike: bool,
}

fn observe(fight: &mut Fight, assets: &Assets, slot: u8, defense: u8) -> Outcome {
    let mut result = Outcome::default();
    let mut health = fight.machine().era_fighters()[1].sub_hp;
    assert_eq!(
        fight.fight_read().fighters[0].move_id,
        slot,
        "command starts the requested move"
    );
    for frame in 0..160 {
        let [attacker, defender] = fight.fight_read().fighters;
        if attacker.move_id == slot
            || (attacker.move_id == THROW_MOVE && defender.move_id == THROWN_MOVE)
        {
            check_floor(fight, assets, slot, frame);
        }
        let airborne = attacker.move_id == slot && attacker.state & 3 == 1;
        result.airborne |= airborne;
        result.caught |= defender.move_id == THROWN_MOVE;
        result.blocked |= defender.blockstun;
        let next_health = fight.machine().era_fighters()[1].sub_hp;
        result.airborne_strike |= airborne && (attacker.attack_active || next_health < health);
        result.hits += usize::from(next_health < health);
        result.damage += health.saturating_sub(next_health);
        health = next_health;
        if attacker.move_id != slot && defender.move_id != THROWN_MOVE {
            return result;
        }
        let before = fight.machine().era_fighters()[1].sub_hp;
        let output = fight.step(assets, PlayerInputs::default(), pad(defense, None), false);
        for cue in output.impacts.into_iter().filter(|cue| cue.kind == ImpactKind::MatSlam) {
            assert!(result.caught, "the grab must precede the slam");
            assert!(fight.machine().era_fighters()[1].sub_hp < before);
            assert_eq!(cue.fighter, ImpactFighter::P2);
            assert_eq!(cue.direction, ImpactDirection::Down);
            assert_eq!(cue.y, GROUND as u16);
            result.mat_slams += 1;
        }
    }
    panic!("move {slot:#04x} did not recover");
}

#[test]
fn birdie_charges_and_releases_his_alpha_specials() {
    let Some((base, assets)) = bout(Birdie, Revision::Sfz3, Loadout::AIsm) else {
        return;
    };
    let mut head = base.clone();
    for _ in 0..44 {
        step(&mut head, &assets, pad(LEFT, None));
    }
    command(&mut head, &assets, &[RIGHT], LP);
    observe(&mut head, &assets, 0x1a, 0);

    let mut horn = base.clone();
    let mut pair = pad(0, Some(LP));
    pair.buttons[MP] = true;
    for _ in 0..44 {
        step(&mut horn, &assets, pair);
    }
    step(&mut horn, &assets, PlayerInputs::default());
    observe(&mut horn, &assets, 0x1b, 0);

    let mut the_birdie = base;
    approach(&mut the_birdie, &assets);
    for _ in 0..44 {
        step(&mut the_birdie, &assets, pad(DOWN | LEFT, None));
    }
    command(&mut the_birdie, &assets, &[RIGHT, LEFT, RIGHT], HP);
    assert_eq!(the_birdie.machine().era_fighters()[0].super_meter, 0);
    let result = observe(&mut the_birdie, &assets, 0x29, 0);
    assert!(
        result.hits >= 2,
        "The Birdie's successive charges hit: {result:?}"
    );
}

#[test]
fn birdie_chains_capture_at_close_range_and_whiff_at_a_distance() {
    let Some((base, assets)) = bout(Birdie, Revision::Sfz3, Loadout::AIsm) else {
        return;
    };
    for (slot, button) in [(0x2c, LP), (0x2d, LK)] {
        let mut close = base.clone();
        approach(&mut close, &assets);
        command(&mut close, &assets, SPIN, button);
        let caught = observe(&mut close, &assets, slot, RIGHT);
        assert!(caught.caught && caught.damage > 0 && !caught.blocked);
        assert_eq!(caught.mat_slams, 1);

        let mut far = base.clone();
        command(&mut far, &assets, SPIN, button);
        let whiff = observe(&mut far, &assets, slot, 0);
        assert!(!whiff.caught && whiff.damage == 0);
        assert_eq!(whiff.mat_slams, 0);
    }
}

#[test]
fn birdie_charge_and_spin_commands_come_out_at_move_list_show_cadence() {
    let Some((base, assets)) = bout(Birdie, Revision::Sfz3, Loadout::AIsm) else {
        return;
    };
    const CHARGE: &[Dir] = &[Dir::ChargeBack, Dir::Forward];
    const CIRCLE: &[Dir] = &[Dir::Forward, Dir::Down, Dir::Back, Dir::Up];
    for (slot, motion, button) in [
        (0x1a, CHARGE, ButtonClass::Punch),
        (0x1b, &[], ButtonClass::ReleasePair),
        (0x2c, CIRCLE, ButtonClass::Punch),
        (0x2d, CIRCLE, ButtonClass::Kick),
    ] {
        let mut fight = base.clone();
        let input = MoveInput { motion, button };
        let mut started = false;
        for frame in input.frames(true, Strength::Medium) {
            step(&mut fight, &assets, frame);
            started |= fight.fight_read().fighters[0].move_id == slot;
        }
        assert!(started, "Show starts Birdie's move {slot:#04x}");
    }
}

#[test]
fn bull_revenger_leaps_into_an_unblockable_capture() {
    let Some((mut fight, assets)) = bout(Birdie, Revision::Sfz3, Loadout::AIsm) else {
        return;
    };
    command(&mut fight, &assets, QCF_DP, HP);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
    let result = observe(&mut fight, &assets, 0x2a, RIGHT);
    assert!(
        result.airborne && result.caught && result.damage > 0 && !result.blocked,
        "{result:?}"
    );
}

#[test]
fn eagle_can_start_every_special_and_each_super_uses_its_own_motion() {
    let Some((mut base, assets)) = bout(Eagle, Revision::Sfz3, Loadout::AIsm) else {
        return;
    };
    approach(&mut base, &assets);
    for (slot, motion, button) in [
        (0x1a, QCB, LP),
        (0x2c, QCF, LP),
        (0x2d, HCF, LK),
        (0x2f, QCB, LK),
        (0x1f, DP, LP),
        (0x29, QCB_X2, HP),
        (0x2a, QCF_X2, HP),
    ] {
        let mut fight = base.clone();
        command(&mut fight, &assets, motion, button);
        let result = observe(&mut fight, &assets, slot, 0);
        if matches!(slot, 0x29 | 0x2a) {
            assert!(
                result.hits >= 2,
                "Super {slot:#04x} landed {} hits",
                result.hits
            );
        }
        if slot == 0x2f {
            assert!(
                result.airborne_strike,
                "Liverpool White strikes during its hop"
            );
        }
    }

    let mut blocked = base.clone();
    command(&mut blocked, &assets, QCB_X2, HP);
    assert!(
        observe(&mut blocked, &assets, 0x29, RIGHT).blocked,
        "Manchester Gold is blockable"
    );

    let mut standing = base.clone();
    command(&mut standing, &assets, QCB, LK);
    assert!(
        observe(&mut standing, &assets, 0x2f, RIGHT).blocked,
        "standing guard blocks Liverpool White"
    );

    let mut crouching = base;
    command(&mut crouching, &assets, QCB, LK);
    let result = observe(&mut crouching, &assets, 0x2f, DOWN | RIGHT);
    assert!(
        result.airborne_strike && result.damage > 0 && !result.blocked,
        "Liverpool White hits crouch guard: {result:?}"
    );
}

#[test]
fn eagle_x_ism_keeps_manchester_gold_and_hides_a_ism_exclusives() {
    let Some((base, assets)) = bout(Eagle, Revision::Sfz3, Loadout::XIsm) else {
        return;
    };
    let mut gold = base.clone();
    command(&mut gold, &assets, QCB_X2, LP);
    assert_eq!(gold.fight_read().fighters[0].move_id, 0x29);
    assert_eq!(gold.machine().era_fighters()[0].super_meter, 0);

    for (slot, motion) in [(0x2c, QCF), (0x1f, DP), (0x2a, QCF_X2)] {
        let mut fight = base.clone();
        command(&mut fight, &assets, motion, HP);
        assert_ne!(fight.fight_read().fighters[0].move_id, slot);
    }
}

#[test]
fn canterbury_blue_waits_for_punch_release() {
    let Some((mut fight, assets)) = bout(Eagle, Revision::Sfz3, Loadout::AIsm) else {
        return;
    };
    approach(&mut fight, &assets);
    command(&mut fight, &assets, QCF, LP);
    for _ in 0..30 {
        let eagle = fight.fight_read().fighters[0];
        assert_eq!(eagle.move_id, 0x2c);
        assert!(!eagle.attack_active, "Canterbury Blue is still winding up");
        step(&mut fight, &assets, pad(0, Some(LP)));
    }
    let result = observe(&mut fight, &assets, 0x2c, 0);
    assert!(result.hits > 0, "the strike comes out on release");
}
