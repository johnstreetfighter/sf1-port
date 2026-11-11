//! Bonus challengers for M4, M5, and M6 (`#117`, `#118`, `#119`).

use sf_engine::bonus::{costumes, BonusCostume, BonusFighter, Bout, Challenger, Side};
use sf_engine::config::RulesetPreset;
use sf_engine::era::EraFamily;
use sf_engine::roster::{CharacterSelectScreen, MenuInput, PlayMode, RosterSettings, TournamentFighter};
use sf_engine::jukebox::Jukebox;

fn filtered(bonus: bool) -> Vec<String> {
    let mut settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    settings.mode = PlayMode::Arcade;
    settings.bonus_challengers = bonus;
    let mut screen = CharacterSelectScreen::open(settings, true);
    screen.apply(MenuInput::Filter);
    screen.rows()
}

#[test]
fn ansatsuken_five_join_when_the_filter_is_on() {
    let hidden = filtered(false);
    assert!(hidden.iter().all(|row| !row.contains("[BONUS '87]")));
    let shown = filtered(true);
    let row = shown.iter().find(|row| row.contains("[BONUS '87]")).unwrap();
    for name in ["Gouken", "Akuma", "Evil Ryu", "Goutetsu", "Sheng Long"] {
        assert!(row.contains(name), "{row}");
    }
    let gouken = BonusFighter::new(Challenger::Gouken, EraFamily::Sf1987, BonusCostume::Default);
    assert!(gouken.special("Forbidden Shoryuken") && gouken.special("Tatsumaki Gorasen"));
    let rules = gouken.sf1_rules().unwrap();
    assert_eq!(rules.hitstun, 26);
    assert!(rules.crouch && rules.jump && rules.block && !rules.boss_lock);
    assert_eq!(gouken.fighter.styles().len(), 8);
    assert_eq!(gouken.fighter.stage(), "Japan Temple Night");
    let akuma = BonusFighter::new(Challenger::Akuma, EraFamily::Sf2, BonusCostume::Default);
    assert!(akuma.special("Zanku Hadoken") && akuma.special("Shun Goku Satsu"));
    assert!(akuma.sf1_rules().is_none());
    let evil = BonusFighter::new(Challenger::EvilRyu, EraFamily::Sf6, BonusCostume::Default);
    assert!(evil.special("Ashura Senkuu") && evil.special("Metsu Shoryuken"));
    let goutetsu = BonusFighter::new(Challenger::Goutetsu, EraFamily::Sf1Plus, BonusCostume::Default);
    assert!(goutetsu.special("Kiai") && goutetsu.special("Counter Throw"));
    let sheng = BonusFighter::new(Challenger::ShengLong, EraFamily::Sf5, BonusCostume::Default);
    assert!(sheng.special("Flying Dragon Fist") && sheng.special("Rising Dragon Kick"));
    for fighter in [Challenger::Gouken, Challenger::Akuma, Challenger::EvilRyu, Challenger::Goutetsu, Challenger::ShengLong] {
        let one = BonusFighter::new(fighter, EraFamily::Sf1987, BonusCostume::Default);
        assert!(!one.tiles.is_empty());
        assert_eq!(std::path::Path::new(&one.theme).extension().and_then(|ext| ext.to_str()), Some("wav"));
        let mut box_ = Jukebox::default();
        box_.set_global(&one.theme);
        assert_eq!(box_.resolve(None, None, None).map(|t| t.file), Some(one.theme.clone()));
    }
    let mut bout = Bout::new(Side::Bonus(Challenger::Akuma), EraFamily::Sf2, Side::Tourney(TournamentFighter::Ryu), EraFamily::Sf1987);
    bout.step(8);
    let snap = bout.snapshot();
    bout.step(8);
    bout.restore(&snap);
    assert_eq!(bout.snapshot(), snap);
}

#[test]
fn final_fight_and_saikyo() {
    let row = filtered(true).into_iter().find(|row| row.contains("[BONUS '87]")).unwrap();
    for name in ["Gou Hibiki", "Dan", "Cody", "Guy", "Haggar"] {
        assert!(row.contains(name), "{row}");
    }
    let gou = BonusFighter::new(Challenger::GouHibiki, EraFamily::Sf1987, BonusCostume::Tengu);
    assert!(gou.special("Haoh Gadoken") && gou.special("Proto Counter"));
    assert_eq!(costumes(Challenger::GouHibiki), &[BonusCostume::Default, BonusCostume::Tengu]);
    let dan = BonusFighter::new(Challenger::Dan, EraFamily::SfZ, BonusCostume::Default);
    assert!(dan.special("Dankukyaku") && dan.special("Legendary Taunt"));
    let cody = BonusFighter::new(Challenger::Cody, EraFamily::Sf5, BonusCostume::Mayor);
    assert!(cody.special("Criminal Upper") && cody.special("Final Destruction"));
    assert!(costumes(Challenger::Cody).contains(&BonusCostume::Prison));
    let guy = BonusFighter::new(Challenger::Guy, EraFamily::Sf3, BonusCostume::Default);
    assert!(guy.special("Hayagake") && guy.special("Bushin Izuna Otoshi"));
    let mut haggar = BonusFighter::new(Challenger::Haggar, EraFamily::Sf4, BonusCostume::Default);
    assert!(haggar.special("Flying Piledriver") && haggar.special("Giant Haggar Press"));
    assert!(haggar.command_throw());
    assert!(!haggar.command_throw());
    assert_eq!(haggar.fighter.stage(), "Wrestling Ring");
    assert_eq!(gou.fighter.stage(), "Thailand Shrine");
}

#[test]
fn nash_bison_and_chun_li_finish_the_twenty_five() {
    let row = filtered(true).into_iter().find(|row| row.contains("[BONUS '87]")).unwrap();
    let names = row.split("  ").filter(|part| !part.is_empty() && !part.contains("BONUS")).count();
    assert_eq!(names, 13);
    for name in ["Charlie", "M. Bison", "Chun-Li"] {
        assert!(row.contains(name), "{row}");
    }
    let nash = BonusFighter::new(Challenger::Nash, EraFamily::Sf5, BonusCostume::Stitched);
    assert!(nash.special("Sonic Boom") && nash.special("Crossfire Blitz"));
    assert!(costumes(Challenger::Nash).contains(&BonusCostume::Stitched));
    assert_eq!(nash.fighter.stage(), "US Harbor");
    let bison = BonusFighter::new(Challenger::Bison, EraFamily::Sf2, BonusCostume::Default);
    assert_eq!(bison.intro(), "Cape");
    assert!(bison.special("Psycho Crusher") && bison.special("Head Press"));
    assert_eq!(bison.fighter.stage(), "Shadaloo Temple");
    let chun = BonusFighter::new(Challenger::ChunLi, EraFamily::Sf6, BonusCostume::Qipao);
    assert!(chun.special("Hyakuretsukyaku") && chun.special("Spinning Bird Kick"));
    assert!(costumes(Challenger::ChunLi).contains(&BonusCostume::Master));
    assert_eq!(chun.fighter.stage(), "China Courtyard");
    let mut bout = Bout::new(Side::Bonus(Challenger::ChunLi), EraFamily::Sf6, Side::Bonus(Challenger::Bison), EraFamily::Sf2);
    let before = bout.snapshot();
    bout.step(4);
    assert_ne!(bout.snapshot(), before);
    bout.restore(&before);
    assert_eq!(bout.snapshot(), before);
}
