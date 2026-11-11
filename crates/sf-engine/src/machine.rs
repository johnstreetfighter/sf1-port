//! Native machine state shared by the bounded scene translations.
//!
//! Original addresses (sfus_map @ mame0289):
//! - `videoram`:   0x800000-0x800fff (text tilemap, 64x32 big-endian u16).
//! - `paletteram`: 0xb00000-0xb007ff (1024 xRGB_444 entries).
//! - `objectram`:  0xffe000-0xffffff (sprite list, 128 entries of 0x20 words;
//!   the renderer reads 4 words per entry: code, attr, sy, sx).
//! - `fg_scroll` / `bg_scroll`: write-only registers 0xc00014 / 0xc00018.
//! - `gfxctrl`:    write-only register 0xc0001a (low byte: bit2 flip,
//!   bit3 tx, bit5 bg, bit6 fg, bit7 sprites).
//! - `sound_latch`: audio space 0xc800 (command byte; 0xff = idle).
//!
//! The struct carries only machine-visible state plus the frame counter;
//! the scene machines (`versus`, `cpu`) drive it through the translated
//! routines.

use sf_video::scene::SceneState;

/// The machine-visible state for a bounded interval. State at frame f is the
/// end-of-frame-f state (post-update), matching the reference driver's
/// frame_done dump timing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineState {
    /// 0x800000-0x800fff.
    pub videoram: Vec<u8>,
    /// 0xffe000-0xffffff.
    pub objectram: Vec<u8>,
    /// 0xb00000-0xb007ff.
    pub paletteram: Vec<u8>,
    /// Write-only register 0xc00018.
    pub bg_scroll: u16,
    /// Write-only register 0xc00014.
    pub fg_scroll: u16,
    /// Low byte of write-only register 0xc0001a.
    pub gfxctrl: u8,
    /// Write-only register 0xc00010 (coin counters bits 0-1, coin lockouts
    /// active-low bits 4-6; mirrored in work RAM at 0xff8684).
    pub coin_ctrl: u16,
    /// gfxctrl bit 2.
    pub flip_screen: bool,
    /// Audio space 0xc800.
    pub sound_latch: u8,
    /// 68000 cycles from the vblank handler's first instruction to this
    /// frame's sound-latch write (`tasks::vblank`; ticket #35), 0 before the
    /// first native frame.
    pub latch_cycles: u32,
    /// Frame f such that this state is the end-of-frame-f state.
    pub frame: u32,
    /// The map page the bg and fg layers read (ADR-0009): renderer state,
    /// not on the board. 0, the ROM tilemap, unless an overhaul gate has
    /// put a layer on a page of the extended background bank.
    pub bg_page: u8,
    pub fg_page: u8,
}

crate::canonical::canonical_struct!(MachineState, "MachineState.v1", {
    videoram, objectram, paletteram, bg_scroll, fg_scroll, gfxctrl, coin_ctrl,
    flip_screen, sound_latch, latch_cycles, frame, bg_page, fg_page,
});

impl MachineState {
    /// Construct from explicit parts (scene fixtures and tests build states
    /// with this).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        videoram: Vec<u8>,
        objectram: Vec<u8>,
        paletteram: Vec<u8>,
        bg_scroll: u16,
        fg_scroll: u16,
        gfxctrl: u8,
        flip_screen: bool,
        sound_latch: u8,
        frame: u32,
    ) -> Self {
        MachineState {
            videoram,
            objectram,
            paletteram,
            bg_scroll,
            fg_scroll,
            gfxctrl,
            coin_ctrl: 0x7070,
            flip_screen,
            sound_latch,
            latch_cycles: 0,
            frame,
            bg_page: 0,
            fg_page: 0,
        }
    }

    /// The renderer-facing view of the current state.
    pub fn scene(&self) -> SceneState {
        SceneState {
            videoram: self.videoram.clone(),
            objectram: self.objectram.clone(),
            paletteram: self.paletteram.clone(),
            bg_scroll: self.bg_scroll,
            fg_scroll: self.fg_scroll,
            active: self.gfxctrl,
            flip_screen: self.flip_screen,
            bg_page: self.bg_page,
            fg_page: self.fg_page,
        }
    }
}

/// The sound-latch byte (a write-only register, not in work RAM) at the end
/// of `frame`, from a reference capture's per-frame record (frames.tsv);
/// None if the capture lacks that row.
pub fn fixture_latch(capture_dir: &std::path::Path, frame: u32) -> Option<u8> {
    let text = std::fs::read_to_string(capture_dir.join("frames.tsv")).ok()?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next()?.split('\t').collect();
    let col = header.iter().position(|h| *h == "latch")?;
    lines.find_map(|line| {
        let cells: Vec<&str> = line.split('\t').collect();
        (cells.first()?.parse::<u32>().ok()? == frame)
            .then(|| u8::from_str_radix(cells.get(col)?, 16).ok())?
    })
}

/// The DIP switch readback `(dsw1, dsw2)` recorded in a reference capture's
/// `meta.tsv`, falling back to `(0xdfff, 0xffff)` if absent.
pub fn fixture_dsw(capture_dir: &std::path::Path) -> (u16, u16) {
    let idle = crate::tasks::Io::idle();
    let Ok(text) = std::fs::read_to_string(capture_dir.join("meta.tsv")) else {
        return (idle.dsw1, idle.dsw2);
    };
    let get = |key: &str| -> Option<u16> {
        let val = text.lines().find_map(|l| {
            l.split_once('\t')
                .filter(|(k, _)| *k == key)
                .map(|(_, v)| v.trim())
        })?;
        u16::from_str_radix(val, 16).ok()
    };
    (
        get("dsw1").unwrap_or(idle.dsw1),
        get("dsw2").unwrap_or(idle.dsw2),
    )
}
