//! Ticket #220 (EH4): call-outs, the era HUD's screen regions and the
//! no-overlap stress test, at the compositor seam.
//!
//! Golden frames live in `tests/goldens/callouts/` and are compared pixel
//! for pixel, one per call-out per gauge art. The lettering is the ROM's
//! 8×8 font, so the golden and lettering tests need the local ROM set and
//! skip without it. `SF_BLESS_GOLDENS=1` rewrites the goldens from the
//! current drawing; look at them before committing.

use std::path::{Path, PathBuf};

use sf_video::callouts::{
    grey, regions, CalloutBoard, CalloutEvent, CalloutFrame, CalloutKind, ComboCount, Part, Side, LETTER_PENS,
    SUPER_DIM_FRAMES, WORD_FRAMES, WORD_SLOTS,
};
use sf_video::compositor::{present, IndexedSprite, OverhaulScene, PresentMode, RomFont};
use sf_video::era_hud::{pens, EraGauge, EraHud, Fill, GaugeArt, GaugeRow, GaugeState};
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const BACKDROP: u32 = 0xff_44_66_88;
const NAME: &str = "Shinku Hadoken";

/// Every gauge art there is. The match fails to compile when a new art is
/// added, so it joins every test here.
fn arts() -> Vec<GaugeArt> {
    let all = vec![GaugeArt::Ruled, GaugeArt::Blocks, GaugeArt::Plate];
    for art in &all {
        match art {
            GaugeArt::Ruled | GaugeArt::Blocks | GaugeArt::Plate => {}
        }
    }
    all
}

fn art_slug(art: GaugeArt) -> String {
    format!("{art:?}").to_lowercase()
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

/// SF6 Drive over Super Art for P1, the Alpha level bar for P2.
fn gauges() -> EraHud {
    EraHud {
        p1: Some(EraGauge::two(GaugeRow::stock(6, 4, Fill::of(1, 2)), GaugeRow::level(1, 3, Fill::of(1, 3)))),
        p2: Some(EraGauge::one(GaugeRow::level(2, 3, Fill::of(2, 3)))),
    }
}

const GI: u32 = 0xff_e8_e0_d0;
const SKIN: u32 = 0xff_d0_90_60;

/// A 16 × 32 stand-in fighter in two colours on `side`'s half.
fn fighter(side: Side) -> Vec<IndexedSprite> {
    let x = match side {
        Side::P1 => 140,
        Side::P2 => WIDTH as i32 - 140 - 16,
    };
    let mut palette = [0u32; 16];
    palette[1] = GI;
    palette[2] = SKIN;
    (0..2)
        .map(|row| {
            let mut pixels = [0u8; 256];
            for (i, p) in pixels.iter_mut().enumerate() {
                *p = if row == 0 && i < 96 { 2 } else { 1 };
            }
            IndexedSprite { x, y: 164 + row * 16, pixels, palette }
        })
        .collect()
}

fn draw(art: GaugeArt, hud: EraHud, callouts: CalloutFrame, stage: Option<&[bool]>, font: Option<&RomFont<'_>>) -> Frame {
    let (p1, p2) = (fighter(Side::P1), fighter(Side::P2));
    present(
        PresentMode::Overhaul,
        &blank(),
        &OverhaulScene {
            hud,
            gauge_art: art,
            font,
            callouts,
            fighters: [&p1, &p2],
            stage,
            ..OverhaulScene::EMPTY
        },
    )
}

fn both(kind: CalloutKind) -> [CalloutEvent; 2] {
    [CalloutEvent::new(Side::P1, kind), CalloutEvent::new(Side::P2, kind)]
}

fn slug(kind: CalloutKind) -> String {
    kind.word().to_lowercase().replace(' ', "_")
}

fn golden_path(name: &str) -> PathBuf {
    Path::new("tests/goldens/callouts").join(format!("{name}.png"))
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

fn pixels_in(frame: &Frame, rect: regions::Rect) -> Vec<u32> {
    (rect.y..rect.y + rect.h).flat_map(|y| (rect.x..rect.x + rect.w).map(move |x| frame.argb[y * WIDTH + x])).collect()
}

fn half(color: u32) -> u32 {
    0xff00_0000 | ((color >> 1) & 0x007f_7f7f)
}

#[test]
fn every_settled_call_out_matches_its_golden_frame_in_its_slot() {
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    for art in arts() {
        for kind in CalloutKind::all(NAME) {
            let mut board = CalloutBoard::new();
            if kind == CalloutKind::Burnout {
                board.set_greyed(Side::P1, true);
                board.set_greyed(Side::P2, true);
            }
            board.step(&both(kind));
            let shown = board.frame().with_static_words();
            let frame = draw(art, gauges(), shown, None, Some(&font));
            check_golden(&format!("{}_{}", art_slug(art), slug(kind)), &frame);

            for side in Side::BOTH {
                // The word sits in the first word slot, from its side's edge, in PLAYER1 gold.
                let word = shown.elements().into_iter().find(|e| e.side == side && e.part == Part::Word(0)).unwrap();
                assert!(regions::word_slot(side, 0).holds(&word.rect), "{kind:?} {side:?}");
                assert_eq!(word.rect.w, 8 * kind.word().len());
                let x = match side {
                    Side::P1 => 8,
                    Side::P2 => WIDTH - 8 - word.rect.w,
                };
                assert_eq!(word.rect.x, x, "{kind:?} {side:?}: set from the screen edge");
                let lettering = pixels_in(&frame, word.rect);
                for pen in LETTER_PENS {
                    assert!(lettering.contains(&pen), "{kind:?} {side:?}: PLAYER1 gold");
                }
            }
        }
    }
}

#[test]
fn words_slide_inward_and_only_super_and_perfect_parry_draw_the_priority_dash() {
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    let events = [
        CalloutEvent::new(Side::P1, CalloutKind::AlphaCounter),
        CalloutEvent::new(Side::P1, CalloutKind::Counter),
        CalloutEvent::new(Side::P1, CalloutKind::PunishCounter),
        CalloutEvent::new(Side::P1, CalloutKind::Super { name: NAME }),
        CalloutEvent::new(Side::P2, CalloutKind::PerfectParry),
    ];
    let mut board = CalloutBoard::new();
    board.set_drop(regions::TRY_DROP);
    board.step(&events);
    let mut launch = None;
    for age in 0..13 {
        let shown = board.frame();
        let still = shown.with_static_words();
        let render = |callouts| {
            let mut frame = blank();
            sf_video::callouts::draw(&mut frame.argb, &callouts, Some(&font));
            frame
        };
        let animated = render(shown);
        let settled = render(still);
        assert_eq!(shown.gauges(gauges()), still.gauges(gauges()), "Alpha Counter keeps its native gauge flash");
        assert_apart(&shown, "animated words");
        assert_boxes_hold_the_drawing(&animated, &shown, "animated words");

        let placed = shown.elements();
        let base = still.elements();
        assert_eq!(placed.len(), base.len());
        for (motion, rest) in placed.into_iter().zip(base) {
            let Part::Word(slot) = motion.part else {
                assert_eq!(motion, rest);
                assert_eq!(pixels_in(&animated, motion.rect), pixels_in(&settled, rest.rect));
                continue;
            };
            let kind = shown.side(motion.side).words[slot].unwrap();
            let priority = matches!(kind, CalloutKind::Super { .. } | CalloutKind::PerfectParry);
            let distance = match age { 0..=2 => 2, 3..=5 => 1, _ => 0 };
            let x = match motion.side { Side::P1 => rest.rect.x + distance, Side::P2 => rest.rect.x - distance };
            assert_eq!(motion.rect.x, x, "{kind:?} at {age}");
            let ink = pixels_in(&animated, motion.rect);
            assert_eq!(ink.contains(&pens::CREAM), priority && age < 6, "{kind:?} at {age}");
            let dash = regions::Rect::new(motion.rect.x, motion.rect.y + 8, motion.rect.w, 1);
            let count = pixels_in(&animated, dash).into_iter().filter(|&p| p != BACKDROP).count();
            assert_eq!(count, if priority && age < 12 { (3 + age * 2).min(11) } else { 0 }, "{kind:?} at {age}");
        }

        if age == 0 {
            launch = Some((board.clone(), animated.argb));
        } else if age == 12 {
            assert_eq!(animated.argb, settled.argb);
            let (launch, picture) = launch.as_ref().unwrap();
            assert_eq!(render(launch.frame()).argb, *picture, "cloned board keeps the entrance frame");
        }
        board.step(&[]);
    }
}

#[test]
fn every_word_is_lettered_in_rom_glyphs() {
    for kind in CalloutKind::all(NAME) {
        for ch in kind.word().chars().filter(|ch| *ch != ' ') {
            assert!(sf_video::compositor::small_glyph(ch).is_some(), "{ch} in {}", kind.word());
        }
        assert!(kind.word().len() <= regions::LINE_LETTERS, "{} fits its column", kind.word());
    }
}

#[test]
fn the_call_out_columns_sit_at_72_outside_a_try_and_lower_during_one() {
    // The era HUD's approved layout: every fight but a Training Try. The
    // columns drop ten pixels for a Try's second strip line, and only then.
    assert_eq!(regions::COLUMN_TOP, 72);
    assert_eq!(regions::callout_column(Side::P1).y, 72);
    assert_eq!(regions::callout_column_at(Side::P1, regions::TRY_DROP).y, 82);
    let events = [CalloutEvent::new(Side::P1, CalloutKind::FirstAttack), CalloutEvent::new(Side::P2, CalloutKind::Perfect)];
    for (drop, top) in [(0, 72), (regions::TRY_DROP, 82)] {
        let mut board = CalloutBoard::new();
        board.set_drop(drop);
        board.set_combo(Side::P1, Some(ComboCount { hits: 4, damage: None }));
        board.step(&events);
        let shown = board.frame();
        assert_eq!(shown.drop, drop);
        for e in shown.elements() {
            let column = regions::callout_column_at(e.side, drop);
            assert_eq!(column.y, top);
            assert!(column.holds(&e.rect), "{e:?} in the column at {top}");
            if e.part == Part::Combo {
                assert_eq!(e.rect.y, top, "the count's slot is the column's top");
            }
        }
        // A board that has not been told to drop is the usual layout, and an idle one is empty either way.
        assert_eq!(CalloutBoard::new().frame(), CalloutFrame::NONE);
        board.set_drop(drop);
        assert!(CalloutFrame { drop, ..CalloutFrame::NONE }.is_empty());
    }
    // The regions of the ordinary layout have no strip line; a Try's have it,
    // and each set is apart.
    assert!(regions::all().iter().all(|(r, _)| *r != regions::Region::StripInputs));
    assert!(regions::all_try().iter().any(|(r, _)| *r == regions::Region::StripInputs));
    for all in [regions::all(), regions::all_try(), regions::all_with(24, regions::TRY_DROP)] {
        for (i, (a, ra)) in all.iter().enumerate() {
            for (b, rb) in &all[i + 1..] {
                assert!(!ra.intersects(rb), "{a:?} {ra:?} meets {b:?} {rb:?}");
            }
        }
    }
}

#[test]
fn the_fixed_regions_never_intersect() {
    let all = regions::all();
    for (i, (a, ra)) in all.iter().enumerate() {
        for (b, rb) in &all[i + 1..] {
            assert!(!ra.intersects(rb), "{a:?} {ra:?} meets {b:?} {rb:?}");
        }
        assert!(ra.x + ra.w <= WIDTH && ra.y + ra.h <= HEIGHT, "{a:?} is on screen");
    }
    // TIME, under the health bars, is inside the health block.
    assert!(regions::HEALTH_BLOCK.holds(&regions::Rect::new(164, 52, 64, 8)));
    for side in Side::BOTH {
        let column = regions::callout_column(side);
        // Each column is in its own half, at mid-height.
        let half = match side {
            Side::P1 => regions::Rect::new(0, 0, WIDTH / 2, HEIGHT),
            Side::P2 => regions::Rect::new(WIDTH / 2, 0, WIDTH / 2, HEIGHT),
        };
        assert!(half.holds(&column) && half.holds(&regions::move_name_line(side)), "{side:?}");
        assert!(column.y < HEIGHT / 2 && column.y + column.h > HEIGHT / 2, "{side:?} at mid-height");
        // The combo slot is on top and the word slots under it, all in the column.
        assert_eq!(regions::combo_slot(side).y, column.y);
        let mut above = regions::combo_slot(side);
        for slot in 0..WORD_SLOTS {
            let word = regions::word_slot(side, slot);
            assert!(column.holds(&word) && word.y >= above.y + above.h, "{side:?} slot {slot}");
            above = word;
        }
        assert!(regions::move_name_line(side).y >= column.y + column.h, "the name is under the column");
    }
}

/// Every element of `shown` against every other, and each in its region.
fn assert_apart(shown: &CalloutFrame, when: &str) {
    let elements = shown.elements();
    for (i, a) in elements.iter().enumerate() {
        for b in &elements[i + 1..] {
            assert!(!a.rect.intersects(&b.rect), "{when}: {a:?} meets {b:?}");
        }
        let region = match a.part {
            Part::Combo | Part::Word(_) => regions::callout_column_at(a.side, shown.drop),
            Part::MoveName => regions::move_name_line_at(a.side, shown.drop),
        };
        assert!(region.holds(&a.rect), "{when}: {a:?} outside {region:?}");
        for (name, rect) in regions::all_with(0, shown.drop) {
            let own = matches!(name, regions::Region::CalloutColumn(s) | regions::Region::MoveName(s) if s == a.side);
            assert!(own || !a.rect.intersects(&rect), "{when}: {a:?} meets {name:?}");
        }
    }
}

/// Every pixel the presented frame changed is inside a drawn element's box
/// or a gauge corner: the boxes are honest.
fn assert_boxes_hold_the_drawing(frame: &Frame, shown: &CalloutFrame, when: &str) {
    let mut boxes: Vec<regions::Rect> = shown.elements().into_iter().map(|e| e.rect).collect();
    boxes.extend(Side::BOTH.map(regions::gauge_corner));
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if frame.argb[y * WIDTH + x] != BACKDROP {
                assert!(boxes.iter().any(|b| b.contains(x, y)), "{when}: drawn outside every box at ({x}, {y})");
            }
        }
    }
}

#[test]
fn stress_every_event_on_both_sides_on_one_frame() {
    let assets = assets();
    let font = assets.as_ref().map(|a| RomFont::new(&a.gfx4, &a.maincpu));
    let no_stage = vec![false; WIDTH * HEIGHT];
    for art in arts() {
        let mut board = CalloutBoard::new();
        for side in Side::BOTH {
            board.set_combo(side, Some(ComboCount { hits: 12, damage: Some(345) }));
            board.set_greyed(side, true);
        }
        let mut events = Vec::new();
        for kind in CalloutKind::all(NAME) {
            events.extend(both(kind));
        }
        board.step(&events);
        let shown = board.frame();
        assert_apart(&shown, "one frame");
        for side in Side::BOTH {
            let side_shown = shown.side(side);
            assert!(side_shown.combo.is_some(), "the count is never dropped");
            // The full column kept the newest words, oldest on top.
            let newest: Vec<_> = CalloutKind::all(NAME)[11 - WORD_SLOTS..].iter().copied().map(Some).collect();
            assert_eq!(side_shown.words.to_vec(), newest, "{side:?}");
            assert_eq!(side_shown.move_name, Some(NAME));
        }
        assert!(shown.dim);
        if let Some(font) = &font {
            // No dim on a stage mask that marks nothing, and no fighters: only the HUD is drawn.
            let frame = present(
                PresentMode::Overhaul,
                &blank(),
                &OverhaulScene {
                    hud: gauges(),
                    gauge_art: art,
                    font: Some(font),
                    callouts: shown,
                    stage: Some(&no_stage),
                    ..OverhaulScene::EMPTY
                },
            );
            assert_boxes_hold_the_drawing(&frame, &shown, "one frame");
        }
    }
}

#[test]
fn stress_a_burst_of_frames_keeps_everything_apart() {
    let assets = assets();
    let font = assets.as_ref().map(|a| RomFont::new(&a.gfx4, &a.maincpu));
    let no_stage = vec![false; WIDTH * HEIGHT];
    let kinds = CalloutKind::all(NAME);
    for art in arts() {
        let mut board = CalloutBoard::new();
        let mut seed: u32 = 0x2201;
        let mut rand = move || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            seed >> 16
        };
        // What each column should hold: arrival order, newest WORD_SLOTS still in their time.
        let mut expected: [Vec<(CalloutKind, u32)>; 2] = [Vec::new(), Vec::new()];
        for t in 0..300u32 {
            let mut events = Vec::new();
            for side in Side::BOTH {
                board.set_combo(side, (t % 50 < 30).then_some(ComboCount { hits: (t % 50) as u16 + 2, damage: None }));
                for _ in 0..rand() % 4 {
                    let kind = kinds[rand() as usize % kinds.len()];
                    events.push(CalloutEvent::new(side, kind));
                }
            }
            board.step(&events);
            for side in Side::BOTH {
                let column = &mut expected[side.index()];
                column.retain(|(_, at)| t - at < u32::from(WORD_FRAMES));
                for event in events.iter().filter(|e| e.side == side) {
                    if column.len() == WORD_SLOTS {
                        column.remove(0);
                    }
                    column.push((event.kind, t));
                }
            }
            let shown = board.frame();
            let when = format!("{} frame {t}", art_slug(art));
            assert_apart(&shown, &when);
            for side in Side::BOTH {
                let side_shown = shown.side(side);
                let want: Vec<_> = expected[side.index()].iter().map(|(k, _)| *k).collect();
                let got: Vec<_> = side_shown.words.iter().flatten().copied().collect();
                assert_eq!(got, want, "{when} {side:?}");
                assert_eq!(side_shown.combo.is_some(), t % 50 < 30, "{when}: the count stays while it is set");
            }
            if t % 25 == 0 {
                if let Some(font) = &font {
                    let frame = present(
                        PresentMode::Overhaul,
                        &blank(),
                        &OverhaulScene {
                            hud: gauges(),
                            gauge_art: art,
                            font: Some(font),
                            callouts: shown,
                            stage: Some(&no_stage),
                            ..OverhaulScene::EMPTY
                        },
                    );
                    assert_boxes_hold_the_drawing(&frame, &shown, &when);
                }
            }
        }
    }
}

#[test]
fn a_tried_moves_call_out_takes_a_word_slot_and_keeps_clear_of_everything() {
    // The word is the move's name and a bang, lettered in the ROM's glyphs,
    // and it fits the column; it is the player's own, so it joins the other
    // words in order without moving the count or the strip.
    for word in ["HADOKEN!", "SHORYUKEN!", "TATSUMAKI!"] {
        let kind = CalloutKind::Move { word };
        assert_eq!(kind.word(), word);
        assert!(word.chars().all(|ch| sf_video::compositor::small_glyph(ch).is_some()), "{word}");
        assert!(word.len() <= regions::LINE_LETTERS);
    }
    let mut board = CalloutBoard::new();
    board.set_combo(Side::P1, Some(ComboCount { hits: 3, damage: Some(20) }));
    board.step(&[CalloutEvent::new(Side::P1, CalloutKind::FirstAttack)]);
    board.step(&[CalloutEvent::new(Side::P1, CalloutKind::Move { word: "SHORYUKEN!" })]);
    let shown = board.frame();
    assert_eq!(shown.side(Side::P1).words[1], Some(CalloutKind::Move { word: "SHORYUKEN!" }));
    assert_apart(&shown, "a tried move's word");
    let strip = regions::STRIP;
    assert!(shown.elements().iter().all(|e| !e.rect.intersects(&strip)), "clear of the strip");
    // Called out is the whole of it: the announcer says no move's name.
    assert_eq!(sf_video::announcer::AnnouncerLine::for_callout(CalloutKind::Move { word: "HADOKEN!" }), None);
}

#[test]
fn a_full_column_drops_its_oldest_word_and_never_the_count() {
    let mut board = CalloutBoard::new();
    board.set_combo(Side::P1, Some(ComboCount { hits: 3, damage: None }));
    let order = [
        CalloutKind::FirstAttack,
        CalloutKind::Counter,
        CalloutKind::Reversal,
        CalloutKind::CrushCounter,
        CalloutKind::PerfectParry,
    ];
    for kind in order {
        board.step(&[CalloutEvent::new(Side::P1, kind)]);
    }
    let shown = board.frame();
    let want: Vec<_> = order[1..].iter().copied().map(Some).collect();
    assert_eq!(shown.side(Side::P1).words.to_vec(), want, "FIRST ATTACK, the oldest, went");
    assert!(shown.side(Side::P1).combo.is_some());
    assert_eq!(shown.side(Side::P2), &sf_video::callouts::SideCallouts::NONE, "P2's column is its own");
    // Words go once their time is up, and the rest move up in order.
    let mut board = CalloutBoard::new();
    board.step(&[CalloutEvent::new(Side::P2, CalloutKind::Counter)]);
    for _ in 1..20 {
        board.step(&[]);
    }
    board.step(&[CalloutEvent::new(Side::P2, CalloutKind::Perfect)]);
    for _ in 20..WORD_FRAMES {
        board.step(&[]);
    }
    assert_eq!(board.frame().side(Side::P2).words, [Some(CalloutKind::Perfect), None, None, None]);
}

#[test]
fn a_lighter_super_dims_only_the_stage_by_one_quarter() {
    let stage_color = 0xff80_6030;
    let hardware = Frame { argb: vec![stage_color; WIDTH * HEIGHT] };
    let stage = [true, false];
    let mut board = CalloutBoard::new();
    let super_move = CalloutKind::Super { name: NAME };
    board.step(&[CalloutEvent::new(Side::P1, super_move)]);
    let draw = |board: &CalloutBoard| present(
        PresentMode::Overhaul,
        &hardware,
        &OverhaulScene {
            callouts: board.frame_with_quarter_dim([true, false]),
            stage: Some(&stage),
            ..OverhaulScene::EMPTY
        },
    );
    assert_eq!(draw(&board).argb[..2], [0xff60_4824, stage_color]);
    board.step(&[CalloutEvent::new(Side::P2, super_move)]);
    assert_eq!(draw(&board).argb[..2], [half(stage_color), stage_color]);
}

#[test]
fn the_super_dim_stays_one_level_with_two_supers_and_each_name_on_its_side() {
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    let p1_super = CalloutKind::Super { name: "Shinku Hadoken" };
    let p2_super = CalloutKind::Super { name: "Shippu Jinraikyaku" };
    for art in arts() {
        let mut one = CalloutBoard::new();
        one.step(&[CalloutEvent::new(Side::P1, p1_super)]);
        let mut two = CalloutBoard::new();
        two.step(&[CalloutEvent::new(Side::P1, p1_super), CalloutEvent::new(Side::P2, p2_super)]);
        let single = draw(art, gauges(), one.frame().with_static_words(), None, Some(&font));
        let double = draw(art, gauges(), two.frame().with_static_words(), None, Some(&font));
        let stage = 30 * WIDTH + 190;
        assert_eq!(single.argb[stage], half(BACKDROP), "one super dims one level");
        assert_eq!(double.argb[stage], half(BACKDROP), "two supers still dim one level");
        // The fighters stand over the dim.
        let fighter_px = 180 * WIDTH + 145;
        assert_eq!(double.argb[fighter_px], GI);

        let shown = two.frame();
        assert_apart(&shown, "two supers");
        for (side, name) in [(Side::P1, "Shinku Hadoken"), (Side::P2, "Shippu Jinraikyaku")] {
            assert_eq!(shown.side(side).move_name, Some(name));
            let line = regions::move_name_line(side);
            let drawn = shown.elements().into_iter().find(|e| e.side == side && e.part == Part::MoveName).unwrap();
            assert!(line.holds(&drawn.rect));
            assert_eq!(drawn.rect.w, 8 * name.len());
            assert!(pixels_in(&double, drawn.rect).contains(&pens::GOLD), "{side:?}'s name is lettered");
        }
        check_golden(&format!("{}_two_supers", art_slug(art)), &double);

        // Staggered: P1's dim runs out while P2's runs on, still one level.
        let mut board = CalloutBoard::new();
        board.step(&[CalloutEvent::new(Side::P1, p1_super)]);
        for _ in 0..10 {
            board.step(&[]);
        }
        board.step(&[CalloutEvent::new(Side::P2, p2_super)]);
        for t in 11..SUPER_DIM_FRAMES + 20 {
            let lit = board.frame().dim;
            assert_eq!(lit, t < SUPER_DIM_FRAMES + 11, "frame {t}");
            let frame = draw(art, EraHud::NONE, board.frame(), None, None);
            let want = if lit { half(BACKDROP) } else { BACKDROP };
            assert_eq!(frame.argb[stage], want, "frame {t}");
            board.step(&[]);
        }
    }
}

#[test]
fn a_burnt_out_fighter_stays_grey_under_the_dim() {
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    let burnt = EraHud {
        p1: Some(EraGauge::two(
            GaugeRow::stock(6, 0, Fill::EMPTY).with_state(GaugeState::Greyed),
            GaugeRow::level(1, 3, Fill::of(1, 3)),
        )),
        p2: gauges().p2,
    };
    for art in arts() {
        let mut board = CalloutBoard::new();
        board.set_greyed(Side::P1, true);
        board.step(&[
            CalloutEvent::new(Side::P1, CalloutKind::Burnout),
            CalloutEvent::new(Side::P2, CalloutKind::Super { name: NAME }),
        ]);
        let shown = board.frame().with_static_words();
        assert!(shown.dim && shown.side(Side::P1).greyed);
        let frame = draw(art, burnt, shown, None, Some(&font));
        assert_eq!(frame.argb[30 * WIDTH + 190], half(BACKDROP), "the stage is dimmed");
        // P1's fighter: its palette in grey, not dimmed and not its colours.
        let (gi, skin) = (180 * WIDTH + 145, 165 * WIDTH + 145);
        assert_eq!(frame.argb[gi], grey(GI));
        assert_eq!(frame.argb[skin], grey(SKIN));
        let px = frame.argb[skin];
        assert!(px & 0xff == (px >> 8) & 0xff && px & 0xff == (px >> 16) & 0xff, "grey");
        // P2's fighter keeps its colours.
        let p2 = 180 * WIDTH + (WIDTH - 140 - 16) + 5;
        assert_eq!(frame.argb[p2], GI);
        // The greyed gauge row is grey under the dim too: Ruled's rule is the
        // grey pen. (The bars start past the level number's column.) Blocks
        // and Plate grey their own pens, which their goldens hold.
        if art == GaugeArt::Ruled {
            let rule_y = HEIGHT - 8 - sf_video::era_hud::CORNER_HEIGHT;
            assert_eq!(frame.argb[rule_y * WIDTH + 8 + 11], pens::GREY);
        }
        check_golden(&format!("{}_burnout_under_dim", art_slug(art)), &frame);
    }
}

#[test]
fn an_alpha_counter_flashes_its_sides_gauge() {
    let mut board = CalloutBoard::new();
    board.step(&[CalloutEvent::new(Side::P2, CalloutKind::AlphaCounter)]);
    let lit = board.frame().gauges(gauges());
    assert!(matches!(lit.p2.unwrap().main, GaugeRow::Level { state: GaugeState::Flashing { lit: true }, .. }));
    assert_eq!(lit.p1, gauges().p1, "the other side's gauge is untouched");
    for _ in 0..sf_video::callouts::FLASH_PHASE {
        board.step(&[]);
    }
    let unlit = board.frame().gauges(gauges());
    assert!(matches!(unlit.p2.unwrap().main, GaugeRow::Level { state: GaugeState::Flashing { lit: false }, .. }));
    for _ in 0..sf_video::callouts::FLASH_FRAMES {
        board.step(&[]);
    }
    assert_eq!(board.frame().gauges(gauges()), gauges(), "the flash ends");
    // A greyed row does not flash.
    let mut board = CalloutBoard::new();
    board.step(&[CalloutEvent::new(Side::P1, CalloutKind::AlphaCounter)]);
    let greyed = EraHud { p1: Some(EraGauge::one(GaugeRow::stock(6, 0, Fill::EMPTY).with_state(GaugeState::Greyed))), p2: None };
    assert_eq!(board.frame().gauges(greyed), greyed);
    // Drawn: the fill is lit in PLAYER1 gold's highlight.
    let frame = draw(GaugeArt::Ruled, gauges(), {
        let mut b = CalloutBoard::new();
        b.step(&[CalloutEvent::new(Side::P2, CalloutKind::AlphaCounter)]);
        b.frame()
    }, None, None);
    let fill_y = HEIGHT - 8 - 8 + 3;
    assert_eq!(frame.argb[fill_y * WIDTH + WIDTH - 1 - 8 - 12], pens::GOLD_LIGHT);
}

#[test]
fn without_the_rom_font_nothing_is_lettered_but_the_effects_apply() {
    let mut board = CalloutBoard::new();
    board.step(&both(CalloutKind::Super { name: NAME }));
    let frame = present(
        PresentMode::Overhaul,
        &blank(),
        &OverhaulScene { callouts: board.frame(), ..OverhaulScene::EMPTY },
    );
    assert!(frame.argb.iter().all(|p| *p == half(BACKDROP)));
}

#[test]
fn faithful_present_ignores_the_call_outs() {
    let mut board = CalloutBoard::new();
    board.set_greyed(Side::P1, true);
    let mut events = Vec::new();
    for kind in CalloutKind::all(NAME) {
        events.extend(both(kind));
    }
    board.step(&events);
    let (p1, p2) = (fighter(Side::P1), fighter(Side::P2));
    let frame = present(
        PresentMode::Faithful,
        &blank(),
        &OverhaulScene { callouts: board.frame(), fighters: [&p1, &p2], hud: gauges(), ..OverhaulScene::EMPTY },
    );
    assert_eq!(frame.argb, blank().argb);
}

#[test]
fn an_idle_board_draws_nothing_and_the_same_events_draw_the_same() {
    let board = CalloutBoard::new();
    assert!(board.frame().is_empty());
    let frame = present(PresentMode::Overhaul, &blank(), &OverhaulScene { callouts: board.frame(), ..OverhaulScene::EMPTY });
    assert_eq!(frame.argb, blank().argb);
    let run = || {
        let mut board = CalloutBoard::new();
        let mut frames = Vec::new();
        for t in 0..120usize {
            let kinds = CalloutKind::all(NAME);
            board.step(&[CalloutEvent::new(if t % 2 == 0 { Side::P1 } else { Side::P2 }, kinds[t % kinds.len()])]);
            frames.push(board.frame());
        }
        frames
    };
    assert_eq!(run(), run());
}

#[test]
fn the_combo_count_is_a_white_number_over_hits_in_its_slot() {
    use sf_video::callouts::{damage_line, COMBO_DAMAGE_LINE, COMBO_HITS_LINE, NUMBER_PENS};
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    for art in arts() {
        for (name, damage) in [("combo", None), ("combo_damage", Some(23))] {
            let mut board = CalloutBoard::new();
            board.set_combo(Side::P1, Some(ComboCount { hits: 3, damage }));
            board.set_combo(Side::P2, Some(ComboCount { hits: 12, damage }));
            board.step(&[]);
            let shown = board.frame();
            let frame = present(
                PresentMode::Overhaul,
                &blank(),
                &OverhaulScene { gauge_art: art, font: Some(&font), callouts: shown, ..OverhaulScene::EMPTY },
            );
            assert_boxes_hold_the_drawing(&frame, &shown, name);
            for (side, digits) in [(Side::P1, 1), (Side::P2, 2)] {
                let slot = regions::combo_slot(side);
                // The number: 16 pixels a digit from the side's edge, in white.
                let number = match side {
                    Side::P1 => regions::Rect::new(slot.x, slot.y, 16 * digits, 16),
                    Side::P2 => regions::Rect::new(slot.x + slot.w - 16 * digits, slot.y, 16 * digits, 16),
                };
                let px = pixels_in(&frame, number);
                assert!(px.contains(&NUMBER_PENS[2]) && px.contains(&NUMBER_PENS[1]), "{name} {side:?}: a white number");
                assert!(!px.contains(&pens::GOLD), "{name} {side:?}: the number is not gold");
                // HITS in gold under it, and the damage line under that (Training).
                let hits = regions::text_box(side, slot.y + COMBO_HITS_LINE, "HITS");
                assert!(pixels_in(&frame, hits).contains(&pens::GOLD), "{name} {side:?}: HITS");
                let line = regions::text_box(side, slot.y + COMBO_DAMAGE_LINE, &damage_line(23));
                assert_eq!(pixels_in(&frame, line).contains(&pens::GOLD), damage.is_some(), "{name} {side:?}: damage");
                assert!(slot.holds(&number) && slot.holds(&hits) && slot.holds(&line));
            }
            check_golden(&format!("{}_{name}", art_slug(art)), &frame);
        }
    }
}

#[test]
fn a_fix_is_a_word_in_the_column_in_any_notation_and_keeps_clear_of_everything() {
    // The coach's fix (ML6) is worded for the chosen notation: the arrow is a
    // glyph made in the ROM font's style, a numpad digit or initials are the
    // font's own. Each takes a word slot, fits the column and the announcer
    // says nothing for it.
    let words = [
        "TOO SLOW",
        "MISSED \u{2198}",
        "MISSED \u{2190}",
        "MISSED 3",
        "MISSED DF",
        "PRESS P SOONER",
        "PRESS K SOONER",
        "BUTTON TOO EARLY",
        "WAIT TO RECOVER",
    ];
    for (word, drop) in words.into_iter().flat_map(|w| [(w, 0), (w, regions::TRY_DROP)]) {
        let kind = CalloutKind::fix(word);
        assert_eq!(kind.word(), word);
        assert!(word.chars().count() <= regions::LINE_LETTERS, "{word}");
        assert_eq!(sf_video::announcer::AnnouncerLine::for_callout(kind), None, "{word}");
        let mut board = CalloutBoard::new();
        board.set_drop(drop);
        board.set_combo(Side::P1, Some(ComboCount { hits: 3, damage: Some(20) }));
        board.step(&[CalloutEvent::new(Side::P1, CalloutKind::Move { word: "HADOKEN!" })]);
        board.step(&[CalloutEvent::new(Side::P1, kind)]);
        let shown = board.frame();
        assert_eq!(shown.side(Side::P1).words[1], Some(kind));
        assert_apart(&shown, word);
        let element = shown.elements().into_iter().find(|e| e.part == Part::Word(1)).unwrap();
        assert_eq!(element.rect.w, 8 * word.chars().count(), "{word}: a cell a character");
        assert!(shown.elements().iter().all(|e| !e.rect.intersects(&regions::STRIP)));
        if drop > 0 {
            assert!(shown.elements().iter().all(|e| !e.rect.intersects(&regions::STRIP_INPUTS)), "{word}: clear of the strip's second line");
        }
    }
    // A line longer than a column is cut at its letters, on a character.
    let long = CalloutKind::fix("MISSED \u{2198} MISSED \u{2198} MISSED \u{2198} MISSED \u{2198}");
    assert_eq!(long.word().chars().count(), regions::LINE_LETTERS);
}

#[test]
fn a_fix_is_lettered_in_gold_arrows_included() {
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    for word in ["TOO SLOW", "MISSED \u{2198}", "MISSED 3", "PRESS P SOONER"] {
        let mut board = CalloutBoard::new();
        board.step(&[CalloutEvent::new(Side::P1, CalloutKind::fix(word))]);
        let shown = board.frame().with_static_words();
        let frame = draw(GaugeArt::Ruled, EraHud::NONE, shown, None, Some(&font));
        let slot = regions::word_slot(Side::P1, 0);
        let rect = regions::text_box(Side::P1, slot.y, word);
        let pixels = pixels_in(&frame, rect);
        assert!(pixels.contains(&LETTER_PENS[2]), "{word}: gold body");
        // Every character but the blanks inks its own cell, the arrow too.
        for (i, ch) in word.chars().enumerate().filter(|(_, ch)| *ch != ' ') {
            let cell = regions::Rect::new(rect.x + 8 * i, rect.y, 8, 8);
            assert!(pixels_in(&frame, cell).iter().any(|p| *p != BACKDROP), "{word}: {ch} is drawn");
        }
        // Nothing is inked in the column outside the word's own box.
        let column = regions::callout_column(Side::P1);
        for y in column.y..column.y + column.h {
            for x in column.x..column.x + column.w {
                assert!(frame.argb[y * WIDTH + x] == BACKDROP || rect.contains(x, y), "{word}: ink outside the word at ({x}, {y})");
            }
        }
    }
}

#[test]
fn the_players_inputs_line_sits_under_the_strip_and_keeps_its_own_region() {
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    let (strip, inputs) = (regions::STRIP, regions::STRIP_INPUTS);
    assert_eq!((inputs.x, inputs.w, inputs.h), (strip.x, strip.w, strip.h));
    assert!(inputs.y >= strip.y + strip.h, "under the strip, not over it");
    // The line is drawn only in a Try, whose columns sit lower to leave it room.
    for side in Side::BOTH {
        assert!(inputs.y + inputs.h <= regions::callout_column_at(side, regions::TRY_DROP).y, "{side:?}: above the column");
    }
    assert!(inputs.y >= regions::HEALTH_BLOCK.y + regions::HEALTH_BLOCK.h);
    let labels: Vec<String> = ["D", "DF", "F", "P"].map(String::from).to_vec();
    let mut frame = blank();
    sf_video::callouts::draw_strip_inputs(&mut frame.argb, &labels, Some(&font));
    // Drawn inside its line, centred, and nowhere else.
    let width = 8 * "D DF F P".len();
    let line = regions::Rect::new(inputs.x + (inputs.w - width) / 2, inputs.y, width, 8);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if frame.argb[y * WIDTH + x] != BACKDROP {
                assert!(line.contains(x, y), "drawn outside its line at ({x}, {y})");
            }
        }
    }
    assert!(pixels_in(&frame, line).contains(&sf_video::callouts::STRIP_INPUTS_PENS[2]));
    // Too many to fit: the oldest labels go, the newest stay.
    let many: Vec<String> = (0..20).map(|n| if n == 19 { "P".to_owned() } else { "DF".to_owned() }).collect();
    let mut frame = blank();
    sf_video::callouts::draw_strip_inputs(&mut frame.argb, &many, Some(&font));
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            if frame.argb[y * WIDTH + x] != BACKDROP {
                assert!(inputs.contains(x, y), "a long line stays inside the strip's width ({x}, {y})");
            }
        }
    }
    // Nothing without the font.
    let mut frame = blank();
    sf_video::callouts::draw_strip_inputs(&mut frame.argb, &labels, None);
    assert!(frame.argb.iter().all(|p| *p == BACKDROP));
}

#[test]
fn a_super_names_hyphen_is_drawn_in_the_roms_style_where_the_font_has_none() {
    // The call-outs' lettering reaches the glyphs made in the ROM font's style
    // (the hyphen, the move list's arrows), not only the ROM's own tiles; the
    // ROM has no hyphen, so `Ansatsu Geki-Retsuzan` used to leave a gap there.
    let Some(assets) = assets() else { return };
    let font = RomFont::new(&assets.gfx4, &assets.maincpu);
    let name = "Ansatsu Geki-Retsuzan";
    let mut board = CalloutBoard::new();
    board.step(&[CalloutEvent::new(Side::P1, CalloutKind::Super { name })]);
    let shown = board.frame();
    let frame = draw(GaugeArt::Ruled, EraHud::NONE, shown, None, Some(&font));
    let line = shown.elements().into_iter().find(|e| e.part == Part::MoveName).unwrap().rect;
    let at = name.chars().position(|c| c == '-').unwrap();
    let cell = regions::Rect::new(line.x + 8 * at, line.y, 8, 8);
    let ink = pixels_in(&frame, cell);
    // (A super dims the stage, so the clear cells are the dimmed backdrop.)
    // The hyphen is the made glyph's two rows in the body gold with its shadow
    // under them, centred in the cell's middle rows, and nothing above or below.
    assert!(ink.contains(&LETTER_PENS[2]) && ink.contains(&LETTER_PENS[1]), "the hyphen is inked in the gold's body and shadow");
    for dy in [0, 1, 2, 7] {
        assert!(pixels_in(&frame, regions::Rect::new(cell.x, cell.y + dy, 8, 1)).iter().all(|p| *p == half(BACKDROP)), "row {dy} of the hyphen's cell is clear");
    }
    // Its neighbours are the ROM's own letters, drawn as before.
    assert!(pixels_in(&frame, regions::Rect::new(cell.x - 8, cell.y, 8, 8)).contains(&LETTER_PENS[2]));
    assert!(pixels_in(&frame, regions::Rect::new(cell.x + 8, cell.y, 8, 8)).contains(&LETTER_PENS[2]));
}

#[test]
fn dramatic_ally2_effects_follow_its_own_gauge_and_style() {
    let hud = gauges();
    let ally2 = hud.p2.unwrap();
    let mut board = CalloutBoard::new();
    board.step_with_ally2(&[], [CalloutKind::AlphaCounter], true);
    assert_eq!(board.frame().gauges(hud), hud);
    assert_eq!(
        board.ally2_gauge(ally2).main,
        ally2.main.with_state(GaugeState::Flashing { lit: true })
    );
    assert_eq!(board.frame().side(Side::P1).words[0], Some(CalloutKind::AlphaCounter));

    let mut shared = CalloutBoard::new();
    shared.step_with_ally2(&[], [CalloutKind::AlphaCounter], false);
    assert_ne!(shared.frame().gauges(hud).p1, hud.p1);
    assert_eq!(shared.ally2_gauge(ally2), ally2);

    let mut supers = CalloutBoard::new();
    let super_callout = CalloutKind::Super { name: "SUPER" };
    supers.step_with_ally2(&[], [super_callout], true);
    assert!(supers.frame_with_ally2_quarter_dim([false; 2], true).dim_quarter);
    supers.step(&[CalloutEvent::new(Side::P1, super_callout)]);
    assert!(!supers.frame_with_ally2_quarter_dim([false; 2], true).dim_quarter);
}
