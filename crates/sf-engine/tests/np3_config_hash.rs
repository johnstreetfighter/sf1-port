use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::netplay::config_hash;
use sf_engine::roster::{StagePick, TournamentFighter};
use sf_engine::session_io::ControlScheme;

#[test]
fn config_identity_changes_when_the_recovery_window_changes() {
    let original = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    let mut changed = original;
    changed.toggles.recovery_buffer_window += 1;
    assert_ne!(
        config_hash(&original, 42),
        config_hash(&changed, 42),
        "different numeric gameplay rules cannot share a configuration identity"
    );
}

#[test]
fn config_identity_tracks_native_rules_fighter_selection_and_seed() {
    let original = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    let expected = config_hash(&original, 42);
    let changes: [(&str, fn(&mut MatchConfig)); 7] = [
        ("native difficulty", |c| c.difficulty = c.difficulty % 4 + 1),
        ("native round timer", |c| {
            c.round_time = if c.round_time == 100 { 150 } else { 100 }
        }),
        ("P1 fighter", |c| c.roster.p1 = TournamentFighter::Ken),
        ("P2 fighter", |c| c.roster.p2 = TournamentFighter::Ryu),
        ("command release grace", |c| {
            c.toggles.command_release_grace += 1
        }),
        ("CPU reaction delay", |c| c.toggles.cpu_reaction_delay += 1),
        ("throw rule", |c| {
            c.sf1plus.untechable_sf2_throws = !c.sf1plus.untechable_sf2_throws
        }),
    ];
    for (name, change) in changes {
        let mut changed = original;
        change(&mut changed);
        assert_ne!(
            config_hash(&changed, 42),
            expected,
            "{name} contributes to identity"
        );
    }
    assert_ne!(
        config_hash(&original, 43),
        expected,
        "a different shared seed contributes too"
    );
}

#[test]
fn config_identity_preserves_no_stage_versus_explicit_bank_zero() {
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    config.roster.stage = None;
    let unpicked = config_hash(&config, 42);
    config.roster.stage = Some(StagePick::Bank(0));
    assert_ne!(
        config_hash(&config, 42),
        unpicked,
        "raw configuration identity must not resolve a missing stage"
    );
}

#[test]
fn inert_scheme_choices_stay_local_but_the_gameplay_penalty_is_hashed() {
    let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    let original = config_hash(&config, 42);
    config.p1_scheme = ControlScheme::Modern;
    config.p2_scheme = ControlScheme::Pneumatic;
    assert_eq!(
        config_hash(&config, 42),
        original,
        "currently inert local selectors do not split peers"
    );
    config.modern_control_damage_penalty = !config.modern_control_damage_penalty;
    assert_ne!(
        config_hash(&config, 42),
        original,
        "the synchronized gameplay rule remains authoritative"
    );
}
