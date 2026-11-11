//! 2v1 Dramatic Battle (`#110`, `O19`).

use sf_engine::dramatic::{DramaticBattle, LifebarMode, Pad, SideMode};
use sf_engine::roster::TournamentFighter;
use sf_video::render::{HEIGHT, WIDTH};

fn attack() -> Pad {
    Pad { attack: true, ..Pad::default() }
}

fn right() -> Pad {
    Pad { right: true, ..Pad::default() }
}

fn left() -> Pad {
    Pad { left: true, ..Pad::default() }
}

#[test]
fn three_fighters_and_shots_step_together_inside_the_view() {
    let mut co = DramaticBattle::preset();
    let x0 = [co.x(0), co.x(1), co.x(2)];
    co.step(right(), left(), right());
    assert!(co.x(0) > x0[0]);
    assert!(co.x(1) < x0[1]);
    assert!(co.x(2) > x0[2]);

    let mut shots = DramaticBattle::preset();
    shots.step(attack(), Pad::default(), attack());
    assert!(shots.shot_x(0).is_some());
    assert!(shots.shot_x(2).is_some());
    let first = shots.shot_x(0).unwrap();
    shots.step(Pad::default(), Pad::default(), Pad::default());
    assert!(shots.shot_x(0).unwrap() > first);

    let mut cpu = DramaticBattle::new(
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        TournamentFighter::Mike,
        SideMode::CpuAlly,
        LifebarMode::Independent,
        false,
    );
    let ken = cpu.x(1);
    cpu.step(Pad::default(), left(), Pad::default());
    assert!(cpu.x(1) > ken, "the CPU ally walks toward the boss");

    let mut run = DramaticBattle::preset();
    for _ in 0..80 {
        run.step(left(), Pad::default(), right());
    }
    for i in 0..3 {
        let screen = run.screen_x(i);
        assert!((8..=WIDTH as i32 - 8).contains(&screen), "fighter {i} at {screen}");
    }
    let span = run.x(0).max(run.x(1)).max(run.x(2)) - run.x(0).min(run.x(1)).min(run.x(2));
    assert!(span <= 384, "span {span}");

    let frame = run.frame();
    assert_eq!(frame.argb.len(), WIDTH * HEIGHT);
    assert!(frame.argb.contains(&0xff_d8_48_30));
    assert!(frame.argb.contains(&0xff_e8_c0_40));
    assert!(frame.argb.contains(&0xff_40_78_d0));
}

#[test]
fn lifebars_friendly_fire_and_the_boss_scale() {
    assert_eq!(DramaticBattle::boss_max(), 1600);
    let mut shared = DramaticBattle::preset();
    assert_eq!(shared.health(2), 1600);
    for _ in 0..40 {
        shared.step(Pad::default(), Pad::default(), attack());
    }
    assert!(shared.shared() < 1000);
    assert_eq!(shared.health(0), shared.health(1));
    assert_eq!(shared.health(0), shared.shared());

    let mut split = DramaticBattle::new(
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        TournamentFighter::Sagat,
        SideMode::CoOp,
        LifebarMode::Independent,
        false,
    );
    for _ in 0..40 {
        split.step(Pad::default(), Pad::default(), attack());
    }
    assert!(split.health(0) == 1000 || split.health(1) == 1000);
    assert!(split.health(0) != split.health(1) || split.health(0) < 1000);

    let mut safe = DramaticBattle::preset();
    for _ in 0..30 {
        safe.step(attack(), Pad::default(), Pad::default());
    }
    assert_eq!(safe.health(1), 1000, "friendly fire off ignores the partner");
    assert!(safe.health(2) < 1600);

    let mut rough = DramaticBattle::new(
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        TournamentFighter::Sagat,
        SideMode::CoOp,
        LifebarMode::Independent,
        true,
    );
    for _ in 0..8 {
        rough.step(attack(), Pad::default(), Pad::default());
    }
    assert!(rough.health(1) < 1000, "friendly fire hits the partner");
}
