//! Ticket #221 (EH5): the combo counter at the front end's fight-screen
//! seam.
//!
//! A real 1987 fight has no true combos: a hit defender is invulnerable
//! (entity +0xf bit 3) for his whole hit reaction, so no hit lands into
//! hitstun and the counter stays silent (user, 2026-11-04). True combos
//! (SP17) change that under Definitive Overhaul; `tests/true_combos.rs`
//! counts a real two-hit combo. The counter's behaviour is driven here with
//! fed fight reads (`GameSession::feed_fight_reads`), which stand in for the
//! reader's on real Local, Training and Arcade fights; the machine never
//! sees them. One test runs a real hit exchange under the classic rules: it
//! shows no count, and the machine's RAM is the same as a bare machine's
//! given the same inputs. Local ROMs only; each test skips without them.

#[path = "support/select.rs"]
mod select;

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight_reader::{FightRead, FighterRead, LandedHit, COMBO_LINGER_FRAMES};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::pause::TrainingBox;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_video::callouts::{regions, ComboCount, Side};
use sf_video::compositor::{ink, small_glyph, RomFont};
use sf_video::decode::decode_char_tile;
use sf_video::render::{Frame, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn tap(s: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let out = s.step(input);
    s.step(idle());
    out
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn start() -> FrameInputs {
    with(|i| i.menu.start = true)
}

fn to_fight_loop(s: &mut GameSession) {
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
}

/// A Local fight (P1 Ryu, P2 Ken, Japan) under `preset`, at the first fight loop.
fn local_fight(a: Assets, preset: RulesetPreset, settings: Settings) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::from_preset(preset)).with_settings(settings);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    tap(&mut s, confirm());
    tap(&mut s, confirm());
    select::confirm_fighter(&mut s, with(|i| i.p1.buttons[0] = true), tap);
    select::confirm_fighter(&mut s, with(|i| i.p2.buttons[0] = true), tap);
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    s
}

/// Training against the dummy (P1 Ryu, the dummy Ken), at the first fight loop.
fn training_dummy(a: Assets, settings: Settings) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::default()).with_settings(settings);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    tap(&mut s, confirm()); // Single Player
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    assert_eq!(tap(&mut s, confirm()).screen, "training");
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    assert_eq!(tap(&mut s, confirm()).screen, "select");
    select::confirm_fighter(&mut s, confirm(), tap); // Ryu
    select::confirm_fighter(&mut s, confirm(), GameSession::step); // the dummy
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    to_fight_loop(&mut s);
    s
}

/// Arcade as Ryu from Japan, against the CPU, at the first fight loop.
fn arcade(a: Assets) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::default());
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    assert_eq!(tap(&mut s, confirm()).screen, "single player");
    assert_eq!(tap(&mut s, confirm()).transition, Some(Transition::Choose(MenuItem::Arcade)));
    select::confirm_fighter(&mut s, confirm(), tap); // Ryu
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true)); // Japan
    to_fight_loop(&mut s);
    s
}

// ---- fed reads --------------------------------------------------------------

const STANDING: FighterRead = FighterRead { present: true, health: 48, move_id: 0x01, state: 0x12, blockstun: false, getting_up: false, attack_active: false };
/// A live hit reaction while busy: hitstun.
const STUNNED: FighterRead = FighterRead { present: true, health: 48, move_id: 0x30, state: 0x8a, blockstun: false, getting_up: false, attack_active: false };

/// A script of reads in which `attacker` hits the other side.
struct Script {
    attacker: Side,
    reads: Vec<FightRead>,
}

impl Script {
    fn new(attacker: Side) -> Self {
        Script { attacker, reads: Vec::new() }
    }

    fn frame(&mut self, defender: FighterRead, landed: Option<LandedHit>) -> &mut Self {
        let mut read = FightRead::default();
        read.fighters[self.attacker.index()] = STANDING;
        read.fighters[1 - self.attacker.index()] = defender;
        read.landed[self.attacker.index()] = landed;
        self.reads.push(read);
        self
    }

    /// A hit for `damage` (into hitstun or not), then `stun` frames of hitstun.
    fn hit(&mut self, damage: u16, into_hitstun: bool, stun: usize) -> &mut Self {
        self.frame(STUNNED, Some(LandedHit { damage, into_hitstun, chip: false }));
        for _ in 0..stun {
            self.frame(STUNNED, None);
        }
        self
    }

    /// `frames` with the defender out of hitstun.
    fn free(&mut self, frames: usize) -> &mut Self {
        for _ in 0..frames {
            self.frame(STANDING, None);
        }
        self
    }
}

/// Feed `script` to `s` and step one frame for each read, alongside `plain`
/// (the same session unfed); the counts shown after each frame. While a
/// count is up only `side`'s combo slot differs from the plain frame, and
/// otherwise the frames are the same.
fn run(s: &mut GameSession, plain: &mut GameSession, script: &Script) -> Vec<Option<ComboCount>> {
    s.feed_fight_reads(&script.reads);
    let side = script.attacker;
    let slot = regions::combo_slot(side);
    let mut shown = Vec::new();
    for t in 0..script.reads.len() {
        let a = s.step(idle());
        let b = plain.step(idle());
        let board = s.fight().unwrap().callouts().frame();
        assert_eq!(board.side(Side::BOTH[1 - side.index()]).combo, None, "frame {t}: the other side shows nothing");
        let combo = board.side(side).combo;
        // The script's first hit is also the round's FIRST ATTACK (EH6), a
        // word in the same column under the combo slot.
        let words = board.side(side).words;
        let in_words = |x: usize, y: usize| {
            words.iter().enumerate().any(|(n, w)| w.is_some() && regions::word_slot(side, n).contains(x, y))
        };
        let differ: Vec<usize> = (0..a.frame.argb.len())
            .filter(|&i| a.frame.argb[i] != b.frame.argb[i] && !in_words(i % WIDTH, i / WIDTH))
            .collect();
        if combo.is_some() {
            assert!(!differ.is_empty(), "frame {t}: the count is drawn");
            for i in differ {
                assert!(slot.contains(i % WIDTH, i / WIDTH), "frame {t}: drawn outside the combo slot at {i}");
            }
        } else {
            assert!(differ.is_empty(), "frame {t}: nothing drawn without a count");
        }
        assert_eq!(
            s.fight_machine().unwrap().save_snapshot(),
            plain.fight_machine().unwrap().save_snapshot(),
            "frame {t}: fed reads never reach the machine"
        );
        shown.push(combo);
    }
    shown
}

fn hits(shown: &[Option<ComboCount>]) -> Vec<u16> {
    shown.iter().map(|c| c.map_or(0, |c| c.hits)).collect()
}

/// Whether `text` is lettered in PLAYER1 gold from pixel (`x`, `y`).
fn gold_text_at(frame: &Frame, a: &Assets, x: usize, y: usize, text: &str) -> bool {
    text.chars().enumerate().all(|(i, ch)| {
        let Some(code) = small_glyph(ch) else { return true };
        let tile = decode_char_tile(&a.gfx4, code as usize);
        (0..8).all(|ty| {
            (0..8).all(|tx| {
                let px = frame.argb[(y + ty) * WIDTH + x + 8 * i + tx];
                tile[ty][tx] == 3 || px == sf_video::callouts::LETTER_PENS[tile[ty][tx] as usize]
            })
        })
    })
}

#[test]
fn a_combo_shows_from_its_second_hit_and_clears_shortly_after() {
    let Some(a) = assets() else { return };
    // HOLD COMBO is Training's: on in the settings, a Local count still clears.
    let settings = Settings { hold_combo: true, ..Settings::default() };
    let mut s = local_fight(a, RulesetPreset::DefinitiveOverhaul, settings);
    let mut plain = s.clone();
    let mut script = Script::new(Side::P1);
    script.free(2).hit(4, false, 5).hit(3, true, 5).hit(6, true, 8);
    let ended = script.reads.len();
    script.free(COMBO_LINGER_FRAMES as usize + 10);
    let shown = run(&mut s, &mut plain, &script);
    let got = hits(&shown);
    assert_eq!(got[..8], [0; 8], "one hit shows nothing");
    assert_eq!(got[8..14], [2; 6], "the count from the second hit");
    assert_eq!(got[14..ended], [3; 9]);
    assert!(shown.iter().flatten().all(|c| c.damage.is_none()), "no damage outside Training");
    // The combo ends as the defender leaves hitstun; the count stays a moment.
    let linger = COMBO_LINGER_FRAMES as usize;
    assert!(got[ended..ended + linger].iter().all(|&h| h == 3), "the count stays up after the combo");
    assert!(got[ended + linger..].iter().all(|&h| h == 0), "then clears");
}

#[test]
fn single_hits_show_nothing_and_hits_after_hitstun_start_a_new_count() {
    let Some(a) = assets() else { return };
    let mut s = local_fight(a, RulesetPreset::DefinitiveOverhaul, Settings::default());
    let mut plain = s.clone();
    let mut script = Script::new(Side::P1);
    // Two single hits, each after the last one's hitstun ended.
    script.hit(4, false, 10).free(5).hit(4, false, 10).free(5);
    // A two-hit combo, its hitstun ends, then a hit and another combo.
    script.hit(2, false, 3).hit(2, true, 3).free(2).hit(2, false, 3).hit(2, true, 3).free(4);
    let shown = run(&mut s, &mut plain, &script);
    let got = hits(&shown);
    assert!(got[..32].iter().all(|&h| h == 0), "single hits never show a count");
    assert!(got.iter().all(|&h| h <= 2), "hits after hitstun ends start a new count: {got:?}");
    assert_eq!(got[36], 2);
    assert_eq!(got[got.len() - 1], 2, "the new combo's own count");
}

#[test]
fn a_cpu_sides_combo_is_counted_in_arcade() {
    let Some(a) = assets() else { return };
    let mut s = arcade(a);
    // The reader reads the CPU's structure for the P2 side in a one-player game.
    s.step(idle());
    let real = *s.fight().unwrap().fight_read().fighter(Side::P2);
    assert!(real.present && real.health > 0, "the CPU side is read: {real:?}");
    let mut plain = s.clone();
    let mut script = Script::new(Side::P2);
    script.hit(5, false, 3).hit(5, true, 3).free(3);
    let shown = run(&mut s, &mut plain, &script);
    assert_eq!(shown[4], Some(ComboCount { hits: 2, damage: None }), "counted as a human side, no damage outside Training");
}

#[test]
fn training_shows_the_combos_damage_under_the_count() {
    let Some(a) = assets() else { return };
    let font_assets = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = training_dummy(a, Settings::default());
    let mut plain = s.clone();
    let mut script = Script::new(Side::P1);
    script.hit(6, false, 3).hit(4, true, 3);
    let shown = run(&mut s, &mut plain, &script);
    assert_eq!(shown[4], Some(ComboCount { hits: 2, damage: Some(10) }));
    let frame = s.step(idle()).frame;
    let slot = regions::combo_slot(Side::P1);
    let line = sf_video::callouts::damage_line(10);
    assert!(gold_text_at(&frame, &font_assets, slot.x, slot.y + sf_video::callouts::COMBO_HITS_LINE, "HITS"));
    assert!(gold_text_at(&frame, &font_assets, slot.x, slot.y + sf_video::callouts::COMBO_DAMAGE_LINE, &line));
}

/// Whether `text` is drawn from text cell (`col`, `row`) in colour `color`.
fn text_at(frame: &Frame, a: &Assets, col: usize, row: usize, text: &str, color: u8) -> bool {
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    text.chars().enumerate().all(|(i, ch)| {
        let Some(code) = small_glyph(ch) else { return true };
        let tile = decode_char_tile(&a.gfx4, code as usize);
        (0..8).all(|ty| {
            (0..8).all(|tx| {
                let px = frame.argb[(row * 8 + ty) * WIDTH + (col + i) * 8 + tx];
                tile[ty][tx] == 3 || px == font.pen(color, tile[ty][tx])
            })
        })
    })
}

#[test]
fn the_hold_row_keeps_the_last_count_and_round_trips_through_settings_toml() {
    let Some(a) = assets() else { return };
    let font_assets = load_assets(ROMS, MANIFEST).unwrap();
    let mut s = training_dummy(a, Settings::default());
    assert!(!s.fight().unwrap().hold_combo(), "off by default");
    // The Training settings box (the pause box's TRAINING row), HOLD COMBO
    // under PRESET.
    s.step(start());
    s.step(idle());
    for _ in 0..6 {
        tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    }
    let out = tap(&mut s, confirm());
    let row = TrainingBox::item_row(1);
    assert!(text_at(&out.frame, &font_assets, TrainingBox::label_col(), row, "HOLD COMBO", ink::BLUE));
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    let out = s.step(confirm());
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::HoldCombo)));
    assert!(out.settings_changed, "saved with the settings");
    assert!(s.settings().hold_combo && s.fight().unwrap().hold_combo());
    let out = s.step(idle());
    assert!(text_at(&out.frame, &font_assets, TrainingBox::value_end_col() - 2, row, "ON", ink::YELLOW), "the row reads ON");
    tap(&mut s, start()); // resume
    s.step(idle());

    let mut plain = s.clone();
    let mut script = Script::new(Side::P1);
    script.hit(3, false, 3).hit(3, true, 3).free(COMBO_LINGER_FRAMES as usize + 30);
    // A single hit does not start a combo: the held count stays.
    script.hit(2, false, 3).free(5);
    let before_next = script.reads.len();
    script.hit(5, false, 3).hit(5, true, 3).hit(5, true, 3).free(3);
    let shown = run(&mut s, &mut plain, &script);
    let held = Some(ComboCount { hits: 2, damage: Some(6) });
    assert!(shown[4..before_next].iter().all(|c| *c == held), "held until the next combo");
    assert_eq!(shown[before_next + 4], Some(ComboCount { hits: 2, damage: Some(10) }), "the next combo replaces it");
    assert_eq!(*shown.last().unwrap(), Some(ComboCount { hits: 3, damage: Some(15) }));

    // settings.toml: the row round-trips, and an old file without it loads OFF.
    let text = s.settings().to_toml();
    assert!(text.contains("hold_combo = true"));
    assert_eq!(Settings::from_toml(&text).unwrap(), *s.settings());
    let old: String = text.lines().filter(|l| !l.starts_with("hold_combo")).map(|l| format!("{l}\n")).collect();
    assert!(!Settings::from_toml(&old).unwrap().hold_combo);
    // A Training fight started with the setting on holds from its start.
    let s = training_dummy(load_assets(ROMS, MANIFEST).unwrap(), Settings { hold_combo: true, ..Settings::default() });
    assert!(s.fight().unwrap().hold_combo());
}

#[test]
fn the_faithful_1987_preset_shows_no_counter() {
    let Some(a) = assets() else { return };
    let mut s = local_fight(a, RulesetPreset::Faithful1987, Settings::default());
    let mut plain = s.clone();
    let mut script = Script::new(Side::P1);
    script.hit(4, false, 3).hit(4, true, 3).hit(4, true, 3).free(5);
    s.feed_fight_reads(&script.reads);
    for t in 0..script.reads.len() {
        let a = s.step(idle());
        let b = plain.step(idle());
        assert_eq!(s.fight().unwrap().callouts().frame().side(Side::P1).combo, None, "frame {t}");
        assert!(a.frame.argb == b.frame.argb, "frame {t}: the plain 1987 screen");
    }
}

#[test]
fn a_real_local_hit_exchange_shows_no_count_and_the_reader_leaves_ram_alone() {
    let Some(a) = assets() else { return };
    // The classic rules: without true combos (SP17, `tests/true_combos.rs`
    // has them) no hit lands in hitstun.
    let mut s = local_fight(a, RulesetPreset::ClassicCompetitive, Settings::default());
    // The same machine stepped bare, with the inputs the fight gives it.
    let mut bare = s.fight_machine().unwrap().clone();
    let script = |t: u32| {
        with(|i| {
            let c = t % 60;
            i.p1.directions[RIGHT] = c < 25;
            i.p1.buttons[(t / 60 % 6) as usize] = (30..32).contains(&c);
            i.p2.directions[LEFT] = c < 20;
            i.p2.buttons[(t / 60 % 3) as usize] = (40..42).contains(&c);
        })
    };
    let mut landed = 0;
    for t in 0..900 {
        let input = script(t);
        s.step(input);
        let [r, l, d, u] = input.p1.directions;
        bare.set_p1_direction(r, l, d, u);
        bare.set_p1_buttons(input.p1.buttons);
        let [r, l, d, u] = input.p2.directions;
        bare.set_p2_direction(r, l, d, u);
        bare.set_p2_buttons(input.p2.buttons);
        bare.set_cabinet_inputs(false, false, false, false, false);
        bare.step();
        let m = s.fight_machine().unwrap();
        assert!(m.work_image() == bare.work_image(), "frame {t}: the reader changed RAM");
        let fight = s.fight().unwrap();
        for side in Side::BOTH {
            if let Some(hit) = fight.fight_read().landed_by(side) {
                landed += 1;
                assert!(!hit.into_hitstun, "frame {t}: the 1987 fight has no true combos");
            }
            assert_eq!(fight.callouts().frame().side(side).combo, None, "frame {t}: one hit is not a combo");
        }
        if s.screen() != "fight" {
            break;
        }
    }
    assert!(landed >= 2, "the exchange landed hits ({landed})");
}

#[test]
fn a_finished_combo_of_five_hits_or_more_is_announced_and_a_shorter_one_is_not() {
    use sf_video::announcer::AnnouncerLine;
    let Some(a) = assets() else { return };
    let mut s = local_fight(a, RulesetPreset::DefinitiveOverhaul, Settings::default());
    let mut script = Script::new(Side::P1);
    // A four-hit combo, then a five-hit one.
    script.free(2).hit(2, false, 4);
    for _ in 0..3 {
        script.hit(2, true, 4);
    }
    script.free(10).hit(2, false, 4);
    for _ in 0..4 {
        script.hit(2, true, 4);
    }
    script.free(10);
    s.feed_fight_reads(&script.reads);
    let mut called = Vec::new();
    for _ in 0..script.reads.len() + 2 {
        called.extend(s.step(idle()).announcements);
    }
    let combos: Vec<_> = called.into_iter().filter(|l| matches!(l, AnnouncerLine::Combo { .. })).collect();
    assert_eq!(combos, vec![AnnouncerLine::Combo { hits: 5 }], "only the five-hit combo is called, once");
}
