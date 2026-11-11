//! Frame pacing for the GPU window (ticket #205, VO2; spec #203, design
//! section 6). The game runs at 60 frames a second, as the reference does;
//! the window presents on vsync. When the display's refresh is within 1%
//! of a whole multiple *n* of 60, one game frame advances every *n*
//! presents, so at 120 Hz every frame is shown for exactly two refreshes.
//! Otherwise (144 Hz, 165 Hz, vsync off, a window that cannot present) the
//! game advances on a 60 Hz clock, which starts afresh whenever pacing is
//! chosen again, so a stale clock never runs the game fast.
//!
//! Everything here is pure: times come in as durations since any fixed
//! start, so the tests drive it with simulated refreshes and no GPU.

use std::time::Duration;

/// The game's frame rate (the MAME reference timeline is 60 Hz).
pub const GAME_HZ: u32 = 60;

/// The most game frames the clock steps before one present. A longer gap
/// (the window dragged, the machine asleep) slows the game instead of
/// fast-forwarding through it.
pub const MAX_CATCH_UP: u32 = 4;

/// The whole multiple *n* of 60 that `refresh_hz` is within 1% of, if any
/// (59.94 Hz is 1, 120 Hz is 2, 240 Hz is 4; 144 Hz and 165 Hz are none).
pub fn refresh_multiple(refresh_hz: f64) -> Option<u32> {
    if !refresh_hz.is_finite() || refresh_hz <= 0.0 {
        return None;
    }
    let game = f64::from(GAME_HZ);
    let n = (refresh_hz / game).round();
    (n >= 1.0 && (refresh_hz - n * game).abs() <= 0.01 * n * game).then_some(n as u32)
}

/// How game frames are timed against presents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pacing {
    /// One game frame every *n* presents on vsync.
    EveryNth(u32),
    /// Game frames on a 60 Hz clock, presented whenever the window can.
    Clock,
}

impl Pacing {
    /// The rule: every *n*th present when vsync is on and the refresh
    /// (`None` when the monitor does not say) is a multiple of 60, else
    /// the clock.
    pub fn choose(vsync: bool, refresh_hz: Option<f64>) -> Pacing {
        match refresh_hz.and_then(refresh_multiple) {
            Some(n) if vsync => Pacing::EveryNth(n),
            _ => Pacing::Clock,
        }
    }
}

/// Decides, before each present, how many game frames to step.
///
/// Whenever the pacing is chosen again (vsync switched, the refresh rate
/// changed) a new pacer replaces the old, so its clock starts from its
/// first present and never from the time the window opened.
#[derive(Clone, Debug)]
pub struct Pacer {
    pacing: Pacing,
    /// Presents since the last game frame (`EveryNth`).
    since: u32,
    /// The clock's last reading and its unspent time, in nanoseconds
    /// times 60, so a game frame is exactly 10^9 units (`Clock`).
    last: Option<Duration>,
    owed: u128,
}

const FRAME_UNITS: u128 = 1_000_000_000;

impl Pacer {
    /// A pacer starting afresh: its first present steps one game frame.
    pub fn new(pacing: Pacing) -> Self {
        Pacer {
            pacing,
            since: 0,
            last: None,
            owed: 0,
        }
    }

    pub fn pacing(&self) -> Pacing {
        self.pacing
    }

    /// The pacer for a display at `refresh_hz` with vsync on or off, fresh.
    pub fn for_display(vsync: bool, refresh_hz: Option<f64>) -> Self {
        Pacer::new(Pacing::choose(vsync, refresh_hz))
    }

    /// The game frames to step before the present at `now`.
    pub fn steps(&mut self, now: Duration) -> u32 {
        match self.pacing {
            Pacing::EveryNth(n) => {
                let step = self.since == 0;
                self.since = (self.since + 1) % n.max(1);
                u32::from(step)
            }
            Pacing::Clock => {
                let Some(last) = self.last.replace(now) else {
                    return 1;
                };
                self.owed += now.saturating_sub(last).as_nanos() * u128::from(GAME_HZ);
                let due = self.owed / FRAME_UNITS;
                self.owed %= FRAME_UNITS;
                due.min(u128::from(MAX_CATCH_UP)) as u32
            }
        }
    }
}

/// What the presents have done since the window opened: how many, how
/// many game frames were stepped, and how many refreshes arrived late
/// (went by with no present of their own). The late count is measured
/// between successive presents, so the first present only starts the
/// measure.
///
/// The times are taken when a frame is acquired, which wobbles: with frames
/// queued, a gap of one and a half refreshes is often followed by one of
/// half a refresh, and no refresh was missed. So the gaps are added up in
/// refreshes against the presents (ticket #214, VO11), and a refresh counts
/// late once the sum is a whole refresh behind. Presents coming early (a
/// queue filling up, vsync off) make up for at most [`EARLY_CREDIT`]
/// refreshes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PresentStats {
    pub presents: u64,
    pub frames: u64,
    pub late: u64,
    /// The refreshes the presents are behind (or, below zero, ahead) and
    /// not yet counted late.
    behind: f64,
    /// How many presents had their work timed, the time they took in all
    /// and the longest: stepping the game, uploading and drawing the frame
    /// (not the frame delay's wait). The headroom `FRAME DELAY` spends.
    pub worked: u64,
    pub work_total: Duration,
    pub work_max: Duration,
    last: Option<Duration>,
}

impl PresentStats {
    /// Count a present at `now` that showed `frames` new game frames, on a
    /// display refreshing every `period` (`None` when unknown; nothing is
    /// then counted late).
    pub fn present(&mut self, now: Duration, frames: u32, period: Option<Duration>) {
        self.presents += 1;
        self.frames += u64::from(frames);
        if let (Some(last), Some(period)) = (self.last, period) {
            if !period.is_zero() {
                let gap = now.saturating_sub(last).as_secs_f64() / period.as_secs_f64();
                self.behind = (self.behind + gap - 1.0).max(-EARLY_CREDIT);
                let late = self.behind.floor().max(0.0);
                self.late += late as u64;
                self.behind -= late;
            }
        }
        self.last = Some(now);
    }
}

/// The most refreshes early presents make up for (the deepest swapchain
/// queue the window asks for).
pub const EARLY_CREDIT: f64 = 3.0;

impl PresentStats {
    /// Count a present's work (a step, an upload and a draw) that took `took`.
    pub fn work(&mut self, took: Duration) {
        self.worked += 1;
        self.work_total += took;
        self.work_max = self.work_max.max(took);
    }

    /// The mean work per present since `earlier`, in milliseconds.
    pub fn mean_work_ms_since(&self, earlier: &PresentStats) -> f64 {
        let n = self.worked.saturating_sub(earlier.worked);
        if n == 0 {
            return 0.0;
        }
        (self.work_total.saturating_sub(earlier.work_total)).as_secs_f64() * 1000.0 / n as f64
    }
}

/// The refresh period of a display at `hz`.
pub fn refresh_period(hz: f64) -> Option<Duration> {
    (hz.is_finite() && hz > 0.0).then(|| Duration::from_secs_f64(1.0 / hz))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Present `seconds` of refreshes at `hz` (with `jitter` added to every
    /// other present's timestamp, as a real clock read wobbles), and return
    /// how many presents each game frame stayed on screen, the frames
    /// stepped and the most frames stepped before one present.
    fn simulate(pacing: Pacing, hz: f64, seconds: f64, jitter: Duration) -> (Vec<u32>, u64, u32) {
        let mut pacer = Pacer::new(pacing);
        let period = 1.0 / hz;
        let presents = (seconds * hz).round() as u64;
        let (mut shown, mut frames, mut most) = (Vec::new(), 0, 0);
        for k in 0..presents {
            let mut now = Duration::from_secs_f64(k as f64 * period);
            if k % 2 == 1 {
                now += jitter;
            }
            let steps = pacer.steps(now);
            most = most.max(steps);
            frames += u64::from(steps);
            if steps > 0 {
                shown.push(0);
            }
            *shown.last_mut().expect("the first present steps a frame") += 1;
        }
        // The last frame may still be on screen; only finished ones count.
        shown.pop();
        (shown, frames, most)
    }

    #[test]
    fn the_rule_picks_n_for_multiples_of_60_and_the_clock_otherwise() {
        assert_eq!(Pacing::choose(true, Some(60.0)), Pacing::EveryNth(1));
        assert_eq!(Pacing::choose(true, Some(59.94)), Pacing::EveryNth(1));
        assert_eq!(Pacing::choose(true, Some(120.0)), Pacing::EveryNth(2));
        assert_eq!(Pacing::choose(true, Some(119.88)), Pacing::EveryNth(2));
        assert_eq!(Pacing::choose(true, Some(240.0)), Pacing::EveryNth(4));
        assert_eq!(Pacing::choose(true, Some(144.0)), Pacing::Clock);
        assert_eq!(Pacing::choose(true, Some(165.0)), Pacing::Clock);
        assert_eq!(Pacing::choose(true, Some(75.0)), Pacing::Clock);
        assert_eq!(Pacing::choose(true, Some(30.0)), Pacing::Clock);
        assert_eq!(Pacing::choose(true, None), Pacing::Clock);
        assert_eq!(Pacing::choose(false, Some(120.0)), Pacing::Clock);
        // The 1% edge, either side.
        assert_eq!(refresh_multiple(121.1), Some(2));
        assert_eq!(refresh_multiple(121.3), None);
        assert_eq!(refresh_multiple(59.45), Some(1));
        assert_eq!(refresh_multiple(59.35), None);
    }

    #[test]
    fn on_a_multiple_of_60_every_frame_is_shown_for_exactly_n_refreshes() {
        for (hz, n) in [(60.0, 1), (120.0, 2), (240.0, 4), (59.94, 1)] {
            let pacing = Pacing::choose(true, Some(hz));
            assert_eq!(pacing, Pacing::EveryNth(n));
            // A millisecond of timestamp wobble changes nothing: vsync
            // pacing counts presents, not time.
            let (shown, frames, most) = simulate(pacing, hz, 10.0, Duration::from_millis(1));
            assert!(shown.iter().all(|&s| s == n), "{hz} Hz: {shown:?}");
            assert_eq!(most, 1, "{hz} Hz");
            assert_eq!(frames, (10.0 * hz).round() as u64 / u64::from(n), "{hz} Hz");
        }
    }

    #[test]
    fn at_144_and_165_hz_the_clock_holds_60_fps_with_no_frame_skipped_or_doubled() {
        for (hz, cadence) in [(144.0, [2, 3]), (165.0, [2, 3])] {
            let pacing = Pacing::choose(true, Some(hz));
            assert_eq!(pacing, Pacing::Clock);
            let (shown, frames, most) = simulate(pacing, hz, 10.0, Duration::from_micros(500));
            // 60 game frames a second, to within the frame the run ends in.
            assert!((599..=601).contains(&frames), "{hz} Hz: {frames} frames in 10 s");
            // Never two frames before one present (one skipped), and each
            // frame on screen for two or three refreshes, never fewer or more.
            assert_eq!(most, 1, "{hz} Hz");
            assert!(shown.iter().all(|s| cadence.contains(s)), "{hz} Hz: {shown:?}");
        }
    }

    #[test]
    fn the_clock_starts_afresh_so_switching_pacing_never_runs_the_game_fast() {
        // Ten seconds on vsync, then the clock chosen anew: its first
        // present steps one frame, not the ten seconds since the window
        // opened.
        let mut pacer = Pacer::new(Pacing::EveryNth(2));
        for k in 0..1200 {
            pacer.steps(Duration::from_secs_f64(k as f64 / 120.0));
        }
        let mut pacer = Pacer::new(Pacing::Clock);
        assert_eq!(pacer.steps(Duration::from_secs(10)), 1);
        // Presenting as fast as possible (vsync off, 1000 presents a
        // second) the clock still steps 60 frames a second.
        let frames: u32 = (1..=1000).map(|k| pacer.steps(Duration::from_secs(10) + Duration::from_millis(k))).sum();
        assert_eq!(frames, 60);
    }

    #[test]
    fn switching_vsync_off_starts_a_fresh_clock_so_the_game_holds_60_fps() {
        // Ten seconds on vsync at 120 Hz, then vsync off with presents at
        // 1000 a second: the fresh pacer steps 60 frames in the next second,
        // not the 600 the stale window clock would owe. Switching back
        // restarts the every-nth count at a step.
        let on = Pacer::for_display(true, Some(120.0));
        assert_eq!(on.pacing(), Pacing::EveryNth(2));
        let switched = Duration::from_secs(10);
        let mut off = Pacer::for_display(false, Some(120.0));
        assert_eq!(off.pacing(), Pacing::Clock);
        assert_eq!(off.steps(switched), 1, "the first present steps one frame");
        let frames: u32 = (1..=1000).map(|k| off.steps(switched + Duration::from_millis(k))).sum();
        assert_eq!(frames, 60, "then 60 frames in the next second of 1000 presents");
        let mut again = Pacer::for_display(true, Some(120.0));
        assert_eq!(again.steps(Duration::from_secs(11)), 1);
    }

    #[test]
    fn work_is_counted_per_present() {
        let mut stats = PresentStats::default();
        let before = stats;
        for ms in [2, 4, 3] {
            stats.work(Duration::from_millis(ms));
        }
        assert_eq!((stats.worked, stats.work_max), (3, Duration::from_millis(4)));
        assert!((stats.mean_work_ms_since(&before) - 3.0).abs() < 1e-9);
        assert_eq!(stats.mean_work_ms_since(&stats), 0.0);
    }

    #[test]
    fn a_long_stall_slows_the_game_instead_of_fast_forwarding() {
        let mut pacer = Pacer::new(Pacing::Clock);
        pacer.steps(Duration::ZERO);
        assert_eq!(pacer.steps(Duration::from_secs(2)), MAX_CATCH_UP);
        assert_eq!(pacer.steps(Duration::from_secs(2) + Duration::from_millis(17)), 1);
    }

    #[test]
    fn late_refreshes_are_the_refreshes_a_present_missed() {
        let period = refresh_period(120.0).unwrap();
        let at = |k: f64| Duration::from_secs_f64(k * period.as_secs_f64());
        let mut stats = PresentStats::default();
        for (k, frames) in [(0.0, 1), (1.0, 0), (2.0, 1), (4.0, 0), (5.0, 1), (10.0, 0)] {
            stats.present(at(k), frames, Some(period));
        }
        assert_eq!((stats.presents, stats.frames, stats.late), (6, 3, 5), "one missed at 3, four from 6 to 9");

        // Acquire times that wobble by half a refresh either way, a present
        // for every refresh: none late.
        let mut stats = PresentStats::default();
        for k in 0..1200 {
            let wobble = if k % 2 == 1 { 0.5 } else { 0.0 };
            stats.present(at(k as f64 + wobble), 1, Some(period));
        }
        assert_eq!(stats.late, 0);

        // Presents come early as a queue fills, but make up for at most
        // three refreshes missed later.
        let mut stats = PresentStats::default();
        for k in [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 6.5] {
            stats.present(at(k), 1, Some(period));
        }
        assert_eq!(stats.late, 2);

        // Unknown refresh: nothing counted late.
        let mut stats = PresentStats::default();
        stats.present(at(0.0), 1, None);
        stats.present(at(9.0), 1, None);
        assert_eq!(stats.late, 0);
    }
}
