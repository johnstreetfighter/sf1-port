//! Observation capture loading.
//!
//! A capture is the on-disk observation set one adapter run produced:
//! per-frame hashes/latch/RAM CRCs (`frames.tsv`), checkpoint pixel dumps
//! (`checkpoints.tsv` + `pixels_*.rgba`), raw RAM dumps (`ram_*.bin`),
//! machine readback (`meta.tsv`), and runner identity (`summary.json`).
//! Pixel buffers and RAM dumps are addressed lazily by path so loading
//! stays cheap; identity files are parsed eagerly.

use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct CaptureError(pub String);

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid capture: {}", self.0)
    }
}

impl std::error::Error for CaptureError {}

#[derive(Debug)]
pub struct FrameObservation {
    pub frame: u64,
    pub screen_hash: String,
    pub latch: String,
    pub latch_changed: bool,
    pub ram_crc: Option<String>,
    pub music_crc: Option<String>,
}

#[derive(Debug)]
pub struct Checkpoint {
    pub frame: u64,
    pub screen_crc32: String,
    pub pixels_path: PathBuf,
}

#[derive(Debug)]
pub struct RamDump {
    pub frame: u64,
    pub region: String,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct CaptureMeta {
    pub dsw1: Option<u16>,
    pub dsw2: Option<u16>,
    pub screen_width: u32,
    pub screen_height: u32,
    pub frames_executed: u64,
}

#[derive(Debug)]
pub struct Capture {
    pub dir: PathBuf,
    pub frames: Vec<FrameObservation>,
    pub checkpoints: Vec<Checkpoint>,
    pub ram_dumps: Vec<RamDump>,
    pub meta: CaptureMeta,
    /// Identity recorded by the producing runner (tooling, not an
    /// observation): used to establish baseline identity, never compared
    /// byte-wise.
    pub scenario_id: Option<String>,
    pub mame_version_reported: Option<String>,
}

pub fn load_capture(dir: impl AsRef<Path>) -> Result<Capture, CaptureError> {
    let dir = dir.as_ref();
    if !dir.is_dir() {
        return Err(CaptureError(format!(
            "capture directory {} does not exist",
            dir.display()
        )));
    }
    let frames = load_frames(&dir.join("frames.tsv"))?;
    let checkpoints = load_checkpoints(dir)?;
    let meta = load_meta(&dir.join("meta.tsv"))?;
    let (scenario_id, mame_version_reported) = load_summary(&dir.join("summary.json"))?;
    let ram_dumps = load_ram_dumps(dir)?;
    Ok(Capture {
        dir: dir.to_path_buf(),
        frames,
        checkpoints,
        ram_dumps,
        meta,
        scenario_id,
        mame_version_reported,
    })
}

fn read_tsv(path: &Path) -> Result<Vec<Vec<String>>, CaptureError> {
    let text = std::fs::read_to_string(path).map_err(|_| {
        CaptureError(format!(
            "observation file {} is missing or unreadable",
            path.file_name().unwrap_or_default().to_string_lossy()
        ))
    })?;
    let mut lines = text.lines();
    let Some(_header) = lines.next() else {
        return Err(CaptureError(format!(
            "{} is empty (expected a header row)",
            path.display()
        )));
    };
    Ok(lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect())
}

fn load_frames(path: &Path) -> Result<Vec<FrameObservation>, CaptureError> {
    let rows = read_tsv(path)?;
    let mut frames = Vec::with_capacity(rows.len());
    for row in rows {
        let expect = |i: usize, col: &str| -> Result<String, CaptureError> {
            row.get(i)
                .cloned()
                .ok_or_else(|| CaptureError(format!("frames.tsv row {:?} lacks column {col}", row)))
        };
        let frame: u64 = expect(0, "frame")?
            .parse()
            .map_err(|_| CaptureError(format!("frames.tsv row {row:?} has a non-numeric frame")))?;
        let crc = |s: String| (s != "-").then_some(s);
        frames.push(FrameObservation {
            frame,
            screen_hash: expect(1, "screen_hash")?,
            latch: expect(2, "latch")?,
            latch_changed: expect(3, "latch_changed")? == "1",
            ram_crc: crc(expect(4, "ram_crc")?),
            music_crc: crc(expect(5, "music_crc")?),
        });
    }
    Ok(frames)
}

fn load_checkpoints(dir: &Path) -> Result<Vec<Checkpoint>, CaptureError> {
    let rows = read_tsv(&dir.join("checkpoints.tsv"))?;
    let mut checkpoints = Vec::with_capacity(rows.len());
    for row in rows {
        if row.len() < 3 {
            return Err(CaptureError(format!(
                "checkpoints.tsv row {row:?} lacks columns"
            )));
        }
        let frame: u64 = row[0].parse().map_err(|_| {
            CaptureError(format!(
                "checkpoints.tsv row {row:?} has a non-numeric frame"
            ))
        })?;
        checkpoints.push(Checkpoint {
            frame,
            screen_crc32: row[1].clone(),
            pixels_path: dir.join(&row[2]),
        });
    }
    Ok(checkpoints)
}

fn load_meta(path: &Path) -> Result<CaptureMeta, CaptureError> {
    let text = std::fs::read_to_string(path).map_err(|_| {
        CaptureError(format!(
            "observation file {} is missing or unreadable",
            path.file_name().unwrap_or_default().to_string_lossy()
        ))
    })?;
    let get = |key: &str| -> Result<String, CaptureError> {
        text.lines()
            .find_map(|l| {
                l.split_once('\t')
                    .filter(|(k, _)| *k == key)
                    .map(|(_, v)| v.to_string())
            })
            .ok_or_else(|| CaptureError(format!("meta.tsv lacks key {key}")))
    };
    let num = |key: &str| -> Result<u64, CaptureError> {
        get(key)?
            .parse()
            .map_err(|_| CaptureError(format!("meta.tsv key {key} is not numeric")))
    };
    let opt_hex16 = |key: &str| -> Result<Option<u16>, CaptureError> {
        let Some(val) = text.lines().find_map(|l| {
            l.split_once('\t')
                .filter(|(k, _)| *k == key)
                .map(|(_, v)| v.trim())
        }) else {
            return Ok(None);
        };
        u16::from_str_radix(val, 16)
            .map(Some)
            .map_err(|_| CaptureError(format!("meta.tsv key {key} is not 16-bit hex")))
    };
    Ok(CaptureMeta {
        dsw1: opt_hex16("dsw1")?,
        dsw2: opt_hex16("dsw2")?,
        screen_width: num("screen_width")? as u32,
        screen_height: num("screen_height")? as u32,
        frames_executed: num("frames_executed")?,
    })
}

fn load_summary(path: &Path) -> Result<(Option<String>, Option<String>), CaptureError> {
    let Ok(text) = std::fs::read_to_string(path) else {
        // Identity is desirable but the observations are authoritative.
        return Ok((None, None));
    };
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| CaptureError(format!("summary.json is not valid JSON: {e}")))?;
    let s = |key: &str| json.get(key).and_then(|v| v.as_str()).map(str::to_string);
    Ok((s("scenario_id"), s("mame_version_reported")))
}

fn load_ram_dumps(dir: &Path) -> Result<Vec<RamDump>, CaptureError> {
    let mut dumps = Vec::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|e| CaptureError(format!("cannot list {}: {e}", dir.display())))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(rest) = name
            .strip_prefix("ram_")
            .and_then(|n| n.strip_suffix(".bin"))
        else {
            continue;
        };
        let Some((frame, region)) = rest.split_once('_') else {
            continue;
        };
        let Ok(frame) = frame.parse::<u64>() else {
            continue;
        };
        dumps.push(RamDump {
            frame,
            region: region.to_string(),
            path: entry.path(),
        });
    }
    dumps.sort_by_key(|d| (d.frame, d.region.clone()));
    Ok(dumps)
}
