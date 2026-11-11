//! The gate on synthetic object and palette RAM: no ROM is needed. The
//! session tests (`tests/mirror_palettes.rs`) play real fights.

use super::*;
use crate::config::{MatchConfig, RulesetPreset};
use crate::fighter::{Program, P1_STRUCT};
use crate::machine::MachineState;
use crate::roster::TournamentFighter;

/// The first side's region and the second's, in object RAM as a fight has
/// them.
const P1_REGION: u16 = 0xa00;
const P2_REGION: u16 = 0x400;
/// The 12 effect objects start here.
const EFFECT_FIRST: u16 = 0x100;
const EFFECT_OWNER: u32 = 0x20;

fn world(config: MatchConfig) -> (Program, MachineState) {
    let mut program = Program::power_on(vec![0; 0x60000]);
    program.match_config = config;
    let state = MachineState::new(vec![0; 0x1000], vec![0; 0x2000], vec![0x11; 0x800], 0, 0, 0, false, 0xff, 0);
    (program, state)
}

fn same_fighter(f: TournamentFighter, mode: PlayMode) -> MatchConfig {
    let mut config = MatchConfig::default();
    config.roster.full_cast = true;
    config.roster.mode = mode;
    config.roster.p1 = f;
    config.roster.p2 = f;
    config
}

/// A fight on screen in game mode `game_mode`: both fighters present, every
/// sprite of both regions drawn (position set) in `body`, with attribute
/// bits 0x500 (big, flipped) beside the palette. The second side is P2's
/// structure, or the CPU's in game mode 2 (its table set to `cpu_table`).
fn fight(bus: &mut Bus, game_mode: u8, body: u16, cpu_table: u32) -> u32 {
    bus.wb(GAME_MODE, game_mode);
    let second = if game_mode == 2 { CPU_STRUCT } else { P2_STRUCT };
    bus.wl(CPU_STRUCT + 0x12, cpu_table);
    for (ps, region) in [(P1_STRUCT, P1_REGION), (second, P2_REGION)] {
        bus.ww(ps + 0x10, region);
        bus.set_eb(region, 8, 2);
        for k in 0..REGION_ENTRIES {
            let s = region + k * ENTRY;
            bus.set_ew(s, 2, 0x500 | body);
            bus.set_ew(s, 4, 0x40);
            bus.set_ew(s, 6, 0x80);
        }
    }
    second
}

fn attr(bus: &Bus, s: u16) -> u16 {
    bus.ew(s, 2)
}

#[test]
fn the_gate_acts_only_in_a_same_fighter_fight_outside_faithful_1987() {
    let ryu = TournamentFighter::Ryu;
    for full_cast in [false, true] {
        for mode in [PlayMode::Arcade, PlayMode::Versus, PlayMode::Training, PlayMode::Cpu] {
            for same in [false, true] {
                for game_mode in [0u8, 1, 2, 4] {
                    let mut config = same_fighter(ryu, mode);
                    config.roster.full_cast = full_cast;
                    config.roster.p2 = if same { ryu } else { TournamentFighter::Ken };
                    let (mut program, mut state) = world(config);
                    let mut bus = program.bus(&mut state);
                    bus.wb(GAME_MODE, game_mode);
                    bus.wl(CPU_STRUCT + 0x12, ryu.move_table());
                    let acts = full_cast
                        && same
                        && matches!((mode, game_mode), (PlayMode::Versus, 1) | (PlayMode::Training, 1) | (PlayMode::Training, 2));
                    assert_eq!(
                        bus.mirror_match().is_some(),
                        acts,
                        "full cast {full_cast}, {mode:?}, same {same}, game mode {game_mode}"
                    );
                }
            }
        }
    }
    // Training's CPU fight: the second side is the CPU's structure, and only
    // when the CPU is the fighter; Local's and the dummy's is P2's.
    let (mut program, mut state) = world(same_fighter(ryu, PlayMode::Training));
    let mut bus = program.bus(&mut state);
    bus.wb(GAME_MODE, 2);
    bus.wl(CPU_STRUCT + 0x12, TournamentFighter::Retsu.move_table());
    assert!(bus.mirror_match().is_none(), "the CPU is not the fighter");
    bus.wl(CPU_STRUCT + 0x12, ryu.move_table());
    assert_eq!(bus.mirror_match().map(|(_, side)| side), Some(CPU_STRUCT));
    bus.wb(GAME_MODE, 1);
    assert_eq!(bus.mirror_match().map(|(_, side)| side), Some(P2_STRUCT));
    // Faithful 1987 is never one, whatever its roster says and whether or
    // not FULL CAST has been switched on in the Roster tab.
    for mode in [PlayMode::Versus, PlayMode::Training] {
        for full_cast in [false, true] {
            let mut config = MatchConfig::faithful();
            config.roster.mode = mode;
            config.roster.full_cast = full_cast;
            config.roster.p1 = ryu;
            config.roster.p2 = ryu;
            let (mut program, mut state) = world(config);
            let mut bus = program.bus(&mut state);
            bus.wb(GAME_MODE, 1);
            assert!(bus.mirror_match().is_none(), "{mode:?}, full cast {full_cast}");
            // The same fight under any other preset is one.
            for preset in [RulesetPreset::DefinitiveOverhaul, RulesetPreset::Custom] {
                let mut config = config;
                config.preset = preset;
                config.roster.full_cast = true;
                let (mut program, mut state) = world(config);
                let mut bus = program.bus(&mut state);
                bus.wb(GAME_MODE, 1);
                assert!(bus.mirror_match().is_some(), "{mode:?} under {preset:?}");
            }
        }
    }
}

#[test]
fn the_pass_points_only_the_second_sides_body_at_the_slot() {
    let f = TournamentFighter::Ryu;
    let mirror = mirror_palette::for_fighter("ryu").unwrap();
    for (mode, game_mode) in [(PlayMode::Versus, 1), (PlayMode::Training, 1), (PlayMode::Training, 2)] {
        let (mut program, mut state) = world(same_fighter(f, mode));
        let mut bus = program.bus(&mut state);
        let second = fight(&mut bus, game_mode, 0, f.move_table());
        // Effects (Hadokens in the shared palettes, hit sparks) are not his
        // body: nobody's are touched.
        for (k, owner, palette) in [(0u16, second, 2u16), (1, second, 4), (2, P1_STRUCT, 2), (3, second, 13)] {
            let s = EFFECT_FIRST + k * ENTRY;
            bus.set_eb(s, 8, 2);
            bus.set_el(s, EFFECT_OWNER, owner);
            bus.set_ew(s, 2, 0x400 | palette);
        }
        // A sprite of his region in another palette (a flash pose) is left.
        bus.set_ew(P2_REGION + 5 * ENTRY, 2, 0x500 | 4);
        let (before_effects, before_palette) = (bus.obj[0x100..0x400].to_vec(), bus.palette.to_vec());
        bus.mirror_palette_pass();
        for k in 0..REGION_ENTRIES {
            let want = if k == 5 { 0x500 | 4 } else { 0x500 | u16::from(mirror.slot) };
            assert_eq!(attr(&bus, P2_REGION + k * ENTRY), want, "{mode:?} mode {game_mode}: sprite {k}");
            assert_eq!(attr(&bus, P1_REGION + k * ENTRY), 0x500, "the first side, sprite {k}");
        }
        assert_eq!(&bus.obj[0x100..0x400], before_effects.as_slice(), "no effect is touched");
        // Palette RAM: the slot is the colours and the replaced palette's
        // pen 15; nothing else moved.
        let base = SPRITE_PALETTES + mirror.slot as usize * PALETTE_BYTES;
        for (pen, word) in mirror.pens.iter().enumerate() {
            assert_eq!(&bus.palette[base + 2 * pen..base + 2 * pen + 2], &word.to_be_bytes());
        }
        let from15 = SPRITE_PALETTES + mirror.from as usize * PALETTE_BYTES + 30;
        assert_eq!(&bus.palette[base + 30..base + 32], &before_palette[from15..from15 + 2]);
        for (i, (a, b)) in before_palette.iter().zip(bus.palette.iter()).enumerate() {
            assert!(a == b || (base..base + PALETTE_BYTES).contains(&i), "palette RAM byte {i:#x} changed outside the slot");
        }
    }
}

#[test]
fn the_pass_is_idempotent_and_waits_for_the_second_side_to_be_on_screen() {
    let (mut program, mut state) = world(same_fighter(TournamentFighter::Ken, PlayMode::Versus));
    let mut bus = program.bus(&mut state);
    fight(&mut bus, 1, 1, 0);
    bus.set_eb(P2_REGION, 8, 0);
    let (object, palette) = (bus.obj.to_vec(), bus.palette.to_vec());
    bus.mirror_palette_pass();
    assert!(bus.obj == object.as_slice() && bus.palette == palette.as_slice(), "off screen: nothing changes");
    bus.set_eb(P2_REGION, 8, 2);
    bus.mirror_palette_pass();
    let (object, palette) = (bus.obj.to_vec(), bus.palette.to_vec());
    bus.mirror_palette_pass();
    assert!(bus.obj == object.as_slice() && bus.palette == palette.as_slice(), "a second pass changes nothing");
    // A new animation frame writes the ROM's palette again, and the next
    // pass points it back.
    bus.set_ew(P2_REGION, 2, 0x100 | 1);
    bus.mirror_palette_pass();
    assert_eq!(attr(&bus, P2_REGION), 0x100 | 14);
}

#[test]
fn nothing_changes_outside_a_same_fighter_fight() {
    let ryu = TournamentFighter::Ryu;
    let mut cases: Vec<(&str, MatchConfig, u8)> = Vec::new();
    for mode in [PlayMode::Versus, PlayMode::Training] {
        let mut faithful = MatchConfig::faithful();
        faithful.roster.mode = mode;
        faithful.roster.p1 = ryu;
        faithful.roster.p2 = ryu;
        cases.push(("Faithful 1987", faithful, 1));
        let mut faithful_cast = faithful;
        faithful_cast.roster.full_cast = true;
        cases.push(("Faithful 1987 with FULL CAST on", faithful_cast, 1));
        let mut two = same_fighter(ryu, mode);
        two.roster.p2 = TournamentFighter::Ken;
        cases.push(("two fighters", two, 1));
        let mut no_cast = same_fighter(ryu, mode);
        no_cast.roster.full_cast = false;
        cases.push(("no full cast", no_cast, 1));
    }
    for mode in [PlayMode::Arcade, PlayMode::Cpu] {
        cases.push(("another play mode", same_fighter(ryu, mode), 1));
    }
    cases.push(("Local's one-player game mode", same_fighter(ryu, PlayMode::Versus), 2));
    cases.push(("game mode 4", same_fighter(ryu, PlayMode::Training), 4));
    for (what, config, game_mode) in cases {
        let (mut program, mut state) = world(config);
        let mut bus = program.bus(&mut state);
        fight(&mut bus, game_mode, 0, ryu.move_table());
        let (object, palette) = (bus.obj.to_vec(), bus.palette.to_vec());
        bus.mirror_palette_pass();
        assert!(bus.obj == object.as_slice() && bus.palette == palette.as_slice(), "{what}");
    }
}

#[test]
fn every_fighters_body_moves_to_the_slot() {
    for f in TournamentFighter::ALL {
        let mirror = mirror_palette::for_fighter(f.name()).unwrap();
        assert_eq!(mirror.from, f.sprite_palette(), "{f:?}");
        let (mut program, mut state) = world(same_fighter(f, PlayMode::Versus));
        let mut bus = program.bus(&mut state);
        fight(&mut bus, 1, u16::from(f.sprite_palette()), 0);
        bus.mirror_palette_pass();
        for k in 0..REGION_ENTRIES {
            assert_eq!(attr(&bus, P2_REGION + k * ENTRY) & 0xf, u16::from(mirror.slot), "{f:?} sprite {k}");
        }
        assert_eq!(attr(&bus, P1_REGION) & 0xf, u16::from(f.sprite_palette()), "{f:?}'s first side");
    }
}
