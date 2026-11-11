//! Capture the key art's sprite guide pieces (ticket #234, `KA1`) from real
//! headless Local fights, at the game's own pixels: each tournament
//! fighter in his fighting stance, Ryu's Hadoken, Ken's Shoryuken, and each
//! 1987 stage's backdrop with no fighters. `tools/key_art/guide.py` lays
//! them out. The fights run under Definitive Overhaul (the full cast, the
//! stage select) with the art pack and the background bank loaded, so the
//! fighters with approved art wear it.
//!
//! The fighters are the frame's sprite layer alone and the backdrops its
//! background and foreground layers alone: the fight's own scene, drawn
//! with the other layers switched off. `figures.json` records where each
//! figure stood in its frame (384×224, the floor at about row 214), so the
//! guide keeps Ken's lift and the fireball's distance from Ryu's hands.
//! Everything written is ROM-derived:
//! the default folder is local only, and nothing here is ever committed.
//!
//! `cargo run -p sf-host --release --example key_art_guide -- [out folder]`

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::roster::TournamentFighter;
use sf_host::log::Log;
use sf_video::background_bank::BackgroundBank;
use sf_video::render::{render_frame, render_frame_staged, TextEdits, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};
use sf_video::scene::SceneState;
use sf_video::sprite_bank::SpriteBank;

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
const OUT: &str = "tools/reference/local/key-art/captures";
/// The fight loop (`0x3eafa`).
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
/// The scene's layer switches (`gfxctrl`): bg, fg, sprites.
const BG_FG: u8 = 0x60;
const SPRITES: u8 = 0x80;
/// Idle frames after the fight loop starts before the stances are taken,
/// and the move ids of the two specials (`fight_reader`).
const SETTLE: usize = 40;
const HADOKEN: u8 = 0x1a;
const SHORYUKEN: u8 = 0x19;
/// Sprite columns further apart than this are separate figures.
const GAP: usize = 4;

/// The stage screen lists the ten 1987 stages in this order.
const STAGE_OWNERS: [&str; 10] = ["retsu", "geki", "joe", "mike", "lee", "gen", "birdie", "eagle", "adon", "sagat"];

/// The country pairs, each fought on the first one's stage (P1 left).
const PAIRS: [(TournamentFighter, TournamentFighter, usize); 5] = [
    (TournamentFighter::Retsu, TournamentFighter::Geki, 0),
    (TournamentFighter::Joe, TournamentFighter::Mike, 2),
    (TournamentFighter::Lee, TournamentFighter::Gen, 4),
    (TournamentFighter::Birdie, TournamentFighter::Eagle, 6),
    (TournamentFighter::Adon, TournamentFighter::Sagat, 8),
];

/// What the captures draw with: the ROM's graphics plus the art pack's
/// sprites and the background bank, as an overhaul fight draws.
struct Kit {
    art: SpriteBank,
    stages: BackgroundBank,
    draw: Assets,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = Path::new(args.get(1).map(String::as_str).unwrap_or(OUT)).to_path_buf();
    std::fs::create_dir_all(out.join("sequences")).unwrap();
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let mut log = Log::create(&std::env::temp_dir().join("key_art_guide.log"));
    let art = sf_host::art::load(&assets, &[Path::new(ROMS)], &mut log).expect("the art pack loads");
    let stages = sf_host::stages::load(&assets, &mut log).expect("the background bank builds");
    let draw = assets.with_sprite_bank(&art.bank).with_background_bank(&stages);
    let kit = Kit { art: art.bank.clone(), stages, draw };
    let mut placed = Vec::new();
    for (fighter, why) in &art.kept {
        println!("note: {fighter} keeps his ROM art: {why}");
    }

    // The pairs: both stances, and the stage behind them.
    let mut fought = [false; 10];
    for (p1, p2, stage) in PAIRS {
        let mut s = fight(&kit, p1, p2, stage);
        let scene = scene_of(&s);
        let stood = figures(&kit.draw, &scene);
        assert_eq!(stood.len(), 2, "{p1:?} and {p2:?} stand apart: {} figures", stood.len());
        write_figure(&mut placed, &out.join(format!("{}-stance.png", name(p1))), &stood[0]);
        write_figure(&mut placed, &out.join(format!("{}-stance.png", name(p2))), &stood[1]);
        write_backdrop(&out, &kit.draw, &scene, stage);
        write_frame(&out.join(format!("fight-{}.png", STAGE_OWNERS[stage])), &kit.draw, &scene);
        fought[stage] = true;
        s.step(idle());
    }

    // Ryu and Ken: their stances, then Ryu's Hadoken on Geki's stage.
    let mut s = fight(&kit, TournamentFighter::Ryu, TournamentFighter::Ken, 1);
    let scene = scene_of(&s);
    let stood = figures(&kit.draw, &scene);
    assert_eq!(stood.len(), 2, "Ryu and Ken stand apart");
    write_figure(&mut placed, &out.join("ryu-stance.png"), &stood[0]);
    write_figure(&mut placed, &out.join("ken-stance.png"), &stood[1]);
    write_backdrop(&out, &kit.draw, &scene, 1);
    fought[1] = true;
    let moves = special(&mut s, HADOKEN, &[&[DOWN], &[DOWN, RIGHT]], &[RIGHT], 0);
    write_sequence(&out, "ryu-hadoken", &kit.draw, &moves);
    // The release: the first frame the fireball has left his hand.
    let (at, pieces) = moves
        .iter()
        .enumerate()
        .map(|(i, m)| (i, figures(&kit.draw, m)))
        .find(|(_, f)| f.len() >= 3)
        .expect("the Hadoken's fireball leaves Ryu's hand");
    println!("ryu-hadoken: frame {at} of the move");
    write_figure(&mut placed, &out.join("ryu-hadoken.png"), &pieces[0]);
    write_figure(&mut placed, &out.join("hadoken-fireball.png"), &pieces[1]);

    // Ken's Shoryuken as P1, on Mike's stage: the top of his rise, the
    // first frame his fist is highest.
    let mut s = fight(&kit, TournamentFighter::Ken, TournamentFighter::Ryu, 3);
    write_backdrop(&out, &kit.draw, &scene_of(&s), 3);
    fought[3] = true;
    let moves = special(&mut s, SHORYUKEN, &[&[RIGHT], &[DOWN]], &[DOWN, RIGHT], 2);
    write_sequence(&out, "ken-shoryuken", &kit.draw, &moves);
    let (at, ken) = moves
        .iter()
        .enumerate()
        .map(|(i, m)| (i, figures(&kit.draw, m).remove(0)))
        .min_by_key(|(i, f)| (f.y, *i))
        .unwrap();
    println!("ken-shoryuken: frame {at} of the move");
    write_figure(&mut placed, &out.join("ken-shoryuken.png"), &ken);

    // The stages no fight above was on, behind Ryu and Ken.
    for stage in (0..10).filter(|&k| !fought[k]) {
        let s = fight(&kit, TournamentFighter::Ryu, TournamentFighter::Ken, stage);
        write_backdrop(&out, &kit.draw, &scene_of(&s), stage);
    }
    let rows: Vec<String> = placed
        .iter()
        .map(|(name, f): &(String, Figure)| {
            format!("  \"{name}\": {{\"x\": {}, \"y\": {}, \"width\": {}, \"height\": {}}}", f.x, f.y, f.width, f.height)
        })
        .collect();
    std::fs::write(out.join("figures.json"), format!("{{
{}
}}
", rows.join(",
"))).unwrap();
    println!("wrote the captures to {}", out.display());
}

/// The Main Menu to a Local fight of `p1` against `p2` on stage `stage`,
/// `SETTLE` frames into the fight loop with both standing still.
fn fight(kit: &Kit, p1: TournamentFighter, p2: TournamentFighter, stage: usize) -> GameSession {
    let assets = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = GameSession::new(assets, MatchConfig::default())
        .with_sprite_bank(&kit.art)
        .with_background_bank(kit.stages.clone());
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    tap(&mut s, confirm());
    assert_eq!(tap(&mut s, confirm()), "select");
    // P1's cursor starts on Ryu (slot 0), P2's on Ken (slot 1); two rows of six.
    let (a, b) = (slot(p1), slot(p2));
    if a >= 6 {
        tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    }
    for _ in 0..a % 6 {
        tap(&mut s, with(|i| i.p1.directions[RIGHT] = true));
    }
    if b >= 6 {
        tap(&mut s, with(|i| i.p2.directions[DOWN] = true));
    }
    let (dir, steps) = if b % 6 >= 1 { (RIGHT, b % 6 - 1) } else { (LEFT, 1) };
    for _ in 0..steps {
        tap(&mut s, with(|i| i.p2.directions[dir] = true));
    }
    tap(&mut s, confirm());
    tap(&mut s, confirm()); // P1's default style
    tap(&mut s, with(|i| i.p2.buttons[0] = true));
    s.step(with(|i| i.p2.buttons[0] = true)); // P2's default style
    s.step(idle());
    until(&mut s, |s| s.screen() == "stage select", "the stage select");
    for _ in 0..stage {
        tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    }
    s.step(confirm());
    s.step(idle());
    until(&mut s, |s| s.fight_machine().map(flow_position) == Some(FIGHT_LOOP), "the fight loop");
    let roster = s.fight_machine().unwrap().match_config().roster;
    assert_eq!((roster.p1, roster.p2), (p1, p2), "the select picked the fighters");
    let listed = s.background_bank().unwrap().listed_stages()[stage].owner.to_ascii_lowercase();
    assert_eq!(listed, STAGE_OWNERS[stage], "the stage screen's order");
    for _ in 0..SETTLE {
        s.step(idle());
    }
    s
}

/// P1's special: each of `motion`'s direction sets held two frames, then
/// `last` with button `button` (LP 0, HP 2), until P1's move is `id`.
/// Returns the scenes of the move from its first frame to its end.
fn special(s: &mut GameSession, id: u8, motion: &[&[usize]], last: &[usize], button: usize) -> Vec<SceneState> {
    let hold = |dirs: &[usize], press: bool| {
        with(|i| {
            for &d in dirs {
                i.p1.directions[d] = true;
            }
            i.p1.buttons[button] = press;
        })
    };
    for dirs in motion {
        s.step(hold(dirs, false));
        s.step(hold(dirs, false));
    }
    s.step(hold(last, true));
    s.step(hold(last, true));
    let mut started = false;
    let mut scenes = Vec::new();
    for _ in 0..120 {
        s.step(idle());
        let now = s.fight().unwrap().fight_read().fighters[0].move_id;
        if now == id {
            started = true;
            scenes.push(scene_of(s));
        } else if started {
            return scenes;
        }
    }
    assert!(started, "P1's move {id:#x} started");
    scenes
}

fn slot(f: TournamentFighter) -> usize {
    TournamentFighter::ALL.iter().position(|&x| x == f).unwrap()
}

fn name(f: TournamentFighter) -> String {
    format!("{f:?}").to_ascii_lowercase()
}

fn scene_of(s: &GameSession) -> SceneState {
    s.fight_machine().expect("a fight").state.scene()
}

fn layers(scene: &SceneState, active: u8) -> SceneState {
    SceneState { active, ..scene.clone() }
}

/// One figure cut from the sprite layer: its pixels (0 where clear), where
/// its top-left stood in the frame, and its size.
#[derive(Clone)]
struct Figure {
    argb: Vec<u32>,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

/// The sprite layer's figures, left to right: runs of columns holding
/// sprite pixels, split where `GAP` empty columns part them, each cut to
/// the rows it fills.
fn figures(draw: &Assets, scene: &SceneState) -> Vec<Figure> {
    let (frame, stage) = render_frame_staged(draw, &layers(scene, SPRITES), &TextEdits::default());
    let lit = |x: usize, y: usize| !stage[y * WIDTH + x];
    let used: Vec<bool> = (0..WIDTH).map(|x| (0..HEIGHT).any(|y| lit(x, y))).collect();
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for x in (0..WIDTH).filter(|&x| used[x]) {
        match runs.last_mut() {
            Some((_, end)) if x - *end <= GAP => *end = x,
            _ => runs.push((x, x)),
        }
    }
    runs.into_iter()
        .map(|(x0, x1)| {
            let rows: Vec<usize> = (0..HEIGHT).filter(|&y| (x0..=x1).any(|x| lit(x, y))).collect();
            let (y0, y1) = (rows[0], *rows.last().unwrap());
            let (width, height) = (x1 - x0 + 1, y1 - y0 + 1);
            let mut argb = vec![0u32; width * height];
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if lit(x, y) {
                        argb[(y - y0) * width + x - x0] = 0xff00_0000 | frame.argb[y * WIDTH + x];
                    }
                }
            }
            Figure { argb, x: x0, y: y0, width, height }
        })
        .collect()
}

/// The figure as a PNG named after it, recorded in `placed`.
fn write_figure(placed: &mut Vec<(String, Figure)>, path: &Path, f: &Figure) {
    write_rgba(path, f.width, f.height, &f.argb);
    let name = path.file_stem().unwrap().to_string_lossy().into_owned();
    placed.push((name, f.clone()));
}

/// The stage alone: its background and foreground layers, no sprites or text.
fn write_backdrop(out: &Path, draw: &Assets, scene: &SceneState, stage: usize) {
    let frame = render_frame(draw, &layers(scene, BG_FG));
    let opaque: Vec<u32> = frame.argb.iter().map(|p| 0xff00_0000 | p).collect();
    write_rgba(&out.join(format!("stage-{}.png", STAGE_OWNERS[stage])), WIDTH, HEIGHT, &opaque);
}

/// The whole frame as the fight shows it, for reference.
fn write_frame(path: &Path, draw: &Assets, scene: &SceneState) {
    let frame = render_frame(draw, scene);
    let opaque: Vec<u32> = frame.argb.iter().map(|p| 0xff00_0000 | p).collect();
    write_rgba(path, WIDTH, HEIGHT, &opaque);
}

/// A move's sprite layer, frame by frame, to pick another pose by eye.
fn write_sequence(out: &Path, what: &str, draw: &Assets, scenes: &[SceneState]) {
    for (i, scene) in scenes.iter().enumerate() {
        let (frame, stage) = render_frame_staged(draw, &layers(scene, SPRITES), &TextEdits::default());
        let argb: Vec<u32> =
            frame.argb.iter().zip(&stage).map(|(&p, &bg)| if bg { 0 } else { 0xff00_0000 | p }).collect();
        write_rgba(&out.join("sequences").join(format!("{what}-{i:02}.png")), WIDTH, HEIGHT, &argb);
    }
}

fn write_rgba(path: &Path, width: usize, height: usize, argb: &[u32]) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let rgba: Vec<u8> = argb.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, (p >> 24) as u8]).collect();
    encoder.write_header().unwrap().write_image_data(&rgba).unwrap();
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn tap(s: &mut GameSession, input: FrameInputs) -> &'static str {
    let out = s.step(input);
    s.step(idle());
    out.screen
}

fn until(s: &mut GameSession, done: impl Fn(&GameSession) -> bool, what: &str) {
    for _ in 0..4_000 {
        if done(s) {
            return;
        }
        s.step(idle());
    }
    panic!("never reached {what} (at {})", s.screen());
}
