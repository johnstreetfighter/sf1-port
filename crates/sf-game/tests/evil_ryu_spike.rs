//! The private launcher drives the same match flow as the game.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sf_engine::combo::throw::{DAMAGE as THROW_DAMAGE, GET_UP_MOVE, RANGE as THROW_RANGE, THROWN_MOVE, THROW_MOVE};
use sf_engine::config::{AwakeningSide, EvilRyuPlaytest, MatchConfig, RulesetPreset};
use sf_engine::data_bank::{ALPHA_COUNTER_SLOT, ALPHA_SUPER_SECOND_SLOT, ALPHA_SUPER_THIRD_SLOT, DataBank, GROUND, SF2_SUPER_SLOT, SF2_SPECIAL_SLOT};
use sf_engine::era::{EraFamily, Loadout, Revision};
use sf_engine::moves::{ButtonClass, Dir, MoveInput, Strength, REGISTRY};
use sf_engine::fighter::{
    CPU_STRUCT, EVIL_RYU_ASHURA_BACK, EVIL_RYU_ASHURA_FORWARD, EVIL_RYU_RAGING_DEMON, P1_STRUCT,
};
use sf_engine::front::cabinet::{flow_position, Players, STAGE_START};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup};
use sf_engine::front::pause::PauseBox;
use sf_engine::front::{FrameInputs, GameSession, MenuItem, PlayerInputs, Transition};
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::tasks::TIME_DIGITS;
use sf_host::art::{find_pack, pack_folders, RECIPES, REVIEWS};
use sf_host::roms::{self, Candidate};
use sf_video::announcer::AnnouncerLine;
use sf_video::art::{load_art_from, Art, ArtSources};
use sf_video::background_bank::{BackgroundBank, LayerView};
use sf_video::compositor::PaletteTint;
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState, BAR_WIDTH, CORNER_BOTTOM, CORNER_EDGE, CORNER_HEIGHT, MAIN_ROW, SECOND_ROW};
use sf_video::render::{render_frame, render_frame_edited, render_frame_staged_with_tints, Frame, SpriteTint, TextEdits, TextSpan, HEIGHT, WIDTH};
use sf_video::romset::Assets;
use sf_video::vs_screens::vs_key;

#[path = "../src/evil_ryu_art.rs"]
mod evil_ryu_art;
#[path = "../src/evil_ryu_vs.rs"]
mod evil_ryu_vs;
#[path = "../src/evil_ryu_vs_name.rs"]
mod evil_ryu_vs_name;

const FIGHT_LOOP: u32 = 0x3eafa;
const WORK_RAM: u32 = 0xff8000;
const RYU_VICTORY_MOVE: u8 = 0x0c;
const RYU_VICTORY_NEXT_FRAME: u32 = 0x009c8;
const SHORYUKEN_MOVE: u8 = 0x19;
const HADOKEN_MOVE: u8 = 0x1a;
const SENPUKYAKU_MOVE: u8 = 0x22;
const FAKE_HADOKEN_MOVE: u8 = 0x23;
const AIR_TATSUMAKI_MOVE: u8 = 0x20;
const SUPER_STOCK: u8 = 48;
const ALPHA_FULL_SUPER: u8 = 3 * SUPER_STOCK;
const V_ISM_FULL_METER: u8 = 2 * SUPER_STOCK;
const AE_FULL_SUPER: u8 = 4 * SUPER_STOCK;
const AE_FOCUS_CHARGE: u8 = 0x2e;
const AE_FOCUS_ATTACK: u8 = 0x2f;
const AE_DASH: u8 = 0x1e;
const AE_ASHURA_BACK: u8 = 0x29;
const RYUSOKYAKU_MOVE: u8 = 0x21;
const SHORYUKEN_START_GAIN: u8 = 7;
const TATSUMAKI_START_GAIN: u8 = 6;
const EVIL_RYU_MAX_HP: u16 = 864;
const SAGAT_MAX_HP: u16 = 960;

fn life(ram: &[u8], player: u32) -> u8 {
    let at = (player - WORK_RAM) as usize;
    4 * ram[at + 7] + ram[at + 6]
}

fn assert_evil_ryu_label(session: &GameSession, frame: &Frame) {
    let machine = session.fight_machine().unwrap();
    let word: &[u8] = if matches!(
        machine.match_config().evil_ryu_playtest,
        Some(EvilRyuPlaytest::Kage | EvilRyuPlaytest::KageII),
    ) {
        b"KAGE"
    } else {
        b"EVIL RYU"
    };
    assert_fighter_label(session.fight().unwrap(), session.assets(), frame, 5, word);
}

fn assert_fighter_label(fight: &Fight, assets: &Assets, frame: &Frame, row: usize, word: &[u8]) {
    let scene = fight.machine().state.scene();
    let first = if word.len() > 7 { 21 } else { 22 };
    let source = 2 * (row * 64 + 22);
    let colour =
        u16::from_be_bytes(scene.videoram[source..source + 2].try_into().unwrap()) & !0x3ff;
    let y = row * 8 - 16;
    let x = first * 8 - 64;
    let edits = TextEdits {
        hidden: vec![TextSpan {
            row,
            col_first: 22,
            col_last: 28,
        }],
        added: word
            .iter()
            .enumerate()
            .map(|(i, &letter)| {
                let code = if letter == b' ' {
                    0x3c
                } else {
                    10 + u16::from(letter - b'A')
                };
                ((x + i * 8) as i32, y as i32, colour | code)
            })
            .collect(),
        ..TextEdits::default()
    };
    let expected = render_frame_edited(assets, &scene, &edits);
    for row in y..y + 8 {
        let start = row * WIDTH + x;
        let label = start..start + word.len() * 8;
        assert_eq!(
            &frame.argb[label.clone()],
            &expected.argb[label],
            "private fighter HUD row {row}"
        );
    }
}

fn capture_frame(var: &str, frame: &Frame) {
    if let Some(path) = std::env::var_os(var) {
        write_ppm(Path::new(&path), frame);
    }
}

fn write_ppm(path: &Path, frame: &Frame) {
    let mut file = BufWriter::new(File::create(path).expect("open the requested PPM capture"));
    write!(file, "P6\n{WIDTH} {HEIGHT}\n255\n").unwrap();
    for pixel in &frame.argb {
        file.write_all(&pixel.to_be_bytes()[1..]).unwrap();
    }
    file.flush().unwrap();
}

fn alpha_clip_dir(profile: EvilRyuPlaytest, name: &str) -> Option<PathBuf> {
    let env = match profile {
        EvilRyuPlaytest::Alpha2 => "SF_EVIL_RYU_ALPHA2_FRAMES_DIR",
        EvilRyuPlaytest::Alpha3A => "SF_EVIL_RYU_ALPHA3_A_FRAMES_DIR",
        EvilRyuPlaytest::Alpha3V => "SF_EVIL_RYU_ALPHA3_V_FRAMES_DIR",
        EvilRyuPlaytest::Alpha3X => "SF_EVIL_RYU_ALPHA3_X_FRAMES_DIR",
        EvilRyuPlaytest::Ae2012 => "SF_EVIL_RYU_AE2012_FRAMES_DIR",
        EvilRyuPlaytest::Ae2012Ultra2 => "SF_EVIL_RYU_AE2012_U2_FRAMES_DIR",
        EvilRyuPlaytest::Usf4 => "SF_EVIL_RYU_USF4_FRAMES_DIR",
        EvilRyuPlaytest::Kage => "SF_KAGE_SFV_FRAMES_DIR",
        EvilRyuPlaytest::KageII => "SF_KAGE_SFV_II_FRAMES_DIR",
        EvilRyuPlaytest::Sf6 => "SF_EVIL_RYU_SF6_FRAMES_DIR",
        EvilRyuPlaytest::UltraSf2 => return None,
    };
    std::env::var_os(env).map(|dir| PathBuf::from(dir).join(name))
}

fn write_clip(dir: &Path, frames: &[Frame]) {
    std::fs::create_dir_all(dir).unwrap();
    for (index, frame) in frames.iter().enumerate() {
        write_ppm(&dir.join(format!("{index:03}.ppm")), frame);
    }
}

struct AlphaClip {
    dir: Option<PathBuf>,
    frames: Vec<Frame>,
}

impl AlphaClip {
    fn new(fight: &Fight, name: &str, selected: bool) -> Self {
        let dir = fight.setup().config.evil_ryu_playtest
            .filter(|_| selected).and_then(|profile| alpha_clip_dir(profile, name));
        Self { dir, frames: Vec::new() }
    }

    fn push(&mut self, frame: &Frame) {
        if self.dir.is_some() {
            self.frames.push(frame.clone());
        }
    }

    fn idle_tail(&mut self, fight: &mut Fight, assets: &Assets, ticks: usize) {
        if self.dir.is_some() {
            let idle = PlayerInputs::default();
            for _ in 0..ticks {
                self.push(&fight.step(assets, idle, idle, false).frame);
            }
        }
    }

    fn finish(self) {
        if let Some(dir) = self.dir {
            write_clip(&dir, &self.frames);
        }
    }
}

fn assert_evil_ryu_vs_frame(session: &GameSession, frame: &Frame, stock_view: LayerView) {
    let bank = session.background_bank().unwrap();
    let mut scene = session.fight_machine().unwrap().state.scene();
    scene.bg_page = stock_view.page;
    scene.bg_scroll = stock_view.scroll;
    scene.paletteram[..0x200].copy_from_slice(bank.palette(stock_view.palette).unwrap());
    let stock = render_frame(&session.assets().with_background_bank(bank), &scene);

    let region_differs = |left, top, width, height| {
        (top..top + height).any(|y| {
            let start = y * WIDTH + left;
            let row = start..start + width;
            frame.argb[row.clone()] != stock.argb[row]
        })
    };
    assert!(region_differs(64, 32, 96, 96), "Evil Ryu's left bust is visible");
    assert!(region_differs(96, 144, 64, 32), "Evil Ryu's left name is visible");
    assert!(!region_differs(224, 32, 96, 96), "Sagat's bust matches the original");
    assert!(!region_differs(224, 144, 64, 32), "Sagat's name matches the original");
    // The one-pixel gap after E exposes the same two grays as the native gutter.
    assert_eq!(frame.argb[152 * WIDTH + 91], 0xff666666);
    assert_eq!(frame.argb[160 * WIDTH + 91], 0xff444444);
}

fn restart_round(session: &mut GameSession) {
    let mut pause = FrameInputs::default();
    pause.menu.start = true;
    assert_eq!(session.step(pause).screen, "pause");
    let mut down = FrameInputs::default();
    down.p1.directions[2] = true;
    for _ in 0..4 {
        session.step(FrameInputs::default());
        session.step(down);
    }
    let mut confirm = FrameInputs::default();
    confirm.menu.confirm = true;
    assert_ne!(session.step(confirm).screen, "pause", "Restart Round resumes play");
    let timer = (TIME_DIGITS - WORK_RAM) as usize;
    assert!((0..600).any(|_| {
        session.step(FrameInputs::default());
        let machine = session.fight_machine().unwrap();
        flow_position(machine) == FIGHT_LOOP
            && machine.arcade_opponent() == Some(TournamentFighter::Sagat)
            && machine.work_ram()[timer..timer + 3] == [1, 0, 0]
            && machine.era_fighters().map(|fighter| fighter.sub_hp) == [EVIL_RYU_MAX_HP, SAGAT_MAX_HP]
    }), "Restart Round returns to Sagat with fresh life and timer");
}

fn ashura_frame(session: &mut GameSession, motion: &'static [Dir], move_id: u8, buttons: ButtonClass) -> Frame {
    let fight = session.fight().unwrap();
    let facing_right = fighter_object(fight, P1_STRUCT)[0x37] & 4 == 0;
    let input = MoveInput { motion, button: buttons };
    let long_clip = matches!(fight.setup().config.evil_ryu_playtest,
        Some(EvilRyuPlaytest::Alpha3A | EvilRyuPlaytest::Alpha3V | EvilRyuPlaytest::Alpha3X))
        && buttons == ButtonClass::TriplePunch;
    let clip_name = match (move_id == EVIL_RYU_ASHURA_FORWARD, long_clip) {
        (true, true) => "ashura-forward-ppp",
        (false, true) => "ashura-backward-ppp",
        (true, false) => "ashura-forward",
        (false, false) => "ashura-backward",
    };
    let mut clip = AlphaClip::new(fight, clip_name, long_clip || buttons == ButtonClass::TripleKick);
    let frame = input.frames(facing_right, Strength::Light).into_iter()
        .chain(std::iter::repeat_n(Default::default(), 45))
        .find_map(|p1| {
            let output = session.step(FrameInputs { p1, ..FrameInputs::default() });
            clip.push(&output.frame);
            let machine = session.fight_machine().unwrap();
            let at = (P1_STRUCT + 0x10 - WORK_RAM) as usize;
            let entity = u16::from_be_bytes(machine.work_ram()[at..at + 2].try_into().unwrap()) as usize;
            let object = &machine.state.objectram[entity..];
            (object[0xa] == move_id && object[0xf] & 8 != 0).then_some(output.frame)
        }).unwrap_or_else(|| panic!("Ashura Senku {move_id:#04x} reaches protected travel in the Sagat fight"));
    if clip.dir.is_some() {
        clip.idle_tail(&mut session.fight().unwrap().clone(), session.fight_assets().unwrap(), 40);
    }
    clip.finish();
    frame
}

fn fighter_object(fight: &Fight, player: u32) -> &[u8] {
    let machine = fight.machine();
    let at = (player + 0x10 - WORK_RAM) as usize;
    let entity = u16::from_be_bytes(machine.work_ram()[at..at + 2].try_into().unwrap()) as usize;
    &machine.state.objectram[entity..]
}

fn center_x(fight: &Fight, player: u32) -> u16 {
    let object = fighter_object(fight, player);
    let left = u16::from_be_bytes([object[6], object[7]]);
    let attr = u16::from_be_bytes([object[2], object[3]]);
    let tile_width = if attr & 0x400 != 0 { 32 } else { 16 };
    left.wrapping_add((u16::from(object[0xd]) + 1) * tile_width / 2)
}

fn raging_demon_inputs(fight: &Fight) -> [PlayerInputs; 6] {
    let forward = usize::from(fighter_object(fight, P1_STRUCT)[0x37] & 4 != 0);
    let mut input = [PlayerInputs::default(); 6];
    input[0].buttons[0] = true;
    input[2].buttons[0] = true;
    for tick in &mut input[3..] {
        tick.directions[forward] = true;
    }
    input[4].buttons[3] = true;
    input[5].buttons[2] = true;
    input
}

struct DemonFrames {
    approach: Frame,
    grab: Frame,
    blackout: Frame,
    reveal: Frame,
    sequence: Vec<Frame>,
}

enum DemonOutcome {
    Whiff(Vec<Frame>),
    Landed(DemonFrames),
}

fn assert_demon_blackout(fight: &Fight, assets: &Assets, frame: &Frame) {
    let mut scene = fight.machine().state.scene();
    scene.active = 0x80;
    scene.paletteram[..2].fill(0);
    let kage_ii = fight.setup().config.evil_ryu_playtest == Some(EvilRyuPlaytest::KageII);
    let mut tints = Vec::new();
    for (slot, object) in scene.objectram.chunks_exact_mut(0x40).enumerate() {
        let demon_spark = (4..16).contains(&slot) && object[8] != 0 && object[0x2e] == 1;
        if !demon_spark {
            object[..8].fill(0);
        } else if kage_ii {
            tints.push(SpriteTint {
                entries: slot * 0x40..(slot + 1) * 0x40,
                tint: PaletteTint { color: 0xffff3038, strength: 128 },
            });
        }
    }
    let expected = render_frame_staged_with_tints(assets, &scene, &TextEdits::default(), &tints).0;
    assert!(frame.argb == expected.argb, "the blackout renders only live ROM Demon sparks");
}

fn try_raging_demon(fight: &mut Fight, assets: &Assets) -> Option<DemonOutcome> {
    let original_order = center_x(fight, P1_STRUCT).cmp(&center_x(fight, CPU_STRUCT));
    let style = fight.machine().era_fighters()[0].style;
    let x_ism = style.loadout == Loadout::XIsm;
    let sf4 = style.family == EraFamily::Sf4;
    let sf6 = fight.setup().config.evil_ryu_playtest == Some(EvilRyuPlaytest::Sf6);
    let kage_ii = fight.setup().config.evil_ryu_playtest == Some(EvilRyuPlaytest::KageII);
    let demon_move = match (sf6, kage_ii) {
        (true, _) => SF6_DEMON,
        (_, true) => KAGE_DEMON,
        _ => EVIL_RYU_RAGING_DEMON,
    };
    let record_command = x_ism || sf4 || kage_ii || sf6;
    let drive = fight.machine().era_fighters()[0].sf6_drive_points();
    let record_sequence = if fight.machine().era_fighters()[0].style.revision == Revision::St {
        std::env::var_os("SF_EVIL_RYU_DEMON_FRAMES_DIR").is_some()
    } else {
        fight.setup().config.evil_ryu_playtest.and_then(|profile| alpha_clip_dir(profile, "demon")).is_some()
    };
    let mut sequence = Vec::new();
    if record_command && record_sequence {
        sequence.push(fight.render(assets));
    }
    let mut saw_rush = false;
    let mut approach = None;
    let mut grab = None;
    let mut blackout = None;
    let mut blackout_ticks = 0;
    let mut health_at_rush = None;
    for p1 in raging_demon_inputs(fight).into_iter().chain(std::iter::repeat_n(PlayerInputs::default(), 120)) {
        let output = fight.step(assets, p1, PlayerInputs::default(), false);
        let read = fight.fight_read();
        let evil_ryu = read.fighters[0];
        let sagat = read.fighters[1];
        if !saw_rush && evil_ryu.move_id == demon_move {
            assert_super_gauge(fight, &output.frame, 0);
            if kage_ii {
                assert_eq!(kage_v_gauge(fight), (0, GaugeState::Normal), "Shun Goku Satsu ends Rikudo Osatsu");
            }
            if sf6 {
                assert_eq!(fight.machine().era_fighters()[0].sf6_drive_points(), drive, "Critical Art leaves Drive alone");
            }
            health_at_rush = Some([evil_ryu.health, sagat.health]);
        }
        saw_rush |= evil_ryu.move_id == demon_move;
        if !saw_rush {
            if record_command && record_sequence {
                sequence.push(output.frame);
            }
            continue;
        }
        if grab.is_none() && evil_ryu.move_id != demon_move && evil_ryu.move_id != THROW_MOVE {
            if record_command && record_sequence {
                sequence.push(output.frame);
            }
            return (approach.is_some() && evil_ryu.state & 0x80 == 0
                && health_at_rush == Some([evil_ryu.health, sagat.health]))
                .then_some(DemonOutcome::Whiff(sequence));
        }

        let frame = output.frame;
        if record_sequence {
            sequence.push(frame.clone());
        }
        let ryu_object = fighter_object(fight, P1_STRUCT);
        let lit = frame.argb.iter().filter(|&&pixel| pixel != 0xff000000).take(1_001).count() > 1_000;
        if evil_ryu.move_id == demon_move && approach.is_none()
            && u32::from_be_bytes(ryu_object[0x2c..0x30].try_into().unwrap()) == 0x00e40
        {
            assert!(lit, "the raised-knee approach is visible");
            assert_eq!(ryu_object[0xf] & 8, 0, "the Demon approach can be interrupted");
            approach = Some(frame.clone());
        }
        if lit && grab.is_none() && evil_ryu.move_id == THROW_MOVE && sagat.move_id == THROWN_MOVE {
            assert!(approach.is_some(), "a raised-knee approach precedes the visible grab");
            grab = Some((frame, sagat.health, fight.machine().era_fighters()[1].sub_hp));
        } else if !lit && grab.is_some() {
            blackout_ticks += 1;
            assert_demon_blackout(fight, assets, &frame);
            if blackout.is_none() && frame.argb.iter().any(|&pixel| {
                if kage_ii { pixel != 0xff000000 } else { matches!(pixel, 0xffdd0000 | 0xffcccccc) }
            }) {
                blackout = Some(frame);
            }
        } else if lit && blackout_ticks > 0 {
            assert_eq!(blackout_ticks, 60, "the stage stays black for one uninterrupted second");
            let (grab, hp_at_grab, sub_hp_at_grab) = grab.unwrap();
            if x_ism {
                assert_eq!(sub_hp_at_grab - fight.machine().era_fighters()[1].sub_hp, 448,
                    "X-ISM applies its native 12% damage bonus to the Demon slam");
            } else if sf4 {
                assert_eq!(sub_hp_at_grab - fight.machine().era_fighters()[1].sub_hp, 360,
                    "SFIV Demon deals its scaled 18 life");
            } else if kage_ii || sf6 {
                assert_eq!(sub_hp_at_grab - fight.machine().era_fighters()[1].sub_hp, 400,
                    "Shun Goku Satsu takes 400 native sub-HP");
            } else {
                assert_eq!(hp_at_grab - sagat.health, 20, "Demon's slam deals 20 life");
            }
            assert_super_gauge(fight, &frame, 0);
            assert_eq!(evil_ryu.move_id, RYU_VICTORY_MOVE, "Evil Ryu holds his victory pose at reveal");
            assert_eq!(
                u32::from_be_bytes(ryu_object[0x2c..0x30].try_into().unwrap()),
                RYU_VICTORY_NEXT_FRAME,
                "the reveal uses the native final victory pose",
            );
            assert_eq!(sagat.move_id, THROWN_MOVE, "Sagat is still down at reveal");
            assert_eq!(
                center_x(fight, P1_STRUCT).cmp(&center_x(fight, CPU_STRUCT)),
                original_order,
                "Raging Demon keeps the original sides",
            );
            let sagat_object = fighter_object(fight, CPU_STRUCT);
            let attr = u16::from_be_bytes(sagat_object[2..4].try_into().unwrap());
            let tile = if attr & 0x400 != 0 { 32 } else { 16 };
            let size = (
                tile * (usize::from(sagat_object[0xd]) + 1),
                tile * (usize::from(sagat_object[0xc]) + 1),
            );
            assert_eq!(size, (128, 32), "Sagat's reveal uses the wide lying pose");
            return Some(DemonOutcome::Landed(DemonFrames {
                approach: approach.unwrap(),
                grab,
                blackout: blackout.expect("the blackout contains at least one visible ROM spark"),
                reveal: frame,
                sequence,
            }));
        }
    }
    None
}

fn land_raging_demon(fight: &Fight, assets: &Assets) -> Option<(Fight, DemonFrames)> {
    // Sagat is a live CPU. Try from several approach frames so a jump or
    // a jab against the first attempt does not hide the native grab path.
    let mut approach = fight.clone();
    for step in 0..80 {
        let p1 = center_x(&approach, P1_STRUCT);
        let sagat = center_x(&approach, CPU_STRUCT);
        if step % 3 == 0 && p1.abs_diff(sagat) <= 140 {
            let mut candidate = approach.clone();
            if let Some(DemonOutcome::Landed(frames)) = try_raging_demon(&mut candidate, assets) {
                return Some((candidate, frames));
            }
        }
        let mut p1_input = PlayerInputs::default();
        p1_input.directions[usize::from(p1 > sagat)] = true;
        approach.step(assets, p1_input, PlayerInputs::default(), false);
    }
    None
}

fn check_raging_demon_recovery(mut after: Fight, assets: &Assets, mut frames: DemonFrames, record: bool) {
    if record {
        capture_frame("SF_EVIL_RYU_DEMON_APPROACH_CAPTURE", &frames.approach);
        capture_frame("SF_EVIL_RYU_DEMON_CAPTURE", &frames.grab);
        capture_frame("SF_EVIL_RYU_DEMON_BLACKOUT_CAPTURE", &frames.blackout);
        capture_frame("SF_EVIL_RYU_DEMON_REVEAL_CAPTURE", &frames.reveal);
    }

    let frames_dir = if record {
        std::env::var_os("SF_EVIL_RYU_DEMON_FRAMES_DIR").map(PathBuf::from)
    } else {
        after.setup().config.evil_ryu_playtest.and_then(|profile| alpha_clip_dir(profile, "demon"))
    };
    let original_order = center_x(&after, P1_STRUCT).cmp(&center_x(&after, CPU_STRUCT));
    let victory_sprite: [u8; 4] = fighter_object(&after, P1_STRUCT)[..4].try_into().unwrap();
    let lying_grid: [u8; 2] = fighter_object(&after, CPU_STRUCT)[0xc..=0xd].try_into().unwrap();
    let mut getup_started = None;
    let mut got_up = false;
    for tick in 1..=180 {
        let frame = after.step(assets, PlayerInputs::default(), PlayerInputs::default(), false).frame;
        let read = after.fight_read();
        let evil_ryu = read.fighters[0];
        let sagat = read.fighters[1];
        assert_eq!(
            center_x(&after, P1_STRUCT).cmp(&center_x(&after, CPU_STRUCT)),
            original_order,
            "the victory hold and wake-up keep both fighters on their original sides",
        );
        if sagat.move_id == GET_UP_MOVE {
            if getup_started.is_none() {
                assert_eq!(tick, 90, "Sagat starts getting up after 90 gameplay ticks");
                if record {
                    capture_frame("SF_EVIL_RYU_DEMON_GETUP_CAPTURE", &frame);
                }
                getup_started = Some(tick);
            }
        } else if getup_started.is_some() {
            got_up = true;
        } else {
            assert_eq!(sagat.move_id, THROWN_MOVE, "Sagat stays down before his get-up");
            assert_eq!(fighter_object(&after, CPU_STRUCT)[0xc..=0xd], lying_grid, "Sagat keeps his lying sprite grid");
            assert_eq!(evil_ryu.move_id, RYU_VICTORY_MOVE, "Evil Ryu keeps the victory pose during the hold");
            assert_eq!(fighter_object(&after, P1_STRUCT)[..4], victory_sprite, "the victory sprite stays visible");
        }
        if frames_dir.is_some() {
            frames.sequence.push(frame);
        }
        if got_up {
            break;
        }
    }
    assert!(got_up, "Sagat finishes his native get-up");

    if let Some(dir) = frames_dir {
        let mut tail = after.clone();
        for _ in 0..8 {
            frames.sequence.push(tail.step(assets, PlayerInputs::default(), PlayerInputs::default(), false).frame);
        }
        write_clip(&dir, &frames.sequence);
    }

    assert!((0..120).any(|_| {
        after.step(assets, PlayerInputs::default(), PlayerInputs::default(), false);
        after.fight_read().fighters[0].state & 0x80 == 0
    }), "Evil Ryu returns to player control after the Demon");
    assert!(after.machine().unrecovered().is_empty());
}

fn fight_with_meter(fight: &Fight, meter: u8) -> Fight {
    let mut fight = fight.clone();
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = meter;
    fight.restore_netplay_snapshot(&snapshot);
    fight
}

fn spike_assets() -> (Assets, Art) {
    let rom_dir = std::env::var_os("SF_EVIL_RYU_ROMS").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/reference/local/roms/sf")
    });
    let assets = roms::find(&[Candidate {
        label: rom_dir.display().to_string(),
        path: Some(rom_dir.clone()),
    }]).expect("the private playtest requires the local SF1 ROM set").assets;
    let bundled_roms = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../roms/sf");
    let pack = find_pack(&pack_folders(&[&bundled_roms, &rom_dir])).expect("the shipped SF1+ art pack");
    let art = load_art_from(&assets, &ArtSources {
        recipes: &RECIPES,
        reviews: Some(&REVIEWS),
        pack: Ok(&pack),
    }).expect("the shipped poses and throw streams load");
    (assets, art)
}

fn boot_spike(profile: EvilRyuPlaytest) -> (GameSession, Frame) {
    let (mut assets, art) = spike_assets();
    let mut bank = BackgroundBank::new();
    bank.add_vs_screens(&assets).unwrap();
    let stock_view = bank.view(&vs_key("Ryu", "Sagat")).unwrap();
    evil_ryu_vs::install(&assets, &mut bank, profile).unwrap();
    let spike_view = bank.view(&vs_key(profile.display_name(), "Sagat")).unwrap();
    let spike_palette = bank.palette(spike_view.palette).unwrap().to_vec();
    let (satsui_body, satsui_fire) = evil_ryu_art::satsui_art(&assets);
    evil_ryu_art::apply(&mut assets, profile);
    let mut config = MatchConfig::definitive();
    config.evil_ryu_playtest = Some(profile);
    let mut session = GameSession::new(assets, config)
        .with_satsui_art(satsui_body, satsui_fire)
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
        .with_background_bank(bank);
    session.start_evil_ryu_playtest();

    let mut saw_vs = false;
    let frame = (0..1_800).find_map(|_| {
        let output = session.step(FrameInputs::default());
        let machine = session.fight_machine().expect("the fight stays active through its intro");
        assert!(machine.unrecovered().is_empty(), "untranslated tasks: {:x?}", machine.unrecovered());
        let state = &machine.state;
        if !saw_vs && state.gfxctrl & 0x60 == 0x20 && state.bg_page == spike_view.page
            && state.bg_scroll == spike_view.scroll && state.paletteram[..0x200] == spike_palette
        {
            assert_evil_ryu_vs_frame(&session, &output.frame, stock_view);
            if profile == EvilRyuPlaytest::UltraSf2 {
                capture_frame("SF_EVIL_RYU_VS_CAPTURE", &output.frame);
            } else if let Some(dir) = alpha_clip_dir(profile, "vs") {
                write_clip(&dir, std::slice::from_ref(&output.frame));
            }
            saw_vs = true;
        }
        (flow_position(machine) == FIGHT_LOOP).then_some(output.frame)
    }).expect("the launcher reaches player control");
    assert!(saw_vs, "the fight showed the private {} versus Sagat intro", profile.display_name());
    (session, frame)
}

fn super_meter(fight: &Fight) -> u8 {
    fight.machine().era_fighters()[0].super_meter
}

fn assert_super_gauge(fight: &Fight, frame: &Frame, meter: u8) {
    assert_eq!(super_meter(fight), meter, "Evil Ryu's native Super meter");
    let state = fight.machine().era_fighters()[0];
    let cells = match (state.style.family, state.style.loadout) {
        (EraFamily::Sf4, _) => 4,
        (EraFamily::Sf5 | EraFamily::Sf6, _) => 3,
        (EraFamily::SfZ, Loadout::VIsm) => 2,
        (EraFamily::SfZ, Loadout::XIsm) => 1,
        (EraFamily::SfZ, _) => 3,
        _ => 1,
    };
    let whole = meter / SUPER_STOCK;
    let fraction = Fill::of(u16::from(meter % SUPER_STOCK), u16::from(SUPER_STOCK));
    let mut gauge = GaugeRow::stock(cells, whole, fraction);
    if state.style.loadout == Loadout::VIsm && state.custom_combo_active() {
        gauge = gauge.with_state(GaugeState::Draining);
    }
    let hud = fight.era_hud().p1.unwrap();
    let sf6 = state.style.family == EraFamily::Sf6;
    let super_row = if sf6 { hud.second.expect("SF6 has a separate Super Art row") } else { hud.main };
    assert_eq!(super_row, gauge);
    assert_eq!(hud.second.is_some(), state.guard_power().is_some()
        || matches!(state.style.family, EraFamily::Sf4 | EraFamily::Sf5 | EraFamily::Sf6));
    let rows = if sf6 { SECOND_ROW } else if hud.second.is_some() { CORNER_HEIGHT } else { MAIN_ROW };
    let top = HEIGHT - CORNER_BOTTOM - rows;
    let fill_start = (top + 1) * WIDTH + CORNER_EDGE + 1;
    let (rail, track) = match (state.style.family, state.style.revision, state.style.loadout) {
        (EraFamily::Sf2, _, _) => (0xffff_ffff, 0xff77_98db),
        (EraFamily::SfZ, Revision::Sfz2, _) => (0xff33_4466, 0xff11_7733),
        (EraFamily::SfZ, _, Loadout::AIsm) => (0xff00_0044, 0xff04_6201),
        (EraFamily::SfZ, _, Loadout::VIsm) => (0xff00_0044, 0xff11_7799),
        (EraFamily::SfZ, _, Loadout::XIsm) => (0xff00_0044, 0xff6e_1a02),
        (EraFamily::Sf4, _, _) => (0xffd5_d5d5, 0xff33_2f30),
        (EraFamily::Sf5, _, _) => (0xffa9_d4de, 0xff18_3441),
        (EraFamily::Sf6, _, _) => (0xffe5_b2cc, 0xff4b_2037),
        style => panic!("unexpected Super gauge palette for {style:?}"),
    };
    assert_eq!(frame.argb[top * WIDTH + CORNER_EDGE], rail, "the era's gauge border is visible");

    // Spent space may still show the amber tail. Classify it with the empty
    // track so a test counts usable fill, even on the frame a move spends it.
    let amber = 0xffff_b960u32;
    let trail = [0, 8, 16].into_iter().fold(0xff00_0000, |argb, shift| {
        let base = (track >> shift) & 0xff;
        let tip = (amber >> shift) & 0xff;
        argb | ((base * 133 + tip * 122 + 127) / 255) << shift
    });
    let cells = usize::from(cells);
    let cell_width = (BAR_WIDTH + 1 - cells) / cells;
    for cell in 0..cells {
        let start = fill_start + cell * (cell_width + 1);
        let pixels = &frame.argb[start..start + cell_width];
        let expected = if cell < usize::from(whole) { cell_width } else if cell == usize::from(whole) { fraction.px(cell_width) } else { 0 };
        for (offset, &pixel) in pixels.iter().enumerate() {
            let filled = ![track, trail, amber].contains(&pixel);
            assert_eq!(filled, offset < expected, "visible fill in Super stock {}, column {offset}", cell + 1);
        }
        assert_eq!(frame.argb[start + cell_width], rail, "the Super stock divider");
    }
}

fn demon_rush_frame(session: &mut GameSession) -> Option<Frame> {
    raging_demon_inputs(session.fight()?).into_iter()
        .chain(std::iter::repeat_n(PlayerInputs::default(), 12))
        .find_map(|p1| {
            let output = session.step(FrameInputs { p1, ..FrameInputs::default() });
            let fighter = session.fight()?.fight_read().fighters[0];
            (fighter.move_id == EVIL_RYU_RAGING_DEMON).then_some(output.frame)
        })
}

fn try_meter_uppercut(session: &mut GameSession, before: u8, sagat_floor: u16) -> Option<Frame> {
    let uppercut = REGISTRY.iter().find(|entry| entry.name == "SHORYUKEN").unwrap();
    let facing_right = fighter_object(session.fight()?, P1_STRUCT)[0x37] & 4 == 0;
    let inputs = uppercut.input.frames(facing_right, Strength::Light).into_iter()
        .chain(std::iter::repeat_n(PlayerInputs::default(), 80));
    let mut started = false;
    for p1 in inputs {
        let output = session.step(FrameInputs { p1, ..FrameInputs::default() });
        let fight = session.fight()?;
        let [ryu, sagat] = fight.fight_read().fighters;
        if ryu.health == 0 || sagat.health <= sagat_floor {
            return None;
        }
        if !started && ryu.move_id == SHORYUKEN_MOVE {
            assert_super_gauge(fight, &output.frame, (before + SHORYUKEN_START_GAIN).min(SUPER_STOCK));
            started = true;
        }
        if started && ryu.state & 0x80 == 0 {
            return Some(output.frame);
        }
    }
    None
}

fn fill_ultra_gauge(session: &mut GameSession, check_partial: bool) -> Frame {
    let mut tried_partial = !check_partial;
    let sagat_floor = if check_partial { 20 } else { 0 };
    for _ in 0..SUPER_STOCK.div_ceil(SHORYUKEN_START_GAIN) {
        let before = super_meter(session.fight().unwrap());
        let mut spacing = session.clone();
        let mut gained = None;
        for step in 0..80 {
            if step % 3 == 0 && spacing.fight().unwrap().fight_read().fighters[0].state & 0x80 == 0 {
                let mut candidate = spacing.clone();
                if let Some(frame) = try_meter_uppercut(&mut candidate, before, sagat_floor) {
                    gained = Some((candidate, frame));
                    break;
                }
            }
            let fight = spacing.fight().unwrap();
            let p1 = center_x(fight, P1_STRUCT);
            let sagat = center_x(fight, CPU_STRUCT);
            let mut p1_input = PlayerInputs::default();
            p1_input.directions[usize::from(p1 < sagat)] = true;
            spacing.step(FrameInputs { p1: p1_input, ..FrameInputs::default() });
        }
        let (candidate, frame) = gained.unwrap_or_else(|| panic!("earn more Super safely at {before}/{SUPER_STOCK}, life {:?}",
            session.fight().unwrap().fight_read().fighters.map(|fighter| fighter.health)));
        *session = candidate;
        let fight = session.fight().unwrap();
        let meter = super_meter(fight);
        assert_super_gauge(fight, &frame, meter);
        if !tried_partial && meter < SUPER_STOCK {
            assert!(demon_rush_frame(&mut session.clone()).is_none(),
                "an earned partial stock cannot start the Demon");
            tried_partial = true;
        }
        if meter == SUPER_STOCK {
            assert!(tried_partial, "ordinary specials build a partial gauge before full");
            return frame;
        }
    }
    panic!("real specials fill the Ultra SFII Super gauge");
}

fn whiff_ultra_demon(session: &mut GameSession) {
    let mut max_gap = 0;
    let mut attempts = 0;
    let mut rushes = 0;
    for _ in 0..180 {
        let fight = session.fight().unwrap();
        let p1 = center_x(fight, P1_STRUCT);
        let sagat = center_x(fight, CPU_STRUCT);
        let fighter = fight.fight_read().fighters[0];
        max_gap = max_gap.max(p1.abs_diff(sagat));
        if p1.abs_diff(sagat) >= 72 && fighter.state & 0x80 == 0 {
            attempts += 1;
            let mut candidate = session.clone();
            if let Some(rush) = demon_rush_frame(&mut candidate) {
                rushes += 1;
                assert_super_gauge(candidate.fight().unwrap(), &rush, 0);
                let hp = candidate.fight().unwrap().fight_read().fighters.map(|fighter| fighter.health);
                let mut completed = false;
                for _ in 0..60 {
                    candidate.step(FrameInputs::default());
                    let read = candidate.fight().unwrap().fight_read();
                    if read.fighters.map(|fighter| fighter.health) != hp || read.fighters[0].move_id == THROW_MOVE {
                        break;
                    }
                    if read.fighters[0].state & 0x80 == 0 {
                        completed = true;
                        break;
                    }
                }
                if completed {
                    assert_eq!(super_meter(candidate.fight().unwrap()), 0, "the whiff keeps the entire stock spent");
                    *session = candidate;
                    return;
                }
            }
        }
        let mut p1_input = PlayerInputs::default();
        p1_input.directions[usize::from(p1 < sagat)] = true;
        p1_input.directions[3] = true;
        session.step(FrameInputs { p1: p1_input, ..FrameInputs::default() });
    }
    panic!("a full gauge permits a Raging Demon rush that whiffs against Sagat (gap {max_gap}, attempts {attempts}, rushes {rushes})");
}

fn check_demon_meter(session: &mut GameSession, frame: &Frame) {
    assert_super_gauge(session.fight().unwrap(), frame, 0);
    assert!(demon_rush_frame(session).is_none(), "an empty gauge cannot start the Demon");
    restart_round(session);

    let full = fill_ultra_gauge(session, true);
    capture_frame("SF_EVIL_RYU_USF2_SUPER_CAPTURE", &full);
    let assets = session.fight_assets().unwrap();
    let (after_hit, hit) = land_raging_demon(session.fight().unwrap(), assets)
        .expect("a full stock can land the Demon on Sagat");
    check_raging_demon_recovery(after_hit, assets, hit, true);
    whiff_ultra_demon(session);
    assert!(demon_rush_frame(&mut session.clone()).is_none(), "a whiff spends the entire stock");

    fill_ultra_gauge(session, false);
    let mut new_round = session.clone();
    restart_round(&mut new_round);
    let frame = new_round.step(FrameInputs::default()).frame;
    assert_super_gauge(new_round.fight().unwrap(), &frame, 0);
    whiff_ultra_demon(session);
    assert!(session.fight_machine().unwrap().unrecovered().is_empty());
}

fn move_list_rows(session: &mut GameSession) -> Vec<String> {
    let mut pause = FrameInputs::default();
    pause.menu.start = true;
    assert_eq!(session.step(pause).screen, "pause");
    let moves_row = PauseBox::open(false)
        .items()
        .iter()
        .position(|&item| item == MenuItem::Moves)
        .expect("the pause menu has a Moves row");
    let mut down = FrameInputs::default();
    down.p1.directions[2] = true;
    for _ in 0..moves_row {
        session.step(FrameInputs::default());
        session.step(down);
    }
    let mut confirm = FrameInputs::default();
    confirm.menu.confirm = true;
    assert_eq!(
        session.step(confirm).transition,
        Some(Transition::Choose(MenuItem::Moves))
    );
    session.move_list().expect("Moves opens from the pause menu").text()
}

#[test]
fn ultra_sf2_evil_ryu_uses_native_styles_and_super_in_the_real_sagat_fight() {
    let (mut session, frame) = boot_spike(EvilRyuPlaytest::UltraSf2);
    capture_frame("SF_EVIL_RYU_CAPTURE", &frame);

    let machine = session.fight_machine().unwrap();
    assert_eq!(machine.match_config().preset, RulesetPreset::DefinitiveOverhaul);
    assert_eq!(machine.arcade_opponent(), Some(TournamentFighter::Sagat));
    let [evil_ryu, sagat] = machine.era_fighters();
    assert_eq!(evil_ryu.style.family, EraFamily::Sf2);
    assert_eq!(evil_ryu.style.revision, Revision::St);
    assert_eq!(evil_ryu.style.loadout, Loadout::Standard);
    assert_eq!(sagat.style, session.fight().unwrap().fighter_style(1).unwrap());
    assert_eq!(sagat.fighter, Some(TournamentFighter::Sagat));
    let cpu_table = (CPU_STRUCT + 0x12 - WORK_RAM) as usize;
    let table = u32::from_be_bytes(machine.work_ram()[cpu_table..cpu_table + 4].try_into().unwrap());
    assert_eq!(Some(table), machine.data_bank().and_then(|bank| bank.table(TournamentFighter::Sagat)));
    assert_eq!((evil_ryu.sub_hp, evil_ryu.max_sub_hp), (EVIL_RYU_MAX_HP, EVIL_RYU_MAX_HP));
    assert_eq!((sagat.sub_hp, sagat.max_sub_hp), (SAGAT_MAX_HP, SAGAT_MAX_HP));
    assert_eq!(life(machine.work_ram(), P1_STRUCT), 48);
    assert_eq!(life(machine.work_ram(), CPU_STRUCT), 48);
    assert_evil_ryu_label(&session, &frame);
    assert!(frame.argb.chunks_exact(WIDTH).skip(60).take(100).any(|row| {
        row[..WIDTH / 2].contains(&0xffff4433)
    }), "the fight frame shows Evil Ryu's bright red eye pen");

    let timer = (TIME_DIGITS - WORK_RAM) as usize;
    assert_eq!(&machine.work_ram()[timer..timer + 3], [1, 0, 0]);
    check_demon_meter(&mut session.clone(), &frame);
    let hadoken = REGISTRY.iter().find(|entry| entry.name == "HADOKEN").unwrap();
    let shaku = hadoken.input.frames(true, Strength::Heavy).into_iter()
        .chain(std::iter::repeat_n(Default::default(), 45))
        .find_map(|p1| {
            let output = session.step(FrameInputs { p1, ..FrameInputs::default() });
            let ember_pixels = output.frame.argb.chunks_exact(WIDTH).skip(80).take(100)
                .flat_map(|row| &row[145..230])
                .filter(|&&pixel| pixel == 0xffff5533)
                .count();
            (ember_pixels >= 30).then_some(output.frame)
        }).expect("heavy Hadoken renders a red projectile on the way to Sagat");
    capture_frame("SF_EVIL_RYU_SHAKUNETSU_CAPTURE", &shaku);
    for _ in 0..120 {
        session.step(FrameInputs::default());
    }
    let machine = session.fight_machine().unwrap();
    assert_eq!(flow_position(machine), FIGHT_LOOP);
    assert!(machine.work_ram()[timer] < 1, "the round clock is counting down");
    assert!(machine.unrecovered().is_empty());

    restart_round(&mut session);
    let forward = ashura_frame(&mut session, &[Dir::Forward, Dir::Down, Dir::DownForward], EVIL_RYU_ASHURA_FORWARD, ButtonClass::Kick);
    capture_frame("SF_EVIL_RYU_ASHURA_CAPTURE", &forward);
    restart_round(&mut session);
    ashura_frame(&mut session, &[Dir::Back, Dir::Down, Dir::DownBack], EVIL_RYU_ASHURA_BACK, ButtonClass::Kick);
    let rows = move_list_rows(&mut session);
    assert!(rows.iter().any(|row| row.starts_with("HADOKEN ") && row.ends_with("[LP/MP]")));
    assert!(rows.iter().any(|row| row.starts_with("SHAKUNETSU HADOKEN ") && row.ends_with("[HP]")));
    assert!(rows.iter().any(|row| row.starts_with("METSU SHORYUKEN ")));
    assert!(rows.iter().any(|row| row.starts_with("ASHURA SENKU FORWARD ") && row.ends_with("[K]")));
    assert!(rows.iter().any(|row| row.starts_with("ASHURA SENKU BACKWARD ") && row.ends_with("[K]")));
    assert!(rows.iter().any(|row| row.starts_with("RAGING DEMON [LP] [LP] ") && row.ends_with("[LK] [HP]")));
}

fn assert_alpha_move(mut fight: Fight, assets: &Assets, input: MoveInput, slot: u8, meter: u8, name: &str) {
    let facing_right = fighter_object(&fight, P1_STRUCT)[0x37] & 4 == 0;
    let mut clip = AlphaClip::new(&fight, &name.to_ascii_lowercase().replace(' ', "-"), true);
    for p1 in input.frames(facing_right, Strength::Light) {
        let frame = fight.step(assets, p1, PlayerInputs::default(), false).frame;
        clip.push(&frame);
        if fight.fight_read().fighters[0].move_id == slot {
            assert_super_gauge(&fight, &frame, meter);
            clip.idle_tail(&mut fight, assets, 65);
            clip.finish();
            return;
        }
    }
    panic!("{name} starts from its native Alpha command");
}

fn check_alpha_demon(fight: &Fight, assets: &Assets, full_meter: u8) {
    // The stock boundary uses the same rollback fixture as the native Alpha
    // Super tests. Every command still goes through the live Sagat fight.
    let mut short = fight_with_meter(fight, full_meter - 1);
    assert_super_gauge(&short, &short.render(assets), full_meter - 1);
    for p1 in raging_demon_inputs(&short) {
        short.step(assets, p1, PlayerInputs::default(), false);
        assert_ne!(short.fight_read().fighters[0].move_id, EVIL_RYU_RAGING_DEMON);
    }
    assert_eq!(super_meter(&short), full_meter - 1, "one point short keeps all earned stock");

    let full = fight_with_meter(fight, full_meter);
    let frame = full.render(assets);
    assert_super_gauge(&full, &frame, full_meter);
    let alpha2 = fight.setup().config.evil_ryu_playtest == Some(EvilRyuPlaytest::Alpha2);
    if alpha2 {
        capture_frame("SF_EVIL_RYU_ALPHA2_SUPER_CAPTURE", &frame);
    }
    let mut restarted = full.clone();
    assert!(restarted.restart_round(), "the round-start snapshot can be restored");
    let frame = restarted.step(assets, PlayerInputs::default(), PlayerInputs::default(), false).frame;
    assert_super_gauge(&restarted, &frame, 0);

    let (after, hit) = land_raging_demon(&full, assets).expect("full meter lands the Demon on Alpha Sagat");
    if alpha2 {
        capture_frame("SF_EVIL_RYU_ALPHA2_DEMON_REVEAL_CAPTURE", &hit.reveal);
    }
    check_raging_demon_recovery(after, assets, hit, false);

    let mut spacing = full;
    for _ in 0..120 {
        let p1_x = center_x(&spacing, P1_STRUCT);
        let sagat_x = center_x(&spacing, CPU_STRUCT);
        if p1_x.abs_diff(sagat_x) >= 112 && spacing.fight_read().fighters[0].state & 0x80 == 0 {
            let mut candidate = spacing.clone();
            if let Some(DemonOutcome::Whiff(mut frames)) = try_raging_demon(&mut candidate, assets) {
                assert_eq!(super_meter(&candidate), 0, "the Alpha Demon whiff spends the full meter");
                if let Some(dir) = fight.setup().config.evil_ryu_playtest
                    .filter(|profile| matches!(profile, EvilRyuPlaytest::Alpha3X | EvilRyuPlaytest::Ae2012 | EvilRyuPlaytest::Usf4))
                    .and_then(|profile| alpha_clip_dir(profile, "demon-whiff"))
                {
                    let idle = PlayerInputs::default();
                    for _ in 0..25 {
                        frames.push(candidate.step(assets, idle, idle, false).frame);
                    }
                    write_clip(&dir, &frames);
                }
                return;
            }
        }
        let mut retreat = PlayerInputs::default();
        retreat.directions[usize::from(p1_x < sagat_x)] = true;
        retreat.directions[3] = true;
        spacing.step(assets, retreat, PlayerInputs::default(), false);
    }
    panic!("a full-meter Demon can whiff against Alpha Sagat");
}

fn alpha_pad(fight: &Fight, forward: bool, buttons: &[usize]) -> PlayerInputs {
    let mut input = PlayerInputs::default();
    let facing_left = fighter_object(fight, P1_STRUCT)[0x37] & 4 != 0;
    input.directions[usize::from(facing_left == forward)] = true;
    for &button in buttons {
        input.buttons[button] = true;
    }
    input
}

fn check_alpha_custom_combo(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let ppk = &[0, 1, 3][..];
    let kkp = &[0, 3, 4][..];
    for (meter, buttons, active) in [
        (SUPER_STOCK - 1, ppk, false),
        (SUPER_STOCK, &[0, 3][..], false),
        (SUPER_STOCK, &[0, 1, 2][..], false),
        (SUPER_STOCK, ppk, true),
        (SUPER_STOCK, kkp, true),
        (SUPER_STOCK + 11, kkp, true),
        (ALPHA_FULL_SUPER, ppk, true),
    ] {
        let mut attempt = fight_with_meter(fight, meter);
        let mut chord = PlayerInputs::default();
        for &button in buttons {
            chord.buttons[button] = true;
        }
        let frame = attempt.step(assets, chord, idle, false);
        assert_eq!(attempt.machine().era_fighters()[0].custom_combo_active(), active, "{meter} meter, {buttons:?}");
        assert_eq!(frame.announcements.contains(&AnnouncerLine::CustomCombo), active);
        assert_eq!(super_meter(&attempt), if active { 0 } else { meter });
        if active {
            let timer = GaugeRow::bar(Fill::of(u16::from(meter) * 2, 288), Fill::FULL)
                .with_state(GaugeState::Draining);
            let stock = GaugeRow::stock(3, 0, Fill::of(0, u16::from(SUPER_STOCK)));
            assert_eq!(attempt.era_hud().p1, Some(EraGauge::two(timer, stock)));
        }
        if meter == ALPHA_FULL_SUPER {
            let mut clip = AlphaClip::new(&attempt, "custom-combo", true);
            clip.push(&frame.frame);
            if clip.dir.is_some() {
                let mut action = attempt.clone();
                for tick in 0..90 {
                    let buttons: &[usize] = match tick {
                        23 => &[2],
                        29 => &[3],
                        38 => &[4],
                        54 => &[5],
                        _ => &[],
                    };
                    let p1 = alpha_pad(&action, true, buttons);
                    clip.push(&action.step(assets, p1, idle, false).frame);
                }
            }
            clip.finish();
            let mut elapsed = 0;
            for _ in 0..50 {
                let frozen = attempt.machine().era_super_freeze() != 0;
                let mut guard = alpha_pad(&attempt, false, &[]);
                guard.directions[2] = true;
                attempt.step(assets, guard, idle, false);
                elapsed += usize::from(!frozen);
                if elapsed > 32 {
                    break;
                }
            }
            assert!(elapsed > 32, "Custom Combo leaves its activation freeze");
            assert!(attempt.machine().era_fighters()[0].custom_combo_active(),
                "three-stock Custom Combo remains active beyond a wrapped 32-frame timer");
            let gauge = attempt.era_hud().p1.unwrap();
            assert_eq!(gauge.main, GaugeRow::bar(Fill::of(288 - 33, 288), Fill::FULL)
                .with_state(GaugeState::Draining));
        }
    }
}

fn p1_projectile_palette(fight: &Fight, move_id: u8) -> Option<u8> {
    let machine = fight.machine();
    machine.toggle_state().era.projectiles().iter()
        .zip(machine.state.objectram[0x100..].chunks_exact(0x40))
        .find_map(|(projectile, object)| {
            (projectile.move_id == move_id && object[8] != 0
                && u32::from_be_bytes(object[0x20..0x24].try_into().unwrap()) == P1_STRUCT)
                .then_some(object[3] & 0x0f)
        })
}

fn check_alpha_feint_and_senpukyaku(session: &GameSession, assets: &Assets) {
    let mut plain_start = session.clone();
    let mut start = FrameInputs::default();
    start.menu.start = true;
    assert_eq!(plain_start.step(start).screen, "pause", "ordinary Start still pauses");

    let fight = session.fight().unwrap();
    let facing_right = fighter_object(fight, P1_STRUCT)[0x37] & 4 == 0;
    let motion = &[Dir::Down, Dir::DownForward, Dir::Forward];
    let mut feint = session.clone();
    let mut fake_clip = AlphaClip::new(fight, "fake-hadoken", true);
    let inputs = (MoveInput { motion, button: ButtonClass::DirectionOnly }).frames(facing_right, Strength::Light);
    let (final_pad, prefix) = inputs.split_last().unwrap();
    for &p1 in prefix {
        fake_clip.push(&feint.step(FrameInputs { p1, ..FrameInputs::default() }).frame);
    }
    start.p1 = *final_pad;
    let output = feint.step(start);
    assert_ne!(output.screen, "pause", "QCF + Start stays in the fight");
    fake_clip.push(&output.frame);
    assert_eq!(feint.fight().unwrap().fight_read().fighters[0].move_id, FAKE_HADOKEN_MOVE);
    for _ in 0..45 {
        let fight = feint.fight().unwrap();
        assert!(p1_projectile_palette(fight, HADOKEN_MOVE).is_none(), "the feint releases no projectile");
        assert_eq!(super_meter(fight), 0, "the feint earns no Super meter");
        fake_clip.push(&feint.step(FrameInputs::default()).frame);
    }
    fake_clip.finish();

    let mut real = fight.clone();
    let mut real_clip = AlphaClip::new(fight, "hadoken", true);
    let inputs = MoveInput { motion, button: ButtonClass::Punch }.frames(facing_right, Strength::Light);
    assert!(inputs.into_iter().chain(std::iter::repeat_n(PlayerInputs::default(), 45)).any(|p1| {
        real_clip.push(&real.step(assets, p1, PlayerInputs::default(), false).frame);
        p1_projectile_palette(&real, HADOKEN_MOVE).is_some()
    }), "ordinary QCF + punch releases a Hadoken");
    assert_eq!(super_meter(&real), 2);
    real_clip.idle_tail(&mut real, assets, 45);
    real_clip.finish();

    let mut kick = fight.clone();
    let mut kick_clip = AlphaClip::new(fight, "senpukyaku", true);
    let p1 = alpha_pad(&kick, true, &[4]);
    kick_clip.push(&kick.step(assets, p1, PlayerInputs::default(), false).frame);
    assert_eq!(kick.fight_read().fighters[0].move_id, SENPUKYAKU_MOVE, "forward + MK starts Senpukyaku");
    kick_clip.idle_tail(&mut kick, assets, 45);
    kick_clip.finish();
}

fn check_alpha_throw(fight: &Fight, assets: &Assets, forward: bool, buttons: &[usize]) {
    let mut approach = fight.clone();
    let idle = PlayerInputs::default();
    let roll = buttons[0] >= 3;
    for _ in 0..96 {
        let p1 = center_x(&approach, P1_STRUCT);
        let sagat = center_x(&approach, CPU_STRUCT);
        if p1.abs_diff(sagat) <= THROW_RANGE {
            let mut attempt = approach.clone();
            let pad = alpha_pad(&attempt, forward, buttons);
            let start = attempt.step(assets, pad, idle, false);
            let [ryu, sagat_at_grab] = attempt.fight_read().fighters;
            if ryu.move_id == THROW_MOVE && sagat_at_grab.move_id == THROWN_MOVE {
                let bank = attempt.machine().data_bank().unwrap();
                let header = if roll {
                    bank.era_move_header(TournamentFighter::Ryu, EraFamily::SfZ, THROW_MOVE).unwrap()
                } else {
                    bank.throw_entry(TournamentFighter::Ryu).unwrap().throw_header
                };
                let stream = u32::from_be_bytes(std::array::from_fn(|i| bank.rb(header + 0x2c + i as u32)));
                let object = fighter_object(&attempt, P1_STRUCT);
                assert_eq!(u32::from_be_bytes(object[0x2c..0x30].try_into().unwrap()), stream,
                    "the requested throw selects the matching banked stream");
                let name = if roll { "tomoe-nage" } else { "shoulder-throw" };
                let mut clip = AlphaClip::new(&attempt, name, forward && (buttons == [4] || buttons.len() == 2));
                clip.push(&start.frame);
                let mut rolled = false;
                for _ in 0..70 {
                    clip.push(&attempt.step(assets, idle, idle, false).frame);
                    let [ryu, victim] = attempt.fight_read().fighters;
                    let object = fighter_object(&attempt, P1_STRUCT);
                    let attr = u16::from_be_bytes([object[2], object[3]]);
                    let mut pose = u16::from_be_bytes([object[0], object[1]]);
                    if attr & 0x100 != 0 {
                        let span = u16::from(object[0xd]) * if attr & 0x400 != 0 { 2 } else { 1 };
                        pose = pose.wrapping_sub(span);
                    }
                    rolled |= ryu.move_id == THROW_MOVE && pose == 0x0484;
                    if victim.health < sagat_at_grab.health {
                        assert_eq!(sagat_at_grab.health - victim.health, u16::from(THROW_DAMAGE));
                        assert_eq!(rolled, roll, "the kick throw uses Tomoe Nage's distinct roll (forward={forward}, buttons={buttons:?})");
                        if rolled {
                            assert_eq!(
                                center_x(&attempt, P1_STRUCT).cmp(&center_x(&attempt, CPU_STRUCT)),
                                sagat.cmp(&p1),
                                "Tomoe Nage switches sides by the slam",
                            );
                        }
                        clip.idle_tail(&mut attempt, assets, 54);
                        clip.finish();
                        return;
                    }
                    if victim.move_id != THROWN_MOVE {
                        break;
                    }
                }
            }
        }
        let pad = alpha_pad(&approach, true, &[]);
        approach.step(assets, pad, idle, false);
    }
    panic!("a live Sagat opening allows the {} throw (forward={forward}, buttons={buttons:?})",
        if roll { "Tomoe Nage" } else { "punch" });
}

fn air_throw_height(fight: &Fight, player: u32) -> (i32, i32) {
    let object = fighter_object(fight, player);
    let attr = u16::from_be_bytes([object[2], object[3]]);
    let tile = if attr & 0x400 != 0 { 32 } else { 16 };
    let height = (i32::from(object[0xc]) + 1) * tile;
    let top = i32::from(i16::from_be_bytes([object[4], object[5]]));
    let center = top + height / 2;
    (center, GROUND - center - i32::from(object[0x17]))
}

fn fight_with_scripted_sagat(fight: &Fight) -> Fight {
    // Keep the Arcade Sagat and the ROM-bank moves. Give this clone both pads
    // to reproduce the same opening and the same inputs in the capture.
    let mut scripted = fight.clone();
    let mut snapshot = scripted.save_netplay_snapshot(0);
    snapshot.setup.config.p2_fair_cpu = false;
    snapshot.machine.match_config.p2_fair_cpu = false;
    scripted.restore_netplay_snapshot(&snapshot);
    scripted
}

fn check_air_shoulder_throw(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let scripted = fight_with_scripted_sagat(fight);
    let sagat_forward = |fight: &Fight| {
        let mut pad = idle;
        pad.directions[usize::from(center_x(fight, CPU_STRUCT) > center_x(fight, P1_STRUCT))] = true;
        pad
    };
    let mut sagat_air_frames = 0;
    let mut paired_air_frames = 0;
    let mut press_frames = 0;
    let mut closest_x = u16::MAX;
    for walk in [16, 8, 0] {
        let mut spacing = scripted.clone();
        for _ in 0..walk {
            let p1 = alpha_pad(&spacing, true, &[]);
            let p2 = sagat_forward(&spacing);
            spacing.step(assets, p1, p2, false);
        }
        for (ryu_delay, sagat_delay) in [(0, 0), (4, 0), (0, 4), (8, 0), (0, 8)] {
            for jump_forward in [true, false] {
                let mut jump = spacing.clone();
                let mut ryu_jump = if jump_forward { alpha_pad(&jump, true, &[]) } else { idle };
                ryu_jump.directions[3] = true;
                let mut sagat_jump = sagat_forward(&jump);
                sagat_jump.directions[3] = true;
                let jump_pads = |tick| {
                    let p1 = if (ryu_delay..ryu_delay + 3).contains(&tick) { ryu_jump } else { idle };
                    let p2 = if (sagat_delay..sagat_delay + 3).contains(&tick) { sagat_jump } else { idle };
                    (p1, p2)
                };
                for tick in 0..57 {
                    let (p1, p2) = jump_pads(tick);
                    jump.step(assets, p1, p2, false);
                    let [ryu, sagat] = jump.fight_read().fighters;
                    let (ryu_y, ryu_clearance) = air_throw_height(&jump, P1_STRUCT);
                    let (sagat_y, sagat_clearance) = air_throw_height(&jump, CPU_STRUCT);
                    let x_gap = center_x(&jump, P1_STRUCT).abs_diff(center_x(&jump, CPU_STRUCT));
                    let sagat_airborne = matches!(sagat.move_id, 9..=0xb | 0x20..=0x26) && sagat_clearance >= 8;
                    sagat_air_frames += usize::from(sagat_airborne);
                    // Catch below the HUD so the showcase can show both poses.
                    if !sagat_airborne || !matches!(ryu.move_id, 9..=0xb) || !(40..=72).contains(&ryu_clearance) {
                        continue;
                    }
                    paired_air_frames += 1;
                    closest_x = closest_x.min(x_gap);
                    if x_gap > THROW_RANGE || ryu_y.abs_diff(sagat_y) > 40 {
                        continue;
                    }

                    press_frames += 1;
                    let mut attempt = jump.clone();
                    let press = alpha_pad(&attempt, true, &[0, 1]);
                    let grab = attempt.step(assets, press, idle, false);
                    let [thrower, victim] = attempt.fight_read().fighters;
                    if thrower.move_id != THROW_MOVE || victim.move_id != THROWN_MOVE {
                        continue;
                    }
                    let mut backward = jump.clone();
                    let back_pp = alpha_pad(&backward, false, &[0, 1]);
                    backward.step(assets, back_pp, idle, false);
                    assert_eq!(
                        backward.fight_read().fighters.map(|fighter| fighter.move_id),
                        [THROW_MOVE, THROWN_MOVE],
                        "backward PP also catches the same airborne opening",
                    );
                    let mut no_direction = jump.clone();
                    let mut neutral_pp = idle;
                    neutral_pp.buttons[..2].fill(true);
                    no_direction.step(assets, neutral_pp, idle, false);
                    assert_ne!(no_direction.fight_read().fighters[0].move_id, THROW_MOVE,
                        "neutral PP misses the same airborne opening");
                    let mut kick_pair = jump.clone();
                    let kk = alpha_pad(&kick_pair, true, &[3, 4]);
                    kick_pair.step(assets, kk, idle, false);
                    assert_ne!(kick_pair.fight_read().fighters[0].move_id, THROW_MOVE,
                        "KK does not perform an airborne shoulder throw");

                    assert_eq!(victim.health, sagat.health, "damage waits for the slam");
                    let (_, grab_clearance) = air_throw_height(&attempt, P1_STRUCT);
                    assert!(grab_clearance >= 8 && ryu_clearance.abs_diff(grab_clearance) <= 12,
                        "the grab stays airborne (clearance before {ryu_clearance}, after {grab_clearance})");
                    assert!(air_throw_height(&attempt, CPU_STRUCT).1 >= 8, "the victim stays in the air at the grab");

                    let mut clip = AlphaClip::new(fight, "air-shoulder-throw", true);
                    if clip.dir.is_some() {
                        let mut opening = spacing.clone();
                        for frame in 0..=tick {
                            let (p1, p2) = jump_pads(frame);
                            clip.push(&opening.step(assets, p1, p2, false).frame);
                        }
                    }
                    clip.push(&grab.frame);
                    let mut damaged = false;
                    let mut got_up = false;
                    let mut thrower_landed = false;
                    for frame in 0..130 {
                        clip.push(&attempt.step(assets, idle, idle, false).frame);
                        let [thrower, victim] = attempt.fight_read().fighters;
                        if frame == 0 {
                            assert!(air_throw_height(&attempt, P1_STRUCT).1 >= 8,
                                "the grab stays elevated on the following gameplay tick");
                        }
                        if victim.health < sagat.health && !damaged {
                            assert_eq!(sagat.health - victim.health, u16::from(THROW_DAMAGE));
                            assert_eq!(victim.move_id, THROWN_MOVE, "Sagat is down after the air throw slam");
                            assert!(air_throw_height(&attempt, CPU_STRUCT).1 <= 0,
                                "the slam deals damage at floor contact");
                            damaged = true;
                        }
                        if thrower.move_id != THROW_MOVE && !thrower_landed {
                            assert_eq!(thrower.state & 3, 2, "Evil Ryu returns to grounded control");
                            thrower_landed = true;
                        }
                        got_up |= victim.move_id == GET_UP_MOVE;
                        if got_up && victim.move_id != GET_UP_MOVE && thrower.state & 0x80 == 0 {
                            assert!(damaged, "the air throw deals damage before Sagat gets up");
                            assert!(thrower_landed, "Evil Ryu lands by the end of Sagat's wake-up");
                            assert_eq!(sagat.health - victim.health, u16::from(THROW_DAMAGE));
                            assert!(attempt.machine().unrecovered().is_empty());
                            clip.idle_tail(&mut attempt, assets, 10);
                            clip.finish();
                            return;
                        }
                    }
                    panic!("Sagat recovers from the airborne shoulder throw");
                }
            }
        }
    }
    panic!("a scripted Sagat jump overlaps an Alpha 3 A-ISM directional PP air throw (Sagat airborne {sagat_air_frames}, both airborne {paired_air_frames}, closest X gap {closest_x}, directional PP attempts {press_frames})");
}

const ALPHA_SUPERS: [(MoveInput, u8, &str); 3] = [
    (MoveInput {
        motion: &[Dir::Down, Dir::DownForward, Dir::Forward, Dir::Down, Dir::DownForward, Dir::Forward],
        button: ButtonClass::Punch,
    }, SF2_SUPER_SLOT, "Shinkuu Hadoken"),
    (MoveInput {
        motion: &[Dir::Down, Dir::DownForward, Dir::Forward, Dir::Down, Dir::DownForward],
        button: ButtonClass::Kick,
    }, ALPHA_SUPER_SECOND_SLOT, "Messatsu Goshoryu"),
    (MoveInput {
        motion: &[Dir::Down, Dir::DownBack, Dir::Back, Dir::Down, Dir::DownBack, Dir::Back],
        button: ButtonClass::Kick,
    }, ALPHA_SUPER_THIRD_SLOT, "Shinkuu Tatsumaki"),
];

fn check_alpha_supers(fight: &Fight, assets: &Assets) {
    let full = fight_with_meter(fight, ALPHA_FULL_SUPER);
    for (input, slot, name) in ALPHA_SUPERS {
        assert_alpha_move(full.clone(), assets, input, slot, 2 * SUPER_STOCK, name);
    }
}

#[test]
fn alpha2_evil_ryu_uses_native_moves_and_three_stock_demon_against_sagat() {
    let (session, frame) = boot_spike(EvilRyuPlaytest::Alpha2);
    capture_frame("SF_EVIL_RYU_ALPHA2_FIGHT_CAPTURE", &frame);
    let fight = session.fight().unwrap();
    let assets = session.fight_assets().unwrap();
    let machine = fight.machine();
    let [evil_ryu, sagat] = machine.era_fighters();
    assert_eq!(machine.arcade_opponent(), Some(TournamentFighter::Sagat));
    assert_eq!(evil_ryu.style.family, EraFamily::SfZ);
    assert_eq!(evil_ryu.style.revision, Revision::Sfz2);
    assert_eq!(evil_ryu.style.loadout, Loadout::Standard);
    assert_eq!((evil_ryu.sub_hp, evil_ryu.max_sub_hp), (EVIL_RYU_MAX_HP, EVIL_RYU_MAX_HP));
    assert_eq!(sagat.style, fight.fighter_style(1).unwrap());
    assert_eq!(sagat.style.revision, Revision::Sfz2);
    assert_eq!(sagat.fighter, Some(TournamentFighter::Sagat));
    assert_evil_ryu_label(&session, &frame);
    assert_super_gauge(fight, &frame, 0);

    let mut uppercut = session.clone();
    let frame = try_meter_uppercut(&mut uppercut, 0, 20).expect("single punch DP still starts Shoryuken");
    assert_super_gauge(uppercut.fight().unwrap(), &frame, SHORYUKEN_START_GAIN);
    for button in [ButtonClass::TriplePunch, ButtonClass::TripleKick] {
        ashura_frame(&mut session.clone(), &[Dir::Forward, Dir::Down, Dir::DownForward], EVIL_RYU_ASHURA_FORWARD, button);
        ashura_frame(&mut session.clone(), &[Dir::Back, Dir::Down, Dir::DownBack], EVIL_RYU_ASHURA_BACK, button);
    }

    assert_alpha_move(fight.clone(), assets, MoveInput {
        motion: &[Dir::Jump, Dir::Down, Dir::DownBack, Dir::Back],
        button: ButtonClass::Kick,
    }, AIR_TATSUMAKI_MOVE, TATSUMAKI_START_GAIN, "Air Tatsumaki");
    check_alpha_supers(fight, assets);
    check_alpha_demon(fight, assets, ALPHA_FULL_SUPER);
    check_alpha_custom_combo(fight, assets);
    check_alpha_feint_and_senpukyaku(&session, assets);
    for forward in [true, false] {
        for button in [2, 4, 5] {
            check_alpha_throw(fight, assets, forward, &[button]);
        }
    }

    let rows = move_list_rows(&mut session.clone());
    for name in ["AIR TATSUMAKI ", "SHINKUU HADOKEN ", "MESSATSU GOSHORYU ", "SHINKUU TATSUMAKI "] {
        assert!(rows.iter().any(|row| row.starts_with(name)), "{name} is in the Alpha 2 Moves panel");
    }
    for name in ["ASHURA SENKU FORWARD ", "ASHURA SENKU BACKWARD "] {
        assert!(rows.iter().any(|row| row.starts_with(name) && row.ends_with("[PPP/KKK]")));
    }
    for (name, input) in [
        ("CUSTOM COMBO ", "[PPK/KKP]"),
        ("SENPUKYAKU ", "[MK]"),
        ("HADO NO KAMAE (FAKE) ", "[START]"),
        ("TOMOE NAGE CLOSE ", "[MK/HK]"),
    ] {
        assert!(rows.iter().any(|row| row.starts_with(name) && row.ends_with(input)), "{name} shows {input}");
    }
    assert!(rows.iter().any(|row| row.starts_with("RAGING DEMON (LV 3) [LP] [LP] ") && row.ends_with("[LK] [HP]")));
}

fn check_alpha3_fireballs(fight: &Fight, assets: &Assets) {
    let qcf = &[Dir::Down, Dir::DownForward, Dir::Forward][..];
    let hcf = &[Dir::Back, Dir::DownBack, Dir::Down, Dir::DownForward, Dir::Forward][..];
    for (name, motion, move_id, red) in [
        ("hadoken", qcf, HADOKEN_MOVE, false),
        ("shakunetsu", hcf, SF2_SPECIAL_SLOT, true),
    ] {
        let mut attempt = fight.clone();
        let mut clip = AlphaClip::new(fight, name, true);
        let facing_right = fighter_object(fight, P1_STRUCT)[0x37] & 4 == 0;
        let inputs = if red {
            let mut sweep: Vec<_> = motion.iter().flat_map(|direction| {
                (MoveInput { motion: std::slice::from_ref(direction), button: ButtonClass::DirectionOnly })
                    .frames(facing_right, Strength::Heavy)
            }).collect();
            sweep.last_mut().unwrap().buttons[2] = true;
            sweep
        } else {
            MoveInput { motion, button: ButtonClass::Punch }.frames(facing_right, Strength::Heavy)
        };
        let palette = inputs.into_iter().chain(std::iter::repeat_n(PlayerInputs::default(), 50)).find_map(|p1| {
            let frame = attempt.step(assets, p1, PlayerInputs::default(), false).frame;
            clip.push(&frame);
            p1_projectile_palette(&attempt, move_id)
        }).unwrap_or_else(|| panic!("{name} produces its native projectile"));
        assert_eq!(palette == 14, red, "{name} uses the correct native projectile palette");
        assert_eq!(super_meter(&attempt), 2, "{name} earns its startup meter once");
        clip.idle_tail(&mut attempt, assets, 50);
        clip.finish();
    }
}

fn alpha3_ashura_ticks(session: &GameSession, motion: &'static [Dir], move_id: u8, button: ButtonClass) -> usize {
    let mut attempt = session.clone();
    ashura_frame(&mut attempt, motion, move_id, button);
    for protected in 1..30 {
        attempt.step(FrameInputs::default());
        let object = fighter_object(attempt.fight().unwrap(), P1_STRUCT);
        if object[0xa] != move_id || object[0xf] & 8 == 0 {
            return protected;
        }
    }
    panic!("Ashura ends its protected glide");
}

fn check_alpha3_counter(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let x_ism = fight.machine().era_fighters()[0].style.loadout == Loadout::XIsm;
    let starting_meter = if x_ism { SUPER_STOCK } else { 2 * SUPER_STOCK };
    let mut not_blocking = fight_with_meter(fight, starting_meter);
    let before = not_blocking.machine().era_fighters()[0];
    let chord = alpha_pad(&not_blocking, true, &[0, 3]);
    let output = not_blocking.step(assets, chord, idle, false);
    assert!(!output.announcements.contains(&AnnouncerLine::AlphaCounter));
    let after = not_blocking.machine().era_fighters()[0];
    if before.style.loadout == Loadout::VIsm {
        assert!(after.custom_combo_active(), "without blockstun the V-ISM chord installs Variable Combo");
        assert!(output.announcements.contains(&AnnouncerLine::CustomCombo));
    } else {
        assert_ne!(not_blocking.fight_read().fighters[0].move_id, ALPHA_COUNTER_SLOT);
        assert!(!after.custom_combo_active());
        assert!(!output.announcements.contains(&AnnouncerLine::CustomCombo));
    }
    assert_eq!(after.guard_power(), before.guard_power());
    assert_eq!(super_meter(&not_blocking), starting_meter);

    // Try several real walking approaches before guarding. A close CPU
    // normal gives the recorded counter an opponent it can reach.
    let mut approach = fight_with_meter(fight, starting_meter);
    for tick in 0..100 {
        if tick % 3 == 0 {
            for crouching in [false, true] {
                let mut attempt = approach.clone();
                let mut clip = AlphaClip::new(fight, "alpha-counter", true);
                for _ in 0..32 {
                    let mut guard = alpha_pad(&attempt, false, &[]);
                    guard.directions[2] = crouching;
                    let frame = attempt.step(assets, guard, idle, false).frame;
                    clip.push(&frame);
                    if !attempt.fight_read().fighters[0].blockstun {
                        continue;
                    }
                    let guard = attempt.machine().era_fighters()[0].guard_power().unwrap();
                    let meter = super_meter(&attempt);
                    let activation = alpha_pad(&attempt, true, &[0, 3]);
                    let frame = attempt.step(assets, activation, idle, false);
                    if x_ism {
                        assert_ne!(attempt.fight_read().fighters[0].move_id, ALPHA_COUNTER_SLOT);
                        assert!(!frame.announcements.contains(&AnnouncerLine::AlphaCounter));
                        assert_eq!(super_meter(&attempt), meter);
                        assert_eq!(attempt.machine().era_fighters()[0].guard_power(), Some(guard));
                        return;
                    }
                    assert_eq!(attempt.fight_read().fighters[0].move_id, ALPHA_COUNTER_SLOT);
                    if attempt.machine().era_fighters()[0].style.loadout == Loadout::VIsm {
                        let object = fighter_object(&attempt, P1_STRUCT);
                        assert_eq!(u32::from_be_bytes(object[0x2c..0x30].try_into().unwrap()), 0x01158,
                            "V-ISM Alpha Counter uses Ryu's ROM crouching heavy kick");
                    }
                    assert!(frame.announcements.contains(&AnnouncerLine::AlphaCounter));
                    assert!(!attempt.fight_read().fighters[0].blockstun);
                    assert_eq!(super_meter(&attempt), meter - SUPER_STOCK);
                    let after = attempt.machine().era_fighters()[0].guard_power().unwrap();
                    assert_eq!(after.maximum, guard.maximum - 12);
                    assert_eq!(after.remaining, guard.remaining - 12);
                    clip.push(&frame.frame);
                    clip.idle_tail(&mut attempt, assets, 56);
                    clip.finish();
                    return;
                }
            }
        }
        let p1 = alpha_pad(&approach, true, &[]);
        approach.step(assets, p1, idle, false);
    }
    panic!("A-ISM Evil Ryu blocks a real Sagat attack and performs Alpha Counter");
}

#[test]
fn alpha3_a_evil_ryu_uses_native_guard_and_a_ism_moves_against_sagat() {
    let (session, frame) = boot_spike(EvilRyuPlaytest::Alpha3A);
    let fight = session.fight().unwrap();
    let assets = session.fight_assets().unwrap();
    let machine = fight.machine();
    assert_eq!(machine.arcade_opponent(), Some(TournamentFighter::Sagat));
    let [evil_ryu, sagat] = machine.era_fighters();
    assert_eq!((evil_ryu.sub_hp, evil_ryu.max_sub_hp), (EVIL_RYU_MAX_HP, EVIL_RYU_MAX_HP));
    assert_eq!(sagat.fighter, Some(TournamentFighter::Sagat));
    for side in [evil_ryu, sagat] {
        assert_eq!((side.style.family, side.style.revision, side.style.loadout), (EraFamily::SfZ, Revision::Sfz3, Loadout::AIsm));
        let guard = side.guard_power().expect("A-ISM has native Guard Power");
        assert_eq!((guard.remaining, guard.maximum), (60, 60));
    }
    let hud = fight.era_hud();
    assert!(hud.p2.is_some_and(|gauge| gauge.second.is_some()));
    assert_evil_ryu_label(&session, &frame);
    assert_super_gauge(fight, &frame, 0);

    check_alpha3_fireballs(fight, assets);
    for (motion, slot) in [
        (&[Dir::Forward, Dir::Down, Dir::DownForward][..], EVIL_RYU_ASHURA_FORWARD),
        (&[Dir::Back, Dir::Down, Dir::DownBack][..], EVIL_RYU_ASHURA_BACK),
    ] {
        let punches = alpha3_ashura_ticks(&session, motion, slot, ButtonClass::TriplePunch);
        let kicks = alpha3_ashura_ticks(&session, motion, slot, ButtonClass::TripleKick);
        assert_eq!((punches, kicks), (10, 6), "PPP has ten protected Ashura ticks and KKK has six");
    }
    assert_alpha_move(fight.clone(), assets, MoveInput {
        motion: &[Dir::Jump, Dir::Down, Dir::DownBack, Dir::Back],
        button: ButtonClass::Kick,
    }, AIR_TATSUMAKI_MOVE, TATSUMAKI_START_GAIN, "Air Tatsumaki");
    assert_alpha_move(fight.clone(), assets, MoveInput {
        motion: &[Dir::Forward],
        button: ButtonClass::MediumKick,
    }, SENPUKYAKU_MOVE, 0, "Senpukyaku");
    check_alpha_supers(fight, assets);
    check_alpha_demon(fight, assets, ALPHA_FULL_SUPER);
    for forward in [true, false] {
        check_alpha_throw(fight, assets, forward, &[0, 1]);
        check_alpha_throw(fight, assets, forward, &[3, 4]);
    }
    check_air_shoulder_throw(fight, assets);
    check_alpha3_counter(fight, assets);
    for buttons in [[0, 1, 3], [0, 3, 4]] {
        let mut attempt = fight_with_meter(fight, ALPHA_FULL_SUPER);
        let mut chord = PlayerInputs::default();
        for button in buttons {
            chord.buttons[button] = true;
        }
        let output = attempt.step(assets, chord, PlayerInputs::default(), false);
        assert!(!attempt.machine().era_fighters()[0].custom_combo_active());
        assert!(!output.announcements.contains(&AnnouncerLine::CustomCombo));
        assert_eq!(super_meter(&attempt), ALPHA_FULL_SUPER);
    }

    let rows = move_list_rows(&mut session.clone());
    for (name, input) in [
        ("SHAKUNETSU HADOKEN ", "[P]"),
        ("SHOULDER THROW CLOSE ", "[PP]"),
        ("AIR SHOULDER THROW CLOSE ", "[PP]"),
        ("TOMOE NAGE CLOSE ", "[KK]"),
        ("ALPHA COUNTER (BLOCK) ", "[P+K SAME LV]"),
    ] {
        assert!(rows.iter().any(|row| row.starts_with(name) && row.ends_with(input)), "{name} shows {input}");
    }
    assert!(!rows.iter().any(|row| row.contains("CUSTOM COMBO") || row.contains("HADO NO KAMAE")));
}

fn check_variable_combo(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let activation = |punch: usize, kick: usize| {
        let mut pad = idle;
        pad.buttons[punch] = true;
        pad.buttons[kick] = true;
        pad
    };
    let p1_state = |fight: &Fight| fight.machine().era_fighters()[0];
    let sagat_hp = |fight: &Fight| fight.machine().era_fighters()[1].sub_hp;
    for (meter, buttons, starts) in [
        (SUPER_STOCK - 1, activation(0, 3), false),
        (SUPER_STOCK, activation(0, 4), false),
        (SUPER_STOCK, activation(0, 3), true),
        (SUPER_STOCK, activation(1, 4), true),
        (SUPER_STOCK, activation(2, 5), true),
    ] {
        let mut attempt = fight_with_meter(fight, meter);
        let output = attempt.step(assets, buttons, idle, false);
        assert_eq!(p1_state(&attempt).custom_combo_active(), starts);
        assert_eq!(output.announcements.contains(&AnnouncerLine::CustomCombo), starts);
        assert_super_gauge(&attempt, &output.frame, meter);
    }

    let mut close = fight_with_meter(&fight_with_scripted_sagat(fight), V_ISM_FULL_METER);
    let mut clip = AlphaClip::new(fight, "variable-combo", true);
    let mut close_ticks = 0;
    for _ in 0..96 {
        if center_x(&close, P1_STRUCT).abs_diff(center_x(&close, CPU_STRUCT)) <= THROW_RANGE {
            close_ticks += 1;
        }
        if close_ticks >= 8 {
            break;
        }
        let p1 = alpha_pad(&close, true, &[]);
        clip.push(&close.step(assets, p1, idle, false).frame);
    }
    for _ in 0..8 {
        clip.push(&close.step(assets, idle, idle, false).frame);
    }

    let chord = activation(0, 3);
    let mut timer = close.clone();
    timer.step(assets, chord, idle, false);
    assert!(timer.machine().era_super_freeze() > 0, "Variable Combo has its native install freeze");
    let mut elapsed = 0;
    for _ in 0..210 {
        let frozen = timer.machine().era_super_freeze() != 0;
        timer.step(assets, idle, idle, false);
        elapsed += u8::from(!frozen);
        assert_eq!(super_meter(&timer), V_ISM_FULL_METER - elapsed / 2);
        if !p1_state(&timer).custom_combo_active() {
            break;
        }
    }
    assert_eq!(elapsed, 192, "a full V gauge runs for 192 gameplay ticks");
    assert_eq!(p1_state(&timer).guard_power(), p1_state(&close).guard_power());

    let hp = sagat_hp(&close);
    let (heavy, ordinary_damage) = [(2, false), (2, true), (5, false)].into_iter().find_map(|(button, crouch)| {
        let mut heavy = idle;
        heavy.buttons[button] = true;
        heavy.directions[2] = crouch;
        let mut ordinary = close.clone();
        ordinary.step(assets, heavy, idle, false);
        for _ in 0..22 {
            ordinary.step(assets, idle, idle, false);
        }
        let damage = hp - sagat_hp(&ordinary);
        (damage > 0).then_some((heavy, damage))
    }).expect("a comparison heavy normal reaches Sagat");

    let mut combo = close;
    clip.push(&combo.step(assets, chord, idle, false).frame);
    for _ in 0..17 {
        clip.push(&combo.step(assets, idle, idle, false).frame);
    }
    assert!(p1_state(&combo).custom_combo_active());
    assert!(super_meter(&combo) < V_ISM_FULL_METER);
    clip.push(&combo.step(assets, heavy, idle, false).frame);
    let heavy_move = combo.fight_read().fighters[0].move_id;
    for _ in 0..20 {
        if sagat_hp(&combo) < hp {
            break;
        }
        clip.push(&combo.step(assets, idle, idle, false).frame);
    }
    assert_eq!(hp - sagat_hp(&combo), ordinary_damage / 2, "Variable Combo halves the same normal's damage");
    assert!(combo.fight_read().fighters[0].state & 0x80 != 0);
    let mut light_kick = idle;
    light_kick.buttons[3] = true;
    clip.push(&combo.step(assets, light_kick, idle, false).frame);
    assert_ne!(combo.fight_read().fighters[0].move_id, heavy_move, "LK cancels the heavy normal's recovery");

    let mut saw_blue_trail = false;
    for tick in 0..70 {
        let mut p1 = if (16..42).contains(&tick) { alpha_pad(&combo, true, &[]) } else { idle };
        match tick {
            30 => p1.buttons[1] = true,
            36 => p1.buttons[4] = true,
            _ => {}
        }
        let output = combo.step(assets, p1, idle, false);
        clip.push(&output.frame);
        if !saw_blue_trail {
            let mut snapshot = combo.save_netplay_snapshot(0);
            if snapshot.trails[0] != Default::default() {
                snapshot.trails[0].clear();
                let mut without_trail = combo.clone();
                without_trail.restore_netplay_snapshot(&snapshot);
                let clean = without_trail.render(assets);
                saw_blue_trail = combo.render(assets).argb.iter().zip(&clean.argb)
                    .any(|(&with, &without)| (with & 0xff) > (without & 0xff));
            }
        }
    }
    assert!(saw_blue_trail, "moving during Variable Combo visibly draws the blue trail");
    assert!(super_meter(&combo) < 60, "the Variable Combo gauge keeps draining through the sequence");
    clip.finish();
}

fn alpha_fight_without_a_supers(fight: &Fight, assets: &Assets, full_meter: u8) -> Fight {
    let full = fight_with_meter(fight, full_meter);
    let facing_right = fighter_object(fight, P1_STRUCT)[0x37] & 4 == 0;
    for (input, slot, name) in ALPHA_SUPERS {
        let mut attempt = full.clone();
        for p1 in input.frames(facing_right, Strength::Light) {
            attempt.step(assets, p1, PlayerInputs::default(), false);
            assert_ne!(attempt.fight_read().fighters[0].move_id, slot, "this ISM excludes {name}");
        }
        assert_eq!(super_meter(&attempt), full_meter);
    }
    full
}

fn check_v_ism_super_exclusion(fight: &Fight, assets: &Assets) {
    let mut attempt = alpha_fight_without_a_supers(fight, assets, V_ISM_FULL_METER);
    for p1 in raging_demon_inputs(&attempt) {
        attempt.step(assets, p1, PlayerInputs::default(), false);
        assert_ne!(attempt.fight_read().fighters[0].move_id, EVIL_RYU_RAGING_DEMON);
    }
    assert_eq!(super_meter(&attempt), V_ISM_FULL_METER);
}

#[test]
fn alpha3_v_evil_ryu_uses_native_variable_combo_against_sagat() {
    let (session, frame) = boot_spike(EvilRyuPlaytest::Alpha3V);
    let fight = session.fight().unwrap();
    let assets = session.fight_assets().unwrap();
    let machine = fight.machine();
    assert_eq!(machine.arcade_opponent(), Some(TournamentFighter::Sagat));
    let [evil_ryu, sagat] = machine.era_fighters();
    assert_eq!((evil_ryu.sub_hp, evil_ryu.max_sub_hp), (EVIL_RYU_MAX_HP, EVIL_RYU_MAX_HP));
    assert_eq!(evil_ryu.style.revision, Revision::Sfz3);
    assert_eq!(evil_ryu.style.loadout, Loadout::VIsm);
    assert_eq!(sagat.style.revision, Revision::Sfz3);
    assert_eq!(sagat.style.loadout, Loadout::AIsm);
    for side in [evil_ryu, sagat] {
        let guard = side.guard_power().unwrap();
        assert_eq!((guard.remaining, guard.maximum), (60, 60));
    }
    let sagat_hud = fight.era_hud().p2.unwrap();
    assert_eq!(sagat_hud.main, GaugeRow::stock(3, 0, Fill::of(0, 48)));
    assert!(sagat_hud.second.is_some());
    assert_evil_ryu_label(&session, &frame);
    assert_super_gauge(fight, &frame, 0);

    check_variable_combo(fight, assets);
    check_v_ism_super_exclusion(fight, assets);
    check_alpha3_fireballs(fight, assets);
    for (motion, slot) in [
        (&[Dir::Forward, Dir::Down, Dir::DownForward][..], EVIL_RYU_ASHURA_FORWARD),
        (&[Dir::Back, Dir::Down, Dir::DownBack][..], EVIL_RYU_ASHURA_BACK),
    ] {
        let punches = alpha3_ashura_ticks(&session, motion, slot, ButtonClass::TriplePunch);
        let kicks = alpha3_ashura_ticks(&session, motion, slot, ButtonClass::TripleKick);
        assert_eq!((punches, kicks), (10, 6));
    }
    assert_alpha_move(fight.clone(), assets, MoveInput {
        motion: &[Dir::Jump, Dir::Down, Dir::DownBack, Dir::Back],
        button: ButtonClass::Kick,
    }, AIR_TATSUMAKI_MOVE, TATSUMAKI_START_GAIN, "Air Tatsumaki");
    assert_alpha_move(fight.clone(), assets, MoveInput {
        motion: &[Dir::Forward],
        button: ButtonClass::MediumKick,
    }, SENPUKYAKU_MOVE, 0, "Senpukyaku");
    check_alpha_throw(fight, assets, true, &[0, 1]);
    check_alpha_throw(fight, assets, true, &[3, 4]);
    check_air_shoulder_throw(fight, assets);
    check_alpha3_counter(fight, assets);

    let rows = move_list_rows(&mut session.clone());
    assert!(rows.iter().any(|row| row.starts_with("VARIABLE COMBO (50%+) ") && row.ends_with("[P+K SAME LV]")));
    assert!(rows.iter().any(|row| row.starts_with("ALPHA COUNTER SWEEP BLOCK ") && row.ends_with("[P+K SAME]")));
    for excluded in ["RAGING DEMON", "SHINKUU HADOKEN", "SHINKUU TATSUMAKI", "MESSATSU GOSHORYU"] {
        assert!(!rows.iter().any(|row| row.starts_with(excluded)), "V-ISM omits {excluded}");
    }
}

fn check_x_ism_no_air_block(fight: &Fight, assets: &Assets) {
    let scripted = fight_with_scripted_sagat(fight);
    let idle = PlayerInputs::default();
    let high_shot = REGISTRY.iter().find(|entry| entry.name == "HIGH TIGER SHOT").unwrap();
    let facing_right = fighter_object(fight, CPU_STRUCT)[0x37] & 4 == 0;
    let shot = high_shot.input.frames(facing_right, Strength::Light);
    for takeoff in (4..42).step_by(2) {
        let mut attempt = scripted.clone();
        for tick in 0..85 {
            let mut p1 = if tick > takeoff { alpha_pad(&attempt, false, &[]) } else { idle };
            p1.directions[3] = tick == takeoff;
            let p2 = shot.get(tick).copied().unwrap_or_default();
            let before = attempt.fight_read().fighters[0];
            let ryu = attempt.machine().era_fighters()[0];
            let airborne = matches!(before.move_id, 9..=0xb) && air_throw_height(&attempt, P1_STRUCT).1 >= 12;
            attempt.step(assets, p1, p2, false);
            let after = attempt.machine().era_fighters()[0];
            if airborne && after.sub_hp < ryu.sub_hp {
                assert!(attempt.fight_read().fighters[0].hitstun(), "the high Tiger Shot hits the X-ISM jump");
                assert!(!attempt.fight_read().fighters[0].blockstun, "holding back in the air cannot block");
                assert_eq!(after.guard_power(), ryu.guard_power(), "airborne damage does not use Guard Power");
                return;
            }
            if after.sub_hp < ryu.sub_hp {
                break;
            }
        }
    }
    panic!("the scripted Tiger Shot can contact Evil Ryu's jump while holding back");
}

#[test]
fn alpha3_x_evil_ryu_uses_native_guard_and_one_stock_demon_against_sagat() {
    let (session, frame) = boot_spike(EvilRyuPlaytest::Alpha3X);
    let fight = session.fight().unwrap();
    let assets = session.fight_assets().unwrap();
    let machine = fight.machine();
    assert_eq!(machine.arcade_opponent(), Some(TournamentFighter::Sagat));
    let [evil_ryu, sagat] = machine.era_fighters();
    assert_eq!((evil_ryu.sub_hp, evil_ryu.max_sub_hp), (EVIL_RYU_MAX_HP, EVIL_RYU_MAX_HP));
    for (side, loadout, guard_max) in [(evil_ryu, Loadout::XIsm, 72), (sagat, Loadout::AIsm, 60)] {
        assert_eq!((side.style.family, side.style.revision, side.style.loadout), (EraFamily::SfZ, Revision::Sfz3, loadout));
        let guard = side.guard_power().unwrap();
        assert_eq!((guard.remaining, guard.maximum), (guard_max, guard_max));
    }
    let sagat_hud = fight.era_hud().p2.unwrap();
    assert_eq!(sagat_hud.main, GaugeRow::stock(3, 0, Fill::of(0, 48)));
    assert!(sagat_hud.second.is_some());
    assert_evil_ryu_label(&session, &frame);
    assert_super_gauge(fight, &frame, 0);

    alpha_fight_without_a_supers(fight, assets, SUPER_STOCK);
    check_alpha_demon(fight, assets, SUPER_STOCK);
    check_alpha3_counter(fight, assets);
    check_x_ism_no_air_block(fight, assets);
    check_alpha3_fireballs(fight, assets);
    for (motion, slot) in [
        (&[Dir::Forward, Dir::Down, Dir::DownForward][..], EVIL_RYU_ASHURA_FORWARD),
        (&[Dir::Back, Dir::Down, Dir::DownBack][..], EVIL_RYU_ASHURA_BACK),
    ] {
        let punches = alpha3_ashura_ticks(&session, motion, slot, ButtonClass::TriplePunch);
        let kicks = alpha3_ashura_ticks(&session, motion, slot, ButtonClass::TripleKick);
        assert_eq!((punches, kicks), (10, 6));
    }
    assert_alpha_move(fight.clone(), assets, MoveInput {
        motion: &[Dir::Jump, Dir::Down, Dir::DownBack, Dir::Back],
        button: ButtonClass::Kick,
    }, AIR_TATSUMAKI_MOVE, TATSUMAKI_START_GAIN, "Air Tatsumaki");
    let mut medium_kick = fight.clone();
    let p1 = alpha_pad(&medium_kick, true, &[4]);
    medium_kick.step(assets, p1, PlayerInputs::default(), false);
    assert_ne!(medium_kick.fight_read().fighters[0].move_id, SENPUKYAKU_MOVE);
    check_alpha_throw(fight, assets, true, &[0, 1]);
    check_alpha_throw(fight, assets, true, &[3, 4]);
    check_air_shoulder_throw(fight, assets);

    let rows = move_list_rows(&mut session.clone());
    assert!(rows.iter().any(|row| row == "SUPER (FULL GAUGE)"));
    assert!(rows.iter().any(|row| row.starts_with("RAGING DEMON [LP] [LP] ") && row.ends_with("[LK] [HP]")));
    for excluded in [
        "SHINKUU HADOKEN", "SHINKUU TATSUMAKI", "MESSATSU GOSHORYU", "ALPHA COUNTER",
        "CUSTOM COMBO", "VARIABLE COMBO", "SENPUKYAKU",
    ] {
        assert!(!rows.iter().any(|row| row.starts_with(excluded)), "X-ISM omits {excluded}");
    }
}

fn ae_revenge(fight: &Fight) -> (Fill, GaugeState) {
    match fight.era_hud().p1.unwrap().second.unwrap() {
        GaugeRow::Bar { fill, state, .. } => (fill, state),
        row => panic!("SFIV Revenge uses a native continuous gauge: {row:?}"),
    }
}

fn approach_scripted_sagat(fight: &mut Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let mut near = 0;
    for _ in 0..90 {
        if center_x(fight, P1_STRUCT).abs_diff(center_x(fight, CPU_STRUCT)) <= THROW_RANGE {
            near += 1;
            if near == 8 {
                break;
            }
        }
        let toward = alpha_pad(fight, true, &[]);
        fight.step(assets, toward, idle, false);
    }
    for _ in 0..8 {
        fight.step(assets, idle, idle, false);
    }
}

fn ae_focus_and_fadc(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let mut charge = fight_with_scripted_sagat(fight);
    approach_scripted_sagat(&mut charge, assets);
    let mut focus = idle;
    focus.buttons[1] = true;
    focus.buttons[4] = true;
    let hp_before = charge.machine().era_fighters()[0].sub_hp;
    let mut clip = AlphaClip::new(fight, "focus-attack", true);
    for tick in 0..22 {
        let mut sagat = idle;
        if tick == 2 {
            sagat.buttons[1] = true;
        }
        clip.push(&charge.step(assets, focus, sagat, false).frame);
    }
    assert_eq!(charge.fight_read().fighters[0].move_id, AE_FOCUS_CHARGE,
        "Focus stays charged after absorbing Sagat's attack");
    assert!(charge.machine().era_fighters()[0].sub_hp < hp_before);
    assert!(ae_revenge(&charge).0.num > 0);
    assert_eq!(super_meter(&charge), 0, "raw Focus does not spend Super");

    let sagat_hp = charge.machine().era_fighters()[1].sub_hp;
    let mut released = false;
    for _ in 0..44 {
        clip.push(&charge.step(assets, idle, idle, false).frame);
        released |= charge.fight_read().fighters[0].move_id == AE_FOCUS_ATTACK;
    }
    assert!(released, "Focus releases its own attack stream");
    assert!(charge.machine().era_fighters()[1].sub_hp < sagat_hp, "released Focus reaches Sagat");
    clip.finish();

    let base = fight_with_meter(&fight_with_scripted_sagat(fight), 3 * SUPER_STOCK);
    let ex_hadoken = MoveInput {
        motion: &[Dir::Down, Dir::DownForward, Dir::Forward],
        button: ButtonClass::PairPunch,
    };
    assert_alpha_move(base.clone(), assets, ex_hadoken, HADOKEN_MOVE, 2 * SUPER_STOCK, "EX Hadoken");
    let mut cancel = base;
    let mut clip = AlphaClip::new(fight, "fadc", true);
    let facing_right = fighter_object(&cancel, P1_STRUCT)[0x37] & 4 == 0;
    for p1 in ex_hadoken.frames(facing_right, Strength::Light) {
        clip.push(&cancel.step(assets, p1, idle, false).frame);
        if cancel.fight_read().fighters[0].move_id == HADOKEN_MOVE {
            break;
        }
    }
    assert_eq!(cancel.fight_read().fighters[0].move_id, HADOKEN_MOVE);
    assert_eq!(super_meter(&cancel), 2 * SUPER_STOCK);
    let frame = cancel.step(assets, focus, idle, false).frame;
    assert_eq!(cancel.fight_read().fighters[0].move_id, AE_FOCUS_CHARGE);
    assert_super_gauge(&cancel, &frame, 0);
    clip.push(&frame);
    for forward in [true, false, true] {
        let p1 = if forward { alpha_pad(&cancel, true, &[1, 4]) } else { focus };
        clip.push(&cancel.step(assets, p1, idle, false).frame);
    }
    assert_eq!(cancel.fight_read().fighters[0].move_id, AE_DASH);
    clip.idle_tail(&mut cancel, assets, 36);
    clip.finish();
}

fn ae_ultra(fight: &Fight, assets: &Assets, slot: u8, button: ButtonClass, name: &str) {
    let idle = PlayerInputs::default();
    let input = MoveInput {
        motion: &[Dir::Down, Dir::DownForward, Dir::Forward, Dir::Down, Dir::DownForward, Dir::Forward],
        button,
    };
    let mut uncharged = fight_with_scripted_sagat(fight);
    let facing_right = fighter_object(&uncharged, P1_STRUCT)[0x37] & 4 == 0;
    for p1 in input.frames(facing_right, Strength::Light) {
        uncharged.step(assets, p1, idle, false);
        assert_ne!(uncharged.fight_read().fighters[0].move_id, slot, "empty Revenge refuses {name}");
    }

    // Actual Sagat normals charge Revenge in a scripted clone. No Revenge
    // resource is seeded in the fixture.
    let mut charged = fight_with_scripted_sagat(fight);
    let ready = |fight: &Fight| fight.fight_read().fighters.iter().all(|fighter| {
        !fighter.locked() && !fighter.reacting() && !fighter.blockstun
    });
    for _ in 0..6 {
        if ae_revenge(&charged).0.num >= 320 {
            break;
        }
        approach_scripted_sagat(&mut charged, assets);
        let before = charged.machine().era_fighters()[0].sub_hp;
        let mut sagat_hp = idle;
        sagat_hp.buttons[2] = true;
        charged.step(assets, idle, sagat_hp, false);
        for _ in 0..90 {
            charged.step(assets, idle, idle, false);
            if charged.machine().era_fighters()[0].sub_hp < before
                && ready(&charged)
            {
                break;
            }
        }
    }
    let (fill, gauge_state) = ae_revenge(&charged);
    assert_eq!(fill.den, 640);
    assert!(fill.num >= 320 && matches!(gauge_state, GaugeState::Flashing { .. }),
        "actual hits from Sagat earn at least half Revenge");
    assert!(ready(&charged), "both fighters recover from the hits that earned Revenge");
    if button == ButtonClass::TriplePunch {
        for _ in 0..22 {
            let back = alpha_pad(&charged, false, &[]);
            charged.step(assets, back, idle, false);
        }
    } else {
        approach_scripted_sagat(&mut charged, assets);
    }

    let facing_right = fighter_object(&charged, P1_STRUCT)[0x37] & 4 == 0;
    let other_button = if button == ButtonClass::TriplePunch { ButtonClass::TripleKick } else { ButtonClass::TriplePunch };
    let mut wrong = charged.clone();
    let wrong_input = MoveInput { button: other_button, ..input };
    for p1 in wrong_input.frames(facing_right, Strength::Light) {
        wrong.step(assets, p1, idle, false);
        assert!(!matches!(wrong.fight_read().fighters[0].move_id, ALPHA_SUPER_SECOND_SLOT | ALPHA_SUPER_THIRD_SLOT));
    }
    assert_eq!(ae_revenge(&wrong).0, fill, "the unselected Ultra leaves Revenge alone");

    if button == ButtonClass::TriplePunch {
        check_metsu_charge(&charged, assets, input, slot, facing_right);
        return;
    }

    let mut clip = AlphaClip::new(fight, name, true);
    let sagat_hp = charged.machine().era_fighters()[1].sub_hp;
    let super_before = super_meter(&charged);
    let mut started = false;
    for p1 in input.frames(facing_right, Strength::Light) {
        let frame = charged.step(assets, p1, idle, false).frame;
        clip.push(&frame);
        let ryu = charged.fight_read().fighters[0];
        if ryu.move_id == slot {
            assert_eq!(ae_revenge(&charged).0.num, 0, "Ultra spends Revenge");
            assert_super_gauge(&charged, &frame, super_before);
            started = true;
            break;
        }
    }
    assert!(started, "earned Revenge starts the selected Ultra {name}; P1 state {:?}", charged.fight_read().fighters[0]);
    for _ in 0..108 {
        clip.push(&charged.step(assets, idle, idle, false).frame);
    }
    assert!(charged.machine().era_fighters()[1].sub_hp < sagat_hp, "{name} hits Sagat");
    clip.finish();
}

#[derive(Clone, Copy)]
enum MetsuHold {
    Tap,
    RepressAfterFreezeRelease,
    ReleaseOn(u16),
    ThroughAutoRelease,
}

struct MetsuShot {
    launch_tick: u16,
    palette: u8,
    damage: u16,
}

fn metsu_shot(
    charged: &Fight,
    assets: &Assets,
    input: MoveInput,
    slot: u8,
    facing_right: bool,
    hold: MetsuHold,
    capture: Option<&str>,
) -> MetsuShot {
    let idle = PlayerInputs::default();
    let mut fight = charged.clone();
    let mut clip = AlphaClip::new(charged, capture.unwrap_or("metsu-hadoken"), capture.is_some());
    let sagat_hp = fight.machine().era_fighters()[1].sub_hp;
    let super_before = super_meter(&fight);
    let started = input.frames(facing_right, Strength::Light).into_iter().any(|p1| {
        let frame = fight.step(assets, p1, idle, false).frame;
        clip.push(&frame);
        if fight.fight_read().fighters[0].move_id != slot {
            return false;
        }
        assert_eq!(ae_revenge(&fight).0.num, 0, "Metsu commits Revenge on activation");
        assert_super_gauge(&fight, &frame, super_before);
        true
    });
    assert!(started, "earned Revenge starts Metsu Hadoken");
    assert!(fight.machine().era_super_freeze() > 0);

    let mut elapsed = 0;
    let mut projectile = None;
    for tick in 0..215 {
        let frozen = fight.machine().era_super_freeze() != 0;
        elapsed += u16::from(!frozen);
        let mut p1 = idle;
        match hold {
            MetsuHold::Tap => {}
            MetsuHold::RepressAfterFreezeRelease => {
                p1.buttons[..3].fill(true);
                if tick == 1 {
                    assert!(frozen);
                    p1.buttons[1] = false;
                }
            }
            MetsuHold::ReleaseOn(release) if frozen || elapsed < release => p1.buttons[..3].fill(true),
            MetsuHold::ReleaseOn(_) => {}
            MetsuHold::ThroughAutoRelease => p1.buttons[..3].fill(true),
        }
        let frame = fight.step(assets, p1, idle, false).frame;
        clip.push(&frame);
        if projectile.is_none() {
            assert_eq!(ae_revenge(&fight).0.num, 0);
            assert_eq!(super_meter(&fight), super_before, "charge and release do not spend Super");
            if let Some(palette) = p1_projectile_palette(&fight, slot) {
                projectile = Some((elapsed, palette));
            }
        }
        if projectile.is_some_and(|(launch, _)| elapsed >= launch + 85) {
            break;
        }
    }
    let (launch_tick, palette) = projectile.expect("Metsu launches a visible native projectile");
    let damage = sagat_hp - fight.machine().era_fighters()[1].sub_hp;
    assert!(damage > 0, "Metsu hits Sagat");
    clip.finish();
    MetsuShot { launch_tick, palette, damage }
}

fn check_metsu_charge(charged: &Fight, assets: &Assets, input: MoveInput, slot: u8, facing_right: bool) {
    let tap = metsu_shot(charged, assets, input, slot, facing_right, MetsuHold::Tap, Some("metsu-hadoken"));
    let broken = metsu_shot(charged, assets, input, slot, facing_right, MetsuHold::RepressAfterFreezeRelease, None);
    assert_eq!((broken.launch_tick, broken.palette, broken.damage), (tap.launch_tick, tap.palette, tap.damage),
        "releasing one punch during the freeze breaks charge and re-pressing cannot restart it");

    assert!(tap.launch_tick > 1, "Metsu has a startup before the projectile");
    let early = metsu_shot(charged, assets, input, slot, facing_right, MetsuHold::ReleaseOn(tap.launch_tick - 1), None);
    assert_eq!((early.launch_tick, early.damage), (tap.launch_tick, tap.damage));

    let partial = metsu_shot(charged, assets, input, slot, facing_right, MetsuHold::ReleaseOn(tap.launch_tick + 9),
        Some("metsu-hadoken-charged"));
    assert_eq!(partial.launch_tick, tap.launch_tick + 9, "partial hold releases the shot on the released frame");
    assert_eq!((partial.palette, partial.damage), (tap.palette, tap.damage), "a partial hold retains tap damage");

    let full = metsu_shot(charged, assets, input, slot, facing_right, MetsuHold::ThroughAutoRelease,
        Some("metsu-hadoken-auto-release"));
    assert_eq!(full.launch_tick, tap.launch_tick + 28, "a full uninterrupted hold auto-releases at the cap");
    assert_ne!(tap.palette, 14);
    assert_eq!(full.palette, 14, "the full charge uses the red projectile palette");
    let damage_ratio = u32::from(full.damage) * 100 / u32::from(tap.damage);
    assert!((112..=118).contains(&damage_ratio), "full charge is about 15% stronger, observed {damage_ratio}%");
}

fn check_ae_carryovers(session: &GameSession, fight: &Fight, assets: &Assets) {
    let base = fight_with_scripted_sagat(fight);
    let full = fight_with_meter(&base, 3 * SUPER_STOCK);
    let air_tatsu = MoveInput {
        motion: &[Dir::Jump, Dir::Down, Dir::DownBack, Dir::Back], button: ButtonClass::Kick,
    };
    assert_alpha_move(base.clone(), assets, air_tatsu, AIR_TATSUMAKI_MOVE, 4, "Air Tatsumaki");
    assert_alpha_move(full, assets, MoveInput { button: ButtonClass::PairKick, ..air_tatsu },
        AIR_TATSUMAKI_MOVE, 2 * SUPER_STOCK, "EX Air Tatsumaki");

    let mut close = base.clone();
    approach_scripted_sagat(&mut close, assets);
    let idle = PlayerInputs::default();
    let hk_startup = if fight.machine().era_fighters()[0].style.revision == Revision::Usf4 { 25 } else { 26 };
    for (button, strength, expected, name) in [
        (ButtonClass::Kick, Strength::Light, (17, 2, 22), "ryusokyaku"),
        (ButtonClass::Kick, Strength::Medium, (20, 2, 20), "ryusokyaku-mk"),
        (ButtonClass::Kick, Strength::Heavy, (hk_startup, 2, 18), "ryusokyaku-hk"),
        (ButtonClass::PairKick, Strength::Light, (21, 2, 22), "ex-ryusokyaku"),
    ] {
        let (stock, meter_after) = if button == ButtonClass::PairKick {
            (3 * SUPER_STOCK, 2 * SUPER_STOCK)
        } else {
            (0, 4)
        };
        let input = MoveInput { motion: &[Dir::Back, Dir::DownBack, Dir::Down, Dir::DownForward, Dir::Forward], button };
        let mut whiff = fight_with_meter(&base, stock);
        let facing_right = fighter_object(&whiff, P1_STRUCT)[0x37] & 4 == 0;
        let hp_before = whiff.machine().era_fighters()[1].sub_hp;
        let mut elapsed = 0;
        let mut first_active = None;
        let mut active = 0;
        for p1 in input.frames(facing_right, strength).into_iter().chain(std::iter::repeat_n(idle, 65)) {
            whiff.step(assets, p1, idle, false);
            let ryu = whiff.fight_read().fighters[0];
            if ryu.move_id == RYUSOKYAKU_MOVE && ryu.state & 0x80 != 0 {
                elapsed += 1;
                if ryu.attack_active {
                    first_active.get_or_insert(elapsed);
                    active += 1;
                }
            } else if elapsed != 0 {
                break;
            }
        }
        assert_eq!(whiff.machine().era_fighters()[1].sub_hp, hp_before, "{name} timing uses a clean whiff");
        let startup = first_active.expect("the axe kick exposes an active hitbox");
        let recovery = elapsed - startup + 1 - active;
        assert_eq!((startup, active, recovery), expected,
            "{name} first-active frame (1-indexed), active frames and recovery");

        let mut axe = fight_with_meter(&close, stock);
        let facing_right = fighter_object(&axe, P1_STRUCT)[0x37] & 4 == 0;
        let mut clip = AlphaClip::new(fight, name, true);
        let hp_before = axe.machine().era_fighters()[1].sub_hp;
        let mut started = false;
        for p1 in input.frames(facing_right, strength).into_iter().chain(std::iter::repeat_n(idle, 65)) {
            let frame = axe.step(assets, p1, idle, false).frame;
            clip.push(&frame);
            if !started && axe.fight_read().fighters[0].move_id == RYUSOKYAKU_MOVE {
                assert_super_gauge(&axe, &frame, meter_after);
                started = true;
            }
        }
        assert!(started && axe.machine().era_fighters()[1].sub_hp < hp_before, "{name} lands on Sagat");
        clip.finish();
    }
    for (name, motion, slot, red) in [
        ("hadoken", &[Dir::Down, Dir::DownForward, Dir::Forward][..], HADOKEN_MOVE, false),
        ("shakunetsu", &[Dir::Forward, Dir::DownForward, Dir::Down, Dir::DownBack, Dir::Back][..], SF2_SPECIAL_SLOT, true),
    ] {
        let mut attempt = base.clone();
        let idle = PlayerInputs::default();
        let mut clip = AlphaClip::new(fight, name, true);
        let facing_right = fighter_object(&attempt, P1_STRUCT)[0x37] & 4 == 0;
        let inputs = MoveInput { motion, button: ButtonClass::Punch }.frames(facing_right, Strength::Heavy);
        let palette = inputs.into_iter().chain(std::iter::repeat_n(idle, 52)).find_map(|p1| {
            clip.push(&attempt.step(assets, p1, idle, false).frame);
            p1_projectile_palette(&attempt, slot)
        }).unwrap_or_else(|| panic!("{name} produces its native projectile"));
        assert_eq!(palette == 14, red);
        assert_eq!(super_meter(&attempt), 4);
        clip.idle_tail(&mut attempt, assets, 40);
        clip.finish();
    }
    for (motion, slot) in [
        (&[Dir::Forward, Dir::Down, Dir::DownForward][..], EVIL_RYU_ASHURA_FORWARD),
        (&[Dir::Back, Dir::Down, Dir::DownBack][..], AE_ASHURA_BACK),
    ] {
        for button in [ButtonClass::TriplePunch, ButtonClass::TripleKick] {
            ashura_frame(&mut session.clone(), motion, slot, button);
        }
    }

    for backward in [false, true] {
        let mut attempt = close.clone();
        let mut input = if backward { alpha_pad(&attempt, false, &[]) } else { idle };
        input.buttons[0] = true;
        input.buttons[3] = true;
        let mut clip = AlphaClip::new(fight, if backward { "tomoe-nage" } else { "shoulder-throw" }, true);
        let output = attempt.step(assets, input, idle, false);
        assert_eq!(attempt.fight_read().fighters.map(|fighter| fighter.move_id), [THROW_MOVE, THROWN_MOVE],
            "SFIV LP+LK throws in range with neutral or directional input");
        clip.push(&output.frame);
        clip.idle_tail(&mut attempt, assets, 65);
        clip.finish();
    }
}

fn verify_sf4_profile(session: &GameSession, frame: &Frame, revision: Revision, loadouts: [Loadout; 2]) {
    let fight = session.fight().unwrap();
    let [evil_ryu, sagat] = fight.machine().era_fighters();
    assert_eq!(fight.machine().arcade_opponent(), Some(TournamentFighter::Sagat));
    assert_eq!(sagat.fighter, Some(TournamentFighter::Sagat));
    assert_eq!((evil_ryu.sub_hp, evil_ryu.max_sub_hp), (EVIL_RYU_MAX_HP, EVIL_RYU_MAX_HP));
    for (side, ultra) in [evil_ryu, sagat].into_iter().zip(loadouts) {
        assert_eq!((side.style.family, side.style.revision, side.style.loadout), (EraFamily::Sf4, revision, ultra));
        assert!(side.guard_power().is_none());
        assert_eq!(side.gauge(), Some(EraGauge::two(
            GaugeRow::stock(4, 0, Fill::of(0, 48)),
            GaugeRow::bar(Fill::of(0, 640), Fill::FULL),
        )));
    }
    assert_evil_ryu_label(session, frame);
    assert_super_gauge(fight, frame, 0);
}

#[test]
fn ae2012_evil_ryu_uses_focus_demon_and_metsu_hadoken_against_sagat() {
    let (session, frame) = boot_spike(EvilRyuPlaytest::Ae2012);
    let fight = session.fight().unwrap();
    let assets = session.fight_assets().unwrap();
    verify_sf4_profile(&session, &frame, Revision::Ae2012, [Loadout::Ultra1, Loadout::Ultra2]);
    check_alpha_demon(fight, assets, AE_FULL_SUPER);
    ae_focus_and_fadc(fight, assets);
    check_ae_carryovers(&session, fight, assets);
    ae_ultra(fight, assets, ALPHA_SUPER_SECOND_SLOT, ButtonClass::TriplePunch, "metsu-hadoken");
    let rows = move_list_rows(&mut session.clone());
    for name in ["METSU HADOKEN ", "RYUSOKYAKU ", "SHAKUNETSU HADOKEN ", "RAGING DEMON ", "FOCUS ATTACK "] {
        assert!(rows.iter().any(|row| row.starts_with(name)), "AE2012 Moves includes {name}");
    }
    assert!(rows.iter().any(|row| row.starts_with("METSU HADOKEN ") && row.ends_with("[PPP] HOLD/RELEASE")));
    assert!(!rows.iter().any(|row| row.starts_with("MESSATSU GOSHORYU ") || row.starts_with("SHINKU HADOKEN ")));
}

#[test]
fn ae2012_ultra_ii_evil_ryu_uses_messatsu_goshoryu_against_sagat() {
    let (session, frame) = boot_spike(EvilRyuPlaytest::Ae2012Ultra2);
    let fight = session.fight().unwrap();
    verify_sf4_profile(&session, &frame, Revision::Ae2012, [Loadout::Ultra2, Loadout::Ultra2]);
    ae_ultra(fight, session.fight_assets().unwrap(), ALPHA_SUPER_THIRD_SLOT, ButtonClass::TripleKick, "messatsu-goshoryu");
    let rows = move_list_rows(&mut session.clone());
    assert!(rows.iter().any(|row| row.starts_with("MESSATSU GOSHORYU ")));
    assert!(!rows.iter().any(|row| row.starts_with("METSU HADOKEN ")));
}

fn check_usf4_red_focus(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let mut normal = idle;
    normal.buttons[1] = true;
    normal.buttons[4] = true;
    let mut red = normal;
    red.buttons[0] = true;
    let mut sagat_punch = idle;
    sagat_punch.buttons[1] = true;

    let mut close = fight_with_scripted_sagat(fight);
    approach_scripted_sagat(&mut close, assets);
    let mut short = fight_with_meter(&close, 2 * SUPER_STOCK - 1);
    let frame = short.step(assets, red, idle, false).frame;
    assert_ne!(short.fight_read().fighters[0].move_id, AE_FOCUS_CHARGE);
    assert_super_gauge(&short, &frame, 2 * SUPER_STOCK - 1);

    let hp_before = close.machine().era_fighters()[0].sub_hp;
    for (chord, name, stock_cost) in [
        (normal, "focus-attack", 0),
        (red, "red-focus", 2 * SUPER_STOCK),
    ] {
        let mut charge = fight_with_meter(&close, AE_FULL_SUPER);
        let mut clip = AlphaClip::new(fight, name, true);
        for _ in 0..14 {
            clip.push(&charge.step(assets, idle, idle, false).frame);
        }
        let frame = charge.step(assets, chord, idle, false).frame;
        assert_eq!(charge.fight_read().fighters[0].move_id, AE_FOCUS_CHARGE);
        assert_super_gauge(&charge, &frame, AE_FULL_SUPER - stock_cost);
        clip.push(&frame);

        let mut before_contact = hp_before;
        let contacts = if stock_cost > 0 { 2 } else { 1 };
        for contact in 1..=contacts {
            for _ in 0..36 {
                if charge.fight_read().fighters[1].state & 0x80 == 0 {
                    break;
                }
                clip.push(&charge.step(assets, chord, idle, false).frame);
            }
            assert_eq!(charge.fight_read().fighters[1].state & 0x80, 0,
                "Sagat is ready to start punch {contact}");
            // The first punch pushes the fighters apart even through armor.
            // Walk Sagat back into range while Evil Ryu keeps the chord held.
            for _ in 0..10 {
                if center_x(&charge, P1_STRUCT).abs_diff(center_x(&charge, CPU_STRUCT)) <= THROW_RANGE {
                    break;
                }
                let sagat_toward = alpha_pad(&charge, false, &[]);
                clip.push(&charge.step(assets, chord, sagat_toward, false).frame);
            }
            clip.push(&charge.step(assets, chord, sagat_punch, false).frame);
            let started = charge.fight_read().fighters[1];
            let start_gap = center_x(&charge, P1_STRUCT).abs_diff(center_x(&charge, CPU_STRUCT));
            for _ in 0..24 {
                if charge.machine().era_fighters()[0].sub_hp < before_contact {
                    break;
                }
                clip.push(&charge.step(assets, chord, idle, false).frame);
            }
            let hp = charge.machine().era_fighters()[0].sub_hp;
            assert!(hp < before_contact, "Sagat lands punch {contact} during {name}, Sagat starts {started:?} at gap {start_gap}; end {:?}",
                charge.fight_read().fighters);
            assert_eq!(charge.fight_read().fighters[0].move_id, AE_FOCUS_CHARGE,
                "{name} armor after punch {contact}");
            let damage = hp_before - hp;
            let expected_revenge = if stock_cost > 0 { 2 * damage } else { damage };
            assert_eq!(ae_revenge(&charge).0.num, expected_revenge.min(640));
            before_contact = hp;
        }

        if stock_cost > 0 {
            let sagat_hp = charge.machine().era_fighters()[1].sub_hp;
            assert!((0..10).any(|_| {
                clip.push(&charge.step(assets, normal, idle, false).frame);
                charge.fight_read().fighters[0].move_id == AE_FOCUS_ATTACK
            }),
                "releasing LP ends Red Focus even while MP+MK are held");
            for _ in 0..55 {
                clip.push(&charge.step(assets, idle, idle, false).frame);
            }
            assert!(charge.machine().era_fighters()[1].sub_hp < sagat_hp, "released Red Focus hits Sagat");
        } else {
            for _ in 0..55 {
                clip.push(&charge.step(assets, idle, idle, false).frame);
            }
        }
        clip.finish();
    }

    let mut hadoken = fight_with_meter(&close, AE_FULL_SUPER);
    let mut clip = AlphaClip::new(fight, "ex-red-focus", true);
    for _ in 0..14 {
        clip.push(&hadoken.step(assets, idle, idle, false).frame);
    }
    let input = MoveInput {
        motion: &[Dir::Down, Dir::DownForward, Dir::Forward], button: ButtonClass::Punch,
    };
    let facing_right = fighter_object(&hadoken, P1_STRUCT)[0x37] & 4 == 0;
    for p1 in input.frames(facing_right, Strength::Light) {
        clip.push(&hadoken.step(assets, p1, idle, false).frame);
        if hadoken.fight_read().fighters[0].move_id == HADOKEN_MOVE {
            break;
        }
    }
    assert_eq!(hadoken.fight_read().fighters[0].move_id, HADOKEN_MOVE);
    let mut short = fight_with_meter(&hadoken, 3 * SUPER_STOCK - 1);
    let frame = short.step(assets, red, idle, false).frame;
    assert_eq!(short.fight_read().fighters[0].move_id, HADOKEN_MOVE, "143 cannot cancel a special into EX Red Focus");
    assert_super_gauge(&short, &frame, 3 * SUPER_STOCK - 1);

    let mut cancel = hadoken;
    let frame = cancel.step(assets, red, idle, false).frame;
    assert_eq!(cancel.fight_read().fighters[0].move_id, AE_FOCUS_CHARGE);
    assert_super_gauge(&cancel, &frame, SUPER_STOCK);
    clip.push(&frame);
    let frame = cancel.step(assets, idle, idle, false).frame;
    assert_eq!(cancel.fight_read().fighters[0].move_id, AE_FOCUS_ATTACK, "EX Red Focus releases immediately at level one");
    clip.push(&frame);
    let sagat_hp = cancel.machine().era_fighters()[1].sub_hp;
    for _ in 0..62 {
        clip.push(&cancel.step(assets, idle, idle, false).frame);
    }
    assert!(cancel.machine().era_fighters()[1].sub_hp < sagat_hp, "the EX Red Focus release hits Sagat");
    clip.finish();
}

fn earn_usf4_revenge(fight: &mut Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let mut sagat_punch = idle;
    sagat_punch.buttons[1] = true;
    for _ in 0..8 {
        if ae_revenge(fight).0.num >= 320 {
            break;
        }
        approach_scripted_sagat(fight, assets);
        let before = fight.machine().era_fighters()[0].sub_hp;
        fight.step(assets, idle, sagat_punch, false);
        for _ in 0..90 {
            fight.step(assets, idle, idle, false);
            if fight.machine().era_fighters()[0].sub_hp < before
                && fight.fight_read().fighters.iter().all(|fighter| {
                    fighter.state & 0x80 == 0 && !fighter.locked() && !fighter.reacting()
                })
            {
                break;
            }
        }
    }
    let (fill, state) = ae_revenge(fight);
    assert!(fill.num >= 320 && matches!(state, GaugeState::Flashing { .. }),
        "Sagat's real attacks earn the next Ultra, observed {fill:?}");
    assert!(fight.machine().era_fighters()[0].sub_hp > 0, "both Ultras fit in the same round");
}

fn run_usf4_ultra(
    fight: &mut Fight,
    assets: &Assets,
    input: MoveInput,
    slot: u8,
    name: &str,
    capture: bool,
) -> u16 {
    let idle = PlayerInputs::default();
    let facing_right = fighter_object(fight, P1_STRUCT)[0x37] & 4 == 0;
    let mut clip = AlphaClip::new(fight, name, capture);
    let sagat_hp = fight.machine().era_fighters()[1].sub_hp;
    let stock = super_meter(fight);
    let started = input.frames(facing_right, Strength::Light).into_iter().any(|p1| {
        let frame = fight.step(assets, p1, idle, false).frame;
        clip.push(&frame);
        if fight.fight_read().fighters[0].move_id != slot {
            return false;
        }
        assert_eq!(ae_revenge(fight).0.num, 0, "{name} spends all Revenge at activation");
        assert_super_gauge(fight, &frame, stock);
        true
    });
    assert!(started, "earned Revenge starts {name}; P1 {:?}", fight.fight_read().fighters[0]);
    let mut first_damage = None;
    for _ in 0..115 {
        clip.push(&fight.step(assets, idle, idle, false).frame);
        let damage = sagat_hp - fight.machine().era_fighters()[1].sub_hp;
        if damage > 0 {
            first_damage.get_or_insert(damage);
        }
    }
    clip.finish();
    first_damage.unwrap_or_else(|| panic!("{name} hits Sagat"))
}

fn check_usf4_ultra_double(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let mut double = fight_with_scripted_sagat(fight);
    for (slot, name, button, single_loadout) in [
        (ALPHA_SUPER_SECOND_SLOT, "metsu-hadoken", ButtonClass::TriplePunch, Loadout::Ultra1),
        (ALPHA_SUPER_THIRD_SLOT, "messatsu-goshoryu", ButtonClass::TripleKick, Loadout::Ultra2),
    ] {
        let input = MoveInput {
            motion: &[Dir::Down, Dir::DownForward, Dir::Forward, Dir::Down, Dir::DownForward, Dir::Forward],
            button,
        };
        let mut empty = double.clone();
        let facing_right = fighter_object(&empty, P1_STRUCT)[0x37] & 4 == 0;
        for p1 in input.frames(facing_right, Strength::Light) {
            empty.step(assets, p1, idle, false);
            assert_ne!(empty.fight_read().fighters[0].move_id, slot, "empty Revenge refuses {name}");
        }

        earn_usf4_revenge(&mut double, assets);
        approach_scripted_sagat(&mut double, assets);
        let metsu = button == ButtonClass::TriplePunch;
        if metsu {
            assert!((0..80).any(|_| {
                let back = alpha_pad(&double, false, &[]);
                double.step(assets, back, idle, false);
                center_x(&double, P1_STRUCT).abs_diff(center_x(&double, CPU_STRUCT)) >= 120
            }), "Metsu has enough space to show its projectile before contact");
            let facing_right = fighter_object(&double, P1_STRUCT)[0x37] & 4 == 0;
            check_metsu_charge(&double, assets, input, slot, facing_right);
        }

        // Compare at identical spacing and earned Revenge. Only the
        // loadout changes in the fork. The Double fight continues to Ultra II.
        let mut single = double.clone();
        let mut snapshot = single.save_netplay_snapshot(0);
        snapshot.machine.toggle_state.era.fighters[0].style.loadout = single_loadout;
        single.restore_netplay_snapshot(&snapshot);
        assert_eq!(ae_revenge(&single).0, ae_revenge(&double).0);
        let single_damage = run_usf4_ultra(&mut single, assets, input, slot, name, false);
        let double_damage = run_usf4_ultra(&mut double, assets, input, slot, name, !metsu);
        assert!(double_damage.abs_diff(single_damage * 3 / 4) <= single_damage / 100 + 1,
            "{name} uses the Double scaling after rounding: {double_damage} vs {single_damage} single");
        assert_eq!(double.machine().era_fighters()[0].style.loadout, Loadout::UltraDouble);
        assert_eq!(ae_revenge(&double).0.num, 0);
    }
}

#[test]
fn usf4_evil_ryu_uses_red_focus_and_both_ultras_against_sagat() {
    let (session, frame) = boot_spike(EvilRyuPlaytest::Usf4);
    verify_sf4_profile(&session, &frame, Revision::Usf4, [Loadout::UltraDouble, Loadout::Ultra1]);
    let fight = session.fight().unwrap();
    let assets = session.fight_assets().unwrap();
    check_usf4_red_focus(fight, assets);
    check_usf4_ultra_double(fight, assets);
    check_alpha_demon(fight, assets, AE_FULL_SUPER);
    check_ae_carryovers(&session, fight, assets);
    let rows = move_list_rows(&mut session.clone());
    for label in ["METSU HADOKEN ", "MESSATSU GOSHORYU ", "ULTRA DOUBLE (75%)", "RED FOCUS (2 STOCKS)", "EX RED FOCUS (3 STOCKS)"] {
        assert!(rows.iter().any(|row| row.starts_with(label)), "USFIV Moves includes {label}");
    }
}

const KAGE_SENHA: u8 = 0x2b;
const KAGE_TRIGGER: u8 = 0x2a;
const KAGE_ASHURA_FORWARD: u8 = 0x25;
const KAGE_ASHURA_BACK: u8 = 0x24;
const KAGE_ASHURA_AIR: u8 = 0x26;
const KAGE_SEKIEIKEN: u8 = 0x1e;
const KAGE_MISOGI: u8 = 0x1d;
const KAGE_EX_SHORYUKEN: u8 = 0x21;
const KAGE_DEMON: u8 = 0x2b;
const KAGE_II_FULL_V: u16 = 3 * SUPER_STOCK as u16;

fn kage_v_gauge(fight: &Fight) -> (u16, GaugeState) {
    let cells = if fight.machine().era_fighters()[0].style.loadout == Loadout::Skill2Trigger2 { 3 } else { 2 };
    match fight.era_hud().p1.unwrap().second.unwrap() {
        GaugeRow::Stock { cells: displayed, full, part, state } if displayed == cells => {
            (u16::from(full) * u16::from(SUPER_STOCK) + part.num, state)
        }
        row => panic!("Kage's V-Gauge has {cells} native stocks: {row:?}"),
    }
}

fn kage_senha(
    fight: &mut Fight,
    assets: &Assets,
    hold: bool,
    clip_name: Option<&str>,
) -> (usize, u16) {
    let idle = PlayerInputs::default();
    let mut skill = idle;
    skill.buttons[1] = true;
    skill.buttons[4] = true;
    let mut clip = AlphaClip::new(fight, clip_name.unwrap_or("senha-tap"), clip_name.is_some());
    if clip_name.is_some() {
        for _ in 0..12 {
            clip.push(&fight.step(assets, idle, idle, false).frame);
        }
    }
    let [kage, sagat] = fight.machine().era_fighters();
    let before = (kage.sub_hp, sagat.sub_hp);
    let mut first_hit = None;
    let mut absorbed = false;
    for tick in 0..90 {
        let p1 = if tick == 0 || (hold && tick < 50) { skill } else { idle };
        let mut p2 = idle;
        if hold && tick == 15 {
            p2.buttons[1] = true;
        }
        clip.push(&fight.step(assets, p1, p2, false).frame);
        let move_id = fight.fight_read().fighters[0].move_id;
        let [kage, sagat] = fight.machine().era_fighters();
        if tick == 0 {
            assert_eq!(move_id, KAGE_SENHA, "MP+MK starts Senha Kassatsu");
        }
        if kage.sub_hp < before.0 && move_id == KAGE_SENHA {
            absorbed = true;
        }
        if sagat.sub_hp < before.1 {
            first_hit.get_or_insert(tick);
        }
    }
    if hold {
        assert!(absorbed, "charged Senha absorbs Sagat's punch and keeps attacking");
    }
    clip.finish();
    (
        first_hit.expect("Senha Kassatsu reaches close Sagat"),
        before.1 - fight.machine().era_fighters()[1].sub_hp,
    )
}

fn kage_ashura(fight: &Fight, assets: &Assets, forward: bool, slot: u8, name: &str) {
    let idle = PlayerInputs::default();
    let mut glide = fight.clone();
    let mut clip = AlphaClip::new(fight, name, true);
    for _ in 0..12 {
        clip.push(&glide.step(assets, idle, idle, false).frame);
    }
    if slot == KAGE_ASHURA_AIR {
        let mut jump = idle;
        jump.directions[3] = true;
        clip.push(&glide.step(assets, jump, idle, false).frame);
        assert!((0..18).any(|_| {
            clip.push(&glide.step(assets, idle, idle, false).frame);
            matches!(glide.fight_read().fighters[0].move_id, 9..=0xb)
                && air_throw_height(&glide, P1_STRUCT).1 >= 24
        }), "Kage jumps before airborne Ashura");
    }
    let x = center_x(&glide, P1_STRUCT);
    let sagat_x = center_x(&glide, CPU_STRUCT);
    let hp = glide.machine().era_fighters()[1].sub_hp;
    let (timer, state) = kage_v_gauge(&glide);
    assert_eq!(state, GaugeState::Draining);

    let input = alpha_pad(&glide, forward, &[2, 5]);
    clip.push(&glide.step(assets, input, idle, false).frame);
    assert_eq!(glide.fight_read().fighters[0].move_id, slot, "{name}");
    assert!(kage_v_gauge(&glide).0 + 2 < timer, "{name} spends V-Timer on use");
    for _ in 0..56 {
        clip.push(&glide.step(assets, idle, idle, false).frame);
    }
    let finish = center_x(&glide, P1_STRUCT);
    assert!(x.abs_diff(finish) >= 32, "{name} moves Kage horizontally");
    assert_eq!(finish > x, forward == (sagat_x > x), "{name} follows the requested direction");
    assert_eq!(glide.machine().era_fighters()[1].sub_hp, hp, "Ashura Senku does no damage");
    clip.finish();
}

#[test]
fn sfv_kage_uses_senha_kassatsu_and_taigyaku_mudo_against_sagat() {
    let (session, frame) = boot_spike(EvilRyuPlaytest::Kage);
    let fight = session.fight().unwrap();
    let assets = session.fight_assets().unwrap();
    let [kage, sagat] = fight.machine().era_fighters();
    assert_eq!(fight.machine().arcade_opponent(), Some(TournamentFighter::Sagat));
    assert_eq!(sagat.fighter, Some(TournamentFighter::Sagat));
    assert_eq!(
        (kage.style.family, kage.style.revision, kage.style.loadout),
        (EraFamily::Sf5, Revision::Sf5Ce, Loadout::Skill1Trigger1),
    );
    assert_eq!((kage.sub_hp, kage.max_sub_hp), (888, 888));
    assert_eq!(sagat.style.revision, Revision::Sf5Ce);
    assert_evil_ryu_label(&session, &frame);
    let empty = Fill::of(0, u16::from(SUPER_STOCK));
    assert_eq!(fight.era_hud().p1, Some(EraGauge::two(
        GaugeRow::stock(3, 0, empty),
        GaugeRow::stock(2, 0, empty),
    )));

    let idle = PlayerInputs::default();
    let mut close = fight_with_scripted_sagat(fight);
    approach_scripted_sagat(&mut close, assets);
    let mut inactive = close.clone();
    let blocked_ashura = alpha_pad(&inactive, true, &[2, 5]);
    inactive.step(assets, blocked_ashura, idle, false);
    assert_ne!(inactive.fight_read().fighters[0].move_id, KAGE_ASHURA_FORWARD);
    assert_ne!(inactive.fight_read().fighters[0].move_id, KAGE_TRIGGER);

    let mut hold = close.clone();
    let (tap_tick, tap_damage) = kage_senha(&mut close, assets, false, Some("senha-tap"));
    let (hold_tick, hold_damage) = kage_senha(&mut hold, assets, true, Some("senha-hold"));
    assert!(hold_tick > tap_tick && hold_damage > tap_damage, "charging changes Senha's timing and hit");
    assert_eq!(kage_v_gauge(&close), (20, GaugeState::Normal), "a landed tap builds native V-Gauge");
    for _ in 0..6 {
        if kage_v_gauge(&close).1 == GaugeState::Full {
            break;
        }
        approach_scripted_sagat(&mut close, assets);
        kage_senha(&mut close, assets, false, None);
    }
    assert_eq!(kage_v_gauge(&close), (2 * u16::from(SUPER_STOCK), GaugeState::Full));
    approach_scripted_sagat(&mut close, assets);

    let mut trigger = idle;
    trigger.buttons[2] = true;
    trigger.buttons[5] = true;
    let super_before = super_meter(&close);
    let mut clip = AlphaClip::new(fight, "taigyaku-mudo", true);
    for _ in 0..16 {
        clip.push(&close.step(assets, idle, idle, false).frame);
    }
    clip.push(&close.step(assets, trigger, idle, false).frame);
    assert_eq!(close.fight_read().fighters[0].move_id, KAGE_TRIGGER);
    assert_eq!(kage_v_gauge(&close).1, GaugeState::Draining);
    assert_eq!(super_meter(&close), super_before, "V-Trigger leaves the EX stock gauge alone");
    for _ in 0..70 {
        clip.push(&close.step(assets, idle, idle, false).frame);
    }
    assert_eq!(kage_v_gauge(&close).1, GaugeState::Draining);
    assert_eq!(close.fight_read().fighters[0].state & 0x80, 0, "Kage recovers during the install");
    clip.finish();
    let mut enhanced = close.clone();
    let (_, trigger_damage) = kage_senha(&mut enhanced, assets, true, Some("senha-v-trigger"));
    assert!(trigger_damage > hold_damage, "Taigyaku Mudo strengthens charged Senha");
    kage_ashura(&close, assets, true, KAGE_ASHURA_FORWARD, "ashura-forward");
    kage_ashura(&close, assets, false, KAGE_ASHURA_BACK, "ashura-backward");
    kage_ashura(&close, assets, true, KAGE_ASHURA_AIR, "ashura-air");

    let rows = move_list_rows(&mut session.clone());
    for name in ["SENHA KASSATSU", "TAIGYAKU MUDO", "ASHURA SENKU"] {
        assert!(rows.iter().any(|row| row.starts_with(name)), "Kage's Moves includes {name}");
    }
}

fn kage_sekieiken(fight: &mut Fight, assets: &Assets, guard: bool) {
    let idle = PlayerInputs::default();
    approach_scripted_sagat(fight, assets);
    let range = if guard { 64 } else { 74 };
    assert!((0..35).any(|_| {
        let retreat = alpha_pad(fight, false, &[]);
        fight.step(assets, retreat, idle, false);
        center_x(fight, P1_STRUCT).abs_diff(center_x(fight, CPU_STRUCT)) >= range
    }), "Sekieiken starts outside point-blank range");
    let mut skill = idle;
    skill.buttons[1] = true;
    skill.buttons[4] = true;
    let mut clip = AlphaClip::new(fight, "sekieiken", !guard);
    for _ in 0..12 {
        clip.push(&fight.step(assets, idle, idle, false).frame);
    }
    let hp = fight.machine().era_fighters()[1].sub_hp;
    let (before, _) = kage_v_gauge(fight);
    let mut blocked = false;
    for tick in 0..75 {
        let p1 = if tick == 0 { skill } else { idle };
        let mut p2 = idle;
        if guard && tick >= 20 {
            let p1_x = center_x(fight, P1_STRUCT);
            let p2_x = center_x(fight, CPU_STRUCT);
            p2.directions[usize::from(p2_x < p1_x)] = true;
        }
        clip.push(&fight.step(assets, p1, p2, false).frame);
        let [kage, sagat] = fight.fight_read().fighters;
        if tick == 0 {
            assert_eq!(kage.move_id, KAGE_SEKIEIKEN, "MP+MK starts Sekieiken");
        }
        blocked |= sagat.blockstun;
    }
    if guard {
        assert!(blocked, "Sagat enters standing blockstun against Sekieiken; V {:?}, HP {} -> {}",
            kage_v_gauge(fight), hp, fight.machine().era_fighters()[1].sub_hp);
    } else {
        assert!(fight.machine().era_fighters()[1].sub_hp < hp, "Sekieiken hits Sagat");
    }
    let gain = if guard { 10 } else { 20 };
    assert_eq!(kage_v_gauge(fight).0, (before + gain).min(KAGE_II_FULL_V));
    clip.finish();
}

fn kage_misogi(fight: &Fight, assets: &Assets, chord: PlayerInputs, hit: bool) {
    let idle = PlayerInputs::default();
    let name = if hit { "misogi" } else { "misogi-whiff" };
    let mut attempt = fight.clone();
    let mut clip = AlphaClip::new(fight, name, true);
    for _ in 0..12 {
        clip.push(&attempt.step(assets, idle, idle, false).frame);
    }
    let gap = center_x(&attempt, P1_STRUCT).abs_diff(center_x(&attempt, CPU_STRUCT));
    let hp = attempt.machine().era_fighters()[1].sub_hp;
    let ex = super_meter(&attempt);
    let (timer, state) = kage_v_gauge(&attempt);
    assert_eq!(state, GaugeState::Draining);
    clip.push(&attempt.step(assets, chord, idle, false).frame);
    assert_eq!(attempt.fight_read().fighters[0].move_id, KAGE_MISOGI);
    let (remaining, state) = kage_v_gauge(&attempt);
    assert_eq!(state, GaugeState::Draining);
    assert!(timer.abs_diff(remaining + KAGE_II_FULL_V / 2) <= 1, "Misogi spends half of the full V-Timer");
    let mut above_sagat = false;
    for tick in 0..90 {
        let mut p2 = idle;
        if !hit && tick == 7 {
            p2.directions[3] = true;
        }
        clip.push(&attempt.step(assets, idle, p2, false).frame);
        above_sagat |= air_throw_height(&attempt, P1_STRUCT).1 >= 48;
    }
    clip.finish();
    assert!(above_sagat, "Misogi rises above the mat for the descending strike");
    assert_eq!(attempt.machine().era_fighters()[1].sub_hp < hp, hit, "{name} from {gap}px");
    if !hit {
        assert_eq!(super_meter(&attempt), ex, "a whiff only costs V-Timer");
    }
}

fn kage_confirm_ex_shoryuken(
    fight: &mut Fight,
    assets: &Assets,
    clip: &mut AlphaClip,
) {
    let idle = PlayerInputs::default();
    let ex = super_meter(fight);
    let sagat_hp = fight.machine().era_fighters()[1].sub_hp;
    let facing_right = fighter_object(fight, P1_STRUCT)[0x37] & 4 == 0;
    let input = MoveInput {
        motion: &[Dir::Forward, Dir::Down, Dir::DownForward],
        button: ButtonClass::PairPunch,
    };
    assert!(input.frames(facing_right, Strength::Light).into_iter().any(|pad| {
        clip.push(&fight.step(assets, pad, idle, false).frame);
        fight.fight_read().fighters[0].move_id == KAGE_EX_SHORYUKEN
    }), "the two-punch DP input starts Kage's EX Shoryuken");
    assert_eq!(super_meter(fight), ex - SUPER_STOCK);
    assert!((0..42).any(|_| {
        clip.push(&fight.step(assets, idle, idle, false).frame);
        let [kage, sagat] = fight.fight_read().fighters;
        kage.move_id == KAGE_EX_SHORYUKEN
            && air_throw_height(fight, P1_STRUCT).1 > 0
            && sagat.reacting()
            && air_throw_height(fight, CPU_STRUCT).1 > 0
            && fight.machine().era_fighters()[1].sub_hp < sagat_hp
    }), "EX Shoryuken lifts both fighters while Sagat is still in hitstun");
}

fn kage_airborne_misogi(fight: &Fight, assets: &Assets, chord: PlayerInputs, active: bool) {
    let idle = PlayerInputs::default();
    let mut attempt = fight_with_meter(fight, 2 * SUPER_STOCK);
    let mut clip = AlphaClip::new(fight, "misogi-air-cancel", active);
    for _ in 0..12 {
        clip.push(&attempt.step(assets, idle, idle, false).frame);
    }
    kage_confirm_ex_shoryuken(&mut attempt, assets, &mut clip);
    let ex = super_meter(&attempt);
    let (timer, state) = kage_v_gauge(&attempt);
    assert_eq!(state == GaugeState::Draining, active);
    let sagat_hp = attempt.machine().era_fighters()[1].sub_hp;
    clip.push(&attempt.step(assets, chord, idle, false).frame);
    if !active {
        assert_ne!(attempt.fight_read().fighters[0].move_id, KAGE_MISOGI);
        assert_eq!(kage_v_gauge(&attempt), (timer, state));
        return;
    }
    assert_eq!(attempt.fight_read().fighters[0].move_id, KAGE_MISOGI, "HP+HK cancels an airborne Shoryuken");
    assert!(air_throw_height(&attempt, P1_STRUCT).1 > 0, "Misogi starts above the ground");
    let (remaining, state) = kage_v_gauge(&attempt);
    assert_eq!(state, GaugeState::Draining);
    assert!(timer.abs_diff(remaining + KAGE_II_FULL_V / 2) <= 1, "the cancel spends half the full V-Timer");
    assert_eq!(super_meter(&attempt), ex, "Misogi does not spend a second EX stock");

    let mut airborne_hit = None;
    let mut combo_count = None;
    let mut flight = Vec::new();
    for tick in 0..82 {
        let sagat_airborne = air_throw_height(&attempt, CPU_STRUCT).1 > 0;
        let sagat_reacting = attempt.fight_read().fighters[1].reacting();
        clip.push(&attempt.step(assets, idle, idle, false).frame);
        let [kage, sagat] = attempt.fight_read().fighters;
        let hp = attempt.machine().era_fighters()[1].sub_hp;
        if kage.move_id == KAGE_MISOGI && hp < sagat_hp {
            let combo = attempt.fight_read().landed[0].map(|hit| hit.into_hitstun);
            airborne_hit.get_or_insert((sagat_airborne, sagat_reacting, combo));
            if let Some(count) = attempt.callouts().frame().sides[0].combo {
                combo_count = Some(count.hits);
            }
        }
        if tick < 40 {
            let x_gap = center_x(&attempt, P1_STRUCT).abs_diff(center_x(&attempt, CPU_STRUCT));
            flight.push((tick, kage.move_id, air_throw_height(&attempt, P1_STRUCT).1,
                sagat.move_id, air_throw_height(&attempt, CPU_STRUCT).1, x_gap, hp));
        }
    }
    clip.finish();
    let Some((true, true, combo)) = airborne_hit else {
        panic!("Misogi hits Sagat before he lands from Shoryuken. Contact: {airborne_hit:?}. Flight: {flight:?}");
    };
    assert_eq!(combo, Some(true), "the native combo state counts the juggle as a continuation");
    assert_eq!(combo_count, Some(2), "the HUD displays the two-hit juggle");
}

fn assert_kage_demon_refused(mut fight: Fight, assets: &Assets) {
    let meter = super_meter(&fight);
    for p1 in raging_demon_inputs(&fight) {
        fight.step(assets, p1, PlayerInputs::default(), false);
        assert_ne!(fight.fight_read().fighters[0].move_id, KAGE_DEMON);
        assert_ne!(fight.fight_read().fighters[0].move_id, EVIL_RYU_RAGING_DEMON);
    }
    assert_eq!(super_meter(&fight), meter, "an unavailable Shun Goku Satsu preserves EX");
}

#[test]
fn sfv_kage_uses_sekieiken_rikudo_osatsu_and_shun_goku_satsu_against_sagat() {
    let (session, frame) = boot_spike(EvilRyuPlaytest::KageII);
    let fight = session.fight().unwrap();
    let assets = session.fight_assets().unwrap();
    let [kage, sagat] = fight.machine().era_fighters();
    assert_eq!(fight.machine().arcade_opponent(), Some(TournamentFighter::Sagat));
    assert_eq!(sagat.fighter, Some(TournamentFighter::Sagat));
    assert_eq!(
        (kage.style.family, kage.style.revision, kage.style.loadout),
        (EraFamily::Sf5, Revision::Sf5Ce, Loadout::Skill2Trigger2),
    );
    assert_eq!((kage.sub_hp, kage.max_sub_hp), (888, 888));
    assert_evil_ryu_label(&session, &frame);
    assert_super_gauge(fight, &frame, 0);
    assert_eq!(kage_v_gauge(fight), (0, GaugeState::Normal));

    let idle = PlayerInputs::default();
    let mut trigger = idle;
    trigger.buttons[2] = true;
    trigger.buttons[5] = true;
    let mut close = fight_with_scripted_sagat(fight);
    assert_kage_demon_refused(fight_with_meter(&close, ALPHA_FULL_SUPER), assets);
    kage_sekieiken(&mut close, assets, false);
    for _ in 0..12 {
        kage_sekieiken(&mut close, assets, true);
    }
    assert_eq!(kage_v_gauge(&close), (140, GaugeState::Normal));
    let mut short = close.clone();
    short.step(assets, trigger, idle, false);
    assert_ne!(short.fight_read().fighters[0].move_id, KAGE_TRIGGER);
    assert_ne!(short.fight_read().fighters[0].move_id, KAGE_MISOGI);
    assert_eq!(kage_v_gauge(&short), (140, GaugeState::Normal));
    let mut inactive = close.clone();
    approach_scripted_sagat(&mut inactive, assets);
    kage_airborne_misogi(&inactive, assets, trigger, false);

    kage_sekieiken(&mut close, assets, true);
    assert_eq!(kage_v_gauge(&close), (KAGE_II_FULL_V, GaugeState::Full));
    approach_scripted_sagat(&mut close, assets);
    let mut clip = AlphaClip::new(fight, "rikudo-osatsu", true);
    for _ in 0..16 {
        clip.push(&close.step(assets, idle, idle, false).frame);
    }
    let ex = super_meter(&close);
    clip.push(&close.step(assets, trigger, idle, false).frame);
    assert_eq!(close.fight_read().fighters[0].move_id, KAGE_TRIGGER);
    assert_eq!(kage_v_gauge(&close), (KAGE_II_FULL_V, GaugeState::Draining));
    assert_eq!(super_meter(&close), ex, "Rikudo Osatsu spends only V");
    for _ in 0..66 {
        clip.push(&close.step(assets, idle, idle, false).frame);
    }
    assert_eq!(close.fight_read().fighters[0].state & 0x80, 0);
    clip.finish();

    kage_misogi(&close, assets, trigger, true);
    kage_airborne_misogi(&close, assets, trigger, true);
    let mut full = fight_with_meter(&close, ALPHA_FULL_SUPER);
    for _ in 0..16 {
        let retreat = alpha_pad(&full, false, &[]);
        full.step(assets, retreat, idle, false);
        if center_x(&full, P1_STRUCT).abs_diff(center_x(&full, CPU_STRUCT)) >= 74 {
            break;
        }
    }
    let (after, frames) = land_raging_demon(&full, assets).expect("Shun Goku Satsu catches nearby Sagat");
    check_raging_demon_recovery(after, assets, frames, false);

    let mut remote = close.clone();
    let mut largest_gap = 0;
    for _ in 0..110 {
        let back = alpha_pad(&remote, false, &[]);
        let away = alpha_pad(&remote, true, &[]);
        remote.step(assets, back, away, false);
        largest_gap = largest_gap.max(center_x(&remote, P1_STRUCT).abs_diff(center_x(&remote, CPU_STRUCT)));
        if largest_gap >= 184 {
            break;
        }
    }
    assert!(largest_gap >= 184, "Sagat backs off to {largest_gap}px");
    assert_kage_demon_refused(fight_with_meter(&remote, ALPHA_FULL_SUPER - 1), assets);

    let mut far_demon = fight_with_meter(&remote, ALPHA_FULL_SUPER);
    let Some(DemonOutcome::Whiff(mut frames)) = try_raging_demon(&mut far_demon, assets) else {
        panic!("full EX and active Rikudo Osatsu give a Demon that can run out of reach");
    };
    if let Some(dir) = alpha_clip_dir(EvilRyuPlaytest::KageII, "demon-whiff") {
        for _ in 0..25 {
            frames.push(far_demon.step(assets, idle, idle, false).frame);
        }
        write_clip(&dir, &frames);
    }

    kage_misogi(&remote, assets, trigger, false);

    let rows = move_list_rows(&mut session.clone());
    for name in ["SEKIEIKEN", "RIKUDO OSATSU", "MISOGI", "SHUN GOKU SATSU"] {
        assert!(rows.iter().any(|row| row.starts_with(name)), "Kage's Moves includes {name}");
    }
}

const SF6_DEMON: u8 = 0x20;
const SF6_OD_SHAKUNETSU: u8 = 0x22;
const SF6_ASHURA_FORWARD: u8 = 0x1c;
const SF6_ASHURA_BACK: u8 = 0x1e;
const SF6_OD_RYUSOKYAKU: u8 = 0x23;
const SF6_RUSH: u8 = 0x2c;
const SF6_PARRY: u8 = 0x2d;
const SF6_IMPACT: u8 = 0x2f;
const SF6_FULL_DRIVE: u16 = 6 * SUPER_STOCK as u16;
const SF6_DANGER_HP: u16 = EVIL_RYU_MAX_HP / 4;

fn sf6_drive(fight: &Fight) -> u16 {
    fight.machine().era_fighters()[0].sf6_drive_points().expect("native Drive")
}

fn check_sf6_impact_and_burnout(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let mut impact = idle;
    impact.buttons[2] = true;
    impact.buttons[5] = true;
    let mut close = fight.clone();
    approach_scripted_sagat(&mut close, assets);
    let mut clip = AlphaClip::new(fight, "drive-impact", true);
    for _ in 0..12 {
        clip.push(&close.step(assets, idle, idle, false).frame);
    }
    let sagat_hp = close.machine().era_fighters()[1].sub_hp;
    clip.push(&close.step(assets, impact, idle, false).frame);
    assert_eq!(close.fight_read().fighters[0].move_id, SF6_IMPACT);
    assert_eq!(sf6_drive(&close), SF6_FULL_DRIVE - u16::from(SUPER_STOCK));
    for _ in 0..58 {
        clip.push(&close.step(assets, idle, idle, false).frame);
    }
    assert!(close.machine().era_fighters()[1].sub_hp < sagat_hp, "Drive Impact lands against Sagat");
    clip.finish();

    let mut empty = fight.clone();
    let mut clip = AlphaClip::new(fight, "burnout", true);
    for stocks in (0..6).rev() {
        let output = empty.step(assets, impact, idle, false);
        if stocks == 0 {
            clip.push(&output.frame);
        }
        assert_eq!(empty.fight_read().fighters[0].move_id, SF6_IMPACT);
        assert_eq!(sf6_drive(&empty), stocks * u16::from(SUPER_STOCK));
        assert_eq!(output.announcements.contains(&AnnouncerLine::Burnout), stocks == 0);
        for _ in 0..40 {
            let output = empty.step(assets, idle, idle, false);
            if stocks <= 1 {
                clip.push(&output.frame);
            }
        }
    }
    assert!(matches!(empty.era_hud().p1.unwrap().main, GaugeRow::Stock {
        cells: 6, state: GaugeState::Greyed, ..
    }));
    let output = empty.step(assets, impact, idle, false);
    assert_ne!(empty.fight_read().fighters[0].move_id, SF6_IMPACT);
    clip.push(&output.frame);
    for _ in 0..26 {
        clip.push(&empty.step(assets, idle, idle, false).frame);
    }
    clip.finish();
    assert!((0..40).any(|_| {
        empty.step(assets, idle, idle, false);
        empty.fight_read().fighters[0].state & 0x80 == 0
    }), "Evil Ryu recovers from the fallback heavy normal");
    let escape = alpha_pad(&empty, false, &[3, 4, 5]);
    empty.step(assets, escape, idle, false);
    assert_eq!(empty.fight_read().fighters[0].move_id, SF6_ASHURA_BACK, "Ashura is available during Burnout");
    assert!(matches!(empty.era_hud().p1.unwrap().main, GaugeRow::Stock {
        cells: 6, state: GaugeState::Greyed, ..
    }));
}

fn check_sf6_parry_and_rush(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let mut parry = idle;
    parry.buttons[1] = true;
    parry.buttons[4] = true;
    let mut punch = idle;
    punch.buttons[1] = true;
    let mut close = fight.clone();
    approach_scripted_sagat(&mut close, assets);
    let mut clip = AlphaClip::new(fight, "perfect-parry", true);
    for _ in 0..12 {
        clip.push(&close.step(assets, idle, idle, false).frame);
    }
    let hp = close.machine().era_fighters()[0].sub_hp;
    clip.push(&close.step(assets, idle, punch, false).frame);
    let mut witnessed = false;
    for _ in 0..28 {
        let mut guarded = close.clone();
        let output = close.step(assets, idle, idle, false);
        if close.machine().era_fighters()[0].sub_hp == hp {
            clip.push(&output.frame);
            continue;
        }
        let output = guarded.step(assets, parry, idle, false);
        assert_eq!(guarded.fight_read().fighters[0].move_id, SF6_PARRY);
        assert!(output.announcements.contains(&AnnouncerLine::PerfectParry));
        assert_eq!(guarded.machine().era_fighters()[0].sub_hp, hp);
        assert_eq!(sf6_drive(&guarded), SF6_FULL_DRIVE);
        clip.push(&output.frame);
        for _ in 0..56 {
            clip.push(&guarded.step(assets, idle, idle, false).frame);
        }
        witnessed = true;
        break;
    }
    assert!(witnessed, "Sagat's actual standing punch reaches Evil Ryu");
    clip.finish();

    let mut rush = fight.clone();
    let mut clip = AlphaClip::new(fight, "drive-rush", true);
    for _ in 0..12 {
        clip.push(&rush.step(assets, idle, idle, false).frame);
    }
    let x = center_x(&rush, P1_STRUCT);
    let input = MoveInput {
        motion: &[Dir::Forward, Dir::Neutral, Dir::Forward],
        button: ButtonClass::MediumPair,
    };
    let facing_right = fighter_object(&rush, P1_STRUCT)[0x37] & 4 == 0;
    assert!(input.frames(facing_right, Strength::Medium).into_iter().any(|pad| {
        clip.push(&rush.step(assets, pad, idle, false).frame);
        rush.fight_read().fighters[0].move_id == SF6_RUSH
    }), "forward double tap and parry chord starts Drive Rush");
    assert_eq!(sf6_drive(&rush), SF6_FULL_DRIVE - u16::from(SUPER_STOCK));
    for _ in 0..45 {
        clip.push(&rush.step(assets, idle, idle, false).frame);
    }
    assert!(center_x(&rush, P1_STRUCT).abs_diff(x) >= 30, "Drive Rush advances at least 30 pixels");
    clip.finish();
}

fn check_sf6_overdrive(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let mut shot = fight.clone();
    assert!((0..48).any(|_| {
        let back = alpha_pad(&shot, false, &[]);
        shot.step(assets, back, idle, false);
        center_x(&shot, P1_STRUCT).abs_diff(center_x(&shot, CPU_STRUCT)) >= 116
    }), "stage a visible Shakunetsu projectile path");
    let mut clip = AlphaClip::new(fight, "overdrive-shakunetsu", true);
    for _ in 0..12 {
        clip.push(&shot.step(assets, idle, idle, false).frame);
    }
    let hp = shot.machine().era_fighters()[1].sub_hp;
    let ex = super_meter(&shot);
    let input = MoveInput {
        motion: &[Dir::Forward, Dir::DownForward, Dir::Down, Dir::DownBack, Dir::Back],
        button: ButtonClass::PairPunch,
    };
    let facing_right = fighter_object(&shot, P1_STRUCT)[0x37] & 4 == 0;
    assert!(input.frames(facing_right, Strength::Light).into_iter().any(|pad| {
        clip.push(&shot.step(assets, pad, idle, false).frame);
        shot.fight_read().fighters[0].move_id == SF6_OD_SHAKUNETSU
    }), "half-circle back and two punches starts OD Shakunetsu");
    assert_eq!(sf6_drive(&shot), SF6_FULL_DRIVE - 2 * u16::from(SUPER_STOCK));
    assert_eq!(super_meter(&shot), ex, "Overdrive spends Drive");
    let mut red = false;
    for _ in 0..98 {
        clip.push(&shot.step(assets, idle, idle, false).frame);
        red |= p1_projectile_palette(&shot, SF6_OD_SHAKUNETSU) == Some(14);
    }
    assert!(red, "OD Shakunetsu visibly uses the native red projectile");
    assert!(shot.machine().era_fighters()[1].sub_hp < hp, "OD Shakunetsu hits Sagat");
    clip.finish();
}

fn check_sf6_ashura(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let mut close = fight.clone();
    approach_scripted_sagat(&mut close, assets);
    for (forward, slot, name) in [
        (true, SF6_ASHURA_FORWARD, "ashura-forward"),
        (false, SF6_ASHURA_BACK, "ashura-backward"),
    ] {
        let mut glide = close.clone();
        let mut clip = AlphaClip::new(fight, name, true);
        for _ in 0..12 {
            clip.push(&glide.step(assets, idle, idle, false).frame);
        }
        let start_x = center_x(&glide, P1_STRUCT);
        let sagat_x = center_x(&glide, CPU_STRUCT);
        let health = glide.machine().era_fighters().map(|fighter| fighter.sub_hp);
        let meter = super_meter(&glide);
        let input = alpha_pad(&glide, forward, &[3, 4, 5]);
        clip.push(&glide.step(assets, input, idle, false).frame);
        assert_eq!(glide.fight_read().fighters[0].move_id, slot, "{name} starts on the directional kick chord");
        for _ in 0..52 {
            let object = fighter_object(&glide, P1_STRUCT);
            if object[0xa] == slot {
                assert_eq!(object[0xf] & 8, 0, "{name} stays hittable");
            }
            clip.push(&glide.step(assets, idle, idle, false).frame);
        }
        let finish_x = center_x(&glide, P1_STRUCT);
        let other_x = center_x(&glide, CPU_STRUCT);
        assert!(finish_x.abs_diff(start_x) >= 36, "{name} crosses a meaningful distance");
        assert_eq!(finish_x > start_x, forward == (sagat_x > start_x), "{name} follows the chosen direction");
        assert_eq!(finish_x.cmp(&other_x) != start_x.cmp(&sagat_x), forward, "forward Ashura crosses Sagat");
        assert_eq!(glide.machine().era_fighters().map(|fighter| fighter.sub_hp), health);
        assert_eq!(sf6_drive(&glide), SF6_FULL_DRIVE);
        assert_eq!(super_meter(&glide), meter);
        clip.finish();
    }

    let mut air = close.clone();
    let mut jump = idle;
    jump.directions[3] = true;
    air.step(assets, jump, idle, false);
    assert!((0..18).any(|_| {
        air.step(assets, idle, idle, false);
        air_throw_height(&air, P1_STRUCT).1 >= 24
    }), "a normal jump raises Evil Ryu before the Ashura check");
    let input = alpha_pad(&air, true, &[3, 4, 5]);
    air.step(assets, input, idle, false);
    assert_ne!(air.fight_read().fighters[0].move_id, SF6_ASHURA_FORWARD, "grounded Ashura is unavailable in a jump");

    let mut punch = idle;
    punch.buttons[1] = true;
    let mut interrupted = close;
    let hp = interrupted.machine().era_fighters()[0].sub_hp;
    let input = alpha_pad(&interrupted, true, &[3, 4, 5]);
    interrupted.step(assets, input, punch, false);
    assert_eq!(interrupted.fight_read().fighters[0].move_id, SF6_ASHURA_FORWARD);
    assert!((0..16).any(|_| {
        interrupted.step(assets, idle, idle, false);
        interrupted.machine().era_fighters()[0].sub_hp < hp
    }), "Sagat interrupts Ashura's startup with a real punch");
    assert!(interrupted.fight_read().fighters[0].reacting());
}

fn check_sf6_ryusokyaku(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    let mut close = fight.clone();
    approach_scripted_sagat(&mut close, assets);
    for (button, strength, slot, timing, name) in [
        (ButtonClass::Kick, Strength::Light, RYUSOKYAKU_MOVE, (17, 2, 22), "ryusokyaku-lk"),
        (ButtonClass::Kick, Strength::Medium, RYUSOKYAKU_MOVE, (20, 2, 20), "ryusokyaku-mk"),
        (ButtonClass::Kick, Strength::Heavy, RYUSOKYAKU_MOVE, (26, 2, 18), "ryusokyaku-hk"),
        (ButtonClass::PairKick, Strength::Light, SF6_OD_RYUSOKYAKU, (21, 2, 22), "overdrive-ryusokyaku"),
    ] {
        let input = MoveInput {
            motion: &[Dir::Back, Dir::DownBack, Dir::Down, Dir::DownForward, Dir::Forward],
            button,
        };
        let expected_drive = if button == ButtonClass::PairKick {
            SF6_FULL_DRIVE - 2 * u16::from(SUPER_STOCK)
        } else {
            SF6_FULL_DRIVE
        };

        let mut whiff = fight.clone();
        let facing_right = fighter_object(&whiff, P1_STRUCT)[0x37] & 4 == 0;
        let mut elapsed = 0;
        let mut first_active = None;
        let mut active = 0;
        for pad in input.frames(facing_right, strength).into_iter().chain(std::iter::repeat_n(idle, 65)) {
            whiff.step(assets, pad, idle, false);
            let ryu = whiff.fight_read().fighters[0];
            if ryu.move_id == slot && ryu.state & 0x80 != 0 {
                elapsed += 1;
                if ryu.attack_active {
                    first_active.get_or_insert(elapsed);
                    active += 1;
                }
            } else if elapsed != 0 {
                break;
            }
        }
        assert_eq!(whiff.machine().era_fighters()[1].sub_hp, fight.machine().era_fighters()[1].sub_hp);
        let startup = first_active.unwrap_or_else(|| panic!("{name} exposes an active hitbox"));
        assert_eq!((startup, active, elapsed - startup + 1 - active), timing, "{name} startup, active and recovery");

        let mut axe = close.clone();
        let mut clip = AlphaClip::new(fight, name, true);
        let facing_right = fighter_object(&axe, P1_STRUCT)[0x37] & 4 == 0;
        for _ in 0..10 {
            clip.push(&axe.step(assets, idle, idle, false).frame);
        }
        let hp = axe.machine().era_fighters()[1].sub_hp;
        let mut started = false;
        for pad in input.frames(facing_right, strength).into_iter().chain(std::iter::repeat_n(idle, 65)) {
            clip.push(&axe.step(assets, pad, idle, false).frame);
            if !started && axe.fight_read().fighters[0].move_id == slot {
                assert_eq!(sf6_drive(&axe), expected_drive, "{name} Drive cost");
                started = true;
            }
        }
        assert!(started && axe.machine().era_fighters()[1].sub_hp < hp, "{name} hits Sagat");
        clip.finish();
    }
}

fn sf6_demon_fixture(fight: &Fight, hp: u16, super_points: u8) -> Fight {
    // This test presets only the CA eligibility inputs. The real command,
    // Drive, hit, whiff and wake-up remain native in the private Arcade fight.
    let mut attempt = fight.clone();
    let mut snapshot = attempt.save_netplay_snapshot(0);
    let fighter = &mut snapshot.machine.toggle_state.era.fighters[0];
    fighter.sub_hp = hp;
    fighter.super_meter = super_points;
    attempt.restore_netplay_snapshot(&snapshot);
    attempt
}

fn check_sf6_critical_art(fight: &Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    for (hp, meter) in [
        (SF6_DANGER_HP + 1, ALPHA_FULL_SUPER),
        (SF6_DANGER_HP, ALPHA_FULL_SUPER - 1),
    ] {
        let mut refused = sf6_demon_fixture(fight, hp, meter);
        for p1 in raging_demon_inputs(&refused).into_iter().chain(std::iter::repeat_n(idle, 10)) {
            refused.step(assets, p1, idle, false);
            assert_ne!(refused.fight_read().fighters[0].move_id, SF6_DEMON, "HP {hp}, Super {meter}");
        }
        assert_eq!(refused.machine().era_fighters()[0].sub_hp, hp);
        assert_eq!(super_meter(&refused), meter);
        assert_eq!(sf6_drive(&refused), SF6_FULL_DRIVE);
    }

    let mut far = fight.clone();
    assert!((0..120).any(|_| {
        let p1 = alpha_pad(&far, false, &[]);
        let p2 = alpha_pad(&far, true, &[]);
        far.step(assets, p1, p2, false);
        center_x(&far, P1_STRUCT).abs_diff(center_x(&far, CPU_STRUCT)) >= 184
    }), "Sagat moves outside Demon range");
    let mut far = sf6_demon_fixture(&far, SF6_DANGER_HP, ALPHA_FULL_SUPER);
    let Some(DemonOutcome::Whiff(mut frames)) = try_raging_demon(&mut far, assets) else {
        panic!("full Super at exactly 25% life starts a Demon that can whiff");
    };
    assert_eq!(super_meter(&far), 0);
    assert_eq!(sf6_drive(&far), SF6_FULL_DRIVE);
    if let Some(dir) = alpha_clip_dir(EvilRyuPlaytest::Sf6, "demon-whiff") {
        for _ in 0..25 {
            frames.push(far.step(assets, idle, idle, false).frame);
        }
        write_clip(&dir, &frames);
    }

    let full = sf6_demon_fixture(fight, SF6_DANGER_HP, ALPHA_FULL_SUPER);
    let (after, frames) = land_raging_demon(&full, assets).expect("nearby Sagat can be caught by Critical Art");
    assert_eq!(super_meter(&after), 0, "the slam leaves the entire Super Art gauge spent");
    assert_eq!(sf6_drive(&after), SF6_FULL_DRIVE);
    check_raging_demon_recovery(after, assets, frames, false);
}

#[test]
fn sf6_evil_ryu_uses_drive_and_critical_art_against_sagat() {
    let (session, frame) = boot_spike(EvilRyuPlaytest::Sf6);
    let fight = session.fight().unwrap();
    let assets = session.fight_assets().unwrap();
    let [evil_ryu, sagat] = fight.machine().era_fighters();
    assert_eq!(fight.machine().arcade_opponent(), Some(TournamentFighter::Sagat));
    assert_eq!(sagat.fighter, Some(TournamentFighter::Sagat));
    for fighter in [evil_ryu, sagat] {
        assert_eq!(
            (fighter.style.family, fighter.style.revision, fighter.style.loadout),
            (EraFamily::Sf6, Revision::Sf6, Loadout::Standard),
        );
        assert_eq!(fighter.sf6_drive_points(), Some(SF6_FULL_DRIVE));
    }
    assert_eq!((evil_ryu.sub_hp, evil_ryu.max_sub_hp), (EVIL_RYU_MAX_HP, EVIL_RYU_MAX_HP));
    assert_evil_ryu_label(&session, &frame);
    assert_super_gauge(fight, &frame, 0);
    assert_eq!(fight.era_hud().p1.unwrap().main, GaugeRow::stock(6, 6, Fill::EMPTY));

    let scripted = fight_with_scripted_sagat(fight);
    check_sf6_impact_and_burnout(&scripted, assets);
    check_sf6_parry_and_rush(&scripted, assets);
    check_sf6_overdrive(&scripted, assets);
    check_sf6_ashura(&scripted, assets);
    check_sf6_ryusokyaku(&scripted, assets);
    check_sf6_critical_art(&scripted, assets);

    let rows = move_list_rows(&mut session.clone());
    for name in ["DRIVE IMPACT", "DRIVE PARRY", "DRIVE RUSH", "OD SHAKUNETSU HADOKEN",
        "SHINKU HADOKEN", "SHIN HASHOGEKI", "METSU SHORYUKEN", "SHUN GOKU SATSU",
        "ASHURA SENKU FORWARD", "ASHURA SENKU BACKWARD", "RYUSOKYAKU", "OD RYUSOKYAKU"] {
        assert!(rows.iter().any(|row| row.starts_with(name)), "SF6 Evil Ryu's Moves includes {name}");
    }
}

const ARCADE_STAGE: usize = (0xffc003 - WORK_RAM) as usize;
const ARCADE_MATCH: usize = (0xffc004 - WORK_RAM) as usize;
const ARCADE_ROUND: usize = (0xffc00a - WORK_RAM) as usize;

fn awakening_arcade(
    assets: &Assets,
    data: &Arc<DataBank>,
    backgrounds: &Arc<BackgroundBank>,
    fighter: TournamentFighter,
    preset: RulesetPreset,
    match_in_thailand: u8,
) -> Fight {
    let mut config = MatchConfig::from_preset(preset);
    config.roster.mode = PlayMode::Arcade;
    config.roster.p1 = fighter;
    config.p2_fair_cpu = fighter == TournamentFighter::Sagat;
    let mut fight = Fight::start_banked(
        &assets.maincpu,
        FightSetup {
            kind: FightKind::Arcade,
            players: Players::One,
            config,
            dsw1: FightDips::DSW1,
            dsw2: FightDips::default().dsw2(),
        },
        Some(0),
        Some(backgrounds.clone()),
        Some(data.clone()),
    ).expect("normal Arcade country select reaches a stage");
    assert_eq!(flow_position(fight.machine()), STAGE_START);
    let mut snapshot = fight.save_netplay_snapshot(0);
    // Skip the first eight ladder matches, then let native intro and result
    // tasks run every round of the selected Thailand matchup.
    snapshot.machine.work[ARCADE_STAGE] = 4;
    snapshot.machine.work[ARCADE_MATCH] = match_in_thailand;
    fight.restore_netplay_snapshot(&snapshot);
    advance_to_arcade_round(&mut fight, assets, 0);
    fight
}

fn advance_to_arcade_round(fight: &mut Fight, assets: &Assets, round: u8) -> Frame {
    let idle = PlayerInputs::default();
    for _ in 0..1800 {
        let frame = fight.step(assets, idle, idle, false).frame;
        let machine = fight.machine();
        if flow_position(machine) == FIGHT_LOOP && machine.work_ram()[ARCADE_ROUND] == round {
            assert!(machine.unrecovered().is_empty());
            return frame;
        }
    }
    panic!("Arcade did not reach round {}: {:?}", round + 1, fight.machine().task_positions());
}

fn finish_arcade_round_on_time(fight: &mut Fight, player_wins: bool) {
    let loser = if player_wins { CPU_STRUCT } else { P1_STRUCT };
    let side = usize::from(player_wins);
    let mut snapshot = fight.save_netplay_snapshot(0);
    let time = (TIME_DIGITS - WORK_RAM) as usize;
    let life = (loser + 6 - WORK_RAM) as usize;
    snapshot.machine.work[time..time + 3].fill(0);
    snapshot.machine.work[life..life + 2].copy_from_slice(&[1, 0]);
    snapshot.machine.toggle_state.era.fighters[side].sub_hp = 20;
    fight.restore_netplay_snapshot(&snapshot);
}

fn arcade_round_wins(fight: &Fight) -> [u8; 2] {
    [P1_STRUCT, CPU_STRUCT].map(|ps| fight.machine().work_ram()[(ps + 8 - WORK_RAM) as usize])
}

fn awakened_ashura_starts(fight: &Fight, assets: &Assets) -> bool {
    let idle = PlayerInputs::default();
    let mut attempt = fight.clone();
    let input = MoveInput {
        motion: &[Dir::Forward, Dir::Down, Dir::DownForward],
        button: ButtonClass::Kick,
    };
    let facing_right = fighter_object(&attempt, P1_STRUCT)[0x37] & 4 == 0;
    input.frames(facing_right, Strength::Light).into_iter().any(|pad| {
        attempt.step(assets, pad, idle, false);
        attempt.fight_read().fighters[0].move_id == EVIL_RYU_ASHURA_FORWARD
    })
}

#[test]
fn arcade_thailand_decider_awakens_only_the_ryu_side() {
    use TournamentFighter::{Adon, Ken, Ryu, Sagat};

    let (rom, art) = spike_assets();
    let data = Arc::new(DataBank::build(&rom.maincpu, &art.catalogues));
    let mut backgrounds = BackgroundBank::new();
    backgrounds.add_vs_screens(&rom).unwrap();
    backgrounds.add_ending_screens(&rom).unwrap();
    let backgrounds = Arc::new(backgrounds);
    let assets = rom.with_background_bank(&backgrounds).with_sprite_bank(&art.bank);
    let idle = PlayerInputs::default();
    for (fighter, opponent, match_in_thailand, preset, expected) in [
        (Ryu, Sagat, 1, RulesetPreset::DefinitiveOverhaul, Some(AwakeningSide::Player)),
        (Sagat, Ryu, 1, RulesetPreset::DefinitiveOverhaul, Some(AwakeningSide::Cpu)),
        (Ken, Sagat, 1, RulesetPreset::DefinitiveOverhaul, None),
        (Ryu, Adon, 0, RulesetPreset::DefinitiveOverhaul, None),
        (Ryu, Sagat, 1, RulesetPreset::Faithful1987, None),
    ] {
        let mut fight = awakening_arcade(&assets, &data, &backgrounds, fighter, preset, match_in_thailand);
        assert_eq!(fight.machine().arcade_opponent(), Some(opponent));
        assert_eq!(fight.player_fighter(0), fighter);
        assert!(fight.machine().satsui_awakening().is_none());
        assert_eq!(arcade_round_wins(&fight), [0, 0]);
        if expected == Some(AwakeningSide::Player) {
            assert_fighter_label(&fight, &assets, &fight.render(&assets), 5, b"RYU");
            assert!(!awakened_ashura_starts(&fight, &assets));
        }
        finish_arcade_round_on_time(&mut fight, true);
        let frame = advance_to_arcade_round(&mut fight, &assets, 1);
        assert!(fight.machine().satsui_awakening().is_none());
        assert_eq!(arcade_round_wins(&fight), [1, 0]);
        if expected == Some(AwakeningSide::Player) {
            assert_fighter_label(&fight, &assets, &frame, 5, b"RYU");
            assert!(!awakened_ashura_starts(&fight, &assets));
            let mut straight_win = fight.clone();
            finish_arcade_round_on_time(&mut straight_win, true);
            for _ in 0..600 {
                straight_win.step(&assets, idle, idle, false);
                assert!(straight_win.machine().satsui_awakening().is_none());
            }
            assert_ne!(straight_win.machine().work_ram()[ARCADE_ROUND], 2, "a 2–0 win never starts the decider");
        }
        finish_arcade_round_on_time(&mut fight, false);
        let before = fight.save_netplay_snapshot(0);
        let frame = advance_to_arcade_round(&mut fight, &assets, 2);
        assert_eq!(arcade_round_wins(&fight), [1, 1]);
        assert_eq!(fight.player_fighter(0), fighter, "the original roster pick stays selected");
        assert_eq!(fight.machine().satsui_awakening().map(|awakening| awakening.side), expected,
            "{fighter:?} versus {opponent:?} with {preset:?}");

        if let Some(side) = expected {
            let awakening = fight.machine().satsui_awakening().unwrap();
            assert_eq!(awakening.profile, EvilRyuPlaytest::UltraSf2);
            let (index, row) = if side == AwakeningSide::Player { (0, 5) } else { (1, 7) };
            let [p1, p2] = fight.machine().era_fighters();
            assert_eq!([p1, p2][index].style.revision, Revision::Sf1Plus);
            assert_eq!([p1, p2][index].max_sub_hp, EVIL_RYU_MAX_HP);
            assert_eq!([p1, p2][index].sub_hp, EVIL_RYU_MAX_HP);
            assert_fighter_label(&fight, &assets, &frame, row, b"EVIL RYU");
            assert_eq!(awakened_ashura_starts(&fight, &assets), side == AwakeningSide::Player);
            let checksum = fight.state_checksum();
            fight.restore_netplay_snapshot(&before);
            advance_to_arcade_round(&mut fight, &assets, 2);
            assert_eq!(fight.state_checksum(), checksum, "rollback repeats the same awakening");
            if side == AwakeningSide::Cpu {
                let mut jab = idle;
                jab.buttons[0] = true;
                fight.step(&assets, jab, idle, false);
                assert!(fight.fight_read().fighters[0].attacking(), "Sagat still answers the human pad");
                assert!((0..300).any(|_| {
                    fight.step(&assets, idle, idle, false);
                    fight.fight_read().fighters[1].attacking()
                }), "the awakened CPU opponent still attacks without a human pad");
            }
        }
    }
}
