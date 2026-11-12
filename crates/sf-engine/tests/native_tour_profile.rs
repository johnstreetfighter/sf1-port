use sf_engine::era::{EraFamily, Loadout, Revision, StylePick};
use sf_engine::roster::TournamentFighter;
use sf_engine::world_tour::native::{map, node, Perk, Stat, WorldTourProfile};

#[test]
fn branching_routes_open_on_first_clear_and_finale_allows_replays() {
    let mut profile = WorldTourProfile::default();
    let get = |id| node(id).unwrap();
    assert_eq!(map().len(), 18);
    assert_eq!(profile.current_node().id, "jp-1-dojo");
    assert!(!profile.select_node("jp-3-era"));
    for id in [
        "jp-1-dojo",
        "jp-3-era",
        "jp-4-boss",
        "cn-1-wall",
        "cn-2-combo",
        "cn-4-boss",
        "en-1-punk",
        "en-2-footsies",
        "en-4-boss",
        "th-2-era",
    ] {
        assert!(profile.complete(get(id)), "clear unlocked {id}");
    }
    assert!(profile.unlocked_perks().contains(&Perk::VitalSpirit));
    assert!(map().iter().all(|node| profile.is_unlocked(node)));
    let xp = profile.xp();
    assert!(!profile.complete(get("jp-1-dojo")));
    assert_eq!(profile.xp(), xp);
}

#[test]
fn atomic_profile_replacement_round_trips_style_and_refundable_stats() {
    let path =
        std::env::temp_dir().join(format!("sf1-port-native-tour-{}.toml", std::process::id()));
    let mut profile = WorldTourProfile::default();
    profile.save(&path).unwrap();
    profile.complete(node("jp-1-dojo").unwrap());
    profile.complete(node("jp-3-era").unwrap());
    assert!(profile.equip(Some(Perk::VitalSpirit)));
    let xp = profile.xp();
    assert!(profile.adjust(Stat::Attack, true));
    assert!(profile.adjust(Stat::Attack, false));
    assert_eq!(profile.xp(), xp);
    assert!(profile.adjust(Stat::Speed, true));
    profile.rpg = false;
    profile.fighter = TournamentFighter::Ken;
    profile.style = StylePick {
        family: EraFamily::SfZ,
        revision: Revision::Sfz3,
        loadout: Loadout::VIsm,
        ..StylePick::default()
    };
    assert!(profile.select_node("jp-3-era"));
    profile.save(&path).unwrap();
    let (loaded, error) = WorldTourProfile::load_or_default(&path);
    std::fs::remove_file(path).unwrap();
    assert_eq!(error, None);
    assert_eq!(loaded, profile);
}
