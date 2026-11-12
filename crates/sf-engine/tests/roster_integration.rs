//! Bonus challengers and what-if styles in AI and rooms (`#121`, `M8`).

use sf_engine::bonus::{Bout, Challenger, Side};
use sf_engine::config::MatchConfig;
use sf_engine::era::EraFamily;
use sf_engine::front::fight::FightDips;
use sf_engine::netplay::{config_hash, Browser, Link, Room, RoomKind, SyncedMatchRules};
use sf_engine::roster::{PlayMode, StagePick, TournamentFighter};
use sf_engine::utility::{decide, Action, ComboTrial, Difficulty, Spot};
use sf_engine::world_tour::lore_episodes;

fn spot_with(command_throw: bool, teleport: bool) -> Spot {
    Spot {
        distance: 80,
        airborne: false,
        throw_attempt: false,
        can_cancel: true,
        attack_incoming: false,
        blocking: false,
        stocks: 1,
        cancel_ok: false,
        command_throw,
        teleport,
    }
}

fn room(what_if: bool, bonus: bool) -> Room {
    Room {
        code: "room".into(),
        host: "198.51.100.4".into(),
        region: "anywhere".into(),
        ping_ms: 20,
        rollback_frames: 2,
        jitter_ms: 1,
        link: Link::Ethernet,
        kind: RoomKind::Versus,
        spectators: 0,
        preset: sf_engine::config::RulesetPreset::DefinitiveOverhaul,
        in_match: false,
        what_if,
        bonus,
    }
}

#[test]
fn utility_and_trials_cover_what_if_and_bonus_fighters() {
    let throw = decide(PlayMode::Arcade, EraFamily::Sf4, Difficulty::VeryHard, true, 3, spot_with(true, false));
    assert_eq!(throw.action, Action::CommandThrow);
    let blink = decide(PlayMode::Versus, EraFamily::Sf6, Difficulty::VeryHard, true, 3, spot_with(false, true));
    assert_eq!(blink.action, Action::Teleport);
}

#[test]
fn lore_rooms_and_synchronized_rules_agree() {
    assert!(lore_episodes(false).is_empty());
    let episodes = lore_episodes(true);
    assert_eq!(episodes.len(), 5);
    assert_eq!(episodes[0].0, "Ryu & Ken vs. Akuma / Gouken");
    assert_eq!(episodes[3].2, "M. Bison");
    let mut expanded = MatchConfig::definitive();
    let strict = {
        let mut config = MatchConfig::definitive();
        config.roster.what_if_styles = false;
        config.roster.bonus_challengers = false;
        config
    };
    assert_ne!(config_hash(&expanded, 7), config_hash(&strict, 7));
    expanded.roster.what_if_styles = true;
    expanded.roster.bonus_challengers = true;
    let host_rules = SyncedMatchRules::new(expanded, [TournamentFighter::Ryu, TournamentFighter::Ken],
        FightDips::default(), StagePick::Bank(0), 7, 123, 456);
    let guest_rules = SyncedMatchRules::from_bytes(&host_rules.to_bytes())
        .expect("the actual synchronized-rules codec carries the full roster configuration");
    assert_eq!(guest_rules, host_rules);
    assert!(guest_rules.config.roster.what_if_styles && guest_rules.config.roster.bonus_challengers);

    let mut bout = Bout::new(Side::Bonus(Challenger::Akuma), EraFamily::Sf2, Side::Tourney(TournamentFighter::Ken), EraFamily::Sf6);
    bout.step(6);
    let snap = bout.snapshot();
    bout.step(6);
    bout.restore(&snap);
    assert_eq!(bout.snapshot(), snap);

    let rooms = vec![room(false, false), room(true, true)];
    let twelve = Browser { wired_only: false, what_if: Some(false), bonus: Some(false) };
    assert_eq!(twelve.visible(&rooms).len(), 1);
    assert_eq!(Browser::badge(&rooms[0]), "1987 Tournament 12");
    assert_eq!(Browser::badge(&rooms[1]), "25-Fighter / What-If");
    let wide = Browser { wired_only: false, what_if: Some(true), bonus: Some(true) };
    assert_eq!(wide.visible(&rooms).len(), 1);

    let mut trial = ComboTrial::new(TournamentFighter::Lee, EraFamily::Sf3);
    assert!(trial.advance("Normal"));
    assert!(trial.advance("Special"));
    assert!(trial.advance("Super Art"));
    assert!(trial.complete());
}
