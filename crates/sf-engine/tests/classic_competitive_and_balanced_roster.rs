//! Ticket #247 (`SP1`; spec #246, design §5.3 and §5.8): Classic Competitive
//! and the Balanced roster switch.
//!
//! The ruleset preset once called SF1++ Competitive is Classic Competitive
//! everywhere a player or the docs see it, and a settings file that names
//! the old preset still loads as Classic Competitive. Balanced roster is one
//! switch in the match configuration (saved, shown in Settings, carried in
//! the rollback hash) that absorbs the two older rebalance toggles, and the
//! three modern rules (true combos, cancels, throws) are switches beside it,
//! inert until later tickets give them behaviour. Each preset bundles
//! exactly the switches the ticket lists.
//!
//! The configuration, Settings and hash checks need no ROM. The behaviour
//! checks go through the headless game session (Local, inputs in; the fight
//! reader out) and skip without the local ROM set.

use std::path::Path;

use sf_engine::config::{CoreToggles, MatchConfig, RulesetPreset};
use sf_engine::fight_reader::LandedHit;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::pause::short_preset_name;
use sf_engine::front::settings::{rows, Row, Settings, SettingsScreen, Tab};
use sf_engine::front::{FrameInputs, GameSession, Transition};
use sf_engine::netplay::config_hash;
use sf_engine::roster::{BanFormat, MenuInput, PlayMode, TournamentFighter};
use sf_video::callouts::Side;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;

// ---- the name -----------------------------------------------------------------

#[test]
fn classic_competitive_is_the_presets_name_and_saved_key() {
    let p = RulesetPreset::ClassicCompetitive;
    assert_eq!(p.display_name(), "Classic Competitive");
    assert_eq!(p.slug(), "classic_competitive");
    assert!(RulesetPreset::ALL.contains(&p));
    for preset in RulesetPreset::ALL {
        let shown = preset.display_name();
        assert!(!shown.contains("SF1++"), "{shown}");
        assert!(!preset.slug().contains("plus_plus"), "{}", preset.slug());
    }
    // The saved file writes the new key.
    let text = MatchConfig::from_preset(p).to_toml();
    assert!(text.contains("preset = \"classic_competitive\""), "{text}");
    assert!(!text.contains("sf1_plus_plus"), "{text}");
    assert!(!text.to_ascii_lowercase().contains("sf1++"), "{text}");
    // The hotkey cycle keeps its order.
    assert_eq!(RulesetPreset::Faithful1987.next_cycle(), p);
    assert_eq!(p.next_cycle(), RulesetPreset::PureFootsies1987Plus);
}

#[test]
fn the_old_names_still_parse_as_classic_competitive() {
    for name in ["classic_competitive", "Classic Competitive", "sf1_plus_plus_competitive", "SF1++ Competitive", "sf1++ competitive"] {
        assert_eq!(RulesetPreset::from_str_name(name), Ok(RulesetPreset::ClassicCompetitive), "{name}");
    }
    assert!(RulesetPreset::from_str_name("SF1+ Competitive").is_err());
}

/// What the last release wrote for SF1++ Competitive: its saved key, and
/// both older rebalance toggles on.
const OLD_COMPETITIVE_FILE: &str = "\
# sf1-port match configuration (tickets #94 O3..#97 O6)
preset = \"sf1_plus_plus_competitive\"

[toggles]
positive_edge_attacks = true
negative_edge_specials = true
recovery_input_buffer = true
recovery_buffer_window = 4
lenient_command_trackers = true
command_tracker_window = 28
command_release_grace = 8
interruptible_walk_cycle = true
anytime_jump_attacks = true
fast_prejump_crouch = true
standard_high_low_block = true
symmetric_shotos_and_blockable_dp = true
balanced_damage_and_chip = true
chip_death = false
disable_fireball_slowdown = true
real_second_timer_and_draws = true
fair_cpu_and_sagat_scaling = true
cpu_reaction_delay = 8

[roster]
full_cast = true
four_country = true
what_if_styles = false
bonus_challengers = false
secret_intrusions = false
ban = \"standard_tournament\"
mode = \"arcade\"
p1 = \"Ryu\"
p2 = \"Ken\"
country = \"Japan\"

[controls]
p1 = \"classic\"
p2 = \"classic\"
modern_control_damage_penalty = true

[dips]
difficulty = 3
round_time = 150
";

#[test]
fn an_old_settings_file_naming_the_old_preset_loads_as_classic_competitive() {
    let loaded = MatchConfig::from_toml(OLD_COMPETITIVE_FILE).expect("the old file loads");
    assert_eq!(loaded.preset, RulesetPreset::ClassicCompetitive, "not Custom either");
    assert_eq!(loaded.toggles, CoreToggles::for_preset(RulesetPreset::ClassicCompetitive));
    assert!(!loaded.toggles.balanced_roster, "the old toggles do not turn Balanced roster on under a named preset");
    assert_eq!((loaded.difficulty, loaded.round_time), (3, 150), "the rest of the file is read as before");
    let mut expect = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive);
    expect.difficulty = 3;
    expect.round_time = 150;
    // Its `ban = "standard_tournament"` line was written by the last release
    // into every file, so it is not a choice: the ban list is the new default.
    assert_eq!(loaded.roster.ban, BanFormat::Unrestricted);
    assert_eq!(loaded, expect);
    // A bare preset line is enough too, by either old spelling.
    for line in ["preset = \"sf1_plus_plus_competitive\"", "preset = \"SF1++ Competitive\""] {
        let c = MatchConfig::from_toml(line).unwrap();
        assert_eq!(c, MatchConfig::from_preset(RulesetPreset::ClassicCompetitive), "{line}");
    }
    // Saved again, it carries the new key and loads to the same thing.
    let again = MatchConfig::from_toml(&loaded.to_toml()).unwrap();
    assert_eq!(again, loaded);
    assert!(loaded.to_toml().contains("classic_competitive"));
}

#[test]
fn the_older_rebalance_keys_in_an_old_file_follow_the_preset_it_names() {
    // Old files wrote both keys for every preset. Under a named preset the
    // preset's bundle decides, so Pure Footsies (old file: both on) is Pure
    // Footsies with Balanced roster off, never Custom.
    let old_footsies = OLD_COMPETITIVE_FILE
        .replace("sf1_plus_plus_competitive", "pure_footsies_1987_plus")
        .replace("lenient_command_trackers = true", "lenient_command_trackers = false");
    let c = MatchConfig::from_toml(&old_footsies).unwrap();
    assert_eq!(c.preset, RulesetPreset::PureFootsies1987Plus);
    assert!(!c.toggles.balanced_roster);
    // Definitive Overhaul (old file: both on) keeps Balanced roster on.
    let old_definitive = OLD_COMPETITIVE_FILE.replace("sf1_plus_plus_competitive", "definitive_overhaul");
    let c = MatchConfig::from_toml(&old_definitive).unwrap();
    assert_eq!(c.preset, RulesetPreset::DefinitiveOverhaul);
    assert!(c.toggles.balanced_roster);
    // Custom: an old file with either rebalance toggle on is a Custom with
    // Balanced roster on; with both off, off.
    let custom = |sym: bool, dmg: bool| {
        let text = OLD_COMPETITIVE_FILE
            .replace("sf1_plus_plus_competitive", "custom")
            .replace("symmetric_shotos_and_blockable_dp = true", &format!("symmetric_shotos_and_blockable_dp = {sym}"))
            .replace("balanced_damage_and_chip = true", &format!("balanced_damage_and_chip = {dmg}"));
        MatchConfig::from_toml(&text).unwrap()
    };
    for (sym, dmg, expect) in [(true, true, true), (true, false, true), (false, true, true), (false, false, false)] {
        let c = custom(sym, dmg);
        assert_eq!(c.preset, RulesetPreset::Custom, "{sym} {dmg}");
        assert_eq!(c.toggles.balanced_roster, expect, "{sym} {dmg}");
    }
    // The old keys are not written any more, and the new key wins when a
    // file names both.
    let text = MatchConfig::definitive().to_toml();
    assert!(!text.contains("symmetric_shotos") && !text.contains("balanced_damage"), "{text}");
    assert!(text.contains("balanced_roster = true"), "{text}");
    let both = "preset = \"custom\"\n[toggles]\nbalanced_roster = false\nbalanced_damage_and_chip = true\n";
    assert!(!MatchConfig::from_toml(both).unwrap().toggles.balanced_roster, "the new key wins");
}

#[test]
fn a_custom_file_gains_no_modern_rule_it_does_not_name() {
    // A Custom file saved before the modern rules existed names none of them:
    // all three load off, though Custom starts from the Definitive bundle.
    let before = "preset = \"custom\"
[toggles]
positive_edge_attacks = true
balanced_roster = true
";
    let c = MatchConfig::from_toml(before).unwrap();
    assert_eq!(c.preset, RulesetPreset::Custom);
    assert!(c.toggles.balanced_roster, "what it names it keeps");
    assert_eq!(bundle(&c.toggles)[7..], [false, false, false]);
    // The old-format Custom file of the last release too.
    let old = OLD_COMPETITIVE_FILE.replace("sf1_plus_plus_competitive", "custom");
    assert_eq!(bundle(&MatchConfig::from_toml(&old).unwrap().toggles)[7..], [false, false, false]);
    // A Custom file that names any of them keeps what it names; the others are off.
    for (key, expect) in [("true_combos", [true, false, false]), ("cancels", [false, true, false]), ("throws", [false, false, true])] {
        let text = format!("preset = \"custom\"
[toggles]
{key} = true
");
        let c = MatchConfig::from_toml(&text).unwrap();
        assert_eq!(bundle(&c.toggles)[7..], expect, "{key}");
    }
    let text = "preset = \"custom\"
[toggles]
true_combos = false
cancels = true
";
    assert_eq!(bundle(&MatchConfig::from_toml(text).unwrap().toggles)[7..], [false, true, false]);
    // Named presets are not Custom: Definitive Overhaul keeps its bundle.
    let c = MatchConfig::from_toml("preset = \"definitive_overhaul\"
").unwrap();
    assert_eq!(bundle(&c.toggles)[7..], [true, true, true]);
    // A saved Custom round-trips whatever it holds (it names all three).
    let mut all = MatchConfig::from_preset(RulesetPreset::Custom);
    all.toggles.true_combos = true;
    all.toggles.cancels = true;
    all.toggles.throws = true;
    assert_eq!(MatchConfig::from_toml(&all.to_toml()).unwrap(), all);
}

#[test]
fn nothing_a_player_sees_says_sf1_plus_plus_competitive() {
    for preset in RulesetPreset::ALL {
        let screen = SettingsScreen::open(MatchConfig::from_preset(preset), Settings::default());
        let shown = screen.value(Row::Preset);
        assert!(!shown.contains("SF1++"), "{shown}");
        assert!(!short_preset_name(preset).contains("SF1"), "{}", short_preset_name(preset));
        assert!(!MatchConfig::from_preset(preset).summary().contains("SF1++"));
    }
    let screen = SettingsScreen::open(MatchConfig::from_preset(RulesetPreset::ClassicCompetitive), Settings::default());
    assert_eq!(screen.value(Row::Preset), "CLASSIC COMPETITIVE");
}

// ---- the bundles --------------------------------------------------------------

/// The switches the ticket names, in the order it lists them: the six
/// control switches, Balanced roster, then the three modern rules.
fn bundle(t: &CoreToggles) -> [bool; 10] {
    [
        t.positive_edge_attacks,
        t.recovery_input_buffer,
        t.lenient_command_trackers,
        t.interruptible_walk_cycle,
        t.anytime_jump_attacks,
        t.standard_high_low_block,
        t.balanced_roster,
        t.true_combos,
        t.cancels,
        t.throws,
    ]
}

#[test]
fn each_preset_turns_on_exactly_the_switches_the_ticket_lists() {
    let on = true;
    let off = false;
    let cases = [
        // Faithful 1987 turns nothing on.
        (RulesetPreset::Faithful1987, [off, off, off, off, off, off, off, off, off, off]),
        // Classic Competitive: the control switches, nothing else.
        (RulesetPreset::ClassicCompetitive, [on, on, on, on, on, on, off, off, off, off]),
        // Pure Footsies: classic with 1987-strict commands.
        (RulesetPreset::PureFootsies1987Plus, [on, on, off, on, on, on, off, off, off, off]),
        // Definitive Overhaul: everything, Balanced roster and the modern three too.
        (RulesetPreset::DefinitiveOverhaul, [on, on, on, on, on, on, on, on, on, on]),
    ];
    for (preset, expect) in cases {
        let t = CoreToggles::for_preset(preset);
        assert_eq!(bundle(&t), expect, "{preset:?}");
        assert_eq!(MatchConfig::from_preset(preset).toggles, t, "{preset:?}");
    }
    // Custom starts from Definitive Overhaul's switches, as before.
    assert_eq!(CoreToggles::for_preset(RulesetPreset::Custom), CoreToggles::for_preset(RulesetPreset::DefinitiveOverhaul));
    // Faithful 1987: every switch is off, the new ones included.
    let f = CoreToggles::faithful();
    assert!(!f.balanced_roster && !f.true_combos && !f.cancels && !f.throws);
    assert!(!f.chip_death && !f.disable_fireball_slowdown && !f.real_second_timer_and_draws && !f.fair_cpu_and_sagat_scaling);
}

#[test]
fn the_switches_outside_the_ticket_keep_their_values() {
    // Not listed, so not changed: negative-edge specials, fast pre-jump,
    // fireball slowdown, the timer, the fair CPU and chip death.
    for preset in [RulesetPreset::ClassicCompetitive, RulesetPreset::PureFootsies1987Plus, RulesetPreset::DefinitiveOverhaul] {
        let t = CoreToggles::for_preset(preset);
        assert!(t.negative_edge_specials && t.fast_prejump_crouch, "{preset:?}");
        assert!(t.disable_fireball_slowdown && t.real_second_timer_and_draws && t.fair_cpu_and_sagat_scaling, "{preset:?}");
        assert!(!t.chip_death, "{preset:?}");
    }
    // The preset's other rules are untouched by the rename.
    let classic = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive);
    assert!(classic.modern_control_damage_penalty, "the competitive preset keeps its modern-controls penalty");
    assert!(!MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul).modern_control_damage_penalty);
}

#[test]
fn classic_competitive_starts_with_no_ban_list_and_standard_tournament_keeps_its_own_rule() {
    // the maintainer, 2026-11-08: all twelve are pickable on both sides by default.
    let classic = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive);
    assert_eq!(classic.roster.ban, BanFormat::Unrestricted);
    assert!(!classic.toggles.balanced_roster);
    for side in 0..2u8 {
        for fighter in TournamentFighter::ALL {
            assert!(classic.roster.ban.allows(fighter, side, classic.toggles.balanced_roster), "{fighter:?} side {side}");
        }
    }
    // Applying the preset resets the ban list to none.
    let mut c = MatchConfig::definitive();
    c.roster.ban = BanFormat::LowTierNoShoto;
    c.apply_preset(RulesetPreset::ClassicCompetitive);
    assert_eq!(c.roster.ban, BanFormat::Unrestricted);
    // A bare old preset line relied on the preset's default: none.
    let c = MatchConfig::from_toml("preset = \"sf1_plus_plus_competitive\"
").unwrap();
    assert_eq!(c.roster.ban, BanFormat::Unrestricted);
    // A file naming the OLD preset loads with the ban list reset to none,
    // whatever its `ban` line says: the last release wrote that line into
    // every file, so it is not a real choice.
    for ban in ["standard_tournament", "low_tier_no_shoto", "unrestricted"] {
        for name in ["sf1_plus_plus_competitive", "SF1++ Competitive"] {
            let text = OLD_COMPETITIVE_FILE
                .replace("sf1_plus_plus_competitive", name)
                .replace("standard_tournament", ban);
            let c = MatchConfig::from_toml(&text).unwrap();
            assert_eq!((c.preset, c.roster.ban), (RulesetPreset::ClassicCompetitive, BanFormat::Unrestricted), "{name} {ban}");
        }
    }
    // A file naming the NEW preset keeps an explicit ban line.
    for (ban, expect) in [
        ("standard_tournament", BanFormat::StandardTournament),
        ("low_tier_no_shoto", BanFormat::LowTierNoShoto),
        ("unrestricted", BanFormat::Unrestricted),
    ] {
        let text = OLD_COMPETITIVE_FILE.replace("sf1_plus_plus_competitive", "classic_competitive").replace("standard_tournament", ban);
        let c = MatchConfig::from_toml(&text).unwrap();
        assert_eq!((c.preset, c.roster.ban), (RulesetPreset::ClassicCompetitive, expect), "{ban}");
    }
    // And a saved Classic Competitive keeps the list the player chose.
    let mut chosen_list = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive);
    chosen_list.roster.ban = BanFormat::StandardTournament;
    assert_eq!(MatchConfig::from_toml(&chosen_list.to_toml()).unwrap(), chosen_list);
    // Standard Tournament, chosen: Sagat banned both sides, Ryu banned for P1
    // only while Balanced roster is off.
    let mut chosen = classic;
    chosen.roster.ban = BanFormat::StandardTournament;
    assert!(!chosen.roster.ban.allows(TournamentFighter::Sagat, 1, false));
    assert!(!chosen.roster.ban.allows(TournamentFighter::Ryu, 0, chosen.toggles.balanced_roster));
    assert!(chosen.roster.ban.allows(TournamentFighter::Ryu, 1, chosen.toggles.balanced_roster));
    assert!(chosen.roster.ban.allows(TournamentFighter::Ryu, 0, true));
}

#[test]
fn applying_a_preset_loads_its_bundle_and_editing_a_switch_makes_it_custom() {
    let mut c = MatchConfig::definitive();
    c.apply_preset(RulesetPreset::ClassicCompetitive);
    assert_eq!(bundle(&c.toggles), [true, true, true, true, true, true, false, false, false, false]);
    for key in ["balanced_roster", "true_combos", "cancels", "throws"] {
        let mut c = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive);
        c.set_field_by_name(key, "true").unwrap();
        assert_eq!(c.preset, RulesetPreset::Custom, "{key}");
        let mut c = MatchConfig::definitive();
        c.set_field_by_name(key, "off").unwrap();
        assert_eq!(c.preset, RulesetPreset::Custom, "{key}");
    }
    // The setters and builders.
    let c = MatchConfig::faithful().with_balanced_roster(true);
    assert!(c.toggles.balanced_roster);
    assert_eq!(c.preset, RulesetPreset::Custom);
    let mut c = MatchConfig::faithful();
    c.set_true_combos(true);
    c.set_cancels(true);
    c.set_throws(true);
    assert_eq!(bundle(&c.toggles)[7..], [true, true, true]);
    // The older two are no longer switches of their own: they are read from
    // an old file (above), never set by name.
    for old in ["balanced_damage_and_chip", "symmetric_shotos_and_blockable_dp"] {
        let mut c = MatchConfig::definitive();
        assert!(c.set_field_by_name(old, "false").is_err(), "{old}");
    }
}

#[test]
fn the_new_switches_are_saved_and_reloaded() {
    for mask in 0u8..16 {
        let mut c = MatchConfig::from_preset(RulesetPreset::Custom);
        c.toggles.balanced_roster = mask & 1 != 0;
        c.toggles.true_combos = mask & 2 != 0;
        c.toggles.cancels = mask & 4 != 0;
        c.toggles.throws = mask & 8 != 0;
        let back = MatchConfig::from_toml(&c.to_toml()).unwrap();
        assert_eq!(back, c, "mask {mask:04b}");
    }
    // A file from before them (no such keys) loads with the preset's value.
    let c = MatchConfig::from_toml("preset = \"definitive_overhaul\"\n").unwrap();
    assert!(c.toggles.balanced_roster && c.toggles.true_combos && c.toggles.cancels && c.toggles.throws);
    // The file names each switch.
    let text = MatchConfig::definitive().to_toml();
    for key in ["balanced_roster = true", "true_combos = true", "cancels = true", "throws = true"] {
        assert!(text.contains(key), "{key}");
    }
    // Files on disk round-trip too.
    let dir = std::env::temp_dir().join(format!("sf1-port-sp1-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("match_config.toml");
    let mut c = MatchConfig::from_preset(RulesetPreset::Custom);
    c.toggles.balanced_roster = false;
    c.toggles.cancels = false;
    c.save(&path).unwrap();
    assert_eq!(MatchConfig::load(&path).unwrap(), c);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn each_new_switch_changes_the_rollback_hash() {
    let base = MatchConfig::from_preset(RulesetPreset::Custom);
    let hash = config_hash(&base, 7);
    let flips: [(&str, fn(&mut MatchConfig)); 4] = [
        ("balanced_roster", |c| c.toggles.balanced_roster = !c.toggles.balanced_roster),
        ("true_combos", |c| c.toggles.true_combos = !c.toggles.true_combos),
        ("cancels", |c| c.toggles.cancels = !c.toggles.cancels),
        ("throws", |c| c.toggles.throws = !c.toggles.throws),
    ];
    let mut seen = vec![hash];
    for (name, flip) in flips {
        let mut c = base;
        flip(&mut c);
        assert_eq!(c.preset, RulesetPreset::Custom, "{name}: the preset alone is not what changes the hash");
        let h = config_hash(&c, 7);
        assert_ne!(h, hash, "{name}");
        assert!(!seen.contains(&h), "{name}: its own hash");
        seen.push(h);
    }
    // Same configuration, same hash.
    assert_eq!(config_hash(&base, 7), hash);
    // The presets differ from each other in the hash.
    let hashes: Vec<u64> = RulesetPreset::ALL.iter().map(|&p| config_hash(&MatchConfig::from_preset(p), 7)).collect();
    for (i, a) in hashes.iter().enumerate() {
        for b in &hashes[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

// ---- Settings ------------------------------------------------------------------

#[test]
fn settings_shows_the_four_switches_beside_the_others_and_not_the_folded_two() {
    let s = SettingsScreen::open(MatchConfig::default(), Settings::default());
    let labels: Vec<&str> = rows(Tab::Gameplay).iter().map(|r| s.label(*r)).collect();
    for label in ["BALANCED ROSTER", "TRUE COMBOS", "CANCELS", "THROWS"] {
        assert!(labels.contains(&label), "{label} in {labels:?}");
    }
    for gone in ["SYMMETRIC SHOTOS", "BALANCED DAMAGE"] {
        assert!(!labels.contains(&gone), "{gone} is folded into BALANCED ROSTER");
    }
    // They sit with the other switches, before DIFFICULTY.
    let at = |l: &str| labels.iter().position(|x| *x == l).unwrap();
    for l in ["BALANCED ROSTER", "TRUE COMBOS", "CANCELS", "THROWS"] {
        assert!(at("HIGH LOW BLOCK") < at(l) && at(l) < at("DIFFICULTY"), "{l}");
    }
    assert_eq!(labels.len(), 1 + 20 + 2 + 1, "PRESET, twenty switches, DIFFICULTY, ROUND TIME, RESET TAB");
}

#[test]
fn the_switches_change_from_settings_and_the_preset_follows() {
    for (label, get) in [
        ("BALANCED ROSTER", (|c: &MatchConfig| c.toggles.balanced_roster) as fn(&MatchConfig) -> bool),
        ("TRUE COMBOS", |c| c.toggles.true_combos),
        ("CANCELS", |c| c.toggles.cancels),
        ("THROWS", |c| c.toggles.throws),
    ] {
        let mut s = SettingsScreen::open(MatchConfig::from_preset(RulesetPreset::ClassicCompetitive), Settings::default());
        while s.label(s.row()) != label {
            assert_eq!(s.apply(MenuInput::Down), Some(Transition::CursorMoved), "{label}: found");
        }
        assert_eq!(s.value(s.row()), "OFF", "{label} is off in Classic Competitive");
        assert!(!get(s.config()));
        assert_eq!(s.apply(MenuInput::Confirm), Some(Transition::ValueChanged), "{label}");
        assert_eq!(s.value(s.row()), "ON", "{label}");
        assert!(get(s.config()), "{label} reaches the configuration");
        assert_eq!(s.config().preset, RulesetPreset::Custom, "{label}");
    }
    // The PRESET row cycles Definitive Overhaul to Classic Competitive with
    // all four off.
    let mut s = SettingsScreen::open(MatchConfig::definitive(), Settings::default());
    s.apply(MenuInput::Right); // Faithful 1987
    s.apply(MenuInput::Right); // Classic Competitive
    assert_eq!(s.value(Row::Preset), "CLASSIC COMPETITIVE");
    assert_eq!(bundle(&s.config().toggles)[6..], [false, false, false, false]);
}

// ---- behaviour, through the game session ---------------------------------------

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

fn tap(s: &mut GameSession, input: FrameInputs) {
    s.step(input);
    s.step(idle());
}

const RIGHT: usize = 0;
const DOWN: usize = 2;

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

/// Power-on under `config`, the Main Menu, Multi Player, Local; P1 Ryu, P2
/// Ken; Japan; the first fight loop.
fn local(a: Assets, config: MatchConfig) -> GameSession {
    let mut s = GameSession::new(a, config);
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.menu.start = true));
    tap(&mut s, with(|i| i.p1.directions[DOWN] = true));
    tap(&mut s, confirm());
    tap(&mut s, confirm());
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    tap(&mut s, with(|i| i.p2.buttons[0] = true));
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
    assert_eq!(s.fight_machine().unwrap().match_config().roster.mode, PlayMode::Versus);
    s
}

/// P1's Hadoken (down, down-forward, forward and LP over nine frames),
/// pressed again every 120 frames. P2's pad stands, or holds back to guard.
fn hadokens(f: u32, p2_guards: bool) -> FrameInputs {
    with(|i| {
        match f % 120 {
            0..=2 => i.p1.directions[DOWN] = true,
            3..=5 => {
                i.p1.directions[DOWN] = true;
                i.p1.directions[RIGHT] = true;
            }
            6..=8 => i.p1.directions[RIGHT] = true,
            _ => {}
        }
        i.p1.buttons[0] = f % 120 == 8;
        i.p2.directions[RIGHT] = p2_guards;
    })
}

/// The first hit P1 lands on P2 in a Local fight under `config`.
fn first_hit(a: Assets, config: MatchConfig, p2_guards: bool) -> LandedHit {
    let mut s = local(a, config);
    for f in 0..900 {
        s.step(hadokens(f, p2_guards));
        assert_eq!(s.screen(), "fight");
        if let Some(hit) = s.fight().unwrap().fight_read().landed_by(Side::P1) {
            assert!(s.fight_machine().unwrap().unrecovered().is_empty());
            return hit;
        }
    }
    panic!("no hit landed");
}

#[test]
fn balanced_roster_on_reproduces_the_rebalanced_special_damage_and_off_gives_the_roms() {
    let Some(_) = assets() else { return };
    let on = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive).with_balanced_roster(true);
    let off = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul).with_balanced_roster(false);
    let hit_on = first_hit(assets().unwrap(), on, false);
    // Definitive Overhaul and a Custom with Balanced roster on play the same.
    let hit_definitive = first_hit(assets().unwrap(), MatchConfig::definitive(), false);
    let hit_off = first_hit(assets().unwrap(), off, false);
    assert_eq!(hit_on.damage, 9, "the rebalanced Hadoken: 9 of 48");
    assert_eq!(hit_definitive.damage, hit_on.damage, "the preset and the switch agree");
    assert_eq!(hit_off.damage, 20, "the ROM's Hadoken: 20 of 48");
    assert!(!hit_on.chip && !hit_off.chip);
}

#[test]
fn balanced_roster_on_normalizes_chip_and_off_gives_the_roms() {
    let Some(_) = assets() else { return };
    let on = MatchConfig::from_preset(RulesetPreset::ClassicCompetitive).with_balanced_roster(true);
    let off = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul).with_balanced_roster(false);
    let chip_on = first_hit(assets().unwrap(), on, true);
    let chip_off = first_hit(assets().unwrap(), off, true);
    assert!(chip_on.chip && chip_off.chip, "a guarded Hadoken is chip: {chip_on:?} {chip_off:?}");
    assert_eq!(chip_on.damage, 1, "rebalanced chip: 1 point on every stage");
    assert!(chip_off.damage > chip_on.damage, "the ROM's chip is the stage's own: {chip_off:?}");
}

/// The first hit P1's standing jab lands on P2 in a Local fight under
/// `config`: P1 walks in, then jabs every 30 frames.
fn first_jab(a: Assets, config: MatchConfig) -> LandedHit {
    let mut s = local(a, config);
    for f in 0..900u32 {
        s.step(with(|i| {
            i.p1.directions[RIGHT] = f < 120;
            i.p1.buttons[0] = f >= 120 && f % 30 == 0;
        }));
        if let Some(hit) = s.fight().unwrap().fight_read().landed_by(Side::P1) {
            return hit;
        }
    }
    panic!("no jab landed");
}

#[test]
fn balanced_roster_on_lifts_the_shoto_jab_into_the_light_band_and_off_gives_the_roms() {
    // #268: Ryu's 1987 jab deals 1 or 2 of 48, under the research's light
    // band (3-5); Balanced roster makes it 3 or 4.
    let Some(_) = assets() else { return };
    let on = first_jab(assets().unwrap(), MatchConfig::from_preset(RulesetPreset::ClassicCompetitive).with_balanced_roster(true));
    let off = first_jab(assets().unwrap(), MatchConfig::from_preset(RulesetPreset::ClassicCompetitive));
    assert!(matches!(on.damage, 3 | 4), "Balanced: {on:?}");
    assert!(matches!(off.damage, 1 | 2), "the ROM's: {off:?}");
    assert_eq!(on.damage, off.damage + 2, "the same depth of hit, two points more");
}
