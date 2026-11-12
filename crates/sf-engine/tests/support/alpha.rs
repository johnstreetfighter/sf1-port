//! ROM-backed local fights with independent Style Card picks.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::era::{
    listed_families, listed_revisions, loadouts, EraFamily, GaugeMode, Loadout, Revision,
};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::menu::MenuItem;
use sf_engine::front::pause::PauseBox;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, Transition};
use sf_engine::moves::{Recognizer, Strength};
use sf_engine::roster::TournamentFighter;
use sf_video::art::{load_art_from, Art, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::{load_assets, Assets};

pub const ROMS: &str = "../../tools/reference/local/roms/sf";
pub const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const ART_PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
const FIGHT_LOOP: u32 = 0x3eafa;
pub const RIGHT: u8 = 1;
pub const LEFT: u8 = 2;
pub const DOWN: u8 = 4;
pub const UP: u8 = 8;
pub const LP: usize = 0;

pub fn pad(directions: u8, button: Option<usize>) -> PlayerInputs {
    let mut input = PlayerInputs {
        directions: [RIGHT, LEFT, DOWN, UP].map(|bit| directions & bit != 0),
        ..PlayerInputs::default()
    };
    if let Some(button) = button {
        input.buttons[button] = true;
    }
    input
}

fn tap(session: &mut GameSession, input: FrameInputs) -> Option<Transition> {
    let transition = session.step(input).transition;
    session.step(FrameInputs::default());
    transition
}

fn choose(session: &mut GameSession, item: MenuItem) {
    for _ in 0..8 {
        if session.menu_item() == Some(item) {
            assert_eq!(
                tap(
                    session,
                    FrameInputs {
                        p1: pad(0, Some(LP)),
                        ..FrameInputs::default()
                    }
                ),
                Some(Transition::Choose(item)),
            );
            return;
        }
        tap(
            session,
            FrameInputs {
                p1: pad(DOWN, None),
                ..FrameInputs::default()
            },
        );
    }
    panic!("{item:?} missing from {}", session.screen());
}

#[allow(dead_code)]
pub fn local_select(gauge_mode: GaugeMode) -> Option<GameSession> {
    local_select_with_p1(gauge_mode, Revision::Sf1Plus, Loadout::Standard)
}

#[allow(dead_code)]
pub fn local_select_with_p1(
    gauge_mode: GaugeMode,
    revision: Revision,
    loadout: Loadout,
) -> Option<GameSession> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROMs assemble");
    let mut config = MatchConfig::definitive();
    config.gauge_mode = gauge_mode;
    Some(select_p1(
        GameSession::new(assets, config),
        TournamentFighter::Ryu,
        revision,
        loadout,
    ))
}

#[allow(dead_code)]
pub fn local_select_with_fighter(
    gauge_mode: GaugeMode,
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
) -> Option<GameSession> {
    Some(select_p1(
        session_with_art(gauge_mode, fighter)?,
        fighter,
        revision,
        loadout,
    ))
}

fn session_with_art(gauge_mode: GaugeMode, fighter: TournamentFighter) -> Option<GameSession> {
    if !Path::new(ROMS).is_dir() || (!fighter.is_shoto() && !Path::new(ART_PACK).is_dir()) {
        eprintln!("skipping: local ROM set or art pack not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROMs assemble");
    let art = (!fighter.is_shoto()).then(|| load_fighter_art(&assets, fighter));
    let mut config = MatchConfig::definitive();
    config.gauge_mode = gauge_mode;
    let mut session = GameSession::new(assets, config);
    if let Some(art) = art {
        session = session
            .with_sprite_bank(&art.bank)
            .with_player_tables(&art.catalogues);
        assert!(
            session
                .data_bank()
                .is_some_and(|bank| bank.table(fighter).is_some()),
            "{fighter:?} bank is bound"
        );
    }
    Some(session)
}

pub fn load_fighter_art(assets: &Assets, fighter: TournamentFighter) -> Art {
    let slug = fighter.name().to_ascii_lowercase();
    let recipe =
        std::fs::read_to_string(format!("../../art/recipes/{slug}.json")).expect("fighter recipe");
    let review = std::fs::read_to_string(format!("../../art/review/{slug}.json"))
        .expect("fighter art review");
    let pack = ArtPack::open(Path::new(ART_PACK)).expect("local art pack opens");
    let art = load_art_from(
        assets,
        &ArtSources {
            recipes: &[&recipe],
            reviews: Some(&[&review]),
            pack: Ok(&pack),
        },
    )
    .expect("local art loads");
    assert!(
        art.catalogue(&slug).is_some(),
        "{fighter:?} catalogue is loaded"
    );
    art
}

fn select_p1(
    mut session: GameSession,
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
) -> GameSession {
    open_menu(&mut session);
    choose(&mut session, MenuItem::MultiPlayer);
    choose(&mut session, MenuItem::Local);
    pick_p1(&mut session, fighter, revision, loadout);
    session
}

fn open_menu(session: &mut GameSession) {
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }
    let mut start = FrameInputs::default();
    start.menu.start = true;
    assert_eq!(tap(session, start), Some(Transition::OpenMainMenu));
}

fn pick_p1(
    session: &mut GameSession,
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
) {
    let index = TournamentFighter::ALL
        .iter()
        .position(|&candidate| candidate == fighter)
        .unwrap();
    for direction in
        std::iter::repeat_n(DOWN, index / 6).chain(std::iter::repeat_n(RIGHT, index % 6))
    {
        tap(
            session,
            FrameInputs {
                p1: pad(direction, None),
                ..FrameInputs::default()
            },
        );
    }
    lock_style(session, false, fighter, revision, loadout);
}

#[allow(dead_code)]
pub fn training_fight_with_fighter(
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
) -> Option<GameSession> {
    let mut session = session_with_art(GaugeMode::EraDefault, fighter)?.with_settings(Settings {
        fill_meters: true,
        ..Settings::default()
    });
    open_menu(&mut session);
    choose(&mut session, MenuItem::SinglePlayer);
    choose(&mut session, MenuItem::Training);
    choose(&mut session, MenuItem::TrainingDummy);
    pick_p1(&mut session, fighter, revision, loadout);

    let confirm = FrameInputs {
        p1: pad(0, Some(LP)),
        ..FrameInputs::default()
    };
    if tap(&mut session, confirm) == Some(Transition::StyleOpened) {
        tap(&mut session, confirm);
    }
    wait_for_fight(&mut session);
    assert_eq!(
        session.fight_machine().unwrap().era_fighters()[0]
            .style
            .revision,
        revision
    );
    Some(session)
}

#[allow(dead_code)]
pub fn assert_training_show(session: &mut GameSession, name: &str, slot: u8) {
    let key = |direction, button| FrameInputs {
        p1: pad(direction, button),
        ..FrameInputs::default()
    };
    let mut start = FrameInputs::default();
    start.menu.start = true;
    assert_eq!(tap(session, start), Some(Transition::Paused));
    let moves_row = PauseBox::open(true)
        .items()
        .iter()
        .position(|&item| item == MenuItem::Moves)
        .unwrap();
    for _ in 0..moves_row {
        tap(session, key(DOWN, None));
    }
    tap(session, key(0, Some(LP)));
    assert!(
        (0..48).any(|_| {
            if session.move_list().unwrap().selected().unwrap().name == name {
                true
            } else {
                tap(session, key(DOWN, None));
                false
            }
        }),
        "{name} appears in MOVES"
    );
    tap(session, key(0, Some(LP)));
    tap(session, key(0, Some(LP)));

    let mut shown = false;
    assert!(
        (0..400).any(|_| {
            session.step(FrameInputs::default());
            shown |= session.fight().unwrap().fight_read().fighters[0].move_id == slot;
            session.coach().is_some()
        }),
        "{name} Show advances to Try"
    );
    assert!(shown, "Show activates {name} in the fight");
    assert_eq!(session.coach().unwrap().entry().name, name);
}

#[allow(dead_code)]
pub fn assert_training_move_demo(
    session: &mut GameSession,
    name: &str,
    slot: u8,
    needs_contact: bool,
) {
    assert_training_show(session, name, slot);
    let entry = *session.coach().unwrap().entry();
    if needs_contact {
        let guard = entry.recognizer == Recognizer::AlphaCounter;
        assert!(
            (0..240).any(|_| {
                let fight = session.fight().unwrap();
                let mut input = FrameInputs::default();
                if guard && fight.fight_read().fighters[1].attacking() {
                    let positions = fight.machine().diagnostic_telemetry();
                    let away = if positions.p1.center_x <= positions.p2.center_x {
                        LEFT
                    } else {
                        RIGHT
                    };
                    input.p1 = pad(away, None);
                }
                session.step(input);
                let [player, opponent] = session.fight().unwrap().fight_read().fighters;
                if guard { player.blockstun } else { opponent.attacking() }
            }),
            "the counter drill reaches the contact window for {name}"
        );
    }
    let positions = session.fight_machine().unwrap().diagnostic_telemetry();
    let facing_right = positions.p1.center_x <= positions.p2.center_x;
    for p1 in entry.input.frames(facing_right, Strength::Medium) {
        session.step(FrameInputs {
            p1,
            ..FrameInputs::default()
        });
    }
    assert_eq!(
        session.coach().unwrap().successes(),
        1,
        "Try recognizes {name}"
    );
}

fn lock_style(
    session: &mut GameSession,
    p2: bool,
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
) -> Option<Transition> {
    let input = |dirs, button| {
        let pad = pad(dirs, button);
        if p2 {
            FrameInputs {
                p2: pad,
                ..FrameInputs::default()
            }
        } else {
            FrameInputs {
                p1: pad,
                ..FrameInputs::default()
            }
        }
    };
    assert_eq!(
        tap(session, input(0, Some(LP))),
        Some(Transition::StyleOpened)
    );
    let family = revision.family();
    let order = [
        EraFamily::Sf1Plus,
        EraFamily::Sf2,
        EraFamily::SfZ,
        EraFamily::Sf3,
        EraFamily::Sf4,
        EraFamily::Sf5,
        EraFamily::Sf6,
        EraFamily::Sf1987,
    ];
    let available = listed_families(fighter, true);
    let family_steps = order
        .into_iter()
        .filter(|family| available.contains(family))
        .position(|candidate| candidate == family)
        .expect("selectable family");
    for _ in 0..family_steps {
        tap(session, input(RIGHT, None));
    }
    let revisions = listed_revisions(fighter, family, true);
    let revision_index = revisions
        .iter()
        .position(|&candidate| candidate == revision)
        .expect("selectable revision");
    let change_revision = revision_index + 1 < revisions.len();
    if change_revision {
        tap(session, input(DOWN, None));
        for _ in 0..=revision_index {
            tap(session, input(RIGHT, None));
        }
    }
    let loadout_index = loadouts(revision)
        .iter()
        .position(|&candidate| candidate == loadout)
        .expect("selectable loadout");
    if loadout_index != 0 {
        tap(
            session,
            input(if change_revision { DOWN } else { UP }, None),
        );
        for _ in 0..loadout_index {
            tap(session, input(RIGHT, None));
        }
    }
    tap(session, input(0, Some(LP)))
}

pub fn local_fight(select: &GameSession, revision: Revision, loadout: Loadout) -> GameSession {
    let mut session = select.clone();
    assert!(matches!(
        lock_style(
            &mut session,
            true,
            TournamentFighter::Ken,
            revision,
            loadout
        ),
        Some(Transition::Fighters(_, _))
    ));
    wait_for_fight(&mut session);
    let style = session.fight_machine().unwrap().era_fighters()[1].style;
    assert_eq!((style.revision, style.loadout), (revision, loadout));
    session
}

fn wait_for_fight(session: &mut GameSession) {
    for _ in 0..60 {
        if session.screen() == "country select" {
            break;
        }
        session.step(FrameInputs::default());
    }
    assert_eq!(session.screen(), "country select");
    tap(
        session,
        FrameInputs {
            p1: pad(0, Some(LP)),
            ..FrameInputs::default()
        },
    );
    for _ in 0..1500 {
        if session.fight_machine().map(flow_position) == Some(FIGHT_LOOP) {
            for _ in 0..4 {
                session.step(FrameInputs::default());
            }
            return;
        }
        session.step(FrameInputs::default());
    }
    panic!("the native fight loop did not open");
}
