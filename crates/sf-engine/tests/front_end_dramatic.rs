//! Local Dramatic setup through the real GameSession screens and fight.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::dramatic::LifebarMode;
use sf_engine::era::Revision;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::FightKind;
use sf_engine::front::pause::PauseBox;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, MenuKeys, Transition};
use sf_engine::roster::{StagePick, TournamentFighter};
use sf_video::background_bank::BackgroundBank;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn tap(session: &mut GameSession, change: impl FnOnce(&mut FrameInputs)) -> FrameOutput {
    let mut input = FrameInputs::default();
    change(&mut input);
    let output = session.step(input);
    session.step(FrameInputs::default());
    output
}

fn confirm(session: &mut GameSession) -> FrameOutput {
    tap(session, |input| input.menu.confirm = true)
}

fn down(session: &mut GameSession) {
    tap(session, |input| input.p1.directions[2] = true);
}

#[test]
fn custom_coop_picks_three_roles_and_the_stage_and_moves_follows_the_pausing_pad() {
    let Some(assets) = assets() else { return };
    let mut bank = BackgroundBank::new();
    bank.add_rom_stages(&assets)
        .expect("the native stages pack");
    let geki = bank
        .listed_stages()
        .iter()
        .find(|stage| stage.owner == "Geki")
        .expect("Geki stage")
        .index;
    let mut session =
        GameSession::new(assets, MatchConfig::definitive()).with_background_bank(bank);
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }

    assert_eq!(
        tap(&mut session, |input| input.menu.start = true).screen,
        "main menu"
    );
    down(&mut session);
    assert_eq!(confirm(&mut session).screen, "multi player");
    tap(&mut session, |input| input.p1.directions[3] = true);
    assert_eq!(session.menu_item(), Some(MenuItem::LocalCoopDramatic));
    assert_eq!(confirm(&mut session).screen, "dramatic setup");

    tap(&mut session, |input| input.p1.directions[1] = true); // Custom wraps from Twin Dragons.
    down(&mut session);
    down(&mut session);
    tap(&mut session, |input| input.p1.directions[0] = true); // Independent.
    down(&mut session);
    tap(&mut session, |input| input.p1.directions[0] = true); // Friendly fire on.
    down(&mut session);
    assert_eq!(confirm(&mut session).screen, "dramatic select");
    assert_eq!(confirm(&mut session).transition, Some(Transition::StyleOpened));
    tap(&mut session, |input| input.p1.directions[0] = true); // SF2 ST.
    assert_eq!(
        confirm(&mut session).transition,
        Some(Transition::Fighter(TournamentFighter::Ryu))
    );
    assert_eq!(
        confirm(&mut session).transition,
        None,
        "Ally 2 belongs to the second pad"
    );
    tap(&mut session, |input| input.p2.directions[0] = true);
    let confirm_ally2 = |session: &mut GameSession| {
        tap(session, |input| {
            input.pads[1] = Some(MenuKeys {
                confirm: true,
                ..MenuKeys::default()
            })
        })
    };
    assert_eq!(confirm_ally2(&mut session).transition, Some(Transition::StyleOpened));
    tap(&mut session, |input| input.p2.directions[1] = true); // SF6.
    let ally2 = confirm_ally2(&mut session);
    assert_eq!(
        ally2.transition,
        Some(Transition::Fighter(TournamentFighter::Ken))
    );
    assert_eq!(confirm(&mut session).transition, Some(Transition::StyleOpened));
    tap(&mut session, |input| input.p1.directions[0] = true);
    tap(&mut session, |input| input.p1.directions[0] = true); // SFZ3.
    assert_eq!(confirm(&mut session).screen, "stage select");
    tap(&mut session, |input| input.p1.directions[0] = true); // Ryu's home cursor starts at Retsu.
    confirm(&mut session);

    let fight = session.fight().expect("the selection starts a native fight");
    let setup = fight.setup();
    assert_eq!(fight.fighter_style(0).unwrap().revision, Revision::St);
    assert_eq!(fight.fighter_style(1).unwrap().revision, Revision::Sf6);
    let rules = setup
        .config
        .dramatic
        .expect("Dramatic rules reach the fight");
    assert_eq!(setup.kind, FightKind::Dramatic { co_op: true });
    assert_eq!(
        (setup.config.roster.p1, rules.ally2, setup.config.roster.p2),
        (
            TournamentFighter::Ryu,
            TournamentFighter::Ken,
            TournamentFighter::Ryu
        )
    );
    assert_eq!(
        (rules.stage, setup.config.roster.stage),
        (TournamentFighter::Geki, Some(StagePick::Bank(geki)))
    );
    assert_eq!(
        (rules.lifebar, rules.friendly_fire, rules.rivalry),
        (LifebarMode::Independent, true, None)
    );
    assert_eq!(rules.styles, [Revision::St, Revision::Sfz3, Revision::Sf6]);

    for _ in 0..2_400 {
        if session
            .fight_machine()
            .is_some_and(|machine| flow_position(machine) == 0x3eafa)
        {
            break;
        }
        session.step(FrameInputs::default());
    }
    assert_eq!(session.fight_machine().map(flow_position), Some(0x3eafa));
    let frame = session.step(FrameInputs::default()).frame;
    let width = sf_video::render::WIDTH;
    assert_eq!(
        &frame.argb[34 * width + 176..34 * width + 226],
        &[0xffd5c49a; 50]
    );
    assert_eq!(
        &frame.argb[35 * width + 177..35 * width + 225],
        &[0xffe6b948; 48]
    );
    let paused = tap(&mut session, |input| {
        input.pads[1] = Some(MenuKeys {
            start: true,
            ..MenuKeys::default()
        })
    });
    assert_eq!(paused.transition, Some(Transition::Paused));
    let moves = PauseBox::open(false)
        .items()
        .iter()
        .position(|&item| item == MenuItem::Moves)
        .expect("a MOVES row");
    for _ in 0..moves {
        down(&mut session);
    }
    assert_eq!(
        confirm(&mut session).transition,
        Some(Transition::Choose(MenuItem::Moves))
    );
    assert_eq!(
        session.move_list().expect("the move list").fighter(),
        TournamentFighter::Ken
    );
    tap(&mut session, |input| input.menu.tab_right = true);
    assert_eq!(
        session
            .move_list()
            .expect("the other ally's move list")
            .fighter(),
        TournamentFighter::Ryu
    );
}

#[test]
fn faithful_menus_skip_dramatic_choices() {
    let Some(assets) = assets() else { return };
    let mut session = GameSession::new(assets, MatchConfig::faithful());
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }
    tap(&mut session, |input| input.menu.start = true);
    assert_eq!(confirm(&mut session).screen, "single player");
    down(&mut session);
    assert_eq!(session.menu_item(), Some(MenuItem::Training));
    down(&mut session);
    assert_eq!(session.menu_item(), Some(MenuItem::Arcade));
    tap(&mut session, |input| input.menu.back = true);
    down(&mut session);
    assert_eq!(confirm(&mut session).screen, "multi player");
    down(&mut session);
    assert_eq!(session.menu_item(), Some(MenuItem::Online));
    down(&mut session);
    assert_eq!(session.menu_item(), Some(MenuItem::Local));
}

#[test]
fn a_preset_boss_requires_a_real_player_table() {
    let Some(assets) = assets() else { return };
    let mut session = GameSession::new(assets, MatchConfig::definitive());
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }
    tap(&mut session, |input| input.menu.start = true);
    assert_eq!(confirm(&mut session).screen, "single player");
    tap(&mut session, |input| input.p1.directions[3] = true);
    assert_eq!(confirm(&mut session).screen, "dramatic setup");
    tap(&mut session, |input| input.p1.directions[3] = true);
    assert_eq!(
        confirm(&mut session).transition,
        Some(Transition::StartDramatic)
    );
    assert_eq!(session.screen(), "dramatic setup");
    assert!(session.fight().is_none());
}
