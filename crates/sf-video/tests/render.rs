//! Frame rendering: layer composition, scroll, transparency, and sprites,
//! transcribed from sf.cpp @ mame0289 (video_start, draw_sprites,
//! screen_update, get_*_tile_info).

use sf_video::compositor::PaletteTint;
use sf_video::render::{
    render_frame, render_frame_staged_with_art, SpriteArt, SpriteArtOverride, SpriteTint,
    TextEdits, HEIGHT, WIDTH,
};
use sf_video::romset::Assets;
use sf_video::scene::SceneState;

/// Assets with every region zero-filled except where a test sets bytes.
fn empty_assets() -> Assets {
    Assets {
        maincpu: Vec::new(),
        gfx1: vec![0u8; 128], // 1 sprite-layout tile
        gfx2: vec![0u8; 128],
        gfx3: vec![0u8; 128],
        gfx4: vec![0u8; 16], // 1 char tile
        tilerom: vec![0u8; 0x40000],
    }
}

fn empty_scene() -> SceneState {
    SceneState {
        videoram: vec![0u8; 0x1000],
        objectram: vec![0u8; 0x2000],
        paletteram: vec![0u8; 0x800],
        bg_scroll: 0,
        fg_scroll: 0,
        active: 0,
        flip_screen: false,
        bg_page: 0,
        fg_page: 0,
    }
}

fn set_palette_entry(scene: &mut SceneState, index: usize, value: u16) {
    scene.paletteram[2 * index] = (value >> 8) as u8;
    scene.paletteram[2 * index + 1] = value as u8;
}

#[test]
fn disabled_layers_fall_back_to_palette_entry_zero() {
    let assets = empty_assets();
    let mut scene = empty_scene();
    scene.active = 0x00; // bg disabled -> fill pen 0
    set_palette_entry(&mut scene, 0, 0x0f00); // red at max
    let frame = render_frame(&assets, &scene);
    assert_eq!(frame.argb[0], 0xffff0000);
    assert_eq!(frame.argb[WIDTH * HEIGHT - 1], 0xffff0000);
}

#[test]
fn alternate_sprite_art_applies_only_to_its_entry_and_keeps_the_tint() {
    let assets = empty_assets();
    let mut scene = empty_scene();
    scene.active = 0x80;
    scene.objectram[4..8].copy_from_slice(&[0, 16, 0, 64]);
    scene.objectram[0x44..0x48].copy_from_slice(&[0, 16, 0, 80]);
    set_palette_entry(&mut scene, 512, 0x0f0);

    let mut palette = [0; 16];
    palette[1] = 0xf80;
    let mut replacement = [[0; 16]; 16];
    replacement[0][0] = 1;
    replacement[0][1] = 15;
    let art = SpriteArt::new(palette, [(0, replacement)]);
    let art = [SpriteArtOverride { entries: 0..0x40, art: &art }];
    let tint = PaletteTint { color: 0xff0000ff, strength: 64 };
    let tints = [SpriteTint { entries: 0..0x40, tint }];
    let (frame, stage) = render_frame_staged_with_art(&assets, &scene, &TextEdits::default(), &tints, &art);

    assert_eq!(frame.argb[0], tint.apply(0xffff8800));
    assert_eq!(frame.argb[1], 0xff000000);
    assert_eq!(frame.argb[16], 0xff00ff00);
    assert!(!stage[0]);
    assert!(stage[1]);
    assert!(!stage[16]);
}

/// Screen pixel (0,0) is bitmap (64,16) = text tile (8,2) = videoram word
/// index 2*64+8.
const ORIGIN_WORD: usize = 2 * 64 + 8;

#[test]
fn the_text_layer_draws_char_tiles_from_videoram() {
    let mut assets = empty_assets();
    let mut scene = empty_scene();
    scene.active = 0x08; // tx only
                         // Char tile 0: make pixel (0,0) pen 1 (bit0 at offset 0: byte 0, 0x80).
    assets.gfx4[0] = 0x80;
    // Origin word: code 0, color 2 -> palette index 768 + 2*4 + 1 (2bpp
    // strides the palette by 4 pens per color).
    scene.videoram[2 * ORIGIN_WORD] = 0x20; // word 0x2000 -> color 2, code 0
    scene.videoram[2 * ORIGIN_WORD + 1] = 0x00;
    set_palette_entry(&mut scene, 0, 0x0000);
    set_palette_entry(&mut scene, 768 + 9, 0x0abc);

    let frame = render_frame(&assets, &scene);
    assert_eq!(frame.argb[0], 0xffaabbcc, "tx tile pixel at screen (0,0)");
    // Pen 0 is drawn on the text layer (only pen 3 is transparent):
    // screen (1,0) is the same tile's pixel (1,0) -> color 2 pen 0, palette
    // index 768+8, zero-filled -> black.
    assert_eq!(frame.argb[1], 0xff000000);
    // Screen (8,0) is tile (9,2), word zeroed -> code 0 color 0 pen 0 ->
    // palette index 768, zero-filled -> black.
    assert_eq!(frame.argb[8], 0xff000000);
}

#[test]
fn tx_pen_3_is_transparent_but_other_pens_are_not() {
    let mut assets = empty_assets();
    let mut scene = empty_scene();
    scene.active = 0x08;
    // Char tile 0, pixel (0,0) pen 3 (both planes: byte 0 -> 0x88).
    assets.gfx4[0] = 0x88;
    // pixel (1,0) pen 2 (bit1 only): bit1 at offset 4+x=5 -> byte 0, 0x04.
    assets.gfx4[0] |= 0x04;
    // pixel (2,0) pen 1 (bit0 only): bit0 at offset x=2 -> byte 0, 0x20.
    assets.gfx4[0] |= 0x20;
    set_palette_entry(&mut scene, 0, 0x0111);
    set_palette_entry(&mut scene, 768 + 1, 0x0f0f);
    set_palette_entry(&mut scene, 768 + 2, 0x00f0);

    let frame = render_frame(&assets, &scene);
    assert_eq!(frame.argb[0], 0xff111111, "pen 3 shows the backdrop");
    assert_eq!(frame.argb[1], 0xff00ff00, "pen 2 drawn");
    assert_eq!(frame.argb[2], 0xffff00ff, "pen 1 drawn");
}
