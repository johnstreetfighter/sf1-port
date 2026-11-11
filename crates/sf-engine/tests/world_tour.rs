//! 1987 World Tour map, missions, and the anti-grind unlock (`#111`, `O20`).

use sf_engine::config::MatchConfig;
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_engine::world_tour::{catalog_unlocked, countries_on_the_map, Campaign, Country, Mission};

#[test]
fn any_fighter_can_walk_the_five_countries() {
    let countries = countries_on_the_map();
    assert_eq!(countries.len(), 5);
    assert!(countries.contains(&Country::Japan) && countries.contains(&Country::Thailand));
    for fighter in TournamentFighter::ALL {
        let mut tour = Campaign::new(fighter, false);
        assert_eq!(tour.at().country, Country::Japan);
        assert!(tour.travel("usa-footsies"));
        assert!(tour.travel("england-survival"));
        assert!(tour.travel("thailand-boss"));
        assert_eq!(tour.at().mission, Mission::DramaticBoss);
        assert!(!tour.travel("japan-dojo"));
    }
}

#[test]
fn missions_write_match_config_and_rpg_stays_in_the_tour() {
    let mut pure = Campaign::new(TournamentFighter::Ken, false);
    pure.travel("usa-footsies");
    let config = pure.start_match(MatchConfig::definitive());
    assert!(config.tour.no_blocking);
    assert!(!pure.allows_block());
    assert_eq!(pure.outgoing(20), 20);
    pure.win();
    assert_eq!(pure.xp, 0);
    assert!(!pure.equip());
}

#[test]
fn rpg_save_and_the_rest_of_the_modes_stay_unlocked() {
    let mut rpg = Campaign::new(TournamentFighter::Ryu, true);
    assert_eq!(rpg.at().mission, Mission::DojoGauntlet);
    let gauntlet = rpg.start_match(MatchConfig::faithful());
    assert_eq!(gauntlet.tour.survival_waves, 3);
    assert!(rpg.travel("china-showdown"));
    assert!(rpg.start_match(MatchConfig::faithful()).tour.cross_era);
    assert!(rpg.travel("england-survival"));
    assert!(rpg.waves_left >= 3 || rpg.start_match(MatchConfig::faithful()).tour.survival_waves >= 3);
    rpg.win();
    assert!(rpg.equip());
    assert_eq!(rpg.outgoing(20), 22);
    assert_eq!(rpg.incoming(10), 9);
    let loaded = Campaign::load(&rpg.save()).unwrap();
    assert_eq!(loaded.fighter, TournamentFighter::Ryu);
    assert_eq!(loaded.xp, rpg.xp);
    assert_eq!(loaded.node, rpg.node);
    assert!(loaded.perk.is_some());

    let mut time = Campaign::new(TournamentFighter::Sagat, false);
    time.travel("usa-footsies");
    time.travel("england-survival");
    time.travel("usa-time");
    let timed = time.start_match(MatchConfig::faithful());
    assert!(timed.tour.projectile_only);
    assert_eq!(timed.tour.time_attack_frames, 1800);
    assert!(time.allows_attack(true));
    assert!(!time.allows_attack(false));
    let round = MatchConfig::from_toml(&timed.to_toml()).unwrap();
    assert!(round.tour.projectile_only);

    let mut boss = Campaign::new(TournamentFighter::Birdie, true);
    boss.travel("usa-footsies");
    boss.travel("thailand-boss");
    assert!(boss.run_dramatic());

    for mode in [PlayMode::Arcade, PlayMode::Versus, PlayMode::Training, PlayMode::Cpu] {
        assert!(catalog_unlocked(0, mode));
        assert!(catalog_unlocked(999, mode));
    }
    let arcade = MatchConfig::from_preset(sf_engine::config::RulesetPreset::Faithful1987);
    assert!(!arcade.tour.rpg);
}
