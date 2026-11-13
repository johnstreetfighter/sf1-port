//! Capture README stills from the headless game session, exactly as
//! `sf-game` draws them: the Main Menu, Settings, the portrait select and a
//! Local fight with the art pack and the background bank loaded. Each file
//! is a 384×224 frame.
//!
//! `cargo run -p sf-host --example readme_shots -- <out folder> [fight frames folder]`

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession};
use sf_host::log::Log;
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::romset::load_assets;

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
const RIGHT: usize = 0;
const DOWN: usize = 2;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = Path::new(args.get(1).map(String::as_str).unwrap_or("docs/media")).to_path_buf();
    let fight_frames = args.get(2).map(|p| Path::new(p).to_path_buf());
    std::fs::create_dir_all(&out).unwrap();
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let mut log = Log::create(&std::env::temp_dir().join("readme_shots.log"));
    let art = sf_host::art::load_with_fallbacks(&assets, &[Path::new(ROMS)], &[Path::new("assets").to_path_buf()], &mut log).expect("the art pack loads");

    let stages = sf_host::stages::load(&assets, &mut log).expect("the background bank builds");

    let fresh = || {
        let assets = load_assets(ROMS, MANIFEST).unwrap();
        GameSession::new(assets, MatchConfig::default()).with_sprite_bank(&art.bank).with_background_bank(stages.clone())
    };

    // Main Menu, then Settings.
    let mut s = fresh();
    for _ in 0..200 {
        s.step(idle());
    }
    let menu = tap(&mut s, with(|i| i.menu.start = true));
    assert_eq!(menu.screen, "main menu");
    write_png(&out.join("main-menu.png"), &settle(&mut s, menu).frame);
    for _ in 0..3 {
        tap(&mut s, stick(DOWN));
    }
    let settings = tap(&mut s, confirm());
    assert_eq!(settings.screen, "settings");
    write_png(&out.join("settings.png"), &settle(&mut s, settings).frame);

    // Local: P1 on Sagat (the last tile), P2 from Ken over to Geki.
    let mut s = fresh();
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, stick(DOWN));
    tap(&mut s, confirm());
    assert_eq!(tap(&mut s, confirm()).screen, "select");
    for d in [DOWN, RIGHT, RIGHT, RIGHT, RIGHT, RIGHT] {
        tap(&mut s, stick(d));
    }
    tap(&mut s, confirm());
    tap(&mut s, confirm()); // P1's default style
    tap(&mut s, with(|i| i.p2.directions[RIGHT] = true));
    let select = tap(&mut s, with(|i| i.p2.directions[RIGHT] = true));
    write_png(&out.join("player-select.png"), &settle(&mut s, select).frame);
    tap(&mut s, with(|i| i.p2.buttons[0] = true));
    s.step(with(|i| i.p2.buttons[0] = true)); // P2's default style
    s.step(idle());
    until(&mut s, "stage select", 60);
    tap(&mut s, confirm());
    until(&mut s, "fight", 60);
    if let Some(dir) = fight_frames {
        std::fs::create_dir_all(&dir).unwrap();
        for f in 0..3_000u32 {
            let o = s.step(fight_script(f));
            if o.screen != "fight" {
                break;
            }
            if f % 20 == 0 {
                write_png(&dir.join(format!("fight-{f:05}.png")), &o.frame);
            }
        }
    }
}

/// Both walk in, then trade buttons: P1 cycles all six, P2 a slower jab and kick.
fn fight_script(f: u32) -> FrameInputs {
    with(|i| {
        let phase = f % 150;
        i.p1.directions[RIGHT] = phase < 30;
        i.p2.directions[1] = phase < 25;
        if phase >= 40 && phase % 12 < 3 {
            i.p1.buttons[((f / 12) % 6) as usize] = true;
        }
        if phase >= 60 && phase % 30 < 3 {
            i.p2.buttons[if (f / 30) % 2 == 0 { 2 } else { 5 }] = true;
        }
    })
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

/// A few idle frames so any fade or cursor blink settles.
fn settle(s: &mut GameSession, mut out: FrameOutput) -> FrameOutput {
    for _ in 0..20 {
        out = s.step(idle());
    }
    out
}

fn until(s: &mut GameSession, name: &str, limit: u32) {
    for _ in 0..limit {
        if s.step(idle()).screen == name {
            return;
        }
    }
    panic!("never reached {name} in {limit} frames (at {})", s.screen());
}

fn write_png(path: &Path, frame: &Frame) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), WIDTH as u32, HEIGHT as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let rgb: Vec<u8> = frame.argb.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8]).collect();
    encoder.write_header().unwrap().write_image_data(&rgb).unwrap();
}
