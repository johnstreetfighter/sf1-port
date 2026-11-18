//! Find and record a combo of five or more hits for a fighter in an era
//! style, from the headless game session. Every frame is the game's own and
//! only P1's pad is scripted; the count is the fight screen's combo counter.
//! No sound.
//!
//! Each pair runs in Training against the standing dummy with FILL METERS
//! on. The search tries pad actions (normals, command motions with each
//! button set, the system button pairs) from the fight's own state and keeps
//! the ones that raise the engine's hit count on the dummy; the route that
//! reaches the target is then replayed from the same state and recorded.
//!
//! `cargo run -p sf-host --release --example combo_clips -- <out> [fighter|all] [revision-substring] [threads]`
//!
//! Writes `<out>/<slug>.mkv` (384x224 at 60 fps, lossless) for each pair that
//! reaches the target and one line per pair in `<out>/results.tsv`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use sf_engine::config::MatchConfig;
use sf_engine::era::{listed_families, listed_revisions, loadouts, EraFamily, Loadout, Revision};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::{Fight, FightKind};
use sf_engine::front::menu::MenuItem;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, Transition};
use sf_engine::roster::TournamentFighter;
use sf_engine::run::PowerOnMachine;
use sf_host::log::Log;
use sf_video::callouts::Side;
use sf_video::render::{HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const TARGET: u16 = 5;

const R: u8 = 1;
const L: u8 = 2;
const D: u8 = 4;
const U: u8 = 8;
const BUTTONS: [&str; 6] = ["LP", "MP", "HP", "LK", "MK", "HK"];

/// One pad frame: direction bits and button bits (LP..HK from bit 0).
type Pad = (u8, u8);

fn inputs((dirs, buttons): Pad) -> PlayerInputs {
    PlayerInputs {
        directions: [R, L, D, U].map(|bit| dirs & bit != 0),
        buttons: std::array::from_fn(|i| buttons & (1 << i) != 0),
    }
}

#[derive(Clone)]
struct Action {
    name: String,
    pads: Vec<Pad>,
    /// The direction held while waiting to press: down-back for a charge move.
    hold: u8,
}

fn button_sets() -> Vec<(String, u8)> {
    let mut sets: Vec<(String, u8)> = (0..6).map(|i| (BUTTONS[i].to_owned(), 1 << i)).collect();
    sets.push(("PP".into(), 0b000_011));
    sets.push(("KK".into(), 0b011_000));
    sets.push(("PPP".into(), 0b000_111));
    sets.push(("KKK".into(), 0b111_000));
    sets
}

fn actions() -> Vec<Action> {
    let mut out = Vec::new();
    for (stance, dir) in [("", 0), ("cr.", D), ("f+", R), ("b+", L)] {
        for i in 0..6 {
            out.push(Action { name: format!("{stance}{}", BUTTONS[i]), pads: vec![(dir, 1 << i), (dir, 1 << i), (dir, 0)], hold: 0 });
        }
    }
    let motions: [(&str, &[u8]); 9] = [
        ("236", &[D, D | R, R]),
        ("214", &[D, D | L, L]),
        ("623", &[R, D, D | R]),
        ("421", &[L, D, D | L]),
        ("41236", &[L, D | L, D, D | R, R]),
        ("63214", &[R, D | R, D, D | L, L]),
        ("236236", &[D, D | R, R, D, D | R, R]),
        ("214214", &[D, D | L, L, D, D | L, L]),
        ("d,d", &[D, 0, D]),
    ];
    for (name, steps) in motions {
        for (set, bits) in button_sets() {
            let mut pads: Vec<Pad> = Vec::new();
            for &step in steps {
                pads.extend([(step, 0), (step, 0)]);
            }
            let last = *steps.last().unwrap();
            pads.extend([(last, bits), (last, bits), (last, bits), (last, 0)]);
            out.push(Action { name: format!("{name}+{set}"), pads, hold: 0 });
        }
    }
    // Charge moves: down-back held through the wait, then the release.
    let releases: [(&str, &[u8]); 3] = [("[4]6", &[R]), ("[2]8", &[U]), ("[4]646", &[R, L, R])];
    for (name, steps) in releases {
        for (set, bits) in button_sets() {
            let mut pads: Vec<Pad> = Vec::new();
            for &step in &steps[..steps.len() - 1] {
                pads.extend([(step, 0), (step, 0)]);
            }
            let last = *steps.last().unwrap();
            pads.extend([(last, bits), (last, bits), (last, bits), (last, 0)]);
            out.push(Action { name: format!("{name}+{set}"), pads, hold: D | L });
        }
    }
    for i in 0..6 {
        out.push(Action { name: format!("db+{}", BUTTONS[i]), pads: vec![(D | L, 1 << i), (D | L, 1 << i), (D | L, 0)], hold: D | L });
    }
    for (name, bits) in [("LP+LK", 0b001_001), ("MP+MK", 0b010_010), ("HP+HK", 0b100_100)] {
        out.push(Action { name: name.into(), pads: vec![(0, bits), (0, bits), (0, bits), (0, 0)], hold: 0 });
    }
    out
}

/// Jump-in openers: up-forward, then the button on the way down.
fn jump_ins() -> Vec<Action> {
    let mut out = Vec::new();
    for button in [2usize, 5, 4] {
        for at in (18..=36).step_by(2) {
            let mut pads = vec![(U | R, 0); 3];
            pads.extend(std::iter::repeat_n((0, 0), at - 3));
            pads.extend([(0, 1 << button), (0, 1 << button), (0, 0)]);
            out.push(Action { name: format!("j.{}", BUTTONS[button]), pads, hold: 0 });
        }
    }
    out
}

struct Kit {
    assets: Assets,
    art: sf_video::art::Art,
    stages: sf_video::background_bank::BackgroundBank,
}

fn tap(s: &mut GameSession, input: FrameInputs) -> Option<Transition> {
    let t = s.step(input).transition;
    s.step(FrameInputs::default());
    t
}

fn p1(pad: Pad) -> FrameInputs {
    FrameInputs { p1: inputs(pad), ..FrameInputs::default() }
}

fn choose(s: &mut GameSession, item: MenuItem) -> Result<(), String> {
    for _ in 0..8 {
        if s.menu_item() == Some(item) {
            tap(s, p1((0, 1)));
            return Ok(());
        }
        tap(s, p1((D, 0)));
    }
    Err(format!("{item:?} missing from {}", s.screen()))
}

/// The Style card, as `tests/support/alpha.rs` drives it.
fn lock_style(s: &mut GameSession, fighter: TournamentFighter, revision: Revision, loadout: Loadout) -> Result<(), String> {
    if tap(s, p1((0, 1))) != Some(Transition::StyleOpened) {
        return Err("the Style card did not open".into());
    }
    let family = revision.family();
    let order = [
        EraFamily::Sf1Plus,
        EraFamily::Sf2,
        EraFamily::SfZ,
        EraFamily::Sf3,
        EraFamily::Sf4,
        EraFamily::Sf5,
        EraFamily::Sf6,
        EraFamily::Sf1987,
    ];
    let available = listed_families(fighter, true);
    let steps = order.into_iter().filter(|f| available.contains(f)).position(|f| f == family).ok_or("family not listed")?;
    for _ in 0..steps {
        tap(s, p1((R, 0)));
    }
    let revisions = listed_revisions(fighter, family, true);
    let at = revisions.iter().position(|&r| r == revision).ok_or("revision not listed")?;
    let change = at + 1 < revisions.len();
    if change {
        tap(s, p1((D, 0)));
        for _ in 0..=at {
            tap(s, p1((R, 0)));
        }
    }
    let load = loadouts(revision).iter().position(|&l| l == loadout).ok_or("loadout not listed")?;
    if load != 0 {
        tap(s, p1((if change { D } else { U }, 0)));
        for _ in 0..load {
            tap(s, p1((R, 0)));
        }
    }
    tap(s, p1((0, 1)));
    Ok(())
}

/// A Training fight against the dummy, P1 walked in close, lifted out of the session.
fn training(kit: &Kit, fighter: TournamentFighter, revision: Revision, loadout: Loadout, p2: u8) -> Result<(GameSession, Fight), String> {
    let assets = load_assets(ROMS, MANIFEST).map_err(|e| e.to_string())?;
    let mut s = GameSession::new(assets, MatchConfig::definitive())
        .with_settings(Settings { fill_meters: true, hold_combo: true, ..Settings::default() })
        .with_sprite_bank(&kit.art.bank)
        .with_player_tables(&kit.art.catalogues)
        .with_background_bank(kit.stages.clone());
    for _ in 0..200 {
        s.step(FrameInputs::default());
    }
    let mut start = FrameInputs::default();
    start.menu.start = true;
    tap(&mut s, start);
    choose(&mut s, MenuItem::SinglePlayer)?;
    choose(&mut s, MenuItem::Training)?;
    choose(&mut s, MenuItem::TrainingDummy)?;
    let index = TournamentFighter::ALL.iter().position(|&f| f == fighter).unwrap();
    for dir in std::iter::repeat_n(D, index / 6).chain(std::iter::repeat_n(R, index % 6)) {
        tap(&mut s, p1((dir, 0)));
    }
    lock_style(&mut s, fighter, revision, loadout)?;
    // The dummy and its default style.
    if tap(&mut s, p1((0, 1))) == Some(Transition::StyleOpened) {
        tap(&mut s, p1((0, 1)));
    }
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        match s.screen() {
            "stage select" | "country select" => {
                tap(&mut s, p1((0, 1)));
            }
            _ => {
                s.step(FrameInputs::default());
            }
        }
        f += 1;
        if f > 3_000 {
            return Err(format!("no fight loop (at {})", s.screen()));
        }
    }
    let style = s.fight_machine().unwrap().era_fighters()[0].style;
    if (style.revision, style.loadout) != (revision, loadout) || s.fight().unwrap().player_fighter(0) != fighter {
        return Err(format!("the fight opened as {:?} {:?} {:?}", s.fight().unwrap().player_fighter(0), style.revision, style.loadout));
    }
    let fa = s.fight_assets().ok_or("no fight assets")?;
    let mut fight = s.fight().unwrap().clone();
    fight.set_fill_meters(true);
    // The Training dummy's pad is held still. The count only grows on a
    // grounded reaction, and a crouching fighter's are all grounded, so the
    // fight goes on as a two-pad one where P2 can hold Down: Training's rules
    // (FILL METERS, the health floor, the stopped clock) stay in the machine.
    let mut snap = fight.save_netplay_snapshot(0);
    snap.setup.kind = FightKind::Local;
    fight.restore_netplay_snapshot(&snap);
    for _ in 0..60 {
        fight.step(fa, PlayerInputs::default(), inputs((p2, 0)), false);
    }
    // Walk in until the gap stops closing.
    let gap = |f: &Fight| {
        let t = f.machine().diagnostic_telemetry();
        t.p2.center_x - t.p1.center_x
    };
    let mut last = gap(&fight);
    let mut still = 0;
    for _ in 0..400 {
        fight.step(fa, inputs((R, 0)), inputs((p2, 0)), false);
        let now = gap(&fight);
        still = if now == last { still + 1 } else { 0 };
        last = now;
        // Some walks pause between steps: a short still spell is not the end.
        if still >= 40 {
            break;
        }
    }
    for _ in 0..20 {
        fight.step(fa, PlayerInputs::default(), inputs((p2, 0)), false);
    }
    eprintln!("{fighter:?} {revision:?}: gap {} after the walk in", gap(&fight));
    Ok((s, fight))
}

/// The search runs on the bare machine: the fight's own step draws every frame.
fn tick(m: &mut PowerOnMachine, (dirs, buttons): Pad, p2: u8) {
    m.set_p1_direction(dirs & R != 0, dirs & L != 0, dirs & D != 0, dirs & U != 0);
    m.set_p1_buttons(std::array::from_fn(|i| buttons & (1 << i) != 0));
    m.set_p2_direction(false, false, p2 & D != 0, false);
    m.set_p2_buttons([false; 6]);
    m.set_cabinet_inputs(false, false, false, false, false);
    m.step();
}

fn engine_hits(m: &PowerOnMachine) -> u8 {
    m.toggle_state().combo.hits[1] as u8
}

fn hud_hits(fight: &Fight) -> u16 {
    fight.callouts().frame().side(Side::P1).combo.map_or(0, |c| c.hits)
}

#[derive(Clone)]
struct Node {
    machine: PowerOnMachine,
    pads: Vec<Pad>,
    names: Vec<String>,
    hits: u8,
    /// The engine count's highest reading if the pad is left alone from here.
    ride: u8,
}

/// Leave the pad alone until the count has been quiet for 50 frames. Returns
/// the count's highest reading, and the state and frame of its last rise.
fn ride_out(machine: &PowerOnMachine, from: u8, p2: u8) -> (u8, Option<(PowerOnMachine, usize, u8)>) {
    let mut m = machine.clone();
    let mut top = from;
    let mut last = None;
    let mut quiet = 0;
    for t in 1..=300 {
        tick(&mut m, (0, 0), p2);
        let e = engine_hits(&m);
        if e > top {
            top = e;
            last = Some((m.clone(), t, e));
            quiet = 0;
        } else {
            quiet += 1;
            if quiet > 50 {
                break;
            }
        }
    }
    (top, last)
}

fn expand(node: &Node, action: &Action, delay: usize, wait: usize, p2: u8) -> Vec<Node> {
    let mut m = node.machine.clone();
    let mut pads = node.pads.clone();
    for &pad in std::iter::repeat_n(&(action.hold, 0), delay).chain(&action.pads) {
        tick(&mut m, pad, p2);
        pads.push(pad);
        if engine_hits(&m) > node.hits {
            break;
        }
    }
    let mut t = 0;
    while engine_hits(&m) <= node.hits {
        // A count that fell back means the combo dropped.
        if t >= wait || engine_hits(&m) < node.hits {
            return Vec::new();
        }
        tick(&mut m, (0, 0), p2);
        pads.push((0, 0));
        t += 1;
    }
    let hits = engine_hits(&m);
    let mut names = node.names.clone();
    names.push(action.name.clone());
    let (ride, last) = ride_out(&m, hits, p2);
    let mut out = vec![Node { machine: m, pads: pads.clone(), names: names.clone(), hits, ride }];
    if let Some((machine, frames, hits)) = last {
        pads.extend(std::iter::repeat_n((0, 0), frames));
        out.push(Node { machine, pads, names, hits, ride });
    }
    out
}

/// How good an opener a move is: the cancelable normals first.
fn lead_in(name: &str) -> usize {
    ["LK", "cr.LP", "LP", "MP", "cr.MP", "cr.LK", "cr.MK"].iter().position(|&n| n == name).map_or(0, |i| i + 1)
}

fn system(action: &Action) -> bool {
    matches!(action.name.as_str(), "LP+LK" | "MP+MK" | "HP+HK")
}

struct Found {
    hits: u16,
    names: Vec<String>,
    pads: Vec<Pad>,
    nodes: usize,
}

fn found(node: Node, nodes: usize) -> Found {
    let mut pads = node.pads;
    pads.extend(std::iter::repeat_n((0, 0), 300));
    Found { hits: node.ride.into(), names: node.names, pads, nodes }
}

fn search(base: &Fight, p2: u8) -> (Option<Found>, u8, usize) {
    let t0 = std::time::Instant::now();
    let health = (sf_engine::fighter::P2_STRUCT + 6 - sf_engine::fighter::WORK_BASE) as usize;
    let root = Node { machine: base.machine().clone(), pads: Vec::new(), names: Vec::new(), hits: 0, ride: 0 };
    let mut open: Vec<Node> = Vec::new();
    // Keep one action of each kind of opener: those whose first hit comes
    // from the same move on the same frame for the same damage do the same.
    let mut all: Vec<Action> = Vec::new();
    let mut seen = Vec::new();
    for action in actions() {
        let nodes = expand(&root, &action, if action.hold != 0 { 60 } else { 0 }, 60, p2);
        let Some(first) = nodes.first() else {
            if system(&action) {
                all.push(action);
            }
            continue;
        };
        let work = first.machine.work_ram();
        let sig = (first.pads.len(), first.machine.diagnostic_telemetry().p1.move_id, first.ride, work[health], work[health + 1]);
        if action.hold != 0 || system(&action) || !seen.contains(&sig) {
            seen.push(sig);
            all.push(action);
            open.extend(nodes);
        }
    }
    for action in jump_ins() {
        open.extend(expand(&root, &action, 0, 60, p2));
    }
    // A system press (Custom Combo, a trigger) that hits nothing, then an opener.
    for press in all.iter().filter(|a| system(a)) {
        let mut m = root.machine.clone();
        for &pad in &press.pads {
            tick(&mut m, pad, p2);
        }
        let primed = Node { machine: m, pads: press.pads.clone(), names: vec![press.name.clone()], hits: 0, ride: 0 };
        for action in &all {
            for delay in [0, 4, 8, 14, 22, 32] {
                open.extend(expand(&primed, action, delay, 50, p2));
            }
        }
    }
    eprintln!("{} actions, {} openers at {:?}: {}", all.len(), open.len(), t0.elapsed(), all.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(" "));
    let mut best = 0u8;
    let mut nodes = 0;
    // The first route found that is one move alone; one with a lead-in is preferred.
    let mut raw: Option<Node> = None;
    while !open.is_empty() && nodes < 70 && t0.elapsed().as_secs() < 420 {
        // The fullest route first; among equals the one with more moves in it.
        open.sort_by_key(|n| (n.ride.min(TARGET as u8 + 3), n.names.len().min(3), n.hits, lead_in(&n.names[0])));
        let node = open.pop().unwrap();
        best = best.max(node.ride);
        eprintln!("node {nodes} at {:?}: hits {} ride {} [{}] open {}", t0.elapsed(), node.hits, node.ride, node.names.join(", "), open.len());
        if u16::from(node.ride) >= TARGET && raw.is_none() {
            raw = Some(node.clone());
        }
        if u16::from(node.ride) >= TARGET && (node.names.len() >= 2 || nodes >= 24) {
            let node = if node.names.len() >= 2 { node } else { raw.clone().unwrap() };
            return (Some(found(node, nodes)), best, all.len());
        }
        nodes += 1;
        // More hits are already on their way from this state: a press here
        // would be credited with them. Its twin at the last hit goes on.
        if node.ride > node.hits {
            continue;
        }
        for action in &all {
            for delay in 0..=22 {
                open.extend(expand(&node, action, delay, 45, p2));
            }
        }
        // Keep the open list small: one route per sequence of moves, the best few.
        open.sort_by_key(|n| (n.ride, n.hits, std::cmp::Reverse(n.pads.len())));
        let mut kept: Vec<Node> = Vec::new();
        for n in open.drain(..).rev() {
            if !kept.iter().any(|k| k.names == n.names && k.hits == n.hits) {
                kept.push(n);
            }
        }
        kept.truncate(36);
        kept.reverse();
        open = kept;
    }
    (raw.map(|node| found(node, nodes)), best, all.len())
}

/// Replay the route from the base state, recording every frame. Returns the
/// counter's highest reading and the frames written.
fn record(base: &Fight, fa: &Assets, found: &Found, path: &Path, p2: u8) -> (u16, usize) {
    let mut ffmpeg = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "bgra", "-s"])
        .arg(format!("{WIDTH}x{HEIGHT}"))
        .args(["-r", "60", "-i", "-", "-an", "-c:v", "libx264rgb", "-preset", "veryfast", "-crf", "0"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .expect("ffmpeg on the PATH");
    let mut f = base.clone();
    let mut best = 0;
    let mut frames = 0;
    let mut quiet = 0;
    let lead = std::iter::repeat_n(&(0, 0), 30);
    for &pad in lead.chain(&found.pads) {
        let out = f.step(fa, inputs(pad), inputs((p2, 0)), false);
        let bytes: Vec<u8> = out.frame.argb.iter().flat_map(|p| p.to_le_bytes()).collect();
        ffmpeg.stdin.as_mut().unwrap().write_all(&bytes).unwrap();
        frames += 1;
        let now = hud_hits(&f);
        if now > best {
            best = now;
            quiet = 0;
        } else if best >= found.hits {
            quiet += 1;
            if quiet >= 100 {
                break;
            }
        }
    }
    drop(ffmpeg.stdin.take());
    assert!(ffmpeg.wait().unwrap().success(), "ffmpeg wrote {}", path.display());
    (best, frames)
}

fn slug(fighter: TournamentFighter, revision: Revision, loadout: Loadout) -> String {
    let clean = |s: &str| s.to_ascii_lowercase().replace([' ', '/', '+'], "-").replace("--", "-").replace("--", "-");
    format!("{}_{}_{}", clean(fighter.name()), clean(&format!("{revision:?}")), clean(&format!("{loadout:?}")))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("combo-clips"));
    let who = args.get(2).map(String::as_str).unwrap_or("all").to_ascii_lowercase();
    let which = args.get(3).map(String::as_str).unwrap_or("").to_ascii_lowercase();
    let threads: usize = args.get(4).and_then(|t| t.parse().ok()).unwrap_or(4);
    std::fs::create_dir_all(&out).unwrap();

    let mut pairs = Vec::new();
    for fighter in TournamentFighter::ALL {
        if who != "all" && fighter.name().to_ascii_lowercase() != who {
            continue;
        }
        for family in listed_families(fighter, true) {
            for revision in listed_revisions(fighter, family, true) {
                for loadout in loadouts(revision) {
                    let what_if = !listed_revisions(fighter, family, false).contains(&revision);
                    let id = slug(fighter, revision, loadout);
                    if which.is_empty() || id.contains(&which) {
                        pairs.push((fighter, revision, loadout, what_if));
                    }
                }
            }
        }
    }
    println!("{} pairs", pairs.len());
    if args.iter().any(|a| a == "--list") {
        for (fighter, revision, loadout, what_if) in &pairs {
            println!("{}\t{}\t{}\t{}", fighter.name(), revision.name(), loadout.name(), if *what_if { "what-if" } else { "canonical" });
        }
        return;
    }

    let queue = Arc::new(Mutex::new(pairs.into_iter()));
    let results = Arc::new(Mutex::new(
        std::fs::OpenOptions::new().create(true).append(true).open(out.join("results.tsv")).unwrap(),
    ));
    let workers: Vec<_> = (0..threads)
        .map(|w| {
            let (queue, results, out) = (queue.clone(), results.clone(), out.clone());
            std::thread::Builder::new()
                .stack_size(64 << 20)
                .spawn(move || {
                    let mut log = Log::create(&out.join(format!("worker-{w}.log")));
                    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
                    let art = sf_host::art::load_with_fallbacks(&assets, &[Path::new(ROMS)], &[Path::new("assets").to_path_buf()], &mut log).expect("the art pack loads");
                    let stages = sf_host::stages::load(&assets, &mut log).expect("the background bank builds");
                    let kit = Kit { assets, art, stages };
                    let _ = &kit.assets;
                    loop {
                        let Some((fighter, revision, loadout, what_if)) = queue.lock().unwrap().next() else { break };
                        let id = slug(fighter, revision, loadout);
                        let started = std::time::Instant::now();
                        if revision == Revision::Arcade1987 || out.join(format!("{id}.mkv")).exists() {
                            continue;
                        }
                        // Against a crouching opponent first, then a standing one.
                        let mut line = String::new();
                        for (stance, p2) in [("crouching", D), ("standing", 0)] {
                            line = match training(&kit, fighter, revision, loadout, p2) {
                                Err(e) => format!("setup-failed\t0\t0\t0\t{e}\t{stance}"),
                                Ok((session, base)) => {
                                    let fa = session.fight_assets().unwrap();
                                    let (found, best, kinds) = search(&base, p2);
                                    match found {
                                        Some(found) => {
                                            let path = out.join(format!("{id}.mkv"));
                                            let (shown, frames) = record(&base, fa, &found, &path, p2);
                                            let status = if shown >= TARGET { "ok" } else { "replay-short" };
                                            if shown < TARGET {
                                                let _ = std::fs::remove_file(&path);
                                            }
                                            format!("{status}\t{shown}\t{frames}\t{}\t{}\t{stance}", found.nodes, found.names.join(", "))
                                        }
                                        None => format!("not-found\t{best}\t0\t{kinds}\t\t{stance}"),
                                    }
                                }
                            };
                            if line.starts_with("ok") {
                                break;
                            }
                        }
                        let row = format!(
                            "{id}\t{}\t{}\t{}\t{}\t{line}\t{:.0}s\n",
                            fighter.name(),
                            revision.name(),
                            loadout.name(),
                            if what_if { "what-if" } else { "canonical" },
                            started.elapsed().as_secs_f32()
                        );
                        print!("{row}");
                        results.lock().unwrap().write_all(row.as_bytes()).unwrap();
                    }
                })
                .unwrap()
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}
