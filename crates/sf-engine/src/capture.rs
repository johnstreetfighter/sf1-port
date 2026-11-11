//! Observation-capture writer: the native side of the comparison boundary.
//!
//! Produces the same on-disk format as the MAME reference driver
//! (tools/reference/driver_template.lua): frames.tsv, checkpoints.tsv,
//! pixels_*.rgba (B,G,R,A bytes), ram_*.bin dumps, meta.tsv, summary.json.
//!
//! Hash convention (verified byte-exact against the reference run): the
//! driver's `screen:pixel(x, y)` reads the 512x256 screen bitmap at raw
//! coordinates, so the stride-4 FNV-1a over x 0..384 / y 0..224 samples the
//! visible crop at (x-64, y-16) and reads opaque black (0xff000000) in the
//! border. Checksum columns the native engine does not model (work-RAM and
//! audio-RAM CRCs) are emitted as "-" and excluded from comparison via the
//! comparison scope — the slice's explicit uncovered-behavior statement.

use std::io;
use std::path::{Path, PathBuf};

use sf_video::render::{Frame, HEIGHT, VISAREA_X, VISAREA_Y, WIDTH};

use crate::machine::MachineState;

/// The driver's stride-4 FNV-1a screen hash (0x811c9dc5 / 0x01000193).
pub fn screen_hash(frame: &Frame) -> u32 {
    let mut acc: u32 = 0x811c9dc5;
    for y in (0..HEIGHT).step_by(4) {
        for x in (0..WIDTH).step_by(4) {
            let px = if x >= VISAREA_X && y >= VISAREA_Y {
                frame.argb[(y - VISAREA_Y) * WIDTH + (x - VISAREA_X)]
            } else {
                0xff000000
            };
            acc ^= px;
            acc = acc.wrapping_mul(0x01000193);
        }
    }
    acc
}

/// Frame pixels as the driver's snapshot bytes (B,G,R,A per pixel).
pub fn rgba_bytes(frame: &Frame) -> Vec<u8> {
    let mut out = Vec::with_capacity(frame.argb.len() * 4);
    for px in &frame.argb {
        out.push((px & 0xff) as u8); // B
        out.push((px >> 8) as u8); // G
        out.push((px >> 16) as u8); // R
        out.push((px >> 24) as u8); // A
    }
    out
}

pub struct CaptureWriter {
    dir: PathBuf,
    frames_tsv: String,
    checkpoints_tsv: String,
    prev_latch: Option<u8>,
}

impl CaptureWriter {
    pub fn create(dir: impl AsRef<Path>) -> io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        Ok(CaptureWriter {
            dir,
            frames_tsv:
                "frame\tscreen_hash\tlatch\tlatch_changed\tram_crc\tmusic_crc\tlatch_cycles\n"
                    .into(),
            checkpoints_tsv: "frame\tscreen_crc32\tpixels_file\n".into(),
            // Reference rows before the interval read the latch as idle
            // (row 2279 = 0xff), so the first in-interval change is detected.
            prev_latch: Some(0xff),
        })
    }

    /// Emit one frames.tsv row. `latch_cycles` is the vblank handler's
    /// 68000 cycles to the frame's latch write (`MachineState::latch_cycles`;
    /// 0 for the fixture row), the row's last column so that readers by
    /// index are unaffected.
    pub fn push_frame(&mut self, frame: u32, screen: &Frame, latch: u8, latch_cycles: u32) {
        let changed = match self.prev_latch {
            Some(prev) if frame > 0 => prev != latch,
            _ => false,
        };
        self.prev_latch = Some(latch);
        self.frames_tsv.push_str(&format!(
            "{frame}\t{:08x}\t{latch:02x}\t{}\t-\t-\t{latch_cycles}\n",
            screen_hash(screen),
            u8::from(changed),
        ));
    }

    /// Emit one checkpoint: full pixels plus the RAM dumps the native state
    /// models (video, palette). Regions the engine does not model (work,
    /// music) are absent, and the comparator skips them.
    pub fn push_checkpoint(
        &mut self,
        frame: u32,
        screen: &Frame,
        state: &MachineState,
    ) -> io::Result<()> {
        let pixels = rgba_bytes(screen);
        let name = format!("pixels_{frame:06}.rgba");
        std::fs::write(self.dir.join(&name), &pixels)?;
        self.checkpoints_tsv.push_str(&format!(
            "{frame}\t{:08x}\t{name}\n",
            crc32fast::hash(&pixels)
        ));
        std::fs::write(
            self.dir.join(format!("ram_{frame:06}_video.bin")),
            &state.videoram,
        )?;
        std::fs::write(
            self.dir.join(format!("ram_{frame:06}_palette.bin")),
            &state.paletteram,
        )?;
        Ok(())
    }

    /// Write the identity/observation files and finish.
    pub fn finish(self, frames_executed: u64, scenario_id: &str) -> io::Result<()> {
        let idle = crate::tasks::Io::idle();
        self.finish_with_dsw(frames_executed, scenario_id, idle.dsw1, idle.dsw2)
    }

    /// Write the identity/observation files, including the `dsw1` and `dsw2`
    /// readback in `meta.tsv`, and finish.
    pub fn finish_with_dsw(
        self,
        frames_executed: u64,
        scenario_id: &str,
        dsw1: u16,
        dsw2: u16,
    ) -> io::Result<()> {
        std::fs::write(self.dir.join("frames.tsv"), self.frames_tsv)?;
        std::fs::write(self.dir.join("checkpoints.tsv"), self.checkpoints_tsv)?;
        std::fs::write(
            self.dir.join("meta.tsv"),
            format!(
                "dsw1\t{dsw1:04x}\ndsw2\t{dsw2:04x}\nscreen_width\t{WIDTH}\nscreen_height\t{HEIGHT}\nframes_executed\t{frames_executed}\n"
            ),
        )?;
        std::fs::write(
            self.dir.join("summary.json"),
            format!(
                "{{\"scenario_id\": \"{scenario_id}\", \"runner\": \"sf-engine (native Rust state translation)\"}}"
            ),
        )?;
        Ok(())
    }
}
