//! The developer Era HUD fixture uses native fighters, gauges and call-outs.

use std::path::Path;

use sf_engine::era_preview::{EraPreview, PreviewEra, FIXTURES};
use sf_engine::fair_cpu::FIGHT_LOOP;
use sf_engine::front::{cabinet::flow_position, PlayerInputs};
use sf_engine::roster::TournamentFighter;
use sf_video::callouts::{regions, CalloutKind, Side, FLASH_PHASE};
use sf_video::era_hud::{pens, GaugeArt, GaugeRow, GaugeState};
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

fn assets() -> Option<Assets> {
    let roms = "../../tools/reference/local/roms/sf";
    if !Path::new(roms).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(
        load_assets(roms, "../../tools/reference/rom-manifest.json")
            .expect("local ROM set assembles"),
    )
}

fn tick(preview: &mut EraPreview, assets: &Assets) -> Frame {
    let output = preview.step(assets, [PlayerInputs::default(); 2]);
    assert!(output.event.is_none(), "{:?}", output.event);
    output.frame
}

fn fight_ready(preview: &mut EraPreview, assets: &Assets) {
    for _ in 0..1500 {
        if flow_position(preview.fight().machine()) == FIGHT_LOOP {
            for _ in 0..4 {
                tick(preview, assets);
            }
            return;
        }
        tick(preview, assets);
    }
    panic!("the native fight loop did not open");
}

fn pixels(frame: &Frame, rect: regions::Rect) -> impl Iterator<Item = u32> + '_ {
    (rect.y..rect.y + rect.h).flat_map(move |y| {
        frame.argb[y * WIDTH + rect.x..y * WIDTH + rect.x + rect.w]
            .iter()
            .copied()
    })
}

#[test]
fn every_pair_of_eras_runs_under_the_native_hud() {
    let Some(assets) = assets() else { return };
    let mut preview = EraPreview::new(&assets, PreviewEra::Sf1, PreviewEra::Sf1).unwrap();
    for p1 in PreviewEra::ALL {
        for p2 in PreviewEra::ALL {
            preview.reset(&assets, p1, p2).unwrap();
            fight_ready(&mut preview, &assets);
            let hud = preview.hud();
            assert_eq!(hud.p1.is_none(), p1 == PreviewEra::Sf1, "{p1:?} vs {p2:?}");
            assert_eq!(hud.p2.is_none(), p2 == PreviewEra::Sf1, "{p1:?} vs {p2:?}");
            assert_eq!(preview.frame(&assets).argb.len(), WIDTH * HEIGHT);
        }
    }
    assert_eq!(preview.fight().player_fighter(0), TournamentFighter::Ryu);
    assert_eq!(preview.fight().player_fighter(1), TournamentFighter::Ken);
}

#[test]
fn callout_fixtures_draw_words_flash_the_gauge_and_dim_the_stage() {
    let Some(assets) = assets() else { return };
    let mut preview = EraPreview::new(&assets, PreviewEra::Sf6, PreviewEra::Alpha).unwrap();
    fight_ready(&mut preview, &assets);
    let reference = preview.frame(&assets);
    preview.set_gauge_art(GaugeArt::Plate);
    assert!(
        pixels(&reference, regions::gauge_corner(Side::P1)).ne(pixels(
            &preview.frame(&assets),
            regions::gauge_corner(Side::P1)
        ))
    );
    preview
        .reset(&assets, PreviewEra::Sf6, PreviewEra::Alpha)
        .unwrap();
    assert_eq!(preview.fight().gauge_art(), GaugeArt::Plate);
    preview.set_gauge_art(GaugeArt::Ruled);
    fight_ready(&mut preview, &assets);

    for &fixture in &FIXTURES[..6] {
        preview.fixture(Side::P1);
        tick(&mut preview, &assets);
        assert!(preview
            .board()
            .frame()
            .side(Side::P1)
            .words
            .contains(&Some(fixture)));
    }
    let flash = preview.frame(&assets);
    let board = preview.board().frame();
    assert_eq!(board.side(Side::P1).flash, Some(true));
    assert!(board
        .elements()
        .into_iter()
        .filter(|el| el.side == Side::P1)
        .any(|el| pixels(&flash, el.rect).any(|pixel| pixel == pens::GOLD)));
    for _ in 0..FLASH_PHASE {
        tick(&mut preview, &assets);
    }
    assert_eq!(preview.board().frame().side(Side::P1).flash, Some(false));
    let before = preview.frame(&assets);
    assert!(pixels(&flash, regions::gauge_corner(Side::P1))
        .ne(pixels(&before, regions::gauge_corner(Side::P1))));

    preview.fixture(Side::P1);
    preview.fixture(Side::P2);
    let after = tick(&mut preview, &assets);
    let board = preview.board().frame();
    assert!(board.dim);
    assert_eq!(board.side(Side::P1).move_name, Some("SHINKU HADOKEN"));
    assert_eq!(board.side(Side::P2).words[0], Some(CalloutKind::Counter));
    assert!(pixels(&after, regions::move_name_line(Side::P1)).any(|pixel| pixel == pens::GOLD));
    let dimmed_stage_pixels = before
        .argb
        .iter()
        .zip(&after.argb)
        .filter(|&(&bright, &dim)| {
            bright != dim && dim == 0xff00_0000 | ((bright >> 1) & 0x007f_7f7f)
        })
        .count();
    assert!(dimmed_stage_pixels > WIDTH * HEIGHT / 4);
}

#[test]
fn native_drive_input_changes_the_hud_and_burns_out() {
    let Some(assets) = assets() else { return };
    let mut preview = EraPreview::new(&assets, PreviewEra::Sf6, PreviewEra::Sf1).unwrap();
    fight_ready(&mut preview, &assets);
    let idle = PlayerInputs::default();
    let mut impact = idle;
    impact.buttons[2] = true;
    impact.buttons[5] = true;
    for i in 0..6 {
        let output = preview.step(&assets, [impact, idle]);
        assert!(output.event.is_none(), "{:?}", output.event);
        assert_eq!(preview.fight().fight_read().fighters[0].move_id, 0x2f);
        assert_eq!(preview.board().frame().side(Side::P1).greyed, i == 5);
        if i < 5 {
            for _ in 0..40 {
                tick(&mut preview, &assets);
            }
        }
    }
    assert!(preview
        .board()
        .frame()
        .side(Side::P1)
        .words
        .contains(&Some(CalloutKind::Burnout)));
    assert!(matches!(
        preview.hud().p1.unwrap().main,
        GaugeRow::Stock {
            state: GaugeState::Greyed,
            ..
        }
    ));
    for _ in 0..450 {
        tick(&mut preview, &assets);
    }
    assert!(!preview.board().frame().side(Side::P1).greyed);
    assert!(matches!(
        preview.hud().p1.unwrap().main,
        GaugeRow::Stock {
            state: GaugeState::Full,
            full: 6,
            ..
        }
    ));
}

#[test]
fn era_arguments_and_cycle_keep_the_existing_order() {
    assert_eq!(PreviewEra::from_slug("sf6"), Some(PreviewEra::Sf6));
    assert_eq!(PreviewEra::Sf6.next(), PreviewEra::Sf1);
}
