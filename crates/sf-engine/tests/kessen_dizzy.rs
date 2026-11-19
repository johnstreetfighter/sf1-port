//! `SF KESSEN` stun and dizzy (KS17, #234): a Kessen attacker's hits build
//! stun, a full total dizzies the defender, and the dizzy ends with time or
//! a hit. Worked values are Ryu's Super Turbo Hadoken (7~13, its middle 10,
//! dizzy time 110) and Super Turbo's dizzy limits 30, 34 and 35.

#[path = "support/alpha.rs"]
mod alpha;
use alpha::{local_fight, local_fight_p2_ryu, local_select_with_fighter, local_select_with_p1, pad, DOWN, LEFT, RIGHT};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::era_fight::EraFighterState;
use sf_engine::front::fight::Fight;
use sf_engine::front::{GameSession, PlayerInputs};
use sf_engine::roster::TournamentFighter;
use sf_video::romset::Assets;

const IDLE: PlayerInputs = PlayerInputs { directions: [false; 4], buttons: [false; 6] };
const HADOKEN: u8 = 10;
const HADOKEN_TIME: u32 = 110;
/// Super Turbo's dizzy limits: 30, then 34 after a first dizzy.
const FIRST_LIMIT: u8 = 30;
const SECOND_LIMIT: u8 = 34;
/// A dizzy lasts 150 frames standing; the sway turns every 12; stun rests 60 after.
const DIZZY_FRAMES: u32 = 150;
const SWAY_FRAMES: usize = 12;
const REST_FRAMES: u32 = 60;

/// Kessen Ryu on Player 1 against SF1+ Ken, at the start of round 1.
fn bout() -> Option<(Fight, GameSession)> {
    let session = local_select_with_p1(GaugeMode::EraDefault, Revision::Kessen, Loadout::Balanced)?;
    let fight = local_fight(&session, Revision::Sf1Plus, Loadout::Standard).fight().expect("local fight").clone();
    Some((fight, session))
}

fn fighter(fight: &Fight, side: usize) -> EraFighterState {
    fight.machine().era_fighters()[side]
}

/// Throw a Hadoken from the side `side` (facing `toward`) and step until
/// it hits or `limit` frames pass. Returns the frames stepped.
fn hadoken(fight: &mut Fight, assets: &Assets, side: usize, toward: u8, limit: u32) -> u32 {
    let other = 1 - side;
    let health = fight.fight_read().fighters[other].health;
    let motion = [DOWN, DOWN | toward, toward];
    let mut frames = 0;
    for (i, direction) in motion.into_iter().enumerate() {
        let input = pad(direction, (i == 2).then_some(0));
        let (p1, p2) = if side == 0 { (input, IDLE) } else { (IDLE, input) };
        fight.step(assets, p1, p2, false);
        frames += 1;
    }
    while frames < limit {
        fight.step(assets, IDLE, IDLE, false);
        frames += 1;
        if fight.fight_read().fighters[other].health < health {
            return frames;
        }
    }
    frames
}

/// Step idle until the side is back in its stance and ready, up to 300 frames.
fn settle(fight: &mut Fight, assets: &Assets, side: usize) {
    for _ in 0..300 {
        let read = fight.fight_read().fighters[side];
        if read.move_id == 1 && !read.reacting() {
            return;
        }
        fight.step(assets, IDLE, IDLE, false);
    }
    panic!("side {side} did not settle");
}

/// Three Hadokens from Kessen Ryu: Ken's stun after each, his dizzy after the third.
fn three_hadokens(fight: &mut Fight, assets: &Assets) -> [u8; 3] {
    let mut seen = [0; 3];
    for stun in &mut seen {
        let frames = hadoken(fight, assets, 0, RIGHT, 120);
        assert!(frames < 120, "the Hadoken hits");
        *stun = fighter(fight, 1).stun();
        if !fighter(fight, 1).is_dizzy() {
            settle(fight, assets, 0);
        }
    }
    seen
}

/// The first sprite code of the side's pose grid: the picture on screen.
fn pose(fight: &Fight, side: usize) -> u16 {
    let machine = fight.machine();
    let telemetry = machine.diagnostic_telemetry();
    let entity = usize::from([telemetry.p1, telemetry.p2][side].entity_offset);
    let ram = &machine.state.objectram;
    u16::from_be_bytes([ram[entity], ram[entity + 1]])
}

#[test]
fn a_kessen_hit_adds_its_move_stun_and_the_total_empties_when_its_timer_runs_out() {
    let Some((mut fight, session)) = bout() else { return };
    let assets = session.assets();
    assert_eq!(fighter(&fight, 1).stun(), 0);
    hadoken(&mut fight, assets, 0, RIGHT, 120);
    assert_eq!(fighter(&fight, 1).stun(), HADOKEN);
    for _ in 0..HADOKEN_TIME - 5 {
        fight.step(assets, IDLE, IDLE, false);
    }
    assert_eq!(fighter(&fight, 1).stun(), HADOKEN, "the stun holds while its timer runs");
    for _ in 0..10 {
        fight.step(assets, IDLE, IDLE, false);
    }
    assert_eq!(fighter(&fight, 1).stun(), 0, "the stun empties when the timer ends");
}

#[test]
fn a_full_stun_dizzies_the_defender_who_cannot_act_and_recovers_with_time() {
    let Some((mut fight, session)) = bout() else { return };
    let assets = session.assets();
    assert_eq!(fighter(&fight, 1).stun_limit(), FIRST_LIMIT);
    assert_eq!(three_hadokens(&mut fight, assets), [10, 20, 0], "the third fills 30 and the stun resets");
    assert!(fighter(&fight, 1).is_dizzy());
    assert_eq!(fighter(&fight, 1).stun_limit(), SECOND_LIMIT, "the next dizzy needs 34");
    // He cannot act: Ken's walk and punches do nothing while dizzy.
    let mut frames = 0;
    while fighter(&fight, 1).is_dizzy() {
        let ken = if frames % 6 == 0 { pad(LEFT, Some(2)) } else { pad(LEFT, None) };
        fight.step(assets, IDLE, ken, false);
        frames += 1;
        let read = fight.fight_read().fighters[1];
        if !read.reacting() {
            assert!(!read.attacking() && !matches!(read.move_id, 2..=4), "a dizzy fighter cannot act: move {:#x}", read.move_id);
        }
        assert!(frames < 400, "the dizzy ends");
    }
    assert!(frames >= DIZZY_FRAMES, "it lasts its time standing: {frames}");
    assert_eq!(fighter(&fight, 1).stun(), 0);
    // Free again: his walk works.
    let before = fight.machine().diagnostic_telemetry().p2.center_x;
    for _ in 0..20 {
        fight.step(assets, IDLE, pad(LEFT, None), false);
    }
    assert_ne!(fight.machine().diagnostic_telemetry().p2.center_x, before, "he walks after the dizzy");
}

#[test]
fn a_dizzy_fighter_can_be_hit_and_the_hit_wakes_him_without_adding_stun() {
    let Some((mut fight, session)) = bout() else { return };
    let assets = session.assets();
    three_hadokens(&mut fight, assets);
    settle(&mut fight, assets, 0);
    assert!(fighter(&fight, 1).is_dizzy());
    let health = fight.fight_read().fighters[1].health;
    let frames = hadoken(&mut fight, assets, 0, RIGHT, 120);
    assert!(frames < 120 && fight.fight_read().fighters[1].health < health, "the Hadoken hits the dizzy fighter");
    assert!(!fighter(&fight, 1).is_dizzy(), "the hit ends the dizzy");
    assert_eq!(fighter(&fight, 1).stun(), 0, "stun rests after a dizzy");
}

#[test]
fn a_dizzy_fighter_sways_in_place_between_his_hit_reaction_and_his_stance() {
    let Some((mut fight, session)) = bout() else { return };
    let assets = session.assets();
    // His own standing hit-reaction poses, from a close jab.
    let stance = pose(&fight, 1);
    let start = fight.save_netplay_snapshot(0);
    for _ in 0..30 {
        fight.step(assets, pad(RIGHT, None), IDLE, false);
    }
    fight.step(assets, pad(0, Some(0)), IDLE, false);
    let mut reaction = Vec::new();
    for _ in 0..60 {
        fight.step(assets, IDLE, IDLE, false);
        let read = fight.fight_read().fighters[1];
        if read.reacting() && pose(&fight, 1) != stance && !reaction.contains(&pose(&fight, 1)) {
            reaction.push(pose(&fight, 1));
        }
    }
    assert!(!reaction.is_empty(), "Ken reacts to the jab");
    fight.restore_netplay_snapshot(&start);
    three_hadokens(&mut fight, assets);
    assert!(fighter(&fight, 1).is_dizzy());
    let mut poses = Vec::new();
    let mut xs = Vec::new();
    while fighter(&fight, 1).is_dizzy() {
        fight.step(assets, IDLE, IDLE, false);
        let read = fight.fight_read().fighters[1];
        if !read.reacting() {
            poses.push(pose(&fight, 1));
            xs.push(fight.machine().diagnostic_telemetry().p2.center_x);
        }
    }
    let swayed = poses.iter().filter(|p| reaction.contains(p)).count();
    assert!(swayed >= SWAY_FRAMES * 2, "he shows a hit-reaction pose: {swayed} of {}", poses.len());
    let stood = poses.iter().filter(|&&p| p == stance).count();
    assert!(stood >= SWAY_FRAMES * 2, "and his stance: {stood} of {}", poses.len());
    // The reaction pose leans him back; each return to his stance finds
    // him where he started, so the sway never walks him.
    let standing: Vec<i32> = poses.iter().zip(&xs).filter(|(&p, _)| p == stance).map(|(_, &x)| x).collect();
    assert!(standing.windows(2).all(|pair| pair[0] == pair[1]), "he sways in place: {xs:?}");
}

#[test]
fn stun_resets_at_the_start_of_a_round() {
    use sf_engine::fighter::{P2_STRUCT, WORK_BASE};
    use sf_engine::tasks::TIME_DIGITS;

    let Some((mut fight, session)) = bout() else { return };
    let assets = session.assets();
    three_hadokens(&mut fight, assets);
    while fighter(&fight, 1).is_dizzy() {
        fight.step(assets, IDLE, IDLE, false);
    }
    for _ in 0..REST_FRAMES {
        fight.step(assets, IDLE, IDLE, false);
    }
    hadoken(&mut fight, assets, 0, RIGHT, 120);
    assert_eq!(fighter(&fight, 1).stun(), HADOKEN, "stun builds again after its rest");
    // Ken loses round 1 on a timeout.
    let mut snapshot = fight.save_netplay_snapshot(0);
    let health = (P2_STRUCT + 6 - WORK_BASE) as usize;
    snapshot.machine.work[health] = 1;
    snapshot.machine.work[health + 1] = 0;
    snapshot.machine.toggle_state.era.fighters[1].sub_hp = 20;
    let timer = (TIME_DIGITS - WORK_BASE) as usize;
    snapshot.machine.work[timer..timer + 3].fill(0);
    fight.restore_netplay_snapshot(&snapshot);
    for _ in 0..900 {
        fight.step(assets, IDLE, IDLE, false);
        if fight.machine().work_ram()[0x400a] == 1 && fight.fight_read().fighters[1].health > 1 {
            break;
        }
    }
    assert_eq!(fight.machine().work_ram()[0x400a], 1, "round 2 starts");
    assert_eq!(fighter(&fight, 1).stun(), 0, "a new round has no stun");
    assert!(!fighter(&fight, 1).is_dizzy());
    assert_eq!(fighter(&fight, 1).stun_limit(), SECOND_LIMIT, "the match's dizzy still raises the limit");
}

#[test]
fn a_kessen_fighter_is_not_dizzied_by_an_attacker_whose_style_deals_no_stun() {
    let Some(select) = local_select_with_fighter(GaugeMode::EraDefault, TournamentFighter::Ken, Revision::Sf1Plus, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight_p2_ryu(&select, Revision::Kessen, Loadout::Balanced);
    let mut fight = session.fight().expect("local fight").clone();
    let assets = session.assets();
    assert!(fighter(&fight, 1).is_kessen());
    let mut hits = 0;
    for _ in 0..4 {
        let health = fight.fight_read().fighters[1].health;
        hadoken(&mut fight, assets, 0, RIGHT, 150);
        if fight.fight_read().fighters[1].health < health {
            hits += 1;
        }
        assert_eq!(fighter(&fight, 1).stun(), 0, "SF1+ Ken deals no stun");
        assert!(!fighter(&fight, 1).is_dizzy());
        settle(&mut fight, assets, 0);
        settle(&mut fight, assets, 1);
    }
    assert!(hits >= 3, "Ken's Hadokens hit: {hits}");
}


