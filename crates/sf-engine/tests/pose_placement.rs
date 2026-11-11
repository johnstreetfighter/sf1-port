//! Real-session regressions for the 2026-11-10 pose-placement audit.
//! Menus and pad inputs reach every scene; observations never write RAM.
#[path = "support/select.rs"]
mod select;

use sf_engine::{
    config::{MatchConfig, RulesetPreset},
    fighter::{P1_STRUCT, P2_STRUCT},
    data_bank::{SF2_SPECIAL_SLOT, SF2_SUPER_SLOT},
    era::Revision,
    front::{cabinet::flow_position, settings::Settings, FrameInputs, GameSession, Transition},
    moves::{facing_right, move_list, Strength},
    roster::TournamentFighter,
};
use sf_video::{
    art::{load_art_from, Art, ArtSources},
    art_pack::ArtPack,
    decode::decode_sprite_tile,
    romset::load_assets,
    sprite_bank::sprite_slot,
};
use std::{collections::BTreeSet, fs, path::Path, sync::OnceLock};

const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;
const GROUND: i32 = 232;
struct Local {
    seed: GameSession,
    art: Art,
    tiles: Vec<[[u8; 16]; 16]>,
}
fn local() -> Option<&'static Local> {
    static LOCAL: OnceLock<Option<Local>> = OnceLock::new();
    LOCAL
        .get_or_init(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let roms = root.join("tools/reference/local/roms/sf");
            let pack = root.join("tools/reference/local/roms/sf1-port-art");
            if !roms.is_dir() || !pack.is_dir() {
                eprintln!("skipping: local ROMs and approved art pack required");
                return None;
            }
            let assets = load_assets(roms, root.join("tools/reference/rom-manifest.json")).unwrap();
            let pack = ArtPack::open(&pack).unwrap();
            let names = [
                "ryu", "ken", "retsu", "geki", "joe", "mike", "lee", "gen", "birdie", "eagle",
                "adon", "sagat",
            ];
            let recipes: Vec<_> = names
                .iter()
                .map(|n| fs::read_to_string(root.join(format!("art/recipes/{n}.json"))).unwrap())
                .collect();
            let reviews: Vec<_> = names
                .iter()
                .map(|n| fs::read_to_string(root.join(format!("art/review/{n}.json"))).unwrap())
                .collect();
            let recipes: Vec<&str> = recipes.iter().map(String::as_str).collect();
            let reviews: Vec<&str> = reviews.iter().map(String::as_str).collect();
            let art = load_art_from(
                &assets,
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
                "all tournament fighters' approved poses load: {:?}",
                art.kept
            );
            let banked = assets.with_sprite_bank(&art.bank);
            let tiles = (0..banked.gfx3.len() / 128)
                .map(|n| decode_sprite_tile(&banked.gfx3, n))
                .collect();
            let seed = GameSession::new(
                assets,
                MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul),
            )
            .with_sprite_bank(&art.bank)
            .with_player_tables(&art.catalogues);
            Some(Local { seed, art, tiles })
        })
        .as_ref()
}
fn idle() -> FrameInputs {
    FrameInputs::default()
}
fn input(side: usize, dirs: &[usize], button: Option<usize>) -> FrameInputs {
    let mut i = idle();
    let p = if side == 0 { &mut i.p1 } else { &mut i.p2 };
    for &d in dirs {
        p.directions[d] = true;
    }
    if let Some(b) = button {
        p.buttons[b] = true;
    }
    i
}
fn menu(f: impl FnOnce(&mut FrameInputs)) -> FrameInputs {
    let mut i = idle();
    f(&mut i);
    i
}
fn tap(s: &mut GameSession, i: FrameInputs) -> Option<Transition> {
    let o = s.step(i);
    s.step(idle());
    o.transition
}
fn player(side: usize) -> u32 {
    if side == 0 {
        P1_STRUCT
    } else {
        P2_STRUCT
    }
}
fn entity(s: &GameSession, side: usize) -> usize {
    let w = s.fight_machine().unwrap().work_ram();
    let a = (player(side) + 0x10 - 0xff8000) as usize;
    u16::from_be_bytes([w[a], w[a + 1]]) as usize
}
fn word(s: &GameSession, side: usize, off: usize) -> u16 {
    let o = &s.fight_machine().unwrap().state.objectram;
    let a = entity(s, side) + off;
    u16::from_be_bytes([o[a], o[a + 1]])
}
fn center_x(s: &GameSession, side: usize) -> i32 {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = entity(s, side);
    let cell = if word(s, side, 2) & 0x400 != 0 {
        32
    } else {
        16
    };
    word(s, side, 6) as i16 as i32 + (i32::from(o[e + 0xd]) + 1) * cell / 2
}
fn facing(s: &GameSession, side: usize) -> bool {
    let m = s.fight_machine().unwrap();
    facing_right(m.work_ram(), &m.state.objectram, player(side))
}
fn at_control(l: &Local, fighter: TournamentFighter, side: usize) -> GameSession {
    at_control_seed(l.seed.clone(), fighter, side, false)
}
fn at_control_seed(
    mut s: GameSession,
    fighter: TournamentFighter,
    side: usize,
    st_training: bool,
) -> GameSession {
    let peer = if fighter == TournamentFighter::Ken && !st_training {
        TournamentFighter::Ryu
    } else {
        TournamentFighter::Ken
    };
    let (p1, p2) = if side == 0 {
        (fighter, peer)
    } else {
        (peer, fighter)
    };
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, menu(|i| i.menu.start = true));
    let confirm = menu(|i| i.menu.confirm = true);
    if st_training {
        tap(&mut s, confirm); // Single Player
        tap(&mut s, input(0, &[DOWN], None));
        tap(&mut s, confirm); // Training
        tap(&mut s, input(0, &[DOWN], None));
        tap(&mut s, confirm); // Dummy
    } else {
        tap(&mut s, input(0, &[DOWN], None));
        tap(&mut s, confirm); // Multi Player
        tap(&mut s, confirm); // Local
    }
    for (pick, f, start) in [(0, p1, 0usize), (1, p2, 1usize)] {
        let at = TournamentFighter::ALL.iter().position(|&x| x == f).unwrap();
        if at >= 6 {
            tap(&mut s, input(pick, &[DOWN], None));
        }
        let col = at % 6;
        if col < start {
            for _ in col..start {
                tap(&mut s, input(pick, &[LEFT], None));
            }
        } else {
            for _ in start..col {
                tap(&mut s, input(pick, &[RIGHT], None));
            }
        }
        let pad = if pick == 0 || st_training {
            confirm
        } else {
            input(1, &[], Some(0))
        };
        let got = if st_training && pick == 0 {
            assert_eq!(tap(&mut s, pad), Some(Transition::StyleOpened));
            assert_eq!(
                tap(&mut s, input(0, &[RIGHT], None)),
                Some(Transition::CursorMoved)
            );
            tap(&mut s, pad)
        } else {
            select::confirm_fighter(&mut s, pad, tap)
        };
        assert_eq!(
            got,
            Some(if pick == 0 {
                Transition::Picked(p1)
            } else {
                Transition::Fighters(p1, p2)
            })
        );
    }
    for _ in 0..3500 {
        if s.fight_machine().map(flow_position) == Some(0x3eafa) {
            for _ in 0..240 {
                s.step(idle());
            }
            return s;
        }
        if s.screen() == "country select" {
            tap(&mut s, input(0, &[], Some(0)));
        } else {
            s.step(idle());
        }
    }
    panic!("never reached control: {}", s.screen());
}
fn close(mut s: GameSession, side: usize) -> GameSession {
    let other = 1 - side;
    for _ in 0..240 {
        if (center_x(&s, 0) - center_x(&s, 1)).abs() <= 40 {
            break;
        }
        let d = if facing(&s, other) { RIGHT } else { LEFT };
        s.step(input(other, &[d], None));
    }
    for _ in 0..16 {
        s.step(idle());
    }
    s
}
/// Actual opaque pixels of the fighter's live sprite entries, before viewport clipping.
fn opaque_bottom(l: &Local, s: &GameSession, side: usize) -> i32 {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = entity(s, side);
    let w = |a| u16::from_be_bytes([o[a], o[a + 1]]);
    let n = ((usize::from(o[e + 0xc]) + 1) * (usize::from(o[e + 0xd]) + 1)).min(24);
    let mut bottom = i32::MIN;
    for k in 0..n {
        let a = e + k * 0x40;
        let code = w(a);
        let attr = w(a + 2);
        let y = i32::from(w(a + 4) as i16);
        let big = attr & 0x400 != 0;
        let fx = attr & 0x100 != 0;
        let fy = attr & 0x200 != 0;
        let mut q = [
            code,
            code.wrapping_add(1),
            code.wrapping_add(16),
            code.wrapping_add(17),
        ];
        if big && fx {
            q.swap(0, 1);
            q.swap(2, 3);
        }
        if big && fy {
            q.swap(0, 2);
            q.swap(1, 3);
        }
        for i in 0..if big { 4 } else { 1 } {
            let tile = &l.tiles[sprite_slot(q[i]) % l.tiles.len()];
            for py in 0..16 {
                if tile[if fy { 15 - py } else { py }].iter().any(|&p| p != 15) {
                    bottom = bottom.max(y + if big { (i / 2) as i32 * 16 } else { 0 } + py as i32);
                }
            }
        }
    }
    bottom
}
fn trace(
    l: &Local,
    mut s: GameSession,
    side: usize,
    script: &[FrameInputs],
    label: &str,
    errors: &mut Vec<String>,
) -> BTreeSet<u8> {
    let mut first = None;
    let mut moves = BTreeSet::new();
    for (tick, i) in script.iter().enumerate() {
        s.step(*i);
        moves.insert(s.fight().unwrap().fight_read().fighters[side].move_id);
        let bottom = opaque_bottom(l, &s, side);
        if bottom >= GROUND && first.is_none() {
            let r = s.fight().unwrap().fight_read().fighters[side];
            first=Some(format!("{label} side{side} tick{tick}: opaque bottom{bottom}, floor{}, move{:#x} state{:#x}, code{:#x}, grid_y{}",GROUND-1,r.move_id,r.state,word(&s,side,0),word(&s,side,4)));
        }
    }
    if let Some(e) = first {
        errors.push(e);
    }
    moves
}

#[test]
fn sf2_rising_specials_keep_both_fighters_above_the_floor() {
    let Some(l) = local() else { return };
    let super_motion = &[2, 3, 6, 2, 3, 6][..];
    let uppercut_motion = &[6, 2, 3][..];
    let mut errors = Vec::new();
    for (fighter, moves) in [
        (
            TournamentFighter::Ken,
            &[("Shoryureppa", super_motion, 0, SF2_SUPER_SLOT)][..],
        ),
        (
            TournamentFighter::Sagat,
            &[
                ("Tiger Uppercut", uppercut_motion, 0, SF2_SPECIAL_SLOT),
                ("Tiger Genocide", super_motion, 3, SF2_SUPER_SLOT),
            ][..],
        ),
    ] {
        let seed = l.seed.clone().with_settings(Settings {
            fill_meters: true,
            ..Settings::default()
        });
        let mut base = at_control_seed(seed, fighter, 0, true);
        for _ in 0..180 {
            if center_x(&base, 1) - center_x(&base, 0) <= 48 {
                break;
            }
            base.step(input(0, &[RIGHT], None));
        }
        for _ in 0..4 {
            base.step(idle());
        }
        assert_eq!(
            base.fight_machine().unwrap().era_fighters()[0].style.revision,
            Revision::St
        );
        for &(name, directions, button, expected_move) in moves {
            let mut script = Vec::new();
            for (index, &direction) in directions.iter().enumerate() {
                let dirs: &[_] = match direction {
                    2 => &[DOWN],
                    3 => &[DOWN, RIGHT],
                    6 => &[RIGHT],
                    _ => unreachable!(),
                };
                let press = (index + 1 == directions.len()).then_some(button);
                script.extend([input(0, dirs, press); 3]);
            }
            script.extend([idle(); 160]);
            let observed = trace(l, base.clone(), 0, &script, name, &mut errors);
            assert!(observed.contains(&expected_move), "{name} starts");
            let opponent = trace(l, base.clone(), 1, &script, name, &mut errors);
            assert!(
                opponent.iter().any(|m| (0x30..=0x3f).contains(m)),
                "{name} lands"
            );
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn lee_jump_startup_and_grounded_lunges_do_not_sink() {
    let Some(l) = local() else { return };
    let mut errors = Vec::new();
    for (fighter, name) in [
        (TournamentFighter::Lee, "ZESSHOUOHKEN"),
        (TournamentFighter::Birdie, "BULL HORN"),
    ] {
        for side in 0..2 {
            let base = at_control(l, fighter, side);
            if fighter == TournamentFighter::Lee {
                let mut script = vec![input(side, &[UP], None); 2];
                script.extend(vec![idle(); 100]);
                let observed = trace(
                    l,
                    base.clone(),
                    side,
                    &script,
                    "Lee neutral jump",
                    &mut errors,
                );
                assert!(observed.contains(&9), "Lee's neutral jump actually starts");
                for button in 0..6 {
                    let mut script = vec![input(side, &[UP], None); 2];
                    script.extend([idle(); 10]);
                    script.extend([input(side, &[], Some(button)); 2]);
                    script.extend([idle(); 65]);
                    let label = format!("Lee neutral jump attack button {button}");
                    let observed = trace(l, base.clone(), side, &script, &label, &mut errors);
                    assert!(observed.iter().any(|m| (0x20..=0x26).contains(m)), "{label} starts");
                }
            }
            let entry = move_list(fighter, RulesetPreset::DefinitiveOverhaul)
                .into_iter()
                .find(|e| e.name == name)
                .unwrap();
            let script: Vec<_> = entry
                .input
                .frames(facing(&base, side), Strength::Heavy)
                .into_iter()
                .map(|p| {
                    let mut i = idle();
                    if side == 0 {
                        i.p1 = p
                    } else {
                        i.p2 = p
                    };
                    i
                })
                .chain(vec![idle(); 140])
                .collect();
            let observed = trace(l, base, side, &script, name, &mut errors);
            assert!(
                observed.contains(&entry.produces),
                "{name} side{side}: registered special actually starts"
            );
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn sagat_air_normals_land_without_sinking() {
    let Some(l) = local() else { return };
    let mut errors = Vec::new();
    for side in 0..2 {
        let base = at_control(l, TournamentFighter::Sagat, side);
        let f = if facing(&base, side) { RIGHT } else { LEFT };
        let b = if facing(&base, side) { LEFT } else { RIGHT };
        for dirs in [vec![UP], vec![UP, f], vec![UP, b]] {
            for button in 0..6 {
                let mut script = vec![input(side, &dirs, None); 2];
                script.extend(vec![idle(); 10]);
                script.extend(vec![input(side, &[], Some(button)); 2]);
                script.extend(vec![idle(); 120]);
                let observed = trace(
                    l,
                    base.clone(),
                    side,
                    &script,
                    &format!("Sagat jump{dirs:?} button{button}"),
                    &mut errors,
                );
                assert!(
                    observed.iter().any(|m| (0x20..=0x26).contains(m)),
                    "Sagat {dirs:?} button{button} side{side}: air normal actually starts"
                );
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn crouching_hit_reactions_do_not_sink_when_restoring_stance() {
    let Some(l) = local() else { return };
    let mut errors = Vec::new();
    for fighter in TournamentFighter::ALL {
        for side in 0..2 {
            let base = close(at_control(l, fighter, side), side);
            // Ken's crouching punches pass above Mike's low crouch. His
            // crouching heavy kick is the verified contact for that control.
            let buttons: &[usize] = if fighter == TournamentFighter::Mike {
                &[5]
            } else {
                &[0, 1, 5]
            };
            for &button in buttons {
                let mut script = vec![input(side, &[DOWN], None); 8];
                let mut hit = input(1 - side, &[DOWN], Some(button));
                if side == 0 {
                    hit.p1.directions[DOWN] = true
                } else {
                    hit.p2.directions[DOWN] = true
                };
                script.extend(vec![hit; 2]);
                script.extend(vec![input(side, &[DOWN], None); 120]);
                let observed = trace(
                    l,
                    base.clone(),
                    side,
                    &script,
                    &format!("{fighter:?} crouching hit by{button}"),
                    &mut errors,
                );
                assert!(
                    observed.iter().any(|m| (0x30..=0x3f).contains(m)),
                    "{fighter:?} side{side}: incoming crouching hit actually connects"
                );
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn early_and_late_bank_air_attacks_keep_their_feet_above_the_floor() {
    let Some(l) = local() else { return };
    let mut errors = Vec::new();
    for fighter in TournamentFighter::ALL.into_iter().filter(|f| !f.is_shoto()) {
        for side in 0..2 {
            let base = at_control(l, fighter, side);
            for delay in [6usize, 18] {
                let mut script = vec![input(side, &[UP], None); 2];
                script.extend(vec![idle(); delay]);
                script.extend(vec![input(side, &[], Some(2)); 2]);
                script.extend(vec![idle(); 120]);
                let label = format!("{fighter:?} anytime HP delay{delay}");
                let observed = trace(l, base.clone(), side, &script, &label, &mut errors);
                assert!(
                    observed.iter().any(|m| (0x20..=0x26).contains(m)),
                    "{label} side{side}: the air attack actually starts"
                );
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn received_throw_lying_and_get_up_follow_catalogue_ground_anchors() {
    let Some(l) = local() else { return };
    let mut errors = Vec::new();
    for fighter in TournamentFighter::ALL {
        let catalogue = l
            .art
            .catalogue(&fighter.name().to_ascii_lowercase())
            .unwrap();
        let thrown = catalogue.move_poses("thrown").unwrap();
        let lying = thrown.last().unwrap();
        let getup = catalogue.move_poses("get-up").unwrap();
        for side in 0..2 {
            let mut s = close(at_control(l, fighter, side), side);
            let other = 1 - side;
            let dir = if facing(&s, other) { RIGHT } else { LEFT };
            let mut saw_lying = false;
            let mut saw_getup = false;
            let mut first_bad = None;
            for tick in 0..160 {
                s.step(if tick < 2 {
                    input(other, &[dir], Some(2))
                } else {
                    idle()
                });
                let read = s.fight().unwrap().fight_read().fighters[side];
                let code = word(&s, side, 0);
                let top = word(&s, side, 4) as i32;
                for pose in std::iter::once(*lying).chain(getup.iter().copied()) {
                    let expected_code = pose.grid.code.wrapping_add(if facing(&s, side) {
                        0
                    } else {
                        u16::from(pose.grid.cols) * if pose.grid.large { 2 } else { 1 }
                    });
                    if code != expected_code {
                        continue;
                    }
                    if pose.name == lying.name {
                        saw_lying = true;
                    }
                    if read.getting_up {
                        saw_getup = true;
                    }
                    if (read.getting_up || pose.name == lying.name)
                        && top != GROUND + i32::from(pose.anchor[1])
                        && first_bad.is_none()
                    {
                        first_bad = Some(format!(
                            "{fighter:?} side{side} tick{tick} {} top{top}, expected{}",
                            pose.name,
                            GROUND + i32::from(pose.anchor[1])
                        ));
                    }
                }
            }
            assert!(
                saw_lying && saw_getup,
                "{fighter:?} side{side}: real throw reached lying={saw_lying}, getup={saw_getup}"
            );
            if let Some(e) = first_bad {
                errors.push(e);
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
