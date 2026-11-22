//! What three Fair CPUs at once cost in Dramatic Battle, re-run under
//! rollback as netplay re-runs it (SC23, `#215`). No window opens.
//!
//! `cargo run -p sf-host --release --example dramatic_cost -- [live frames] [first seed]`
//!
//! A solo Dramatic fight, Ryu in the SF6 era style as Ally 1, the boss and
//! Ally 2, each played by its own Fair CPU at Very Difficult (Ally 1's is
//! the measuring run's), is stepped as netplay's rollback engine steps a
//! frame (`RollbackEngine::simulate`): a snapshot of the fight saved, the
//! front end's `Fight::step` (the frame drawn), and the fight's state
//! checksum. On every tenth live frame the worst rollback netplay allows
//! (8 frames, the most its settings take) is taken in place of one frame:
//! the snapshot of 8 frames back restored and 9 frames so re-run. Matches
//! are played from the first seed on until the live frames are counted.
//! It runs with all three on the old play and then all three on the
//! stronger play, prints each, and the difference: what the stronger play
//! adds. The route books are parsed before any timing (the stronger play
//! parses them when a fight starts); the time of that first parse is
//! printed too. Times are wall clock on this machine, one thread, against
//! the 60 Hz frame budget of 16.67 ms.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use sf_engine::era::Revision;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::PlayerInputs;
use sf_engine::measure::{dramatic_fight, DramaticSpec};
use sf_engine::roster::TournamentFighter;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
/// The most frames netplay rolls back (its settings clamp to 1..=8).
const MAX_ROLLBACK: usize = 8;
const BUDGET: Duration = Duration::from_micros(16_667);
/// The fight loop's place in the game flow: a round is being fought.
const FIGHT_LOOP: u32 = 0x3eafa;

#[derive(Default)]
struct Times {
    frames: Vec<Duration>,
    rollbacks: Vec<Duration>,
}

fn quantile(times: &[Duration], q: f64) -> Duration {
    let mut sorted = times.to_vec();
    sorted.sort();
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

fn ms(d: Duration) -> String {
    format!("{:.3} ms", d.as_secs_f64() * 1000.0)
}

fn run(assets: &Assets, stronger: bool, live: usize, first_seed: u64) -> Result<Times, String> {
    let mut times = Times::default();
    let mut seed = first_seed;
    while times.frames.len() + times.rollbacks.len() * 10 < live {
        let spec = DramaticSpec {
            fighters: [TournamentFighter::Ryu; 3],
            styles: [Revision::Sf6; 3],
            difficulty: [4; 3],
            seed,
            stronger: [stronger; 3],
        };
        seed += 1;
        let mut fight = dramatic_fight(&assets.maincpu, None, &spec)?;
        if fight.machine().dramatic_fair_cpus().is_none() || fight.machine().p1_fair_cpu().is_none() {
            return Err("the three Fair CPUs are not in place".into());
        }
        let mut ring = VecDeque::new();
        let mut counted = 0usize;
        let mut net_frame = 0u32;
        // One frame as `RollbackEngine::simulate` runs it.
        let simulate = |fight: &mut sf_engine::front::fight::Fight, frame: u32| {
            let before = fight.save_netplay_snapshot(frame);
            fight.step(assets, PlayerInputs::default(), PlayerInputs::default(), false);
            std::hint::black_box(fight.state_checksum());
            before
        };
        for _ in 0..60 * 60 * 10 {
            let live_now = flow_position(fight.machine()) == FIGHT_LOOP;
            if live_now && ring.len() == MAX_ROLLBACK + 1 && counted % 10 == 9 {
                // The ring holds the snapshots from before frames N-9 to
                // N-1: back to frame N-8, then re-run it to N, 9 frames.
                let t = Instant::now();
                ring.pop_front();
                let back: sf_engine::front::fight::FightSnapshot = ring.pop_front().unwrap();
                fight.restore_netplay_snapshot(&back);
                ring.clear();
                for k in 0..=MAX_ROLLBACK as u32 {
                    ring.push_back(simulate(&mut fight, back.frame + k));
                }
                times.rollbacks.push(t.elapsed());
            } else {
                let t = Instant::now();
                ring.push_back(simulate(&mut fight, net_frame));
                let took = t.elapsed();
                if live_now {
                    times.frames.push(took);
                }
            }
            while ring.len() > MAX_ROLLBACK + 1 {
                ring.pop_front();
            }
            net_frame += 1;
            counted += usize::from(live_now);
            if fight.machine().match_over() || !fight.machine().unrecovered().is_empty() {
                break;
            }
        }
        if !fight.machine().unrecovered().is_empty() {
            return Err(format!("untranslated branches with seed {}", seed - 1));
        }
    }
    Ok(times)
}

fn line(what: &str, list: &[Duration]) -> String {
    format!(
        "  {what}: median {}, 99th percentile {}, max {}  ({:.1}% of the frame budget at the 99th)",
        ms(quantile(list, 0.5)),
        ms(quantile(list, 0.99)),
        ms(quantile(list, 1.0)),
        100.0 * quantile(list, 0.99).as_secs_f64() / BUDGET.as_secs_f64(),
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let live: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(3000);
    let seed: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(1);
    let t = Instant::now();
    sf_engine::route_book::warm();
    println!("first parse of the route books: {}", ms(t.elapsed()));
    let assets = match load_assets(ROMS, MANIFEST) {
        Ok(assets) => assets,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    // A whole match is deep in the translated program's calls.
    let worker = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let mut medians = Vec::new();
            for (name, stronger) in [("old play x3", false), ("stronger play x3", true)] {
                match run(&assets, stronger, live, seed) {
                    Ok(times) => {
                        println!("{name}: {} live frames, {} worst rollbacks", times.frames.len(), times.rollbacks.len());
                        println!("{}", line("frame (snapshot, Fight::step, checksum)", &times.frames));
                        println!("{}", line("worst rollback (restore, then 9 such frames)", &times.rollbacks));
                        medians.push((quantile(&times.frames, 0.5), quantile(&times.rollbacks, 0.5), quantile(&times.rollbacks, 0.99)));
                    }
                    Err(e) => {
                        eprintln!("{name}: {e}");
                        std::process::exit(1);
                    }
                }
            }
            let [(old_frame, old_roll, old_p99), (new_frame, new_roll, new_p99)] = medians[..] else { unreachable!() };
            let delta = |a: Duration, b: Duration| (b.as_secs_f64() - a.as_secs_f64()) * 1000.0;
            println!(
                "the stronger play adds: {:+.3} ms a frame (median), {:+.3} ms a worst rollback (median), {:+.3} ms at the 99th",
                delta(old_frame, new_frame),
                delta(old_roll, new_roll),
                delta(old_p99, new_p99),
            );
        })
        .unwrap();
    worker.join().unwrap();
}
