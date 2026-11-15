"""The announcer's takes from the vendor's PCM to the cabinet's ADPCM (EH9, #225).

    prepare(raw, out_4k)    trim the silence, downsample to 4 kHz mono 16-bit
                            (ffmpeg), normalise the peak
    Codec.encode(...)       the repo's own MSM5205 encoder: `sf-audio voice-encode`
    Codec.rom_sample(...)   a ROM sample as its chip plays it: `sf-audio voice-rom-sample`
    Codec.pack(...)         the voice pack from chosen takes: `sf-audio voice-pack`

The codec is Rust (crates/sf-audio/src/voice_pipeline.rs), so Python never
reimplements it: what is judged on the listening sheet is what the game's
chip plays. Build it with `cargo build --release -p sf-audio`.
"""

import array
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import wave

# Everything the processing depends on. Changing a value re-processes every
# take from its archived original, free; bump "version" when the code changes.
PROCESS = {
    "version": 1,
    "trim_below_peak_db": -40.0,  # silence: quieter than this under the take's peak
    "pad_ms": 20,                 # kept either side of the sound
    "rate": 4000,                 # the MSM5205's rate on the board (ADR 0011)
    "resampler": "ffmpeg -ar (swr, its anti-alias low-pass)",
    "peak_db": -2.0,              # the ROM's FIGHT! peaks about 2 dB under the chip's full scale
    "codec": "sf-audio voice-encode (sf_video::asset_pack::encode_adpcm, 16-bit >> 4 to the chip's 12 bits)",
}


def read_wav(path):
    with wave.open(str(path), "rb") as w:
        if w.getsampwidth() != 2 or w.getnchannels() != 1:
            raise ValueError(f"{path}: expected mono 16-bit PCM")
        rate = w.getframerate()
        samples = array.array("h", w.readframes(w.getnframes()))
    return rate, samples


def write_wav(path, rate, samples):
    """Mono 16-bit: `samples` are ints, or the vendor's raw little-endian PCM bytes."""
    path = pathlib.Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    frames = bytes(samples) if isinstance(samples, (bytes, bytearray)) else array.array("h", samples).tobytes()
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(frames)


def trim(samples, rate, below_peak_db, pad_ms):
    """The take from its first to its last sample within `below_peak_db` of
    its peak, with `pad_ms` kept either side."""
    peak = max((abs(s) for s in samples), default=0)
    if peak == 0:
        raise ValueError("the take is silent")
    floor = peak * 10 ** (below_peak_db / 20)
    loud = [i for i, s in enumerate(samples) if abs(s) >= floor]
    pad = int(rate * pad_ms / 1000)
    return samples[max(0, loud[0] - pad):min(len(samples), loud[-1] + 1 + pad)]


def normalise(samples, peak_db):
    peak = max((abs(s) for s in samples), default=0)
    if peak == 0:
        raise ValueError("the take is silent")
    gain = 32767 * 10 ** (peak_db / 20) / peak
    return array.array("h", (max(-32768, min(32767, round(s * gain))) for s in samples))


def ffmpeg():
    exe = shutil.which("ffmpeg")
    if not exe:
        raise SystemExit("ffmpeg is not on the PATH (the voice pipeline needs it to downsample)")
    return exe


def prepare(raw, out_4k, spec=PROCESS):
    """Prepare a take for 4 kHz ADPCM, with optional tempo and reverb."""
    rate, samples = read_wav(raw)
    original_seconds = len(samples) / rate
    out_4k = pathlib.Path(out_4k)
    out_4k.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        scratch = pathlib.Path(tmp)
        tempo = spec.get("tempo", 1)
        if tempo != 1:
            paced = scratch / "paced.wav"
            subprocess.run([ffmpeg(), "-hide_banner", "-loglevel", "error", "-y", "-i", str(raw),
                            "-af", f"atempo={tempo}", "-c:a", "pcm_s16le", str(paced)], check=True)
            rate, samples = read_wav(paced)
        src = scratch / "trimmed.wav"
        dst = scratch / "down.wav"
        trimmed = trim(samples, rate, spec["trim_below_peak_db"], spec["pad_ms"])
        write_wav(src, rate, trimmed)
        subprocess.run([ffmpeg(), "-hide_banner", "-loglevel", "error", "-y", "-i", str(src),
                        "-ac", "1", "-ar", str(spec["rate"]), "-c:a", "pcm_s16le", str(dst)], check=True)
        r, down = read_wav(dst)
        if reverb := spec.get("reverb"):
            delays = "|".join(map(str, reverb["delays_ms"]))
            decays = "|".join(map(str, reverb["decays"]))
            gain_in = reverb.get("input_gain", 1)
            gain_out = reverb.get("output_gain", 1)
            wet = scratch / "reverb.wav"
            write_wav(dst, r, normalise(down, spec["peak_db"]))
            subprocess.run([ffmpeg(), "-hide_banner", "-loglevel", "error", "-y", "-i", str(dst),
                            "-af", f"aecho={gain_in}:{gain_out}:{delays}:{decays}", "-c:a", "pcm_s16le", str(wet)], check=True)
            r, down = read_wav(wet)
    write_wav(out_4k, r, normalise(down, spec["peak_db"]))
    return {"seconds_raw": round(original_seconds, 3), "seconds": round(len(down) / r, 3)}


class Codec:
    """The `sf-audio` binary's voice subcommands."""

    def __init__(self, binary, roms, manifest, cwd):
        self.binary, self.roms, self.manifest, self.cwd = pathlib.Path(binary), roms, manifest, cwd

    def ready(self):
        return self.binary.exists()

    def _run(self, *args):
        if not self.ready():
            raise SystemExit(f"{self.binary} is not built: run `cargo build --release -p sf-audio`")
        r = subprocess.run([str(self.binary), *map(str, args)], capture_output=True, text=True, cwd=self.cwd)
        if r.returncode != 0:
            raise SystemExit(f"sf-audio {args[0]}: {r.stderr.strip()}")
        return json.loads(r.stdout)

    def encode(self, wav_4k, vox, heard):
        return self._run("voice-encode", "--wav", wav_4k, "--vox", vox, "--heard", heard)

    def rom_sample(self, command, out):
        return self._run("voice-rom-sample", "--command", command, "--out", out,
                         "--roms", self.roms, "--manifest", self.manifest)

    def pack(self, plan, folder):
        return self._run("voice-pack", "--plan", plan, "--out", folder)


def binary_path(repo):
    name = "sf-audio.exe" if os.name == "nt" else "sf-audio"
    return pathlib.Path(os.environ.get("SF_AUDIO_BIN", pathlib.Path(repo) / "target" / "release" / name))
