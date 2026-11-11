//! Scene state for fixture reconstruction: the machine-visible state the
//! renderer consumes, with original addresses documented per field.

/// The state needed to reconstruct one frame.
///
/// Original addresses (sfus_map @ mame0289):
/// - `videoram`: 0x800000-0x800fff (text tilemap words, big-endian u16).
/// - `objectram`: 0xffe000-0xffffff (sprite list, big-endian u16 words).
/// - `paletteram`: 0xb00000-0xb007ff (1024 xRGB_444 entries).
/// - `fg_scroll`: write-only register 0xc00014; `bg_scroll`: 0xc00018;
///   `active`: low byte of the gfxctrl register 0xc0001a
///   (bit2 flip, bit3 tx, bit5 bg, bit6 fg, bit7 sprites). These three are
///   recovered from reference-run register taps, not from RAM dumps.
/// - `bg_page` / `fg_page`: the map page each scroll layer reads (ADR-0009).
///   Not 1987 machine state: 0, the ROM tilemap, unless an overhaul gate
///   has put a layer on a page of the extended background bank.
#[derive(Clone)]
pub struct SceneState {
    pub videoram: Vec<u8>,
    pub objectram: Vec<u8>,
    pub paletteram: Vec<u8>,
    pub bg_scroll: u16,
    pub fg_scroll: u16,
    pub active: u8,
    pub flip_screen: bool,
    pub bg_page: u8,
    pub fg_page: u8,
}
