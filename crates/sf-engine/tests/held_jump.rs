//! A held jump must take off and finish its first arc just as a two-frame
//! tap does. Actual Local and Arcade GameSessions reached through the
//! front-end flow; no fight-state writes, forced poses or replacement engine.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fighter::{CPU_STRUCT, P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession, Transition};
use sf_engine::moves::{facing_right, input_gate_free, MachineRead};
use sf_engine::roster::{StartingCountry, TournamentFighter as Fighter};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::load_assets;
use sf_video::stage::pack_records;

const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;

fn input(side: usize, directions: &[usize]) -> FrameInputs {
    let mut i = FrameInputs::default();
    let p = if side == 0 { &mut i.p1 } else { &mut i.p2 };
    for &d in directions {
        p.directions[d] = true;
    }
    i
}

fn tap(s: &mut GameSession, i: FrameInputs) -> sf_engine::front::FrameOutput {
    let out = s.step(i);
    s.step(FrameInputs::default());
    out
}

fn seed(preset: RulesetPreset) -> Option<GameSession> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rom = root.join("tools/reference/local/roms/sf");
    let pack = root.join("tools/reference/local/roms/sf1-port-art");
    if !rom.is_dir() || !pack.is_dir() {
        eprintln!("skipping: local ROM set or art pack not present");
        return None;
    }
    let a = load_assets(rom, root.join("tools/reference/rom-manifest.json")).unwrap();
    let pack = ArtPack::open(&pack).unwrap();
    let names = [
        "ryu", "ken", "retsu", "geki", "joe", "mike", "lee", "gen", "birdie", "eagle", "adon",
        "sagat",
    ];
    let read = |dir: &str| {
        names
            .iter()
            .map(|n| std::fs::read_to_string(root.join(format!("art/{dir}/{n}.json"))).unwrap())
            .collect::<Vec<_>>()
    };
    let recipes = read("recipes");
    let reviews = read("review");
    let recipes: Vec<_> = recipes.iter().map(String::as_str).collect();
    let reviews: Vec<_> = reviews.iter().map(String::as_str).collect();
    let art = load_art_from(
        &a,
        &ArtSources {
            recipes: &recipes,
            reviews: Some(&reviews),
            pack: Ok(&pack),
        },
    )
    .unwrap();
    assert_eq!(
        art.catalogues.len(),
        12,
        "all twelve reviewed fighters loaded"
    );
    let mut backgrounds = pack_records(&a, &[]).unwrap();
    backgrounds.add_rom_stages(&a).unwrap();
    backgrounds.add_vs_screens(&a).unwrap();
    Some(
        GameSession::new(a, MatchConfig::from_preset(preset))
            .with_sprite_bank(&art.bank)
            .with_player_tables(&art.catalogues)
            .with_background_bank(backgrounds),
    )
}

fn local(mut s: GameSession, p1: Fighter, p2: Fighter) -> GameSession {
    for _ in 0..200 {
        s.step(FrameInputs::default());
    }
    let mut i = FrameInputs::default();
    i.menu.start = true;
    tap(&mut s, i);
    tap(&mut s, input(0, &[DOWN]));
    let mut i = FrameInputs::default();
    i.menu.confirm = true;
    tap(&mut s, i); // Multi Player
    tap(&mut s, i); // Local
    assert_eq!(s.screen(), "select");
    for (side, f, from) in [(0, p1, 0usize), (1, p2, 1usize)] {
        let at = Fighter::ALL.iter().position(|&x| x == f).unwrap();
        if at >= 6 {
            tap(&mut s, input(side, &[DOWN]));
        }
        let col = at % 6;
        for _ in 0..from.abs_diff(col) {
            tap(
                &mut s,
                input(side, &[if col < from { LEFT } else { RIGHT }]),
            );
        }
        let mut i = FrameInputs::default();
        if side == 0 {
            i.menu.confirm = true;
        } else {
            i.p2.buttons[0] = true;
        }
        let expected = if side == 0 {
            Transition::Picked(p1)
        } else {
            Transition::Fighters(p1, p2)
        };
        assert_eq!(select::confirm_fighter(&mut s, i, tap).transition, Some(expected));
    }
    for _ in 0..3500 {
        if s.fight_machine().map(flow_position) == Some(0x3eafa) {
            for _ in 0..240 {
                s.step(FrameInputs::default());
            }
            let r = s.fight().unwrap().setup().config.roster;
            assert_eq!((r.p1, r.p2), (p1, p2));
            return s;
        }
        if matches!(s.screen(), "stage select" | "country select") {
            let mut i = FrameInputs::default();
            i.p1.buttons[0] = true;
            tap(&mut s, i);
        } else {
            s.step(FrameInputs::default());
        }
    }
    panic!("never reached the real fight loop; screen {}", s.screen());
}

#[derive(Clone, Copy, Debug)]
struct Sample {
    move_id: u8,
    state: u8,
    center_y: i32,
}

fn sample(s: &GameSession, side: usize) -> Sample {
    let ps = if side == 0 { P1_STRUCT } else { P2_STRUCT };
    sample_at(s, ps, side)
}

fn sample_at(s: &GameSession, ps: u32, side: usize) -> Sample {
    let m = s.fight_machine().unwrap();
    let p = (ps + 0x10 - 0xff8000) as usize;
    let w = m.work_ram();
    let e = u16::from_be_bytes([w[p], w[p + 1]]) as usize;
    let o = &m.state.objectram;
    let attr = u16::from_be_bytes([o[e + 2], o[e + 3]]);
    let y = u16::from_be_bytes([o[e + 4], o[e + 5]]) as i16 as i32;
    let cell = if attr & 0x400 != 0 { 32 } else { 16 };
    let r = s.fight().unwrap().fight_read().fighters[side];
    Sample {
        move_id: r.move_id,
        state: r.state,
        center_y: y + (i32::from(o[e + 0xc]) + 1) * cell / 2,
    }
}

fn trace(base: &GameSession, side: usize, dirs: &[usize], hold: usize) -> Vec<Sample> {
    let mut s = base.clone();
    (0..220)
        .map(|f| {
            s.step(if f < hold {
                input(side, dirs)
            } else {
                FrameInputs::default()
            });
            assert_eq!(s.screen(), "fight");
            assert!(s.fight_machine().unwrap().unrecovered().is_empty());
            let observed = sample(&s, side);
            let roster = s.fight().unwrap().setup().config.roster;
            let fighter = if side == 0 { roster.p1 } else { roster.p2 };
            if !fighter.is_shoto() && observed.state == 0x82 && matches!(observed.move_id, 9..=0xb)
            {
                let m = s.fight_machine().unwrap();
                let machine = MachineRead {
                    rom: m.program_rom(),
                    work: m.work_ram(),
                    obj: &m.state.objectram,
                    bank: m.data_bank(),
                };
                assert!(
                    !input_gate_free(
                        machine,
                        if side == 0 { P1_STRUCT } else { P2_STRUCT },
                        &m.match_config().toggles
                    ),
                    "the coach must see the banked descending jump as busy"
                );
            }
            observed
        })
        .collect()
}

fn airborne(s: Sample, ground_center: i32) -> bool {
    matches!(s.state, 0x81 | 0xc1) && s.center_y < ground_center - 8
}

fn check(preset: RulesetPreset) {
    let Some(seed) = seed(preset) else { return };
    let mut failures = Vec::new();
    for fighter in Fighter::ALL {
        let peer = if fighter == Fighter::Ken {
            Fighter::Ryu
        } else {
            Fighter::Ken
        };
        for side in 0..2 {
            let base = if side == 0 {
                local(seed.clone(), fighter, peer)
            } else {
                local(seed.clone(), peer, fighter)
            };
            let m = base.fight_machine().unwrap();
            let fr = facing_right(
                m.work_ram(),
                &m.state.objectram,
                if side == 0 { P1_STRUCT } else { P2_STRUCT },
            );
            let forward = if fr { RIGHT } else { LEFT };
            let back = if fr { LEFT } else { RIGHT };
            let ground = sample(&base, side).center_y;
            for (name, dirs) in [
                ("neutral", vec![UP]),
                ("forward", vec![UP, forward]),
                ("back", vec![UP, back]),
            ] {
                let tapped = trace(&base, side, &dirs, 2);
                let held = trace(&base, side, &dirs, 120);
                let takeoff = tapped
                    .iter()
                    .position(|&s| airborne(s, ground))
                    .expect("two-frame tap leaves the ground");
                let held_takeoff = held.iter().position(|&s| airborne(s, ground));
                let landing = tapped
                    .iter()
                    .enumerate()
                    .skip(takeoff + 1)
                    .find(|(_, s)| s.move_id == 1 && (s.center_y - ground).abs() <= 1)
                    .map(|(n, _)| n)
                    .expect("tapped jump lands back in stance");
                let held_lands_after_release = held
                    .iter()
                    .skip(120)
                    .any(|s| s.move_id == 1 && (s.center_y - ground).abs() <= 1);
                // Compare the first airborne arc only: held Up may legitimately
                // begin another jump after this first landing.
                let ground_return = tapped
                    .iter()
                    .enumerate()
                    .skip(takeoff + 1)
                    .find(|(_, s)| s.center_y >= ground - 1)
                    .map(|(n, _)| n)
                    .expect("tapped jump returns to ground height");
                let same_arc =
                    (takeoff..ground_return).all(|n| held[n].center_y == tapped[n].center_y);
                if held_takeoff != Some(takeoff) || !same_arc || !held_lands_after_release {
                    failures.push(format!("{fighter:?} P{} {name}: tap takeoff={takeoff} land={landing}; held takeoff={held_takeoff:?}, same_first_arc={same_arc}, lands_after_release={held_lands_after_release}", side + 1));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "held-Up continuation differs under {preset:?}:\n{}",
        failures.join("\n")
    );
}

#[test]
fn held_up_keeps_the_first_jump_arc_under_modern_rules() {
    check(RulesetPreset::DefinitiveOverhaul);
}

#[test]
fn held_up_keeps_the_first_jump_arc_under_classic_rules() {
    check(RulesetPreset::ClassicCompetitive);
}

#[test]
fn lee_neutral_jump_takes_off_while_up_is_held() {
    let Some(seed) = seed(RulesetPreset::DefinitiveOverhaul) else {
        return;
    };
    let base = local(seed, Fighter::Lee, Fighter::Ken);
    let ground = sample(&base, 0).center_y;
    let tapped = trace(&base, 0, &[UP], 2);
    let held = trace(&base, 0, &[UP], 120);
    assert_eq!(
        held.iter().position(|&s| airborne(s, ground)),
        tapped.iter().position(|&s| airborne(s, ground)),
        "holding Up must not restart takeoff"
    );
}

#[test]
fn held_up_does_not_restart_a_banked_jump_during_descent() {
    let Some(seed) = seed(RulesetPreset::DefinitiveOverhaul) else {
        return;
    };
    let mut failures = Vec::new();
    for (fighter, dirs) in [(Fighter::Lee, vec![UP, RIGHT]), (Fighter::Sagat, vec![UP])] {
        let base = local(seed.clone(), fighter, Fighter::Ken);
        let ground = sample(&base, 0).center_y;
        let tapped = trace(&base, 0, &dirs, 2);
        let held = trace(&base, 0, &dirs, 120);
        let takeoff = tapped.iter().position(|&s| airborne(s, ground)).unwrap();
        let ground_return = (takeoff + 1..tapped.len())
            .find(|&n| tapped[n].center_y >= ground - 1)
            .unwrap();
        assert!(
            tapped[takeoff..ground_return]
                .iter()
                .any(|s| s.state == 0x82),
            "the regression exercises the bank's falling posture"
        );
        if let Some(n) = (takeoff..ground_return).find(|&n| tapped[n].center_y != held[n].center_y)
        {
            failures.push(format!("{fighter:?} restarted before ground return at{ground_return}: frame{n} tap={:?}, held={:?}", tapped[n], held[n]));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn arcade_lee(mut s: GameSession) -> GameSession {
    s.set_fight_dips(sf_engine::front::FightDips {
        difficulty: 3,
        round_time: 200,
    });
    for _ in 0..200 {
        s.step(FrameInputs::default());
    }
    let mut start = FrameInputs::default();
    start.menu.start = true;
    tap(&mut s, start);
    let mut confirm = FrameInputs::default();
    confirm.menu.confirm = true;
    tap(&mut s, confirm); // Single Player
    tap(&mut s, confirm); // Arcade
    assert_eq!(s.screen(), "select");
    assert_eq!(
        select::confirm_fighter(&mut s, confirm, tap).transition,
        Some(Transition::Fighter(Fighter::Ryu))
    );
    for _ in 0..60 {
        if s.screen() == "country select" {
            break;
        }
        s.step(FrameInputs::default());
    }
    assert_eq!(s.screen(), "country select");
    let china = (0..4)
        .find(|&i| s.assets().maincpu[0x3e2ea + i] == StartingCountry::China as u8)
        .unwrap();
    tap(&mut s, input(0, &[if china >= 2 { DOWN } else { UP }]));
    tap(
        &mut s,
        input(0, &[if china % 2 == 1 { RIGHT } else { LEFT }]),
    );
    let mut lp = FrameInputs::default();
    lp.p1.buttons[0] = true;
    tap(&mut s, lp);
    for _ in 0..3500 {
        if s.fight_machine().map(flow_position) == Some(0x3eafa) {
            for _ in 0..240 {
                s.step(FrameInputs::default());
            }
            assert_eq!(
                sf_engine::front::cabinet::chosen_country(s.fight_machine().unwrap()),
                StartingCountry::China as u8
            );
            return s;
        }
        s.step(FrameInputs::default());
    }
    panic!("Arcade never reached China's first fight");
}

#[test]
fn arcade_banked_cpu_jump_keeps_its_falling_gate_closed() {
    let Some(seed) = seed(RulesetPreset::DefinitiveOverhaul) else {
        return;
    };
    let s = arcade_lee(seed);
    // This is the real reached Arcade CPU slot and table. Use its normal
    // public pad interface deterministically instead of the fair CPU's
    // randomized plan, whose jump-ins normally become jump attacks.
    let mut m = s.fight_machine().unwrap().clone();
    let mut config = *m.match_config();
    config.p2_fair_cpu = false;
    m.set_match_config(config);
    let at = (CPU_STRUCT + 0x12 - 0xff8000) as usize;
    let table = u32::from_be_bytes(m.work_ram()[at..at + 4].try_into().unwrap());
    assert_eq!(Some(table), s.data_bank().unwrap().table(Fighter::Lee));
    let ground = machine_sample(&m, CPU_STRUCT).center_y;
    let mut saw_airborne = false;
    for f in 0..100 {
        m.set_p1_direction(false, false, false, false);
        m.set_p1_buttons([false; 6]);
        m.set_p2_direction(false, f < 2, false, f < 2);
        m.set_p2_buttons([false; 6]);
        m.step();
        let current = machine_sample(&m, CPU_STRUCT);
        if matches!(current.move_id, 9..=0xb) && airborne(current, ground) {
            saw_airborne = true;
        }
        if saw_airborne && matches!(current.move_id, 9..=0xb) && current.state == 0x82 {
            let machine = MachineRead {
                rom: m.program_rom(),
                work: m.work_ram(),
                obj: &m.state.objectram,
                bank: m.data_bank(),
            };
            let closed = !input_gate_free(machine, CPU_STRUCT, &m.match_config().toggles);
            let mut quiet = m.clone();
            let mut attack = m;
            let mut divergence = None;
            let mut compared = 0;
            for next in 0..20 {
                quiet.set_p2_direction(false, false, false, false);
                quiet.set_p2_buttons([false; 6]);
                quiet.step();
                attack.set_p2_direction(false, false, false, false);
                let mut hp = [false; 6];
                hp[2] = next < 2;
                attack.set_p2_buttons(hp);
                attack.step();
                let q = machine_sample(&quiet, CPU_STRUCT);
                let a = machine_sample(&attack, CPU_STRUCT);
                if q.center_y >= ground - 1 {
                    break;
                }
                compared += 1;
                if q.move_id != a.move_id || q.center_y != a.center_y {
                    divergence.get_or_insert((next, q, a));
                }
            }
            assert!(
                compared > 0,
                "the CPU falling posture is observed before touchdown"
            );
            assert!(
                closed && divergence.is_none(),
                "Arcade CPU falling at{f} must stay locked: gate_closed={closed}, attack divergence={divergence:?}"
            );
            return;
        }
        if current.move_id >= 0x30 || current.move_id == 1 {
            saw_airborne = false;
        }
    }
    panic!("the real Arcade CPU never exposed a falling bank jump");
}

fn machine_sample(m: &sf_engine::run::PowerOnMachine, ps: u32) -> Sample {
    let w = m.work_ram();
    let p = (ps + 0x10 - 0xff8000) as usize;
    let e = u16::from_be_bytes([w[p], w[p + 1]]) as usize;
    let o = &m.state.objectram;
    let attr = u16::from_be_bytes([o[e + 2], o[e + 3]]);
    let y = u16::from_be_bytes([o[e + 4], o[e + 5]]) as i16 as i32;
    let cell = if attr & 0x400 != 0 { 32 } else { 16 };
    Sample {
        move_id: o[e + 0xa],
        state: o[e + 9],
        center_y: y + (i32::from(o[e + 0xc]) + 1) * cell / 2,
    }
}

fn grid_edge(m: &sf_engine::run::PowerOnMachine, ps: u32, horizontal: bool) -> i32 {
    let w = m.work_ram();
    let p = (ps + 0x10 - 0xff8000) as usize;
    let e = u16::from_be_bytes([w[p], w[p + 1]]) as usize;
    let o = &m.state.objectram;
    let big = u16::from_be_bytes([o[e + 2], o[e + 3]]) & 0x400 != 0;
    let cell = if big { 32 } else { 16 };
    let (pos, extent) = if horizontal { (6, 0xd) } else { (4, 0xc) };
    let origin = u16::from_be_bytes([o[e + pos], o[e + pos + 1]]) as i16 as i32;
    origin + (i32::from(o[e + extent]) + 1) * cell / if horizontal { 2 } else { 1 }
}

#[test]
fn arcade_banked_cpu_crouching_hit_stays_on_the_floor() {
    let Some(seed) = seed(RulesetPreset::DefinitiveOverhaul) else {
        return;
    };
    let s = arcade_lee(seed);
    let mut m = s.fight_machine().unwrap().clone();
    let mut config = *m.match_config();
    config.p2_fair_cpu = false;
    m.set_match_config(config);
    m.set_p1_buttons([false; 6]);
    m.set_p2_buttons([false; 6]);
    m.set_p2_direction(false, false, true, false);
    for _ in 0..120 {
        if (grid_edge(&m, P1_STRUCT, true) - grid_edge(&m, CPU_STRUCT, true)).abs() <= 48 {
            break;
        }
        m.set_p1_direction(true, false, false, false);
        m.step();
    }
    m.set_p1_direction(false, false, true, false);
    for _ in 0..8 {
        m.step();
    }
    assert_eq!(
        machine_sample(&m, CPU_STRUCT).move_id,
        5,
        "the Arcade CPU is crouching before contact"
    );
    for f in 0..70 {
        let mut buttons = [false; 6];
        buttons[5] = f < 2;
        m.set_p1_buttons(buttons);
        m.step();
        let current = machine_sample(&m, CPU_STRUCT);
        if (0x30..=0x3f).contains(&current.move_id) {
            // Allow the first reaction record to load after the collision.
            m.set_p1_buttons([false; 6]);
            m.step();
            assert_eq!(
                grid_edge(&m, CPU_STRUCT, false),
                232,
                "Arcade Lee's first crouching-hit pose must be grounded"
            );
            return;
        }
    }
    panic!("the input-driven low kick did not hit the crouching Arcade CPU");
}
