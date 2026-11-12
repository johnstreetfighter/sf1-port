use super::*;
use crate::config::{MatchConfig, RulesetPreset};
use crate::data_bank::DataBank;
use crate::fighter::Program;
use crate::machine::MachineState;
use crate::roster::TournamentFighter;
use sf_video::background_bank::BackgroundBank;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

fn arcade_program() -> Option<Program> {
    let roms = "../../tools/reference/local/roms/sf";
    if !Path::new(roms).is_dir() {
        return None;
    }
    let rom = sf_video::romset::load_program_rom(roms, "../../tools/reference/rom-manifest.json")
        .expect("local Street Fighter ROM");
    let bank = Arc::new(DataBank::build(&rom, &[]));
    let mut program = Program::power_on(rom);
    program.data_bank = Some(bank);
    program.match_config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    Some(program)
}

fn state() -> MachineState {
    MachineState::new(
        vec![0; 0x1000],
        vec![0; 0x2000],
        vec![0; 0x800],
        0,
        0,
        0,
        false,
        0xff,
        0,
    )
}

fn block_text(bus: &Bus, block: u32) -> Vec<u16> {
    let offset = u32::from(bus.rw(block));
    let rows = u32::from(bus.rb(block + 2));
    let cols = u32::from(bus.rb(block + 3));
    (0..rows)
        .flat_map(|row| (0..cols).map(move |col| bus.rw(0x800000 + offset + row * 0x80 + col * 2)))
        .collect()
}

fn enter_win_card(
    program: &mut Program,
    state: &mut MachineState,
    country: u8,
    step: u8,
) -> Vec<u16> {
    let mut bus = program.bus(state);
    bus.wb(MODE_DEMO, 1);
    bus.wb(GAME_MODE, 2);
    bus.wb(0xffc006, country);
    bus.wl(0xffc01c, 0x19908 + 0x28 * u32::from(country));
    bus.wb(0xffc003, step / 2);
    bus.wb(0xffc004, step % 2);
    bus.wb(0xffc009, 1);
    let mut screen_state = Screen::WinText { text: 0x158e4 };
    assert!(matches!(
        screen(&mut bus, &Io::idle(), &mut screen_state),
        Yield::Sleep(180)
    ));
    assert_eq!(screen_state, Screen::WinCard { held: false });
    block_text(&bus, if step == 9 { 0x159b2 } else { 0x158e4 })
}

#[test]
fn native_arcade_win_cards_have_distinct_quotes_and_a_rotated_stand_in() {
    let Some(mut program) = arcade_program() else {
        eprintln!("skipping: local ROMs not present");
        return;
    };
    let mut machine = state();
    let mut quotes = BTreeSet::new();
    let mut final_quotes = BTreeSet::new();
    for fighter in TournamentFighter::ALL {
        program.match_config.roster.p1 = fighter;
        let regular = enter_win_card(&mut program, &mut machine, 0, 0);
        let final_quote = enter_win_card(&mut program, &mut machine, 0, 9);
        assert!(
            quotes.insert(regular),
            "repeated regular quote for {fighter:?}"
        );
        assert!(
            final_quotes.insert(final_quote),
            "repeated final quote for {fighter:?}"
        );
    }

    program.match_config.roster.p1 = TournamentFighter::Retsu;
    // The China route meets Retsu at step four. Japan meets him at step zero.
    enter_win_card(&mut program, &mut machine, 1, 4);
    assert_eq!(machine.bg_scroll, 0x0900);
    assert_eq!(program.bus(&mut machine).rl(0xff8634), 0x1ae68);
    enter_win_card(&mut program, &mut machine, 1, 0);
    assert_eq!(machine.bg_scroll, 0x4180);
}

#[test]
fn native_arcade_ending_swaps_the_montage_slot_then_types_the_epilogue() {
    let Some(mut program) = arcade_program() else {
        eprintln!("skipping: local ROMs not present");
        return;
    };
    let mut backgrounds = BackgroundBank::new();
    let page = backgrounds.add_page();
    backgrounds.set_view(
        "ending:Retsu",
        LayerView {
            page,
            scroll: 0x80,
            palette: 0x1b068,
        },
    );
    program.backgrounds = Some(Arc::new(backgrounds));
    program.match_config.roster.p1 = TournamentFighter::Retsu;
    let mut machine = state();
    let mut bus = program.bus(&mut machine);
    bus.wb(MODE_DEMO, 1);
    bus.wb(GAME_MODE, 2);
    bus.wb(0xffc003, 4);
    bus.wb(0xffc004, 1);
    let mut scheduler = Scheduler::new();
    let mut at = Flow::AwaitMatchWipe { versus: false };
    let io = Io::idle();
    assert!(matches!(
        flow(&mut scheduler, &mut bus, &io, &mut at),
        Yield::Sleep(1)
    ));
    assert_eq!(at, Flow::EndingStart);
    assert!(matches!(
        flow(&mut scheduler, &mut bus, &io, &mut at),
        Yield::Sleep(120)
    ));
    assert_eq!(*bus.bg_scroll, 0x0900);
    assert_eq!(bus.rl(0xff8634), 0x1ae68);
    let glyph = sf_video::compositor::header_glyph('R').unwrap() | 0x9000;
    assert_eq!(bus.rw(0x8009b8), glyph);
    assert!(matches!(
        flow(&mut scheduler, &mut bus, &io, &mut at),
        Yield::Sleep(120)
    ));
    assert_eq!(*bus.bg_scroll, 0x2500);
    assert_eq!(bus.rl(0xff8634), 0x1b068);
    assert_ne!(bus.rw(0x8009b8), glyph);

    for _ in 0..9 {
        flow(&mut scheduler, &mut bus, &io, &mut at);
    }
    assert_eq!(at, Flow::EndingMessageSetup);
    assert!(matches!(
        flow(&mut scheduler, &mut bus, &io, &mut at),
        Yield::Sleep(5)
    ));
    assert_eq!(
        (*bus.bg_page, *bus.bg_scroll, bus.rw(TEXT_SOUND)),
        (page, 0x80, 0x000d)
    );
    let glyph = sf_video::compositor::small_glyph('H').unwrap() | 0x9000;
    assert_eq!(bus.rw(0x800838), glyph);
    for _ in 0..260 {
        if at == Flow::EndingMessageHold {
            break;
        }
        flow(&mut scheduler, &mut bus, &io, &mut at);
    }
    assert_eq!(at, Flow::EndingMessageHold);
    flow(&mut scheduler, &mut bus, &io, &mut at);
    assert_eq!(at, Flow::EndingStaffSetup);
    assert_eq!(bus.rw(0x800838), 0x103c);
}

#[test]
fn native_arcade_keeps_classic_shoto_ending_portraits() {
    let Some(mut program) = arcade_program() else {
        eprintln!("skipping: local ROMs not present");
        return;
    };
    let mut machine = state();
    let mut scheduler = Scheduler::new();
    for (fighter, expected) in [
        (TournamentFighter::Ryu, 0x7700),
        (TournamentFighter::Ken, 0x7880),
    ] {
        program.match_config.roster.p1 = fighter;
        let mut bus = program.bus(&mut machine);
        bus.wb(MODE_DEMO, 1);
        bus.wb(GAME_MODE, 2);
        let mut at = Flow::EndingMessageSetup;
        assert!(matches!(
            flow(&mut scheduler, &mut bus, &Io::idle(), &mut at),
            Yield::Sleep(5)
        ));
        assert_eq!(*bus.bg_scroll, expected);
        assert_eq!(*bus.bg_page, 0);
        assert_eq!(bus.rl(0xff8634), 0x1b868);
    }
}
