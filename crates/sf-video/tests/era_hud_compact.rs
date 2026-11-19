//! Ticket #237 (CG1): the Compact gauge layout in the Ruled gauge art, at
//! the compositor seam. Compact gauges are the same gauges, shorter and
//! without labels, flush in the bottom corners and below the lowest row a
//! cornered fighter is drawn in (row 215, measured across the cast).

use sf_video::compositor::{present, OverhaulScene, PresentMode};
use sf_video::era_hud::{
    pens, EraGauge, EraHud, Fill, GaugeArt, GaugeCharge, GaugeKind, GaugeLayout, GaugePresentation, GaugeRow,
    GaugeState, RowPresentation,
};
use sf_video::render::{Frame, HEIGHT, WIDTH};

const BACKDROP: u32 = 0xff_11_22_33;
/// The first row under every cornered fighter's feet and shadow.
const UNDER_THE_FEET: usize = 216;
/// A compact corner's widest drawing: a 54-pixel bar and its two caps.
const COMPACT_WIDTH: usize = 56;

fn scene(layout: GaugeLayout, hud: EraHud, presentation: Option<GaugePresentation>) -> Frame {
    let blank = Frame { argb: vec![BACKDROP; WIDTH * HEIGHT] };
    present(
        PresentMode::Overhaul,
        &blank,
        &OverhaulScene {
            hud,
            gauge_art: GaugeArt::Ruled,
            gauge_layout: layout,
            gauge_presentation: [presentation; 2],
            ..OverhaulScene::EMPTY
        },
    )
}

fn both(gauge: EraGauge) -> EraHud {
    EraHud { p1: Some(gauge), p2: Some(gauge) }
}

fn compact(gauge: EraGauge) -> Frame {
    scene(GaugeLayout::Compact, both(gauge), None)
}

/// The columns and rows a frame draws on, as (left, right, top, bottom) of
/// its left half.
fn drawn_box(frame: &Frame) -> (usize, usize, usize, usize) {
    let (mut left, mut right, mut top, mut bottom) = (WIDTH, 0, HEIGHT, 0);
    for y in 0..HEIGHT {
        for x in 0..WIDTH / 2 {
            if frame.argb[y * WIDTH + x] != BACKDROP {
                left = left.min(x);
                right = right.max(x);
                top = top.min(y);
                bottom = bottom.max(y);
            }
        }
    }
    (left, right, top, bottom)
}

fn count(frame: &Frame, y: usize, columns: std::ops::Range<usize>, pen: u32) -> usize {
    columns.filter(|&x| frame.argb[y * WIDTH + x] == pen).count()
}

fn two_rows() -> EraGauge {
    EraGauge::two(GaugeRow::stock(2, 0, Fill::of(1, 2)), GaugeRow::stock(5, 3, Fill::EMPTY))
}

#[test]
fn a_compact_gauge_is_flush_in_its_corner_and_under_the_feet() {
    for gauge in [two_rows(), EraGauge::one(GaugeRow::stock(1, 0, Fill::of(1, 2)))] {
        let frame = compact(gauge);
        let (left, right, top, bottom) = drawn_box(&frame);
        assert_eq!(left, 0, "flush to the side");
        assert_eq!(bottom, HEIGHT - 1, "flush to the bottom");
        assert_eq!(right, COMPACT_WIDTH - 1);
        assert!(top >= UNDER_THE_FEET, "the gauge starts at row {top}");
    }
    // Two rows fill the eight rows under the feet exactly.
    assert_eq!(drawn_box(&compact(two_rows())).2, UNDER_THE_FEET);
}

#[test]
fn player_twos_compact_gauge_is_player_ones_mirrored() {
    let frame = compact(two_rows());
    for y in 0..HEIGHT {
        for x in 0..WIDTH / 2 {
            assert_eq!(frame.argb[y * WIDTH + x], frame.argb[y * WIDTH + WIDTH - 1 - x], "({x}, {y})");
        }
    }
}

#[test]
fn a_compact_gauge_carries_no_label() {
    let presentation = GaugePresentation {
        main: RowPresentation::new(GaugeKind::Sf2Super),
        second: Some(RowPresentation::new(GaugeKind::KessenVersatile)),
    };
    let labelled = scene(GaugeLayout::Classic, both(two_rows()), Some(presentation));
    let plain = scene(GaugeLayout::Classic, both(two_rows()), None);
    assert!(drawn_box(&labelled).1 > drawn_box(&plain).1, "a Classic gauge has its label after the bar");
    let (_, right, top, _) = drawn_box(&scene(GaugeLayout::Compact, both(two_rows()), Some(presentation)));
    assert_eq!(right, COMPACT_WIDTH - 1, "nothing is drawn after a Compact bar");
    assert_eq!(top, UNDER_THE_FEET);
}

#[test]
fn a_compact_bar_shows_the_same_proportion_filled() {
    // Half of a 54-pixel bar is 27 lit columns, on both of its fill rows.
    let half = compact(EraGauge::one(GaugeRow::bar(Fill::of(1, 2), Fill::FULL)));
    for y in [HEIGHT - 3, HEIGHT - 2] {
        assert_eq!(count(&half, y, 0..COMPACT_WIDTH, pens::GOLD), 27, "row {y}");
    }
    // One full stock of two and half of the next: 26 columns, then 13 of 26.
    let stocks = compact(EraGauge::one(GaugeRow::stock(2, 1, Fill::of(1, 2))));
    assert_eq!(count(&stocks, HEIGHT - 3, 0..COMPACT_WIDTH, pens::GOLD), 26 + 13);
}

#[test]
fn a_compact_gauge_shows_every_state_as_the_classic_gauge_does() {
    let lit = |layout: GaugeLayout, state: GaugeState| {
        let gauge = EraGauge::one(GaugeRow::bar(Fill::FULL, Fill::FULL).with_state(state));
        let frame = scene(layout, both(gauge), None);
        // The first fill row and the one after it, three columns in.
        let top = match layout {
            GaugeLayout::Classic => HEIGHT - 8 - 8,
            GaugeLayout::Compact => HEIGHT - 4,
        };
        let x = match layout {
            GaugeLayout::Classic => 8 + 3,
            GaugeLayout::Compact => 3,
        };
        [frame.argb[(top + 1) * WIDTH + x], frame.argb[(top + 2) * WIDTH + x]]
    };
    for state in [
        GaugeState::Normal,
        GaugeState::Full,
        GaugeState::Draining,
        GaugeState::Greyed,
        GaugeState::Flashing { lit: true },
        GaugeState::Flashing { lit: false },
    ] {
        assert_eq!(lit(GaugeLayout::Compact, state), lit(GaugeLayout::Classic, state), "{state:?}");
    }
    assert_eq!(lit(GaugeLayout::Compact, GaugeState::Draining), [pens::GOLD_DARK; 2]);
    assert_eq!(lit(GaugeLayout::Compact, GaugeState::Greyed), [pens::GREY_LIGHT; 2]);
}

#[test]
fn a_compact_gauge_plays_the_charge_sweep_and_the_spend_cue() {
    let gauge = EraGauge::one(GaugeRow::stock(2, 1, Fill::EMPTY));
    let steady = RowPresentation::new(GaugeKind::Sf2Super);
    let frame = |row: RowPresentation| {
        scene(GaugeLayout::Compact, both(gauge), Some(GaugePresentation { main: row, second: None }))
    };
    let still = frame(steady);
    // A stock just earned: the sweep crosses the bar, a frame at a time.
    let sweep = |age| frame(RowPresentation {
        ready: Fill::of(1, 2),
        charge: Some(GaugeCharge { from: Fill::EMPTY, to: Fill::of(1, 2), age }),
        ..steady
    });
    assert!(sweep(2).argb != still.argb, "the sweep is drawn");
    assert!(sweep(2).argb != sweep(6).argb, "and it moves");
    // Classic flashes its label cream as the charge begins; Compact
    // shows the cream on the row's rule.
    let rule = (HEIGHT - 4) * WIDTH + 3;
    assert!(sweep(2).argb[rule] != still.argb[rule], "the charge cue is on the rule");
    assert!(sweep(10).argb[rule] == still.argb[rule], "and it ends when the label's would");
    // A stock just spent. Classic flashes its label amber; with no label
    // to flash, Compact shows the amber on the row's rule.
    let spent = frame(RowPresentation { spend_age: Some(2), ..steady });
    assert!(spent.argb != still.argb, "the spend cue is drawn");
    assert!(spent.argb[(HEIGHT - 4) * WIDTH + 3] != still.argb[(HEIGHT - 4) * WIDTH + 3], "on the rule");
    let later = frame(RowPresentation { spend_age: Some(10), ..steady });
    assert!(later.argb == still.argb, "and it ends when the label's would");
    // Every cue stays inside the compact corner.
    for cued in [sweep(2), sweep(6), spent] {
        let (left, right, top, bottom) = drawn_box(&cued);
        assert!(left == 0 && right < COMPACT_WIDTH && top >= UNDER_THE_FEET && bottom == HEIGHT - 1);
    }
}

#[test]
fn the_classic_layout_is_the_default_and_draws_as_before() {
    assert_eq!(GaugeLayout::default(), GaugeLayout::Classic);
    assert_eq!(OverhaulScene::EMPTY.gauge_layout, GaugeLayout::Classic);
    // Classic sits 8 pixels in and 8 up, with a 96-pixel bar and its caps.
    let full = GaugeRow::bar(Fill::FULL, Fill::FULL);
    let (left, right, top, bottom) = drawn_box(&scene(GaugeLayout::Classic, both(EraGauge::two(full, full)), None));
    assert_eq!((left, right, top, bottom), (8, 8 + 97, HEIGHT - 8 - 16, HEIGHT - 8 - 1));
}

#[test]
fn gauge_layout_names_round_trip_and_cycle() {
    for layout in GaugeLayout::ALL {
        assert_eq!(GaugeLayout::from_slug(layout.slug()), Some(layout));
        assert_eq!(layout.next().prev(), layout);
    }
    assert_eq!(GaugeLayout::Classic.label(), "CLASSIC");
    assert_eq!(GaugeLayout::Compact.label(), "COMPACT");
    assert_eq!(GaugeLayout::Classic.next(), GaugeLayout::Compact);
    assert_eq!(GaugeLayout::from_slug("sideways"), None);
}
