//! Capture the README's era HUD stills from the headless game session, as
//! `sf-game` draws them: a Training fight (Ryu against the dummy Ken) with
//! the art pack loaded. Real fights carry no era state until project 4, so
//! the gauges (Ryu in SF6, Ken in Alpha), the call-outs and the combo count
//! are staged through the session's test hooks; everything else, from the
//! moved TIME to the drawing, is the game's own.
//!
//! `cargo run -p sf-host --release --example era_hud_shots -- <out folder>`

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight_reader::{FightRead, FighterRead, LandedHit};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::OverlayMode;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession};
use sf_host::log::Log;
use sf_video::callouts::{CalloutEvent, CalloutKind, Side};
use sf_video::era_hud::{EraGauge, EraHud, Fill, GaugeArt, GaugeRow};
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::romset::load_assets;

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const DOWN: usize = 2;
/// The bottom rows the gauge-art strip keeps of each frame.
const STRIP: usize = 40;

const STANDING: FighterRead =
    FighterRead { present: true, health: 48, move_id: 0x01, state: 0x12, blockstun: false, getting_up: false, attack_active: false };
const STUNNED: FighterRead =
    FighterRead { present: true, health: 44, move_id: 0x30, state: 0x8a, blockstun: false, getting_up: false, attack_active: false };

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = Path::new(args.get(1).map(String::as_str).unwrap_or("docs/media")).to_path_buf();
    std::fs::create_dir_all(&out).unwrap();
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let mut log = Log::create(&std::env::temp_dir().join("era_hud_shots.log"));
    let art = sf_host::art::load(&assets, &[Path::new(ROMS)], &mut log).expect("the art pack loads");
    let gauges = EraHud {
        // Ryu in SF6: four and a half Drive bars, Super Art level 1 filling.
        p1: Some(EraGauge::two(GaugeRow::stock(6, 4, Fill::of(1, 2)), GaugeRow::level(1, 3, Fill::of(1, 3)))),
        // Ken in Alpha: the gauge at level 2.
        p2: Some(EraGauge::one(GaugeRow::level(2, 3, Fill::of(2, 3)))),
    };

    let mut s = training(load_assets(ROMS, MANIFEST).unwrap(), &art.bank);
    s.set_era_hud(gauges);
    // Ryu walks in and throws a few buttons so the shots catch a fight.
    for f in 0..120u32 {
        s.step(with(|i| {
            i.p1.directions[RIGHT] = f < 40;
            if f >= 50 && f % 14 < 3 {
                i.p1.buttons[((f / 14) % 3) as usize] = true;
            }
        }));
    }

    // A three-hit combo that opened with a counter hit.
    s.feed_fight_reads(&combo_reads(3));
    s.push_callouts(&[CalloutEvent::new(Side::P1, CalloutKind::Counter)]);
    let combo = steps(&mut s, 16);
    write_png(&out.join("era-hud-fight.png"), &combo.frame);
    steps(&mut s, 160);

    // Ryu's perfect parry while Ken's super dims the stage.
    s.push_callouts(&[
        CalloutEvent::new(Side::P1, CalloutKind::PerfectParry),
        CalloutEvent::new(Side::P2, CalloutKind::Super { name: "Shoryureppa" }),
    ]);
    let sup = steps(&mut s, 6);
    write_png(&out.join("era-hud-super.png"), &sup.frame);
    steps(&mut s, 160);

    // The same gauges in Ruled, Blocks and Plate: the bottom of each frame.
    let mut strips = Vec::new();
    for art in GaugeArt::ALL {
        while s.fight().unwrap().gauge_art() != art {
            cycle_gauge_art(&mut s);
        }
        strips.push(steps(&mut s, 4).frame);
    }
    write_strips(&out.join("era-hud-gauge-art.png"), &strips);

    // Training: the frame meter on, the gauges lifted above it, the combo's damage.
    set_overlay(&mut s, OverlayMode::FrameMeter);
    s.feed_fight_reads(&combo_reads(4));
    let meter = steps(&mut s, 22);
    write_png(&out.join("era-hud-training.png"), &meter.frame);
}

/// Reads of a combo of `hits` by P1: the first hit lands clean and each
/// next one lands while Ken is still in hitstun.
fn combo_reads(hits: usize) -> Vec<FightRead> {
    let mut reads = Vec::new();
    for h in 0..hits {
        let mut read = FightRead::default();
        read.fighters = [STANDING, STUNNED];
        read.landed[0] = Some(LandedHit { damage: 3, into_hitstun: h > 0, chip: false });
        reads.push(read);
        for _ in 0..4 {
            reads.push(FightRead { fighters: [STANDING, STUNNED], ..FightRead::default() });
        }
    }
    reads
}

fn training(a: sf_video::romset::Assets, bank: &sf_video::sprite_bank::SpriteBank) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul)).with_sprite_bank(bank);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, confirm()); // Single Player
    tap(&mut s, stick(DOWN));
    tap(&mut s, confirm()); // Training
    tap(&mut s, stick(DOWN));
    tap(&mut s, confirm()); // DUMMY
    assert_eq!(s.screen(), "select");
    tap(&mut s, confirm()); // P1 Ryu
    s.step(confirm()); // the dummy Ken
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    s
}

/// The pause box's GAUGE ART row (the fourth, under LOOK): Right moves it to
/// the next art.
fn cycle_gauge_art(s: &mut GameSession) {
    tap(s, with(|i| i.menu.start = true));
    tap(s, stick(DOWN));
    tap(s, stick(DOWN));
    tap(s, stick(DOWN));
    tap(s, stick(RIGHT));
    tap(s, with(|i| i.menu.start = true));
}

/// The pause box's OVERLAYS row cycled to `mode`.
fn set_overlay(s: &mut GameSession, mode: OverlayMode) {
    tap(s, with(|i| i.menu.start = true));
    tap(s, stick(DOWN));
    while s.fight().unwrap().overlay() != mode {
        tap(s, confirm());
    }
    tap(s, with(|i| i.menu.start = true));
}

fn steps(s: &mut GameSession, n: usize) -> FrameOutput {
    let mut out = s.step(idle());
    for _ in 1..n {
        out = s.step(idle());
    }
    out
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn stick(dir: usize) -> FrameInputs {
    with(|i| i.p1.directions[dir] = true)
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn tap(s: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let out = s.step(input);
    s.step(idle());
    out
}

fn write_png(path: &Path, frame: &Frame) {
    write_rgb(path, HEIGHT, frame.argb.iter().copied());
}

/// The bottom [`STRIP`] rows of each frame, one above the next.
fn write_strips(path: &Path, frames: &[Frame]) {
    let rows = frames.iter().flat_map(|f| f.argb[(HEIGHT - STRIP) * WIDTH..].iter().copied());
    write_rgb(path, STRIP * frames.len(), rows);
}

fn write_rgb(path: &Path, height: usize, argb: impl Iterator<Item = u32>) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), WIDTH as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let rgb: Vec<u8> = argb.flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, p as u8]).collect();
    encoder.write_header().unwrap().write_image_data(&rgb).unwrap();
}
