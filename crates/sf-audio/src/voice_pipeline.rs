//! The voice pipeline's Rust side (spec #216, `EH9`, #225): the codec steps
//! of `tools/voice/pipeline.py`, so Python never reimplements the MSM5205.
//! The `sf-audio voice-*` subcommands call these
//! (docs/reference/voice-pipeline.md).
//!
//! - [`encode_take`]: a take at 4 kHz, mono, 16-bit PCM, scaled to the
//!   chip's 12-bit signal and encoded with the asset pack's encoder
//!   ([`encode_adpcm`], which validates it as 4 kHz 4-bit ADPCM).
//! - [`decode`]: nibbles through the board's chip model ([`Msm5205`]), reset
//!   first as the announcer's chip is at a line's start, one output a
//!   nibble: what the player hears.
//! - [`rom_sample`]: one of the ROM's own samples (the samples CPU's command
//!   `0x81..=0xbf`, e.g. `0x94`, "FIGHT!"), played through the translated
//!   samples CPU from power-on, as that chip's nibbles.
//! - [`pack_nibbles`] / [`unpack_nibbles`]: the `.vox` byte order, two
//!   nibbles a byte, high first (the samples ROM's order, and
//!   [`VoiceStore::write`]'s).
//! - [`build_pack`]: the chosen takes' `.vox` files written as the voice
//!   pack with [`VoiceStore::write`], then opened again with
//!   [`VoiceStore::open`] to prove the game will load it.

use std::path::Path;

use sf_video::announcer::AnnouncerLine;
use sf_video::asset_pack::{encode_adpcm, AdpcmClip, ADPCM_RATE};

use crate::announcer::VoiceStore;
use crate::msm5205::Msm5205;
use crate::samples::PortWrite;
use crate::{LatchTimeline, SamplesSound, TICK_HZ};

/// A take for the encoder: mono 16-bit PCM at [`ADPCM_RATE`], or why not.
pub fn read_take_wav(path: &Path) -> Result<Vec<i16>, String> {
    let mut r = hound::WavReader::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = r.spec();
    if spec.channels != 1
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
        || spec.sample_rate != ADPCM_RATE
    {
        return Err(format!(
            "{}: a take must be mono 16-bit PCM at {ADPCM_RATE} Hz (it is {} channel(s), {}-bit, {} Hz)",
            path.display(),
            spec.channels,
            spec.bits_per_sample,
            spec.sample_rate
        ));
    }
    r.samples::<i16>()
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Encode a 4 kHz 16-bit take: each sample is taken down to the chip's
/// 12-bit signal (an arithmetic shift by 4), then [`encode_adpcm`].
pub fn encode_take(pcm16: &[i16]) -> Result<AdpcmClip, String> {
    let pcm12: Vec<i16> = pcm16.iter().map(|s| s >> 4).collect();
    encode_adpcm(&pcm12).map_err(|e| e.message)
}

/// The chip's DAC output for `nibbles` from a reset chip, one value a
/// nibble (-0.5..0.5, [`Msm5205::output`]).
pub fn decode(nibbles: &[u8]) -> Vec<f32> {
    let mut chip = Msm5205::new();
    nibbles
        .iter()
        .map(|n| {
            chip.write(n & 0x0f);
            chip.output()
        })
        .collect()
}

/// `.vox` bytes: two nibbles a byte, high first; an odd count's last low
/// nibble is 0.
pub fn pack_nibbles(nibbles: &[u8]) -> Vec<u8> {
    nibbles
        .chunks(2)
        .map(|p| (p[0] << 4) | p.get(1).copied().unwrap_or(0))
        .collect()
}

/// The first `count` nibbles of `.vox` bytes.
pub fn unpack_nibbles(bytes: &[u8], count: usize) -> Result<Vec<u8>, String> {
    if bytes.len() != count.div_ceil(2) {
        return Err(format!(
            "{} bytes cannot hold {count} nibbles (needs {})",
            bytes.len(),
            count.div_ceil(2)
        ));
    }
    Ok(bytes.iter().flat_map(|b| [b >> 4, b & 0x0f]).take(count).collect())
}

/// The ROM's own sample for samples-CPU command `command` (`0x81..=0xbf`):
/// the translated samples CPU from power-on, the command latched for one
/// frame, and the nibbles it writes to the chip that plays it. The samples
/// CPU holds the chip in reset (`0xc0`), releases it (`0x40`, a 0 nibble),
/// writes the sample's `2 × bytes` nibbles with bit 6 set, and resets the
/// chip at its natural end; the nibbles returned are the sample's own,
/// without the release. The latch idles at `0xff`, as the 68000 leaves it.
pub fn rom_sample(audio2: Vec<u8>, command: u8) -> Result<Vec<u8>, String> {
    if !(0x81..=0xbf).contains(&command) {
        return Err(format!("0x{command:02x} is not a sample command (0x81..=0xbf)"));
    }
    let at = 0.1;
    let mut latch = LatchTimeline::new(vec![(0.0, 0xff), (at, command), (at + 1.0 / 60.0, 0xff)]);
    let mut snd = SamplesSound::new(audio2);
    let mut writes: Vec<PortWrite> = Vec::new();
    // The longest ROM sample is 7.04 s (docs/reference/adpcm-samples.md).
    let last = (TICK_HZ as f64 * (at + 10.0)) as u64;
    while snd.ticks() < last {
        snd.tick(&mut latch, &mut |w| {
            if w.port < 2 {
                writes.push(w)
            }
        });
    }
    let release = writes
        .iter()
        .position(|w| w.t >= at && w.data == 0x40)
        .ok_or_else(|| format!("command 0x{command:02x} played nothing"))?;
    let port = writes[release].port;
    let mut nibbles = Vec::new();
    for w in writes[release + 1..].iter().filter(|w| w.port == port) {
        if w.data & 0x80 != 0 {
            return Ok(nibbles);
        }
        nibbles.push(w.data & 0x0f);
    }
    Err(format!("command 0x{command:02x} did not end within 10 s"))
}

/// One chosen take for the pack: the line's file stem, its `.vox` and its
/// nibble count.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PackLine {
    pub vox: std::path::PathBuf,
    pub nibbles: usize,
}

/// A build-pack plan: `{"lines": {"super": {"vox": "...", "nibbles": 2412}, ...}}`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackPlan {
    pub lines: std::collections::BTreeMap<String, PackLine>,
}

/// Write the voice pack in `folder` from `plan` with [`VoiceStore::write`],
/// then open it with [`VoiceStore::open`] as the game does. Returns the
/// opened store.
pub fn build_pack(plan: &PackPlan, folder: &Path) -> Result<VoiceStore, String> {
    let mut clips = Vec::new();
    for (stem, entry) in &plan.lines {
        let line = AnnouncerLine::from_file_stem(stem).ok_or_else(|| format!("{stem}: not an announcer line"))?;
        let bytes = std::fs::read(&entry.vox).map_err(|e| format!("{}: {e}", entry.vox.display()))?;
        let nibbles = unpack_nibbles(&bytes, entry.nibbles).map_err(|e| format!("{}: {e}", entry.vox.display()))?;
        clips.push((line, AdpcmClip { sample_rate: ADPCM_RATE, nibbles }));
    }
    let store = VoiceStore::from_clips(clips.iter().cloned())?;
    store.write(folder)?;
    let opened = VoiceStore::open(folder)?;
    for (line, clip) in &clips {
        if opened.line(*line) != Some(clip.nibbles.as_slice()) {
            return Err(format!("{}: the pack read back differs from the take", line.file_stem()));
        }
    }
    Ok(opened)
}
