//! Ticket #223 (EH7): the Training overlay and the era HUD never overlap.
//!
//! While the frame meter is on the gauge corners sit directly above its
//! panel, and the per-fighter telemetry boxes keep clear of every fixed
//! region of the era HUD. The hitbox viewer has no fixed part (its drawings
//! follow the fighters), so it adds no box. The golden frames are a Training
//! fight with the meter off and on, with both gauges drawn; they need the
//! local ROM set and skip without it. `SF_BLESS_GOLDENS=1` rewrites them
//! from the current drawing; look at them before committing.

#[path = "support/select.rs"]
mod select;

use std::path::{Path, PathBuf};

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::OverlayMode;
use sf_engine::front::{FrameInputs, GameSession};
use sf_engine::overlay::{
    fixed_boxes, frame_meter_panel, gauge_lift, telemetry_box, OverlayColors, FRAME_METER_CELLS,
    FRAME_METER_CELL_ALPHA, FRAME_METER_PANEL_ALPHA, FRAME_METER_ROWS,
};
use sf_video::callouts::{
    regions, CalloutBoard, CalloutEvent, CalloutKind, ComboCount, Part, Side,
};
use sf_video::era_hud::{EraGauge, EraHud, Fill, GaugeRow, CORNER_HEIGHT};
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const NAME: &str = "Shinku Hadoken";

const MODES: [OverlayMode; 4] =
    [OverlayMode::Off, OverlayMode::Hitboxes, OverlayMode::FrameMeter, OverlayMode::Both];

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn tap(s: &mut GameSession, input: FrameInputs) {
    s.step(input);
    s.step(idle());
}

const DOWN: usize = 2;

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn start() -> FrameInputs {
    with(|i| i.menu.start = true)
}

fn down() -> FrameInputs {
    with(|i| i.p1.directions[DOWN] = true)
}

/// Training against the dummy Ken under the overhaul layout, P1 Ryu, at
/// the first fight loop.
fn training(a: Assets) -> GameSession {
    let mut s = GameSession::new(a, MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul));
    for _ in 0..200 {
        s.step(idle());
    }
    tap(&mut s, start());
    tap(&mut s, confirm()); // Single Player
    tap(&mut s, down());
    tap(&mut s, confirm()); // Training
    tap(&mut s, down());
    tap(&mut s, confirm()); // DUMMY
    assert_eq!(s.screen(), "select");
    select::confirm_fighter(&mut s, confirm(), tap); // P1 Ryu
    select::confirm_fighter(&mut s, confirm(), GameSession::step); // the dummy Ken
    while s.screen() != "country select" {
        s.step(idle());
    }
    tap(&mut s, with(|i| i.p1.buttons[0] = true));
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    s
}

/// Cycles the pause box's OVERLAYS row to `mode` and resumes.
fn set_overlay(s: &mut GameSession, mode: OverlayMode) {
    tap(s, start());
    tap(s, down());
    while s.fight().unwrap().overlay() != mode {
        tap(s, confirm());
    }
    tap(s, start());
}

fn gauges() -> EraHud {
    EraHud {
        p1: Some(EraGauge::two(GaugeRow::stock(6, 4, Fill::of(1, 2)), GaugeRow::level(1, 3, Fill::of(1, 3)))),
        p2: Some(EraGauge::one(GaugeRow::level(2, 3, Fill::of(2, 3)))),
    }
}

fn golden_path(name: &str) -> PathBuf {
    Path::new("tests/goldens/training_overlay").join(format!("{name}.png"))
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

#[test]
fn the_gauges_sit_directly_above_the_frame_meter_in_training() {
    let Some(a) = assets() else { return };
    let mut s = training(a);
    s.set_era_hud(gauges());
    for _ in 0..30 {
        s.step(idle());
    }

    // Meter off: the gauges in the bottom corners, as everywhere else.
    let off = s.step(idle()).frame;
    check_golden("meter_off", &off);
    for side in Side::BOTH {
        let corner = regions::gauge_corner(side);
        assert_eq!(corner.y + corner.h, HEIGHT - 8);
    }

    // Meter on: the same gauges, lifted so their bottom edge is the panel's top.
    set_overlay(&mut s, OverlayMode::FrameMeter);
    let on = s.step(idle()).frame;
    check_golden("meter_on", &on);
    let panel = frame_meter_panel();
    assert_eq!(panel.y, HEIGHT - FRAME_METER_ROWS);
    for side in Side::BOTH {
        let corner = regions::gauge_corner_lifted(side, gauge_lift(true));
        assert_eq!(corner.y + corner.h, panel.y, "{side:?}: directly above the panel");
        assert!(!corner.intersects(&panel));
        // The gauges moved: P1's two rows are drawn in the lifted corner, not in the old one.
        assert!(corner.h == CORNER_HEIGHT);
    }
    let changed_in = |frame: &Frame, rect: regions::Rect| {
        (rect.y..rect.y + rect.h)
            .flat_map(|y| (rect.x..rect.x + rect.w).map(move |x| (x, y)))
            .filter(|&(x, y)| frame.argb[y * WIDTH + x] != off.argb[y * WIDTH + x])
            .count()
    };
    let lifted = regions::gauge_corner_lifted(Side::P1, gauge_lift(true));
    assert!(changed_in(&on, lifted) > 0, "the lifted corner is drawn");
}

/// Every box that must be apart: the regions with the corners lifted for
/// `mode`, the overlay's fixed boxes and `board`'s call-out elements.
fn assert_all_apart(board: &CalloutBoard, mode: OverlayMode, when: &str) {
    let (_, meter) = mode.parts();
    let mut boxes: Vec<(String, regions::Rect)> =
        regions::all_lifted(gauge_lift(meter)).into_iter().map(|(r, rect)| (format!("{r:?}"), rect)).collect();
    for (i, rect) in fixed_boxes(meter).into_iter().enumerate() {
        boxes.push((format!("overlay box {i}"), rect));
    }
    for (i, (a, ra)) in boxes.iter().enumerate() {
        assert!(ra.x + ra.w <= WIDTH && ra.y + ra.h <= HEIGHT, "{when} {mode:?}: {a} is on screen");
        for (b, rb) in &boxes[i + 1..] {
            // The telemetry boxes sit over the 1987 health block, as the
            // Training overlay always has; only TIME there is the era HUD's.
            let over_health = (a == "HealthBlock") != (b == "HealthBlock")
                && (a.starts_with("overlay") || b.starts_with("overlay"));
            if over_health {
                let overlay = if a.starts_with("overlay") { ra } else { rb };
                assert!(!overlay.intersects(&regions::TIME), "{when} {mode:?}: an overlay box meets TIME");
                continue;
            }
            assert!(!ra.intersects(rb), "{when} {mode:?}: {a} {ra:?} meets {b} {rb:?}");
        }
    }
    // Each drawn call-out element against every region it is not in, and the overlay's boxes.
    for e in board.frame().elements() {
        let own = match e.part {
            Part::Combo | Part::Word(_) => regions::Region::CalloutColumn(e.side),
            Part::MoveName => regions::Region::MoveName(e.side),
        };
        for (name, rect) in &boxes {
            if *name == format!("{own:?}") {
                assert!(rect.holds(&e.rect), "{when} {mode:?}: {e:?} outside {name}");
            } else {
                assert!(!e.rect.intersects(rect), "{when} {mode:?}: {e:?} meets {name} {rect:?}");
            }
        }
    }
}

#[test]
fn the_overlay_and_the_era_hud_never_overlap_in_any_overlay_setting() {
    let kinds = CalloutKind::all(NAME);
    for mode in MODES {
        // Every event on both sides on one frame.
        let mut board = CalloutBoard::new();
        for side in Side::BOTH {
            board.set_combo(side, Some(ComboCount { hits: 12, damage: Some(345) }));
        }
        let events: Vec<_> = kinds.iter().flat_map(|k| Side::BOTH.map(|s| CalloutEvent::new(s, *k))).collect();
        board.step(&events);
        assert_all_apart(&board, mode, "one frame");

        // And over a burst of frames.
        let mut board = CalloutBoard::new();
        let mut seed: u32 = 0x2237;
        let mut rand = move || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            seed >> 16
        };
        for t in 0..300u32 {
            let mut events = Vec::new();
            for side in Side::BOTH {
                board.set_combo(side, (t % 50 < 30).then_some(ComboCount { hits: (t % 50) as u16 + 2, damage: None }));
                for _ in 0..rand() % 4 {
                    events.push(CalloutEvent::new(side, kinds[rand() as usize % kinds.len()]));
                }
            }
            board.step(&events);
            assert_all_apart(&board, mode, &format!("frame {t}"));
        }
    }
}

#[test]
fn the_telemetry_boxes_are_fixed_on_their_own_halves_clear_of_the_hud() {
    for side in Side::BOTH {
        let rect = telemetry_box(side);
        let half = match side {
            Side::P1 => regions::Rect::new(0, 0, WIDTH / 2, HEIGHT),
            Side::P2 => regions::Rect::new(WIDTH / 2, 0, WIDTH / 2, HEIGHT),
        };
        assert!(half.holds(&rect), "{side:?}");
        // Clear of everything the era HUD adds, with the corners lifted or
        // not: it sits over the 1987 health block, as it always has, but
        // never over TIME under the bars.
        assert!(!rect.intersects(&regions::TIME), "{side:?} meets TIME");
        for lift in [0, gauge_lift(true)] {
            for (name, region) in regions::all_lifted(lift) {
                if name == regions::Region::HealthBlock {
                    continue;
                }
                assert!(!rect.intersects(&region), "{side:?} meets {name:?} (lift {lift})");
            }
        }
        assert!(!rect.intersects(&frame_meter_panel()));
    }
}

#[test]
fn a_fights_gauges_are_drawn_in_the_chosen_gauge_art() {
    let Some(a) = assets() else { return };
    let mut s = training(a);
    s.set_era_hud(gauges());
    for _ in 0..30 {
        s.step(idle());
    }
    let ruled = s.step(idle()).frame;
    // The pause box's GAUGE ART row (under LOOK): Right moves Ruled to Blocks.
    tap(&mut s, start());
    for _ in 0..3 {
        tap(&mut s, down());
    }
    tap(&mut s, with(|i| i.p1.directions[0] = true));
    assert_eq!(s.fight().unwrap().gauge_art(), sf_video::era_hud::GaugeArt::Blocks);
    tap(&mut s, start());
    for _ in 0..5 {
        s.step(idle());
    }
    let blocks = s.step(idle()).frame;
    let corner = regions::gauge_corner(Side::P1);
    let differ = (corner.y..corner.y + corner.h)
        .flat_map(|y| (corner.x..corner.x + corner.w).map(move |x| (x, y)))
        .filter(|&(x, y)| ruled.argb[y * WIDTH + x] != blocks.argb[y * WIDTH + x])
        .count();
    assert!(differ > 0, "the corner is redrawn in Blocks");
}

#[test]
fn the_paused_fight_shows_a_gauge_art_change_behind_the_pause_box() {
    let Some(a) = assets() else { return };
    let mut s = training(a);
    s.set_era_hud(gauges());
    for _ in 0..30 {
        s.step(idle());
    }
    tap(&mut s, start());
    for _ in 0..3 {
        tap(&mut s, down());
    }
    let before = s.step(idle()).frame;
    tap(&mut s, with(|i| i.p1.directions[0] = true));
    let after = s.step(idle()).frame;
    let corner = regions::gauge_corner(Side::P1);
    let differ = (corner.y..corner.y + corner.h)
        .flat_map(|y| (corner.x..corner.x + corner.w).map(move |x| (x, y)))
        .filter(|&(x, y)| before.argb[y * WIDTH + x] != after.argb[y * WIDTH + x])
        .count();
    assert!(differ > 0, "the dimmed fight behind the box is redrawn in the new art");
}

#[test]
fn reset_positions_clears_the_abandoned_attempts_call_outs() {
    use sf_video::callouts::CalloutEvent;
    let Some(a) = assets() else { return };
    let mut s = training(a);
    for _ in 0..30 {
        s.step(idle());
    }
    s.push_callouts(&[CalloutEvent::new(Side::P1, CalloutKind::Counter)]);
    s.step(idle());
    assert!(!s.fight().unwrap().callouts().frame().is_empty());
    // The pause box's RESET POSITIONS (the fifth row).
    tap(&mut s, start());
    for _ in 0..4 {
        tap(&mut s, down());
    }
    tap(&mut s, confirm());
    s.step(idle());
    assert!(s.fight().unwrap().callouts().frame().is_empty(), "the old words went with the attempt");
}

#[test]
fn a_tries_lowered_columns_and_strip_lines_clear_the_overlay_and_the_gauges() {
    // A Training Try drops the call-out columns ten pixels for the strip's
    // second line; with the corners lifted or not, and the overlay's meter
    // on or off, nothing meets anything (the telemetry boxes sit over the
    // 1987 health block as they always have, never over TIME).
    for meter in [false, true] {
        let mut boxes: Vec<(String, regions::Rect)> = regions::all_with(gauge_lift(meter), regions::TRY_DROP)
            .into_iter()
            .map(|(r, rect)| (format!("{r:?}"), rect))
            .collect();
        for (i, rect) in fixed_boxes(meter).into_iter().enumerate() {
            boxes.push((format!("overlay box {i}"), rect));
        }
        assert!(boxes.iter().any(|(name, _)| name == "StripInputs"));
        for (i, (a, ra)) in boxes.iter().enumerate() {
            assert!(ra.x + ra.w <= WIDTH && ra.y + ra.h <= HEIGHT, "meter {meter}: {a} is on screen");
            for (b, rb) in &boxes[i + 1..] {
                let over_health = (a == "HealthBlock") != (b == "HealthBlock") && (a.starts_with("overlay") || b.starts_with("overlay"));
                if over_health {
                    let overlay = if a.starts_with("overlay") { ra } else { rb };
                    assert!(!overlay.intersects(&regions::TIME), "meter {meter}: an overlay box meets TIME");
                    continue;
                }
                assert!(!ra.intersects(rb), "meter {meter}: {a} {ra:?} meets {b} {rb:?}");
            }
        }
    }
}

/// A pixel of `dst` with `src` laid over it at `alpha` out of 256: the
/// overlay's own blend, written out so the test does not lean on it.
fn blend(dst: u32, src: u32, alpha: u32) -> u32 {
    let inv = 256 - alpha;
    let ch = |shift: u32| (((dst >> shift) & 0xff) * inv + ((src >> shift) & 0xff) * alpha) >> 8;
    0xff00_0000 | (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// Training with the overlay on `mode`, 30 fight frames in and a few more
/// after the pause box: the pause box freezes the fight, so two sessions
/// that differ only in `mode` show the same stage on the same frame.
fn training_frame(mode: OverlayMode) -> Option<Frame> {
    let a = assets()?;
    let mut s = training(a);
    for _ in 0..30 {
        s.step(idle());
    }
    set_overlay(&mut s, mode);
    for _ in 0..5 {
        s.step(idle());
    }
    Some(s.step(idle()).frame)
}

fn pixel(frame: &Frame, x: usize, y: usize) -> u32 {
    frame.argb[y * WIDTH + x]
}

#[test]
fn the_frame_meters_panel_is_a_faint_tint_over_the_stage() {
    let (Some(plain), Some(meter)) = (training_frame(OverlayMode::Off), training_frame(OverlayMode::FrameMeter)) else {
        return;
    };
    let panel = frame_meter_panel();
    let telemetry: Vec<_> = Side::BOTH.map(telemetry_box).into();
    let in_box = |x: usize, y: usize| telemetry.iter().any(|r| r.x <= x && x < r.x + r.w && r.y <= y && y < r.y + r.h);
    let in_panel = |x: usize, y: usize| panel.x <= x && x < panel.x + panel.w && panel.y <= y && y < panel.y + panel.h;

    // Nothing outside the meter's panel and the telemetry boxes moved: the
    // two frames are the same stage on the same frame.
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if !in_panel(x, y) && !in_box(x, y) {
                assert_eq!(pixel(&meter, x, y), pixel(&plain, x, y), "({x},{y}) outside the meter changed");
            }
        }
    }

    // The panel's own rows (the strips above, between and under the two
    // cell rows) are the stage under a faint tint, and so are not what the
    // old near-solid panel (alpha 200) gave.
    let tint_rows = [0, 1, 8, 9, 16, 17];
    let (mut tracked, mut solid_200, mut checked) = (0, 0, 0);
    for dy in tint_rows {
        for x in panel.x..panel.x + panel.w {
            let y = panel.y + dy;
            let under = pixel(&plain, x, y);
            checked += 1;
            if pixel(&meter, x, y) == blend(under, OverlayColors::HUD_BG, FRAME_METER_PANEL_ALPHA) {
                tracked += 1;
            }
            if pixel(&meter, x, y) == blend(under, OverlayColors::HUD_BG, 200) {
                solid_200 += 1;
            }
        }
    }
    assert_eq!(tracked, checked, "every tint pixel is the stage under the faint panel");
    assert!(solid_200 < checked / 10, "{solid_200} of {checked} pixels still look like the 200-alpha panel");
    // The stage shows through: the panel does not flatten what is under it.
    let distinct = |f: &Frame| {
        let mut seen = std::collections::BTreeSet::new();
        for dy in tint_rows {
            for x in panel.x..panel.x + panel.w {
                seen.insert(pixel(f, x, panel.y + dy));
            }
        }
        seen.len()
    };
    assert!(distinct(&plain) > 1, "the stage under the panel has more than one colour");
    assert!(distinct(&meter) > 1, "and the panel keeps showing it");
}

#[test]
fn the_frame_meters_cells_and_text_are_blended_over_the_stage() {
    let (Some(plain), Some(meter)) = (training_frame(OverlayMode::Off), training_frame(OverlayMode::FrameMeter)) else {
        return;
    };
    let panel = frame_meter_panel();
    let tinted = |x: usize, y: usize| blend(pixel(&plain, x, y), OverlayColors::HUD_BG, FRAME_METER_PANEL_ALPHA);

    // A fresh Training fight, nobody acting: every cell is neutral. Each
    // cell pixel is the neutral colour laid over the tinted stage, so it
    // carries what is under it; and the 1-pixel gap between cells is only
    // the tint (the stage shows between the cells).
    for (row_y, label) in [(panel.y + 2, "P1"), (panel.y + 10, "P2")] {
        let mut under = std::collections::BTreeSet::new();
        for idx in 0..FRAME_METER_CELLS {
            let x0 = 34 + idx * 5;
            for dy in 0..6 {
                for dx in 0..4 {
                    let (x, y) = (x0 + dx, row_y + dy);
                    assert_eq!(
                        pixel(&meter, x, y),
                        blend(tinted(x, y), OverlayColors::FRAME_NEUTRAL, FRAME_METER_CELL_ALPHA),
                        "{label} cell {idx} pixel ({x},{y}) is blended over the stage"
                    );
                    assert_ne!(pixel(&meter, x, y), OverlayColors::FRAME_NEUTRAL, "{label} cell {idx} is not solid");
                    under.insert(pixel(&plain, x, y));
                }
            }
            if idx + 1 < FRAME_METER_CELLS {
                for dy in 0..6 {
                    let (x, y) = (x0 + 4, row_y + dy);
                    assert_eq!(pixel(&meter, x, y), tinted(x, y), "{label} gap after cell {idx} shows the stage");
                }
            }
        }
        assert!(under.len() > 1, "{label}: the stage under the cells is not one flat colour");
    }

    // The text is blended too: no pixel of the meter's two text rows is the
    // solid text colour any more.
    for y in panel.y..panel.y + panel.h {
        for x in panel.x..panel.x + panel.w {
            assert_ne!(pixel(&meter, x, y), OverlayColors::HUD_TEXT, "solid text pixel at ({x},{y})");
        }
    }
    // And it is still there: the text region differs from the bare tint.
    let text_changed = (panel.y + 3..panel.y + 8)
        .flat_map(|y| (6..30).map(move |x| (x, y)))
        .filter(|&(x, y)| pixel(&meter, x, y) != tinted(x, y))
        .count();
    assert!(text_changed > 0, "the P1 label and advantage are drawn");
}

#[test]
fn the_telemetry_boxes_draw_as_they_did_before_the_see_through_meter() {
    let Some(meter) = training_frame(OverlayMode::FrameMeter) else { return };
    let Some(plain) = training_frame(OverlayMode::Off) else { return };
    // The boxes are drawn as before the meter went see-through: every pixel
    // is the frame under it behind the alpha-180 panel, or a solid text
    // pixel. Checked against the frame beneath rather than a pinned hash,
    // so a change to the screen under the boxes (TR3 hid the round-win
    // slots there) does not read as a change to the boxes.
    const TEXT: [u32; 3] = [OverlayColors::HUD_TEXT, OverlayColors::HUD_ACTIVE, OverlayColors::HUD_WARN];
    for side in Side::BOTH {
        let rect = telemetry_box(side);
        let mut text = 0;
        for y in rect.y..rect.y + rect.h {
            for x in rect.x..rect.x + rect.w {
                let got = pixel(&meter, x, y);
                if TEXT.contains(&got) {
                    text += 1;
                } else {
                    assert_eq!(got, blend(pixel(&plain, x, y), OverlayColors::HUD_BG, 180), "{side:?} box pixel ({x},{y})");
                }
            }
        }
        assert!(text > 0, "{side:?} box has its solid text");
    }
}
