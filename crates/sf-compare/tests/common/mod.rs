//! Shared synthetic-fixture helpers for sf-compare integration tests.
//!
//! Fixtures are tiny, hand-built capture directories (declared screen
//! geometry 4x2, a handful of frames) so tests never depend on the
//! local-only MAME captures or ROM files.
//!
//! Each test binary compiles this module and uses a subset of it.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

pub const FIXTURE_WIDTH: usize = 4;
pub const FIXTURE_HEIGHT: usize = 2;

/// A fresh, uniquely named directory under the system temp directory,
/// removed with its contents when the returned guard is dropped (also when
/// the test panics), so repeated test runs leave nothing behind.
pub fn temp_dir(test_name: &str) -> TempDir {
    let dir = std::env::temp_dir().join(format!(
        "sf-compare-test-{}-{}-{}",
        test_name,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    TempDir(dir)
}

/// See [`temp_dir`].
pub struct TempDir(PathBuf);

impl std::ops::Deref for TempDir {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl AsRef<Path> for TempDir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Build a ROM manifest JSON string from (name, region, offset, content)
/// entries; hashes are computed from the content so tests state their
/// expectations as literal file bytes.
pub fn manifest_json(rom_set: &str, entries: &[(&str, &str, u32, &[u8])]) -> String {
    let mut out = "{\"rom_set\": \"REPLACEME\", \"entries\": [".replace("REPLACEME", rom_set);
    for (i, (name, region, offset, content)) in entries.iter().enumerate() {
        let crc = crc32fast::hash(content);
        let sha = sha1_hex(content);
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"name\": \"{name}\", \"region\": \"{region}\", \"offset\": {offset}, \"size\": {}, \"crc32\": \"{crc:08x}\", \"sha1\": \"{sha}\"}}",
            content.len()
        ));
    }
    out.push_str("]}");
    out
}

pub fn sha1_hex(data: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut h = Sha1::new();
    h.update(data);
    hex(&h.finalize())
}

pub fn crc32_hex(data: &[u8]) -> String {
    format!("{:08x}", crc32fast::hash(data))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn write_file(dir: &Path, name: &str, content: &[u8]) {
    fs::write(dir.join(name), content).unwrap();
}

/// One row of a synthetic frames.tsv.
#[derive(Clone)]
pub struct FrameRow {
    pub screen_hash: String,
    pub latch: String,
    pub latch_changed: u8,
    pub ram_crc: String,
    pub music_crc: String,
}

impl FrameRow {
    pub fn stable(frame: usize) -> Self {
        FrameRow {
            screen_hash: format!("{:08x}", 0x1000 + frame),
            latch: "00".into(),
            latch_changed: 0,
            ram_crc: if frame.is_multiple_of(2) {
                format!("{:08x}", 0x2000 + frame)
            } else {
                "-".into()
            },
            music_crc: if frame.is_multiple_of(2) {
                format!("{:08x}", 0x3000 + frame)
            } else {
                "-".into()
            },
        }
    }
}

/// A synthetic capture: frames plus checkpoint pixel dumps, with the
/// identity files the runner needs (meta.tsv, summary.json).
pub struct CaptureBuilder {
    pub scenario_id: String,
    pub mame_version: String,
    pub frames: Vec<FrameRow>,
    /// (frame, pixel buffer) checkpoints; the crc in checkpoints.tsv is
    /// computed from the buffer.
    pub checkpoints: Vec<(usize, Vec<u8>)>,
    pub ram_dumps: Vec<(usize, String, Vec<u8>)>,
}

impl CaptureBuilder {
    pub fn new(frame_count: usize) -> Self {
        CaptureBuilder {
            scenario_id: "sf-fixed-fight-01".into(),
            mame_version: "0.289 (mame0289)".into(),
            frames: (0..frame_count).map(FrameRow::stable).collect(),
            checkpoints: Vec::new(),
            ram_dumps: Vec::new(),
        }
    }

    pub fn solid_pixels(&mut self, frame: usize, bgra: [u8; 4]) -> &mut Self {
        let mut buf = Vec::with_capacity(FIXTURE_WIDTH * FIXTURE_HEIGHT * 4);
        for _ in 0..FIXTURE_WIDTH * FIXTURE_HEIGHT {
            buf.extend_from_slice(&bgra);
        }
        self.checkpoints.push((frame, buf));
        self
    }

    pub fn write(&self, dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        let mut frames_tsv =
            String::from("frame\tscreen_hash\tlatch\tlatch_changed\tram_crc\tmusic_crc\n");
        for (i, row) in self.frames.iter().enumerate() {
            frames_tsv.push_str(&format!(
                "{i}\t{}\t{}\t{}\t{}\t{}\n",
                row.screen_hash, row.latch, row.latch_changed, row.ram_crc, row.music_crc
            ));
        }
        write_file(dir, "frames.tsv", frames_tsv.as_bytes());

        let mut checkpoints_tsv = String::from("frame\tscreen_crc32\tpixels_file\n");
        for (frame, buf) in &self.checkpoints {
            let name = format!("pixels_{frame:06}.rgba");
            checkpoints_tsv.push_str(&format!("{frame}\t{}\t{name}\n", crc32_hex(buf)));
            write_file(dir, &name, buf);
        }
        write_file(dir, "checkpoints.tsv", checkpoints_tsv.as_bytes());

        write_file(
            dir,
            "meta.tsv",
            format!(
                "dsw1\tdfff\ndsw2\tffff\nscreen_width\t{FIXTURE_WIDTH}\nscreen_height\t{FIXTURE_HEIGHT}\nframes_executed\t{}\n",
                self.frames.len()
            )
            .as_bytes(),
        );

        for (frame, region, buf) in &self.ram_dumps {
            write_file(dir, &format!("ram_{frame:06}_{region}.bin"), buf);
        }

        write_file(
            dir,
            "summary.json",
            format!(
                "{{\"scenario_id\": \"{}\", \"mame_version_reported\": \"{}\"}}",
                self.scenario_id, self.mame_version
            )
            .as_bytes(),
        );
    }
}

/// Minimal scenario JSON matching the synthetic captures above.
pub fn scenario_json(frame_count: usize, checkpoints: &[usize]) -> String {
    format!(
        r#"{{
  "scenario_id": "sf-fixed-fight-01",
  "format_version": 1,
  "reference": {{ "rom_set": "sf", "mame_version": "0.289" }},
  "machine": {{ "set": "sf", "mame_options": [], "dipswitches": "factory defaults" }},
  "controls": {{ "p1_right": {{ "port": "IN1", "bit": 0 }} }},
  "duration_frames": {frame_count},
  "inputs": [],
  "observations": {{
    "screen_downsample_stride": 4,
    "ram_crc_every_frames": 2,
    "checkpoints": {checkpoints:?}
  }}
}}"#
    )
}
