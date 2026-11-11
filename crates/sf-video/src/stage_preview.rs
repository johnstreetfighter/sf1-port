//! Small previews of the stages for the stage screen (spec #192, ticket
//! #197 `ST3`): each cut from the stage's own backdrop. The stage record
//! gives both layers' views (page, scroll, palette); the backdrop is drawn
//! as a fight starts on it (the foreground at its word plus the camera's
//! `0xc0`, the background at its word, no sprites or text) and halved, so a
//! stage costs no art of its own and a record from the art pack previews
//! like a 1987 stage.

use std::collections::BTreeMap;

use crate::background_bank::{BackgroundBank, LayerView, StageView, PALETTE_BYTES};
use crate::render::{render_frame, HEIGHT, WIDTH};
use crate::romset::Assets;
use crate::scene::SceneState;

/// A preview's size: the fight's frame halved.
pub const PREVIEW_WIDTH: usize = WIDTH / 2;
pub const PREVIEW_HEIGHT: usize = HEIGHT / 2;
/// The camera's start offset `stage_reset` adds to the foreground's word.
const CAMERA_START: u16 = 0xc0;
/// The background and foreground layers on, nothing else.
const LAYERS_ONLY: u8 = 0x60;

/// One stage's backdrop at half size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagePreview {
    pub width: usize,
    pub height: usize,
    pub argb: Vec<u32>,
}

/// The previews of the listed stages, by the stage's place in the bank.
pub type StagePreviews = BTreeMap<u8, StagePreview>;

impl BackgroundBank {
    /// The preview of the stage at `index`, drawn from `assets` (which must
    /// hold this bank's tiles and pages: `Assets::with_background_bank`).
    /// `None` when the bank has no such stage or a palette it names is
    /// missing.
    pub fn stage_preview(&self, assets: &Assets, index: u8) -> Option<StagePreview> {
        let view = self.stage(index)?;
        let paletteram = self.backdrop_palettes(assets, &view)?;
        let scene = SceneState {
            videoram: vec![0; 0x1000],
            objectram: vec![0; 0x2000],
            paletteram,
            bg_scroll: view.bg.scroll,
            fg_scroll: view.fg.scroll.wrapping_add(CAMERA_START),
            active: LAYERS_ONLY,
            flip_screen: false,
            bg_page: view.bg.page,
            fg_page: view.fg.page,
        };
        let frame = render_frame(assets, &scene);
        Some(halved(&frame.argb))
    }

    /// The previews of every listed stage.
    pub fn listed_previews(&self, assets: &Assets) -> StagePreviews {
        self.listed_stages()
            .iter()
            .filter_map(|l| self.stage_preview(assets, l.index).map(|p| (l.index, p)))
            .collect()
    }

    /// Palette RAM holding the stage's background palette then its
    /// foreground's, from the ROM or from this bank.
    fn backdrop_palettes(&self, assets: &Assets, view: &StageView) -> Option<Vec<u8>> {
        let layer = |layer: &LayerView| -> Option<&[u8]> {
            self.palette(layer.palette).or_else(|| {
                let at = layer.palette as usize;
                assets.maincpu.get(at..at + PALETTE_BYTES)
            })
        };
        let mut ram = vec![0u8; 2048];
        ram[..PALETTE_BYTES].copy_from_slice(layer(&view.bg)?);
        ram[PALETTE_BYTES..2 * PALETTE_BYTES].copy_from_slice(layer(&view.fg)?);
        Some(ram)
    }
}

/// `argb` (a full frame) at half size, each pixel the mean of its 2x2 block.
fn halved(argb: &[u32]) -> StagePreview {
    let mean = |x: usize, y: usize, shift: u32| -> u32 {
        let sum: u32 = [(0, 0), (1, 0), (0, 1), (1, 1)]
            .iter()
            .map(|(dx, dy)| (argb[(2 * y + dy) * WIDTH + 2 * x + dx] >> shift) & 0xff)
            .sum();
        sum / 4
    };
    let mut out = Vec::with_capacity(PREVIEW_WIDTH * PREVIEW_HEIGHT);
    for y in 0..PREVIEW_HEIGHT {
        for x in 0..PREVIEW_WIDTH {
            out.push(0xff00_0000 | (mean(x, y, 16) << 16) | (mean(x, y, 8) << 8) | mean(x, y, 0));
        }
    }
    StagePreview { width: PREVIEW_WIDTH, height: PREVIEW_HEIGHT, argb: out }
}
