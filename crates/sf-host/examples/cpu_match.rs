//! Record a match of Ryu against the game's CPU Ken, both in the SF6 style
//! (or the SF5 one),
//! at the Very Difficult setting, from the headless game session. Every
//! frame is the game's own. No sound.
//!
//! Ken is the fair CPU on Player 2's pad. Ryu's pad is scripted: the game's
//! reaction CPU (`PowerOnMachine::utility_takeover_pad`) when it presses
//! anything, otherwise a seeded plan of approach and attack written here.
//! The fight is opened as Training against a CPU and then switched to Versus
//! rules, so that it has rounds, a clock and knockouts.
//!
//! `cargo run -p sf-host --release --example cpu_match -- <out> <first seed> <count> [sf6|sf5]`
//!
//! Writes `<out>/match-<seed>.mkv` (384x224 at 60 fps, lossless) and one line
//! per match in `<out>/matches.tsv`: seed, frames, whether the match ended,
//! attacks connected by each side, the longest HUD combo of each side, the
//! frames Ryu's pad came from the reaction CPU and from the plan, and the
//! frame of each connected attack.
//!
//! The picked match is encoded with ffmpeg (two passes at a bit rate that
//! keeps the file under 4 MB, 768x448, no audio).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sf_engine::config::MatchConfig;
use sf_engine::roster::PlayMode;
use sf_engine::era::{listed_families, listed_revisions, loadouts, EraFamily, Loadout, Revision};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::{FightEvent, FightKind};
use sf_engine::front::menu::MenuItem;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, Transition};
use sf_engine::roster::TournamentFighter;
use sf_engine::utility::UtilityState;
use sf_host::log::Log;
use sf_video::callouts::Side;
use sf_video::render::{HEIGHT, WIDTH};
use sf_video::romset::load_assets;

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const R: usize = 0;
const D: usize = 2;
const U: usize = 3;

fn pad(side: usize, direction: Option<usize>, lp: bool) -> FrameInputs {
    let mut one = PlayerInputs::default();
    if let Some(d) = direction {
        one.directions[d] = true;
    }
    one.buttons[0] = lp;
    let mut input = FrameInputs::default();
    if side == 0 {
        input.p1 = one;
    } else {
        input.p2 = one;
    }
    input
}

fn tap(s: &mut GameSession, input: FrameInputs) -> Option<Transition> {
    let t = s.step(input).transition;
    s.step(FrameInputs::default());
    t
}

fn choose(s: &mut GameSession, item: MenuItem) -> Result<(), String> {
    for _ in 0..8 {
        if s.menu_item() == Some(item) {
            tap(s, pad(0, None, true));
            return Ok(());
        }
        tap(s, pad(0, Some(D), false));
    }
    Err(format!("{item:?} missing from {}", s.screen()))
}

/// The Style card, as `tests/support/alpha.rs` drives it, on one side's pad.
fn lock_style(s: &mut GameSession, side: usize, fighter: TournamentFighter, revision: Revision, loadout: Loadout) -> Result<(), String> {
    if tap(s, pad(side, None, true)) != Some(Transition::StyleOpened) {
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
        tap(s, pad(side, Some(R), false));
    }
    let revisions = listed_revisions(fighter, family, true);
    let at = revisions.iter().position(|&r| r == revision).ok_or("revision not listed")?;
    let change = at + 1 < revisions.len();
    if change {
        tap(s, pad(side, Some(D), false));
        for _ in 0..=at {
            tap(s, pad(side, Some(R), false));
        }
    }
    let load = loadouts(revision).iter().position(|&l| l == loadout).ok_or("loadout not listed")?;
    if load != 0 {
        tap(s, pad(side, Some(if change { D } else { U }), false));
        for _ in 0..load {
            tap(s, pad(side, Some(R), false));
        }
    }
    tap(s, pad(side, None, true));
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("cpu-match"));
    let first: u64 = args.get(2).and_then(|t| t.parse().ok()).unwrap_or(1);
    let count: u64 = args.get(3).and_then(|t| t.parse().ok()).unwrap_or(1);
    // The style both fighters take: SF6, or the first SF5 with V-Skill I and V-Trigger I.
    let style = match args.get(4).map(String::as_str) {
        Some("sf5") => (Revision::Sf5, Loadout::Skill1Trigger1),
        _ => (Revision::Sf6, Loadout::Standard),
    };
    std::fs::create_dir_all(&out).unwrap();

    let handles: Vec<_> = (first..first + count)
        .map(|seed| {
            let out = out.clone();
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn(move || {
                    let row = one(&out, seed, style).unwrap_or_else(|e| format!("{seed}\tfailed\t{e}"));
                    println!("{row}");
                    row
                })
                .unwrap()
        })
        .collect();
    let rows: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(out.join("matches.tsv")).unwrap();
    for row in rows {
        writeln!(file, "{row}").unwrap();
    }
}

/// One plan for Ryu's pad, facing right: direction bits (right 1, left 2,
/// down 4, up 8) and button bits (LP..HK from bit 0), a frame each.
fn attack(gap: u32, roll: u64) -> Vec<(u8, u8)> {
    const HP: u8 = 4;
    const MK: u8 = 16;
    const HK: u8 = 32;
    let hold = |pad: (u8, u8), frames: usize| std::iter::repeat_n(pad, frames);
    let motion = |steps: &[u8], buttons: u8| {
        let mut out: Vec<(u8, u8)> = steps.iter().flat_map(|&d| [(d, 0), (d, 0)]).collect();
        let last = *steps.last().unwrap();
        out.extend([(last, buttons), (last, buttons), (last, buttons), (last, 0)]);
        out
    };
    let fireball = || motion(&[4, 5, 1], HP);
    let walk = || hold((1, 0), 18).collect::<Vec<_>>();
    let jump_in = || {
        let mut out: Vec<(u8, u8)> = hold((9, 0), 3).chain(hold((0, 0), 30)).chain(hold((0, HK), 3)).chain(hold((0, 0), 8)).collect();
        out.extend(hold((4, MK), 3));
        out.extend(hold((4, 0), 2));
        out.extend(motion(&[4, 5, 1], HP));
        out
    };
    let low_super = || {
        let mut out: Vec<(u8, u8)> = hold((4, MK), 3).chain(hold((4, 0), 2)).collect();
        out.extend(motion(&[4, 5, 1, 4, 5, 1], 7));
        out.extend(hold((0, 0), 20));
        out
    };
    let low_uppercut = || {
        let mut out: Vec<(u8, u8)> = hold((4, 8), 3).chain(hold((4, 0), 6)).chain(hold((4, 1), 3)).chain(hold((4, 0), 4)).collect();
        out.extend(motion(&[1, 4, 5], HP));
        out.extend(hold((0, 0), 20));
        out
    };
    let impact = || hold((0, HP | HK), 4).chain(hold((0, 0), 30)).collect::<Vec<_>>();
    let guard = || hold((6, 0), 25).collect::<Vec<_>>();
    match (gap, roll) {
        (121.., 0..50) => fireball(),
        (121.., 50..85) => walk(),
        (121.., _) => jump_in(),
        (60..=120, 0..35) => jump_in(),
        (60..=120, 35..65) => walk(),
        (60..=120, 65..85) => fireball(),
        (60..=120, _) => impact(),
        (_, 0..35) => low_super(),
        (_, 35..70) => low_uppercut(),
        (_, 70..85) => impact(),
        _ => guard(),
    }
}

fn one(out: &Path, seed: u64, (revision, loadout): (Revision, Loadout)) -> Result<String, String> {
    let mut log = Log::create(&out.join(format!("match-{seed}.log")));
    let assets = load_assets(ROMS, MANIFEST).map_err(|e| e.to_string())?;
    let art = sf_host::art::load_with_fallbacks(&assets, &[Path::new(ROMS)], &[Path::new("assets").to_path_buf()], &mut log)
        .ok_or("no art pack")?;
    let stages = sf_host::stages::load(&assets, &mut log).ok_or("no background bank")?;
    let mut config = MatchConfig::definitive();
    config.difficulty = 4;
    let mut s = GameSession::new(assets, config)
        .with_settings(Settings::default())
        .with_sprite_bank(&art.bank)
        .with_player_tables(&art.catalogues)
        .with_background_bank(stages);
    for _ in 0..200 {
        s.step(FrameInputs::default());
    }
    let mut start = FrameInputs::default();
    start.menu.start = true;
    tap(&mut s, start);
    choose(&mut s, MenuItem::SinglePlayer)?;
    choose(&mut s, MenuItem::Training)?;
    choose(&mut s, MenuItem::TrainingCpu)?;
    lock_style(&mut s, 0, TournamentFighter::Ryu, revision, loadout)?;
    lock_style(&mut s, 0, TournamentFighter::Ken, revision, loadout)?;
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        match s.screen() {
            "stage select" | "country select" => {
                tap(&mut s, pad(0, None, true));
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
    let machine = s.fight_machine().unwrap();
    let fight = s.fight().unwrap();
    let styles = machine.era_fighters().map(|e| (e.style.revision, e.style.loadout));
    let who = [fight.player_fighter(0), fight.player_fighter(1)];
    if who != [TournamentFighter::Ryu, TournamentFighter::Ken] || styles != [(revision, loadout); 2] {
        return Err(format!("the fight opened as {who:?} {styles:?}"));
    }
    if machine.match_config().difficulty != 4 {
        return Err(format!("difficulty {}", machine.match_config().difficulty));
    }
    let fa = s.fight_assets().ok_or("no fight assets")?;
    let mut fight = fight.clone();
    // Training against a CPU is the one menu path that puts the fair CPU on
    // Player 2's pad with a free choice of opponent and style, but Training
    // holds health at a floor and stops the clock. The same fight goes on as
    // a Versus match (rounds, clock, knockouts) with the CPU kept on P2.
    let mut snap = fight.save_netplay_snapshot(0);
    snap.setup.kind = FightKind::Local;
    snap.setup.config.roster.mode = PlayMode::Versus;
    let mut machine = fight.machine().clone();
    machine.match_config_mut().roster.mode = PlayMode::Versus;
    snap.machine = machine.save_snapshot();
    fight.restore_netplay_snapshot(&snap);
    let now = fight.machine().match_config();
    if now.roster.mode != PlayMode::Versus || !now.p2_fair_cpu || now.difficulty != 4 {
        return Err(format!("the match runs as {:?}, CPU {}, difficulty {}", now.roster.mode, now.p2_fair_cpu, now.difficulty));
    }

    let path = out.join(format!("match-{seed}.mkv"));
    let mut ffmpeg = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "bgra", "-s"])
        .arg(format!("{WIDTH}x{HEIGHT}"))
        .args(["-r", "60", "-i", "-", "-an", "-c:v", "libx264rgb", "-preset", "veryfast", "-crf", "0"])
        .arg(&path)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    let mut brain = UtilityState::seeded(seed);
    let mut rng = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    let mut plan: std::collections::VecDeque<(u8, u8)> = Default::default();
    let (mut from_brain, mut from_plan) = (0u32, 0u32);
    let mut connects = [0u32; 2];
    let mut longest = [0u16; 2];
    let mut marks: Vec<String> = Vec::new();
    let mut ended = "no";
    let mut frames = 0u32;
    let mut after = 0;
    while frames < 60 * 240 {
        // Ryu's pad: the game's reaction CPU when it presses anything,
        // otherwise this recorder's own plan of approach and attack.
        let t = fight.machine().diagnostic_telemetry();
        let gap = t.p2.center_x - t.p1.center_x;
        let react = fight.machine().utility_takeover_pad(0, &mut brain);
        let p1 = if react != PlayerInputs::default() {
            plan.clear();
            from_brain += 1;
            react
        } else {
            if plan.is_empty() && fight.round_live() {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                plan.extend(attack(gap.unsigned_abs(), (rng >> 20) % 100));
            }
            let (dirs, buttons) = plan.pop_front().unwrap_or((0, 0));
            from_plan += u32::from((dirs, buttons) != (0, 0));
            // The plans are written facing right.
            let dirs = if gap < 0 { (dirs & !3) | ((dirs & 1) << 1) | ((dirs & 2) >> 1) } else { dirs };
            PlayerInputs {
                directions: [1, 2, 4, 8].map(|bit| dirs & bit != 0),
                buttons: std::array::from_fn(|i| buttons & (1 << i) != 0),
            }
        };
        // Ken's pad is the fair CPU's: the fight sets it.
        let p2 = PlayerInputs::default();
        let step = fight.step(fa, p1, p2, false);
        let bytes: Vec<u8> = step.frame.argb.iter().flat_map(|p| p.to_le_bytes()).collect();
        ffmpeg.stdin.as_mut().unwrap().write_all(&bytes).map_err(|e| e.to_string())?;
        frames += 1;
        let t = fight.machine().diagnostic_telemetry();
        for (i, side) in [(0, &t.p1), (1, &t.p2)] {
            if side.attack_connected_this_frame || side.projectile_impacted_this_frame {
                connects[i] += 1;
                marks.push(format!("{frames}:{}", i + 1));
            }
        }
        let hud = fight.callouts().frame();
        for (i, side) in [Side::P1, Side::P2].into_iter().enumerate() {
            longest[i] = longest[i].max(hud.side(side).combo.map_or(0, |c| c.hits));
        }
        match step.event {
            Some(FightEvent::MatchOver) => ended = "match over",
            Some(FightEvent::Untranslated { .. }) => {
                ended = "untranslated";
                break;
            }
            _ => {}
        }
        if ended == "match over" {
            after += 1;
            if after > 90 {
                break;
            }
        }
    }
    drop(ffmpeg.stdin.take());
    if !ffmpeg.wait().map_err(|e| e.to_string())?.success() {
        return Err("ffmpeg failed".into());
    }
    Ok(format!(
        "{seed}\t{frames}\t{ended}\t{}\t{}\t{}\t{}\t{from_brain}\t{from_plan}\t{}",
        connects[0],
        connects[1],
        longest[0],
        longest[1],
        marks.join(" ")
    ))
}
