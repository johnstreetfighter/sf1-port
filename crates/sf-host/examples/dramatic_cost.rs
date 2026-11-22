//! What three Fair CPUs at once cost in Dramatic Battle, re-run under
//! rollback (SC23, `#215`). Nothing is drawn and no window opens.
//!
//! `cargo run -p sf-host --release --example dramatic_cost -- [live frames] [seed]`
//!
//! A solo Dramatic fight, Ryu in the SF6 era style as Ally 1, the boss and
//! Ally 2, each played by its own Fair CPU at Very Difficult (Ally 1's is
//! the measuring run's), is stepped on the bare machine as netplay steps
//! it: a snapshot saved every frame, and on every tenth live frame the
//! worst rollback netplay allows (8 frames, the most its settings take)
//! instead of one step: the snapshot of 8 frames back restored and 9
//! frames stepped. It runs once with all three on the old play and once
//! with all three on the stronger play, and prints the times of a frame's
//! step, a snapshot's save, and a worst rollback, against the 60 Hz frame
//! budget of 16.67 ms. Times are wall-clock on this machine, one thread.

use std::time::{Duration, Instant};

use sf_engine::era::Revision;
use sf_engine::front::cabinet::flow_position;
use sf_engine::measure::{dramatic_machine, DramaticSpec};
use sf_engine::roster::TournamentFighter;
use sf_video::romset::load_assets;

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
/// The most frames netplay rolls back (its settings clamp to 1..=8).
const MAX_ROLLBACK: usize = 8;
const BUDGET: Duration = Duration::from_micros(16_667);
/// The fight loop's place in the game flow: a round is being fought.
const FIGHT_LOOP: u32 = 0x3eafa;

struct Times {
    steps: Vec<Duration>,
    saves: Vec<Duration>,
    rollbacks: Vec<Duration>,
}

fn quantile(times: &mut [Duration], q: f64) -> Duration {
    times.sort();
    times[((times.len() - 1) as f64 * q).round() as usize]
}

fn ms(d: Duration) -> String {
    format!("{:.3} ms", d.as_secs_f64() * 1000.0)
}

fn run(rom: &[u8], stronger: bool, live: usize, seed: u64) -> Result<Times, String> {
    let spec = DramaticSpec {
        fighters: [TournamentFighter::Ryu; 3],
        styles: [Revision::Sf6; 3],
        difficulty: [4; 3],
        seed,
        stronger: [stronger; 3],
    };
    let mut machine = dramatic_machine(rom, None, &spec)?;
    if machine.dramatic_fair_cpus().is_none() || machine.p1_fair_cpu().is_none() {
        return Err("the three Fair CPUs are not in place".into());
    }
    let mut times = Times { steps: Vec::new(), saves: Vec::new(), rollbacks: Vec::new() };
    let mut ring = std::collections::VecDeque::new();
    let mut counted = 0usize;
    let mut frames = 0u32;
    while counted < live {
        frames += 1;
        if frames > 60 * 60 * 10 {
            return Err(format!("only {counted} live frames in ten minutes of play"));
        }
        let t = Instant::now();
        let snapshot = machine.save_snapshot();
        let save = t.elapsed();
        ring.push_back(snapshot);
        if ring.len() > MAX_ROLLBACK + 1 {
            ring.pop_front();
        }
        // A live frame: the round is being fought with all three in it.
        let live_now = flow_position(&machine) == FIGHT_LOOP;
        if live_now && ring.len() == MAX_ROLLBACK + 1 && counted % 10 == 9 {
            // Back to the snapshot of 8 frames ago (saved before the step
            // that left that frame), then re-run those 8 and this one.
            let t = Instant::now();
            machine.restore_snapshot(&ring[0]);
            for _ in 0..=MAX_ROLLBACK {
                machine.set_cabinet_inputs(false, false, false, false, false);
                machine.step();
            }
            times.rollbacks.push(t.elapsed());
            // The ring now holds snapshots of a timeline it re-ran.
            ring.clear();
        } else {
            let t = Instant::now();
            machine.set_cabinet_inputs(false, false, false, false, false);
            machine.step();
            if live_now {
                times.steps.push(t.elapsed());
                times.saves.push(save);
            }
        }
        if live_now {
            counted += 1;
        }
        if machine.match_over() {
            break;
        }
        if !machine.unrecovered().is_empty() {
            return Err(format!("untranslated branches at frame {}", machine.state.frame));
        }
    }
    Ok(times)
}

fn report(name: &str, mut times: Times) {
    println!("{name}: {} live frames stepped, {} worst rollbacks", times.steps.len(), times.rollbacks.len());
    for (what, list) in [("step", &mut times.steps), ("snapshot save", &mut times.saves), ("worst rollback (restore + 9 steps)", &mut times.rollbacks)] {
        if list.is_empty() {
            continue;
        }
        let (median, p99, max) = (quantile(list, 0.5), quantile(list, 0.99), quantile(list, 1.0));
        println!(
            "  {what}: median {}, 99th percentile {}, max {}  ({:.0}% of the frame budget at the 99th)",
            ms(median), ms(p99), ms(max), 100.0 * p99.as_secs_f64() / BUDGET.as_secs_f64(),
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let live: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(3000);
    let seed: u64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(1);
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
            for (name, stronger) in [("old play x3", false), ("stronger play x3", true)] {
                match run(&assets.maincpu, stronger, live, seed) {
                    Ok(times) => report(name, times),
                    Err(e) => {
                        eprintln!("{name}: {e}");
                        std::process::exit(1);
                    }
                }
            }
        })
        .unwrap();
    worker.join().unwrap();
}
