//! Ticket #99 (O8): the extended compositor and the asset-pack rules.
//!
//! These tests build their own pixels. They do not read arcade ROMs.

use sf_video::asset_pack::{
    decode_adpcm, encode_adpcm, quantize_sheet, simple_fm_patch, validate_adpcm, validate_fm_patch,
    validate_pack, validate_tile, AdpcmClip, FmPatch, Rgba,
};
use sf_video::compositor::{
    present, present_with_move_effects, render_player_select, select_cell_origin, AfterimageTrail,
    IndexedSprite, OverhaulScene,
    PaletteTint, PresentMode, SelectPortrait,
    SELECT_BLUE, SELECT_GOLD, SELECT_YELLOW,
};
use sf_video::era_hud::{EraGauge, EraHud, Fill, GaugeRow};
use sf_video::move_effects::signature::{
    denjin_hands, signature_impact, signature_projectile, ImpactKind, ProjectileKind,
};
use sf_video::move_effects::{effect_frame, EffectKind};
use sf_video::render::{Frame, HEIGHT, WIDTH};

fn blank() -> Frame {
    Frame {
        argb: vec![0xff_11_22_33; WIDTH * HEIGHT],
    }
}

#[test]
fn signature_effects_stay_anchored_when_flipped_and_impacts_expire() {
    let center = (72, 80);
    let draw = |sprites: &[IndexedSprite]| {
        present_with_move_effects(
            PresentMode::Overhaul,
            &blank(),
            &OverhaulScene::EMPTY,
            sprites,
        )
    };
    let check_frames = |frame: &dyn Fn(u8, bool) -> Vec<IndexedSprite>, duration: u8| {
        for age in 0..duration {
            let right = draw(&frame(age, false));
            let left = draw(&frame(age, true));
            assert_ne!(right.argb, blank().argb, "effect frame {age} has no color");
            for row in 48..112 {
                for x in 42..=102 {
                    assert_eq!(
                        right.argb[row * WIDTH + x],
                        left.argb[row * WIDTH + 2 * center.0 as usize - x]
                    );
                }
            }
        }
    };
    check_frames(
        &|age, flip| denjin_hands(age, center.0, center.1, flip),
        8,
    );
    for kind in [ProjectileKind::Denjin, ProjectileKind::TigerCannon] {
        check_frames(
            &|age, flip| signature_projectile(kind, age, center.0, center.1, flip),
            12,
        );
    }
    for kind in [
        ImpactKind::Denjin,
        ImpactKind::TigerCannon,
        ImpactKind::Zanei,
        ImpactKind::LightHit,
        ImpactKind::MediumHit,
        ImpactKind::HeavyHit,
        ImpactKind::DriveImpactHit,
        ImpactKind::DriveImpactGuard,
        ImpactKind::SlamDust,
    ] {
        check_frames(
            &|age, flip| signature_impact(kind, age, center.0, center.1, flip),
            kind.lifetime(),
        );
        assert!(signature_impact(kind, kind.lifetime(), center.0, center.1, false).is_empty());
    }
}

#[test]
fn hit_sparks_increase_in_size_with_strength() {
    let frame = |kind| signature_impact(kind, 0, 72, 80, false);
    let colored_pixels = |kind| {
        frame(kind).iter()
            .map(|sprite| sprite.pixels.iter().filter(|&&pen| pen != 0).count())
            .sum::<usize>()
    };
    let light = colored_pixels(ImpactKind::LightHit);
    let medium = colored_pixels(ImpactKind::MediumHit);
    let heavy = colored_pixels(ImpactKind::HeavyHit);
    let drive = colored_pixels(ImpactKind::DriveImpactHit);
    assert!(light < medium && medium < heavy && heavy * 2 < drive);
    assert_ne!(frame(ImpactKind::DriveImpactHit), frame(ImpactKind::DriveImpactGuard));
}

#[test]
fn embedded_move_effects_have_visible_frames_and_only_draw_in_overhaul() {
    let kinds = [
        EffectKind::LandingDust,
        EffectKind::Parry,
        EffectKind::KenFire,
        EffectKind::KenBurn,
    ];
    for kind in kinds {
        for age in 0..kind.lifetime() {
            let sprite = effect_frame(kind, age, 40, 40, false).expect("active effect frame");
            assert!(sprite.pixels.iter().any(|&pen| pen != 0), "{kind:?} at {age}");
        }
        let next = effect_frame(kind, kind.lifetime(), 40, 40, false);
        if kind == EffectKind::KenFire {
            assert_eq!(next, effect_frame(kind, 0, 40, 40, false));
        } else {
            assert!(next.is_none());
        }
    }

    let sprites = signature_impact(ImpactKind::HeavyHit, 0, 40, 40, false);
    let sprite = &sprites[0];
    let index = sprite.pixels.iter().position(|&pen| pen != 0).unwrap();
    let x = sprite.x + (index % 16) as i32;
    let y = sprite.y + (index / 16) as i32;
    let fighter = [IndexedSprite::solid(x, y, 0xff_20_40_60)];
    let scene = OverhaulScene {
        fighters: [&fighter, &[]],
        ..OverhaulScene::EMPTY
    };
    let hardware = blank();
    let effects = &sprites;
    let faithful = present_with_move_effects(PresentMode::Faithful, &hardware, &scene, effects);
    assert_eq!(faithful.argb, hardware.argb);
    let overhaul = present_with_move_effects(PresentMode::Overhaul, &hardware, &scene, effects);
    assert_eq!(
        overhaul.argb[y as usize * WIDTH + x as usize],
        sprite.palette[usize::from(sprite.pixels[index])]
    );
}

#[test]
fn faithful_present_ignores_extra_sprites_and_the_hud() {
    let hardware = blank();
    let sprites = vec![IndexedSprite::solid(0, 0, 0xff_ff_00_00); 80];
    let lines = vec!["RYU".to_string()];
    let stock = EraGauge::one(GaugeRow::stock(1, 1, Fill::FULL));
    let presented = present(
        PresentMode::Faithful,
        &hardware,
        &OverhaulScene {
            sprites: &sprites,
            hud: EraHud { p1: Some(stock), p2: Some(stock) },
            select_lines: &lines,
            training: Some(&[0xff_00_ff_00; WIDTH * HEIGHT]),
            ..OverhaulScene::EMPTY
        },
    );
    assert_eq!(presented.argb, hardware.argb);
}

#[test]
fn overhaul_draws_more_than_sixty_four_sprites_and_sixteen_palettes() {
    let hardware = blank();
    let mut sprites = Vec::new();
    for i in 0..80 {
        let color = 0xff_00_00_00 | ((i as u32 + 1) * 0x0103);
        sprites.push(IndexedSprite::solid((i % 24) * 16, (i / 24) * 16, color));
    }
    let first = present(
        PresentMode::Overhaul,
        &hardware,
        &OverhaulScene {
            sprites: &sprites,
            ..OverhaulScene::EMPTY
        },
    );
    let presented = present(
        PresentMode::Overhaul,
        &hardware,
        &OverhaulScene {
            sprites: &sprites,
            ..OverhaulScene::EMPTY
        },
    );
    assert_eq!(first.argb, presented.argb, "a still scene must not flicker");
    let mut colours = std::collections::BTreeSet::new();
    for sprite in &sprites {
        let x = sprite.x as usize;
        let y = sprite.y as usize;
        let px = presented.argb[y * WIDTH + x];
        assert_ne!(px, hardware.argb[0]);
        colours.insert(px);
    }
    assert!(sprites.len() > 64);
    assert!(colours.len() > 16, "saw {} palettes", colours.len());
    assert_eq!(presented.argb.len(), WIDTH * HEIGHT);
}

#[test]
fn fighter_tints_leave_the_other_side_untouched_and_trails_keep_three_poses() {
    let hardware = blank();
    let mut trail = AfterimageTrail::default();
    for x in 12..16 {
        trail.record(&[IndexedSprite::solid(x, 20, 0xffcc6633)]);
    }
    let fighter = [IndexedSprite::solid(15, 20, 0xffcc6633)];
    let opponent = [IndexedSprite::solid(16, 20, 0xff224488)];
    let tint = PaletteTint { color: 0xffffffff, strength: 128 };
    let scene = OverhaulScene {
        fighters: [&fighter, &opponent],
        fighter_tints: [Some(tint), None],
        afterimages: [Some(&trail), None],
        ..OverhaulScene::EMPTY
    };
    let offset = 20 * WIDTH;
    let frame = present(PresentMode::Overhaul, &hardware, &scene);
    assert_eq!(frame.argb[offset + 12], hardware.argb[offset + 12]);
    assert_ne!(frame.argb[offset + 13], hardware.argb[offset + 13]);
    assert_ne!(frame.argb[offset + 14], frame.argb[offset + 13]);
    assert_eq!(frame.argb[offset + 15], tint.apply(0xffcc6633));
    assert_eq!(frame.argb[offset + 16], 0xff224488);
    assert_eq!(present(PresentMode::Faithful, &hardware, &scene).argb, hardware.argb);
    trail.clear();
    let frame = present(PresentMode::Overhaul, &hardware, &OverhaulScene {
        afterimages: [Some(&trail), None],
        ..OverhaulScene::EMPTY
    });
    assert_eq!(frame.argb, hardware.argb);
}

#[test]
fn select_type_and_training_draw_and_no_strip_is_left_at_the_bottom() {
    let hardware = blank();
    let lines = vec!["RYU".to_string(), "SAGAT".to_string()];
    let mut training = vec![0u32; WIDTH * HEIGHT];
    training[40 * WIDTH + 40] = 0xff_00_ff_88;
    let presented = present(
        PresentMode::Overhaul,
        &hardware,
        &OverhaulScene {
            sprites: &[],
            select_lines: &lines,
            training: Some(&training),
            ..OverhaulScene::EMPTY
        },
    );
    assert_eq!(presented.argb[100 * WIDTH + 200], hardware.argb[0]);
    let bottom = &presented.argb[(HEIGHT - 16) * WIDTH..];
    assert!(
        bottom.iter().all(|px| *px == hardware.argb[0]),
        "with no era gauges the bottom of the frame is the hardware's"
    );
    assert_eq!(
        presented.argb[0], 0xff_e8_d8_b0,
        "select type starts at the top left"
    );
    assert_eq!(
        presented.argb[6 * WIDTH],
        0xff_e8_d8_b0,
        "the next select line is its own row"
    );
    assert_eq!(presented.argb[40 * WIDTH + 40], 0xff_00_ff_88);
    assert_eq!(presented.argb[40 * WIDTH + 41], hardware.argb[0]);
    assert_eq!(presented.argb.len(), WIDTH * HEIGHT);
}

#[test]
fn quantizer_emits_sixteen_by_sixteen_tiles_with_a_transparent_pen() {
    let mut sheet = vec![Rgba::TRANSPARENT; 32 * 16];
    for px in sheet.iter_mut().take(16) {
        *px = Rgba {
            r: 200,
            g: 10,
            b: 10,
            a: 255,
        };
    }
    let tiles = quantize_sheet(&sheet, 32, 16).expect("sheet");
    assert_eq!(tiles.len(), 2);
    validate_pack(&tiles, &[], &[simple_fm_patch(0, 0x10)]).unwrap();
    assert_eq!(tiles[0].pixels[0], 1);
    assert_eq!(tiles[0].pixels[16], 0);
    let mut too_many = tiles[0].clone();
    too_many.pixels[0] = 16;
    assert!(validate_tile(&too_many).is_err());
}

#[test]
fn adpcm_and_fm_patches_obey_the_1987_encoding_rules() {
    let pcm: Vec<i16> = (0..64).map(|i| (i * 400) as i16).collect();
    let clip = encode_adpcm(&pcm).unwrap();
    // The hardware's rate: 4 kHz a chip (ADR 0011).
    assert_eq!(clip.sample_rate, 4_000);
    assert_eq!(clip.nibbles.len(), pcm.len());
    validate_adpcm(&clip).unwrap();
    for rate in [22_050, 8_000] {
        let bad = AdpcmClip {
            sample_rate: rate,
            nibbles: vec![1],
        };
        assert!(validate_adpcm(&bad).is_err(), "{rate} Hz is refused");
    }
    let wide = AdpcmClip {
        sample_rate: 4_000,
        nibbles: vec![16],
    };
    assert!(validate_adpcm(&wide).is_err());

    let patch = simple_fm_patch(1, 0x20);
    validate_fm_patch(&patch).unwrap();
    let incomplete = FmPatch {
        regs: vec![(0x08, 0x78)],
    };
    assert!(validate_fm_patch(&incomplete).is_err());
}

#[test]
fn a_generated_sheet_keeps_the_reference_box_and_the_pack_validates() {
    use sf_video::asset_pack::{
        conform_to_proportion, proportion_from_decoded_sprite, proportion_from_tile,
        validate_pack_dir, write_pack, AssetPack, IndexedTile, ProportionRef,
    };

    let mut reference = IndexedTile {
        pixels: [0; 256],
        palette: [Rgba::TRANSPARENT; 16],
    };
    reference.palette[1] = Rgba {
        r: 10,
        g: 10,
        b: 10,
        a: 255,
    };
    for y in 4..12 {
        for x in 3..13 {
            reference.pixels[y * 16 + x] = 1;
        }
    }
    let mut decoded = [[15u8; 16]; 16];
    for y in 2..10 {
        for x in 1..8 {
            decoded[y][x] = 0;
        }
    }
    assert_eq!(
        proportion_from_decoded_sprite(&decoded),
        ProportionRef {
            left: 1,
            top: 2,
            right: 7,
            bottom: 9,
        }
    );
    let roms = std::path::Path::new("../../tools/reference/local/roms/sf");
    let manifest = std::path::Path::new("../../tools/reference/rom-manifest.json");
    if roms.is_dir() && manifest.is_file() {
        let assets = sf_video::romset::load_assets(roms, manifest).expect("local ROM set");
        let tile_count = assets.gfx3.len() / 128;
        let found = (0..tile_count).find_map(|index| {
            let tile = sf_video::decode::decode_sprite_tile(&assets.gfx3, index);
            let prop = proportion_from_decoded_sprite(&tile);
            (prop.right > prop.left || prop.bottom > prop.top).then_some(prop)
        });
        assert!(
            found.is_some(),
            "a decoded 1987 sprite must yield a proportion box"
        );
    } else {
        eprintln!("skipping decoded-ROM proportion: local ROM set not present");
    }

    let prop = proportion_from_tile(&reference);
    assert_eq!(
        prop,
        ProportionRef {
            left: 3,
            top: 4,
            right: 12,
            bottom: 11,
        }
    );

    let authored = vec![
        Rgba {
            r: 240,
            g: 30,
            b: 20,
            a: 255,
        };
        8 * 8
    ];
    let fitted = conform_to_proportion(&authored, 8, 8, prop).unwrap();
    assert_eq!(fitted.len(), 256);
    assert_eq!(fitted[4 * 16 + 3].a, 255);
    assert_eq!(fitted[0].a, 0);
    let tiles = quantize_sheet(&fitted, 16, 16).unwrap();
    assert_eq!(proportion_from_tile(&tiles[0]).left, 3);

    let held = decode_adpcm(&encode_adpcm(&[1000; 80]).unwrap()).unwrap();
    assert!(
        held.last().copied().unwrap_or(0) > 900,
        "MSM5205 tone {held:?}"
    );
    let pcm: Vec<i16> = (0..32).map(|i| i * 100).collect();
    let pack = AssetPack {
        tiles,
        clips: vec![encode_adpcm(&pcm).unwrap()],
        patches: vec![simple_fm_patch(0, 0x18)],
        proportions: vec![prop],
    };
    let dir = std::env::temp_dir().join("sf1-port-o8-pack");
    write_pack(&dir, &pack).unwrap();
    let loaded = validate_pack_dir(&dir).unwrap();
    assert_eq!(loaded.tiles.len(), 1);
    assert_eq!(loaded.clips[0].sample_rate, 4_000);

    let mut broken = pack.clone();
    broken.clips[0].sample_rate = 44_100;
    write_pack(&dir, &broken).unwrap_err();
}

#[test]
fn portrait_select_moves_the_yellow_cursor_and_blits_the_bust() {
    let names = [
        "RYU", "KEN", "RETSU", "GEKI", "JOE", "MIKE", "LEE", "GEN", "BIRDIE", "EAGLE", "ADON", "SAGAT",
    ];
    let mut portraits: [Option<SelectPortrait>; 12] = [None, None, None, None, None, None, None, None, None, None, None, None];
    let red = [0xff_e0_20_20; 4];
    portraits[0] = Some(SelectPortrait { argb: &red, width: 2, height: 2 });
    let at = |frame: &Frame, slot: usize| {
        let (x, y) = select_cell_origin(slot);
        frame.argb[y * WIDTH + x]
    };
    let ryu = render_player_select(&names, 0, "RYU", &portraits);
    assert_eq!(ryu.argb.len(), WIDTH * HEIGHT);
    assert_eq!(at(&ryu, 0), SELECT_YELLOW);
    assert_eq!(at(&ryu, 1), SELECT_GOLD);
    assert_eq!(ryu.argb[12 * WIDTH + 140], SELECT_BLUE);
    assert_eq!(ryu.argb[40 * WIDTH + 18], 0xff_e0_20_20);
    let ken = render_player_select(&names, 1, "KEN", &portraits);
    assert_eq!(at(&ken, 1), SELECT_YELLOW);
    assert_eq!(at(&ken, 0), SELECT_GOLD);
    assert_ne!(ken.argb[40 * WIDTH + 18], 0xff_e0_20_20);
}

#[test]
fn a_heavy_contact_draws_a_short_spark_without_moving_the_scene_or_hud() {
    let hardware = blank();
    let gauge = EraGauge::one(GaugeRow::stock(1, 1, Fill::FULL));
    let hud = EraHud { p1: Some(gauge), p2: None };
    let baseline = present(PresentMode::Overhaul, &hardware, &OverhaulScene { hud, ..OverhaulScene::EMPTY });
    let sparks = [sf_video::compositor::HeavyImpactSpark { x: 190, y: 105, age: 0 }];
    let scene = OverhaulScene { hud, heavy_impacts: &sparks, ..OverhaulScene::EMPTY };
    let shown = present(PresentMode::Overhaul, &hardware, &scene);

    assert_ne!(shown.argb[105 * WIDTH + 190], baseline.argb[105 * WIDTH + 190]);
    for (index, (before, after)) in baseline.argb.iter().zip(&shown.argb).enumerate() {
        if before != after {
            assert!((178..=202).contains(&(index % WIDTH)));
            assert!((93..=117).contains(&(index / WIDTH)));
        }
    }
    assert_eq!(present(PresentMode::Faithful, &hardware, &scene).argb, hardware.argb);
    let spent = [sf_video::compositor::HeavyImpactSpark { age: 6, ..sparks[0] }];
    let settled = OverhaulScene { hud, heavy_impacts: &spent, ..OverhaulScene::EMPTY };
    assert_eq!(present(PresentMode::Overhaul, &hardware, &settled).argb, baseline.argb);
}
