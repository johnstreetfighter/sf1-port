//! Tickets #238 (CG2) and #239 (CG3): the Compact gauge layout in the
//! Blocks and Plate gauge arts, above Training's frame meter, and for the
//! Dramatic Battle ally's gauge. All at the compositor seam, on drawn frames.

use sf_video::compositor::{present, OverhaulScene, PresentMode};
use sf_video::era_hud::{
    lift_above_rows, EraGauge, EraHud, Fill, GaugeArt, GaugeCharge, GaugeKind, GaugeLayout, GaugePresentation,
    GaugeRow, GaugeState, RowPresentation,
};
use sf_video::render::{Frame, HEIGHT, WIDTH};

const BACKDROP: u32 = 0xff_11_22_33;
/// The first row under every cornered fighter's feet and shadow.
const UNDER_THE_FEET: usize = 216;
/// A compact corner's widest drawing without a level number.
const COMPACT_WIDTH: usize = 56;
/// The level number's column beside a compact bar.
const DIGIT_COLUMN: usize = 10;

#[derive(Clone, Copy)]
struct Shot {
    art: GaugeArt,
    layout: GaugeLayout,
    hud: EraHud,
    presentation: Option<GaugePresentation>,
    ally: Option<EraGauge>,
    lift: usize,
}

impl Shot {
    fn of(art: GaugeArt, layout: GaugeLayout, gauge: EraGauge) -> Shot {
        Shot { art, layout, hud: EraHud { p1: Some(gauge), p2: Some(gauge) }, presentation: None, ally: None, lift: 0 }
    }

    fn frame(self) -> Frame {
        let blank = Frame { argb: vec![BACKDROP; WIDTH * HEIGHT] };
        present(
            PresentMode::Overhaul,
            &blank,
            &OverhaulScene {
                hud: self.hud,
                gauge_art: self.art,
                gauge_layout: self.layout,
                gauge_presentation: [self.presentation; 2],
                ally2_gauge: self.ally,
                hud_lift: self.lift,
                ..OverhaulScene::EMPTY
            },
        )
    }
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

fn two_rows() -> EraGauge {
    EraGauge::two(GaugeRow::stock(2, 0, Fill::of(1, 2)), GaugeRow::stock(5, 3, Fill::EMPTY))
}

/// One gauge of each kind of row, and the widest column each may reach.
/// These frames have no ROM font, so a level's number is not drawn and its
/// bar starts past the number's column.
fn every_kind() -> [(EraGauge, usize); 4] {
    let level = GaugeRow::level(2, 3, Fill::of(1, 2));
    [
        (two_rows(), COMPACT_WIDTH - 1),
        (EraGauge::one(GaugeRow::bar(Fill::of(1, 2), Fill::FULL)), COMPACT_WIDTH - 1),
        (EraGauge::one(level), DIGIT_COLUMN + COMPACT_WIDTH - 1),
        (EraGauge::two(GaugeRow::stock(3, 1, Fill::EMPTY), level), DIGIT_COLUMN + COMPACT_WIDTH - 1),
    ]
}

const OTHER_ARTS: [GaugeArt; 2] = [GaugeArt::Blocks, GaugeArt::Plate];

#[test]
fn blocks_and_plate_draw_every_gauge_kind_flush_in_the_corner_under_the_feet() {
    for art in OTHER_ARTS {
        for (i, (gauge, widest)) in every_kind().into_iter().enumerate() {
            let frame = Shot::of(art, GaugeLayout::Compact, gauge).frame();
            let (left, right, top, bottom) = drawn_box(&frame);
            let levelled = widest >= COMPACT_WIDTH;
            assert_eq!(left, if levelled { DIGIT_COLUMN } else { 0 }, "{art:?} {i}: flush to the side");
            assert_eq!(bottom, HEIGHT - 1, "{art:?} {i}: flush to the bottom");
            assert!(top >= UNDER_THE_FEET, "{art:?} {i}: top row {top}");
            assert!(right <= widest, "{art:?} {i}: right column {right}");
            // P2's corner is P1's mirrored in where it draws.
            for y in 0..HEIGHT {
                for x in 0..WIDTH / 2 {
                    let (p1, p2) = (frame.argb[y * WIDTH + x], frame.argb[y * WIDTH + WIDTH - 1 - x]);
                    assert_eq!(p1 == BACKDROP, p2 == BACKDROP, "{art:?} {i}: ({x}, {y})");
                }
            }
        }
        let (_, right, top, _) = drawn_box(&Shot::of(art, GaugeLayout::Compact, two_rows()).frame());
        assert_eq!((right, top), (COMPACT_WIDTH - 1, UNDER_THE_FEET), "{art:?}: two rows fill the corner");
    }
}

#[test]
fn each_art_keeps_its_own_drawing_in_compact() {
    let frames: Vec<Frame> = GaugeArt::ALL
        .into_iter()
        .map(|art| Shot::of(art, GaugeLayout::Compact, two_rows()).frame())
        .collect();
    assert!(frames[0].argb != frames[1].argb, "Blocks is not Ruled");
    assert!(frames[0].argb != frames[2].argb, "Plate is not Ruled");
    assert!(frames[1].argb != frames[2].argb, "Plate is not Blocks");
    for art in OTHER_ARTS {
        let classic = Shot::of(art, GaugeLayout::Classic, two_rows()).frame();
        assert_eq!(drawn_box(&classic).0, 8, "{art:?}: Classic keeps its margin");
        assert_eq!(drawn_box(&classic).3, HEIGHT - 9, "{art:?}: and its rows");
    }
}

#[test]
fn blocks_and_plate_carry_no_label_in_compact() {
    for art in OTHER_ARTS {
        let presentation = GaugePresentation { main: RowPresentation::new(GaugeKind::Sf2Super), second: None };
        let plain = Shot::of(art, GaugeLayout::Compact, two_rows());
        let labelled = Shot { presentation: Some(presentation), ..plain };
        assert_eq!(drawn_box(&labelled.frame()), drawn_box(&plain.frame()), "{art:?}");
        let classic = Shot { layout: GaugeLayout::Classic, ..labelled };
        assert!(drawn_box(&classic.frame()).1 > 8 + 98, "{art:?}: Classic still letters its label");
    }
}

#[test]
fn blocks_and_plate_show_every_state_in_compact() {
    for art in OTHER_ARTS {
        // A fight's gauge always carries its era's presentation, and that
        // is what tells a full row from a lit flash in one fill row.
        let worn = Some(GaugePresentation { main: RowPresentation::new(GaugeKind::Sf2Super), second: None });
        let rows = [GaugeRow::bar(Fill::FULL, Fill::FULL), GaugeRow::stock(2, 2, Fill::EMPTY)];
        let shot = |state: GaugeState| -> Vec<u32> {
            rows.iter()
                .flat_map(|row| {
                    let gauge = EraGauge::one(row.with_state(state));
                    Shot { presentation: worn, ..Shot::of(art, GaugeLayout::Compact, gauge) }.frame().argb
                })
                .collect()
        };
        let states = [
            GaugeState::Normal,
            GaugeState::Full,
            GaugeState::Draining,
            GaugeState::Greyed,
            GaugeState::Flashing { lit: true },
        ];
        let classic = |state: GaugeState| -> Vec<u32> {
            rows.iter()
                .flat_map(|row| {
                    let gauge = EraGauge::one(row.with_state(state));
                    Shot { presentation: worn, ..Shot::of(art, GaugeLayout::Classic, gauge) }.frame().argb
                })
                .collect()
        };
        for (i, a) in states.into_iter().enumerate() {
            for b in states.into_iter().skip(i + 1) {
                // Two states the classic drawing tells apart, compact does too.
                if classic(a) != classic(b) {
                    assert!(shot(a) != shot(b), "{art:?}: {a:?} and {b:?} look alike");
                }
            }
        }
        // The same proportion filled: a half bar lights half as many columns.
        let lit = |fill: Fill| {
            let empty = Shot::of(art, GaugeLayout::Compact, EraGauge::one(GaugeRow::bar(Fill::EMPTY, Fill::FULL))).frame();
            let frame = Shot::of(art, GaugeLayout::Compact, EraGauge::one(GaugeRow::bar(fill, Fill::FULL))).frame();
            let y = HEIGHT - 3;
            (0..COMPACT_WIDTH).filter(|&x| frame.argb[y * WIDTH + x] != empty.argb[y * WIDTH + x]).count()
        };
        let (half, full) = (lit(Fill::of(1, 2)), lit(Fill::FULL));
        assert!(full >= 50, "{art:?}: a full bar lights {full} columns");
        assert!(half.abs_diff(full / 2) <= 1, "{art:?}: half lights {half} of {full}");
    }
}

#[test]
fn blocks_and_plate_play_the_sweep_and_the_cues_inside_the_compact_corner() {
    for art in OTHER_ARTS {
        let gauge = EraGauge::one(GaugeRow::stock(2, 1, Fill::EMPTY));
        let steady = RowPresentation::new(GaugeKind::Sf2Super);
        let frame = |row: RowPresentation| {
            Shot { presentation: Some(GaugePresentation { main: row, second: None }), ..Shot::of(art, GaugeLayout::Compact, gauge) }
                .frame()
        };
        let still = frame(steady);
        let sweep = |age| frame(RowPresentation {
            ready: Fill::of(1, 2),
            charge: Some(GaugeCharge { from: Fill::EMPTY, to: Fill::of(1, 2), age }),
            ..steady
        });
        assert!(sweep(2).argb != still.argb, "{art:?}: the sweep is drawn");
        assert!(sweep(2).argb != sweep(6).argb, "{art:?}: and it moves");
        // With no label to flash, the spend's amber is on the art's top edge.
        let spent = frame(RowPresentation { spend_age: Some(2), ..steady });
        let edge = (HEIGHT - 4) * WIDTH + 3;
        assert!(spent.argb[edge] != still.argb[edge], "{art:?}: the spend cue is on the top edge");
        let later = frame(RowPresentation { spend_age: Some(10), ..steady });
        assert!(later.argb == still.argb, "{art:?}: and it ends when the label's would");
        for cued in [sweep(2), sweep(6), spent] {
            let (left, right, top, bottom) = drawn_box(&cued);
            assert_eq!((left, bottom), (0, HEIGHT - 1), "{art:?}");
            assert!(top >= HEIGHT - 4 && right < COMPACT_WIDTH, "{art:?}: a cue left the corner");
        }
    }
}

/// #239: the frame meter takes the bottom rows and compact gauges sit
/// directly above it, flush to the side.
#[test]
fn compact_gauges_sit_directly_above_the_frame_meter() {
    const METER_ROWS: usize = 24;
    for art in GaugeArt::ALL {
        let shot = Shot { lift: lift_above_rows(METER_ROWS), ..Shot::of(art, GaugeLayout::Compact, two_rows()) };
        let (left, right, top, bottom) = drawn_box(&shot.frame());
        assert_eq!(bottom, HEIGHT - METER_ROWS - 1, "{art:?}: the row above the meter");
        assert_eq!(top, HEIGHT - METER_ROWS - 8, "{art:?}: eight rows tall");
        assert_eq!((left, right), (0, COMPACT_WIDTH - 1), "{art:?}");
        // Classic sits on the same bottom row, as it did.
        let classic = Shot { layout: GaugeLayout::Classic, ..shot };
        assert_eq!(drawn_box(&classic.frame()).3, HEIGHT - METER_ROWS - 1, "{art:?}: Classic");
    }
}

/// #239: the independent ally's gauge is drawn compact and stacked above
/// the left corner.
#[test]
fn the_allys_gauge_is_compact_and_stacked_above_the_left_corner() {
    let ally = EraGauge::one(GaugeRow::stock(3, 1, Fill::of(1, 2)));
    for art in GaugeArt::ALL {
        let corner = Shot::of(art, GaugeLayout::Compact, two_rows());
        let with_ally = Shot { ally: Some(ally), ..corner };
        let (alone, both) = (corner.frame(), with_ally.frame());
        // The corner under it is untouched, and so is P2's side.
        for y in UNDER_THE_FEET..HEIGHT {
            assert!(alone.argb[y * WIDTH..(y + 1) * WIDTH] == both.argb[y * WIDTH..(y + 1) * WIDTH], "{art:?}: row {y}");
        }
        for y in 0..HEIGHT {
            assert!(
                alone.argb[y * WIDTH + WIDTH / 2..(y + 1) * WIDTH] == both.argb[y * WIDTH + WIDTH / 2..(y + 1) * WIDTH],
                "{art:?}: P2's side, row {y}"
            );
        }
        // One compact row, four rows tall, directly on the corner.
        let (left, right, top, bottom) = drawn_box(&both);
        assert_eq!((left, right), (0, COMPACT_WIDTH - 1), "{art:?}");
        assert_eq!((top, bottom), (UNDER_THE_FEET - 4, HEIGHT - 1), "{art:?}");
        for y in UNDER_THE_FEET - 4..UNDER_THE_FEET {
            assert!(both.argb[y * WIDTH + 3] != BACKDROP, "{art:?}: the ally's row {y} is drawn");
        }
        // An ally with two rows is eight rows tall.
        let tall = Shot { ally: Some(two_rows()), ..corner };
        assert_eq!(drawn_box(&tall.frame()).2, UNDER_THE_FEET - 8, "{art:?}");
        // Above the frame meter it stacks the same way.
        let lifted = Shot { lift: lift_above_rows(24), ..with_ally };
        let (_, _, top, bottom) = drawn_box(&lifted.frame());
        assert_eq!((top, bottom), (HEIGHT - 24 - 12, HEIGHT - 24 - 1), "{art:?}: lifted");
    }
}

/// #239: Classic's ally gauge is where it was: a second-row-high gauge two
/// rows above the 16-row corner.
#[test]
fn the_allys_classic_gauge_is_where_it_was() {
    let ally = EraGauge::one(GaugeRow::stock(3, 1, Fill::of(1, 2)));
    for art in GaugeArt::ALL {
        let shot = Shot { ally: Some(ally), ..Shot::of(art, GaugeLayout::Classic, two_rows()) };
        let (left, _, top, bottom) = drawn_box(&shot.frame());
        assert_eq!(left, 8, "{art:?}");
        assert_eq!(bottom, HEIGHT - 9, "{art:?}: the corner's bottom");
        assert_eq!(top, HEIGHT - 8 - 16 - 2 - 6, "{art:?}: the ally's top");
    }
}

/// With the ROM's font a level's bare number stands in the corner's first
/// columns in every art, and the gauge beside it stays under the feet.
#[test]
fn a_compact_level_number_is_the_bare_digit_in_every_art() {
    use sf_video::compositor::RomFont;
    let Ok(rom) = sf_video::romset::load_assets("../../tools/reference/local/roms/sf", "../../tools/reference/rom-manifest.json")
    else {
        return;
    };
    let font = RomFont::new(&rom.gfx4, &rom.maincpu);
    let gauge = EraGauge::one(GaugeRow::level(2, 3, Fill::of(1, 2)));
    let frames: Vec<Frame> = GaugeArt::ALL
        .into_iter()
        .map(|art| {
            let blank = Frame { argb: vec![BACKDROP; WIDTH * HEIGHT] };
            present(
                PresentMode::Overhaul,
                &blank,
                &OverhaulScene {
                    hud: EraHud { p1: Some(gauge), p2: None },
                    gauge_art: art,
                    gauge_layout: GaugeLayout::Compact,
                    font: Some(&font),
                    ..OverhaulScene::EMPTY
                },
            )
        })
        .collect();
    for (art, frame) in GaugeArt::ALL.into_iter().zip(&frames) {
        let (left, right, top, bottom) = drawn_box(frame);
        assert_eq!((left, bottom), (0, HEIGHT - 1), "{art:?}");
        assert_eq!(right, DIGIT_COLUMN + COMPACT_WIDTH - 1, "{art:?}: the number, then a full bar");
        assert_eq!(top, HEIGHT - 8, "{art:?}: an 8-row glyph on the bottom row");
        // The number's own columns are the same glyph in every art.
        for y in 0..HEIGHT {
            assert!(frame.argb[y * WIDTH..y * WIDTH + 8] == frames[0].argb[y * WIDTH..y * WIDTH + 8], "{art:?}: row {y}");
        }
    }
}
