//! Palette and scene-state handling, verified against the reference
//! hardware format (palette_device::xRGB_444 over 1024 u16 entries).

use sf_video::palette::Palette;

#[test]
fn palette_entries_are_xrgb444_big_endian_words() {
    // Entry 1: bytes 0x04, 0x7f -> value 0x047f -> r=4, g=7, b=15.
    let mut ram = vec![0u8; 2048];
    ram[2] = 0x04;
    ram[3] = 0x7f;
    let palette = Palette::from_ram(&ram);
    let argb = palette.argb(1);
    assert_eq!(argb & 0xff00_0000, 0xff00_0000, "opaque alpha");
    assert_eq!(argb, 0xff4477ff, "4-bit channels expand via (v<<4)|v");
}

#[test]
fn palette_entry_zero_is_the_backdrop() {
    // Entry 0: bytes 0x01, 0x11 -> r=1, g=1, b=1.
    let mut ram = vec![0u8; 2048];
    ram[0] = 0x01;
    ram[1] = 0x11;
    let palette = Palette::from_ram(&ram);
    assert_eq!(palette.argb(0), 0xff111111);
}
