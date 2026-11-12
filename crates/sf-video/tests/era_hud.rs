//! Tickets #217 (EH1) and #219 (EH3): the era HUD's gauges in Ruled, Blocks
//! and Plate gauge art, at the compositor seam.
//!
//! Golden frames live in `tests/goldens/era_hud/` and are compared pixel for
//! pixel. Level numbers are the ROM's 8×8 digits, so the golden tests need
//! the local ROM set and skip without it. `SF_BLESS_GOLDENS=1` rewrites the
//! goldens from the current drawing; look at them before committing.

use std::path::{Path, PathBuf};

use sf_video::compositor::{present, OverhaulScene, PresentMode, RomFont};
use sf_video::era_hud::{
    pens, EraGauge, EraHud, Fill, GaugeArt, GaugeRow, GaugeState, CORNER_BOTTOM, CORNER_EDGE, CORNER_HEIGHT,
    CORNER_WIDTH, SECOND_ROW,
};
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const BACKDROP: u32 = 0xff_11_22_33;

/// The lettering of a level row, in local columns from the corner's screen
/// edge: it reads left to right on both sides, so P2 does not mirror it.
fn lettering(art: GaugeArt) -> std::ops::Range<usize> {
    match art {
        GaugeArt::Ruled => 0..8,
        GaugeArt::Blocks => 1..9,
        GaugeArt::Plate => 0..24,
    }
}

/// A golden frame's name: Ruled keeps the names it had.
fn golden_name(art: GaugeArt, case: &str, side: &str) -> String {
    match art {
        GaugeArt::Ruled => format!("{case}_{side}"),
        art => format!("{}_{case}_{side}", art.slug()),
    }
}

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set"))
}

fn blank() -> Frame {
    Frame { argb: vec![BACKDROP; WIDTH * HEIGHT] }
}

fn draw(hud: EraHud, font: Option<&RomFont<'_>>) -> Frame {
    draw_in(GaugeArt::Ruled, hud, font)
}

fn draw_in(gauge_art: GaugeArt, hud: EraHud, font: Option<&RomFont<'_>>) -> Frame {
    present(PresentMode::Overhaul, &blank(), &OverhaulScene { hud, gauge_art, font, ..OverhaulScene::EMPTY })
}

/// Every era's gauge shapes, as the era models report them (the era
/// session tests check the models against these).
fn cases() -> Vec<(&'static str, EraGauge)> {
    vec![
        ("st_filling", EraGauge::one(GaugeRow::stock(1, 0, Fill::of(24, 48)))),
        ("st_stocked", EraGauge::one(GaugeRow::stock(1, 1, Fill::of(48, 48)))),
        ("alpha_level_2", EraGauge::one(GaugeRow::level(2, 3, Fill::of(2, 3)))),
        ("alpha_level_3", EraGauge::one(GaugeRow::level(3, 3, Fill::of(3, 3)))),
        (
            "alpha_custom_combo",
            EraGauge::one(GaugeRow::level(1, 3, Fill::of(45, 90)).with_state(GaugeState::Draining)),
        ),
        (
            "alpha_flashing",
            EraGauge::one(GaugeRow::level(2, 3, Fill::of(2, 3)).with_state(GaugeState::Flashing { lit: true })),
        ),
        ("sf3_art1", EraGauge::one(GaugeRow::level(0, 1, Fill::of(60, 120)).with_length(Fill::of(120, 120)))),
        ("sf3_art2", EraGauge::one(GaugeRow::level(1, 2, Fill::of(20, 80)).with_length(Fill::of(80, 120)))),
        ("sf3_art3", EraGauge::one(GaugeRow::level(3, 3, Fill::FULL).with_length(Fill::of(40, 120)))),
        (
            "sf4_bars_revenge",
            EraGauge::two(GaugeRow::stock(4, 2, Fill::EMPTY), GaugeRow::stock(1, 0, Fill::of(120, 200))),
        ),
        (
            "sf4_full_ultra",
            EraGauge::two(GaugeRow::stock(4, 4, Fill::EMPTY), GaugeRow::stock(1, 1, Fill::of(200, 200))),
        ),
        ("sf5_ex_v_gauge", EraGauge::two(GaugeRow::stock(3, 1, Fill::EMPTY), GaugeRow::stock(2, 1, Fill::of(16, 30)))),
        (
            "sf5_v_trigger",
            EraGauge::two(
                GaugeRow::stock(3, 3, Fill::EMPTY),
                GaugeRow::stock(3, 0, Fill::of(2, 30)).with_state(GaugeState::Draining),
            ),
        ),
        (
            "sf6_drive_half_bar",
            EraGauge::two(GaugeRow::stock(6, 4, Fill::of(1, 2)), GaugeRow::level(1, 3, Fill::of(1, 3))),
        ),
        (
            "sf6_burnout",
            EraGauge::two(
                GaugeRow::stock(6, 0, Fill::of(0, 2)).with_state(GaugeState::Greyed),
                GaugeRow::level(3, 3, Fill::of(3, 3)),
            ),
        ),
    ]
}

fn has_level(gauge: &EraGauge) -> bool {
    std::iter::once(gauge.main).chain(gauge.second).any(|row| matches!(row, GaugeRow::Level { .. }))
}

fn golden_path(name: &str) -> PathBuf {
    Path::new("tests/goldens/era_hud").join(format!("{name}.png"))
}

fn write_png(path: &Path, frame: &Frame) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut encoder = png::Encoder::new(file, WIDTH as u32, HEIGHT as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let rgb: Vec<u8> = frame.argb.iter().flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8]).collect();
    encoder.write_header().unwrap().write_image_data(&rgb).unwrap();
}

fn read_png(path: &Path) -> Vec<u32> {
    let decoder = png::Decoder::new(std::fs::File::open(path).unwrap_or_else(|_| {
        panic!("{} is missing; run with SF_BLESS_GOLDENS=1 and check it", path.display())
    }));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!((info.width as usize, info.height as usize), (WIDTH, HEIGHT));
    buf[..info.buffer_size()]
        .chunks(3)
        .map(|c| 0xff00_0000 | (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32)
        .collect()
}

fn check_golden(name: &str, frame: &Frame) {
    let path = golden_path(name);
    if std::env::var_os("SF_BLESS_GOLDENS").is_some() {
        write_png(&path, frame);
        return;
    }
    let golden = read_png(&path);
    let differ = golden.iter().zip(&frame.argb).filter(|(a, b)| a != b).count();
    assert_eq!(differ, 0, "{name}: {differ} pixels differ from {}", path.display());
}

fn mirrored_x(x: usize) -> usize {
    WIDTH - 1 - x
}

#[test]
fn every_era_gauge_matches_its_golden_frame_for_p1_and_p2_in_every_art() {
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    let top = HEIGHT - CORNER_BOTTOM - CORNER_HEIGHT;
    for art in GaugeArt::ALL {
        let letters = lettering(art);
        for (name, gauge) in cases() {
            let p1 = draw_in(art, EraHud { p1: Some(gauge), p2: None }, Some(&font));
            let p2 = draw_in(art, EraHud { p1: None, p2: Some(gauge) }, Some(&font));
            let label = format!("{} {name}", art.label());
            check_golden(&golden_name(art, name, "p1"), &p1);
            check_golden(&golden_name(art, name, "p2"), &p2);

            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let (a, b) = (p1.argb[y * WIDTH + x], p2.argb[y * WIDTH + mirrored_x(x)]);
                    // Only the P1 corner is drawn, within its two rows.
                    let in_corner = (top..HEIGHT - CORNER_BOTTOM).contains(&y)
                        && (CORNER_EDGE..CORNER_EDGE + CORNER_WIDTH).contains(&x);
                    if !in_corner {
                        assert_eq!(a, BACKDROP, "{label}: P1 drew outside its corner at ({x}, {y})");
                        assert_eq!(b, BACKDROP, "{label}: P2 drew outside its corner at ({}, {y})", mirrored_x(x));
                        continue;
                    }
                    // P2 is P1 mirrored, except that the lettering reads left to right.
                    let lx = x - CORNER_EDGE;
                    // Blocks' bevels are lit from the screen's top left on
                    // both sides, so there the edge pens swap while the shape
                    // mirrors: both sides draw the same pixels.
                    let reshaded = art == GaugeArt::Blocks && a != BACKDROP && b != BACKDROP;
                    if !(has_level(&gauge) && letters.contains(&lx)) && !reshaded {
                        assert_eq!(a, b, "{label}: P2 is not P1 mirrored at ({x}, {y})");
                    }
                }
            }
            if has_level(&gauge) {
                for y in top..HEIGHT - CORNER_BOTTOM {
                    for dx in 0..letters.len() {
                        let a = p1.argb[y * WIDTH + CORNER_EDGE + letters.start + dx];
                        let b = p2.argb[y * WIDTH + WIDTH - CORNER_EDGE - letters.end + dx];
                        assert_eq!(a, b, "{label}: P2's lettering is not P1's at row {y}");
                    }
                }
            }
        }
    }
}

#[test]
fn blocks_bevels_are_lit_from_the_top_left_on_both_sides() {
    let (_, sf4) = cases().into_iter().find(|(name, _)| name.starts_with("sf4")).unwrap();
    let top = HEIGHT - CORNER_BOTTOM - CORNER_HEIGHT;
    for (side, hud) in [("P1", EraHud { p1: Some(sf4), p2: None }), ("P2", EraHud { p1: None, p2: Some(sf4) })] {
        let frame = draw_in(GaugeArt::Blocks, hud, None);
        // A row through the middle of the stock boxes: each box starts with
        // its light edge and ends with its dark edge and shadow, read left to right.
        let row = &frame.argb[(top + 3) * WIDTH..(top + 4) * WIDTH];
        let lights = row.iter().filter(|p| **p == pens::BEVEL_LIGHT).count();
        assert!(lights > 0, "{side}: no bevel light");
        for x in 1..WIDTH {
            if row[x] == pens::BEVEL_LIGHT {
                assert!(row[x - 1] != pens::BEVEL_DARK, "{side}: a light edge right of a dark one at x {x}");
            }
            if row[x] == pens::BEVEL_DARK && x + 1 < WIDTH {
                assert_eq!(row[x + 1], pens::SHADOW, "{side}: a dark edge without its shadow to the right at x {x}");
            }
        }
    }
}

#[test]
fn a_side_with_no_gauge_draws_nothing() {
    let (_, sf6) = cases().into_iter().find(|(name, _)| *name == "sf6_drive_half_bar").unwrap();
    for art in GaugeArt::ALL {
        assert_eq!(draw_in(art, EraHud::NONE, None).argb, blank().argb, "{}", art.label());
        let frame = draw_in(art, EraHud { p1: None, p2: Some(sf6) }, None);
        for y in 0..HEIGHT {
            assert!(frame.argb[y * WIDTH..y * WIDTH + WIDTH / 2].iter().all(|p| *p == BACKDROP));
        }
        assert!(frame.argb.iter().any(|p| *p != BACKDROP), "{}", art.label());
    }
}

#[test]
fn a_continuous_second_row_keeps_its_width_when_drained_and_shrinks_with_its_maximum() {
    let rom = assets();
    let font = rom.as_ref().map(|rom| RomFont::new(&rom.gfx4, &rom.maincpu));
    let y = HEIGHT - CORNER_BOTTOM - SECOND_ROW;
    let occupied = |frame: &Frame| {
        frame.argb[y * WIDTH + CORNER_EDGE..y * WIDTH + CORNER_EDGE + CORNER_WIDTH]
            .iter()
            .filter(|&&pixel| pixel != BACKDROP)
            .count()
    };
    for art in GaugeArt::ALL {
        let render_guard = |fill, length, font: Option<&RomFont<'_>>| {
            let gauge = EraGauge::two(GaugeRow::stock(3, 1, Fill::EMPTY), GaugeRow::bar(fill, length));
            draw_in(art, EraHud { p1: Some(gauge), p2: None }, font)
        };
        let full = render_guard(Fill::FULL, Fill::FULL, None);
        let drained = render_guard(Fill::of(1, 2), Fill::FULL, None);
        let reduced = render_guard(Fill::FULL, Fill::of(1, 2), None);
        let with_font = render_guard(Fill::FULL, Fill::of(1, 2), font.as_ref());
        assert_eq!(occupied(&drained), occupied(&full), "{art:?}");
        assert_ne!(drained.argb, full.argb, "{art:?}");
        assert!(occupied(&reduced) < occupied(&full), "{art:?}");
        assert_eq!(reduced.argb, with_font.argb, "{art:?}");
    }
}

#[test]
fn every_art_stays_in_the_corner_box_without_the_rom_font() {
    let top = HEIGHT - CORNER_BOTTOM - CORNER_HEIGHT;
    let in_box = |x: usize| (CORNER_EDGE..CORNER_EDGE + CORNER_WIDTH).contains(&x);
    for art in GaugeArt::ALL {
        for (name, gauge) in cases() {
            let frame = draw_in(art, EraHud { p1: Some(gauge), p2: Some(gauge) }, None);
            for y in 0..HEIGHT {
                for x in 0..WIDTH {
                    let inside = (top..HEIGHT - CORNER_BOTTOM).contains(&y) && (in_box(x) || in_box(mirrored_x(x)));
                    if !inside {
                        assert_eq!(frame.argb[y * WIDTH + x], BACKDROP, "{} {name}: ({x}, {y})", art.label());
                    }
                }
            }
        }
    }
}

#[test]
fn ruled_gauges_use_the_health_bar_rule_gold_fill_and_shadow() {
    let row = |y_from_bottom: usize| (HEIGHT - CORNER_BOTTOM - y_from_bottom) * WIDTH;
    // ST: one cell, half full. Main row only: rows 208..=215.
    let frame = draw(EraHud { p1: Some(EraGauge::one(GaugeRow::stock(1, 0, Fill::of(24, 48)))), p2: None }, None);
    let (x0, cap) = (CORNER_EDGE + 1, CORNER_EDGE);
    assert_eq!(frame.argb[row(8) + x0], pens::RULE, "top rule");
    assert_eq!(frame.argb[row(7) + cap], pens::RULE, "the cap at the screen edge");
    assert_eq!(frame.argb[row(7) + x0], pens::GOLD, "the fill starts at the screen edge");
    assert_eq!(frame.argb[row(7) + x0 + 47], pens::GOLD);
    assert_eq!(frame.argb[row(7) + x0 + 48], pens::SHADOW, "empty interior");
    assert_eq!(frame.argb[row(2) + x0], pens::RULE, "bottom rule");
    assert_eq!(frame.argb[row(1) + x0], pens::SHADOW, "shadow row");
    assert_eq!(frame.argb[row(1) + cap], BACKDROP, "the cap casts no shadow, as the health bar's");
    assert_eq!(frame.argb[row(9) + x0], BACKDROP, "one row only");

    // A full row lights its top fill row; draining and greyed rows change pens.
    let full = draw(EraHud { p1: Some(EraGauge::one(GaugeRow::stock(1, 1, Fill::EMPTY))), p2: None }, None);
    assert_eq!(full.argb[row(7) + x0], pens::GOLD_LIGHT);
    assert_eq!(full.argb[row(6) + x0], pens::GOLD);
    let draining = GaugeRow::stock(1, 1, Fill::EMPTY).with_state(GaugeState::Draining);
    let draining = draw(EraHud { p1: Some(EraGauge::one(draining)), p2: None }, None);
    assert_eq!(draining.argb[row(6) + x0], pens::GOLD_DARK);
    let greyed = GaugeRow::stock(1, 1, Fill::EMPTY).with_state(GaugeState::Greyed);
    let greyed = draw(EraHud { p1: Some(EraGauge::one(greyed)), p2: None }, None);
    assert_eq!(greyed.argb[row(8) + x0], pens::GREY);
    assert_eq!(greyed.argb[row(6) + x0], pens::GREY_LIGHT);

    // Six cells split by rules: the fourth and a half bar lit.
    let sf6 = EraGauge::one(GaugeRow::stock(6, 4, Fill::of(1, 2)));
    let frame = draw(EraHud { p1: Some(sf6), p2: None }, None);
    let cell = 15;
    let divider = x0 + cell;
    assert_eq!(frame.argb[row(6) + divider], pens::RULE, "cells are split by rules");
    assert_eq!(frame.argb[row(1) + divider], pens::SHADOW);
    let fifth = x0 + 4 * (cell + 1);
    assert_eq!(frame.argb[row(6) + fifth + 6], pens::GOLD, "half of the fifth cell");
    assert_eq!(frame.argb[row(6) + fifth + 7], pens::SHADOW);
}

#[test]
fn level_numbers_need_the_rom_font_and_bars_do_not() {
    let alpha = EraHud { p1: Some(EraGauge::one(GaugeRow::level(2, 3, Fill::of(2, 3)))), p2: None };
    let bare = draw(alpha, None);
    let y = HEIGHT - CORNER_BOTTOM - 4;
    assert!(bare.argb[y * WIDTH + CORNER_EDGE..y * WIDTH + CORNER_EDGE + 8].iter().all(|p| *p == BACKDROP));
    assert_eq!(bare.argb[y * WIDTH + CORNER_EDGE + 11], pens::GOLD, "the bar starts past the number's column");
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    let lettered = draw(alpha, Some(&font));
    let digit: Vec<u32> = (HEIGHT - CORNER_BOTTOM - 8..HEIGHT - CORNER_BOTTOM)
        .flat_map(|y| (0..8).map(move |x| (y, x)))
        .map(|(y, x)| lettered.argb[y * WIDTH + CORNER_EDGE + x])
        .collect();
    assert!(digit.contains(&pens::GOLD) && digit.contains(&pens::GOLD_DARK), "a PLAYER1 gold digit");
}

#[test]
fn era_hud_pens_are_the_rom_palette() {
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    let expected = [
        (pens::RULE, (1, 2)),
        (pens::SHADOW, (1, 0)),
        (pens::GOLD, (4, 2)),
        (pens::GOLD_LIGHT, (4, 0)),
        (pens::GOLD_DARK, (4, 1)),
        (pens::LAVENDER_LIGHT, (5, 0)),
        (pens::LAVENDER_DARK, (5, 1)),
        (pens::GREY, (10, 2)),
        (pens::GREY_LIGHT, (10, 0)),
    ];
    for (pen, (color, index)) in expected {
        assert_eq!(pen, font.pen(color, index), "text colour {color} pen {index}");
    }
    // The round-win marker's bevel is in the ROM's palette table, and the
    // plate's pale top is a text colour.
    let word = |at: usize| u16::from_be_bytes([assets.maincpu[at], assets.maincpu[at + 1]]);
    let rgb = |v: u16| {
        let c = |x: u16| u32::from(x & 0xf) * 0x11;
        0xff00_0000 | (c(v >> 8) << 16) | (c(v >> 4) << 8) | c(v)
    };
    let [light, face, dark] = pens::BEVEL_ROM.map(|at| rgb(word(at)));
    assert_eq!([pens::BEVEL_LIGHT, pens::BEVEL, pens::BEVEL_DARK], [light, face, dark], "the marker bevel");
    assert_eq!(pens::CREAM, font.pen(13, 0), "text colour 13 pen 0");
    // The design's samples from a Local fight frame agree.
    assert_eq!(
        [pens::RULE, pens::SHADOW, pens::GOLD, pens::GOLD_DARK, pens::GOLD_LIGHT],
        [0xff_99_99_ff, 0xff_44_44_44, 0xff_cc_99_00, 0xff_88_55_00, 0xff_ff_cc_77]
    );
}

#[test]
fn faithful_present_ignores_the_era_hud() {
    let (_, gauge) = cases().remove(0);
    let frame = present(
        PresentMode::Faithful,
        &blank(),
        &OverhaulScene { hud: EraHud { p1: Some(gauge), p2: Some(gauge) }, ..OverhaulScene::EMPTY },
    );
    assert_eq!(frame.argb, blank().argb);
}

#[test]
fn blocks_are_bevelled_boxes_lit_gold_when_full() {
    let row = |y_from_bottom: usize| (HEIGHT - CORNER_BOTTOM - y_from_bottom) * WIDTH;
    let blocks = |gauge| draw_in(GaugeArt::Blocks, EraHud { p1: Some(gauge), p2: None }, None);
    let x0 = CORNER_EDGE;
    // Four stocks, two lit: boxes 24 or 25 pixels wide side by side, gold or steel.
    let sf4 = blocks(EraGauge::one(GaugeRow::stock(4, 2, Fill::EMPTY)));
    assert_eq!(sf4.argb[row(8) + x0], pens::GOLD_LIGHT, "a lit box's top edge");
    assert_eq!(sf4.argb[row(7) + x0], pens::GOLD_LIGHT, "and its left edge");
    assert_eq!(sf4.argb[row(7) + x0 + 1], pens::GOLD);
    assert_eq!(sf4.argb[row(2) + x0 + 1], pens::GOLD_DARK, "inside its bottom");
    assert_eq!(sf4.argb[row(1) + x0 + 1], pens::SHADOW, "the shadow row");
    assert_eq!(sf4.argb[row(7) + x0 + 23], pens::GOLD_DARK, "inside its right");
    assert_eq!(sf4.argb[row(7) + x0 + 24], pens::SHADOW, "the shadow column");
    let third = x0 + 50;
    assert_eq!(sf4.argb[row(8) + third], pens::BEVEL_LIGHT, "an unlit box is steel");
    assert_eq!(sf4.argb[row(6) + third + 1], pens::BEVEL);
    assert_eq!(sf4.argb[row(2) + third + 1], pens::BEVEL_DARK);
    assert_eq!(sf4.argb[row(9) + x0], BACKDROP, "one row only");

    // A filling stock lights its box from the screen edge.
    let st = blocks(EraGauge::one(GaugeRow::stock(1, 0, Fill::of(24, 48))));
    assert_eq!(st.argb[row(8) + x0], pens::BEVEL_LIGHT);
    assert_eq!(st.argb[row(6) + x0 + 1], pens::GOLD);
    assert_eq!(st.argb[row(6) + x0 + 47], pens::GOLD);
    assert_eq!(st.argb[row(6) + x0 + 48], pens::BEVEL, "the rest of the box is unlit");

    // A full stock lights the top fill row.
    let full = blocks(EraGauge::one(GaugeRow::stock(1, 1, Fill::EMPTY)));
    assert_eq!(full.argb[row(7) + x0 + 1], pens::GOLD_LIGHT);
    assert_eq!(full.argb[row(6) + x0 + 1], pens::GOLD);

    // A level gauge is a thin ruled line beside a bevelled box for the level.
    let alpha = blocks(EraGauge::one(GaugeRow::level(2, 3, Fill::of(2, 3))));
    assert_eq!(alpha.argb[row(10) + x0], pens::BEVEL_LIGHT, "the level box, ten pixels square");
    assert_eq!(alpha.argb[row(1) + x0 + 9], pens::BEVEL_DARK);
    assert_eq!(alpha.argb[row(9) + x0 + 5], pens::BEVEL, "its face");
    assert_eq!(alpha.argb[row(7) + x0 + 10], pens::RULE, "the line's rule");
    assert_eq!(alpha.argb[row(6) + x0 + 10], pens::GOLD, "its fill");
    assert_eq!(alpha.argb[row(6) + x0 + 10 + 63], pens::GOLD);
    assert_eq!(alpha.argb[row(6) + x0 + 10 + 64], pens::SHADOW, "two thirds lit");
    assert_eq!(alpha.argb[row(4) + x0 + 10], pens::SHADOW, "the line's shadow row");
}

#[test]
fn plates_are_gold_frames_with_a_light_top_a_dark_shadow_and_lettering() {
    let row = |y_from_bottom: usize| (HEIGHT - CORNER_BOTTOM - y_from_bottom) * WIDTH;
    let plate = |gauge| draw_in(GaugeArt::Plate, EraHud { p1: Some(gauge), p2: None }, None);
    let x0 = CORNER_EDGE;
    let sf6 = plate(EraGauge::one(GaugeRow::stock(6, 4, Fill::of(1, 2))));
    assert_eq!(sf6.argb[row(8) + x0], pens::GOLD_LIGHT, "the light top edge");
    assert_eq!(sf6.argb[row(7) + x0], pens::GOLD, "the frame");
    assert_eq!(sf6.argb[row(7) + x0 + 97], pens::GOLD);
    assert_eq!(sf6.argb[row(2) + x0 + 5], pens::GOLD, "the frame under the bar");
    assert_eq!(sf6.argb[row(1) + x0 + 5], pens::GOLD_DARK, "the shadow");
    assert_eq!(sf6.argb[row(5) + x0 + 1], pens::GOLD_LIGHT, "a lit cell");
    // Six cells of 15 (the first 16) split by dark gold; the fifth is half lit.
    assert_eq!(sf6.argb[row(5) + x0 + 1 + 16], pens::GOLD_DARK, "cells are split by dark gold");
    let fifth = x0 + 1 + 65;
    assert_eq!(sf6.argb[row(5) + fifth + 6], pens::GOLD_LIGHT);
    assert_eq!(sf6.argb[row(5) + fifth + 7], pens::SHADOW);
    assert_eq!(sf6.argb[row(9) + x0], BACKDROP, "one row only");

    // A level bar leaves room for LV and the number, and shortens to fit.
    let level = EraGauge::one(GaugeRow::level(2, 3, Fill::of(2, 3)));
    let alpha = plate(level);
    assert_eq!(alpha.argb[row(8) + x0 + 26], pens::GOLD_LIGHT, "the plate starts past the lettering");
    assert_eq!(alpha.argb[row(5) + x0 + 26 + 1], pens::GOLD_LIGHT);
    assert_eq!(alpha.argb[row(8) + x0 + 26 + 81], pens::GOLD_LIGHT, "the plate's far end");
    assert_eq!(alpha.argb[row(8) + x0 + 26 + 82], BACKDROP);
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    let lettered = draw_in(GaugeArt::Plate, EraHud { p1: Some(level), p2: None }, Some(&font));
    let text: Vec<u32> = (HEIGHT - CORNER_BOTTOM - 8..HEIGHT - CORNER_BOTTOM)
        .flat_map(|y| (0..24).map(move |x| (y, x)))
        .map(|(y, x)| lettered.argb[y * WIDTH + CORNER_EDGE + x])
        .collect();
    assert!(text.contains(&pens::GOLD) && text.contains(&pens::GOLD_DARK), "PLAYER1 gold lettering");
}

#[test]
fn gauge_art_names_round_trip_and_cycle() {
    for art in GaugeArt::ALL {
        assert_eq!(GaugeArt::from_slug(art.slug()), Some(art));
        assert_eq!(art.next().prev(), art);
    }
    assert_eq!(GaugeArt::default(), GaugeArt::Ruled);
    assert_eq!(GaugeArt::Plate.next(), GaugeArt::Ruled);
    assert_eq!(GaugeArt::from_slug("neon"), None);
}
