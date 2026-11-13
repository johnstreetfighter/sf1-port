//! Live sound for the development launcher (tickets #10, #37): both
//! translated sound programs and their chips (`sf_audio::mix::GameSound`:
//! the samples CPU with its two MSM5205s, the music CPU with the YM2151,
//! mixed as the pinned MAME mixes them) driven by the native engine's
//! per-frame sound latch, played in stereo through the default output
//! device.
//!
//! The sound starts from power-on and is pre-rolled to the fixture frame on
//! the reference's latch writes through that frame (a reference audio run
//! whose history up to the fixture is the fixture capture's; the fixture
//! frame's own vblank ran in the reference, not natively), so the round
//! music and a voice still playing at the fixture are heard as in the
//! reference; from the first stepped frame on, the engine's latch drives it
//! (the interrupt
//! latency the observed band's middle, ticket #35). The both-chips tests
//! verify this start and the frame-by-frame streaming (at the band's low
//! end); at the middle used here the samples CPU differs from the fixed
//! fight's reference from frame 2619 on (docs/reference/both-chips.md).
//! Without the reference run the pre-roll has no writes and the sound
//! starts silent.
//!
//! The game loop pushes each frame's latch; the audio callback plays only
//! as far as the pushed frames' writes reach, so sound never runs ahead of
//! the game (if the game stalls, the output holds).
//!
//! The announcer's own chip (ADR 0011) is mixed beside the board's two.
//! [`Audio::push_game_frame`] routes classic calls and the game's own
//! announcements for the selected voice.

use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use sf_audio::mix::GameSound;
use sf_audio::score::NativeScore;
use sf_audio::{LatchTimeline, FRAME_HZ, IRQ_LATENCY_MID};
use sf_engine::front::{settings::LiveSound, FrameOutput};

use crate::jukebox::{soft_limit, TrackMixer, TRACK_GAIN};
use crate::track::Track;
use crate::voice::{AnnouncerFrame, VoicePacks, VoicePlayback};

/// Where the launcher's sound starts: the sound ROMs, the fixture frame,
/// and the reference's latch writes through it.
#[derive(Clone)]
pub struct SoundStart {
    pub audio2: Vec<u8>,
    pub audiocpu: Vec<u8>,
    pub frame: u32,
    pub preroll: LatchTimeline,
}

impl SoundStart {
    /// Both programs from power-on, pre-rolled to the fixture frame's start
    /// (its writes, through its end, are the reference's).
    pub fn build(&self) -> GameSound {
        let known = (self.frame + 1) as f64 / FRAME_HZ as f64;
        let mut sound = GameSound::power_on(
            self.audio2.clone(),
            self.audiocpu.clone(),
            self.preroll.clone(),
            known,
        );
        sound.skip_to(self.frame as f64 / FRAME_HZ as f64);
        sound
    }
}

/// Captures the launcher's live output (mono average `(l + r) / 2` at
/// [`sf_audio::check::WAV_RATE`]) over frames `first..=last` while a scripted
/// launcher run pushes frames one at a time at [`IRQ_LATENCY_MID`].
pub struct LiveCapture {
    sound: GameSound,
    t0: f64,
    t_advance: f64,
    target_len: usize,
    out: Vec<f32>,
}

impl LiveCapture {
    pub fn new(start: &SoundStart, first: u32, last: u32) -> Self {
        let sound = start.build();
        let t_advance = start.frame as f64 / FRAME_HZ as f64;
        let t0 = first as f64 / FRAME_HZ as f64;
        let t_end = (last + 1) as f64 / FRAME_HZ as f64;
        let rate = sf_audio::check::WAV_RATE as f64;
        let target_len = ((t_end - t0) * rate).round() as usize;
        LiveCapture {
            sound,
            t0,
            t_advance,
            target_len,
            out: Vec::with_capacity(target_len),
        }
    }

    /// Set the extended tune table before pushing the frame that can use it.
    pub fn set_tune_table(&self, table: Option<Arc<Vec<u8>>>) {
        self.sound.set_tune_table(table);
    }

    /// Push one stepped frame's latch at [`IRQ_LATENCY_MID`] and drain the
    /// live output that becomes ready.
    pub fn push_frame(&mut self, frame: u32, latch: u8, latch_cycles: u32) {
        self.sound
            .push_frame(frame, latch, latch_cycles, IRQ_LATENCY_MID);
        let frame_ready = (frame + 1) as f64 / FRAME_HZ as f64 - 0.001;
        self.sound.wait_ready(frame_ready);
        if self.t_advance < self.t0 {
            let step_to = frame_ready.min(self.t0);
            if step_to > self.t_advance {
                self.sound.skip_to(step_to);
                self.t_advance = step_to;
            }
            if self.t_advance < self.t0 {
                return;
            }
        }
        let rate = sf_audio::check::WAV_RATE as f64;
        let t_now = self.t0 + self.out.len() as f64 / rate;
        let ready = self.sound.ready_until(t_now);
        while self.out.len() < self.target_len {
            let t = self.t0 + self.out.len() as f64 / rate;
            if t > ready {
                break;
            }
            let [l, r] = self.sound.sample_at(t);
            self.out.push((l + r) / 2.0);
        }
    }

    pub fn unrecovered(&self) -> Vec<u16> {
        self.sound.unrecovered()
    }

    pub fn into_samples(self) -> Vec<f32> {
        self.out
    }
}

struct Shared {
    sound: GameSound,
    /// Voice packs and the current announcer routing for the game.
    voice: Option<VoicePlayback>,
    /// The extended tune table the music CPU runs with now (an overhaul
    /// fight's), if any.
    tunes: Option<Arc<Vec<u8>>>,
    /// The `MASTER`, `ANNOUNCER`, `FM` and `SAMPLES` volumes as gains
    /// (0.0..=1.0).
    gains: Gains,
    /// The Jukebox's track, when one plays in place of the native music.
    track: TrackMixer,
    /// The output's machine time (seconds).
    t: f64,
    /// Where the output started, and its largest level since (for the
    /// smoke check).
    started: f64,
    peak: f32,
}

/// The launcher's audio output; dropping it stops the stream.
pub struct Audio {
    shared: Arc<Mutex<Shared>>,
    _stream: cpal::Stream,
}

impl Audio {
    /// Open the default output device, the sound pre-rolled to the
    /// fixture. Returns None (with a message) when no device is usable:
    /// the launcher then runs silent.
    pub fn open(start: &SoundStart) -> Option<Audio> {
        let t = start.frame as f64 / FRAME_HZ as f64;
        let shared = Arc::new(Mutex::new(Shared {
            sound: start.build(),
            voice: None,
            tunes: None,
            gains: Gains::FULL,
            track: TrackMixer::default(),
            t,
            started: t,
            peak: 0.0,
        }));
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let rate = config.sample_rate().0 as f64;
        let channels = config.channels() as usize;
        let cb_shared = shared.clone();
        let err = |e| eprintln!("audio stream error: {e}");
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| fill(&cb_shared, data, channels, rate),
                err,
                None,
            ),
            other => {
                eprintln!("audio: unsupported output sample format {other:?}; running silent");
                return None;
            }
        };
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("audio: cannot open the output stream ({e}); running silent");
                return None;
            }
        };
        stream.play().ok()?;
        Some(Audio {
            shared,
            _stream: stream,
        })
    }

    /// Push the sound latch of the frame just produced (`frame`, the
    /// machine's frame counter), with the vblank handler's cycles to its
    /// write (`MachineState::latch_cycles`).
    pub fn push_frame(&self, frame: u32, latch: u8, latch_cycles: u32) {
        self.shared
            .lock()
            .unwrap()
            .sound
            .push_frame(frame, latch, latch_cycles, IRQ_LATENCY_MID);
    }

    /// Install both voice packs. They last across [`Audio::reset`].
    pub fn set_voice_packs(&self, packs: VoicePacks) {
        let mut s = self.shared.lock().unwrap();
        s.voice = Some(VoicePlayback::new(packs, &mut s.sound));
    }

    /// Push the front end's frame, with the latch already processed by
    /// the Jukebox and the live sound settings for the frame's audition.
    pub fn push_game_frame(
        &self,
        out: &FrameOutput,
        latch: u8,
        score: Option<Arc<NativeScore>>,
        live: LiveSound,
    ) {
        let mut s = self.shared.lock().unwrap();
        let Shared { sound, voice, gains, .. } = &mut *s;
        *gains = Gains::from_volumes(live.master, live.fm, live.samples, live.announcer);
        let frame = out.sound_frame;
        let latch_cycles = out.latch_cycles;
        if let Some(voice) = voice {
            voice.push_frame_with_score(sound, AnnouncerFrame {
                frame,
                latch,
                latch_cycles,
                overhaul_fight: out.overhaul_fight,
                voice: live.announcer_voice,
                lines: &out.announcements,
            }, score);
        } else {
            sound.push_frame_with_score(frame, latch, latch_cycles, IRQ_LATENCY_MID, score);
        }
    }

    /// Give the music CPU the extended tune table `table` (`None`: the ROM
    /// alone), from the next frame pushed. The launcher follows
    /// `FrameOutput::tune_table`; the CPU keeps the table until the tune
    /// playing from it is over. It lasts across
    /// [`Audio::reset`].
    pub fn set_tune_table(&self, table: Option<Arc<Vec<u8>>>) {
        let mut s = self.shared.lock().unwrap();
        s.sound.set_tune_table(table.clone());
        s.tunes = table;
    }

    /// Start the Jukebox's `track` from its beginning at `frame` (the frame
    /// whose latch was just pushed). It is mixed over the board's sound at
    /// the music's level (`MASTER` and `FM`, and the chip route's gain) until [`Audio::stop_track`].
    pub fn start_track(&self, frame: u32, track: Arc<Track>) {
        self.shared.lock().unwrap().track.start(f64::from(frame) / f64::from(FRAME_HZ), track);
    }

    /// Stop the Jukebox's track at `frame`.
    pub fn stop_track(&self, frame: u32) {
        self.shared.lock().unwrap().track.stop(f64::from(frame) / f64::from(FRAME_HZ));
    }

    /// Restart from power-on and the pre-roll (the launcher reloaded its
    /// fixture); the new sound is built before the old one is replaced.
    pub fn reset(&self, start: &SoundStart) {
        let mut sound = start.build();
        let old = {
            let mut s = self.shared.lock().unwrap();
            if let Some(voice) = &mut s.voice {
                voice.reset(&mut sound);
            }
            sound.set_tune_table(s.tunes.clone());
            s.t = start.frame as f64 / FRAME_HZ as f64;
            s.started = s.t;
            s.peak = 0.0;
            s.track.clear();
            std::mem::replace(&mut s.sound, sound)
        };
        // Stopping the old sound's music thread waits for it; not under the
        // lock the audio callback takes.
        drop(old);
    }

    /// How much machine time the output has played since the (re)start,
    /// and its largest level.
    pub fn played(&self) -> (f64, f32) {
        let s = self.shared.lock().unwrap();
        (s.t - s.started, s.peak)
    }

    /// Untranslated music-CPU branches reached so far.
    pub fn unrecovered(&self) -> Vec<u16> {
        self.shared.lock().unwrap().sound.unrecovered()
    }

    /// Set the output's volumes.
    pub fn set_gains(&self, gains: Gains) {
        self.shared.lock().unwrap().gains = gains;
    }
}

/// The front end's volumes as gains on the mixer: `MASTER` scales every
/// route, `FM` the YM2151's, `SAMPLES` the board's two MSM5205s',
/// `ANNOUNCER` the announcer's own chip (0 is off).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gains {
    pub master: f32,
    pub fm: f32,
    pub samples: f32,
    pub announcer: f32,
}

impl Gains {
    /// Every volume at 10.
    pub const FULL: Gains = Gains {
        master: 1.0,
        fm: 1.0,
        samples: 1.0,
        announcer: 1.0,
    };

    /// From the 0..=10 volume rows.
    pub fn from_volumes(master: u8, fm: u8, samples: u8, announcer: u8) -> Self {
        let g = |v: u8| v.min(10) as f32 / 10.0;
        Gains {
            master: g(master),
            fm: g(fm),
            samples: g(samples),
            announcer: g(announcer),
        }
    }

    /// One mixed sample at machine time `t` with these gains.
    pub fn sample(&self, sound: &mut GameSound, t: f64) -> [f32; 2] {
        if *self == Gains::FULL {
            return sound.sample_at(t);
        }
        sound.sample_at_mix(
            t,
            self.master * self.fm,
            self.master * self.samples,
            self.master * self.announcer,
        )
    }
}

fn fill(shared: &Arc<Mutex<Shared>>, data: &mut [f32], channels: usize, rate: f64) {
    let mut s = shared.lock().unwrap();
    let ready = s.sound.ready_until(s.t);
    for frame in data.chunks_mut(channels) {
        let next = s.t + 1.0 / rate;
        let advance = next <= ready;
        if advance {
            s.t = next;
        } // else starved: hold until the game catches up
        let t = s.t;
        let gains = s.gains;
        let [l, r] = gains.sample(&mut s.sound, t);
        let (l, r) = if s.track.is_playing() {
            // The track at the music's level (MASTER and FM, and the chip
            // route's gain), and the sum kept inside the output's range.
            let [tl, tr] = s.track.sample(t, advance, rate);
            let music = gains.master * gains.fm * TRACK_GAIN;
            (soft_limit(l + tl * music), soft_limit(r + tr * music))
        } else {
            s.track.sample(t, advance, rate);
            (l, r)
        };
        s.peak = s.peak.max(l.abs()).max(r.abs());
        match frame.len() {
            1 => frame[0] = (l + r) / 2.0,
            _ => {
                frame[0] = l;
                frame[1] = r;
                for out in frame[2..].iter_mut() {
                    *out = (l + r) / 2.0;
                }
            }
        }
    }
}

/// A looping player for a pre-rendered mono buffer (ticket #11's phrase
/// demonstration); dropping it stops the stream.
pub struct Player {
    _stream: cpal::Stream,
}

impl Player {
    /// Play `pcm` (at `rate` Hz) in a loop on the default output device,
    /// with linear interpolation to the device's rate.
    pub fn looping(pcm: Vec<f32>, rate: u32) -> Option<Player> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        if config.sample_format() != cpal::SampleFormat::F32 {
            eprintln!("audio: unsupported output sample format; not playing");
            return None;
        }
        let out_rate = config.sample_rate().0 as f64;
        let channels = config.channels() as usize;
        let step = rate as f64 / out_rate;
        let mut pos = 0.0f64;
        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    for frame in data.chunks_mut(channels) {
                        let k = pos.floor() as usize;
                        let f = (pos - pos.floor()) as f32;
                        let a = pcm[k % pcm.len()];
                        let b = pcm[(k + 1) % pcm.len()];
                        let v = a + (b - a) * f;
                        for out in frame.iter_mut() {
                            *out = v;
                        }
                        pos += step;
                        if pos >= pcm.len() as f64 {
                            pos -= pcm.len() as f64;
                        }
                    }
                },
                |e| eprintln!("audio stream error: {e}"),
                None,
            )
            .ok()?;
        stream.play().ok()?;
        Some(Player { _stream: stream })
    }
}
