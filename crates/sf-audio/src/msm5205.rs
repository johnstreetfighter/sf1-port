//! OKI MSM5205 ADPCM decoder model (ticket #10).
//!
//! Provenance: the decoding follows the MSM5205's documented 4-bit ADPCM
//! scheme as implemented by the pinned comparison baseline's device model
//! (MAME 0.289 `src/devices/sound/msm5205.cpp`, BSD-3-Clause, copyright
//! Aaron Giles; read at tag mame0289): 49 step sizes `floor(16 * 1.1^n)`,
//! the difference `sign * (step*b2 + step/2*b1 + step/4*b0 + step/8)`, the
//! step-index shifts {-1,-1,-1,-1,2,4,6,8}, a 12-bit signal clamped to
//! -2048..=2047, reset forcing signal and step to 0, and a 10-bit DAC (the
//! two low bits dropped) scaled by 1/4096.
//!
//! Integration constraints (sf.cpp, `msm_w`): the chips run in VCK slave
//! mode (`SEX_4B`); every port write sets reset (bit 7) and data (bits 0-3)
//! and pulses VCK, so each write decodes exactly one nibble (15.6 us after
//! the write in the device model — below this model's tick resolution).
//! The output holds its level between decodes; mixing gain 1.0 per chip.

/// One MSM5205.
#[derive(Debug, Clone)]
pub struct Msm5205 {
    signal: i32,
    step: i32,
    diff: [i32; 49 * 16],
}

const INDEX_SHIFT: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

impl Default for Msm5205 {
    fn default() -> Self {
        Self::new()
    }
}

impl Msm5205 {
    pub fn new() -> Self {
        let mut diff = [0i32; 49 * 16];
        for step in 0..49 {
            let stepval = (16.0f64 * (11.0f64 / 10.0).powi(step as i32)).floor() as i32;
            for nib in 0..16 {
                let sign = if nib & 8 != 0 { -1 } else { 1 };
                let b = |bit: i32| (nib >> bit) & 1;
                diff[step * 16 + nib as usize] =
                    sign * (stepval * b(2) + stepval / 2 * b(1) + stepval / 4 * b(0) + stepval / 8);
            }
        }
        Msm5205 {
            signal: 0,
            step: 0,
            diff,
        }
    }

    /// A port write from the samples CPU: bit 7 = reset, bits 0-3 = data;
    /// one decode.
    pub fn write(&mut self, data: u8) {
        if data & 0x80 != 0 {
            self.signal = 0;
            self.step = 0;
            return;
        }
        let val = (data & 0x0f) as usize;
        self.signal = (self.signal + self.diff[self.step as usize * 16 + val]).clamp(-2048, 2047);
        self.step = (self.step + INDEX_SHIFT[val & 7]).clamp(0, 48);
    }

    /// The 12-bit decoder signal.
    pub fn signal(&self) -> i32 {
        self.signal
    }

    /// The DAC output (10 bits, scaled to -0.5..0.5).
    pub fn output(&self) -> f32 {
        (self.signal & !3) as f32 / 4096.0
    }

    /// Encode 12-bit PCM as the nibbles this chip will decode. Each sample is
    /// clamped to -2048..=2047, then the nibble whose step lands closest is chosen.
    pub fn encode_nibbles(pcm: &[i16]) -> Vec<u8> {
        let mut chip = Self::new();
        let mut out = Vec::with_capacity(pcm.len());
        for sample in pcm {
            let target = i32::from(*sample).clamp(-2048, 2047);
            let mut best = 0u8;
            let mut best_err = i32::MAX;
            for nib in 0..16 {
                let next =
                    (chip.signal + chip.diff[chip.step as usize * 16 + nib]).clamp(-2048, 2047);
                let err = (next - target).abs();
                if err < best_err {
                    best_err = err;
                    best = nib as u8;
                }
            }
            chip.write(best);
            out.push(best);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_table_matches_the_documented_sizes() {
        let m = Msm5205::new();
        // diff for nibble 7 at step 0 = 16 + 8 + 4 + 2 = 30; nibble 0 = 2.
        assert_eq!(m.diff[7], 30);
        assert_eq!(m.diff[0], 2);
        assert_eq!(m.diff[8], -2);
        // the largest step value, floor(16 * 1.1^48) = 1552.
        assert_eq!(m.diff[48 * 16 + 4], 1552 + 1552 / 8);
    }

    #[test]
    fn decode_clamps_and_reset_clears() {
        let mut m = Msm5205::new();
        for _ in 0..40 {
            m.write(0x47);
        }
        assert_eq!(m.signal(), 2047, "clamped at +2047");
        m.write(0xc0);
        assert_eq!(m.signal(), 0);
        m.write(0x40);
        assert_eq!(m.signal(), 2, "a 0 nibble after reset adds step/8");
    }

    #[test]
    fn encoded_nibbles_decode_toward_the_pcm() {
        let pcm = [1000i16; 80];
        let nibbles = Msm5205::encode_nibbles(&pcm);
        assert!(nibbles.iter().all(|n| *n <= 15));
        let mut chip = Msm5205::new();
        for nibble in nibbles {
            chip.write(nibble);
        }
        assert!(
            chip.signal() > 900,
            "a held tone must climb the MSM5205 step table, got {}",
            chip.signal()
        );
    }
}
