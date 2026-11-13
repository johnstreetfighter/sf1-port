//! The original cooperative task scheduler (ticket #14) and the tasks that
//! run under it during the fight.
//!
//! The 68000 program runs its game as up to 16 cooperative tasks in slots
//! of 0x10 bytes at 0xff8006 (+0 state: 1 new, 2 ready, 4 running, 8
//! sleeping; +1 sleep count; +2 SR; +4 PC; +8 USP). Each vblank (0x3b052)
//! counts the sleeping slots down and makes those reaching zero ready;
//! the main loop (0x3b19a) then runs the ready slots in slot order, each
//! until it sleeps again (trap #2 through 0x3b24e, usually from the sleep
//! helpers 0x3b51a..0x3b5aa). Translated tasks are resumable state machines
//! here, their resume points named by the original addresses; the slot
//! bytes (state, sleep count) and the scheduler's own variables (0xff8000
//! vblank flag, 0xff8001 current slot, 0xff8002 / 0xff8004 frame counters)
//! are kept in work RAM as the original keeps them. The saved register
//! images on the tasks' stacks and the slots' SR / PC / USP words are not
//! reproduced: the translation keeps its continuations in Rust.
//!
//! Tasks of the fight (slots as the fixtures show them):
//!
//! | Slot | Task | Resume point(s) | Translation |
//! | --- | --- | --- | --- |
//! | 0 | boot setup (0x3c0c4), round timer and screen/countdown timers (0x3f4e2) | 0x3c0ce, 0x3b634, 0x3f7ac | translated (tickets #24, #41) |
//! | 1 | game flow: attract (0x3ccec..), start screen (0x3dfce), round start, fight loop (0x3e9bc..) | 0x3ccec, 0x3cd50, 0x3cd86, 0x3cda8, 0x3b588, 0x3cdb8, 0x3dfce, 0x3ea30, 0x3ea3e, 0x3eafa | translated (incl. tickets #43, #52) |
//! | 2 | screen events (0x3f7b0) | 0x3f7d6 | translated |
//! | 3 | attract texts / challenger watch (0x3f374) | 0x3f3a2 | translated for attract (ticket #43) and game watch; challenger join is not |
//! | 7 | random state and coins (0x4813e) | 0x48270, 0x4819e, 0x481e8, 0x48212, 0x4824e, 0x48344 | translated incl. coin arrival, counter pulse, service switch and attract start (tickets #52, #73) |
//!
//! A fixture's task positions come from each sleeping task's stack: the
//! return address past the sleep wrapper (0x3b5a8) is the resume point in
//! the task body. An occupied slot whose position is not translated is
//! refused rather than guessed.

use crate::config::{AwakeningSide, EvilRyuPlaytest, SatsuiAwakening};
use crate::fighter::{Bus, CPU_STRUCT, DRAMATIC_ALLY2, P1_STRUCT, P2_STRUCT};
use crate::data_bank::arcade_text;
use crate::roster::{PlayMode, StagePick, TournamentFighter};
use sf_video::background_bank::{LayerView, StageView};
use sf_video::ending_screens::ending_key;
use sf_video::vs_screens::vs_key;

/// The slot table (A5+6).
const SLOTS: u32 = 0xff8006;
const SLOT_COUNT: u32 = 16;
/// Scheduler variables: vblank flag, current slot, frame counter (byte),
/// demo/attract mode, frame counter (word).
const VBLANK_FLAG: u32 = 0xff8000;
const CURRENT_SLOT: u32 = 0xff8001;
const FRAME_BYTE: u32 = 0xff8002;
const MODE_DEMO: u32 = 0xff8003;
const FRAME_WORD: u32 = 0xff8004;
/// "The fight loop's frame work is done" (set at the loop top, cleared by
/// vblank).
const FRAME_PENDING: u32 = 0xff8632;
/// Palette-upload request serviced by vblank (0x3b362).
const PALETTE_REQUEST: u32 = 0xff8633;
const SCENE_MODE: u32 = 0xffc008;
const GAME_MODE: u32 = 0xffc002;
/// KO slow-motion flag (set by scene event 9).
const SLOW_MOTION: u32 = 0xffc00b;
/// The round result's winner (a player-structure pointer) and the bonus
/// tally's numbers, one decimal digit per byte: the frozen TIME copy (3),
/// the vitality (4) and the bonus (5).
const WINNER: u32 = 0xff8e38;
const TIME_COPY: u32 = 0xff8e3c;
const VITALITY: u32 = 0xff8e40;
const BONUS: u32 = 0xff8e44;
/// TIME digits (three bytes, one decimal digit each).
pub const TIME_DIGITS: u32 = 0xff8e18;
/// Credits (word) and its BCD companion (the long at 0xff865a).
const CREDITS: u32 = 0xff865c;
const CREDITS_LONG: u32 = 0xff865a;
/// Coin-control output copy (written to 0xc00010 with it).
const COIN_CONTROL: u32 = 0xff8684;
/// Aux-control output copy (written to 0xc00012 with it).
const AUX_CONTROL: u32 = 0xff8686;
/// The four coin structures (8 bytes each).
const COIN_SLOTS: [u32; 4] = [0xff863a, 0xff8642, 0xff864a, 0xff8652];
/// Text-writer sound command (cleared at the end of every text block).
const TEXT_SOUND: u32 = 0xff868c;
/// The watch task's continuation words (A5+0xf20 / +0xf24).
const WATCH_NEXT: u32 = 0xff8f20;
const WATCH_STATE: u32 = 0xff8f24;

/// The hardware inputs vblank and the tasks read (active low except the
/// DIP switches' settings).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Io {
    /// 0xc00000: coins, P1 HP / HK, P2 HP / HK.
    pub in0: u16,
    /// 0xc00002: P2 byte : P1 byte.
    pub in1: u16,
    /// 0xc00004: unmapped (`sfus_map` `nopr()` with `unmap_value_high()`,
    /// `sf-mame0289.cpp:167, 173`).
    pub unmap_c00004: u16,
    /// 0xc00006: unmapped (`sfus_map` `nopr()` with `unmap_value_high()`,
    /// `sf-mame0289.cpp:167, 174`).
    pub unmap_c00006: u16,
    /// 0xc0000c: start buttons, service.
    pub system: u16,
    /// 0xc0000e: unmapped (`sfus_map` `nopr()` with `unmap_value_high()`,
    /// `sf-mame0289.cpp:167, 178`).
    pub unmap_c0000e: u16,
    /// 0xc00008.
    pub dsw1: u16,
    /// 0xc0000a.
    pub dsw2: u16,
}

// Canonical gameplay schema: explicit tags and exhaustive fields (NP1).
crate::canonical::canonical_struct!(Io, "Io.v1", {
    in0, in1, unmap_c00004, unmap_c00006, system, unmap_c0000e, dsw1, dsw2,
});

crate::canonical::canonical_struct!(Scheduler, "Scheduler.v1", {
    tasks,
});

crate::canonical::canonical_struct!(TextWriter, "TextWriter.v1", {
    a0, d0, row_start, rows, cols, col, attr, filling, count, delay, finished,
});

crate::canonical::canonical_enum!(ServiceInput, "ServiceInput.v1";
    Entry { d7 } => 0,
    Upload { step, d7 } => 1,
    Loop { d7 } => 2,
);

crate::canonical::canonical_enum!(ServiceMenu, "ServiceMenu.v1";
    Entry => 0,
    BlinkErase { d7 } => 1,
    BlinkRedraw { d7 } => 2,
    InItem { d7 } => 3,
    ReturnToMenu { d7 } => 4,
);

crate::canonical::canonical_enum!(ServiceItem, "ServiceItem.v1";
    CrosshatchEntry => 0,
    CrosshatchLoop => 1,
    ColorBarEntry => 2,
    ColorBarLoop => 3,
    CharTest1Entry => 4,
    CharTest1Loop => 5,
    CharRomEntry => 6,
    CharRomLoop => 7,
    SpriteTestEntry => 8,
    SpriteTestLoop => 9,
    ScrollTestEntry => 10,
    ScrollTestInitBg => 11,
    ScrollTestStart => 12,
    ScrollTestLoop => 13,
    ScrollTestBgOnly => 14,
    OutputTestEntry => 15,
    OutputTestPalette2 => 16,
    OutputTestStart => 17,
    OutputTestApply { d1 } => 18,
    OutputTestReset { d1 } => 19,
);

crate::canonical::canonical_enum!(Boot, "Boot.v1";
    Entry => 0,
    Upload(value0) => 1,
    SettleOverrun => 2,
    Warning => 3,
    WarningTyping { index, writer } => 4,
    WarningPause { index } => 5,
    WarningHold => 6,
);

crate::canonical::canonical_enum!(RankingReturn, "RankingReturn.v1";
    VersusCarry => 0,
    CpuMatchLose => 1,
    GameOverP1 => 2,
    GameOverP2 => 3,
    Attract => 4,
);

crate::canonical::canonical_enum!(BonusCopy, "BonusCopy.v1";
    Japan => 0,
    Stage2 => 1,
);

crate::canonical::canonical_enum!(Bonus10Copy, "Bonus10Copy.v1";
    Stage1 => 0,
    Stage3 => 1,
);

crate::canonical::canonical_enum!(Flow, "Flow.v1";
    RoundStart => 0,
    FightLoop => 1,
    KoStart => 2,
    KoSettle => 3,
    Victory => 4,
    AwaitResult => 5,
    AwaitWipe => 6,
    AwaitAnnounce { new_opponent } => 7,
    AwaitWinScreen { versus } => 8,
    AwaitMatchWipe { versus } => 9,
    VersusCarryStagePalette => 10,
    VersusCarryCountryPalette => 11,
    AwaitCpuWinScreen => 12,
    AwaitCpuMatchWipe => 13,
    ContinuePalette1 => 14,
    ContinuePalette2 => 15,
    ContinueLoop => 16,
    ContinueAccept => 17,
    ContinueCountryPalette => 18,
    ContinueStagePalette => 19,
    TimeOverStart => 20,
    TimeOverHold { left } => 21,
    AwaitDraw => 22,
    DoubleKoSettle => 23,
    DoubleKoHold { left } => 24,
    AwaitGameOverWipe => 25,
    GameOverCycle { index, effect_ticks, entries_left } => 26,
    GameOverDrain => 27,
    GameOverAwaitEvent => 28,
    GameOverPalette => 29,
    GameOverHold => 30,
    StageStart => 31,
    DemoCountryPalette => 32,
    DemoStagePalette => 33,
    AwaitCountryScreen => 34,
    AwaitOpponentIntro => 35,
    Intro => 36,
    FightShown { left } => 37,
    AttractJoin => 38,
    AttractStart => 39,
    AttractScroll { d1 } => 40,
    AttractTitleBlock { d1 } => 41,
    AttractLogoHold => 42,
    AttractWait300 { second_sleep } => 43,
    AttractRanking => 44,
    StartScreenEntry => 45,
    StartScreen => 46,
    CountrySelectArm => 47,
    CountrySelectLoop { first } => 48,
    CountrySelect4Arm => 49,
    CountrySelect4Loop { first, d6, d7 } => 50,
    ChallengerHold1 => 51,
    ChallengerTyping { writer } => 52,
    ChallengerHold2 => 53,
    ChallengerWipe => 54,
    NameEntryPalette { ret } => 55,
    NameEntryLoop { ret } => 56,
    RankingMarked { ret } => 57,
    BonusJapanPalette { copy } => 58,
    BonusJapanBlinkDraw { copy, d1 } => 59,
    BonusJapanBlinkErase { copy, d1 } => 60,
    BonusJapanMeterStart { copy } => 61,
    BonusJapanMeter { copy, cont, d0 } => 62,
    BonusJapanWindup { copy, a0 } => 63,
    BonusJapanBreak { copy, a1, d2 } => 64,
    BonusJapanSuccessPose { copy } => 65,
    BonusJapanCheerStart { copy } => 66,
    BonusJapanCheerA { copy, d2 } => 67,
    BonusJapanCheerB { copy, d2 } => 68,
    BonusJapanHold1 { copy } => 69,
    BonusJapanHold2 { copy } => 70,
    BonusJapanFailPose { copy } => 71,
    BonusJapanFailLaugh1 { copy } => 72,
    BonusJapanFailLaugh2 { copy } => 73,
    BonusJapanFailLaugh3 { copy } => 74,
    BonusJapanFailEnd { copy } => 75,
    BonusStage1Palette1 { copy } => 76,
    BonusStage1Palette2 { copy } => 77,
    BonusStage1BlinkDraw { copy, d1 } => 78,
    BonusStage1BlinkErase { copy, d1 } => 79,
    BonusStage1Start { copy } => 80,
    BonusStage1Loop { copy } => 81,
    BonusStage1Settle { copy } => 82,
    BonusStage1After { copy, left } => 83,
    BonusStage1Hold1 { copy } => 84,
    BonusStage1Hold2 { copy } => 85,
    EndingStart => 86,
    EndingSceneHold { d1, d4, d5 } => 87,
    EndingMessageSetup => 88,
    EndingMessageTyping { writer } => 89,
    EndingMessageHold => 90,
    EndingStaffSetup => 91,
    EndingStaffHeader => 92,
    EndingStaffStart => 93,
    EndingStaffTyping { d1, writer } => 94,
    EndingStaffHold { d1 } => 95,
    EndingFinalHold => 96,
);

crate::canonical::canonical_enum!(Screen, "Screen.v1";
    Poll => 0,
    ResultTyping { writer, text } => 1,
    ResultHeld { text } => 2,
    ResultSettle => 3,
    Wipe { column, d2, tile, announce } => 4,
    AnnounceCleared => 5,
    AnnounceHeld => 6,
    AnnounceSound { last } => 7,
    SlowMotion { first } => 8,
    ResultTyped { text } => 9,
    TallyStart => 10,
    TallyTime => 11,
    TallyVitality => 12,
    WinText { text } => 13,
    WinCard { held } => 14,
    WinCardCpu => 15,
    VersusWinCard { shown } => 16,
    OpponentIntro { stage } => 17,
    VersusIntro { stage } => 18,
    TravelMapSpawn { a1 } => 19,
    TravelMapLoop => 20,
    TravelMapEnd => 21,
    CountryIntro { a1, stage } => 22,
    DrawTyping { writer } => 23,
    DrawHeld => 24,
    GameOverPalette { next } => 25,
    GameOverHeld => 26,
);

crate::canonical::canonical_enum!(Random, "Random.v1";
    Pass => 0,
    Coins => 1,
    ServiceDebounce => 2,
    ServiceCooldown => 3,
    AttractStarted => 4,
    Pulse { d1, d2, a1 } => 5,
);

crate::canonical::canonical_enum!(Task, "Task.v1";
    New { entry } => 0,
    Boot(value0) => 1,
    ServiceInput(value0) => 2,
    Timer => 3,
    Flow(value0) => 4,
    ServiceMenu(value0) => 5,
    Screen(value0) => 6,
    ServiceItem(value0) => 7,
    Watch { d2 } => 8,
    Random(value0) => 9,
);


impl Io {
    /// No input pressed; DIP switches at the pinned baseline's defaults
    /// (sf.cpp; DSW1 0xdfff: Freeze off, Speed normal; DSW2 0xffff; unmapped
    /// ports 0xc00004, 0xc00006 and 0xc0000e at MAME's `unmap_value_high()`
    /// 0xffff).
    pub fn idle() -> Self {
        Io {
            in0: 0xffff,
            in1: 0xffff,
            unmap_c00004: 0xffff,
            unmap_c00006: 0xffff,
            system: 0xff7f,
            unmap_c0000e: 0xffff,
            dsw1: 0xdfff,
            dsw2: 0xffff,
        }
    }

    /// Read word `index` (`0..8`) at `0xc00000 + 2 * index` (`(A6)+` in
    /// `0x484d4..0x484f6`).
    pub fn port_word(&self, index: usize) -> u16 {
        match index {
            0 => self.in0,
            1 => self.in1,
            2 => self.unmap_c00004,
            3 => self.unmap_c00006,
            4 => self.dsw1,
            5 => self.dsw2,
            6 => self.system,
            7 => self.unmap_c0000e,
            _ => 0xffff,
        }
    }
}

/// What a task does when it stops running this frame.
#[derive(Clone, Copy)]
enum Yield {
    /// Sleep `n` frames (trap #2): ready again `n` vblanks later.
    Sleep(u8),
    /// Exit the current task (trap #3, 0x3b29a..0x3b2b2): clear the running
    /// slot's state byte (+0) through 0x3b2b8 and rescan from slot 0
    /// (0x3b19a).
    Exit,
    /// The task's work overran this frame's cycle budget before reaching
    /// trap #2: the slot stays in state 4 (running) across vblank and
    /// continues in the next frame before rescanning from slot 0.
    Overrun,
    /// An untranslated branch was reached; the slot stops (the address is
    /// already recorded on the bus).
    Halt,
}

/// Service-mode input, DIP and sound-test task (slot 0, 0x483f8..0x48642;
/// ticket #145, F73).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ServiceInput {
    /// 0x483f8: newly spawned entry before `0x3b5c8` and the first palette
    /// upload (`0x1ae68 -> 0x0000` at 0x4840c). `d7` carries the RAM-test
    /// result longword from `0x4834e..0x483a8`.
    Entry { d7: u32 },
    /// 0x3b634: after each of the four palette uploads' 1-frame sleeps at
    /// 0x3b62e (`step = 1..=3` next upload, `step = 4` into `0x48442`).
    Upload { step: u8, d7: u32 },
    /// 0x485ee -> 0x484d4: after the per-frame 1-frame sleep at 0x485e8.
    /// `d7` is the low byte of D7 (the selected sound code).
    Loop { d7: u8 },
}

/// Service-mode menu task (slot 1, 0x48644..0x487f6; ticket #146, F74).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ServiceMenu {
    /// 0x48644: newly spawned entry before erasing the 5 input-screen blocks,
    /// drawing the 9 menu blocks, and sleeping 6 frames at 0x48716.
    Entry,
    /// 0x4871c: after the 6-frame sleep at 0x48716 — erases the highlighted
    /// menu item `d7` (`0..=6`) and sleeps 6 frames at 0x4872a.
    BlinkErase { d7: u8 },
    /// 0x48730: after the 6-frame sleep at 0x4872a — either spawns slot 2 for
    /// item `d7` (when `0xff86b0 > 4`) or redraws item `d7`, steps cursor
    /// (`0x48692..0x48712`), and sleeps 6 frames at 0x48716.
    BlinkRedraw { d7: u8 },
    /// 0x4877e: per-frame item wait loop after `0x3b51a` at 0x48778 or 0x487f0;
    /// exits item `d7` when P1 Up is held (`IN1` bit 3 low) and P1 kick is
    /// released (`0xff86b1 > 4`).
    InItem { d7: u8 },
    /// 0x487e4 -> 0x48678: after the 1-frame exit sleep at 0x487de — redraws
    /// the menu with `d7` preserved (`0x48678` skips `0x48676: clr.w D7`).
    ReturnToMenu { d7: u8 },
}

/// Service-mode test item task (slot 2, 0x487f8..0x4929a; tickets #146 / F74,
/// #147 / F75, and #148 / F76).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ServiceItem {
    /// 0x487f8: item 1 (crosshatch grid) entry before its 18-frame sleep loop
    /// at 0x48850.
    CrosshatchEntry,
    /// 0x48850: item 1 18-frame sleep loop (`0x3b542`).
    CrosshatchLoop,
    /// 0x488f8: item 2 (color bar) entry before its 18-frame sleep loop at
    /// 0x48988.
    ColorBarEntry,
    /// 0x48988: item 2 18-frame sleep loop (`0x3b542`).
    ColorBarLoop,
    /// 0x48858: item 3 (character test 1 / color attribute matrix) entry
    /// before its 18-frame sleep loop at 0x488e0.
    CharTest1Entry,
    /// 0x488e0: item 3 18-frame sleep loop (`0x3b542`).
    CharTest1Loop,
    /// 0x48990: item 4 (character ROM test) entry before its 1-frame sleep at
    /// 0x48a3a.
    CharRomEntry,
    /// 0x48a40: item 4 per-frame `IN1` page/color loop after `0x3b51a` at
    /// 0x48a3a or 0x48afe.
    CharRomLoop,
    /// 0x48b6a: item 5 (sprite / object ROM test) entry before its 1-frame
    /// sleep at 0x48c18 (ticket #147, F75).
    SpriteTestEntry,
    /// 0x48c1e: item 5 per-frame `IN1` page/color loop after `0x3b51a` at
    /// 0x48c18 (ticket #147, F75).
    SpriteTestLoop,
    /// 0x48da4: item 6 (scroll test) entry before its FG palette-load sleep at
    /// 0x49080 (`0x1a268 -> 0x0200`, ticket #148, F76).
    ScrollTestEntry,
    /// 0x49086 -> 0x48e6a: item 6 after the FG palette-load sleep, before the
    /// BG init and BG palette-load sleep at 0x4909c (`0x1ae68 -> 0x0000`).
    ScrollTestInitBg,
    /// 0x490a2 -> 0x48e8c: item 6 after the BG palette-load sleep — enables FG
    /// and BG layers (`gfx_on(0x60)`) and runs the first per-frame scroll pass
    /// (`0x48e96..0x48ff0`).
    ScrollTestStart,
    /// 0x48e96: item 6 per-frame FG + BG scroll loop after `0x3b51a` at
    /// 0x48fea or 0x4909c (`0x48fb2 -> 0x49088 -> 0x48fb6`).
    ScrollTestLoop,
    /// 0x49086 -> 0x48f0c -> 0x48f40: item 6 after an FG threshold-crossing
    /// palette-load sleep at 0x49080 (called from 0x48f08) — skips the FG step
    /// and runs the BG step (`0x48f40..0x48ff0`).
    ScrollTestBgOnly,
    /// 0x490f8: item 7 (output port test) entry before its first palette-load
    /// sleep at 0x49080 (`0x1a268 -> 0x0200`, ticket #147, F75).
    OutputTestEntry,
    /// 0x49086 -> 0x4916a: item 7 after the first palette-load sleep, before
    /// the second palette-load sleep at 0x4909c (`0x1ae68 -> 0x0000`).
    OutputTestPalette2,
    /// 0x490a2 -> 0x49174: item 7 after the second palette-load sleep, before
    /// the initial 30-frame baseline sleep at 0x491ca (`d1 = 0`).
    OutputTestStart,
    /// 0x491d0: item 7 after a 30-frame baseline sleep at 0x491ca — applies the
    /// output entry at `0x491ee + d1` and sleeps 30 frames at 0x491dc.
    OutputTestApply { d1: u16 },
    /// 0x491e2: item 7 after a 30-frame active-output sleep at 0x491dc —
    /// advances `d1` (`+ 2`, wrapping `0x96 -> 0`), resets outputs at 0x49196,
    /// and sleeps 30 frames at 0x491ca.
    OutputTestReset { d1: u16 },
}

/// The boot task's resume points (slot 0, 0x3c0c4..0x3c290; tickets #41, #55).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Boot {
    /// 0x3c0c4: new-task entry before its initial 1-frame sleep at 0x3c0c8.
    Entry,
    /// 0x3c0ce (step 0) and 0x3b634 after each of the four palette uploads'
    /// 1-frame sleeps at 0x3b62e (step 1..=3 next upload, step 4 into
    /// 0x3c112).
    Upload(u8),
    /// 0x3c204 tail into 0x3c20a: the set-up chunk 0x3c112..0x3c204
    /// overruns frame 6 into frame 7 (~139,882 cycles against 133,333 1/3
    /// per frame), so slot 0 stays in state 4 across vblank 7 and sleeps 1
    /// frame at 0x3c20a in frame 7.
    SettleOverrun,
    /// 0x3c210: after the sleep at 0x3c20a — turns on tx (0x08) and starts
    /// typing the first warning block (ticket #55, F15).
    Warning,
    /// 0x3bc10: inside the typewriter (0x3bb98, D4 = 2) for warning block
    /// `index` (0..4; D2 = 3 - index).
    WarningTyping { index: u8, writer: TextWriter },
    /// 0x3c23e: after the 12-frame pause (0x3b53a at 0x3c238) following
    /// warning block `index` (0..4; D2 = 3 - index).
    WarningPause { index: u8 },
    /// 0x3c25a: after the 240-frame warning hold (0x3b57a at 0x3c254):
    /// erases all four warning blocks, enables coin control 0x7070, spawns
    /// slots 1 and 7, and jumps into the timer task at 0x3f4de.
    WarningHold,
}

/// Where `0x42422` (`ranking_check`) returns after name entry and the
/// marked-entry ranking screen (`0x4268a: rts`, ticket #122, F50).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RankingReturn {
    /// 0x3eec2: return from `0x42422` at `0x3ee88 / 0x3eebc` in versus carry-over.
    VersusCarry,
    /// 0x3ef1e / 0x3ef50 -> 0x3efda: return from `0x42422` at `0x3ef18 / 0x3ef4a` after CPU match win.
    CpuMatchLose,
    /// 0x3f2e0: return from P1's `0x42422` at `0x3f2da` in `versus_game_over`.
    GameOverP1,
    /// 0x3f2f0: return from P2's `0x42422` at `0x3f2ea` in `versus_game_over`.
    GameOverP2,
    /// 0x3ce54: attract-mode ranking screen called from `0x3ce4e`.
    Attract,
}

/// The game-flow task's resume points (slot 1).
/// Which copy of the power/timing bonus stage (`0x40d30` after stage 0 vs
/// `0x41a02` after stage 2) is running (tickets #69 / F29 and #130 / F58).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BonusCopy {
    /// Copy 1 after stage 0 (`0x40d30..0x412a4`, grid `0x412aa`).
    Japan,
    /// Copy 2 after stage 2 (`0x41a02..0x41f5c`, grid `0x41f5e`).
    Stage2,
}

impl BonusCopy {
    fn grid_table(self) -> u32 {
        match self {
            BonusCopy::Japan => 0x412aa,
            BonusCopy::Stage2 => 0x41f5e,
        }
    }

    fn meter_shift(self) -> u32 {
        match self {
            BonusCopy::Japan => 0,
            BonusCopy::Stage2 => 0xcd2,
        }
    }

    fn meter_step_src(self) -> u32 {
        match self {
            BonusCopy::Japan => 0x40f1b,
            BonusCopy::Stage2 => 0x41bed,
        }
    }

    fn restart_table(self) -> u32 {
        match self {
            BonusCopy::Japan => 0x4101e,
            BonusCopy::Stage2 => 0x41cf0,
        }
    }

    fn threshold_table(self) -> u32 {
        match self {
            BonusCopy::Japan => 0x410d0,
            BonusCopy::Stage2 => 0x41d96,
        }
    }

    fn sound_table(self) -> u32 {
        match self {
            BonusCopy::Japan => 0x4112a,
            BonusCopy::Stage2 => 0x41de0,
        }
    }

    fn result_table(self) -> u32 {
        match self {
            BonusCopy::Japan => 0x4110a,
            BonusCopy::Stage2 => 0x41dc8,
        }
    }
}

/// Which copy of the 10-second object-breaking bonus stage (`0x41740` after
/// stage 1 vs `0x41fe6` after stage 3) is running (tickets #124 / F52 and
/// #133 / F61).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bonus10Copy {
    /// Copy 1 after stage 1 (`0x41740..0x419fc`, 3 stacks, `5,000` max bonus).
    Stage1,
    /// Copy 2 after stage 3 (`0x41fe6..0x422b8`, 4 stacks, `8,000` max bonus).
    Stage3,
}

impl Bonus10Copy {
    fn shift(self) -> u32 {
        match self {
            Bonus10Copy::Stage1 => 0,
            Bonus10Copy::Stage3 => 0x8b8,
        }
    }

    fn templates(self) -> &'static [u32] {
        match self {
            Bonus10Copy::Stage1 => &[0x18e24, 0x18e44, 0x18e64],
            Bonus10Copy::Stage3 => &[0x18e44, 0x18e64, 0x18e84, 0x18ea4],
        }
    }

    fn fg_scroll(self) -> u16 {
        match self {
            Bonus10Copy::Stage1 => 0x1680,
            Bonus10Copy::Stage3 => 0x1800,
        }
    }

    fn max_bonus_addr(self) -> u32 {
        0x41936 + self.shift()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flow {
    /// 0x3ea3e: after the round-start sleep — the fight begins.
    RoundStart,
    /// 0x3eafa: after the fight loop's sleep.
    FightLoop,
    /// 0x3eb28: after the KO path's first sleep.
    KoStart,
    /// 0x3eb62: the fighters finishing their moves after the KO (loop
    /// 0x3eb44).
    KoSettle,
    /// 0x3ec0e: the winner's victory pose playing out (loop 0x3ebf0).
    Victory,
    /// 0x3ec2a: waiting for the round-result event to finish (loop
    /// 0x3ec24).
    AwaitResult,
    /// 0x422e0 (called from 0x3ec52): waiting for the next-round wipe
    /// (event 6).
    AwaitWipe,
    /// 0x422e0 (called from 0x3e786 / 0x3e6f4): waiting for the round
    /// announcement (event 5); `new_opponent` after the match end, when
    /// the opponent's tables are set up before the HUD (0x3e714).
    AwaitAnnounce { new_opponent: bool },
    /// 0x422e0 (called from 0x3ef62, or 0x3ee6a / 0x3ee9c in versus):
    /// waiting for the match-won screen (event 0xb).
    AwaitWinScreen { versus: bool },
    /// 0x422e0 (called from 0x3ef6c, or 0x3ee74 / 0x3eea6 in versus):
    /// waiting for the wipe after it.
    AwaitMatchWipe { versus: bool },
    /// 0x3eee6: after the versus carry-over's stage-palette sleep.
    VersusCarryStagePalette,
    /// 0x3eeec: after its country-palette sleep, before player placement.
    VersusCarryCountryPalette,
    /// 0x422e0 (called from 0x3eefe / 0x3ef30): waiting for the CPU's
    /// match-won screen (event 0xb).
    AwaitCpuWinScreen,
    /// 0x422e0 (called from 0x3ef08 / 0x3ef3a): waiting for the wipe after
    /// the CPU's match-won screen before `0x42422` and `0x3efda`.
    AwaitCpuMatchWipe,
    /// 0x3f04c: after the continue screen's first palette request sleep
    /// (`0x1a268 -> 0x200` at 0x3f046).
    ContinuePalette1,
    /// 0x3f066: after the continue screen's second palette request sleep
    /// (`0x1ae68 -> 0x000` at 0x3f060).
    ContinuePalette2,
    /// 0x3f092 / 0x3f162: the continue countdown loop (`0x3f092..0x3f162`).
    ContinueLoop,
    /// 0x3f172: after the continue acceptance's 1-frame sleep at 0x3f16c
    /// (ticket #79, F39).
    ContinueAccept,
    /// 0x3f1b2: after the continue acceptance's country-palette sleep
    /// (`0x436b8` called from 0x3f1ac).
    ContinueCountryPalette,
    /// 0x3f1b8: after the continue acceptance's stage-palette sleep
    /// (`0x43748` called from 0x3f1b2), before re-entering `0x3e6ca`.
    ContinueStagePalette,
    /// 0x3ed26: the time-over path after its first sleep.
    TimeOverStart,
    /// 0x3ed96: the time-over hold (0x3ed46: 0x82 counted down before each
    /// sleep, `left` to go) while the fighters finish their moves.
    TimeOverHold { left: u16 },
    /// 0x422e0 (called from 0x3edec): waiting for the draw text (event 8).
    AwaitDraw,
    /// 0x3ecca: the double KO's settle loop after its sleep.
    DoubleKoSettle,
    /// 0x3ed00: the double KO text's hold (0x3ecec: 0x82 counted down
    /// before each sleep).
    DoubleKoHold { left: u16 },
    /// 0x422e0 (called from 0x3ee52): waiting for the wipe before the
    /// game over (0x3f2d0) after a versus match with no winner.
    AwaitGameOverWipe,
    /// 0x3f204: the game-over effect cycle; `index` is the current table
    /// entry, `effect_ticks` its remaining run/sleep count, and `entries_left`
    /// the D3 count of further entries.
    GameOverCycle {
        index: u16,
        effect_ticks: u16,
        entries_left: u8,
    },
    /// 0x3f25a: run and count the remaining game-over effects.
    GameOverDrain,
    /// 0x3f26a: wait for event 10 to clear the scene bit, then reset.
    GameOverAwaitEvent,
    /// 0x3f32c: the shared game-over screen after its palette-request sleep.
    GameOverPalette,
    /// 0x3f33c: the shared game-over screen after its 240-frame hold.
    GameOverHold,
    /// 0x3e59c: stage entry after country select (or stage step 0x3efc4):
    /// object clears, demo check and event-2 request (ticket #44, F4).
    StageStart,
    /// 0x436e6 (with caller 0x3e5d6): after the demo stage-start country
    /// palette sleep at 0x3e5d0 (`0x3e5c6..0x3e5d0`, ticket #88, F48).
    DemoCountryPalette,
    /// 0x43776 (with caller 0x3e704): after the demo next-opponent stage
    /// palette sleep at 0x3e6fe (`0x3e6fe..0x3e70e`, ticket #88, F48).
    DemoStagePalette,
    /// 0x422e0 (called from 0x3e5bc): waiting for event 2 / event 3
    /// (country screen) before player placement at 0x3e5d6.
    AwaitCountryScreen,
    /// 0x422e0 (called from 0x3e6e4): waiting for the next opponent's
    /// stage intro (event 4).
    AwaitOpponentIntro,
    /// 0x3ea30: the intro loop (0x3e9bc) — the fighters walking in.
    Intro,
    /// 0x3ea0e: FIGHT! shown; the wait's counter (0x1e, counted down
    /// before each sleep).
    FightShown { left: u16 },
    /// 0x3ccae: slot 1's entry — spawns slot 3 at 0x3f374, disables layers
    /// 0xe8, clears the text layer, requests palette 0x1a268 at 0x200, and
    /// sleeps 1 frame at 0x3cce6 into `AttractStart` (ticket #55, F15).
    AttractJoin,
    /// 0x3ccec: the attract loop's start after its initial 1-frame sleep at
    /// 0x3cce6 (ticket #43, F3).
    AttractStart,
    /// 0x3cd50: after a sleep in the ten-entry attract scroll loop
    /// (0x3cd14..0x3cd54; `d1` is the 0..=9 entry index in D1).
    AttractScroll { d1: u16 },
    /// 0x3cd86: after a sleep in the three-entry title-block loop
    /// (0x3cd60..0x3cd94; `d1` is the 0..=2 entry index in D1).
    AttractTitleBlock { d1: u16 },
    /// 0x3cda8: after the title logo's 60-frame hold at 0x3cda2.
    AttractLogoHold,
    /// 0x3b588 (`second_sleep == false`) and 0x3cdb8 (`second_sleep == true`):
    /// the 300-frame title wait (0x3cdb2 -> 0x3b582: 240 frames at 0x3b57a
    /// into 0x3b588, then 60 frames at 0x3b562 into 0x3cdb8).
    AttractWait300 { second_sleep: bool },
    /// 0x42736 (with caller 0x3ce54 on stack): the attract ranking screen's
    /// timer-3 wait loop (0x4268c..0x4274e; ticket #56, F16).
    AttractRanking,
    /// 0x3dec2: the start-screen entry setup (0x3dec2..0x3df48; ticket #73, F33).
    StartScreenEntry,
    /// 0x3dfce: the start-screen loop (0x3df4c..0x3e020; ticket #52, F12).
    StartScreen,
    /// 0x3e0e6: two-country select after its initial 1-frame palette/sound
    /// sleep at 0x3e0e0 (ticket #74, F34).
    CountrySelectArm,
    /// 0x3e110 (`first == true`, after 0x3e10a) / 0x3e13a (`first == false`,
    /// after 0x3e134): the two-country select poll loop (`0x3e110..0x3e18a`;
    /// ticket #74, F34).
    CountrySelectLoop { first: bool },
    /// 0x3e252: four-country select after its initial 1-frame palette/sound
    /// sleep at 0x3e24c (ticket #87, F47).
    CountrySelect4Arm,
    /// 0x3e27c (`first == true`, after 0x3e276) / 0x3e32c (`first == false`,
    /// after 0x3e326): the four-country select poll loop (`0x3e27c..0x3e32f`),
    /// with the 2x2 cursor coordinates `d6` (column 0..=1) and `d7` (row 0..=1)
    /// kept in task state (ticket #87, F47).
    CountrySelect4Loop { first: bool, d6: u16, d7: u16 },
    /// 0x423de: after the challenger announcement's 120-frame hold (`0x3b56e`
    /// at `0x423d8`, ticket #81, F41).
    ChallengerHold1,
    /// 0x3bc10 (with caller `0x423ee`): inside the typewriter (`0x3bb98`,
    /// `D4 = 3`) for the challenger subtitle block `0x15c30` (ticket #81, F41).
    ChallengerTyping { writer: TextWriter },
    /// 0x423f4: after the challenger subtitle's 240-frame hold (`0x3b57a` at
    /// `0x423ee`, ticket #81, F41).
    ChallengerHold2,
    /// 0x422e0 (called from `0x423f6`, caller `0x423fc`): waiting for event 6's
    /// wipe after the challenger screen before converting the fight to versus
    /// at `0x423fc..0x4241c` (ticket #81, F41).
    ChallengerWipe,
    /// 0x42552: after the name-entry screen's initial 1-frame palette/timer
    /// sleep (`0x3b51a` at `0x4254c`, ticket #82, F42).
    NameEntryPalette { ret: RankingReturn },
    /// 0x42584: the name-entry countdown and cursor loop (`0x42578..0x425c2`,
    /// after `0x3b51a` at `0x4257e`, ticket #82, F42).
    NameEntryLoop { ret: RankingReturn },
    /// 0x4275c: the marked-entry ranking screen blink loop (`0x42750..0x427e0`,
    /// after `0x3b51a` at `0x42756`, ticket #122, F50).
    RankingMarked { ret: RankingReturn },
    /// 0x40dd4 / 0x41aa6: after the power/timing bonus stage's initial 1-frame
    /// palette sleep (`0x3b51a` at `0x40dce` / `0x41aa0`, tickets #69 / F29 and
    /// #130 / F58).
    BonusJapanPalette { copy: BonusCopy },
    /// 0x40dfc / 0x41ace: before drawing `PUSH PUNCH BUTTON` (`0x147d2`) in the
    /// 5-blink prompt loop (`0x40dfc..0x40e1a` / `0x41ace..0x41aec`,
    /// `d1 = 3..=0`, F29 / F58).
    BonusJapanBlinkDraw { copy: BonusCopy, d1: u16 },
    /// 0x40e0e / 0x41ae0: before erasing `PUSH PUNCH BUTTON` (`0x147d2`) in the
    /// 5-blink prompt loop (`0x40dfc..0x40e1a` / `0x41ace..0x41aec`,
    /// `d1 = 4..=0`, F29 / F58).
    BonusJapanBlinkErase { copy: BonusCopy, d1: u16 },
    /// 0x40e56 / 0x41b28: after the 1-frame arm sleep at `0x40e50` / `0x41b22`,
    /// before queuing sound `0x3a` and entering the power-meter loop
    /// (`0x40e56..0x4103c` / `0x41b28..0x41d0e`, F29 / F58).
    BonusJapanMeterStart { copy: BonusCopy },
    /// 0x40f14 / 0x41be6: after the 1-frame sleep at `0x40f0e` / `0x41be0` in
    /// the power-meter loop, with `cont` (`A0` on stack) and `d0` (loop counter
    /// on stack when `cont == 0x40ffe` or `0x41cd0`) preserved (F29 / F58).
    BonusJapanMeter { copy: BonusCopy, cont: u32, d0: u16 },
    /// 0x41078 / 0x41d4e: after a wind-up scroll sleep (`0x3b590` at `0x41072`
    /// / `0x41d48`) with table pointer `a0` (F29 / F58).
    BonusJapanWindup { copy: BonusCopy, a0: u32 },
    /// 0x41158 / 0x41e0c: after a break animation step's sleep (`0x3b590` at
    /// `0x41152` / `0x41e06`) with stream pointer `a1` and remaining count `d2`
    /// (F29 / F58).
    BonusJapanBreak { copy: BonusCopy, a1: u32, d2: u16 },
    /// 0x41166 / 0x41e1a: after the 90-frame hold (`0x3b56e` at `0x41160` /
    /// `0x41e14`) following the break (F29 / F58).
    BonusJapanSuccessPose { copy: BonusCopy },
    /// 0x41184 / 0x41e38: after the 30-frame hold (`0x3b552` at `0x4117e` /
    /// `0x41e32`) before the 15-iteration score-tally cheer loop (F29 / F58).
    BonusJapanCheerStart { copy: BonusCopy },
    /// 0x411a6 / 0x41e5a: after the first 6-frame cheer sleep (`0x3b522` at
    /// `0x411a0` / `0x41e54`) in the 15-iteration cheer loop (`d2 = 14..=0`,
    /// F29 / F58).
    BonusJapanCheerA { copy: BonusCopy, d2: u16 },
    /// 0x411ba / 0x41e6e: after the second 6-frame cheer sleep (`0x3b522` at
    /// `0x411b4` / `0x41e68`) in the 15-iteration cheer loop (`d2 = 14..=0`,
    /// F29 / F58).
    BonusJapanCheerB { copy: BonusCopy, d2: u16 },
    /// 0x4120c / 0x41ec0: after the first 120-frame closing hold (`0x3b56e` at
    /// `0x41206` / `0x41eba`, F29 / F58).
    BonusJapanHold1 { copy: BonusCopy },
    /// 0x41212 / 0x41ec6: after the second 120-frame closing hold (`0x3b56e` at
    /// `0x4120c` / `0x41ec0`, F29 / F58).
    BonusJapanHold2 { copy: BonusCopy },
    /// 0x4123a / 0x41eee: after the 90-frame initial failure sleep (`0x3b590` at
    /// `0x4122e` / `0x41ee2`, F29 / F58).
    BonusJapanFailPose { copy: BonusCopy },
    /// 0x41258 / 0x41f0c: after the 60-frame failure pose sleep (`0x3b562` at
    /// `0x41252` / `0x41f06`, F29 / F58).
    BonusJapanFailLaugh1 { copy: BonusCopy },
    /// 0x41276 / 0x41f2a: after the first 60-frame failure laugh sleep (`0x3b562`
    /// at `0x41270` / `0x41f24`, F29 / F58).
    BonusJapanFailLaugh2 { copy: BonusCopy },
    /// 0x41286 / 0x41f3a: after the second 60-frame failure laugh sleep
    /// (`0x3b562` at `0x41280` / `0x41f34`, F29 / F58).
    BonusJapanFailLaugh3 { copy: BonusCopy },
    /// 0x41296 / 0x41f4a: after the 180-frame failure closing hold (`0x3b572` at
    /// `0x41290` / `0x41f44`, F29 / F58).
    BonusJapanFailEnd { copy: BonusCopy },
    /// 0x41814 / 0x420cc: after the 10-second bonus's first palette sleep
    /// (`0x3b51a` at `0x4180e` / `0x420c6`, tickets #124 / F52 and #133 / F61).
    BonusStage1Palette1 { copy: Bonus10Copy },
    /// 0x4182e / 0x420e6: after the 10-second bonus's second palette sleep
    /// (`0x3b51a` at `0x41828` / `0x420e0`, F52 / F61).
    BonusStage1Palette2 { copy: Bonus10Copy },
    /// 0x4185c / 0x42114: before drawing `BREAK THE OBJECTS` (`0x14802`) in the
    /// 5-blink prompt loop (`0x41856..0x4187a` / `0x4210e..0x42132`, `d1 = 3..=0`,
    /// F52 / F61).
    BonusStage1BlinkDraw { copy: Bonus10Copy, d1: u16 },
    /// 0x4186e / 0x42126: before erasing `BREAK THE OBJECTS` (`0x14802`) in the
    /// 5-blink prompt loop (`0x41856..0x4187a` / `0x4210e..0x42132`, `d1 = 4..=0`,
    /// F52 / F61).
    BonusStage1BlinkErase { copy: Bonus10Copy, d1: u16 },
    /// 0x418ac / 0x42164: after the 1-frame arm sleep at `0x418a6` / `0x4215e`,
    /// entering the 10-second object-breaking loop (`0x418ac..0x41934` /
    /// `0x42164..0x421ec`, F52 / F61).
    BonusStage1Start { copy: Bonus10Copy },
    /// 0x4191c / 0x421d4: after the `0x3b5aa` sleep at `0x41916` / `0x421ce` in
    /// the 10-second bonus loop, checking whether `0xff8f1c..0xff8f1e` reached
    /// `5,000` (`0x41936`) or `8,000` (`0x421ee`, F52 / F61).
    BonusStage1Loop { copy: Bonus10Copy },
    /// 0x41962 / 0x4221a: after the `0x3b5aa` sleep at `0x4195c` / `0x42214`,
    /// waiting for the fighter to finish its current move (`+9 & 0x80 == 0`,
    /// F52 / F61).
    BonusStage1Settle { copy: Bonus10Copy },
    /// 0x419a0 / 0x42258: the 10-frame post-settle effect/fighter drain
    /// (`0x4198a..0x419ac` / `0x42242..0x42264`, `left = 10..=1` on stack,
    /// F52 / F61).
    BonusStage1After { copy: Bonus10Copy, left: u16 },
    /// 0x419b4 / 0x4226c: after the first 120-frame hold (`0x3b56a` at `0x419ae`
    /// / `0x42266`), before erasing `TIME OVER` (`0x14f74`) and drawing
    /// `BONUS PTS.` (`0x14812`) and `0xff8f1c..0xff8f1f` at `0x8005bc` (F52 / F61).
    BonusStage1Hold1 { copy: Bonus10Copy },
    /// 0x419ee / 0x422a6: after the second 120-frame hold (`0x3b56a` at `0x419e8`
    /// / `0x422a0`), stopping sound (`0x3f`), clearing `SCENE_MODE`, and
    /// advancing the stage at `0x3efc4` (F52 / F61).
    BonusStage1Hold2 { copy: Bonus10Copy },
    /// 0x44a60: the ending fixture resume point after `0x44a40..0x44a5e`
    /// (`0x3b51a` at `0x44a5a`, ticket #136, F64).
    EndingStart,
    /// 0x44ae8: after a 120-frame hold (`0x3b56a` at `0x44ae2`) on one of the
    /// ten 2x2-tile country-card sub-steps across the five ending scenes
    /// (`d1 = 0..=4`, `d4 = 1..=0`, `d5 = (byte & 0x0f) << 2`, F64).
    EndingSceneHold { d1: u16, d4: u16, d5: u16 },
    /// 0x44b20: after the 1-frame sleep (`0x3b51a` at `0x44b1a`) before the
    /// congratulatory message (`0x44d60`, F64).
    EndingMessageSetup,
    /// 0x3bc10 (with caller `0x44b72`): inside the typewriter (`0x3bb98`,
    /// `D4 = 5`, `TEXT_SOUND = 0x000d`) for `0x44d60` (F64).
    EndingMessageTyping { writer: TextWriter },
    /// 0x44b78: after the congratulatory message's 120-frame hold (`0x3b56a`
    /// at `0x44b72`, F64).
    EndingMessageHold,
    /// 0x44b88: after the 1-frame sleep (`0x3b51a` at `0x44b82`) following the
    /// erase of `0x44d60`, starting the staff roll (`0x36`, F64).
    EndingStaffSetup,
    /// 0x44ba8: after the 1-frame sleep (`0x3b51a` at `0x44ba2`) before drawing
    /// the 2x2 staff header `*(0x44f58)` (`0x44f80`, F64).
    EndingStaffHeader,
    /// 0x44bc8: after the 2-frame sleep (`0x3b522` at `0x44bc2`) following the
    /// staff header, starting the 8-block staff loop (`d1 = 7..=0`, F64).
    EndingStaffStart,
    /// 0x3bc10 (with caller `0x44bd4`): inside the typewriter (`0x3bb98`,
    /// `D4 = 2`, `TEXT_SOUND = 0`) for staff block `7 - d1` (`0x44f5c..0x44f7b`,
    /// F64).
    EndingStaffTyping { d1: u16, writer: TextWriter },
    /// 0x44bda: after a staff block's 120-frame hold (`0x3b56a` at `0x44bd4`,
    /// `d1 = 7..=0`, F64).
    EndingStaffHold { d1: u16 },
    /// 0x44c04: after the 120-frame hold (`0x3b56a` at `0x44bfe`) on the 2x2
    /// `*(0x44f7c)` (`0x452cc`) closing block, before erasing it, queuing
    /// `0x3b7a8`, and jumping to `0x3efc4` (F64).
    EndingFinalHold,
}

/// The screen-event task's resume points (slot 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    /// 0x3f7d6: the event poll.
    Poll,
    /// Inside the round-result text's typewriter (0x3bc10).
    ResultTyping { writer: TextWriter, text: u32 },
    /// 0x3fd46: the result text held.
    ResultHeld { text: u32 },
    /// 0x3fee0: after the result's closing wait.
    ResultSettle,
    /// Event 6 (0x3fc8a) / the second half of event 5 (0x3fbf8): a column
    /// wipe — `column` columns done, the next column's top at `d2`, the
    /// tile `tile`; `announce` for event 5.
    Wipe {
        column: u16,
        d2: u16,
        tile: u16,
        announce: bool,
    },
    /// Event 5 (0x3fbc2): after the screen clear's 6-frame sleep.
    AnnounceCleared,
    /// Event 5 (0x3fbf8): after the round text's 60-frame hold.
    AnnounceHeld,
    /// Event 5 (0x3fc48 / 0x3fc58): after the closing sounds' sleeps.
    AnnounceSound { last: bool },
    /// Event 9 (0x3ff3c / 0x3ff42): the slow motion after a projectile
    /// hit, `first` after its 60-frame sleep, else after its 30.
    SlowMotion { first: bool },
    /// 0x3fd94: a human winner's result text after its 30-frame hold.
    ResultTyped { text: u32 },
    /// 0x3fdf8: the bonus screen shown; the tally begins.
    TallyStart,
    /// 0x3fe62: after a time tick's 3-frame sleep.
    TallyTime,
    /// 0x3fece: after a vitality tick's 3-frame sleep.
    TallyVitality,
    /// Event 0xb (0x40104, or 0x40132 / 0x40056 in the other modes): the
    /// match text after its 60-frame hold.
    WinText { text: u32 },
    /// Event 0xb (0x401e6 / 0x401ec): the winner's card after its 180-frame
    /// hold, then (`held`) after its 30.
    WinCard { held: bool },
    /// Event 0xb (0x4029a): the CPU's card after its 240-frame hold.
    WinCardCpu,
    /// Event 0xb in versus (0x400c2 / 0x400d4): after the one-frame sleep
    /// before the winner's text, then (`shown`) after its 120-frame hold.
    VersusWinCard { shown: bool },
    /// Event 4 (0x43776 / 0x3fb2e / 0x3fb34): the stage intro after the
    /// palette request's sleep, its 120-frame hold and its 30-frame hold.
    OpponentIntro { stage: u8 },
    /// Event 4 in versus (0x3fa46 / 0x3fa4c / 0x43776): after the 120- and
    /// 30-frame holds, then after the palette request's sleep.
    VersusIntro { stage: u8 },
    /// Event 2 (0x3f87c): after the travel map's 120-frame opening hold
    /// (`0x3b56a` at `0x3f876`, ticket #83, F43), with `a1` pointing to the
    /// country/stage plane effect template pointer in `0x3f8a4..0x3f8e0`.
    TravelMapSpawn { a1: u32 },
    /// Event 2 (0x3f890): after a 1-frame plane-effect tick (`0x3b51a` at
    /// `0x3f88a`, ticket #83, F43).
    TravelMapLoop,
    /// Event 2 (0x3f8a0): after the 60-frame post-flight hold (`0x3b562` at
    /// `0x3f89a`, ticket #83, F43) before entering `0x3f8e4`.
    TravelMapEnd,
    /// Event 3 (0x436e6 / 0x3f95e / 0x3f964): the country screen after the
    /// palette request's sleep (`stage == 0`), after the 60-frame hold
    /// (`stage == 1`), and after the 30-frame hold (`stage == 2`).
    CountryIntro { a1: u32, stage: u8 },
    /// Event 8 (0x3fef2): inside the draw text's typewriter (0x3bc10).
    DrawTyping { writer: TextWriter },
    /// Event 8 (0x3ff0e): after the draw text's 120-frame hold.
    DrawHeld,
    /// Event 10 (0x3ff82): after a timed game-over palette write; `next` is
    /// the next of the 25 table entries, or 25 once all writes are done.
    GameOverPalette { next: u8 },
    /// Event 10 (0x3ffae): after the 180-frame hold; clear the scene bit.
    GameOverHeld,
}

/// The random/coin task's resume points (slot 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Random {
    /// 0x48270: after the 6-frame sleep — the loop goes back to the pass.
    Pass,
    /// 0x4819e: after the pass's 1-frame sleep — the coin work.
    Coins,
    /// 0x481e8: after the service switch's 6-frame debounce sleep at 0x481e2.
    ServiceDebounce,
    /// 0x48212: after the service credit's 30-frame cooldown sleep at 0x4820c.
    ServiceCooldown,
    /// 0x4824e: after the attract-start 1-frame sleep at 0x48248 (ticket #73, F33).
    AttractStarted,
    /// 0x48344: after a coin counter pulse's 6-frame sleep at 0x4833e, with
    /// loop counter `d1` (3..=0), pulse mask `d2` (from 0x48274 + (d1 << 1))
    /// and coin-slot pointer `a1` preserved.
    Pulse { d1: u16, d2: u16, a1: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Task {
    /// Newly spawned slot (0x3b20a) awaiting its first entry at
    /// 0x3b1e8..0x3b1f8.
    New {
        entry: u32,
    },
    /// Slot 0's boot task (0x3c0c4..0x3c20a).
    Boot(Boot),
    /// Slot 0's service-mode input, DIP and sound test (0x483f8..0x48642).
    ServiceInput(ServiceInput),
    /// Slot 0's round timer and screen/countdown timers (0x3f4e2 loop).
    Timer,
    Flow(Flow),
    /// Slot 1's service-mode test menu (0x48644..0x487f6).
    ServiceMenu(ServiceMenu),
    /// Slot 2 (0x3f7b4 loop).
    Screen(Screen),
    /// Slot 2's service-mode test item (0x487f8..0x48b68).
    ServiceItem(ServiceItem),
    /// Slot 3 (0x3f388 loop); `d2` is the task's D2 (the other player's
    /// start-button mask), kept across its sleeps.
    Watch {
        d2: u16,
    },
    Random(Random),
}

/// The scheduler: which translated task each slot holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scheduler {
    tasks: [Option<Task>; SLOT_COUNT as usize],
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// 0x3b2be: slot address in the slot table (A5 + 6 + (D0 << 4)).
pub fn slot_address(slot: u8) -> u32 {
    SLOTS + ((slot as u32) << 4)
}

/// 0x3b2b8: the running slot's address (0xff8001 into 0x3b2be).
pub fn current_slot_address(bus: &Bus) -> u32 {
    slot_address(bus.rb(CURRENT_SLOT))
}

/// 0x3b1e8..0x3b1f8: enter a newly spawned task (state bit 0 set) at its
/// entry address `($4,A2,D0.w)`.
fn enter_new_task(k: u32, entry: u32, bus: &mut Bus) -> Option<Task> {
    match (k, entry) {
        (0, 0x3c0c4) => Some(Task::Boot(Boot::Entry)),
        (0, 0x483f8) => Some(Task::ServiceInput(ServiceInput::Entry { d7: 0 })),
        (1, 0x3ccae) => Some(Task::Flow(Flow::AttractJoin)),
        (1, 0x3dec2) => Some(Task::Flow(Flow::StartScreenEntry)),
        (1, 0x48644) => Some(Task::ServiceMenu(ServiceMenu::Entry)),
        (2, 0x3f7b0) => Some(Task::Screen(Screen::Poll)),
        (2, 0x487f8) => Some(Task::ServiceItem(ServiceItem::CrosshatchEntry)),
        (2, 0x488f8) => Some(Task::ServiceItem(ServiceItem::ColorBarEntry)),
        (2, 0x48858) => Some(Task::ServiceItem(ServiceItem::CharTest1Entry)),
        (2, 0x48990) => Some(Task::ServiceItem(ServiceItem::CharRomEntry)),
        (2, 0x48b6a) => Some(Task::ServiceItem(ServiceItem::SpriteTestEntry)),
        (2, 0x48da4) => Some(Task::ServiceItem(ServiceItem::ScrollTestEntry)),
        (2, 0x490f8) => Some(Task::ServiceItem(ServiceItem::OutputTestEntry)),
        (3, 0x3f374) => {
            // 0x3f374..0x3f386: lea ($422,A5),A7; store 0x3f3be into 0xff8f20
            // and 0x3f41e into 0xff8f24 before entering 0x3f388.
            bus.wl(WATCH_NEXT, 0x3f3be);
            bus.wl(WATCH_STATE, 0x3f41e);
            Some(Task::Watch { d2: 0 })
        }
        (7, 0x4813e) => Some(Task::Random(Random::Pass)),
        _ => {
            bus.note(entry);
            None
        }
    }
}

impl Scheduler {
    /// Empty slot table at power-on before reset spawns slot 0.
    pub fn new() -> Self {
        Scheduler {
            tasks: [None; SLOT_COUNT as usize],
        }
    }

    /// 0x3b20a (trap #0, 0x3b216..0x3b22e): spawn task `slot` (D0) at
    /// `entry` (A0): state 1 (new) at +0, sleep 0 at +1; 0x3b1e8 enters it
    /// when the scheduler scan reaches the slot.
    pub fn spawn(&mut self, bus: &mut Bus, slot: u8, entry: u32) {
        let a1 = slot_address(slot);
        bus.wb(a1, 1);
        bus.wb(a1 + 1, 0);
        self.tasks[slot as usize] = Some(Task::New { entry });
    }

    /// 0x3b230 (trap #1, 0x3b23c..0x3b24c): kill task `slot` (D0) — clear
    /// its state byte (+0) through 0x3b2be and drop its continuation.
    /// Translated in F1 (#41) and labelled unexercised; F33 (0x48232), F25
    /// (0x3f368) and F55 (0x3ceb2) exercise it first.
    pub fn kill(&mut self, bus: &mut Bus, slot: u8) {
        let a1 = slot_address(slot);
        bus.wb(a1, 0);
        self.tasks[slot as usize] = None;
    }
    /// Decode the task positions of a fixture's work RAM (0xff8000-based).
    pub fn from_fixture(work: &[u8]) -> Result<Self, String> {
        let rl = |a: u32| -> u32 {
            let i = (a - 0xff8000) as usize;
            u32::from_be_bytes([work[i], work[i + 1], work[i + 2], work[i + 3]])
        };
        let rw = |a: u32| -> u16 {
            let i = (a - 0xff8000) as usize;
            u16::from_be_bytes([work[i], work[i + 1]])
        };
        let mut tasks = [None; SLOT_COUNT as usize];
        for k in 0..SLOT_COUNT {
            let slot = SLOTS + k * 0x10;
            let state = work[(slot - 0xff8000) as usize];
            if state == 0 {
                continue;
            }
            if state != 8 {
                return Err(format!(
                    "task slot {k} is in state {state:#x}, not sleeping"
                ));
            }
            let usp = rl(slot + 8);
            // The sleep wrapper's movem saved D0-D7/A0-A6 (60 bytes); above
            // them the return into the sleep helper, then into the body.
            if rl(usp + 60) != 0x3b5a8 {
                return Err(format!("task slot {k} sleeps outside the sleep helpers"));
            }
            let resume = rl(usp + 64);
            let task = match (k, resume) {
                (0, 0x3f7ac) => Task::Timer,
                (1, 0x3ea3e) => Task::Flow(Flow::RoundStart),
                (1, 0x3eafa) => Task::Flow(Flow::FightLoop),
                (1, 0x3eb28) => Task::Flow(Flow::KoStart),
                (1, 0x3eb62) => Task::Flow(Flow::KoSettle),
                (1, 0x3ec0e) => Task::Flow(Flow::Victory),
                (1, 0x3ec2a) => Task::Flow(Flow::AwaitResult),
                (1, 0x3e59c) => Task::Flow(Flow::StageStart),
                (1, 0x3ea0e) => Task::Flow(Flow::FightShown { left: rw(usp + 68) }),
                (1, 0x3ea30) => Task::Flow(Flow::Intro),
                (1, 0x422e0) => match rl(usp + 68) {
                    0x3e5c2 => Task::Flow(Flow::AwaitCountryScreen),
                    0x3e6ea => Task::Flow(Flow::AwaitOpponentIntro),
                    0x3e6fa => Task::Flow(Flow::AwaitAnnounce { new_opponent: true }),
                    0x3e78c => Task::Flow(Flow::AwaitAnnounce {
                        new_opponent: false,
                    }),
                    0x3ec58 => Task::Flow(Flow::AwaitWipe),
                    0x3ee70 | 0x3eea2 => Task::Flow(Flow::AwaitWinScreen { versus: true }),
                    0x3ee7a | 0x3eeac => Task::Flow(Flow::AwaitMatchWipe { versus: true }),
                    0x3ef68 => Task::Flow(Flow::AwaitWinScreen { versus: false }),
                    0x3ef72 => Task::Flow(Flow::AwaitMatchWipe { versus: false }),
                    0x423fc => Task::Flow(Flow::ChallengerWipe),
                    caller => {
                        return Err(format!(
                            "task slot 1 sleeps in 0x422dc/0x422e0 with caller {caller:#07x}"
                        ))
                    }
                },
                (1, 0x423de) => Task::Flow(Flow::ChallengerHold1),
                (1, 0x423f4) => Task::Flow(Flow::ChallengerHold2),
                (1, 0x42552 | 0x42584) => {
                    let caller = rl(usp + 74);
                    let ret = match caller {
                        0x3eec2 => RankingReturn::VersusCarry,
                        0x3ef1e | 0x3ef50 => RankingReturn::CpuMatchLose,
                        0x3f2e0 => RankingReturn::GameOverP1,
                        0x3f2f0 => RankingReturn::GameOverP2,
                        _ => {
                            return Err(format!(
                                "task slot 1 sleeps in {resume:#07x} with caller {caller:#07x}"
                            ))
                        }
                    };
                    if resume == 0x42552 {
                        Task::Flow(Flow::NameEntryPalette { ret })
                    } else {
                        Task::Flow(Flow::NameEntryLoop { ret })
                    }
                }
                (1, 0x4275c) => {
                    let caller = rl(usp + 68);
                    let ret = match caller {
                        0x3ce54 => RankingReturn::Attract,
                        0x4267a => match rl(usp + 72) {
                            0x3eec2 => RankingReturn::VersusCarry,
                            0x3ef1e | 0x3ef50 => RankingReturn::CpuMatchLose,
                            0x3f2e0 => RankingReturn::GameOverP1,
                            0x3f2f0 => RankingReturn::GameOverP2,
                            ret => {
                                return Err(format!(
                                    "task slot 1 sleeps in 0x4275c/0x4267a with caller {ret:#07x}"
                                ))
                            }
                        },
                        _ => {
                            return Err(format!(
                                "task slot 1 sleeps in 0x4275c with caller {caller:#07x}"
                            ))
                        }
                    };
                    Task::Flow(Flow::RankingMarked { ret })
                }
                (1, 0x40dd4) => Task::Flow(Flow::BonusJapanPalette {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x40dfc) => Task::Flow(Flow::BonusJapanBlinkDraw {
                    copy: BonusCopy::Japan,
                    d1: rw(usp + 6),
                }),
                (1, 0x40e0e) => Task::Flow(Flow::BonusJapanBlinkErase {
                    copy: BonusCopy::Japan,
                    d1: rw(usp + 6),
                }),
                (1, 0x40e56) => Task::Flow(Flow::BonusJapanMeterStart {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x40f14) => {
                    let cont = rl(usp + 68);
                    let d0 = if cont == 0x40ffe { rw(usp + 72) } else { 0 };
                    Task::Flow(Flow::BonusJapanMeter {
                        copy: BonusCopy::Japan,
                        cont,
                        d0,
                    })
                }
                (1, 0x41078) => Task::Flow(Flow::BonusJapanWindup {
                    copy: BonusCopy::Japan,
                    a0: rl(usp + 32),
                }),
                (1, 0x41158) => Task::Flow(Flow::BonusJapanBreak {
                    copy: BonusCopy::Japan,
                    a1: rl(usp + 36),
                    d2: rw(usp + 10),
                }),
                (1, 0x41166) => Task::Flow(Flow::BonusJapanSuccessPose {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x41184) => Task::Flow(Flow::BonusJapanCheerStart {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x411a6) => Task::Flow(Flow::BonusJapanCheerA {
                    copy: BonusCopy::Japan,
                    d2: rw(usp + 10),
                }),
                (1, 0x411ba) => Task::Flow(Flow::BonusJapanCheerB {
                    copy: BonusCopy::Japan,
                    d2: rw(usp + 10),
                }),
                (1, 0x4120c) => Task::Flow(Flow::BonusJapanHold1 {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x41212) => Task::Flow(Flow::BonusJapanHold2 {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x4123a) => Task::Flow(Flow::BonusJapanFailPose {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x41258) => Task::Flow(Flow::BonusJapanFailLaugh1 {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x41276) => Task::Flow(Flow::BonusJapanFailLaugh2 {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x41286) => Task::Flow(Flow::BonusJapanFailLaugh3 {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x41296) => Task::Flow(Flow::BonusJapanFailEnd {
                    copy: BonusCopy::Japan,
                }),
                (1, 0x41aa6) => Task::Flow(Flow::BonusJapanPalette {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41ace) => Task::Flow(Flow::BonusJapanBlinkDraw {
                    copy: BonusCopy::Stage2,
                    d1: rw(usp + 6),
                }),
                (1, 0x41ae0) => Task::Flow(Flow::BonusJapanBlinkErase {
                    copy: BonusCopy::Stage2,
                    d1: rw(usp + 6),
                }),
                (1, 0x41b28) => Task::Flow(Flow::BonusJapanMeterStart {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41be6) => {
                    let cont = rl(usp + 68);
                    let d0 = if cont == 0x41cd0 { rw(usp + 72) } else { 0 };
                    Task::Flow(Flow::BonusJapanMeter {
                        copy: BonusCopy::Stage2,
                        cont,
                        d0,
                    })
                }
                (1, 0x41d4e) => Task::Flow(Flow::BonusJapanWindup {
                    copy: BonusCopy::Stage2,
                    a0: rl(usp + 32),
                }),
                (1, 0x41e0c) => Task::Flow(Flow::BonusJapanBreak {
                    copy: BonusCopy::Stage2,
                    a1: rl(usp + 36),
                    d2: rw(usp + 10),
                }),
                (1, 0x41e1a) => Task::Flow(Flow::BonusJapanSuccessPose {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41e38) => Task::Flow(Flow::BonusJapanCheerStart {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41e5a) => Task::Flow(Flow::BonusJapanCheerA {
                    copy: BonusCopy::Stage2,
                    d2: rw(usp + 10),
                }),
                (1, 0x41e6e) => Task::Flow(Flow::BonusJapanCheerB {
                    copy: BonusCopy::Stage2,
                    d2: rw(usp + 10),
                }),
                (1, 0x41ec0) => Task::Flow(Flow::BonusJapanHold1 {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41ec6) => Task::Flow(Flow::BonusJapanHold2 {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41eee) => Task::Flow(Flow::BonusJapanFailPose {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41f0c) => Task::Flow(Flow::BonusJapanFailLaugh1 {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41f2a) => Task::Flow(Flow::BonusJapanFailLaugh2 {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41f3a) => Task::Flow(Flow::BonusJapanFailLaugh3 {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41f4a) => Task::Flow(Flow::BonusJapanFailEnd {
                    copy: BonusCopy::Stage2,
                }),
                (1, 0x41814) => Task::Flow(Flow::BonusStage1Palette1 {
                    copy: Bonus10Copy::Stage1,
                }),
                (1, 0x4182e) => Task::Flow(Flow::BonusStage1Palette2 {
                    copy: Bonus10Copy::Stage1,
                }),
                (1, 0x4185c) => Task::Flow(Flow::BonusStage1BlinkDraw {
                    copy: Bonus10Copy::Stage1,
                    d1: rw(usp + 6),
                }),
                (1, 0x4186e) => Task::Flow(Flow::BonusStage1BlinkErase {
                    copy: Bonus10Copy::Stage1,
                    d1: rw(usp + 6),
                }),
                (1, 0x418ac) => Task::Flow(Flow::BonusStage1Start {
                    copy: Bonus10Copy::Stage1,
                }),
                (1, 0x4191c) => Task::Flow(Flow::BonusStage1Loop {
                    copy: Bonus10Copy::Stage1,
                }),
                (1, 0x41962) => Task::Flow(Flow::BonusStage1Settle {
                    copy: Bonus10Copy::Stage1,
                }),
                (1, 0x419a0) => Task::Flow(Flow::BonusStage1After {
                    copy: Bonus10Copy::Stage1,
                    left: rw(usp + 68),
                }),
                (1, 0x419b4) => Task::Flow(Flow::BonusStage1Hold1 {
                    copy: Bonus10Copy::Stage1,
                }),
                (1, 0x419ee) => Task::Flow(Flow::BonusStage1Hold2 {
                    copy: Bonus10Copy::Stage1,
                }),
                (1, 0x420cc) => Task::Flow(Flow::BonusStage1Palette1 {
                    copy: Bonus10Copy::Stage3,
                }),
                (1, 0x420e6) => Task::Flow(Flow::BonusStage1Palette2 {
                    copy: Bonus10Copy::Stage3,
                }),
                (1, 0x42114) => Task::Flow(Flow::BonusStage1BlinkDraw {
                    copy: Bonus10Copy::Stage3,
                    d1: rw(usp + 6),
                }),
                (1, 0x42126) => Task::Flow(Flow::BonusStage1BlinkErase {
                    copy: Bonus10Copy::Stage3,
                    d1: rw(usp + 6),
                }),
                (1, 0x42164) => Task::Flow(Flow::BonusStage1Start {
                    copy: Bonus10Copy::Stage3,
                }),
                (1, 0x421d4) => Task::Flow(Flow::BonusStage1Loop {
                    copy: Bonus10Copy::Stage3,
                }),
                (1, 0x4221a) => Task::Flow(Flow::BonusStage1Settle {
                    copy: Bonus10Copy::Stage3,
                }),
                (1, 0x42258) => Task::Flow(Flow::BonusStage1After {
                    copy: Bonus10Copy::Stage3,
                    left: rw(usp + 68),
                }),
                (1, 0x4226c) => Task::Flow(Flow::BonusStage1Hold1 {
                    copy: Bonus10Copy::Stage3,
                }),
                (1, 0x422a6) => Task::Flow(Flow::BonusStage1Hold2 {
                    copy: Bonus10Copy::Stage3,
                }),
                (1, 0x44a60) => Task::Flow(Flow::EndingStart),
                (1, 0x44ae8) => Task::Flow(Flow::EndingSceneHold {
                    d1: rw(usp + 6),
                    d4: rw(usp + 18),
                    d5: rw(usp + 22),
                }),
                (1, 0x44b20) => Task::Flow(Flow::EndingMessageSetup),
                (1, 0x44b78) => Task::Flow(Flow::EndingMessageHold),
                (1, 0x44b88) => Task::Flow(Flow::EndingStaffSetup),
                (1, 0x44ba8) => Task::Flow(Flow::EndingStaffHeader),
                (1, 0x44bc8) => Task::Flow(Flow::EndingStaffStart),
                (1, 0x44bda) => Task::Flow(Flow::EndingStaffHold { d1: rw(usp + 6) }),
                (1, 0x44c04) => Task::Flow(Flow::EndingFinalHold),
                (1, 0x3ccec) => Task::Flow(Flow::AttractStart),
                (1, 0x3cd50) => {
                    let (d1, d2, d3, a2) = (rw(usp + 6), rw(usp + 10), rw(usp + 14), rl(usp + 40));
                    if d1 >= 10 || d1.wrapping_add(d2) != 9 || d3 != (d1 << 3) || a2 != 0x3cde0 {
                        return Err(format!(
                            "task slot 1 at 0x3cd50 has invalid loop registers D1={d1} D2={d2} D3={d3:#x} A2={a2:#07x}"
                        ));
                    }
                    Task::Flow(Flow::AttractScroll { d1 })
                }
                (1, 0x3cd86) => {
                    let (d1, d2, d3, a2) = (rw(usp + 6), rw(usp + 10), rw(usp + 14), rl(usp + 40));
                    if d1 >= 3 || d1.wrapping_add(d2) != 2 || d3 != (d1 << 3) || a2 != 0x3cdc8 {
                        return Err(format!(
                            "task slot 1 at 0x3cd86 has invalid loop registers D1={d1} D2={d2} D3={d3:#x} A2={a2:#07x}"
                        ));
                    }
                    Task::Flow(Flow::AttractTitleBlock { d1 })
                }
                (1, 0x3cda8) => Task::Flow(Flow::AttractLogoHold),
                (1, 0x3b588) => {
                    let caller = rl(usp + 68);
                    if caller != 0x3cdb8 {
                        return Err(format!(
                            "task slot 1 sleeps in 0x3b582/0x3b588 with caller {caller:#07x}, not 0x3cdb8"
                        ));
                    }
                    Task::Flow(Flow::AttractWait300 {
                        second_sleep: false,
                    })
                }
                (1, 0x3cdb8) => Task::Flow(Flow::AttractWait300 { second_sleep: true }),
                (1, 0x42736) => {
                    let caller = rl(usp + 68);
                    if caller != 0x3ce54 {
                        return Err(format!(
                            "task slot 1 sleeps in 0x4268c/0x42736 with caller {caller:#07x}, not 0x3ce54"
                        ));
                    }
                    Task::Flow(Flow::AttractRanking)
                }
                (1, 0x3dfce) => Task::Flow(Flow::StartScreen),
                (1, 0x3e0e6) => Task::Flow(Flow::CountrySelectArm),
                (1, 0x3e110) => Task::Flow(Flow::CountrySelectLoop { first: true }),
                (1, 0x3e13a) => Task::Flow(Flow::CountrySelectLoop { first: false }),
                (1, 0x3e252) => Task::Flow(Flow::CountrySelect4Arm),
                (1, 0x3e27c) => Task::Flow(Flow::CountrySelect4Loop {
                    first: true,
                    d6: 0,
                    d7: 0,
                }),
                (1, 0x3e32c) => Task::Flow(Flow::CountrySelect4Loop {
                    first: false,
                    d6: rw(usp + 26),
                    d7: rw(usp + 30),
                }),
                (1, 0x3f04c) => Task::Flow(Flow::ContinuePalette1),
                (1, 0x3f066) => Task::Flow(Flow::ContinuePalette2),
                (1, 0x3f162) => Task::Flow(Flow::ContinueLoop),
                (1, 0x3f172) => Task::Flow(Flow::ContinueAccept),
                (1, 0x436e6) => {
                    let caller = rl(usp + 68);
                    match caller {
                        0x3e5d6 => Task::Flow(Flow::DemoCountryPalette),
                        0x3eeec => Task::Flow(Flow::VersusCarryCountryPalette),
                        0x3f1b2 => Task::Flow(Flow::ContinueCountryPalette),
                        _ => {
                            return Err(format!(
                                "task slot 1 sleeps in 0x436b8/0x436e6 with caller {caller:#07x}"
                            ))
                        }
                    }
                }
                (1, 0x43776) => {
                    let caller = rl(usp + 68);
                    match caller {
                        0x3e704 => Task::Flow(Flow::DemoStagePalette),
                        0x3eee6 => Task::Flow(Flow::VersusCarryStagePalette),
                        0x3f1b8 => Task::Flow(Flow::ContinueStagePalette),
                        _ => {
                            return Err(format!(
                                "task slot 1 sleeps in 0x43748/0x43776 with caller {caller:#07x}"
                            ))
                        }
                    }
                }
                (1, 0x3f26a) => Task::Flow(Flow::GameOverAwaitEvent),
                (2, 0x3f7d6) => Task::Screen(Screen::Poll),
                (2, 0x3f87c) => Task::Screen(Screen::TravelMapSpawn { a1: rl(usp + 36) }),
                (2, 0x3f890) => Task::Screen(Screen::TravelMapLoop),
                (2, 0x3f8a0) => Task::Screen(Screen::TravelMapEnd),
                (2, 0x436e6) => {
                    let caller = rl(usp + 68);
                    if caller != 0x3f93c {
                        return Err(format!(
                            "task slot 2 sleeps in 0x436b8/0x436e6 with caller {caller:#07x}, not 0x3f93c"
                        ));
                    }
                    Task::Screen(Screen::CountryIntro {
                        a1: rl(usp + 36),
                        stage: 0,
                    })
                }
                (2, 0x3f95e) => Task::Screen(Screen::CountryIntro {
                    a1: rl(usp + 36),
                    stage: 1,
                }),
                (2, 0x3f964) => Task::Screen(Screen::CountryIntro {
                    a1: rl(usp + 36),
                    stage: 2,
                }),
                (2, 0x3fa46) => Task::Screen(Screen::VersusIntro { stage: 0 }),
                (2, 0x3fa4c) => Task::Screen(Screen::VersusIntro { stage: 1 }),
                (2, 0x43776) => match rl(usp + 68) {
                    0x3fa52 => Task::Screen(Screen::VersusIntro { stage: 2 }),
                    0x3fafa | 0x3fb28 => Task::Screen(Screen::OpponentIntro { stage: 0 }),
                    caller => {
                        return Err(format!(
                            "task slot 2 sleeps in 0x43748/0x43776 with caller {caller:#07x}"
                        ))
                    }
                },
                (2, 0x3fb2e) => Task::Screen(Screen::OpponentIntro { stage: 1 }),
                (2, 0x3fb34) => Task::Screen(Screen::OpponentIntro { stage: 2 }),
                (2, 0x3fbc2) => Task::Screen(Screen::AnnounceCleared),
                (2, 0x3fbf8) => Task::Screen(Screen::AnnounceHeld),
                (2, 0x3fc28) => Task::Screen(Screen::Wipe {
                    column: rw(usp + 6),
                    d2: rw(usp + 10),
                    tile: 0x103c,
                    announce: true,
                }),
                (2, 0x3fc48) => Task::Screen(Screen::AnnounceSound { last: false }),
                (2, 0x3fc58) => Task::Screen(Screen::AnnounceSound { last: true }),
                (2, 0x3ffae) => Task::Screen(Screen::GameOverHeld),
                (3, 0x3f3a2) => {
                    let mode_demo = work[(MODE_DEMO - 0xff8000) as usize];
                    match mode_demo {
                        0 => {
                            let cont = rl(WATCH_NEXT);
                            if !matches!(cont, 0x3f3be | 0x3f3d8 | 0x3f402) {
                                return Err(format!(
                                    "task slot 3 attract continuation {cont:#07x} at 0xff8f20, not translated"
                                ));
                            }
                        }
                        0xff => {
                            let cont = rl(WATCH_STATE);
                            if !matches!(cont, 0x3f41e | 0x3f454 | 0x3f47e | 0x3f4c6) {
                                return Err(format!(
                                    "task slot 3 watch continuation {cont:#07x} at 0xff8f24, not translated"
                                ));
                            }
                        }
                        _ => {}
                    }
                    Task::Watch {
                        d2: rw(usp + 8 + 2),
                    }
                }
                (7, 0x48270) => Task::Random(Random::Pass),
                (7, 0x4819e) => Task::Random(Random::Coins),
                (7, 0x481e8) => Task::Random(Random::ServiceDebounce),
                (7, 0x48212) => Task::Random(Random::ServiceCooldown),
                (7, 0x4824e) => Task::Random(Random::AttractStarted),
                (7, 0x48344) => {
                    let caller = rl(usp + 68);
                    let (d1, d2, a1) = (rw(usp + 6), rw(usp + 10), rl(usp + 36));
                    let expected_d2 = match d1 {
                        3 => 0x0001,
                        2 => 0x0002,
                        1 => 0x0100,
                        0 => 0x0200,
                        _ => 0,
                    };
                    if caller != 0x48262
                        || d1 > 3
                        || d2 != expected_d2
                        || a1 != COIN_SLOTS[(3 - d1) as usize]
                    {
                        return Err(format!(
                            "task slot 7 at 0x48344 has invalid pulse frame caller={caller:#07x} D1={d1} D2={d2:#06x} A1={a1:#08x}"
                        ));
                    }
                    Task::Random(Random::Pulse { d1, d2, a1 })
                }
                _ => {
                    return Err(format!(
                        "task slot {k} resumes at {resume:#07x}, not translated"
                    ))
                }
            };
            tasks[k as usize] = Some(task);
        }
        Ok(Scheduler { tasks })
    }

    /// Whether the game-flow task (slot 1) has just ended a match and waits
    /// on its wipe. In versus: before `0x3ee7a` / `0x3eeac` carry the winner
    /// on against the CPU, or before the game over after a match with no
    /// winner (`0x3ee4e`). In a one-player game: after the player's
    /// match-won screen, or the CPU's (`0x3ef08` / `0x3ef3a`), before the
    /// next opponent or the continue. Read-only, for the front end's stop.
    pub fn match_over(&self) -> bool {
        matches!(
            self.tasks[1],
            Some(Task::Flow(
                Flow::AwaitMatchWipe { .. } | Flow::AwaitCpuMatchWipe | Flow::AwaitGameOverWipe
            ))
        )
    }

    /// Replace the finished World Tour finale's wipe with the native ending.
    pub(crate) fn start_world_tour_ending(&mut self, bus: &mut Bus) {
        self.kill(bus, 2);
        bus.wb(SCENE_MODE, 0);
        clear_objects(bus);
        let mut at = Flow::EndingMessageSetup;
        let Yield::Sleep(delay) = ending_message_start(bus, &mut at) else {
            unreachable!("ending message setup always sleeps");
        };
        self.tasks[1] = Some(Task::Flow(at));
        let slot = slot_address(1);
        bus.wb(slot, 8);
        bus.wb(slot + 1, delay);
    }

    /// The occupied slots and the original resume address each task's
    /// translation continues from (for tooling and tests).
    pub fn positions(&self) -> Vec<(u32, u32)> {
        self.tasks
            .iter()
            .enumerate()
            .filter_map(|(k, t)| {
                let resume = match t.as_ref()? {
                    Task::New { entry } => *entry,
                    Task::Boot(Boot::Entry) => 0x3c0c4,
                    Task::Boot(Boot::Upload(0)) => 0x3c0ce,
                    Task::Boot(Boot::Upload(_)) => 0x3b634,
                    Task::Boot(Boot::SettleOverrun) => 0x3c204,
                    Task::Boot(Boot::Warning) => 0x3c210,
                    Task::Boot(Boot::WarningTyping { .. }) => 0x3bc10,
                    Task::Boot(Boot::WarningPause { .. }) => 0x3c23e,
                    Task::Boot(Boot::WarningHold) => 0x3c25a,
                    Task::Timer => 0x3f7ac,
                    Task::Flow(Flow::RoundStart) => 0x3ea3e,
                    Task::Flow(Flow::FightLoop) => 0x3eafa,
                    Task::Flow(Flow::KoStart) => 0x3eb28,
                    Task::Flow(Flow::KoSettle) => 0x3eb62,
                    Task::Flow(Flow::Victory) => 0x3ec0e,
                    Task::Flow(Flow::AwaitResult) => 0x3ec2a,
                    Task::Flow(
                        Flow::AwaitWipe
                        | Flow::AwaitAnnounce { .. }
                        | Flow::AwaitWinScreen { .. }
                        | Flow::AwaitMatchWipe { .. }
                        | Flow::AwaitCpuWinScreen
                        | Flow::AwaitCpuMatchWipe
                        | Flow::AwaitCountryScreen
                        | Flow::AwaitOpponentIntro
                        | Flow::AwaitDraw
                        | Flow::AwaitGameOverWipe
                        | Flow::ChallengerWipe,
                    ) => 0x422e0,
                    Task::Flow(Flow::ChallengerHold1) => 0x423de,
                    Task::Flow(Flow::ChallengerTyping { .. }) => 0x3bc10,
                    Task::Flow(Flow::ChallengerHold2) => 0x423f4,
                    Task::Flow(Flow::NameEntryPalette { .. }) => 0x42552,
                    Task::Flow(Flow::NameEntryLoop { .. }) => 0x42584,
                    Task::Flow(Flow::RankingMarked { .. }) => 0x4275c,
                    Task::Flow(Flow::BonusJapanPalette { copy }) => 0x40dd4 + copy.meter_shift(),
                    Task::Flow(Flow::BonusJapanBlinkDraw { copy, .. }) => {
                        0x40dfc + copy.meter_shift()
                    }
                    Task::Flow(Flow::BonusJapanBlinkErase { copy, .. }) => {
                        0x40e0e + copy.meter_shift()
                    }
                    Task::Flow(Flow::BonusJapanMeterStart { copy }) => 0x40e56 + copy.meter_shift(),
                    Task::Flow(Flow::BonusJapanMeter { copy, .. }) => 0x40f14 + copy.meter_shift(),
                    Task::Flow(Flow::BonusJapanWindup { copy, .. }) => match copy {
                        BonusCopy::Japan => 0x41078,
                        BonusCopy::Stage2 => 0x41d4e,
                    },
                    Task::Flow(Flow::BonusJapanBreak { copy, .. }) => match copy {
                        BonusCopy::Japan => 0x41158,
                        BonusCopy::Stage2 => 0x41e0c,
                    },
                    Task::Flow(Flow::BonusJapanSuccessPose { copy }) => match copy {
                        BonusCopy::Japan => 0x41166,
                        BonusCopy::Stage2 => 0x41e1a,
                    },
                    Task::Flow(Flow::BonusJapanCheerStart { copy }) => match copy {
                        BonusCopy::Japan => 0x41184,
                        BonusCopy::Stage2 => 0x41e38,
                    },
                    Task::Flow(Flow::BonusJapanCheerA { copy, .. }) => match copy {
                        BonusCopy::Japan => 0x411a6,
                        BonusCopy::Stage2 => 0x41e5a,
                    },
                    Task::Flow(Flow::BonusJapanCheerB { copy, .. }) => match copy {
                        BonusCopy::Japan => 0x411ba,
                        BonusCopy::Stage2 => 0x41e6e,
                    },
                    Task::Flow(Flow::BonusJapanHold1 { copy }) => match copy {
                        BonusCopy::Japan => 0x4120c,
                        BonusCopy::Stage2 => 0x41ec0,
                    },
                    Task::Flow(Flow::BonusJapanHold2 { copy }) => match copy {
                        BonusCopy::Japan => 0x41212,
                        BonusCopy::Stage2 => 0x41ec6,
                    },
                    Task::Flow(Flow::BonusJapanFailPose { copy }) => match copy {
                        BonusCopy::Japan => 0x4123a,
                        BonusCopy::Stage2 => 0x41eee,
                    },
                    Task::Flow(Flow::BonusJapanFailLaugh1 { copy }) => match copy {
                        BonusCopy::Japan => 0x41258,
                        BonusCopy::Stage2 => 0x41f0c,
                    },
                    Task::Flow(Flow::BonusJapanFailLaugh2 { copy }) => match copy {
                        BonusCopy::Japan => 0x41276,
                        BonusCopy::Stage2 => 0x41f2a,
                    },
                    Task::Flow(Flow::BonusJapanFailLaugh3 { copy }) => match copy {
                        BonusCopy::Japan => 0x41286,
                        BonusCopy::Stage2 => 0x41f3a,
                    },
                    Task::Flow(Flow::BonusJapanFailEnd { copy }) => match copy {
                        BonusCopy::Japan => 0x41296,
                        BonusCopy::Stage2 => 0x41f4a,
                    },
                    Task::Flow(Flow::BonusStage1Palette1 { copy }) => 0x41814 + copy.shift(),
                    Task::Flow(Flow::BonusStage1Palette2 { copy }) => 0x4182e + copy.shift(),
                    Task::Flow(Flow::BonusStage1BlinkDraw { copy, .. }) => 0x4185c + copy.shift(),
                    Task::Flow(Flow::BonusStage1BlinkErase { copy, .. }) => 0x4186e + copy.shift(),
                    Task::Flow(Flow::BonusStage1Start { copy }) => 0x418ac + copy.shift(),
                    Task::Flow(Flow::BonusStage1Loop { copy }) => 0x4191c + copy.shift(),
                    Task::Flow(Flow::BonusStage1Settle { copy }) => 0x41962 + copy.shift(),
                    Task::Flow(Flow::BonusStage1After { copy, .. }) => 0x419a0 + copy.shift(),
                    Task::Flow(Flow::BonusStage1Hold1 { copy }) => 0x419b4 + copy.shift(),
                    Task::Flow(Flow::BonusStage1Hold2 { copy }) => 0x419ee + copy.shift(),
                    Task::Flow(Flow::EndingStart) => 0x44a60,
                    Task::Flow(Flow::EndingSceneHold { .. }) => 0x44ae8,
                    Task::Flow(Flow::EndingMessageSetup) => 0x44b20,
                    Task::Flow(Flow::EndingMessageTyping { .. }) => 0x3bc10,
                    Task::Flow(Flow::EndingMessageHold) => 0x44b78,
                    Task::Flow(Flow::EndingStaffSetup) => 0x44b88,
                    Task::Flow(Flow::EndingStaffHeader) => 0x44ba8,
                    Task::Flow(Flow::EndingStaffStart) => 0x44bc8,
                    Task::Flow(Flow::EndingStaffTyping { .. }) => 0x3bc10,
                    Task::Flow(Flow::EndingStaffHold { .. }) => 0x44bda,
                    Task::Flow(Flow::EndingFinalHold) => 0x44c04,
                    Task::Flow(Flow::VersusCarryStagePalette) => 0x3eee6,
                    Task::Flow(Flow::VersusCarryCountryPalette) => 0x3eeec,
                    Task::Flow(Flow::ContinuePalette1) => 0x3f04c,
                    Task::Flow(Flow::ContinuePalette2) => 0x3f066,
                    Task::Flow(Flow::ContinueLoop) => 0x3f162,
                    Task::Flow(Flow::ContinueAccept) => 0x3f172,
                    Task::Flow(Flow::ContinueCountryPalette) => 0x3f1b2,
                    Task::Flow(Flow::ContinueStagePalette) => 0x3f1b8,
                    Task::Flow(Flow::GameOverCycle { .. }) => 0x3f204,
                    Task::Flow(Flow::GameOverDrain) => 0x3f25a,
                    Task::Flow(Flow::GameOverAwaitEvent) => 0x3f26a,
                    Task::Flow(Flow::GameOverPalette) => 0x3f32c,
                    Task::Flow(Flow::GameOverHold) => 0x3f33c,
                    Task::Flow(Flow::StageStart) => 0x3e59c,
                    Task::Flow(Flow::DemoCountryPalette) => 0x436e6,
                    Task::Flow(Flow::DemoStagePalette) => 0x43776,
                    Task::Flow(Flow::TimeOverStart) => 0x3ed26,
                    Task::Flow(Flow::TimeOverHold { .. }) => 0x3ed96,
                    Task::Flow(Flow::DoubleKoSettle) => 0x3ecca,
                    Task::Flow(Flow::DoubleKoHold { .. }) => 0x3ed00,
                    Task::Flow(Flow::Intro) => 0x3ea30,
                    Task::Flow(Flow::FightShown { .. }) => 0x3ea0e,
                    Task::Flow(Flow::AttractJoin) => 0x3ccae,
                    Task::Flow(Flow::AttractStart) => 0x3ccec,
                    Task::Flow(Flow::AttractScroll { .. }) => 0x3cd50,
                    Task::Flow(Flow::AttractTitleBlock { .. }) => 0x3cd86,
                    Task::Flow(Flow::AttractLogoHold) => 0x3cda8,
                    Task::Flow(Flow::AttractWait300 {
                        second_sleep: false,
                    }) => 0x3b588,
                    Task::Flow(Flow::AttractWait300 { second_sleep: true }) => 0x3cdb8,
                    Task::Flow(Flow::AttractRanking) => 0x42736,
                    Task::Flow(Flow::StartScreenEntry) => 0x3dec2,
                    Task::Flow(Flow::StartScreen) => 0x3dfce,
                    Task::Flow(Flow::CountrySelectArm) => 0x3e0e6,
                    Task::Flow(Flow::CountrySelectLoop { first: true }) => 0x3e110,
                    Task::Flow(Flow::CountrySelectLoop { first: false }) => 0x3e13a,
                    Task::Flow(Flow::CountrySelect4Arm) => 0x3e252,
                    Task::Flow(Flow::CountrySelect4Loop { first: true, .. }) => 0x3e27c,
                    Task::Flow(Flow::CountrySelect4Loop { first: false, .. }) => 0x3e32c,
                    Task::Screen(Screen::Wipe { .. }) => 0x3fc28,
                    Task::Screen(Screen::AnnounceCleared) => 0x3fbc2,
                    Task::Screen(Screen::AnnounceHeld) => 0x3fbf8,
                    Task::Screen(Screen::AnnounceSound { .. }) => 0x3fc48,
                    Task::Screen(Screen::SlowMotion { first: true }) => 0x3ff3c,
                    Task::Screen(Screen::SlowMotion { first: false }) => 0x3ff42,
                    Task::Screen(Screen::Poll) => 0x3f7d6,
                    Task::Screen(Screen::ResultTyping { .. }) => 0x3bc10,
                    Task::Screen(Screen::ResultHeld { .. }) => 0x3fd46,
                    Task::Screen(Screen::ResultSettle) => 0x3fee0,
                    Task::Screen(Screen::ResultTyped { .. }) => 0x3fd94,
                    Task::Screen(Screen::TallyStart) => 0x3fdf8,
                    Task::Screen(Screen::TallyTime) => 0x3fe62,
                    Task::Screen(Screen::TallyVitality) => 0x3fece,
                    Task::Screen(Screen::WinText { .. }) => 0x40104,
                    Task::Screen(Screen::WinCard { held: false }) => 0x401e6,
                    Task::Screen(Screen::WinCard { held: true }) => 0x401ec,
                    Task::Screen(Screen::WinCardCpu) => 0x4029a,
                    Task::Screen(Screen::VersusWinCard { shown: false }) => 0x400c2,
                    Task::Screen(Screen::VersusWinCard { shown: true }) => 0x400d4,
                    Task::Screen(Screen::OpponentIntro { stage: 0 }) => 0x43776,
                    Task::Screen(Screen::OpponentIntro { stage: 1 }) => 0x3fb2e,
                    Task::Screen(Screen::OpponentIntro { .. }) => 0x3fb34,
                    Task::Screen(Screen::VersusIntro { stage: 0 }) => 0x3fa46,
                    Task::Screen(Screen::VersusIntro { stage: 1 }) => 0x3fa4c,
                    Task::Screen(Screen::VersusIntro { .. }) => 0x43776,
                    Task::Screen(Screen::TravelMapSpawn { .. }) => 0x3f87c,
                    Task::Screen(Screen::TravelMapLoop) => 0x3f890,
                    Task::Screen(Screen::TravelMapEnd) => 0x3f8a0,
                    Task::Screen(Screen::CountryIntro { stage: 0, .. }) => 0x436e6,
                    Task::Screen(Screen::CountryIntro { stage: 1, .. }) => 0x3f95e,
                    Task::Screen(Screen::CountryIntro { .. }) => 0x3f964,
                    Task::Screen(Screen::DrawTyping { .. }) => 0x3bc10,
                    Task::Screen(Screen::DrawHeld) => 0x3ff0e,
                    Task::Screen(Screen::GameOverPalette { .. }) => 0x3ff82,
                    Task::Screen(Screen::GameOverHeld) => 0x3ffae,
                    Task::Watch { .. } => 0x3f3a2,
                    Task::ServiceInput(ServiceInput::Entry { .. }) => 0x483f8,
                    Task::ServiceInput(ServiceInput::Upload { .. }) => 0x3b634,
                    Task::ServiceInput(ServiceInput::Loop { .. }) => 0x485ee,
                    Task::ServiceMenu(ServiceMenu::Entry) => 0x48644,
                    Task::ServiceMenu(ServiceMenu::BlinkErase { .. }) => 0x4871c,
                    Task::ServiceMenu(ServiceMenu::BlinkRedraw { .. }) => 0x48730,
                    Task::ServiceMenu(ServiceMenu::InItem { .. }) => 0x4877e,
                    Task::ServiceMenu(ServiceMenu::ReturnToMenu { .. }) => 0x487e4,
                    Task::ServiceItem(ServiceItem::CrosshatchEntry) => 0x487f8,
                    Task::ServiceItem(ServiceItem::CrosshatchLoop) => 0x48856,
                    Task::ServiceItem(ServiceItem::ColorBarEntry) => 0x488f8,
                    Task::ServiceItem(ServiceItem::ColorBarLoop) => 0x4898e,
                    Task::ServiceItem(ServiceItem::CharTest1Entry) => 0x48858,
                    Task::ServiceItem(ServiceItem::CharTest1Loop) => 0x488e6,
                    Task::ServiceItem(ServiceItem::CharRomEntry) => 0x48990,
                    Task::ServiceItem(ServiceItem::CharRomLoop) => 0x48a40,
                    Task::ServiceItem(ServiceItem::SpriteTestEntry) => 0x48b6a,
                    Task::ServiceItem(ServiceItem::SpriteTestLoop) => 0x48c1e,
                    Task::ServiceItem(ServiceItem::ScrollTestEntry) => 0x48da4,
                    Task::ServiceItem(ServiceItem::ScrollTestInitBg) => 0x49086,
                    Task::ServiceItem(ServiceItem::ScrollTestStart) => 0x490a2,
                    Task::ServiceItem(ServiceItem::ScrollTestLoop) => 0x48e96,
                    Task::ServiceItem(ServiceItem::ScrollTestBgOnly) => 0x49086,
                    Task::ServiceItem(ServiceItem::OutputTestEntry) => 0x490f8,
                    Task::ServiceItem(ServiceItem::OutputTestPalette2) => 0x49086,
                    Task::ServiceItem(ServiceItem::OutputTestStart) => 0x490a2,
                    Task::ServiceItem(ServiceItem::OutputTestApply { .. }) => 0x491d0,
                    Task::ServiceItem(ServiceItem::OutputTestReset { .. }) => 0x491e2,
                    Task::Random(Random::Pass) => 0x48270,
                    Task::Random(Random::Coins) => 0x4819e,
                    Task::Random(Random::ServiceDebounce) => 0x481e8,
                    Task::Random(Random::ServiceCooldown) => 0x48212,
                    Task::Random(Random::AttractStarted) => 0x4824e,
                    Task::Random(Random::Pulse { .. }) => 0x48344,
                };
                Some((k as u32, resume))
            })
            .collect()
    }

    /// Apply the result of running slot `k`: a sleep stores state 8 and the
    /// sleep count into the current slot's header (0x3b24e..0x3b286) and
    /// returns `true` so the caller rescans from slot 0 (0x3b19a); a task
    /// exit (trap #3, 0x3b29a..0x3b2b2) clears the current slot's state byte
    /// (+0) and rescans from slot 0 (0x3b19a); an overrun leaves the slot in
    /// state 4 across the next vblank; a halt clears the translated
    /// continuation.
    fn apply_yield(&mut self, bus: &mut Bus, k: usize, y: Yield) -> bool {
        match y {
            Yield::Sleep(n) => {
                let a1 = current_slot_address(bus);
                bus.wb(a1, 8);
                bus.wb(a1 + 1, n);
                true
            }
            Yield::Exit => {
                let a1 = current_slot_address(bus);
                bus.wb(a1, 0);
                self.tasks[k] = None;
                true
            }
            Yield::Overrun => false,
            Yield::Halt => {
                self.tasks[k] = None;
                false
            }
        }
    }

    /// 0x3b19a..0x3b286: run the ready and new slots in slot order,
    /// rescanning from slot 0 (0x3b19a, which clears the vblank flag at
    /// 0x3b19e) after every sleep (trap #2, 0x3b286). If a task overran the
    /// previous frame's cycle budget (remaining in state 4 across vblank),
    /// vblank's `rte` returns into that running task first before its trap
    /// #2 jumps to 0x3b19a.
    pub fn run(&mut self, bus: &mut Bus, io: &Io) {
        let running = bus.rb(CURRENT_SLOT);
        let running_slot = slot_address(running);
        if (running as u32) < SLOT_COUNT && bus.rb(running_slot) == 4 {
            let Some(mut task) = self.tasks[running as usize] else {
                bus.note(0x3b1d6);
                return;
            };
            let y = task.run(self, bus, io);
            self.tasks[running as usize] = Some(task);
            if !self.apply_yield(bus, running as usize, y) {
                return;
            }
        }
        'rescan: loop {
            bus.wb(VBLANK_FLAG, 0);
            for k in 0..SLOT_COUNT {
                let slot = slot_address(k as u8);
                let d3 = bus.rb(slot);
                if d3 & 3 == 0 {
                    continue;
                }
                bus.wb(CURRENT_SLOT, k as u8);
                bus.wb(slot, 4);
                bus.wb(slot + 1, 0);
                if d3 & 1 != 0 {
                    let Some(Task::New { entry }) = self.tasks[k as usize] else {
                        bus.note(0x3b1f0);
                        self.tasks[k as usize] = None;
                        continue;
                    };
                    self.tasks[k as usize] = enter_new_task(k, entry, bus);
                    if self.tasks[k as usize].is_none() {
                        bus.wb(slot, 0);
                        continue;
                    }
                }
                let Some(mut task) = self.tasks[k as usize] else {
                    bus.note(0x3b1d6);
                    continue;
                };
                let y = task.run(self, bus, io);
                self.tasks[k as usize] = Some(task);
                if self.apply_yield(bus, k as usize, y) {
                    continue 'rescan;
                }
                if matches!(y, Yield::Overrun) {
                    return;
                }
            }
            break;
        }
    }
}

impl Task {
    fn run(&mut self, scheduler: &mut Scheduler, bus: &mut Bus, io: &Io) -> Yield {
        match self {
            Task::New { entry } => {
                bus.note(*entry);
                Yield::Halt
            }
            Task::Boot(at) => {
                let (next_task, y) = boot(scheduler, bus, io, at);
                if let Some(t) = next_task {
                    *self = t;
                }
                y
            }
            Task::ServiceInput(at) => service_input(scheduler, bus, io, at),
            Task::Timer => {
                round_timer(bus);
                Yield::Sleep(1)
            }
            Task::Flow(at) => flow(scheduler, bus, io, at),
            Task::ServiceMenu(at) => service_menu(scheduler, bus, io, at),
            Task::Screen(at) => screen(bus, io, at),
            Task::ServiceItem(at) => service_item(bus, io, at),
            Task::Watch { d2 } => watch(bus, io, d2),
            Task::Random(at) => random(scheduler, bus, io, at),
        }
    }
}

/// 68000 cycles from power-on (`t = 0`) through the reset path (`0x3b000..0x3b050`)
/// and the pending level-6 vblank interrupt's exception entry to the handler's
/// first fetch (`0x3b052`):
/// - reset vector fetch (`0x000000..0x000007`) to `0x3b000`: 24 + 2 = 26
/// - `0x3b000..0x3b01c` (register setup 48 + 6,144 x `clr.l (A0)+` 122,880 + `dbra` 61,444): 184,372
/// - `0x3b01e..0x3b028` (self-test DIP check): 30
/// - `0x3b030 -> 0x3b2e4` (`decode_coinage`): 176
/// - `0x3b034 -> 0x3b77a` (`boot_init`): 1,114
/// - `0x3b03a..0x3b048 -> 0x3b20a` (`spawn` slot 0): 488
/// - `0x3b04a: move #$2000, SR` (12) + `0x3b04e: bra.w $3b19a` (10) +
///   autovectored level-6 exception entry (44 + 3 E-clock sync): 69
///
/// Total: 186,275 cycles from power-on (186,272 before the 3-cycle E-clock
/// autovector alignment), holding frame 1's vblank across `1 / 60 s`
/// (`133,333 1/3` cycles) until `0x3b04a`. Adding vblank 1's 682 handler
/// cycles places frame 1's `0x27` latch write at 186,957 cycles from
/// power-on (`0.023369625 s`).
pub const RESET_TO_VBLANK1_CYCLES: u32 = 26 + 184_372 + 30 + 176 + 1_114 + 488 + 69;

/// 68000 cycles from power-on (`t = 0`) through the reset clear
/// (`0x3b000..0x3b01c`), the self-test DIP branch (`0x3b01e..0x3b02a`), and
/// the service RAM test (`0x4834e..0x483d4`, ticket #145, F73):
/// - reset vector fetch (`0x000000..0x000007`) to `0x3b000`: 24 + 2 = 26
/// - `0x3b000..0x3b01c` (register setup 48 + 6,144 x `clr.l (A0)+` 122,880 + `dbra` 61,444): 184,372
/// - `0x3b01e..0x3b02a` (self-test DIP check + `jmp $4834e.l`): 44
/// - `0x4834e..0x48356` (`gfx_off(0xff)`): 88
/// - `0x48358..0x483a8` (work+object RAM `0xff8000..0xffffff` `0x5555`/`0xaaaa` write+verify): 1,507,502
/// - `0x483aa..0x483b8 -> 0x4836a..0x483a8` (video RAM `0x800000..0x800fff` `0x5555`/`0xaaaa` write+verify): 188,590
/// - `0x483ba..0x483c4` (8,192 x `clr.l (A0)+` over `0xff8000..0xffffff`): 245,776
/// - `0x483c6..0x483d4` (2,048 x `move.w #$103c, (A0)+` over `0x800000..0x800fff`): 45,080
pub const SERVICE_RAM_TEST_END_CYCLES: u64 =
    26 + 184_372 + 44 + 88 + 1_507_502 + 188_590 + 245_776 + 45_080;

/// The 60 Hz frame (`133,333 1/3` cycles/frame at 8 MHz) in which the service
/// RAM test (`0x4834e..0x483d4`) finishes and `0x483d8..0x483f2` writes
/// `0xff8003 = 0x55`, spawns slot 0 at `0x483f8`, and unmasks interrupts:
/// `floor(2,171,478 / (400,000 / 3)) = 16`.
pub const SERVICE_RAM_TEST_DONE_FRAME: u32 = ((SERVICE_RAM_TEST_END_CYCLES * 3) / 400_000) as u32;

/// Number of frame ends (`1..=15`) that fall inside `0x4834e..0x483d4` after
/// frame 0's reset clear (`0x3b012..0x3b01a`).
pub const SERVICE_RAM_TEST_FRAMES: u32 = SERVICE_RAM_TEST_DONE_FRAME - 1;

/// Cycles from the start of frame `SERVICE_RAM_TEST_DONE_FRAME` (`16`) to the
/// held level-6 vblank handler's first instruction (`0x3b052`) after
/// `0x483d8..0x483f2` (`move.b #$55,($3,A5)` 16 + `spawn` 488 +
/// `move #$2000,SR` 12 + `jmp $3b19a.l` 12 + autovector entry 47 = 575 cycles).
pub const SERVICE_RAM_TEST_TO_VBLANK_CYCLES: u32 = ((SERVICE_RAM_TEST_END_CYCLES + 575)
    - (SERVICE_RAM_TEST_DONE_FRAME as u64 * 400_000 / 3))
    as u32;

/// 0x3b000..0x3b04e: the 68000 reset path.
///
/// The 6,144-longword clear of work RAM (0x3b012..0x3b01a: 184,398 cycles
/// from reset entry, over the 133,333 1/3-cycle frame at 8 MHz) runs with
/// interrupts masked (SR = 0x2700), holding frame 0's vblank interrupt
/// (`irq1_line_hold`) across the end of frame 0 into frame 1. In frame 1
/// the clear finishes, the self-test DIP is checked (0x3b01e..0x3b02a:
/// bit 15 low jumps to 0x4834e), the coinage DIPs are decoded (0x3b2e4),
/// the sound queue and layer control are initialised (0x3b77a), slot 0 is
/// spawned at 0x3c0c4 (0x3b20a), and `move #$2000, SR` (0x3b04a) releases
/// the held vblank before entering the scheduler loop (0x3b19a).
pub fn reset(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io) -> Option<LatchWrite> {
    bus.work.fill(0);
    if io.dsw1 & 0x8000 == 0 {
        ram_test_start(bus);
        return None;
    }
    decode_coinage(bus, io.dsw1);
    boot_init(bus);
    scheduler.spawn(bus, 0, 0x3c0c4);
    let mut latch = vblank(bus, io);
    latch.cycles += RESET_TO_VBLANK1_CYCLES;
    scheduler.run(bus, io);
    Some(latch)
}

/// 0x4834e..0x48356: start the service RAM test in frame 1 after the reset
/// clear (`0x3b012..0x3b01a`) and self-test DIP branch (`0x3b01e..0x3b02a`):
/// `gfx_off(0xff)` disables all video layers while interrupts remain masked
/// (`SR = 0x2700`) across frames `1..=15` (`SERVICE_RAM_TEST_FRAMES`).
pub fn ram_test_start(bus: &mut Bus) {
    gfx_off(bus, 0xff);
}

/// 0x483ba..0x483f2: finish the service RAM test in frame
/// `SERVICE_RAM_TEST_DONE_FRAME` (`16`): clear `0xff8000..0xffffff` (`0x483ba`),
/// fill video RAM `0x800000..0x800fff` with `0x103c` (`0x483c6`), write `0x55`
/// to `0xff8003` (`MODE_DEMO`, `0x483d8`), spawn slot 0 at `0x483f8`
/// (`0x483de..0x483e8`), take the held vblank interrupt (`0x483ee`), and run
/// the scheduler (`0x483f2`).
pub fn ram_test_finish(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io) -> LatchWrite {
    bus.work.fill(0);
    bus.obj.fill(0);
    clear_text_layer(bus);
    bus.wb(MODE_DEMO, 0x55);
    scheduler.spawn(bus, 0, 0x483f8);
    let mut latch = vblank(bus, io);
    latch.cycles += SERVICE_RAM_TEST_TO_VBLANK_CYCLES;
    scheduler.run(bus, io);
    latch
}

/// 0x3b5c8..0x3b5f6: initialize a service-mode screen: `boot_init` (`0x3b77a`),
/// `clear_text_layer` (`0x3bdb0`), `gfx_off(0xff)` (`0x3b690`),
/// `coin_ctrl_off(0xffff)` (`0x3b6a2`), `aux_ctrl_off(0x00ff)` (`0x3b6b4`),
/// and `clr.w ($14,A6)` (`*bus.fg_scroll = 0`).
fn service_screen_init(bus: &mut Bus) {
    boot_init(bus);
    clear_text_layer(bus);
    gfx_off(bus, 0xff);
    coin_ctrl_off(bus, 0xffff);
    aux_ctrl_off(bus, 0x00ff);
    *bus.fg_scroll = 0;
    *bus.fg_page = 0;
}

/// 0x3b664..0x3b672: set `mask` bits in `AUX_CONTROL` (`0xff8686`) and write
/// the auxiliary control port (`0xc00012`).
fn aux_ctrl_or(bus: &mut Bus, mask: u16) {
    let v = bus.rw(AUX_CONTROL) | mask;
    bus.ww(AUX_CONTROL, v);
}

/// 0x3b6b4..0x3b6c4: clear `mask` bits in `AUX_CONTROL` (`0xff8686`) and write
/// the auxiliary control port (`0xc00012`).
fn aux_ctrl_off(bus: &mut Bus, mask: u16) {
    let v = bus.rw(AUX_CONTROL) & !mask;
    bus.ww(AUX_CONTROL, v);
}

/// 0x3bffc..0x3c012: write four hex-digit cells for word `val` (`D0`) with
/// attribute `attr` (`D1`) at `addr..addr + 8` (`A0`).
fn write_hex_word(bus: &mut Bus, addr: u32, val: u16, attr: u16) {
    write_hex_byte(bus, addr, (val >> 8) as u8, attr);
    write_hex_byte(bus, addr + 4, val as u8, attr);
}

/// 0x3c014..0x3c032: write two hex-digit cells (`0x00..=0x0f | attr`) for byte
/// `val` (`D0`) with attribute `attr` (`D1`) at `addr` and `addr + 2` (`A0`).
fn write_hex_byte(bus: &mut Bus, addr: u32, val: u8, attr: u16) {
    let hi = (((val >> 4) & 0x0f) as u16) | attr;
    let lo = ((val & 0x0f) as u16) | attr;
    bus.ww(addr, hi);
    bus.ww(addr + 2, lo);
}

/// Slot 0's service-mode input, DIP and sound test (0x483f8..0x48642;
/// ticket #145, F73).
fn service_input(
    scheduler: &mut Scheduler,
    bus: &mut Bus,
    io: &Io,
    at: &mut ServiceInput,
) -> Yield {
    match *at {
        ServiceInput::Entry { d7 } => {
            // 0x483f8..0x4840c: lea ($2a2,A5),A7; jsr $3b5c8.l; upload 0x1ae68
            // at 0x0000 and sleep 1 frame at 0x3b62e.
            service_screen_init(bus);
            let (src, off) = BOOT_PALETTES[0];
            upload_palette(bus, src, off);
            *at = ServiceInput::Upload { step: 1, d7 };
            Yield::Sleep(1)
        }
        ServiceInput::Upload { step, d7 } => {
            if io.system & 0x40 == 0 {
                bus.note(0x3b602);
                return Yield::Halt;
            }
            if (step as usize) < BOOT_PALETTES.len() {
                let (src, off) = BOOT_PALETTES[step as usize];
                upload_palette(bus, src, off);
                *at = ServiceInput::Upload { step: step + 1, d7 };
                return Yield::Sleep(1);
            }
            // 0x48442..0x48454: draw the 6 text blocks in 0x486c0..0x486d8.
            for i in 0..6u32 {
                text_block(bus, bus.rl(0x486c0 + 4 * i));
            }
            // 0x48456..0x48478: check D7 RAM-test error words (low word = video
            // RAM at 0x8006be, high word = work RAM at 0x8005be).
            if (d7 as u16) == 1 {
                bus.wl(0x8006be, 0x0017_0010);
            }
            if (d7 >> 16) as u16 == 1 {
                bus.wl(0x8005be, 0x0017_0010);
            }
            // 0x4847a..0x4848c: enable tx (0x08) and pulse bit 0.
            gfx_on(bus, 9);
            gfx_off(bus, 1);
            // 0x4848e..0x484a4: initialize dynamic 8x17 text block header at
            // 0xff872c..0xff8731 and clear sound-selector frame counter 0xff8848.
            bus.ww(0xff872c, 0x0936);
            bus.ww(0xff872e, 0x0811);
            bus.ww(0xff8730, 0x9000);
            bus.wb(0xff8848, 0);
            // 0x484a6..0x484c0: draw initial `00` for the 4 latched punch/kick
            // bytes at the video-RAM offsets in 0x49a8e.
            for i in 0..4u32 {
                let off = bus.rw(0x49a8e + 2 * i) as u32;
                write_hex_byte(bus, 0x800000 + off, 0, 0x5000);
            }
            // 0x484c2..0x484d2: reset button latches and enable P1/P2 vblank
            // button latching (0xffc002 = 1, 0xffc008 = 0x80).
            reset_button_latches(bus);
            bus.wb(0xffc002, 1);
            bus.wb(0xffc008, 0x80);
            let mut d7_byte = 0u8;
            let y = service_input_loop(scheduler, bus, io, &mut d7_byte);
            *at = ServiceInput::Loop { d7: d7_byte };
            y
        }
        ServiceInput::Loop { mut d7 } => {
            let y = service_input_loop(scheduler, bus, io, &mut d7);
            *at = ServiceInput::Loop { d7 };
            y
        }
    }
}

/// Slot-1 entry address for the service-mode test menu (spawned at 0x48614;
/// translated in F74, #146).
const SERVICE_MENU_ENTRY: u32 = 0x48644;

/// 0x484d4..0x48642: one frame of the service input, DIP and sound-test loop.
fn service_input_loop(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, d7: &mut u8) -> Yield {
    // 0x484d4..0x48502: format the 8 words at 0xc00000..0xc0000f into the 8x17
    // text block at 0xff872c..0xff8841 (inverting all words except D2 == 5 and
    // D2 == 4, i.e. 0xc00004 and 0xc00006) and draw it via 0x3bba0.
    let mut a2 = 0xff8732u32;
    for idx in 0..8usize {
        let d2 = 7 - idx;
        let mut d0 = io.port_word(idx);
        if d2 != 5 && d2 != 4 {
            d0 = !d0;
        }
        for d1 in (1..=16u16).rev() {
            let bit = (d0 >> 15) & 1;
            d0 <<= 1;
            if d1 == 8 {
                bus.ww(a2, 0x003c);
                a2 += 2;
            }
            bus.ww(a2, bit);
            a2 += 2;
        }
    }
    text_block(bus, 0xff872c);

    // 0x48504..0x48548: latch nonzero punch/kick strengths from P1 (0xff86b0)
    // and P2 (0xff86da) into 0xff8862..0xff8865 and display them at the four
    // offsets in 0x49a8e.
    let mut a3 = 0xff8862u32;
    for p in 0..2u32 {
        let mut a2 = bus.rl(0x49a96 + 4 * p);
        for _ in 0..2 {
            let d0 = bus.rb(a2);
            if d0 != 0 && d0 != bus.rb(a3) {
                bus.wb(a3, d0);
            }
            a2 += 1;
            a3 += 1;
        }
    }
    for i in 0..4u32 {
        let d0 = bus.rb(0xff8862 + i);
        let d3 = bus.rw(0x49a8e + 2 * i) as u32;
        write_hex_byte(bus, 0x800000 + d3, d0, 0x5000);
    }

    // 0x4854a..0x4859c: every 5th frame (0xff8848 == 5), check P1 Up (bit 3) /
    // P1 Down (bit 2) with toggle bytes 0xff8846 / 0xff8847 and draw D7 at
    // 0x800652. When a toggle byte clears to 0, 0x48574 / 0x48588 branches
    // directly to 0x485c4 (skipping both the hex redraw at 0x4858e and the
    // punch trigger at 0x4859e).
    let tick = bus.rb(0xff8848).wrapping_add(1);
    bus.wb(0xff8848, tick);
    let mut skip_to_exit_check = false;
    if tick == 5 {
        bus.wb(0xff8848, 0);
        let d0 = (!io.in1) & 0x000f;
        if d0 == 8 {
            let v = bus.rb(0xff8846) ^ 0xff;
            bus.wb(0xff8846, v);
            if v == 0 {
                skip_to_exit_check = true;
            } else {
                *d7 = d7.wrapping_add(1);
            }
        }
        if !skip_to_exit_check && d0 == 4 {
            let v = bus.rb(0xff8847) ^ 0xff;
            bus.wb(0xff8847, v);
            if v == 0 {
                skip_to_exit_check = true;
            } else {
                *d7 = d7.wrapping_sub(1);
            }
        }
        if !skip_to_exit_check {
            write_hex_byte(bus, 0x800652, *d7, 0x9000);
        }
    }

    // 0x4859e..0x485c2: P1 medium/heavy punch release (0xff86b0 > 4) toggles
    // 0xff8849; on the 0xff half-cycle, queues `d7 & 0xbf` via 0x3b7c8.
    if !skip_to_exit_check && bus.rb(0xff86b0) > 4 {
        let v = bus.rb(0xff8849) ^ 0xff;
        bus.wb(0xff8849, v);
        if v != 0 {
            bus.queue_sound((*d7 & 0xbf) as u16);
        }
    }

    // 0x485c4..0x48618: exit to the service menu (slot 1) when P1 Up is held
    // (IN1 bit 3 low) and P1 medium/heavy kick was released (0xff86b1 > 4).
    if io.in1 & 0x0008 == 0 && bus.rb(0xff86b1) > 4 {
        bus.wb(0xffc002, 0);
        bus.wb(0xffc008, 0);
        bus.ww(0xff86b0, 0);
        bus.ww(0xff86da, 0);
        queue_boot_sounds(bus);
        scheduler.spawn(bus, 1, SERVICE_MENU_ENTRY);
        Yield::Exit
    } else {
        bus.ww(0xff86b0, 0);
        bus.ww(0xff86da, 0);
        Yield::Sleep(1)
    }
}

/// Slot 1's service-mode test menu (0x48644..0x487f6; ticket #146, F74).
fn service_menu(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut ServiceMenu) -> Yield {
    match *at {
        ServiceMenu::Entry => {
            // 0x48644..0x48676: reset button latches, enable P1/P2 vblank
            // button latching, erase the 5 input-screen text blocks at
            // 0x486c0..0x486d0, and clear D7.
            reset_button_latches(bus);
            bus.wb(0xffc002, 1);
            bus.wb(0xffc008, 0x80);
            bus.ww(0xff86b0, 0);
            bus.ww(0xff86da, 0);
            for i in 0..5u32 {
                erase_block(bus, bus.rl(0x486c0 + 4 * i));
            }
            let mut d7 = 0u8;
            service_menu_draw(bus);
            service_menu_step_cursor(bus, io, &mut d7);
            *at = ServiceMenu::BlinkErase { d7 };
            Yield::Sleep(6)
        }
        ServiceMenu::ReturnToMenu { mut d7 } => {
            // 0x487e4 -> 0x48678: redraw the menu while keeping D7 on the item
            // just exited (`0x48678` skips `0x48676: clr.w D7`).
            service_menu_draw(bus);
            service_menu_step_cursor(bus, io, &mut d7);
            *at = ServiceMenu::BlinkErase { d7 };
            Yield::Sleep(6)
        }
        ServiceMenu::BlinkErase { d7 } => {
            // 0x4871c..0x4872e: erase the highlighted item `d7` (`0x486dc + 4 * d7`)
            // and sleep 6 frames (`0x3b532`).
            erase_block(bus, bus.rl(0x486dc + 4 * d7 as u32));
            *at = ServiceMenu::BlinkRedraw { d7 };
            Yield::Sleep(6)
        }
        ServiceMenu::BlinkRedraw { mut d7 } => {
            // 0x48730..0x4877c: if a P1 punch was released (`0xff86b0 > 4`),
            // spawn slot 2 at `0x4874e + 4 * d7` and enter the item wait loop
            // at 0x4877e; otherwise redraw item `d7` and step the cursor at
            // 0x48692.
            if bus.rb(0xff86b0) > 4 {
                let item_entry = bus.rl(0x4874e + 4 * d7 as u32);
                scheduler.spawn(bus, 2, item_entry);
                *at = ServiceMenu::InItem { d7 };
                Yield::Sleep(1)
            } else {
                text_block(bus, bus.rl(0x486dc + 4 * d7 as u32));
                service_menu_step_cursor(bus, io, &mut d7);
                *at = ServiceMenu::BlinkErase { d7 };
                Yield::Sleep(6)
            }
        }
        ServiceMenu::InItem { d7 } => {
            // 0x4877e..0x487f6: wait inside item `d7` until P1 Up is held
            // (`IN1` bit 3 low) and a P1 kick is released (`0xff86b1 > 4`).
            if io.in1 & 0x0008 == 0 && bus.rb(0xff86b1) > 4 {
                scheduler.kill(bus, 2);
                service_screen_init(bus);
                *bus.gfxctrl = 8;
                bus.wb(0xff8682, 8);
                if bus.rl(0xff8634) == 0x1be68 {
                    bus.wl(0xff8634, 0x1bc68);
                    bus.ww(0xff8638, 0x0600);
                    bus.wb(PALETTE_REQUEST, 0xff);
                }
                bus.ww(0xff86b0, 0);
                bus.ww(0xff86da, 0);
                *at = ServiceMenu::ReturnToMenu { d7 };
                Yield::Sleep(1)
            } else {
                bus.ww(0xff86b0, 0);
                bus.ww(0xff86da, 0);
                Yield::Sleep(1)
            }
        }
    }
}

/// 0x48678..0x48690: draw the 9 service-menu text blocks (`0x486d4..0x486f4`)
/// and set the initial P1 Up edge latch `0xff884a = 8`.
fn service_menu_draw(bus: &mut Bus) {
    for i in 0..9u32 {
        text_block(bus, bus.rl(0x486d4 + 4 * i));
    }
    bus.wb(0xff884a, 8);
}

/// 0x48692..0x48715: step the service-menu cursor `d7` (`0..=6`) on P1 Up
/// (`IN1` bit 3) or P1 Down (`IN1` bit 2) with wrap and toggle byte `0xff884a`.
fn service_menu_step_cursor(bus: &mut Bus, io: &Io, d7: &mut u8) {
    let d1 = (!io.in1) & 0x000c;
    if d1 == 0 {
        bus.wb(0xff884a, 0);
    } else if d1 == 8 {
        let v = bus.rb(0xff884a) ^ 8;
        bus.wb(0xff884a, v);
        if v != 0 {
            if *d7 == 0 {
                *d7 = 6;
            } else {
                *d7 -= 1;
            }
        }
    } else {
        let v = bus.rb(0xff884a) ^ 4;
        bus.wb(0xff884a, v);
        if v != 0 {
            *d7 += 1;
            if *d7 == 7 {
                *d7 = 0;
            }
        }
    }
}

/// 0x4929c..0x492d5: erase the 9 service-menu text blocks listed at
/// `0x492b2..0x492d2` via `0x3bd00`.
fn erase_service_menu(bus: &mut Bus) {
    for i in 0..9u32 {
        erase_block(bus, bus.rl(0x492b2 + 4 * i));
    }
}

/// Slot 2's service-mode test items 1..=7 (0x487f8..0x4929a; tickets #146 /
/// F74, #147 / F75, and #148 / F76).
fn service_item(bus: &mut Bus, io: &Io, at: &mut ServiceItem) -> Yield {
    match *at {
        ServiceItem::CrosshatchEntry => {
            service_crosshatch_init(bus);
            *at = ServiceItem::CrosshatchLoop;
            Yield::Sleep(0x12)
        }
        ServiceItem::CrosshatchLoop => Yield::Sleep(0x12),
        ServiceItem::ColorBarEntry => {
            service_color_bar_init(bus);
            *at = ServiceItem::ColorBarLoop;
            Yield::Sleep(0x12)
        }
        ServiceItem::ColorBarLoop => Yield::Sleep(0x12),
        ServiceItem::CharTest1Entry => {
            service_char_test1_init(bus);
            *at = ServiceItem::CharTest1Loop;
            Yield::Sleep(0x12)
        }
        ServiceItem::CharTest1Loop => Yield::Sleep(0x12),
        ServiceItem::CharRomEntry => {
            service_char_rom_init(bus);
            *at = ServiceItem::CharRomLoop;
            Yield::Sleep(1)
        }
        ServiceItem::CharRomLoop => {
            service_char_rom_loop(bus, io);
            Yield::Sleep(1)
        }
        ServiceItem::SpriteTestEntry => {
            service_sprite_test_init(bus);
            *at = ServiceItem::SpriteTestLoop;
            Yield::Sleep(1)
        }
        ServiceItem::SpriteTestLoop => {
            service_sprite_test_loop(bus, io);
            Yield::Sleep(1)
        }
        ServiceItem::ScrollTestEntry => {
            service_scroll_test_init_fg(bus);
            *at = ServiceItem::ScrollTestInitBg;
            Yield::Sleep(1)
        }
        ServiceItem::ScrollTestInitBg => {
            service_scroll_test_init_bg(bus);
            *at = ServiceItem::ScrollTestStart;
            Yield::Sleep(1)
        }
        ServiceItem::ScrollTestStart => {
            // 0x48e8c..0x48e94: enable FG + BG (0x60) and fall into 0x48e96.
            gfx_on(bus, 0x60);
            if service_scroll_test_fg_step(bus, io) {
                *at = ServiceItem::ScrollTestBgOnly;
                Yield::Sleep(1)
            } else {
                service_scroll_test_bg_step(bus, io);
                *at = ServiceItem::ScrollTestLoop;
                Yield::Sleep(1)
            }
        }
        ServiceItem::ScrollTestLoop => {
            if service_scroll_test_fg_step(bus, io) {
                *at = ServiceItem::ScrollTestBgOnly;
                Yield::Sleep(1)
            } else {
                service_scroll_test_bg_step(bus, io);
                *at = ServiceItem::ScrollTestLoop;
                Yield::Sleep(1)
            }
        }
        ServiceItem::ScrollTestBgOnly => {
            // 0x49086 -> 0x48f0c: bra $48f40.
            service_scroll_test_bg_step(bus, io);
            *at = ServiceItem::ScrollTestLoop;
            Yield::Sleep(1)
        }
        ServiceItem::OutputTestEntry => {
            service_output_test_init(bus);
            *at = ServiceItem::OutputTestPalette2;
            Yield::Sleep(1)
        }
        ServiceItem::OutputTestPalette2 => {
            // 0x4916a..0x49172 -> 0x4908c..0x490a2: request palette 0x1ae68 at
            // 0x0000 and sleep 1 frame.
            request_palette(bus, 0x1ae68, 0x0000);
            *at = ServiceItem::OutputTestStart;
            Yield::Sleep(1)
        }
        ServiceItem::OutputTestStart => {
            // 0x49174..0x491ce: clr.w D1; reset outputs at 0x49196 and sleep
            // 30 frames (0x3b552) into 0x491d0.
            output_test_reset_outputs(bus);
            *at = ServiceItem::OutputTestApply { d1: 0 };
            Yield::Sleep(0x1e)
        }
        ServiceItem::OutputTestApply { d1 } => {
            // 0x491d0..0x491e0: apply the output entry at 0x491ee + d1 and
            // sleep 30 frames (0x3b552) into 0x491e2.
            output_test_apply_step(bus, d1);
            *at = ServiceItem::OutputTestReset { d1: d1 + 4 };
            Yield::Sleep(0x1e)
        }
        ServiceItem::OutputTestReset { d1 } => {
            // 0x491e2..0x491ec -> 0x49196..0x491ce: advance D1 by 2, wrap
            // 0x96 -> 0, reset outputs, and sleep 30 frames into 0x491d0.
            let mut next_d1 = d1 + 2;
            if next_d1 == 0x96 {
                next_d1 = 0;
            }
            output_test_reset_outputs(bus);
            *at = ServiceItem::OutputTestApply { d1: next_d1 };
            Yield::Sleep(0x1e)
        }
    }
}

/// 0x487f8..0x48856: service menu item 1 (crosshatch grid test) — draw the
/// 14 x 24 2x2 crosshatch grid (`0x1043..0x1046`) across video RAM `0x800000`.
fn service_crosshatch_init(bus: &mut Bus) {
    let mut d1_row = 0x0110u32;
    let mut d2_row = 0x0190u32;
    for _ in 0..14 {
        let mut d1 = d1_row;
        let mut d2 = d2_row;
        for _ in 0..24 {
            bus.ww(0x800000 + d1, 0x1043);
            bus.ww(0x800002 + d1, 0x1044);
            bus.ww(0x800000 + d2, 0x1045);
            bus.ww(0x800002 + d2, 0x1046);
            d1 += 4;
            d2 += 4;
        }
        d1_row += 0x0100;
        d2_row += 0x0100;
    }
}

/// 0x48858..0x488e6: service menu item 3 (character test 1 / color attribute
/// matrix) — erase the menu (`0x4929c`), draw the 4 headers at `0x488e8..0x488f4`,
/// write `0x5000..=0x500f` column headers at `0x800420`, and fill the 4 bands
/// of 4 x 16 longwords starting at `0x800520`.
fn service_char_test1_init(bus: &mut Bus) {
    erase_service_menu(bus);
    for i in 0..4u32 {
        text_block(bus, bus.rl(0x488e8 + 4 * i));
    }
    let mut d1 = 0x0420u32;
    let mut d0 = 0x5000u16;
    for _ in 0..16 {
        bus.ww(0x800000 + d1, d0);
        d1 += 4;
        d0 += 1;
    }
    let mut a1_band = 0x800520u32;
    let mut d1_band = 0x003c_003cu32;
    for _ in 0..4 {
        let mut a1_row = a1_band;
        for _ in 0..4 {
            let mut a1 = a1_row;
            let mut d1_val = d1_band;
            for _ in 0..16 {
                bus.wl(a1, d1_val);
                a1 += 4;
                d1_val = d1_val.wrapping_add(0x1000_1000);
            }
            a1_row += 0x0080;
        }
        a1_band += 0x0280;
        d1_val_step(&mut d1_band);
    }
}

fn d1_val_step(d1_band: &mut u32) {
    *d1_band = d1_band.wrapping_add(0x0001_0001);
}

/// 0x488f8..0x4898e: service menu item 2 (color bar test) — erase the menu
/// (`0x4929c`), draw the 4 headers at `0x48934..0x48940`, request palette
/// `0x1be68 -> 0x0600`, and fill the 4 color bands of 4 x 32 words starting at
/// `0x8004aa`.
fn service_color_bar_init(bus: &mut Bus) {
    erase_service_menu(bus);
    for i in 0..4u32 {
        text_block(bus, bus.rl(0x48934 + 4 * i));
    }
    bus.wl(0xff8634, 0x1be68);
    bus.ww(0xff8638, 0x0600);
    bus.wb(PALETTE_REQUEST, 0xff);
    let mut d0_band = 0x403cu16;
    let mut a1_band = 0x8004aau32;
    for _ in 0..4 {
        let mut d0 = d0_band;
        let mut a1_row = a1_band;
        for _ in 0..4 {
            let mut a1 = a1_row;
            for _ in 0..32 {
                bus.ww(a1, d0);
                a1 += 2;
            }
            a1_row += 0x0080;
            d0 += 1;
        }
        a1_band += 0x0280;
        d0_band += 0x1000;
    }
}

/// 0x48990..0x48a3e: service menu item 4 (character ROM test) entry — erase
/// the menu (`0x4929c`), draw the 7 headers at `0x489d0..0x489e8`, write the
/// 16 column headers (`0x9000..=0x900f`) at `0x800524`, fill the 16 x 16
/// character table (`0x0000..=0x00ff`) at `0x800624` with row headers at
/// `0x80061c`, and clear `0xff884a`, `0xff884c`, and `0xff884e`.
fn service_char_rom_init(bus: &mut Bus) {
    erase_service_menu(bus);
    for i in 0..7u32 {
        text_block(bus, bus.rl(0x489d0 + 4 * i));
    }
    let mut d1 = 0x0524u32;
    let mut d0 = 0x9000u16;
    for _ in 0..16 {
        bus.ww(0x800000 + d1, d0);
        d1 += 4;
        d0 += 1;
    }
    let mut d1_col = 0x0624u32;
    let mut d2_col = 0x061cu32;
    let mut d4_lo = 0x8000u16;
    let mut cell = 0u16;
    for _ in 0..16 {
        let header = ((d4_lo as u32) << 16) | 0x8000;
        bus.wl(0x800000 + d2_col, header);
        let mut d1_cell = d1_col;
        for _ in 0..16 {
            bus.ww(0x800000 + d1_cell, cell);
            d1_cell += 4;
            cell += 1;
        }
        d1_col += 0x0080;
        d2_col += 0x0080;
        d4_lo += 1;
    }
    bus.wb(0xff884a, 0);
    bus.ww(0xff884c, 0);
    bus.ww(0xff884e, 0);
}

/// 0x48a40..0x48b06: one frame of service menu item 4's `IN1` page/color
/// selector loop.
fn service_char_rom_loop(bus: &mut Bus, io: &Io) {
    let d1 = (!io.in1) & 0x000f;
    if d1 == 8 {
        if bus.rb(0xff884a) != 8 {
            bus.wb(0xff884a, 8);
            let v = bus.rw(0xff884c).wrapping_add(1) & 3;
            bus.ww(0xff884c, v);
            char_rom_update_page(bus);
        }
    } else if d1 == 4 {
        if bus.rb(0xff884a) != 4 {
            bus.wb(0xff884a, 4);
            let v = bus.rw(0xff884c).wrapping_sub(1) & 3;
            bus.ww(0xff884c, v);
            char_rom_update_page(bus);
        }
    } else if d1 == 1 {
        if bus.rb(0xff884a) != 1 {
            bus.wb(0xff884a, 1);
            let v = bus.rw(0xff884e).wrapping_add(1) & 0x0f;
            bus.ww(0xff884e, v);
            char_rom_update_color(bus);
        }
    } else if d1 == 2 {
        if bus.rb(0xff884a) != 2 {
            bus.wb(0xff884a, 2);
            let v = bus.rw(0xff884e).wrapping_sub(1) & 0x0f;
            bus.ww(0xff884e, v);
            char_rom_update_color(bus);
        }
    } else {
        bus.wb(0xff884a, 0);
    }
}

/// 0x48b08..0x48b1a: write the 2-bit character page (`0xff884c`) as hex at
/// `0x8003a2` and update bits 8..9 of all 256 cells in the 16 x 16 table.
fn char_rom_update_page(bus: &mut Bus) {
    let d0 = bus.rw(0xff884c);
    let d4 = d0 << 8;
    write_hex_byte(bus, 0x8003a2, d0 as u8, 0x5000);
    char_rom_apply_mask(bus, d4, 0xfcff);
}

/// 0x48b1c..0x48b2c: write the 4-bit color attribute (`0xff884e`) as hex at
/// `0x8003ba` and update bits 12..15 of all 256 cells in the 16 x 16 table.
fn char_rom_update_color(bus: &mut Bus) {
    let d0 = bus.rw(0xff884e);
    let d4 = d0 << 12;
    write_hex_byte(bus, 0x8003ba, d0 as u8, 0x5000);
    char_rom_apply_mask(bus, d4, 0x0fff);
}

/// 0x48b2e..0x48b68: apply `(cell & d5) | d4` across the 16 x 16 character
/// table starting at `0x800624`.
fn char_rom_apply_mask(bus: &mut Bus, d4: u16, d5: u16) {
    let mut d1_col = 0x0624u32;
    for _ in 0..16 {
        let mut d1 = d1_col;
        for _ in 0..16 {
            let addr = 0x800000 + d1;
            let d3 = (bus.rw(addr) & d5) | d4;
            bus.ww(addr, d3);
            d1 += 4;
        }
        d1_col += 0x0080;
    }
}

/// 0x48b6a..0x48c1c: service menu item 5 (sprite / object ROM test) entry —
/// erase the menu (`0x4929c`), draw the 7 headers at `0x48d88..0x48da0`, write
/// the 16 column headers (`0x9000..=0x900f`) at `0x800520`, clear `0xff8850`
/// and `0xff884e`, write the 8 row headers (`0x48d52`), populate 32 sprites
/// (`4` rows x `8` cols) in object RAM from `0xffe080` (stride `0x40`), clear
/// `0xff884a`, and enable the sprite layer (`0x80` via `0x3b644`).
fn service_sprite_test_init(bus: &mut Bus) {
    erase_service_menu(bus);
    for i in 0..7u32 {
        text_block(bus, bus.rl(0x48d88 + 4 * i));
    }
    let mut d2 = 0x0520u32;
    let mut d0 = 0x9000u16;
    for _ in 0..16 {
        bus.ww(0x800000 + d2, d0);
        d2 += 4;
        d0 += 1;
    }
    bus.ww(0xff8850, 0);
    bus.ww(0xff884e, 0);
    sprite_test_row_headers(bus, 0);
    let mut d1_row = 0u16;
    let mut d2_row = 0x0060_0080u32;
    let mut d4 = 0x0080u32;
    for _ in 0..4 {
        let mut d1 = d1_row;
        let mut d2 = d2_row;
        for _ in 0..8 {
            bus.ww(0xffe000 + d4, d1);
            bus.ww(0xffe002 + d4, 0x0400);
            bus.wl(0xffe004 + d4, d2);
            d4 += 0x40;
            d2 = (d2 & 0xffff_0000) | ((d2 as u16).wrapping_add(0x20) as u32);
            d1 = d1.wrapping_add(2);
        }
        d2_row = d2_row.wrapping_add(0x0020_0000);
        d1_row = d1_row.wrapping_add(0x0020);
    }
    bus.wb(0xff884a, 0);
    gfx_on(bus, 0x80);
}

/// 0x48c1e..0x48d50: one frame of service menu item 5's `IN1` page/color
/// selector loop.
fn service_sprite_test_loop(bus: &mut Bus, io: &Io) {
    let d1 = (!io.in1) & 0x000f;
    if !matches!(d1, 8 | 4 | 2 | 1) {
        bus.wb(0xff884a, 0);
        return;
    }
    if bus.rb(0xff884a) == d1 as u8 {
        return;
    }
    bus.wb(0xff884a, d1 as u8);
    match d1 {
        1 => {
            let v = bus.rw(0xff884e).wrapping_add(1) & 0x000f;
            bus.ww(0xff884e, v);
            sprite_test_update_color(bus);
        }
        2 => {
            let v = bus.rw(0xff884e).wrapping_sub(1) & 0x000f;
            bus.ww(0xff884e, v);
            sprite_test_update_color(bus);
        }
        8 => {
            let lo = bus.rb(0xff8851).wrapping_add(0x20);
            bus.wb(0xff8851, lo);
            if lo > 0x80 {
                let hi = bus.rb(0xff8850).wrapping_add(1) & 0x3f;
                bus.wb(0xff8850, hi);
                bus.wb(0xff8851, 0);
                write_hex_byte(bus, 0x8003a2, hi, 0x5000);
            }
            sprite_test_update_page(bus);
        }
        4 => {
            let (lo, borrow) = bus.rb(0xff8851).overflowing_sub(0x20);
            bus.wb(0xff8851, lo);
            if borrow {
                let hi = bus.rb(0xff8850).wrapping_sub(1) & 0x3f;
                bus.wb(0xff8850, hi);
                bus.wb(0xff8851, 0x80);
                write_hex_byte(bus, 0x8003a2, hi, 0x5000);
            }
            sprite_test_update_page(bus);
        }
        _ => {}
    }
}

/// 0x48cd2..0x48d0c: update the 8 row headers (`0x48d52`) and rewrite `+0`
/// across all 32 sprites (`4` rows x `8` cols from `0xffe080`, stride `0x40`)
/// from the 16-bit base index at `0xff8850`.
fn sprite_test_update_page(bus: &mut Bus) {
    let mut d1_row = bus.rw(0xff8850);
    sprite_test_row_headers(bus, d1_row & 0x00ff);
    let mut d0 = 0x0080u32;
    for _ in 0..4 {
        let mut d1 = d1_row;
        for _ in 0..8 {
            bus.ww(0xffe000 + d0, d1);
            d1 = d1.wrapping_add(2);
            d0 += 0x40;
        }
        d1_row = d1_row.wrapping_add(0x0020);
    }
}

/// 0x48d20..0x48d50: write the 4-bit sprite palette (`0xff884e`) as hex at
/// `0x8003ba` and update `+2` of all 32 sprites (`0xffe082 + i * 0x40`) to
/// `0x0400 | (0xff884e & 0x0f)`.
fn sprite_test_update_color(bus: &mut Bus) {
    let d0 = bus.rw(0xff884e);
    write_hex_byte(bus, 0x8003ba, d0 as u8, 0x5000);
    let attr = d0 | 0x0400;
    let mut d1 = 0x0080u32;
    for _ in 0..32 {
        bus.ww(0xffe002 + d1, attr);
        d1 += 0x40;
    }
}

/// 0x48d52..0x48d86: write 8 two-digit hex row headers (`0x9000_9000` or'ed
/// with the high and low nibbles of `d0`, advancing `d0` by `0x10`) at
/// `0x800618` (stride `0x0100`).
fn sprite_test_row_headers(bus: &mut Bus, mut d0: u16) {
    let mut d2 = 0x0618u32;
    for _ in 0..8 {
        let hi = (((d0 as u8) >> 4) & 0x0f) as u32;
        let lo = ((d0 as u8) & 0x0f) as u32;
        let d4 = ((0x9000 | hi) << 16) | (0x9000 | lo);
        bus.wl(0x800000 + d2, d4);
        d2 += 0x0100;
        d0 = d0.wrapping_add(0x0010);
    }
}

/// 0x48da4..0x48e68: service menu item 6 (scroll test) entry and FG init —
/// erase the menu (`0x4929c`), draw the 5 text blocks at `0x48dd6..0x48de6`,
/// draw the `7 x 6` grid of `0x6045` ticks from `0x8001c0` (column stride `+8`,
/// row stride `+0x0200`) and the axis digits `0x6000..=0x6007` (`0x48e0a..0x48e3c`),
/// clear `0xffc680` and `0xffc682`, clear `0xff8852..0xff8862` (`0x3b93c`),
/// scroll FG to `0` (`0x3b674`), draw the FG coordinate words (`0x490a4`), and
/// request FG palette `0x1a268` at `0x0200` (`0x4906c`).
fn service_scroll_test_init_fg(bus: &mut Bus) {
    erase_service_menu(bus);
    for i in 0..5u32 {
        text_block(bus, bus.rl(0x48dd6 + 4 * i));
    }
    let mut row_off = 0x01c0u16;
    for _ in 0..7 {
        let mut d0 = row_off;
        for _ in 0..6 {
            bus.ww(0x800000 + d0 as u32, 0x6045);
            d0 = d0.wrapping_add(8);
        }
        row_off = row_off.wrapping_add(0x0200);
    }
    let mut d0 = 0x6000u16;
    let mut d1 = 0x6001u16;
    bus.ww(0x8001bc, d0);
    let mut d4 = 0x01c4u16;
    let mut d5 = 0x02bcu16;
    for _ in 0..6 {
        bus.ww(0x800000 + d4 as u32, d0);
        bus.ww(0x800000 + d5 as u32, d1);
        d0 = d0.wrapping_add(1);
        d1 = d1.wrapping_add(1);
        d4 = d4.wrapping_add(8);
        d5 = d5.wrapping_add(0x0200);
    }
    bus.ww(0x800e3c, d1);
    bus.ww(0xffc680, 0);
    bus.ww(0xffc682, 0);
    bus.clear_words_8(0xff8852);
    add_fg_scroll(bus, 0);
    scroll_test_draw_fg_coords(bus);
    let pal = bus.rl(0x48f28);
    bus.wl(0xff8856, pal);
    request_palette(bus, pal, 0x0200);
}

/// 0x48e6a..0x48e8a: service menu item 6 BG init — clear `0xff885a..0xff886a`
/// (`0x3b93c`), scroll BG to `0` (`0x3b682`), draw the BG coordinate words
/// (`0x490ce`), and request BG palette `0x1ae68` at `0x0000` (`0x49088`).
fn service_scroll_test_init_bg(bus: &mut Bus) {
    bus.clear_words_8(0xff885a);
    add_bg_scroll(bus, 0);
    scroll_test_draw_bg_coords(bus);
    let pal = bus.rl(0x48fd2);
    bus.wl(0xff885e, pal);
    request_palette(bus, pal, 0x0000);
}

/// 0x48e96..0x48f0c: one frame's FG scroll step (`A2 = 0xff8852`, P1 `IN1` low
/// byte at `0xc00003`). Returns `true` when `0x48f08: bsr $4906c` requests a
/// new FG palette and sleeps 1 frame at `0x49080` before `0x48f0c: bra $48f40`.
fn service_scroll_test_fg_step(bus: &mut Bus, io: &Io) -> bool {
    scroll_test_step_input(bus, 0xff8852, io.in1 as u8);
    let delay = bus.rb(0xff8852);
    if delay != 0 {
        bus.wb(0xff8852, delay.wrapping_sub(1));
        return false;
    }
    let step = bus.rb(0xff8853);
    if step != 0 {
        let delta = step as i8 as i16 as u16;
        bus.wb(0xff8853, 0);
        add_fg_scroll(bus, delta);
        scroll_test_draw_fg_coords(bus);
    }
    let d0 = bus.rw(0xffc680);
    for d2 in 0..6u32 {
        let a0 = 0x48f10 + 4 * d2;
        if d0 == bus.rw(a0) || d0 <= bus.rw(a0 + 2) {
            let d1 = bus.rl(0x48f28 + 4 * d2);
            if d1 != bus.rl(0xff8856) {
                bus.wl(0xff8856, d1);
                request_palette(bus, d1, 0x0200);
                return true;
            }
            break;
        }
    }
    false
}

/// 0x48f40..0x48ff0: one frame's BG scroll step (`A2 = 0xff885a`, P2 `IN1` high
/// byte at `0xc00002`), followed by a 1-frame sleep at `0x48fea` (or `0x4909c`
/// via `0x48fb2: bsr $49088`) back into `0x48e96`.
fn service_scroll_test_bg_step(bus: &mut Bus, io: &Io) {
    scroll_test_step_input(bus, 0xff885a, (io.in1 >> 8) as u8);
    let delay = bus.rb(0xff885a);
    if delay != 0 {
        bus.wb(0xff885a, delay.wrapping_sub(1));
        return;
    }
    let step = bus.rb(0xff885b);
    if step != 0 {
        let delta = step as i8 as i16 as u16;
        bus.wb(0xff885b, 0);
        add_bg_scroll(bus, delta);
        scroll_test_draw_bg_coords(bus);
    }
    let d0 = bus.rw(0xffc682);
    for d2 in 0..6u32 {
        let a0 = 0x48fba + 4 * d2;
        if d0 == bus.rw(a0) || d0 <= bus.rw(a0 + 2) {
            let d1 = bus.rl(0x48fd2 + 4 * d2);
            if d1 != bus.rl(0xff885e) {
                bus.wl(0xff885e, d1);
                request_palette(bus, d1, 0x0000);
            }
            break;
        }
    }
}

/// 0x48ff4..0x4906b: decode a cardinal direction from `raw_byte` (`(!raw_byte) &
/// 0x0f` against `0x49060..0x49063` = `[8, 4, 2, 1]`) into the 8-byte scroll
/// control block at `a2`: `+0` delay, `+1` signed step, `+2` held-direction
/// latch, `+3` 2-bit auto-repeat counter (`0..=3`, clearing `+2` on wrap to 0
/// so a held direction re-triggers every 4th check after `+1` is consumed).
fn scroll_test_step_input(bus: &mut Bus, a2: u32, raw_byte: u8) {
    if bus.rb(a2 + 1) != 0 {
        return;
    }
    let d1 = (!raw_byte) & 0x0f;
    if d1 == 0 {
        bus.wb(a2 + 2, 0);
        bus.wb(a2 + 3, 0);
        return;
    }
    let mut matched_d0 = None;
    for k in 0..4u32 {
        let d0 = 3 - k;
        if bus.rb(0x49060 + k) == d1 {
            matched_d0 = Some(d0);
            break;
        }
    }
    let Some(d0) = matched_d0 else {
        bus.wb(a2 + 2, 0);
        bus.wb(a2 + 3, 0);
        return;
    };
    if bus.rb(a2 + 2) == d1 {
        let c = bus.rb(a2 + 3).wrapping_add(1) & 3;
        bus.wb(a2 + 3, c);
        if c == 0 {
            bus.wb(a2 + 2, 0);
        }
    } else {
        bus.wb(a2 + 2, d1);
        bus.wb(a2 + 3, 0);
        let entry = 0x49064 + 2 * d0;
        bus.wb(a2 + 1, bus.rb(entry));
        bus.wb(a2, bus.rb(entry + 1));
    }
}

/// 0x490a4..0x490cc: draw `FG_SCROLL_COPY` (`0xffc680`) at `0x80069c` (`attr =
/// 0x9000`) and `FG_SCROLL_COPY + 0x0100` at `0x8006a8` (`attr = 0x5000`).
fn scroll_test_draw_fg_coords(bus: &mut Bus) {
    let d0 = bus.rw(0xffc680);
    write_hex_word(bus, 0x80069c, d0, 0x9000);
    write_hex_word(bus, 0x8006a8, d0.wrapping_add(0x0100), 0x5000);
}

/// 0x490ce..0x490f6: draw `BG_SCROLL_COPY` (`0xffc682`) at `0x80051c` (`attr =
/// 0x9000`) and `BG_SCROLL_COPY + 0x0100` at `0x800528` (`attr = 0x5000`).
fn scroll_test_draw_bg_coords(bus: &mut Bus) {
    let d0 = bus.rw(0xffc682);
    write_hex_word(bus, 0x80051c, d0, 0x9000);
    write_hex_word(bus, 0x800528, d0.wrapping_add(0x0100), 0x5000);
}

/// 0x490f8..0x49168: service menu item 7 (output port test) entry — erase the
/// menu (`0x4929c`), draw the 7 headers at `0x4917a..0x49192`, clear `+4` of
/// all 32 sprite entries from `0xffe080`, place sprite 0 (`0x008c`, `0x0402`,
/// `0x0090_0108`) at `0xffe080`, scroll fg to `0x13a0` and bg to `0x1380`, and
/// request palette `0x1a268` at `0x0200` (`0x49070`).
fn service_output_test_init(bus: &mut Bus) {
    erase_service_menu(bus);
    for i in 0..7u32 {
        text_block(bus, bus.rl(0x4917a + 4 * i));
    }
    let mut d1 = 0x0080u32;
    for _ in 0..32 {
        bus.wl(0xffe004 + d1, 0);
        d1 += 0x40;
    }
    bus.ww(0xffe080, 0x008c);
    bus.ww(0xffe082, 0x0402);
    bus.wl(0xffe084, 0x0090_0108);
    bus.ww(0xffc680, 0);
    bus.ww(0xffc682, 0);
    add_fg_scroll(bus, 0x13a0);
    add_bg_scroll(bus, 0x1380);
    request_palette(bus, 0x1a268, 0x0200);
}

/// 0x49196..0x491ce: reset all output ports and test targets to their baseline
/// state between 30-frame pulses (`gfxctrl = 0xe8`, `AUX_CONTROL = 0`,
/// `COIN_CONTROL = 0`, `0xffe082 &= 0x04ff`, `0x8005c2 &= 0xf3ff`).
fn output_test_reset_outputs(bus: &mut Bus) {
    *bus.gfxctrl = 0xe8;
    *bus.flip_screen = false;
    bus.wb(0xff8682, 0xe8);
    bus.ww(AUX_CONTROL, 0);
    *bus.coin_ctrl = 0;
    bus.ww(COIN_CONTROL, 0);
    bus.ww(0xffe082, bus.rw(0xffe082) & 0x04ff);
    bus.ww(0x8005c2, bus.rw(0x8005c2) & 0xf3ff);
}

/// 0x491d0..0x491da, 0x491ee..0x4929a: dispatch one 6-byte entry `(fn_addr, d0)`
/// at `0x491ee + d1` (`d1 = 0, 6, ..., 0x90`) in the 25-step output cycle.
fn output_test_apply_step(bus: &mut Bus, d1: u16) {
    let entry = 0x491ee + d1 as u32;
    let fn_addr = bus.rl(entry);
    let d0 = bus.rw(entry + 4);
    match fn_addr {
        0x3b654 => coin_ctrl_or(bus, d0),
        0x3b664 => aux_ctrl_or(bus, d0),
        0x3b690 => gfx_off(bus, d0 as u8),
        0x3b644 => gfx_on(bus, d0 as u8),
        0x49284 => bus.ww(0xffe082, bus.rw(0xffe082) | d0),
        0x49290 => bus.ww(0x8005c2, bus.rw(0x8005c2) | d0),
        _ => unreachable!("unexpected output test step target {fn_addr:#x}"),
    }
}

/// 0x3b2e4: decode the two coin slots' coinage words from DSW1 bits 0-2
/// (slot 1 at 0xff863c) and bits 3-5 (slot 2 at 0xff8644) through the 8-word
/// table at 0x1456a.
fn decode_coinage(bus: &mut Bus, dsw1: u16) {
    let d1 = !dsw1;
    let d0 = (d1 & 0x0007) << 1;
    bus.ww(COIN_SLOTS[0] + 2, bus.rw(0x1456a + d0 as u32));
    let d0 = (d1 & 0x0038) >> 2;
    bus.ww(COIN_SLOTS[1] + 2, bus.rw(0x1456a + d0 as u32));
}

/// 0x3b77a: pulse gfxctrl bit 0 (0x3b644 / 0x3b690), reset the sound
/// queue's read/write indices (0xff865e / 0xff8660), fill its 16 words with
/// 0xffff (0x3b914), and queue the boot commands 0x27, 0x3f, 0x80 (0x3b7a8).
fn boot_init(bus: &mut Bus) {
    gfx_on(bus, 1);
    gfx_off(bus, 1);
    bus.ww(0xff865e, 0);
    bus.ww(0xff8660, 0);
    fill_words_16(bus, 0xff8662, 0xffff);
    bus.toggle_state.queued_themes.fill(None);
    queue_boot_sounds(bus);
}

/// 0x3b904: fill 4 words from `addr` with `value` (`D1 = 0x3` into
/// 0x3b928).
fn fill_words_4(bus: &mut Bus, addr: u32, value: u16) {
    for i in 0..4u32 {
        bus.ww(addr + 2 * i, value);
    }
}

/// 0x3b914: fill 16 words from `addr` with `value` (`D1 = 0xf` into
/// 0x3b928).
fn fill_words_16(bus: &mut Bus, addr: u32, value: u16) {
    for i in 0..16u32 {
        bus.ww(addr + 2 * i, value);
    }
}

/// 0x3b7a8: queue the three boot sound commands (0x27, 0x3f, 0x80) through
/// the unfiltered entry 0x3b802.
fn queue_boot_sounds(bus: &mut Bus) {
    for cmd in [0x27, 0x3f, 0x80] {
        bus.queue_sound_unfiltered(cmd);
    }
}

/// 0x3b5f8..0x3b62e: copy 0x80 longwords (0x200 bytes) from ROM `src` (A0)
/// into palette RAM at `offset` (D1), before sleeping 1 frame at 0x3b62e
/// (with D1 and A0 saved on the stack). On wakeup (0x3b634), 0x3b638 checks
/// SYSTEM bit 6 and loops to 0x3b602 when low (reported, not translated:
/// MAME defines bit 6 as IPT_UNKNOWN).
fn upload_palette(bus: &mut Bus, src: u32, offset: u16) {
    let off = offset as usize;
    // A pointer into the extended background bank's palettes, which only a
    // stage gate names (ADR-0009); the 1987 game requests 0x1a268..0x1c068.
    if let Some(bytes) = bus.backgrounds.and_then(|bank| bank.palette(src)) {
        for (i, &b) in bytes.iter().enumerate() {
            if let Some(p) = bus.palette.get_mut(off + i) {
                *p = b;
            }
        }
        return;
    }
    for i in 0..0x200usize {
        let b = bus.rb(src + i as u32);
        if let Some(p) = bus.palette.get_mut(off + i) {
            *p = b;
        }
    }
}

/// 0x3bdb0: fill the 64x32 text tilemap (0x800000..0x801000, 0x800 words)
/// with the blank tile 0x103c.
fn clear_text_layer(bus: &mut Bus) {
    for i in 0..0x800u32 {
        bus.ww(0x800000 + 2 * i, 0x103c);
    }
}

/// The four palette uploads of 0x3c0d2..0x3c10c: (ROM source, palette byte
/// offset).
const BOOT_PALETTES: [(u32, u16); 4] = [
    (0x1ae68, 0x0000),
    (0x1a268, 0x0200),
    (0x1ba68, 0x0400),
    (0x1bc68, 0x0600),
];

/// Slot 0's boot task (0x3c0c4..0x3c290; tickets #41, #55).
fn boot(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Boot) -> (Option<Task>, Yield) {
    match *at {
        Boot::Entry => {
            // 0x3c0c4: lea ($2a2,A5), A7; 0x3c0c8: jsr $3b51a.l.
            *at = Boot::Upload(0);
            (None, Yield::Sleep(1))
        }
        Boot::Upload(step) => {
            if step == 0 {
                // 0x3c0ce: addq.w #1, ($622,A5)
                bus.ww(0xff8622, bus.rw(0xff8622).wrapping_add(1));
            } else if io.system & 0x40 == 0 {
                // 0x3b638..0x3b640: SYSTEM bit 6 low repeats the upload at
                // 0x3b602 (reported, not translated).
                bus.note(0x3b602);
                return (None, Yield::Halt);
            }
            if (step as usize) < BOOT_PALETTES.len() {
                let (src, off) = BOOT_PALETTES[step as usize];
                upload_palette(bus, src, off);
                *at = Boot::Upload(step + 1);
                return (None, Yield::Sleep(1));
            }
            // 0x3c112..0x3c204: clear object RAM and the text layer,
            // disable layers 0xe8, decode DSW2 / DSW1, copy the ranking
            // table (0x14598), top score and player structures (0x3c296),
            // disable 0xe8 and clear the text layer again. By summed 68000
            // costs (~139,882 cycles > 133,333 1/3 per frame), the second
            // 0x3bdb0 (0x3c204) overruns frame 6 into frame 7 before
            // reaching 0x3c20a's 1-frame sleep.
            boot_setup_chunk(bus, io);
            *at = Boot::SettleOverrun;
            (None, Yield::Overrun)
        }
        Boot::SettleOverrun => {
            // 0x3c20a: jsr $3b51a.l (after 0x3c204 finishes in frame 7).
            *at = Boot::Warning;
            (None, Yield::Sleep(1))
        }
        Boot::Warning => {
            // 0x3c210..0x3c236: turn on tx (0x08) and start typing block 0
            // (D2 = 3, D4 = 2).
            gfx_on(bus, 8);
            (None, start_warning_block(bus, at, 0))
        }
        Boot::WarningTyping { index, mut writer } => match writer.run(bus) {
            Some(frames) => {
                *at = Boot::WarningTyping { index, writer };
                (None, Yield::Sleep(frames))
            }
            None => {
                // 0x3c238: jsr $3b53a.l (12-frame pause after each block).
                *at = Boot::WarningPause { index };
                (None, Yield::Sleep(12))
            }
        },
        Boot::WarningPause { index } => {
            // 0x3c23e: dbra D2, $3c230; after all four blocks, hold 240 frames
            // (0x3b57a at 0x3c254, duration word at 0x3b57c).
            if index + 1 < 4 {
                (None, start_warning_block(bus, at, index + 1))
            } else {
                let hold = bus.rw(0x3b57c) as u8;
                *at = Boot::WarningHold;
                (None, Yield::Sleep(hold))
            }
        }
        Boot::WarningHold => {
            // 0x3c25a..0x3c290: erase all four warning blocks (0x3bd00),
            // enable coin control 0x7070 (0x3b654), spawn slot 1 at 0x3ccae
            // and slot 7 at 0x4813e (0x3b20a), and jump into the timer task
            // at 0x3f4de.
            for i in 0..4u8 {
                let block = warning_block_ptr(bus, i);
                erase_block(bus, block);
            }
            coin_ctrl_or(bus, 0x7070);
            scheduler.spawn(bus, 1, 0x3ccae);
            scheduler.spawn(bus, 7, 0x4813e);
            round_timer(bus);
            (Some(Task::Timer), Yield::Sleep(1))
        }
    }
}

/// 0x3c21a..0x3c226 / 0x3c242..0x3c24e: warning block `index` (0..4) from
/// the table at `0x3c300 + sext(byte_at(0x3cc9a))`.
fn warning_block_ptr(bus: &Bus, index: u8) -> u32 {
    let off = bus.rb(0x3cc9a) as i8 as i16 as i32;
    let base = 0x3c300u32.wrapping_add(off as u32);
    bus.rl(base + 4 * index as u32)
}

/// 0x3c230..0x3c238: start typing warning block `index` with typewriter
/// delay D4 = 2 (`0x3bb98`).
fn start_warning_block(bus: &mut Bus, at: &mut Boot, index: u8) -> Yield {
    let block = warning_block_ptr(bus, index);
    let mut writer = TextWriter::new(bus, block, 2);
    match writer.run(bus) {
        Some(frames) => {
            *at = Boot::WarningTyping { index, writer };
            Yield::Sleep(frames)
        }
        None => {
            *at = Boot::WarningPause { index };
            Yield::Sleep(12)
        }
    }
}

/// 0x3c112..0x3c204: object and text clears, DIP decode, ranking table,
/// top score and player-structure copies.
fn boot_setup_chunk(bus: &mut Bus, io: &Io) {
    // 0x3c112..0x3c11a: clear object RAM (0x800 longs at A4 = 0xffe000).
    bus.obj.fill(0);
    // 0x3c11e: jsr $3bdb0.l
    clear_text_layer(bus);
    // 0x3c124..0x3c12c: clr.w ($16,A6); move.w #$e8, D0; jsr $3b690.l
    gfx_off(bus, 0xe8);
    // 0x3c132..0x3c142: difficulty from DSW2 low byte bits 5-6.
    let dsw2_lo = !(io.dsw2 as u8);
    let difficulty = (dsw2_lo & 0x60) >> 5;
    bus.wb(0xffc007, difficulty);
    bus.wb(0xff8f28, difficulty);
    // 0x3c146..0x3c152: round-time setting from DSW2 low byte bits 3-4.
    bus.wb(TIME_SETTING, (dsw2_lo & 0x18) >> 3);
    // 0x3c156..0x3c166: game continuation from DSW2 bits 0-2 via table
    // 0x3c16a.
    let cont = bus.rb(0x3c16a + (dsw2_lo & 0x07) as u32);
    bus.wb(0xffc000, cont);
    // 0x3c172..0x3c184: buy-in from DSW2 bits 7-9 via table 0x3c188.
    let dsw2_inv = !io.dsw2;
    let buyin = bus.rb(0x3c188 + ((dsw2_inv & 0x0380) >> 7) as u32);
    bus.wb(0xffc001, buyin);
    // 0x3c190..0x3c1a8: countries count from DSW2 bit 10 (0 = 4 countries
    // -> 0x00, 1 = 2 countries -> 0xff).
    bus.wb(0xffc005, if dsw2_inv & 0x0400 != 0 { 0 } else { 0xff });
    // 0x3c1ac..0x3c1bc: flip screen from DSW1 high byte bit 0 (active low
    // -> gfx_on(4)).
    if (io.dsw1 >> 8) as u8 & 1 == 0 {
        gfx_on(bus, 4);
    }
    // 0x3c1c2..0x3c1d2: 40 longs from 0x14598 to 0xff8e6e..0xff8f0e.
    for i in 0..0x28u32 {
        bus.wl(0xff8e6e + 4 * i, bus.rl(0x14598 + 4 * i));
    }
    // 0x3c1d6..0x3c1e4: 8 bytes from 0xff8e73 to 0xffc012 (top score).
    for i in 0..8u32 {
        bus.wb(0xffc012 + i, bus.rb(0xff8e73 + i));
    }
    // 0x3c1e8..0x3c1f6: 48 words from 0x3c296 to 0xff86b0..0xff8710.
    for i in 0..0x30u32 {
        bus.ww(0xff86b0 + 2 * i, bus.rw(0x3c296 + 2 * i));
    }
    // 0x3c1fa..0x3c204: gfx_off(0xe8) and clear_text_layer (0x3bdb0).
    gfx_off(bus, 0xe8);
    clear_text_layer(bus);
}

/// The timer task's flags (0xff8e16): bit 0 reloads the round timer; bit 1
/// starts the continue countdown (0xff8e1e, 10.00); bit 2 starts the
/// name-entry countdown (0xff8e24, 30.00); bit 3 starts the ranking display
/// timer (0xff8e2a, 03.00); bits 4-5 start the country-select (05.00) and
/// bonus-stage (10.00) timer (0xff8e30).
const TIMER_FLAGS: u32 = 0xff8e16;
/// The round timer's frame counter (0..=0x12) and the round-time setting
/// (DSW2: 100 / 150 / 200 / 250, index into the reload table 0x3f544).
const TIME_FRAMES: u32 = 0xff8e1b;
const TIME_SETTING: u32 = 0xff8e1c;

/// One pass of the timer task (0x3f4e2..0x3f7a6), before its one-frame
/// sleep:
///
/// - The round timer: a reload request (flag bit 0) copies the setting's
///   three digits from 0x3f544; otherwise a TIME at zero stays there, and
///   every 0x13th pass (the frame counter reaching 0x12) counts the digits
///   down by one (0x3bb74).
/// - Four screen/countdown timers (A, B words, a frame byte C, a done byte
///   D; at 0xff8e1e continue, 0xff8e24 name entry, 0xff8e2a ranking,
///   0xff8e30 country select / bonus stage): a start flag loads A:B and sets
///   C = 2, D = 0; otherwise while B is not zero, every second pass takes 3
///   from B (0x3bb48); with B at zero, every second pass moves one from A
///   into B as 0x0909 until A is zero, then marks D = 1.
///
/// In Training, under every preset, the round timer never counts down: the
/// digits stay at the reloaded value, so time-over never comes (ADR-0013,
/// ticket #240). An overhaul-only gate on the translated path, keyed on the
/// play mode the engine already reads; never set in Arcade, Local, the
/// attract or the faithful cabinet.
pub fn round_timer(bus: &mut Bus) {
    if bus.rb(TIMER_FLAGS) & 1 != 0 {
        bus.wb(TIMER_FLAGS, bus.rb(TIMER_FLAGS) & 0xfe);
        if let Some(ticks) = bus.world_tour_rules().and_then(|rules| rules.time_limit()) {
            bus.wb(TIME_DIGITS, ticks / 100);
            bus.wb(TIME_DIGITS + 1, ticks / 10 % 10);
            bus.wb(TIME_DIGITS + 2, ticks % 10);
            bus.wb(TIME_FRAMES, 0);
        } else {
            let table = 0x3f544 + ((bus.rb(TIME_SETTING) as u32) << 2);
            for i in 0..3 {
                bus.wb(TIME_DIGITS + i, bus.rb(table + i));
            }
        }
    } else if time_word(bus) != 0 && bus.roster.mode != PlayMode::Training && !bus.era_frozen() {
        let limit = if bus.toggles.real_second_timer_and_draws {
            59
        } else {
            0x12
        };
        if bus.rb(TIME_FRAMES) >= limit {
            bus.wb(TIME_FRAMES, 0);
            decimal_down(bus, TIME_FRAMES);
        } else {
            bus.wb(TIME_FRAMES, bus.rb(TIME_FRAMES) + 1);
        }
    }
    let flags = bus.rb(TIMER_FLAGS);
    for (mask, base, start, c_addr) in [
        (0x02u8, 0xff8e1e, 0x0100_0000u32, 0x3f5dbu32),
        (0x04, 0xff8e24, 0x0300_0000, 0x3f669),
        (0x08, 0xff8e2a, 0x0003_0000, 0x3f6fb),
    ] {
        if flags & mask != 0 {
            bus.wb(TIMER_FLAGS, bus.rb(TIMER_FLAGS) & !mask);
            bus.wl(base, start);
            bus.wb(base + 4, bus.rb(c_addr));
            bus.wb(base + 5, 0);
        } else {
            tally(bus, base);
        }
    }
    let base = 0xff8e30;
    if flags & 0x30 != 0 {
        // The last counter's start value depends on which flag started it.
        let start = if bus.rb(TIMER_FLAGS) == 0x10 {
            0x0005_0000
        } else {
            0x0100_0000
        };
        bus.wl(base, start);
        bus.wb(base + 4, 2);
        bus.wb(TIMER_FLAGS, bus.rb(TIMER_FLAGS) & 0x0f);
        bus.wb(base + 5, 0);
    } else {
        tally(bus, base);
    }
}

/// One screen/countdown timer's pass (A at `base`, B at +2, C +4, D +5).
fn tally(bus: &mut Bus, base: u32) {
    let c = bus.rb(base + 4).wrapping_sub(1);
    bus.wb(base + 4, c);
    if bus.rw(base + 2) != 0 {
        if c == 0 {
            for _ in 0..3 {
                decimal_down(bus, base + 4);
            }
            bus.wb(base + 4, 2);
        }
    } else if c == 0 {
        if bus.rw(base) != 0 {
            decimal_down(bus, base + 2);
            bus.ww(base + 2, 0x0909);
            bus.wb(base + 4, 2);
        } else if bus.rb(base + 5) != 1 {
            bus.wb(base + 5, 1);
        }
    }
}

/// The round timer's zero test (move.w e18; add.b e1a: the word with its
/// low byte summed).
fn time_word(bus: &Bus) -> u16 {
    let w = bus.rw(TIME_DIGITS);
    (w & 0xff00) | (w as u8).wrapping_add(bus.rb(TIME_DIGITS + 2)) as u16
}

/// 0x3bb74: count the decimal number whose digits (one per byte) end just
/// before `end` down by one, borrowing 9s leftwards.
fn decimal_down(bus: &mut Bus, end: u32) {
    let mut a = end;
    loop {
        a -= 1;
        let d = bus.rb(a);
        if d >= 1 {
            bus.wb(a, d - 1);
            return;
        }
        bus.wb(a, 9);
    }
}

/// 0x3ea3e..0x3ea60: the fight begins — scene mode 0x80, the button-latch
/// groups and both players' latched strengths cleared (0x3b6c6), and the
/// TIME label drawn (text block 0x14f1c).
fn round_start(bus: &mut Bus) {
    bus.wb(SCENE_MODE, 0x80);
    reset_button_latches(bus);
    for a in [0xff86b0, 0xff86b1, 0xff86da, 0xff86db] {
        bus.wb(a, 0);
    }
    if bus.dramatic_rules().is_some() {
        bus.ww(CPU_STRUCT, 0);
        bus.toggle_state.dramatic.boss_latches.fill(0);
    }
    text_block(bus, 0x14f1c);
}

/// How the fight loop goes on after its sleep (0x3eafa..0x3eb1e).
enum Checks {
    Fight,
    /// A fighter is knocked out (+8 == 8): the KO path 0x3eb22.
    Ko,
    /// TIME is zero: the time-over path 0x3ed20 (not translated).
    TimeOver,
}

/// 0x3eafa..0x3eb1e: after the fight loop's sleep, a timer at zero ends the
/// round by time (0x3ed20) and a knocked-out fighter by KO (0x3eb22).
fn fight_checks(bus: &mut Bus) -> Checks {
    if time_word(bus) == 0 {
        return Checks::TimeOver;
    }
    if bus.dramatic_rules().is_some() {
        let [ally1, boss, ally2] = bus.toggle_state.dramatic.health;
        return if boss == 0 || (ally1 == 0 && ally2 == 0) {
            Checks::Ko
        } else {
            Checks::Fight
        };
    }
    // D1 holds the two fighters' entity offsets.
    if bus.rb(0xffe408) == 8 || bus.rb(0xffea08) == 8 {
        return Checks::Ko;
    }
    Checks::Fight
}

/// The game-flow task (slot 1): attract mode, the start screen, the round
/// start, the fight loop, and the KO path to the round result.
fn flow(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    match *at {
        Flow::RoundStart => {
            round_start(bus);
            *at = Flow::FightLoop;
            fight_frame(bus, io, at)
        }
        Flow::FightLoop => match fight_checks(bus) {
            Checks::Fight => fight_frame(bus, io, at),
            Checks::Ko => {
                // 0x3eb22: the KO path starts with a sleep.
                *at = Flow::KoStart;
                speed_sleep(bus, io)
            }
            Checks::TimeOver => {
                if bus.world_tour_trial(crate::world_tour::native::RuleModifier::ProjectileDefense) {
                    // The trial judges survival at TIME zero, before any
                    // remaining move or projectile settles in the result hold.
                    let survived = bus.era_health(P1_STRUCT).is_some_and(|(hp, _)| hp != 0);
                    bus.wl(WINNER, if survived { P1_STRUCT } else { P2_STRUCT });
                }
                // 0x3ed20: the time-over path starts with a sleep.
                *at = Flow::TimeOverStart;
                speed_sleep(bus, io)
            }
        },
        Flow::ChallengerHold1 => {
            // 0x423de..0x423f4: type 0x15c30 with delay D4 = 3 (0x3bb98),
            // then hold 240 frames (0x3b57a at 0x423ee).
            let mut writer = TextWriter::new(bus, 0x15c30, 3);
            match writer.run(bus) {
                Some(frames) => {
                    *at = Flow::ChallengerTyping { writer };
                    Yield::Sleep(frames)
                }
                None => {
                    *at = Flow::ChallengerHold2;
                    Yield::Sleep(bus.rw(0x3b57c) as u8)
                }
            }
        }
        Flow::ChallengerTyping { mut writer } => match writer.run(bus) {
            Some(frames) => {
                *at = Flow::ChallengerTyping { writer };
                Yield::Sleep(frames)
            }
            None => {
                *at = Flow::ChallengerHold2;
                Yield::Sleep(bus.rw(0x3b57c) as u8)
            }
        },
        Flow::ChallengerHold2 => {
            // 0x423f4..0x423f8: request event 6 (wipe) and wait at 0x422e0.
            request_event(bus, 6);
            *at = Flow::ChallengerWipe;
            speed_sleep(bus, io)
        }
        Flow::ChallengerWipe => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            // 0x423fc..0x4241c: GAME_MODE = 1, ROUND_NO = 0, SCENE_MODE = 0,
            // clear all object regions (0x3b9bc..0x3b9d4), and jump to 0x3e5d6
            // (place_players -> next_opponent).
            bus.wb(GAME_MODE, 1);
            bus.wb(0xffc00a, 0);
            bus.wb(SCENE_MODE, 0);
            clear_objects(bus);
            place_players(bus);
            next_opponent(bus, io, at)
        }
        Flow::NameEntryPalette { ret } => name_entry_arm(bus, at, ret),
        Flow::NameEntryLoop { ret } => name_entry_frame(bus, at, ret),
        Flow::RankingMarked { ret } => ranking_marked_wait(scheduler, bus, io, at, ret),
        Flow::BonusJapanPalette { copy } => {
            // 0x40dd4..0x40dfa (Copy 2: 0x41aa6..0x41acc): load the 32-byte
            // palette tail (0x414f0), turn on layers 0xe0 (0x3b644), draw
            // PUSH PUNCH BUTTON (0x147d2, D1 = 4 into 0x40dfc / 0x41ace), and
            // sleep 12 frames (0x3b53a into 0x40e0e / 0x41ae0).
            bonus_load_palette_tail(bus);
            gfx_on(bus, 0xe0);
            text_block(bus, 0x147d2);
            *at = Flow::BonusJapanBlinkErase { copy, d1: 4 };
            Yield::Sleep(12)
        }
        Flow::BonusJapanBlinkDraw { copy, d1 } => {
            // 0x40dfc..0x40e08 (Copy 2: 0x41ace..0x41ada): draw PUSH PUNCH
            // BUTTON (0x147d2) and sleep 12 frames (0x3b53a into 0x40e0e /
            // 0x41ae0).
            text_block(bus, 0x147d2);
            *at = Flow::BonusJapanBlinkErase { copy, d1 };
            Yield::Sleep(12)
        }
        Flow::BonusJapanBlinkErase { copy, d1 } => {
            // 0x40e0e..0x40e50 (Copy 2: 0x41ae0..0x41b22): erase PUSH PUNCH
            // BUTTON (0x147d2); while D1 > 0 sleep 12 frames (0x3b53a into
            // 0x40dfc / 0x41ace); when D1 == 0, arm timer 3 (0xff8e16 = 0x10),
            // clear the active player's latches, set SCENE_MODE |= 0x80, scroll
            // bg to 0x0d80, reset button latches (0x3b6c6), and sleep 1 frame
            // (0x3b51a into 0x40e56 / 0x41b28).
            erase_block(bus, 0x147d2);
            if d1 != 0 {
                *at = Flow::BonusJapanBlinkDraw { copy, d1: d1 - 1 };
                return Yield::Sleep(12);
            }
            bus.wb(0xff8f0e, 0);
            bus.wb(TIMER_FLAGS, 0x10);
            let a3 = bus.rl(0xff8f14);
            bus.ww(a3, 0);
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) | 0x80);
            bus.ww(0xffc682, 0);
            add_bg_scroll(bus, 0x0d80);
            reset_button_latches(bus);
            *at = Flow::BonusJapanMeterStart { copy };
            Yield::Sleep(1)
        }
        Flow::BonusJapanMeterStart { copy } => {
            // 0x40e56..0x40e6a (Copy 2: 0x41b28..0x41b3c): queue sound 0x3a,
            // push 0x40f20 / 0x41bf2, clear 0xff8f0f and enter the power-meter
            // loop at 0x40e76 / 0x41b48.
            bus.queue_sound(0x3a);
            bus.wb(0xff8f0f, 0);
            bonus_japan_meter_frame(bus, at, copy, 0x40f20 + copy.meter_shift(), 0)
        }
        Flow::BonusJapanMeter { copy, cont, d0 } => {
            bonus_japan_meter_frame(bus, at, copy, cont, d0)
        }
        Flow::BonusJapanWindup { copy, a0 } => bonus_japan_windup(bus, at, copy, a0),
        Flow::BonusJapanBreak { copy, a1, d2 } => bonus_japan_break(bus, at, copy, a1, d2),
        Flow::BonusJapanSuccessPose { copy } => {
            // 0x41166..0x4117e (Copy 2: 0x41e1a..0x41e32): queue voice 0x8e,
            // scroll fg to 0x1380, and sleep 30 frames (0x3b552 into 0x41184 /
            // 0x41e38).
            bus.queue_sound(0x8e);
            bus.ww(0xffc680, 0);
            add_fg_scroll(bus, 0x1380);
            *at = Flow::BonusJapanCheerStart { copy };
            Yield::Sleep(30)
        }
        Flow::BonusJapanCheerStart { copy } => {
            // 0x41184..0x411a0 (Copy 2: 0x41e38..0x41e54): scroll bg to 0x0f00,
            // set D2 = 14, queue cheer sound 0xa0, and sleep 6 frames (0x3b532
            // into 0x411a6 / 0x41e5a).
            bus.ww(0xffc682, 0);
            add_bg_scroll(bus, 0x0f00);
            bus.queue_sound(0xa0);
            *at = Flow::BonusJapanCheerA { copy, d2: 14 };
            Yield::Sleep(6)
        }
        Flow::BonusJapanCheerA { copy, d2 } => {
            // 0x411a6..0x411b4 (Copy 2: 0x41e5a..0x41e68): scroll bg to 0x1080
            // and sleep 6 frames (0x3b532 into 0x411ba / 0x41e6e).
            bus.ww(0xffc682, 0);
            add_bg_scroll(bus, 0x1080);
            *at = Flow::BonusJapanCheerB { copy, d2 };
            Yield::Sleep(6)
        }
        Flow::BonusJapanCheerB { copy, d2 } => bonus_japan_cheer_b(bus, at, copy, d2),
        Flow::BonusJapanHold1 { copy } => {
            // 0x4120c (Copy 2: 0x41ec0): second 120-frame hold (0x3b56a into
            // 0x41212 / 0x41ec6).
            *at = Flow::BonusJapanHold2 { copy };
            Yield::Sleep(120)
        }
        Flow::BonusJapanHold2 { .. } => {
            // 0x41212..0x41220 (Copy 2: 0x41ec6..0x41ed4): stop sound (0x3f),
            // clear SCENE_MODE (0xffc008), and advance stage at 0x3efc4.
            bus.queue_sound(0x3f);
            bus.wb(SCENE_MODE, 0);
            stage_step(scheduler, bus, io, at)
        }
        Flow::BonusJapanFailPose { copy } => {
            // 0x4123a..0x41252 (Copy 2: 0x41eee..0x41f06): queue voice 0x8d,
            // scroll fg to 0x1500, and sleep 60 frames (0x3b562 into 0x41258 /
            // 0x41f0c).
            bus.queue_sound(0x8d);
            bus.ww(0xffc680, 0);
            add_fg_scroll(bus, 0x1500);
            *at = Flow::BonusJapanFailLaugh1 { copy };
            Yield::Sleep(60)
        }
        Flow::BonusJapanFailLaugh1 { copy } => {
            // 0x41258..0x41270 (Copy 2: 0x41f0c..0x41f24): scroll bg to 0x1200,
            // queue sound 0x9f, and sleep 60 frames (0x3b562 into 0x41276 /
            // 0x41f2a).
            bus.ww(0xffc682, 0);
            add_bg_scroll(bus, 0x1200);
            bus.queue_sound(0x9f);
            *at = Flow::BonusJapanFailLaugh2 { copy };
            Yield::Sleep(60)
        }
        Flow::BonusJapanFailLaugh2 { copy } => {
            // 0x41276..0x41280 (Copy 2: 0x41f2a..0x41f34): queue sound 0x9f and
            // sleep 60 frames (0x3b562 into 0x41286 / 0x41f3a).
            bus.queue_sound(0x9f);
            *at = Flow::BonusJapanFailLaugh3 { copy };
            Yield::Sleep(60)
        }
        Flow::BonusJapanFailLaugh3 { copy } => {
            // 0x41286..0x41290 (Copy 2: 0x41f3a..0x41f44): queue sound 0x9f and
            // sleep 180 frames (0x3b572 into 0x41296 / 0x41f4a).
            bus.queue_sound(0x9f);
            *at = Flow::BonusJapanFailEnd { copy };
            Yield::Sleep(180)
        }
        Flow::BonusJapanFailEnd { .. } => {
            // 0x41296..0x412a4 (Copy 2: 0x41f4a..0x41f58): stop sound (0x3f),
            // clear SCENE_MODE (0xffc008), and advance stage at 0x3efc4.
            bus.queue_sound(0x3f);
            bus.wb(SCENE_MODE, 0);
            stage_step(scheduler, bus, io, at)
        }
        Flow::BonusStage1Palette1 { copy } => {
            // 0x41814..0x4182c (Copy 2: 0x420cc..0x420e4): request palette
            // 0x1a268 at offset 0x200 and sleep 1 frame (0x3b51a into
            // 0x4182e / 0x420e6).
            request_palette(bus, 0x1a268, 0x200);
            *at = Flow::BonusStage1Palette2 { copy };
            Yield::Sleep(1)
        }
        Flow::BonusStage1Palette2 { copy } => {
            // 0x4182e..0x41854 (Copy 2: 0x420e6..0x4210c): turn on layers 0xe0
            // (0x3b644), queue tune 0x3b, draw BREAK THE OBJECTS (0x14802, D1 =
            // 4 into 0x41868 / 0x42120), and sleep 12 frames (0x3b53a into
            // 0x4186e / 0x42126).
            gfx_on(bus, 0xe0);
            bus.queue_sound(0x3b);
            text_block(bus, 0x14802);
            *at = Flow::BonusStage1BlinkErase { copy, d1: 4 };
            Yield::Sleep(12)
        }
        Flow::BonusStage1BlinkDraw { copy, d1 } => {
            // 0x4185c..0x4186c (Copy 2: 0x42114..0x42124): draw BREAK THE
            // OBJECTS (0x14802) and sleep 12 frames (0x3b53a into 0x4186e /
            // 0x42126).
            text_block(bus, 0x14802);
            *at = Flow::BonusStage1BlinkErase { copy, d1 };
            Yield::Sleep(12)
        }
        Flow::BonusStage1BlinkErase { copy, d1 } => {
            // 0x4186e..0x418aa (Copy 2: 0x42126..0x42162): erase BREAK THE
            // OBJECTS (0x14802); while D1 > 0 sleep 12 frames (0x3b53a at
            // 0x41856 / 0x4210e into 0x4185c / 0x42114); when D1 == 0, erase
            // 0x1484e, clear 4 words at 0xff8f18 (0x3b934), set SCENE_MODE |=
            // 0x80, arm timer 3 for 10.00 (0xff8e16 = 0x20), reset button
            // latches (0x3b6c6), and sleep 1 frame (0x3b51a into 0x418ac /
            // 0x42164).
            erase_block(bus, 0x14802);
            if d1 != 0 {
                *at = Flow::BonusStage1BlinkDraw { copy, d1: d1 - 1 };
                return Yield::Sleep(12);
            }
            erase_block(bus, 0x1484e);
            bus.clear_words_4(0xff8f18);
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) | 0x80);
            bus.wb(TIMER_FLAGS, 0x20);
            reset_button_latches(bus);
            *at = Flow::BonusStage1Start { copy };
            Yield::Sleep(1)
        }
        Flow::BonusStage1Start { copy } => bonus_stage1_frame(bus, io, at, copy),
        Flow::BonusStage1Loop { copy } => {
            // 0x4191c..0x41934 (Copy 2: 0x421d4..0x421ec): compare
            // 0xff8f1c..0xff8f1e against 0x41936 (05 00 00, 5,000 bonus points
            // = 3 stacks broken) or 0x421ee (08 00 00, 8,000 bonus points = 4
            // stacks broken) via 0x3baf6; if equal, settle at 0x41950 /
            // 0x42208, otherwise loop to 0x418ac / 0x42164.
            if bus.compare_bytes_3(0xff8f1c, copy.max_bonus_addr()) == std::cmp::Ordering::Equal {
                bonus_stage1_settle_step(bus, io, at, copy)
            } else {
                bonus_stage1_frame(bus, io, at, copy)
            }
        }
        Flow::BonusStage1Settle { copy } => {
            // 0x41962..0x4199e (Copy 2: 0x4221a..0x42256): select the active
            // player structure (0x4196c / 0x42224 P1 0xff86b0 in mode 2;
            // 0x41974 / 0x4222c P2 0xff86da in mode 4, mode 4 unexercised) and
            // check whether its fighter's move is still in progress (+9 bit 7);
            // if so, loop back to 0x41950 / 0x42208; otherwise start the
            // 10-frame post-settle drain (0x4198a..0x4199e / 0x42242..0x42256).
            let ps = if bus.rb(GAME_MODE) == 4 {
                0xff86da
            } else {
                0xff86b0
            };
            let me = bus.rw(ps + 0x10);
            if bus.rb(0xffe000 + me as u32 + 9) & 0x80 != 0 {
                return bonus_stage1_settle_step(bus, io, at, copy);
            }
            for ps in bus.dispatch_order() {
                bus.run_player(ps);
            }
            bus.run_effects();
            *at = Flow::BonusStage1After { copy, left: 10 };
            speed_sleep(bus, io)
        }
        Flow::BonusStage1After { copy, left } => {
            // 0x419a0..0x419b2 (Copy 2: 0x42258..0x4226a): pop and decrement
            // the 10-frame counter; while nonzero, push it back and run
            // 0x4198e..0x4199e / 0x42246..0x42256; at zero, sleep 120 frames
            // (0x3b56a at 0x419ae / 0x42266 into 0x419b4 / 0x4226c).
            let left = left - 1;
            if left != 0 {
                for ps in bus.dispatch_order() {
                    bus.run_player(ps);
                }
                bus.run_effects();
                *at = Flow::BonusStage1After { copy, left };
                return speed_sleep(bus, io);
            }
            *at = Flow::BonusStage1Hold1 { copy };
            Yield::Sleep(bus.rw(0x3b56c) as u8)
        }
        Flow::BonusStage1Hold1 { copy } => {
            // 0x419b4..0x419ec (Copy 2: 0x4226c..0x422a4): erase TIME OVER
            // (0x14f74), draw BONUS PTS. (0x14812), draw the 4 bonus digits
            // from 0xff8f1c at 0x8005bc (attr 0x8000, D1 = 3 via 0x3bdf2), and
            // sleep 120 frames (0x3b56a into 0x419ee / 0x422a6).
            erase_block(bus, 0x14f74);
            text_block(bus, 0x14812);
            draw_digits(bus, 0xff8f1c, 0x8005bc, 0x8000, 3);
            *at = Flow::BonusStage1Hold2 { copy };
            Yield::Sleep(bus.rw(0x3b56c) as u8)
        }
        Flow::BonusStage1Hold2 { .. } => {
            // 0x419ee..0x419fc (Copy 2: 0x422a6..0x422b4): stop sound (0x3f),
            // clear SCENE_MODE (0xffc008), and advance stage at 0x3efc4.
            bus.queue_sound(0x3f);
            bus.wb(SCENE_MODE, 0);
            stage_step(scheduler, bus, io, at)
        }
        Flow::EndingStart => {
            // 0x44a60..0x44aa8: turn on bg + tx (0x28 via 0x3b644) and start
            // the five-scene ending loop (d1 = 0..=4, d2 = 4..=0).
            gfx_on(bus, 0x28);
            ending_scene_outer(bus, at, 0)
        }
        Flow::EndingSceneHold { d1, d4, d5 } => {
            // 0x44ae8..0x44aff: erase the current 2x2 country block
            // *(0x44ca4 + d5) via 0x3bd3e; if d4 != 0 (dbra D4, $44ab0), run
            // the second sub-step of scene `d1`; otherwise advance `d1`
            // (dbra D2, $44a72) or enter 0x44b00.
            let (block, stand_in) = ending_montage_card(bus, d5);
            erase_block_2x2(bus, block);
            if d4 != 0 {
                let scene = ending_scene_address(bus, d1);
                if stand_in {
                    request_ending_scene_palette(bus, scene);
                }
                let byte = bus.rb(scene + 2);
                return ending_scene_step(bus, at, d1, 0, byte);
            }
            if d1 < 4 {
                return ending_scene_outer(bus, at, d1 + 1);
            }
            ending_message_start(bus, at)
        }
        Flow::EndingMessageSetup => {
            // 0x44b20..0x44b70: turn on bg + tx (0x28), request palette 0x1b868
            // at offset 0, scroll bg to 0x44d5c (0x7700 when GAME_MODE == 2 at
            // 0x44b48, else 0x44d5e = 0x7880 at 0x44b52, unexercised), set
            // TEXT_SOUND = 0x000d, and start typing 0x44d60 with delay D4 = 5
            // (0x3bb98).
            gfx_on(bus, 0x28);
            bus.ww(0xffc682, 0);
            let block = if let Some((view, block)) = boss_ending(bus) {
                request_palette(bus, view.palette, 0);
                add_bg_scroll(bus, view.scroll);
                *bus.bg_page = view.page;
                block
            } else {
                request_palette(bus, 0x1b868, 0);
                let ken = if let Some(fighter) = ending_player(bus) {
                    fighter == TournamentFighter::Ken
                } else {
                    bus.rb(GAME_MODE) != 2
                };
                add_bg_scroll(bus, bus.rw(if ken { 0x44d5e } else { 0x44d5c }));
                0x44d60
            };
            bus.ww(TEXT_SOUND, 0x000d);
            let writer = TextWriter::new(bus, block, 5);
            ending_message_type(bus, at, writer)
        }
        Flow::EndingMessageTyping { writer } => ending_message_type(bus, at, writer),
        Flow::EndingMessageHold => {
            // 0x44b78..0x44b86: erase 0x44d60 (0x3bd00) and sleep 1 frame
            // (0x3b51a into 0x44b88).
            let block = boss_ending(bus).map_or(0x44d60, |(_, block)| block);
            erase_block(bus, block);
            *at = Flow::EndingStaffSetup;
            Yield::Sleep(1)
        }
        Flow::EndingStaffSetup => {
            // 0x44b88..0x44ba6: queue staff tune 0x36, turn off layers 0xe8,
            // clear the text layer, and sleep 1 frame (0x3b51a into 0x44ba8).
            bus.queue_sound(0x36);
            gfx_off(bus, 0xe8);
            clear_text_layer(bus);
            *at = Flow::EndingStaffHeader;
            Yield::Sleep(1)
        }
        Flow::EndingStaffHeader => {
            // 0x44ba8..0x44bc6: turn on tx (0x08), draw the 2x2 staff header
            // *(0x44f58) (0x44f80) via 0x3bc3e, and sleep 2 frames (0x3b522
            // into 0x44bc8).
            gfx_on(bus, 0x08);
            let header = bus.rl(0x44f58);
            text_block_2x2(bus, header);
            *at = Flow::EndingStaffStart;
            Yield::Sleep(bus.rw(0x3b524) as u8)
        }
        Flow::EndingStaffStart => ending_staff_block(bus, at, 7),
        Flow::EndingStaffTyping { d1, writer } => ending_staff_type(bus, at, d1, writer),
        Flow::EndingStaffHold { d1 } => {
            // 0x44bda..0x44c02: erase the current staff block *(0x44f5c + 4 *
            // (7 - d1)) via 0x3bd00; while d1 > 0 (dbra D1, $44bcc), start the
            // next staff block; when d1 == 0, erase the 2x2 header *(0x44f58)
            // (0x3bd3e), draw the 2x2 closing block *(0x44f7c) (0x452cc,
            // 0x3bc3e), and hold 120 frames (0x3b56a into 0x44c04).
            let a2 = 0x44f5c + 4 * (7 - d1) as u32;
            erase_block(bus, bus.rl(a2));
            if d1 != 0 {
                return ending_staff_block(bus, at, d1 - 1);
            }
            erase_block_2x2(bus, bus.rl(0x44f58));
            text_block_2x2(bus, bus.rl(0x44f7c));
            *at = Flow::EndingFinalHold;
            Yield::Sleep(bus.rw(0x3b56c) as u8)
        }
        Flow::EndingFinalHold => {
            // 0x44c04..0x44c1a: erase the 2x2 closing block *(0x44f7c)
            // (0x3bd3e), queue the boot/stop commands (0x3b7a8: 0x27, 0x3f,
            // 0x80), and jump to 0x3efc4 (stage_step, advancing STAGE_NO from
            // 4 to 5 into 0x3f2d0).
            erase_block_2x2(bus, bus.rl(0x44f7c));
            queue_boot_sounds(bus);
            if bus.world_tour_rules().is_some() {
                bus.wb(MODE_DEMO, 0);
                return Yield::Exit;
            }
            stage_step(scheduler, bus, io, at)
        }
        Flow::KoStart => {
            // 0x3eb28: the fight is over: scene mode 0, the TIME HUD drawn
            // once more and frozen in its copy at 0xff8e3c.
            bus.wb(SCENE_MODE, 0);
            draw_time_digits(bus);
            for i in 0..3 {
                bus.wb(0xff8e3c + i, bus.rb(TIME_DIGITS + i));
            }
            *at = Flow::KoSettle;
            settle_frame(bus, io)
        }
        Flow::KoSettle => {
            if fighters_busy(bus) {
                return settle_frame(bus, io);
            }
            // 0x3eb78: the winner. A fighter that is down leaves the win to
            // its opponent (+0x3c), unless that one is down too (a double
            // KO, 0x3eca2); one standing is the winner (+0x38). Either
            // fighter gives the same answer.
            let (winner, double_ko) = if bus.dramatic_rules().is_some() {
                let [ally1, boss, ally2] = bus.toggle_state.dramatic.health;
                let team_down = ally1 == 0 && ally2 == 0;
                (if team_down { CPU_STRUCT } else { P1_STRUCT }, team_down && boss == 0)
            } else if bus.rb(0xffe408) == 8 {
                let opponent = bus.rl(0xffe43c);
                let e = bus.rw(opponent + 0x10);
                (opponent, bus.rb(0xffe000 + e as u32 + 8) == 8)
            } else {
                (bus.rl(0xffe438), false)
            };
            if double_ko {
                return double_ko_start(bus, io, at);
            }
            record_win(scheduler, bus, io, at, winner)
        }
        Flow::Victory => {
            if fighters_busy(bus) {
                return settle_frame(bus, io);
            }
            *at = Flow::AwaitResult;
            speed_sleep(bus, io)
        }
        Flow::AwaitResult => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            let winner = bus.rl(0xff8e38);
            let wins_needed = if bus.world_tour_rules().is_some_and(|rules| rules.single_round()) { 1 } else { 2 };
            if bus.rb(winner + 8) == wins_needed {
                // 0x3ec70: the match is won (0xffc009 = 1 P1, 2 P2, 4 CPU).
                let who = match winner {
                    0xff86b0 => 1,
                    0xff86da => 2,
                    _ => 4,
                };
                bus.wb(0xffc009, who);
                return match_end(bus, io, at);
            }
            after_round(bus, io, at)
        }
        Flow::TimeOverStart => {
            // 0x3ed26: the fight is over: scene mode 0, TIME OVER (0x14f74)
            // shown, the TIME digits drawn, sound 0x96; then the hold's
            // counter 0x82 is counted down before each sleep.
            bus.wb(SCENE_MODE, 0);
            text_block(bus, 0x14f74);
            draw_time_digits(bus);
            bus.queue_sound(0x96);
            time_over_hold(scheduler, bus, io, at, 0x82)
        }
        Flow::TimeOverHold { left } => time_over_hold(scheduler, bus, io, at, left),
        Flow::AwaitDraw => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            after_round(bus, io, at)
        }
        Flow::DoubleKoSettle => {
            // 0x3ecca: while either fighter is still busy the loop goes
            // round again (0x3eca2); then DOUBLE K.O. (0x14f56) is shown
            // and held.
            if fighters_busy(bus) {
                return double_ko_start(bus, io, at);
            }
            text_block(bus, 0x14f56);
            double_ko_hold(scheduler, bus, io, at, 0x82)
        }
        Flow::DoubleKoHold { left } => double_ko_hold(scheduler, bus, io, at, left),
        Flow::AwaitGameOverWipe => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            versus_game_over(bus, at)
        }
        Flow::GameOverCycle {
            index,
            effect_ticks,
            entries_left,
        } => game_over_cycle(bus, at, index, effect_ticks, entries_left),
        Flow::GameOverDrain => game_over_drain(bus, at),
        Flow::GameOverAwaitEvent => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return Yield::Sleep(1);
            }
            game_over_reset(scheduler, bus, io, at)
        }
        Flow::GameOverPalette => {
            gfx_on(bus, 0x40);
            *at = Flow::GameOverHold;
            Yield::Sleep(bus.rw(0x3b57c) as u8)
        }
        Flow::GameOverHold => game_over_reset(scheduler, bus, io, at),
        Flow::StageStart => {
            // 0x3e59c..0x3e5d0: clear all object regions; in demo mode
            // (0x3e5c6..0x3e5d0, ticket #88, F48) turn off layers 0xe0
            // (0x3b690) and request the country palette (0x436b8, with its
            // speed_sleep) into 0x3e5d6; otherwise request event 2 and wait at
            // 0x422e0.
            clear_objects(bus);
            if bus.rb(MODE_DEMO) == 0 {
                gfx_off(bus, 0xe0);
                request_country_palette(bus);
                *at = Flow::DemoCountryPalette;
                return speed_sleep(bus, io);
            }
            request_event(bus, 2);
            *at = Flow::AwaitCountryScreen;
            speed_sleep(bus, io)
        }
        Flow::DemoCountryPalette => {
            // 0x3e5d6..0x3e6fa: after 0x436b8 returns to 0x3e5d6 in demo mode,
            // place the players' HUD and entity offsets and advance into 0x3e6ca.
            place_players(bus);
            next_opponent(bus, io, at)
        }
        Flow::DemoStagePalette => {
            // 0x3e704..0x3e712: after 0x43748 returns to 0x3e704 in demo mode,
            // reset the stage scroll/camera (0x437d8), turn on layers 0xe8
            // (0x3b644), and enter 0x3e714 directly without events 4 and 5.
            stage_reset(bus);
            gfx_on(bus, 0xe8);
            *at = Flow::AwaitAnnounce { new_opponent: true };
            flow(scheduler, bus, io, at)
        }
        Flow::AwaitCountryScreen => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            // 0x3e5d6..0x3e6fa: place the players' HUD and entity offsets by
            // game mode, then advance into 0x3e6ca.
            place_players(bus);
            next_opponent(bus, io, at)
        }
        Flow::AwaitWinScreen { versus } => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            // 0x3ef68 (0x3ee70 / 0x3eea2 in versus): the wipe (event 6).
            request_event(bus, 6);
            *at = Flow::AwaitMatchWipe { versus };
            speed_sleep(bus, io)
        }
        Flow::AwaitMatchWipe { versus: true } => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            // 0x3ee7a / 0x3eeac: the versus winner goes on against the CPU
            // in a one-player mode (2 for P1, 4 for P2; the scene byte 1 /
            // 2); the departing player's ranking check and name entry
            // (0x42422) and carry-over setup (0x3eec2..0x3eeec, F26) follow.
            let (mode, scene, departing) = if bus.rb(0xffc009) == 1 {
                (2, 1, 0xff86da)
            } else {
                (4, 2, 0xff86b0)
            };
            bus.wb(GAME_MODE, mode);
            bus.wb(SCENE_MODE, scene);
            if let Some(y) = ranking_check(bus, departing, RankingReturn::VersusCarry, at) {
                return y;
            }
            versus_carry_after_ranking(bus, io, at)
        }
        Flow::VersusCarryStagePalette => {
            // 0x3eee6: country palette, then its sleep.
            request_country_palette(bus);
            *at = Flow::VersusCarryCountryPalette;
            speed_sleep(bus, io)
        }
        Flow::VersusCarryCountryPalette => {
            // 0x3eeec -> 0x3e5d6: no country-event wait on this entry.
            place_players(bus);
            next_opponent(bus, io, at)
        }
        Flow::AwaitCpuWinScreen => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            // 0x3ef04 / 0x3ef36: request the wipe (event 6).
            request_event(bus, 6);
            *at = Flow::AwaitCpuMatchWipe;
            speed_sleep(bus, io)
        }
        Flow::AwaitCpuMatchWipe => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            // 0x3ef0e..0x3ef1e (mode 2) / 0x3ef40..0x3ef50 (mode 4,
            // unverified in F5): set the scene byte, run the departing
            // player's ranking check (0x42422), and enter the continue
            // screen (0x3efda).
            let (ps, scene) = if bus.rb(GAME_MODE) == 2 {
                (0xff86b0, 2)
            } else {
                (0xff86da, 1)
            };
            bus.wb(SCENE_MODE, scene);
            if let Some(y) = ranking_check(bus, ps, RankingReturn::CpuMatchLose, at) {
                return y;
            }
            continue_start(bus, at)
        }
        Flow::ContinuePalette1 => {
            // 0x3f04c..0x3f060: request palette 0x1ae68 at offset 0 and
            // sleep 1 frame.
            request_palette(bus, 0x1ae68, 0);
            *at = Flow::ContinuePalette2;
            Yield::Sleep(1)
        }
        Flow::ContinuePalette2 => {
            // 0x3f066..0x3f08c: turn on bg + fg (0x60), start timer counter 1
            // (0x3f070: move.b #2, ($e16,A5)), queue sound 0x3c, and sleep 1
            // frame before the continue loop (0x3f092).
            gfx_on(bus, 0x60);
            let flag = bus.rb(0x3f073);
            bus.wb(TIMER_FLAGS, flag);
            bus.queue_sound(0x3c);
            *at = Flow::ContinueLoop;
            Yield::Sleep(1)
        }
        Flow::ContinueLoop => continue_frame(bus, io, at),
        Flow::ContinueAccept => {
            // 0x3f172..0x3f1ac: erase CONTINUE (0x15b04) and the big and
            // fractional countdown digits, clear the countdown timer word
            // (0xff8e1e), take one credit (0x3bb74 at 0xff865e), and request
            // the country palette (0x436b8, with its sleep).
            erase_block(bus, 0x15b04);
            for offset in [0x738, 0x73c, 0x7b8, 0x7bc, 0x7c4] {
                bus.wl(0x800000 + offset, 0x003c003c);
            }
            bus.wl(0xff8e1e, 0);
            let credit_end = bus.rw(0x3f1a4);
            if credit_end != 0 {
                decimal_down(bus, 0xff8000 + credit_end as u32);
            }
            request_country_palette(bus);
            *at = Flow::ContinueCountryPalette;
            speed_sleep(bus, io)
        }
        Flow::ContinueCountryPalette => {
            // 0x3f1b2: request the stage palette (0x43748) and sleep.
            request_stage_palette(bus);
            *at = Flow::ContinueStagePalette;
            speed_sleep(bus, io)
        }
        Flow::ContinueStagePalette => {
            // 0x3f1b8: bra $3e6ca (re-enter next_opponent against the same
            // stage/fight opponent).
            next_opponent(bus, io, at)
        }
        Flow::AwaitMatchWipe { versus: false } => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            // 0x3ef72: sound 0x3f; the round counter and the winner
            // cleared, the fight counter up. A country's second fight won
            // enters 0x3ef96: clear the counters again and dispatch through
            // its per-stage table (0x3efb0). The bonus stages themselves
            // remain boundaries until their own tickets are implemented.
            bus.queue_sound(0x3f);
            bus.wb(0xffc00a, 0);
            bus.wb(0xffc004, bus.rb(0xffc004).wrapping_add(1));
            bus.wb(0xffc009, 0);
            if bus.rb(0xffc004) == 2 {
                // 0x3ef96..0x3efc2: clear fight, round and winner bytes and
                // jump through the per-stage table at 0x3efb0. Stage 0 enters
                // the Japan bonus stage at 0x40d30 (ticket #69, F29); stages
                // 1..=4 remain boundaries until their own tickets land.
                bus.wb(0xffc004, 0);
                bus.wb(0xffc00a, 0);
                bus.wb(0xffc009, 0);
                let stage = bus.rb(0xffc003) as i8 as i32;
                let target = bus.rl((0x3efb0i32 + stage * 4) as u32);
                if target == 0x40d30 {
                    return bonus_japan_start(bus, at, BonusCopy::Japan);
                }
                if target == 0x41740 {
                    return bonus_stage1_start(bus, at, Bonus10Copy::Stage1);
                }
                if target == 0x41a02 {
                    return bonus_japan_start(bus, at, BonusCopy::Stage2);
                }
                if target == 0x41fe6 {
                    return bonus_stage1_start(bus, at, Bonus10Copy::Stage3);
                }
                if target == 0x44a40 {
                    return ending_start(bus, at);
                }
                bus.note(target);
                return Yield::Halt;
            }
            next_opponent(bus, io, at)
        }
        Flow::AwaitOpponentIntro => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            // 0x3e6ea: the stage position, then the round announcement.
            stage_reset(bus);
            request_event(bus, 5);
            *at = Flow::AwaitAnnounce { new_opponent: true };
            speed_sleep(bus, io)
        }
        Flow::AwaitWipe => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            // 0x3ec58: clear the object slots, then the round
            // initialisation (0x3e77c): the stage position, and the round
            // announcement (event 5).
            start_satsui_awakening(bus);
            clear_objects(bus);
            stage_reset(bus);
            request_event(bus, 5);
            *at = Flow::AwaitAnnounce {
                new_opponent: false,
            };
            speed_sleep(bus, io)
        }
        Flow::AwaitAnnounce { new_opponent } => {
            if bus.rb(SCENE_MODE) & 0x20 != 0 {
                return speed_sleep(bus, io);
            }
            if new_opponent {
                // 0x3e714: in the one-player modes the new opponent's move
                // table and AI tables (0x44860 / 0x44880); both fighters'
                // wins cleared.
                let (a, b) = match bus.rb(GAME_MODE) {
                    2 => {
                        bus.setup_cpu_opponent();
                        (0xff86b0, 0xffc664)
                    }
                    4 => {
                        bus.setup_cpu_opponent();
                        (0xff86da, 0xffc664)
                    }
                    _ => (0xff86b0, 0xff86da),
                };
                bus.ww(a + 8, 0);
                bus.ww(b + 8, 0);
            }
            if !round_hud(bus) {
                return Yield::Halt;
            }
            // 0x3e960: the stage music, then the input handlers once.
            stage_music(bus);
            bus.wb(FRAME_PENDING, 0xff);
            bus.drive_fight_inputs();
            *at = Flow::Intro;
            intro_frame(bus, io, at)
        }
        Flow::Intro => intro_frame(bus, io, at),
        Flow::FightShown { left } => {
            // 0x3e9fe: count the wait down; at its end erase FIGHT! and
            // request the round timer's reload (0x3ea10..0x3ea38).
            let left = left - 1;
            if left != 0 {
                *at = Flow::FightShown { left };
                return speed_sleep(bus, io);
            }
            erase_block(bus, 0x14f44);
            bus.wb(TIMER_FLAGS, bus.rb(TIMER_FLAGS) | 1);
            *at = Flow::RoundStart;
            speed_sleep(bus, io)
        }
        Flow::AttractJoin => {
            // 0x3ccae..0x3ccbc, 0x3ccc2..0x3cce6: spawn slot 3 at 0x3f374
            // (0x3b20a) and enter the attract loop head at 0x3ccc2.
            scheduler.spawn(bus, 3, 0x3f374);
            attract_loop_head(bus, at)
        }
        Flow::AttractStart => {
            // 0x3ccec..0x3cd10: request palette 0x1ae68 at offset 0, turn on
            // tx/bg/fg (0x68), and start the ten-entry scroll loop (0x3cde0).
            request_palette(bus, 0x1ae68, 0);
            gfx_on(bus, 0x68);
            attract_scroll_loop(bus, at, 0)
        }
        Flow::AttractScroll { d1 } => attract_scroll_loop(bus, at, d1 + 1),
        Flow::AttractTitleBlock { d1 } => {
            // 0x3cd86..0x3cd94: erase this title block; if more remain in
            // 0x3cdc8, draw the next; otherwise draw the title logo 0x14afc
            // and hold 60 frames (0x3cd96..0x3cda2).
            let block = bus.rl(0x3cdc8 + ((d1 as u32) << 3));
            erase_block(bus, block);
            if d1 + 1 < 3 {
                return attract_title_block(bus, at, d1 + 1);
            }
            text_block(bus, 0x14afc);
            *at = Flow::AttractLogoHold;
            Yield::Sleep(0x3c)
        }
        Flow::AttractLogoHold => {
            // 0x3cda8..0x3cdb2: turn off bg/fg (0x60), then enter the
            // 300-frame nested sleep 0x3b582 (first 240 frames at 0x3b57a).
            gfx_off(bus, 0x60);
            *at = Flow::AttractWait300 {
                second_sleep: false,
            };
            Yield::Sleep(0xf0)
        }
        Flow::AttractWait300 {
            second_sleep: false,
        } => {
            // 0x3b588: bra $3b562 (second 60 frames of 0x3b582).
            *at = Flow::AttractWait300 { second_sleep: true };
            Yield::Sleep(0x3c)
        }
        Flow::AttractWait300 { second_sleep: true } => {
            // 0x3cdb8..0x3cdc4, 0x3ce30..0x3ce4e: erase the logo 0x14afc,
            // turn on bg (0x20), request palette 0x1b868 at offset 0, and
            // call the ranking screen 0x4268c (ticket #56, F16).
            erase_block(bus, 0x14afc);
            gfx_on(bus, 0x20);
            request_palette(bus, 0x1b868, 0);
            ranking_screen_start(bus, at, RankingReturn::Attract)
        }
        Flow::AttractRanking => attract_ranking_wait(scheduler, bus, io, at),
        Flow::StartScreenEntry => start_screen_entry(scheduler, bus, io, at),
        Flow::StartScreen => start_screen_frame(scheduler, bus, io, at),
        Flow::CountrySelectArm => country_select_arm(bus, at),
        Flow::CountrySelectLoop { .. } => country_select_frame(scheduler, bus, io, at),
        Flow::CountrySelect4Arm => country_select4_arm(bus, at),
        Flow::CountrySelect4Loop { first, d6, d7 } => {
            country_select4_frame(scheduler, bus, io, at, first, d6, d7)
        }
    }
}

/// 0x3dec2..0x3df48: enter the start screen — set `0xff8003 = 0xff`
/// (`MODE_DEMO`), clear all object and fighter regions (`0x3b9bc / 0x3b9c8 /
/// 0x3b9d4`), queue stop commands `0x3f` and `0x80` (`0x3b7c8`), clear
/// `0xffc00b`, `0xffc002` (`GAME_MODE`), `0xffc008` (`SCENE_MODE`) and
/// `0xff8e36`, turn off layers `0xe8` (`0x3b690`) and turn on `0x08`
/// (`0x3b644`), clear the text layer (`0x3bdb0`), draw the five start-screen
/// text blocks from `0x3e024` (when `0xff865a <= 1`) or `0x3e038` (when
/// `0xff865a > 1`) through `0x3bba0`, clear the blink counters `0xffc01a` and
/// `0xffc01b`, and fall through into `0x3df4c` (`start_screen_frame`).
fn start_screen_entry(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    bus.wb(MODE_DEMO, 0xff);
    clear_objects(bus);
    bus.queue_sound(0x3f);
    bus.queue_sound(0x80);
    bus.wb(0xffc00b, 0);
    bus.wb(GAME_MODE, 0);
    bus.wb(0xffc008, 0);
    bus.wb(0xff8e36, 0);
    gfx_off(bus, 0xe8);
    gfx_on(bus, 0x08);
    clear_text_layer(bus);
    let table = if bus.rl(CREDITS_LONG) > 1 {
        0x3e038
    } else {
        0x3e024
    };
    for i in 0..5u32 {
        let block = bus.rl(table + 4 * i);
        text_block(bus, block);
    }
    bus.wb(0xffc01a, 0);
    bus.wb(0xffc01b, 0);
    *at = Flow::StartScreen;
    start_screen_frame(scheduler, bus, io, at)
}

/// 0x4268c..0x42730, 0x42750..0x42756: set up the ranking screen — clear
/// 0xffc682, add 0x7580 to bg scroll (0x3b682), clear the text layer (0x3bdb0),
/// draw the ranking header block 0x14682 (0x3bba0), queue tune 0x39 (0x3b7c8),
/// draw the 10 entries' 8-digit scores (+5 at video offset +0) and 3-char
/// initials (+13 at video offset +2) from 0xff8e6e through 0x3bdf2
/// (`draw_digits`), check for a newly entered marker (+4 == 0xff -> 0x42750,
/// ticket #122, F50), arm timer 3 (`0xff8e16 = 8`), and sleep 1 frame into
/// 0x42736 (unmarked) or 0x4275c (marked).
fn ranking_screen_start(bus: &mut Bus, at: &mut Flow, ret: RankingReturn) -> Yield {
    bus.ww(0xffc682, 0);
    add_bg_scroll(bus, 0x7580);
    clear_text_layer(bus);
    text_block(bus, 0x14682);
    bus.queue_sound(0x39);
    for d2 in 0..10u32 {
        let entry = 0xff8e6e + (d2 << 4);
        let score_off = bus.rw(entry) as i16 as i32 as u32;
        draw_digits(
            bus,
            entry + 5,
            0x800000u32.wrapping_add(score_off),
            0x9000,
            7,
        );
        let name_off = bus.rw(entry + 2) as i16 as i32 as u32;
        draw_digits(
            bus,
            entry + 13,
            0x800000u32.wrapping_add(name_off),
            0x9000,
            2,
        );
    }
    for d2 in 0..10u32 {
        let entry = 0xff8e6e + (d2 << 4);
        if bus.rb(entry + 4) == 0xff {
            bus.wb(TIMER_FLAGS, 0x08);
            *at = Flow::RankingMarked { ret };
            return Yield::Sleep(1);
        }
    }
    bus.wb(TIMER_FLAGS, 0x08);
    *at = Flow::AttractRanking;
    Yield::Sleep(1)
}

/// 0x4275c..0x427e0, 0x4267a..0x4268a: one pass of the marked-entry ranking
/// blink loop (ticket #122, F50): when timer-3 sub-counter `0xff8e2c` is
/// `0x0909`, draw the marked entry's 8-digit score (attr `0x1000`, `D1 = 7`)
/// and 3-char initials (attr `0xa000`, `D1 = 2`); when `0xff8e2c` is `0x0303`,
/// blank the score cells via `0x3b914` (`fill_words_16`, 16 words of `0x003c`)
/// and the name cells via `0x3b904` (`fill_words_4`, 4 words of `0x003c`);
/// sleep 1 frame while `0xff8e2f == 0`; when timer 3 expires (`0xff8e2f != 0`),
/// queue `0x3f`, clear the text layer (`0x3bdb0`), clear `SCENE_MODE`
/// (`0xffc008`) and the entry marker (`a2 + 4`), and return to the caller of
/// `0x42422`.
fn ranking_marked_wait(
    scheduler: &mut Scheduler,
    bus: &mut Bus,
    io: &Io,
    at: &mut Flow,
    ret: RankingReturn,
) -> Yield {
    let mut a3 = bus.rl(0xff8e66);
    for d2 in 0..10u32 {
        let entry = 0xff8e6e + (d2 << 4);
        if bus.rb(entry + 4) == 0xff {
            a3 = entry;
            break;
        }
    }
    if bus.rw(0xff8e2c) == 0x0909 {
        let score_off = bus.rw(a3) as i16 as i32 as u32;
        draw_digits(bus, a3 + 5, 0x800000u32.wrapping_add(score_off), 0x1000, 7);
        let name_off = bus.rw(a3 + 2) as i16 as i32 as u32;
        draw_digits(bus, a3 + 13, 0x800000u32.wrapping_add(name_off), 0xa000, 2);
    }
    if bus.rw(0xff8e2c) == 0x0303 {
        let score_off = bus.rw(a3) as i16 as i32 as u32;
        fill_words_16(bus, 0x800000u32.wrapping_add(score_off), 0x003c);
        let name_off = bus.rw(a3 + 2) as i16 as i32 as u32;
        fill_words_4(bus, 0x800000u32.wrapping_add(name_off), 0x003c);
    }
    if bus.rb(0xff8e2f) == 0 {
        *at = Flow::RankingMarked { ret };
        return Yield::Sleep(1);
    }
    bus.queue_sound(0x3f);
    clear_text_layer(bus);
    if ret == RankingReturn::Attract {
        return attract_demo_setup(scheduler, bus, io, at);
    }
    let a2 = bus.rl(0xff8e66);
    bus.wb(SCENE_MODE, 0);
    bus.wb(a2 + 4, 0);
    match ret {
        RankingReturn::VersusCarry => versus_carry_after_ranking(bus, io, at),
        RankingReturn::CpuMatchLose => continue_start(bus, at),
        RankingReturn::GameOverP1 => versus_game_over_p2(bus, at),
        RankingReturn::GameOverP2 => game_over_screen(bus, at),
        RankingReturn::Attract => unreachable!(),
    }
}

/// 0x3eec2..0x3eee0: clear round/winner and every object, clear `SCENE_MODE`,
/// and request the stage palette before the first carry-over sleep.
fn versus_carry_after_ranking(bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    bus.wb(0xffc00a, 0);
    bus.wb(0xffc009, 0);
    clear_objects(bus);
    bus.wb(SCENE_MODE, 0);
    request_stage_palette(bus);
    *at = Flow::VersusCarryStagePalette;
    speed_sleep(bus, io)
}

/// 0x3ce54..0x3ce86: attract demo setup after the attract ranking screen.
fn attract_demo_setup(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    gfx_off(bus, 0x28);
    bus.wb(0xff8f28, bus.rb(0xffc007));
    bus.ww(0xff8f2a, 0);
    bus.wb(0xffc007, bus.rb(0x3cebe));
    bus.wb(0xffc00b, 0);
    bus.wb(0xffc006, 0);
    bus.wb(0xffc003, 0);
    bus.wb(0xffc004, 0);
    bus.wb(GAME_MODE, 2);
    country_select_start(scheduler, bus, io, at)
}

/// 0x42736..0x4274e, 0x3ce54..0x3ce86: wait for timer counter 3 (`0xff8e2f !=
/// 0`), then queue stop command 0x3f, clear the text layer (0x3bdb0), and
/// return to 0x3ce54 (attract demo setup, ticket #88, F48): turn off layers
/// 0x28 (0x3b690), save difficulty `0xffc007` to `0xff8f28`, clear the demo
/// input record offset `0xff8f2a`, set difficulty `0xffc007` from `0x3cebe`
/// (`0`), clear `0xffc00b`, `0xffc006`, `0xffc003` and `0xffc004`, set
/// `GAME_MODE = 2`, and jump to `0x3e04c` (`country_select_start`).
fn attract_ranking_wait(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    if bus.rb(0xff8e2f) == 0 {
        return Yield::Sleep(1);
    }
    bus.queue_sound(0x3f);
    clear_text_layer(bus);
    attract_demo_setup(scheduler, bus, io, at)
}

/// 0x3df4c..0x3e020: one pass of the start-screen loop (resume 0x3dfce):
/// blinks `PUSH 1 PLAYER START BUTTON` (0x14e2a, credits <= 1) or
/// `PUSH 1 OR 2 PLAYERS START BUTTON` (0x14e4e, credits > 1) via the
/// counters 0xffc01a (erase at 0x0f) and 0xffc01b (draw at 0x1e), draws the
/// three credit digits at 0x800e66 (0x3be8a), and checks the start buttons
/// (`(!io.system) & 3`): 1P start (`== 1`) selects game mode 2 and takes 1
/// credit; 2P start (`!= 1`) requires at least 2 credits, selects game mode
/// 1 and takes 2 credits; then erases the credit digits (0x3bd96) and the
/// five blocks in 0x3e038 (the original always uses 0x3e038 on exit) and
/// enters country select at 0x3e04c (F34).
fn start_screen_frame(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    let a = bus.rb(0xffc01a).wrapping_add(1);
    bus.wb(0xffc01a, a);
    if a == 0x0f {
        let block = if (bus.rl(CREDITS_LONG) as i32) > 1 {
            0x14e4e
        } else {
            0x14e2a
        };
        erase_block(bus, block);
        bus.wb(0xffc01a, 0);
    }
    let b = bus.rb(0xffc01b).wrapping_add(1);
    bus.wb(0xffc01b, b);
    if b == 0x1e {
        let block = if (bus.rl(CREDITS_LONG) as i32) > 1 {
            0x14e4e
        } else {
            0x14e2a
        };
        text_block(bus, block);
        bus.wb(0xffc01b, 0);
    }
    draw_credit_digits(bus);
    let d1 = (!io.system) & 0x0003;
    if d1 == 0 {
        return speed_sleep(bus, io);
    }
    let cost = if d1 == 1 {
        bus.wb(GAME_MODE, 2);
        1u32
    } else {
        if bus.rl(CREDITS_LONG) < 2 {
            return speed_sleep(bus, io);
        }
        bus.wb(GAME_MODE, 1);
        2
    };
    for _ in 0..cost {
        decimal_down(bus, 0xff865e);
    }
    erase_credit_digits(bus);
    for i in 0..5u32 {
        let block = bus.rl(0x3e038 + 4 * i);
        erase_block(bus, block);
    }
    country_select_start(scheduler, bus, io, at)
}

/// 0x3e04c..0x3e0e0: country select entry — demo mode (`0xff8003 == 0`)
/// jumps straight to `0x3e372` (`country_select_exit`); the 4-country DIP
/// (`0xffc005 == 0`) branches to `0x3e1e4` (F47); otherwise the 2-country
/// screen draws `0x15d42`, `0x15d1e` (`0x3bc3e` 2x2), `0x15cb6`, `0x15cc6`
/// and `0x15c9a`, scrolls fg to `0x4200`, requests palette `0x1a468` at
/// `0x200`, turns off layers `0xa0` and turns on `0x48`, clears `0xffc006`
/// (Japan), queues tune `0x34`, and sleeps 1 frame (`0x3b51a`) into
/// `0x3e0e6`.
fn country_select_start(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    if bus.rb(MODE_DEMO) == 0 {
        return country_select_exit(scheduler, bus, io, at);
    }
    // Overhaul presets open Japan, USA, China, and England without the
    // four-country DIP. Faithful 1987 still follows `0xffc005` (`#98`, `O7`).
    if bus.roster.four_country || bus.rb(0xffc005) == 0 {
        return country_select4_start(bus, at);
    }
    text_block(bus, 0x15d42);
    text_block_2x2(bus, 0x15d1e);
    text_block(bus, 0x15cb6);
    text_block(bus, 0x15cc6);
    text_block(bus, 0x15c9a);
    bus.ww(0xffc680, 0);
    add_fg_scroll(bus, 0x4200);
    request_palette(bus, 0x1a468, 0x200);
    gfx_off(bus, 0xa0);
    gfx_on(bus, 0x48);
    bus.wb(0xffc006, 0);
    bus.queue_sound(0x34);
    *at = Flow::CountrySelectArm;
    Yield::Sleep(1)
}

/// 0x3e0e6..0x3e10e: clear `0xffc006`, arm timer counter 4 (`0xff8e16 =
/// 0x10`, `05.00` at `0xff8e30..0xff8e35`), clear attack-button latches
/// (`0x3b6c6`) and P1's button word (`0xff86b0`), set `0xffc008 = 2`
/// (`SCENE_MODE`), and sleep 1 frame (`0x3b51a`) into `0x3e110`.
fn country_select_arm(bus: &mut Bus, at: &mut Flow) -> Yield {
    bus.wb(0xffc006, 0);
    bus.wb(TIMER_FLAGS, 0x10);
    reset_button_latches(bus);
    bus.ww(0xff86b0, 0);
    bus.wb(SCENE_MODE, 2);
    *at = Flow::CountrySelectLoop { first: true };
    Yield::Sleep(1)
}

/// 0x3e110..0x3e1e3: one pass of the two-country select loop. Exits to
/// `0x3e18c` when timer counter 4 expires (`0xff8e35 != 0`) or P1 presses
/// any attack button (`0xff86b0 != 0`); otherwise checks `(A3)` (`0xff868f`,
/// pointed to by `0xff86b2`) for pure Left (`0xfd` -> country `0`, `0x15c9a`
/// drawn, `0x15ca8` erased) or pure Right (`0xfe` -> country `2`, `0x15ca8`
/// drawn, `0x15c9a` erased), ignoring diagonals, and sleeps 1 frame.
fn country_select_frame(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    if bus.rb(0xff8e35) != 0 || bus.rw(0xff86b0) != 0 {
        bus.queue_sound(0x3f);
        bus.wb(SCENE_MODE, 0);
        erase_block_2x2(bus, 0x15cda);
        for block in [0x15cb6, 0x15cc6, 0x15c9a, 0x15ca8] {
            erase_block(bus, block);
        }
        gfx_off(bus, 0x48);
        return country_select_exit(scheduler, bus, io, at);
    }
    let a3 = bus.rl(0xff86b2);
    match bus.rb(a3) {
        0xfd if bus.rb(0xffc006) != 0 => {
            bus.wb(0xffc006, 0);
            text_block(bus, 0x15c9a);
            erase_block(bus, 0x15ca8);
        }
        0xfe if bus.rb(0xffc006) != 2 => {
            bus.wb(0xffc006, 2);
            text_block(bus, 0x15ca8);
            erase_block(bus, 0x15c9a);
        }
        _ => {}
    }
    *at = Flow::CountrySelectLoop { first: false };
    Yield::Sleep(1)
}

/// 0x3e1e4..0x3e250: four-country select screen setup (`0xffc005 == 0`,
/// ticket #87, F47) — draws `0x15d42` (`0x3bba0`), `0x15d1e` (`0x3bc3e` 2x2),
/// and the initial Japan marker `0x15e64` (`0x3bba0`), scrolls fg to `0x4380`,
/// requests palette `0x1a668` at `0x200` (`0x3e216..0x3e228`), turns off
/// layers `0xa0` (`0x3b690`) and turns on `0x48` (`0x3b644`), clears `0xffc006`
/// (Japan), queues tune `0x34` (`0x3b7c8`), and sleeps 1 frame (`0x3b51a`)
/// into `0x3e252`.
fn country_select4_start(bus: &mut Bus, at: &mut Flow) -> Yield {
    text_block(bus, 0x15d42);
    text_block_2x2(bus, 0x15d1e);
    text_block(bus, 0x15e64);
    bus.ww(0xffc680, 0);
    add_fg_scroll(bus, 0x4380);
    request_palette(bus, 0x1a668, 0x200);
    gfx_off(bus, 0xa0);
    gfx_on(bus, 0x48);
    bus.wb(0xffc006, 0);
    bus.queue_sound(0x34);
    *at = Flow::CountrySelect4Arm;
    Yield::Sleep(1)
}

/// 0x3e252..0x3e27a: clear `0xffc006`, arm timer counter 4 (`0xff8e16 =
/// 0x10`, `05.00` at `0xff8e30..0xff8e35`), clear attack-button latches
/// (`0x3b6c6`) and P1's button word (`0xff86b0`), set `0xffc008 = 2`
/// (`SCENE_MODE`), and sleep 1 frame (`0x3b51a`) into `0x3e27c`.
fn country_select4_arm(bus: &mut Bus, at: &mut Flow) -> Yield {
    bus.wb(0xffc006, 0);
    bus.wb(TIMER_FLAGS, 0x10);
    reset_button_latches(bus);
    bus.ww(0xff86b0, 0);
    bus.wb(SCENE_MODE, 2);
    *at = Flow::CountrySelect4Loop {
        first: true,
        d6: 0,
        d7: 0,
    };
    Yield::Sleep(1)
}

/// 0x3e27c..0x3e371: one pass of the four-country select loop. On the first
/// frame (`0x3e27c`), clears `D6 = 0` and `D7 = 0`. Exits to `0x3e330` when
/// timer counter 4 expires (`0xff8e35 != 0`) or P1 presses any attack button
/// (`0xff86b0 != 0`); otherwise reads `(A3)` (`0xff868f`), complements and
/// masks the low 4 bits (`0x3e294..0x3e29a`), updates `D7` on pure Up (`8` ->
/// `0`) / pure Down (`4` -> `1`) or `D6` on pure Left (`2` -> `0`) / pure
/// Right (`1` -> `1`), ignoring neutral and diagonals (`0x3e29c..0x3e2d2`),
/// maps `(D7 << 1) + D6` through `0x3e2ea` (`[0, 2, 3, 1]`), and when
/// `0xffc006` changes, erases the old marker and draws the new marker from
/// `0x3e316` (`[0x15e64, 0x15ee2, 0x15f60, 0x15fde]`), then sleeps 1 frame
/// at `0x3e326` (`0x3b51a` into `0x3e32c`).
fn country_select4_frame(
    scheduler: &mut Scheduler,
    bus: &mut Bus,
    io: &Io,
    at: &mut Flow,
    first: bool,
    mut d6: u16,
    mut d7: u16,
) -> Yield {
    if first {
        d6 = 0;
        d7 = 0;
    }
    if bus.rb(0xff8e35) != 0 || bus.rw(0xff86b0) != 0 {
        bus.queue_sound(0x3f);
        bus.wb(SCENE_MODE, 0);
        let cur_marker = bus.rl(0x3e316 + 4 * (bus.rb(0xffc006) as u32));
        erase_block(bus, cur_marker);
        erase_block(bus, 0x15d42);
        erase_block_2x2(bus, 0x15d1e);
        gfx_off(bus, 0x48);
        return country_select_exit(scheduler, bus, io, at);
    }
    let a3 = bus.rl(0xff86b2);
    let d1 = (!bus.rb(a3) as u16) & 0x000f;
    let moved = match d1 {
        8 => {
            d7 = 0;
            true
        }
        4 => {
            d7 = 1;
            true
        }
        2 => {
            d6 = 0;
            true
        }
        1 => {
            d6 = 1;
            true
        }
        _ => false,
    };
    if moved {
        let idx = ((d7 << 1) + d6) as u32;
        let new_country = bus.rb(0x3e2ea + idx);
        let old_country = bus.rb(0xffc006);
        if new_country != old_country {
            bus.wb(0xffc006, new_country);
            let old_marker = bus.rl(0x3e316 + 4 * (old_country as u32));
            erase_block(bus, old_marker);
            let new_marker = bus.rl(0x3e316 + 4 * (new_country as u32));
            text_block(bus, new_marker);
        }
    }
    *at = Flow::CountrySelect4Loop {
        first: false,
        d6,
        d7,
    };
    Yield::Sleep(1)
}

/// 0x3e372..0x3e596: turn on layer `0x80` (`0x3b644`), dispatch by
/// `0xffc006` through `0x3e38a` (`0x3e39a / 0x3e416 / 0x3e492 / 0x3e50e`) to
/// write the 15 opponent pointer arrays at `0xffc01c..0xffc057` (`0x19908 +
/// 0x28 * country + 0xa0 * n`), spawn slot 2 at `0x3f7b0` (`0x3b20a`), and
/// yield `0x3b5aa` (`speed_sleep`) into `0x3e59c` (`Flow::StageStart`).
fn country_select_exit(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    gfx_on(bus, 0x80);
    let country = ((sext(bus.rb(0xffc006)) as u32) << 2) & 0xffff;
    let handler = bus.rl(0x3e38au32.wrapping_add(country as i16 as i32 as u32));
    for n in 0..15u32 {
        let ptr = bus.rl(handler + 8 * n + 2);
        bus.wl(0xffc01c + 4 * n, ptr);
    }
    if bus.evil_ryu_playtest.is_some() {
        // The private launcher picks Japan, so Sagat is the second fight in country five.
        bus.wb(0xffc003, 4);
        bus.wb(0xffc004, 1);
    }
    scheduler.spawn(bus, 2, 0x3f7b0);
    *at = Flow::StageStart;
    speed_sleep(bus, io)
}

/// 0x3be8a: draw the three credit digits (0xff865b..0xff865d) at 0x800e66
/// (attr 0x4000, leading zeros blanked as 0x3c) through 0x3bdfa.
fn draw_credit_digits(bus: &mut Bus) {
    draw_digits_blanked(bus, 0xff865b, 0x800e66, 0x4000, 2);
}

/// 0x3bd96..0x3bdcc: erase the three credit-digit cells at 0x800e66..0x800e6a
/// with the blank tile 0x103c (D0 = 2 into 0x3bdc2).
fn erase_credit_digits(bus: &mut Bus) {
    for i in 0..3u32 {
        bus.ww(0x800e66 + 2 * i, 0x103c);
    }
}

/// 0x3bb5c: count the decimal number whose digits (one per byte) end just
/// before `end` up by one, carrying 0s leftwards.
fn decimal_up(bus: &mut Bus, end: u32) {
    let mut a = end;
    loop {
        a -= 1;
        let d = bus.rb(a).wrapping_add(1);
        if d == 10 {
            bus.wb(a, 0);
        } else {
            bus.wb(a, d);
            return;
        }
    }
}

/// 0x3b654: coin-control copy (0xff8684) |= mask, and 0xc00010 written.
fn coin_ctrl_or(bus: &mut Bus, mask: u16) {
    let v = bus.rw(COIN_CONTROL) | mask;
    bus.ww(COIN_CONTROL, v);
    *bus.coin_ctrl = v;
}

/// 0x3b6a2: coin-control copy (0xff8684) &= !mask, and 0xc00010 written.
fn coin_ctrl_off(bus: &mut Bus, mask: u16) {
    let v = bus.rw(COIN_CONTROL) & !mask;
    bus.ww(COIN_CONTROL, v);
    *bus.coin_ctrl = v;
}

/// Request a vblank palette load (`0xff8634 = src`, `0xff8638 = offset`,
/// `0xff8633 = 0xff`).
fn request_palette(bus: &mut Bus, src: u32, offset: u16) {
    bus.wl(0xff8634, src);
    bus.ww(0xff8638, offset);
    bus.wb(PALETTE_REQUEST, 0xff);
}

/// 0x3cd14..0x3cd5e: run the ten-entry attract scroll loop (`0x3cde0`, 8
/// bytes per entry: fg_scroll, bg_scroll, sleep, sound) starting from index
/// `d1` (D1, 0..10); when an entry has sleep == 0 (entry 9), continue into
/// the three-entry title-block loop (`0x3cd56..0x3cd94`).
fn attract_scroll_loop(bus: &mut Bus, at: &mut Flow, mut d1: u16) -> Yield {
    while d1 < 10 {
        bus.ww(0xffc680, 0);
        bus.ww(0xffc682, 0);
        let entry = 0x3cde0 + ((d1 as u32) << 3);
        add_fg_scroll(bus, bus.rw(entry));
        add_bg_scroll(bus, bus.rw(entry + 2));
        let snd = bus.rw(entry + 6);
        if snd != 0 {
            bus.queue_sound(snd);
        }
        let sleep = bus.rw(entry + 4);
        if sleep != 0 {
            *at = Flow::AttractScroll { d1 };
            return Yield::Sleep(sleep as u8);
        }
        d1 += 1;
    }
    attract_title_block(bus, at, 0)
}

/// 0x3cd60..0x3cd80: draw title block `d1` (0..3) from `0x3cdc8` (8 bytes per
/// entry: block pointer, sleep, sound), queue its sound if nonzero, and
/// sleep.
fn attract_title_block(bus: &mut Bus, at: &mut Flow, d1: u16) -> Yield {
    let entry = 0x3cdc8 + ((d1 as u32) << 3);
    text_block(bus, bus.rl(entry));
    let snd = bus.rw(entry + 6);
    if snd != 0 {
        bus.queue_sound(snd);
    }
    let sleep = bus.rw(entry + 4) as u8;
    *at = Flow::AttractTitleBlock { d1 };
    Yield::Sleep(sleep)
}

/// 0x3e9bc..0x3ea30: one frame of the intro loop: the stage drift and the
/// object dispatcher; while either fighter is still in a 0x80 state (the
/// walk-in) the loop sleeps; then FIGHT! is drawn (0x14f44), its voice
/// queued (0x94), and a 0x1e-iteration wait begins.
fn intro_frame(bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    bus.wb(FRAME_PENDING, 0xff);
    bus.drift_stage();
    bus.dispatch_fighters();
    let dramatic = bus.dramatic_rules().is_some();
    if bus.rb(0xffe409) == 0x80 || bus.rb(0xffea09) == 0x80
        || (dramatic && bus.eb(DRAMATIC_ALLY2, 9) == 0x80)
    {
        *at = Flow::Intro;
        return speed_sleep(bus, io);
    }
    if dramatic {
        bus.position_dramatic_ally();
    }
    text_block(bus, 0x14f44);
    bus.queue_sound(0x94);
    // 0x3e9fa: the wait's counter 0x1e is counted down before each sleep.
    *at = Flow::FightShown { left: 0x1d };
    speed_sleep(bus, io)
}

/// 0x422d0: request scene event `n` (0xff8e36, scene bit 0x20); the caller
/// then sleeps until the screen-event task clears the bit.
fn request_event(bus: &mut Bus, n: u8) {
    bus.wb(0xff8e36, n);
    bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) | 0x20);
}

/// 0x3b9bc / 0x3b9c8 / 0x3b9d4: clear the effect objects (16 entries from
/// 0xffe000) and both fighter regions (24 entries each at 0x400 / 0xa00):
/// each entry's +8 byte and its position long.
fn clear_objects(bus: &mut Bus) {
    let count = if bus.dramatic_rules().is_some() { 4 } else { 3 };
    for (base, n) in [(0xffe000u32, 16u32), (0xffe400, 24), (0xffea00, 24), (0xfff000, 24)].into_iter().take(count) {
        for k in 0..n {
            let e = base + 0x40 * k;
            bus.wb(e + 8, 0);
            bus.wl(e + 4, 0);
        }
    }
}

/// 0x3e5d6..0x3e6c6: initialise both active player structures' HUD and
/// entity offsets (+0xa health bar, +0xc round markers, +0xe opponent
/// entity, +0x10 own entity, +0x20 score cell) and both fighter entities'
/// structure pointers (+0x38 own, +0x3c opponent) by game mode. Mode 2
/// (0x3e632) is exercised in F4 (#44), mode 1 (0x3e5ea) in F46 (#86), and
/// mode 4 (0x3e674) in F40 (#80).
fn place_players(bus: &mut Bus) {
    let (a1, a2) = match bus.rb(GAME_MODE) {
        2 => {
            for (off, val) in [
                (0xa, 0x2bc),
                (0xc, 0x298),
                (0x10, 0x400),
                (0xe, 0xa00),
                (0x20, 0x19a),
            ] {
                bus.ww(0xff86b0 + off, val);
            }
            for (off, val) in [(0xa, 0x3bc), (0xc, 0x2e0), (0x10, 0xa00), (0xe, 0x400)] {
                bus.ww(0xffc664 + off, val);
            }
            (0xff86b0, 0xffc664)
        }
        4 => {
            for (off, val) in [
                (0xa, 0x2bc),
                (0xc, 0x2e0),
                (0x10, 0x400),
                (0xe, 0xa00),
                (0x20, 0x1dc),
            ] {
                bus.ww(0xff86da + off, val);
            }
            for (off, val) in [(0xa, 0x3bc), (0xc, 0x298), (0x10, 0xa00), (0xe, 0x400)] {
                bus.ww(0xffc664 + off, val);
            }
            (0xff86da, 0xffc664)
        }
        _ => {
            for (off, val) in [
                (0xa, 0x2bc),
                (0xc, 0x298),
                (0x10, 0x400),
                (0xe, 0xa00),
                (0x20, 0x19a),
            ] {
                bus.ww(0xff86b0 + off, val);
            }
            for (off, val) in [
                (0xa, 0x3bc),
                (0xc, 0x2e0),
                (0x10, 0xa00),
                (0xe, 0x400),
                (0x20, 0x1dc),
            ] {
                bus.ww(0xff86da + off, val);
            }
            (0xff86b0, 0xff86da)
        }
    };
    bus.wl(0xffe438, a1);
    bus.wl(0xffe43c, a2);
    bus.wl(0xffea38, a2);
    bus.wl(0xffea3c, a1);
    if bus.dramatic_rules().is_some() {
        // Ally 2 owns P2's controls and a separate object grid. The left
        // HUD belongs to Ally 1 and the right HUD to the boss.
        bus.wl(P2_STRUCT + 2, 0xff868e);
        bus.ww(P2_STRUCT + 0xa, 0);
        bus.ww(P2_STRUCT + 0xe, 0xa00);
        bus.ww(P2_STRUCT + 0x10, DRAMATIC_ALLY2);
        bus.set_el(DRAMATIC_ALLY2, 0x38, P2_STRUCT);
        bus.set_el(DRAMATIC_ALLY2, 0x3c, CPU_STRUCT);
    }
}

/// 0x3e6ca..0x3e712: clear the fighter object regions; in demo play
/// (`0xff8003 == 0`, `0x3e6fe..0x3e70e`) request the stage palette (`0x43748`,
/// with its `speed_sleep`) into `0x3e704`; outside demo play, request the stage
/// intro (event 4) and wait at 0x422e0.
fn next_opponent(bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    bus.toggle_state.satsui_awakening = None;
    clear_fighters(bus);
    if bus.rb(MODE_DEMO) == 0 {
        request_stage_palette(bus);
        *at = Flow::DemoStagePalette;
        return speed_sleep(bus, io);
    }
    request_event(bus, 4);
    *at = Flow::AwaitOpponentIntro;
    speed_sleep(bus, io)
}

/// 0x3b9c8 / 0x3b9d4: clear both fighter regions only (the next opponent's
/// setup, 0x3e6ca).
fn clear_fighters(bus: &mut Bus) {
    let count = if bus.dramatic_rules().is_some() { 3 } else { 2 };
    for base in [0xffe400u32, 0xffea00, 0xfff000].into_iter().take(count) {
        for k in 0..24 {
            let e = base + 0x40 * k;
            bus.wb(e + 8, 0);
            bus.wl(e + 4, 0);
        }
    }
}

/// 0x3ee26: the match end by game mode and winner (0xffc009). A player's
/// match win in a one-player mode (0x3ef54): sound 0x37, then the
/// match-won screen (event 0xb). The CPU's match win (0x3eefa in mode 2;
/// 0x3ef2c in mode 4, unverified in F5): the CPU's lose-quote screen
/// (event 0xb), the wipe (event 6), the departing player's ranking check
/// (0x42422), and the continue screen (0x3efda). The versus match end
/// (0x3ee3a) sends a winner on against the CPU or, on a draw, wipes into
/// the versus game over (0x3f2d0).
fn match_end(bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    match (bus.rb(GAME_MODE), bus.rb(0xffc009)) {
        (2, 1) | (4, 2) => {
            bus.queue_sound(0x37);
            request_event(bus, 0xb);
            *at = Flow::AwaitWinScreen { versus: false };
            speed_sleep(bus, io)
        }
        (2 | 4, _) => {
            // 0x3eefa (mode 2) / 0x3ef2c (mode 4, unverified in F5):
            // request event 0xb and wait at 0x422e0.
            request_event(bus, 0xb);
            *at = Flow::AwaitCpuWinScreen;
            speed_sleep(bus, io)
        }
        (_, 1 | 2) => {
            // 0x3ee5c / 0x3ee8e: a versus winner goes on against the CPU:
            // sound 0x37, the match-won screen, then the wipe.
            bus.queue_sound(0x37);
            request_event(bus, 0xb);
            *at = Flow::AwaitWinScreen { versus: true };
            speed_sleep(bus, io)
        }
        _ => {
            // 0x3ee4e: no winner in versus: the wipe, then game over.
            request_event(bus, 6);
            *at = Flow::AwaitGameOverWipe;
            speed_sleep(bus, io)
        }
    }
}

/// 0x42422..0x42484: the departing player's ranking check and name entry.
/// Stores `(ps + 2)` at `0xff8e62` and `ps` at `0xff8e5e`, then compares
/// the player's 7-digit score (`ps + 0x22`) against the ten 16-byte ranking
/// entries (`0xff8e6e + 16 * d2 + 5`, `d2 = 0..=9`) via `0x3bafe`
/// (`Bus::compare_bytes_7`). A qualifying score stores its entry address at
/// `0xff8e66` and enters the name-entry screen (`0x42486`, ticket #82, F42);
/// a non-qualifying score (`0x42470`) clears the 8-byte score at `ps + 0x22`
/// via `0x3b934` (`Bus::clear_words_4`) and returns `None`.
fn ranking_check(bus: &mut Bus, ps: u32, ret: RankingReturn, at: &mut Flow) -> Option<Yield> {
    let score = ps + 0x22;
    bus.wl(0xff8e62, bus.rl(ps + 2));
    bus.wl(0xff8e5e, ps);
    let mut d2 = 0u16;
    loop {
        let entry_score = 0xff8e6e + ((d2 as u32) << 4) + 5;
        let rank = match bus.compare_bytes_7(entry_score, score) {
            std::cmp::Ordering::Greater => Some(d2),
            std::cmp::Ordering::Equal => {
                // 0x42450 -> 0x42468: addq.b #1, D2; cmpi.w #9, D2; bls $42458.
                d2 = (d2 & 0xff00) | (d2 as u8).wrapping_add(1) as u16;
                if d2 > 9 {
                    break;
                }
                Some(d2)
            }
            std::cmp::Ordering::Less => {
                // 0x42454 -> 0x4247a: addq.w #1, D2; cmpi.w #9, D2; bhi $42470.
                d2 = d2.wrapping_add(1);
                if d2 > 9 {
                    break;
                }
                None
            }
        };
        if let Some(rank) = rank {
            // 0x42458..0x42464: retain the qualifying entry, then name entry.
            bus.wl(0xff8e66, 0xff8e6e + ((rank as u32) << 4));
            return Some(name_entry_start(bus, at, ret));
        }
    }
    // 0x42470..0x42478: not qualified -> clear 4 words at `ps + 0x22` (0x3b934).
    bus.clear_words_4(score);
    None
}

/// 0x42486..0x42550: set up the name-entry screen (ticket #82, F42):
/// initialize `0xff8e6a` (`0x3131_3100`), `0xff8e54` (`0x062e`), `0xff8e58`
/// (`0x0444`), `0xff8e5a` (`0xff8e6a`) and `0xff8e52` (`0`), clear the text
/// layer (`0x3bdb0`), draw blocks `0x14656` and `0x1466e` (`0x3bba0`), draw
/// the 3x10 character grid from `0x14638` (`0xa000 | code` at `0x80062e +
/// 0x100 * row + 4 * col`), scroll fg to `0x4080` (`0x3b674`), request palette
/// `0x1ac68` at offset `0x200`, turn off bg (`0x20`, `0x3b690`) and turn on
/// `0xc8` (`0x3b644`), clear all object regions (`0x3b9bc..0x3b9d4`), start
/// timer 2 (`0xff8e16 = 4`), and sleep 1 frame (`0x3b51a` at `0x4254c` into
/// `0x42552`).
fn name_entry_start(bus: &mut Bus, at: &mut Flow, ret: RankingReturn) -> Yield {
    bus.wl(0xff8e6a, 0x3131_3100);
    bus.ww(0xff8e54, 0x062e);
    bus.ww(0xff8e58, 0x0444);
    bus.wl(0xff8e5a, 0xff8e6a);
    bus.ww(0xff8e52, 0);
    clear_text_layer(bus);
    text_block(bus, 0x14656);
    text_block(bus, 0x1466e);
    let mut a0 = 0x14638u32;
    for row in 0..3u32 {
        let row_base = 0x80062e + row * 0x100;
        for col in 0..10u32 {
            let tile = 0xa000 | (bus.rb(a0) as u16);
            a0 += 1;
            bus.ww(row_base + col * 4, tile);
        }
    }
    bus.ww(0xffc680, 0);
    add_fg_scroll(bus, 0x4080);
    request_palette(bus, 0x1ac68, 0x200);
    gfx_off(bus, 0x20);
    gfx_on(bus, 0xc8);
    clear_objects(bus);
    bus.wb(TIMER_FLAGS, 4);
    *at = Flow::NameEntryPalette { ret };
    Yield::Sleep(1)
}

/// 0x42552..0x42582: spawn the cursor effect `0x1845c` via `0x42f72`
/// (`Bus::spawn_effect_small`), clear the active player's latched punch byte
/// `*(0xff8e5e)` and the attack-button latches (`0x3b6c6`), queue tune `0x38`
/// (`0x3b7c8`), run effects once (`0x42d22` at `0x42578`), and sleep 1 frame
/// (`0x3b51a` at `0x4257e` into `0x42584`).
fn name_entry_arm(bus: &mut Bus, at: &mut Flow, ret: RankingReturn) -> Yield {
    bus.spawn_effect_small(0x1845c, 0);
    let ps = bus.rl(0xff8e5e);
    bus.wb(ps, 0);
    reset_button_latches(bus);
    bus.queue_sound(0x38);
    bus.run_effects();
    *at = Flow::NameEntryLoop { ret };
    Yield::Sleep(1)
}

/// 0x42584..0x425c2: one pass of the name-entry loop: draw the integer and
/// fractional countdown digits (`0xff8e24` at `0x800b44`, `0xff8e26` at
/// `0x800b4a`, attr `0x1000`, `D1 = 1`), then exit to `0x425c4` (`name_entry_insert`,
/// ticket #122, F50) when timer 2 expires (`0xff8e29 != 0`) or the cursor
/// effect deactivates (`0x4329c` returns 0); otherwise run effects (`0x42d22`)
/// and sleep 1 frame (`0x3b51a` into `0x42584`).
fn name_entry_frame(bus: &mut Bus, at: &mut Flow, ret: RankingReturn) -> Yield {
    draw_digits(bus, 0xff8e24, 0x800b44, 0x1000, 1);
    draw_digits(bus, 0xff8e26, 0x800b4a, 0x1000, 1);
    if bus.rb(0xff8e29) != 0 || bus.live_effect_count() == 0 {
        return name_entry_insert(bus, at, ret);
    }
    bus.run_effects();
    *at = Flow::NameEntryLoop { ret };
    Yield::Sleep(1)
}

/// 0x425c4..0x42676: clear the 16 effect objects (`0x3b9bc`), queue sound stop
/// `0x3f` (`0x3b7c8`), shift ranking entries `rank..9` down by one slot (`11`
/// bytes each at `+5..+16` when `rank < 9`), mark the new entry (`a2 + 4 =
/// 0xff`), copy the player's 8-byte score (`ps + 0x22`) into `a2 + 5..+13` and
/// 3-byte initials (`0xff8e6a..0xff8e6d`) into `a2 + 13..+16`, clear the
/// player's score via `0x3b934` (`Bus::clear_words_4`), turn off fg (`0x40`)
/// and turn on bg (`0x20`), request palette `0x1b868` at offset `0`, and enter
/// the ranking screen at `0x4268c` (ticket #122, F50).
fn name_entry_insert(bus: &mut Bus, at: &mut Flow, ret: RankingReturn) -> Yield {
    for k in 0..16u32 {
        let e = 0xffe000 + 0x40 * k;
        bus.wb(e + 8, 0);
        bus.wl(e + 4, 0);
    }
    bus.queue_sound(0x3f);
    let a2 = bus.rl(0xff8e66);
    let d2 = ((a2 - 0xff8e6e) >> 4) as u16;
    if d2 < 9 {
        for rank in (d2 as u32..9).rev() {
            let src = 0xff8e6e + 16 * rank + 5;
            let dst = src + 16;
            for i in 0..11u32 {
                bus.wb(dst + i, bus.rb(src + i));
            }
        }
    }
    let score_addr = bus.rl(0xff8e5e) + 0x22;
    bus.wb(a2 + 4, 0xff);
    for i in 0..8u32 {
        bus.wb(a2 + 5 + i, bus.rb(score_addr + i));
    }
    for i in 0..3u32 {
        bus.wb(a2 + 13 + i, bus.rb(0xff8e6a + i));
    }
    bus.clear_words_4(score_addr);
    gfx_off(bus, 0x40);
    gfx_on(bus, 0x20);
    request_palette(bus, 0x1b868, 0);
    ranking_screen_start(bus, at, ret)
}

/// 0x3efc4..0x3efd8: advance `STAGE_NO` (`0xffc003`) after a country's bonus
/// stage (or Thailand's Sagat fight). Stage 5 enters `0x3f2d0`
/// (`versus_game_over`); stages 1..=4 clear `SCENE_MODE` (`0xffc008`) and
/// jump to `0x3e59c` (`Flow::StageStart`, ticket #69, F29).
fn stage_step(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    bus.wb(0xffc003, bus.rb(0xffc003).wrapping_add(1));
    if bus.rb(0xffc003) == 5 {
        return versus_game_over(bus, at);
    }
    bus.wb(SCENE_MODE, 0);
    *at = Flow::StageStart;
    flow(scheduler, bus, io, at)
}

/// 0x40d30..0x40dce (`BonusCopy::Japan`, ticket #69, F29) / 0x41a02..0x41aa0
/// (`BonusCopy::Stage2`, ticket #130, F58): set up the power/timing bonus
/// stage: set `SCENE_MODE = 0x40`, store the active player structure pointer
/// (`0xff86da` in mode 4, `0xff86b0` in mode 2) at `0xff8f14`, clear the text
/// layer (`0x3bdb0`), draw blocks `0x14786`, `0x147a2`, and `0x147c0`, draw
/// gauge index 0 (`0x4147e`), clear all objects (`0x3b9bc..0x3b9d4`), place
/// the 2x4 sprite grid from `0x412aa` / `0x41f5e` (`0x4141a`), scroll fg to
/// `0x0d80` and bg to `0x0c00`, request palette `0x1ae68` at offset `0`, and
/// sleep 1 frame (`0x3b51a` into `0x40dd4` / `0x41aa6`).
fn bonus_japan_start(bus: &mut Bus, at: &mut Flow, copy: BonusCopy) -> Yield {
    bus.wb(SCENE_MODE, 0x40);
    let ps = if bus.rb(GAME_MODE) == 4 {
        0xff86da
    } else {
        0xff86b0
    };
    bus.wl(0xff8f14, ps);
    clear_text_layer(bus);
    for block in [0x14786, 0x147a2, 0x147c0] {
        text_block(bus, block);
    }
    bonus_draw_gauge(bus, 0);
    clear_objects(bus);
    bonus_place_grid(bus, copy.grid_table());
    bus.ww(0xffc680, 0);
    bus.ww(0xffc682, 0);
    add_fg_scroll(bus, 0x0d80);
    add_bg_scroll(bus, 0x0c00);
    request_palette(bus, 0x1ae68, 0);
    *at = Flow::BonusJapanPalette { copy };
    Yield::Sleep(1)
}

/// 0x40e6e..0x4103c (`BonusCopy::Japan`) / 0x41b40..0x41d0e
/// (`BonusCopy::Stage2`): one frame of the power-meter loop: draw the timer-3
/// digits (`0xff8e30` at `0x800492`, `0xff8e32` at `0x800498`), step the
/// 3-digit rolling power display (`0xff8f11..0xff8f14`, wrapping `999` to `000`
/// via `0x3b96c` or adding `111` from `0x40f1b` / `0x41bed` via `0x3ba12`) and
/// draw it at `0x800712`, check for timer-3 expiry (`0xff8e35 != 0`) or a
/// latched punch (`*(0xff8f14) != 0`) to enter `0x4103e` / `0x41d10`, and step
/// the gauge/stance continuation state machine whenever `0xff8f0f` reaches 0.
fn bonus_japan_meter_frame(
    bus: &mut Bus,
    at: &mut Flow,
    copy: BonusCopy,
    mut cont: u32,
    mut d0: u16,
) -> Yield {
    let shift = copy.meter_shift();
    loop {
        draw_digits(bus, 0xff8e30, 0x800492, 0x3000, 1);
        draw_digits(bus, 0xff8e32, 0x800498, 0x3000, 1);
        if bus.rb(0xff8f11) == 9 && bus.rw(0xff8f12) == 0x0909 {
            bus.clear_bytes_3(0xff8f11);
        } else {
            decimal_add(bus, 0xff8f14, copy.meter_step_src(), 3);
        }
        draw_digits(bus, 0xff8f11, 0x800712, 0x3000, 2);
        let a3 = bus.rl(0xff8f14);
        if bus.rb(0xff8e35) != 0 || bus.rb(a3) != 0 {
            return bonus_japan_strike(bus, at, copy);
        }
        let wait = bus.rb(0xff8f0f);
        if wait != 0 {
            bus.wb(0xff8f0f, wait.wrapping_sub(1));
            *at = Flow::BonusJapanMeter { copy, cont, d0 };
            return Yield::Sleep(1);
        }
        match cont.wrapping_sub(shift) {
            0x40f20 => {
                bus.wb(0xff8f0e, 0);
                bonus_draw_gauge(bus, 0x0c);
                bus.ww(0xffc680, 0);
                add_fg_scroll(bus, 0x0d80);
                cont = 0x40f44 + shift;
            }
            0x40f44 => {
                bonus_draw_gauge(bus, 0x0b);
                cont = 0x40f56 + shift;
            }
            0x40f56 => {
                bonus_draw_gauge(bus, 0x0a);
                cont = 0x40f68 + shift;
            }
            0x40f68 => {
                bus.wb(0xff8f0e, 1);
                bonus_draw_gauge(bus, 0x09);
                bus.ww(0xffc680, 0);
                add_fg_scroll(bus, 0x0f00);
                cont = 0x40f8e + shift;
            }
            0x40f8e => {
                bonus_draw_gauge(bus, 0x08);
                cont = 0x40fa0 + shift;
            }
            0x40fa0 => {
                bonus_draw_gauge(bus, 0x07);
                cont = 0x40fb2 + shift;
            }
            0x40fb2 => {
                bonus_draw_gauge(bus, 0x06);
                cont = 0x40fc4 + shift;
            }
            0x40fc4 => {
                bus.wb(0xff8f0e, 2);
                bonus_draw_gauge(bus, 0x05);
                bus.ww(0xffc680, 0);
                add_fg_scroll(bus, 0x1080);
                cont = 0x40fea + shift;
            }
            0x40fea => {
                d0 = 4;
                bonus_draw_gauge(bus, 4);
                cont = 0x40ffe + shift;
            }
            0x40ffe => {
                d0 = d0.wrapping_sub(1);
                if d0 != 0 {
                    bonus_draw_gauge(bus, d0);
                    cont = 0x40ffe + shift;
                } else {
                    bonus_draw_gauge(bus, 0);
                    let idx = (((bus.rb(0xff8e4b) & 0x0f) >> 1) << 2) as u32;
                    cont = bus.rl(copy.restart_table() + idx);
                }
            }
            _ => {
                bus.note(cont);
                return Yield::Halt;
            }
        }
        bus.wb(0xff8f0f, 3);
    }
}

/// 0x4103e..0x41058 (`BonusCopy::Japan`) / 0x41d10..0x41d2e
/// (`BonusCopy::Stage2`): resolve the player's punch strength (`0x41558`),
/// queue strike voice `0x8c`, and start the wind-up scroll sequence
/// (`0x4107a + 4 * sext(0xff8f0e)`). Copy 2 uses `0x41d28: lea (-$cb0,PC), A0`
/// and `0x41d2c: lea (0,A0,D0.w), A0` to index Copy 1's shared wind-up table at
/// `0x4107a`.
fn bonus_japan_strike(bus: &mut Bus, at: &mut Flow, copy: BonusCopy) -> Yield {
    let a3 = bus.rl(0xff8f14);
    bonus_resolve_power(bus, a3);
    bus.queue_sound(0x8c);
    let phase = bus.rb(0xff8f0e) as i8 as i32;
    let a0 = (0x4107ai32 + phase * 4) as u32;
    bonus_japan_windup(bus, at, copy, a0)
}

/// 0x4105a..0x41078 (`BonusCopy::Japan`) / 0x41d30..0x41d4e
/// (`BonusCopy::Stage2`): step the wind-up fg-scroll stream `(scroll, sleep)`
/// at `a0` until the `0xffff` terminator enters `0x4108a` / `0x41d50`
/// (`bonus_japan_judge`).
fn bonus_japan_windup(bus: &mut Bus, at: &mut Flow, copy: BonusCopy, mut a0: u32) -> Yield {
    loop {
        let scroll = bus.rw(a0);
        a0 += 2;
        if scroll == 0xffff {
            return bonus_japan_judge(bus, at, copy);
        }
        bus.ww(0xffc680, 0);
        add_fg_scroll(bus, scroll);
        let frames = bus.rw(a0);
        a0 += 2;
        if frames != 0 {
            *at = Flow::BonusJapanWindup { copy, a0 };
            return Yield::Sleep(frames as u8);
        }
    }
}

/// 0x4108a..0x41158 (`BonusCopy::Japan`) / 0x41d50..0x41e0c
/// (`BonusCopy::Stage2`): judge the strike against the phase's threshold table
/// (`0x410d0[0xff8f0e]` or `0x41d96[0xff8f0e]`). When the resolved strength
/// level `d3` (`0xffe000 + me + 0x34`) is below the minimum threshold `(A1)`,
/// branch to the failure sequence at `0x41226` / `0x41eda`; otherwise pick the
/// break tier `d2`, queue its crash sound (`0x4112a + 2 * d2` / `0x41de0 + 2 *
/// d2`), copy its 8-byte bonus digits into `0xff8f18..0xff8f20`, and start its
/// sprite animation stream (`0x4110a + 8 * d2` / `0x41dc8 + 8 * d2`).
fn bonus_japan_judge(bus: &mut Bus, at: &mut Flow, copy: BonusCopy) -> Yield {
    let phase = bus.rb(0xff8f0e) as i8 as i32;
    let mut a1 = bus.rl((copy.threshold_table() as i32 + phase * 4) as u32);
    let a3 = bus.rl(0xff8f14);
    let me = bus.rw(a3 + 0x10);
    let d3 = bus.rb(0xffe000 + me as u32 + 0x34);
    if d3 < bus.rb(a1) {
        // 0x41226..0x41234 / 0x41eda..0x41ee8: queue thud sound 0x10 and sleep
        // 90 frames (0x3b5a2 into 0x4123a / 0x41eee).
        bus.queue_sound(0x10);
        *at = Flow::BonusJapanFailPose { copy };
        return Yield::Sleep(0x5a);
    }
    // 0x410a4..0x410c4 / 0x41d6a..0x41d8a: advance `a1` by 2 while the next
    // pair's threshold `($2,A1)` is not `0xff` and `d3 >= ($2,A1)`.
    while bus.rb(a1 + 2) != 0xff && d3 >= bus.rb(a1 + 2) {
        a1 += 2;
    }
    let d2 = bus.rb(a1 + 1) as i8 as i16 as u16;
    bus.queue_sound(bus.rw(copy.sound_table() + ((d2 as u32) << 1)));
    let entry = copy.result_table() + ((d2 as u32) << 3);
    let score_src = bus.rl(entry + 4);
    for i in 0..4u32 {
        bus.ww(0xff8f18 + 2 * i, bus.rw(score_src + 2 * i));
    }
    let mut anim = bus.rl(entry);
    let count = bus.rw(anim);
    anim += 2;
    bonus_update_sprites(bus, &mut anim);
    let frames = bus.rw(anim);
    anim += 2;
    *at = Flow::BonusJapanBreak {
        copy,
        a1: anim,
        d2: count,
    };
    Yield::Sleep(frames as u8)
}

/// 0x41158..0x41164 (`BonusCopy::Japan`) / 0x41e0c..0x41e18
/// (`BonusCopy::Stage2`): step the breaking sprite stream `(8 words, sleep)`
/// `d2` times (`dbra D2, $4114c` / `$41e00`); when `d2 == 0`, sleep 90 frames
/// (`0x3b5a2` at `0x41160` / `0x41e14` into `0x41166` / `0x41e1a`).
fn bonus_japan_break(bus: &mut Bus, at: &mut Flow, copy: BonusCopy, mut a1: u32, d2: u16) -> Yield {
    let d2 = d2.wrapping_sub(1);
    if d2 != 0xffff {
        bonus_update_sprites(bus, &mut a1);
        let frames = bus.rw(a1);
        a1 += 2;
        *at = Flow::BonusJapanBreak { copy, a1, d2 };
        return Yield::Sleep(frames as u8);
    }
    *at = Flow::BonusJapanSuccessPose { copy };
    Yield::Sleep(90)
}

/// 0x411ba..0x4120a (`BonusCopy::Japan`) / 0x41e6e..0x41ebe
/// (`BonusCopy::Stage2`): scroll bg back to `0x0f00` (`0x3b682`) and repeat the
/// 12-frame cheer cycle (`dbra D2, $411a0` / `$41e54`, sleeping 6 frames at
/// `0x411a0` / `0x41e54` into `0x411a6` / `0x41e5a`); after 15 cycles
/// (`0x411cc..0x4120a` / `0x41e80..0x41ebe`), draw the `BONUS PTS.` label
/// (`0x147e2`, `0x3bba0`), draw the 4 bonus digits from `0xff8f1c` at
/// `0x8008c4` (`0x3bdf2`), add the 7-digit bonus (`0xff8f18..0xff8f1f`) onto
/// the active player's 7-digit score (`a3 + 0x22..+0x29`, `0x3ba0e`), update
/// the top score (`0x3ba84`), and sleep 120 frames (`0x3b56a` at `0x41206` /
/// `0x41eba` into `0x4120c` / `0x41ec0`).
fn bonus_japan_cheer_b(bus: &mut Bus, at: &mut Flow, copy: BonusCopy, d2: u16) -> Yield {
    bus.ww(0xffc682, 0);
    add_bg_scroll(bus, 0x0f00);
    let d2 = d2.wrapping_sub(1);
    if d2 != 0xffff {
        *at = Flow::BonusJapanCheerA { copy, d2 };
        return Yield::Sleep(6);
    }
    text_block(bus, 0x147e2);
    draw_digits(bus, 0xff8f1c, 0x8008c4, 0x3000, 3);
    let a3 = bus.rl(0xff8f14);
    if bus.rb(MODE_DEMO) != 0 {
        decimal_add(bus, a3 + 0x29, 0xff8f1f, 7);
    }
    let me = bus.rw(a3 + 0x10);
    bus.check_hiscore(me);
    *at = Flow::BonusJapanHold1 { copy };
    Yield::Sleep(120)
}

/// 0x4141a..0x41466: populate the eight effect object slots (`0xffe100 +
/// 0x40 * k`, `k = 0..8`) as a 2-row by 4-column grid (`+0x20` per column and
/// row) from the descriptor at `a1` (`y0:x0` long at `+0`, base attr word at
/// `+4`, eight sprite code words at `+6..+22`, bit 15 setting x-flip bit 8 in
/// `+2`).
fn bonus_place_grid(bus: &mut Bus, a1: u32) {
    let d1 = bus.rl(a1);
    let (y0, x0) = ((d1 >> 16) as u16, d1 as u16);
    let d2 = bus.rw(a1 + 4);
    let mut ptr = a1 + 6;
    let mut d3 = 0x100u32;
    for row in 0..2u16 {
        let y = y0.wrapping_add(0x20 * row);
        for col in 0..4u16 {
            let x = x0.wrapping_add(0x20 * col);
            let slot = 0xffe000 + d3;
            bus.wl(slot + 4, ((y as u32) << 16) | (x as u32));
            let d4 = bus.rw(ptr);
            ptr += 2;
            let attr = if d4 & 0x8000 != 0 { d2 | 0x0100 } else { d2 };
            bus.ww(slot + 2, attr);
            bus.ww(slot, d4 & 0x7fff);
            d3 += 0x40;
        }
    }
}

/// 0x41468..0x4147c: write eight sprite-code words from `*a1` into the eight
/// effect object slots (`0xffe100 + 0x40 * k`, `k = 0..8`), advancing `*a1`
/// by 16 bytes.
fn bonus_update_sprites(bus: &mut Bus, a1: &mut u32) {
    for k in 0..8u32 {
        let w = bus.rw(*a1);
        *a1 += 2;
        bus.ww(0xffe100 + 0x40 * k, w);
    }
}

/// 0x4147e..0x414ba: draw the bonus power-gauge bar at `0x800992` from entry
/// `index` (`0..=12`) of `0x414bc` (three signed byte counts for tiles `0x00c2`,
/// `0x00c1`, and `0x00c0`, each `dbra` loop skipped when negative).
fn bonus_draw_gauge(bus: &mut Bus, index: u16) {
    let entry = 0x414bc + ((index as u32) << 2);
    let mut a1 = 0x800992u32;
    for (i, tile) in [(0u32, 0x00c2u16), (1, 0x00c1), (2, 0x00c0)] {
        let n = bus.rb(entry + i) as i8;
        if n >= 0 {
            for _ in 0..=n {
                bus.ww(a1, tile);
                a1 += 2;
            }
        }
    }
}

/// 0x414f0..0x41516: copy 32 bytes (16 palette words) from `0x41518` when
/// `GAME_MODE == 2` (`0x414f6..0x41502`) or `0x41538` otherwise
/// (`0x41504..0x4150a`) into palette RAM at offset `0x3e0..0x400`.
fn bonus_load_palette_tail(bus: &mut Bus) {
    let src = if bus.rb(GAME_MODE) == 2 {
        0x41518u32
    } else {
        0x41538
    };
    for i in 0..0x20usize {
        bus.palette[0x3e0 + i] = bus.rb(src + i as u32);
    }
}

/// 0x41558..0x4159c: clear the fighter's strength-level word at `0xffe000 +
/// me + 0x34`, map the active player's latched punch (`a3`) through the
/// 10-entry threshold table `0x4159e` via `0x40b72` (`Bus::pick_by_strength`,
/// storing level `1..=10` in `+0x34` or leaving `0` on no match), and draw the
/// corresponding 3-digit final power display from `0x415b4[d2]` (indexed by
/// `((0xff8e4f & 0x0f) >> 1) << 2`) at `0x800712`.
fn bonus_resolve_power(bus: &mut Bus, a3: u32) {
    let me = bus.rw(a3 + 0x10);
    bus.ww(0xffe000 + me as u32 + 0x34, 0);
    let d2 = bus.pick_by_strength(a3, 0x4159e, me).unwrap_or(0) as u16;
    let table = bus.rl(0x415b4 + ((d2 as u32) << 2));
    let off = (((bus.rb(0xff8e4f) & 0x0f) >> 1) << 2) as u32;
    draw_digits(bus, table + off, 0x800712, 0x3000, 2);
}

/// 0x41740..0x41812 (Copy 1, ticket #124, F52) / 0x41fe6..0x420ca (Copy 2,
/// ticket #133, F61): set up the 10-second bonus stage: clear the text layer
/// (`0x3bdb0`) and all three object regions (`0x3b9bc..0x3b9d4`), set
/// `SCENE_MODE = 0x40`, place the active player at `x = 0x100` facing right
/// (`0x44174` for P1 `0xff86b0` in mode 2; `0x41776` / `0x4201c -> 0x44192`
/// for P2 `0xff86da` in mode 4, mode 4 unexercised), clear the player's button
/// latches (`clr.w (A1)`), spawn the concrete block stack effects (`[0x18e24,
/// 0x18e44, 0x18e64]` in Copy 1; `[0x18e44, 0x18e64, 0x18e84, 0x18ea4]` in Copy
/// 2) with `owner = rw(ps + 0x0e)` via `0x42f1e` (`Bus::spawn_effect_absolute`),
/// running `0x42d22` (`Bus::run_effects`) after each spawn, run the object
/// dispatcher once (`0x429f2`), draw text blocks `0x14832` (`TIME`) and
/// `0x1484e` (`1000`), scroll fg to `0x1680` (Copy 1) or `0x1800` (Copy 2) and
/// bg to `0x1380`, request palette `0x1ae68` at offset `0`, and sleep 1 frame
/// (`0x3b51a` into `0x41814` / `0x420cc`).
fn bonus_stage1_start(bus: &mut Bus, at: &mut Flow, copy: Bonus10Copy) -> Yield {
    clear_text_layer(bus);
    clear_objects(bus);
    bus.wb(SCENE_MODE, 0x40);
    let ps = if bus.rb(GAME_MODE) == 4 {
        0xff86da
    } else {
        0xff86b0
    };
    bus.place_bonus_player(ps);
    bus.ww(ps, 0);
    let owner = bus.rw(ps + 0x0e);
    for &template in copy.templates() {
        bus.spawn_effect_absolute(template, owner);
        bus.run_effects();
    }
    for ps in bus.dispatch_order() {
        bus.run_player(ps);
    }
    text_block(bus, 0x14832);
    text_block(bus, 0x1484e);
    bus.ww(0xffc680, 0);
    bus.ww(0xffc682, 0);
    add_fg_scroll(bus, copy.fg_scroll());
    add_bg_scroll(bus, 0x1380);
    request_palette(bus, 0x1ae68, 0);
    *at = Flow::BonusStage1Palette1 { copy };
    Yield::Sleep(1)
}

/// 0x418ac..0x4191a (Copy 1) / 0x42164..0x421d2 (Copy 2): one pass of the
/// 10-second bonus loop: run the active player's input handler (`0x402f8` on
/// `0xff86b0` in mode 2; `0x418c4` / `0x4217c` on `0xff86da` in mode 4, mode 4
/// unexercised), draw the timer-3 integer and fractional digits (`0xff8e30` at
/// `0x8003c0`, `0xff8e32` at `0x8003c6`, attr `0xa000`, `D1 = 1` via `0x3bdf2`),
/// branch on timer expiry (`0xff8e35 != 0` -> `0x4193a` / `0x421f2`: draw
/// `TIME OVER` `0x14f74`, queue sound `0x96`, and enter the settle loop at
/// `0x41950` / `0x42208`), or run the object dispatcher (`0x429f2`) and effects
/// (`0x42d22`) and sleep (`0x3b5aa` into `0x4191c` / `0x421d4`).
fn bonus_stage1_frame(bus: &mut Bus, io: &Io, at: &mut Flow, copy: Bonus10Copy) -> Yield {
    let ps = if bus.rb(GAME_MODE) == 4 {
        0xff86da
    } else {
        0xff86b0
    };
    bus.input_handler(ps);
    draw_digits(bus, 0xff8e30, 0x8003c0, 0xa000, 1);
    draw_digits(bus, 0xff8e32, 0x8003c6, 0xa000, 1);
    if bus.rb(0xff8e35) != 0 {
        text_block(bus, 0x14f74);
        bus.queue_sound(0x96);
        return bonus_stage1_settle_step(bus, io, at, copy);
    }
    for ps in bus.dispatch_order() {
        bus.run_player(ps);
    }
    bus.run_effects();
    *at = Flow::BonusStage1Loop { copy };
    speed_sleep(bus, io)
}

/// 0x41950..0x41960 (Copy 1) / 0x42208..0x42218 (Copy 2): one pass of the
/// 10-second bonus settle loop: run the object dispatcher (`0x429f2`) and
/// effects (`0x42d22`) and sleep (`0x3b5aa` into `0x41962` / `0x4221a`).
fn bonus_stage1_settle_step(bus: &mut Bus, io: &Io, at: &mut Flow, copy: Bonus10Copy) -> Yield {
    for ps in bus.dispatch_order() {
        bus.run_player(ps);
    }
    bus.run_effects();
    *at = Flow::BonusStage1Settle { copy };
    speed_sleep(bus, io)
}

/// 0x44a40..0x44a5e: enter the ending (ticket #136, F64): queue ending tune
/// `0x32` (`0x3b7c8`), turn off layers `0xe8` (`0x3b690`), clear the text
/// layer (`0x3bdb0`), and sleep 1 frame (`0x3b51a` into `0x44a60`).
fn ending_start(bus: &mut Bus, at: &mut Flow) -> Yield {
    bus.queue_sound(0x32);
    gfx_off(bus, 0xe8);
    clear_text_layer(bus);
    *at = Flow::EndingStart;
    Yield::Sleep(1)
}

/// 0x44a72..0x44ab2: start scene `d1` (`0..=4`) of the five-scene ending
/// sequence for starting country `0xffc006` (`0x44c1c` table: `0x44c2c` Japan,
/// `0x44c40` USA, `0x44c54` China, `0x44c68` England): request palette
/// `*(0x44c7c + 4 * (entry[0] & 0x0f))` at offset `0` (`0x44a84..0x44aa2`) and
/// run the first sub-step (`d4 = 1`) with `entry[1]`.
fn ending_scene_outer(bus: &mut Bus, at: &mut Flow, d1: u16) -> Yield {
    let scene = ending_scene_address(bus, d1);
    request_ending_scene_palette(bus, scene);
    ending_scene_step(bus, at, d1, 1, bus.rb(scene + 1))
}

fn ending_scene_address(bus: &Bus, d1: u16) -> u32 {
    let country = bus.rb(0xffc006) as u32;
    bus.rl(0x44c1c + (country << 2)) + (u32::from(d1) << 2)
}

fn request_ending_scene_palette(bus: &mut Bus, scene: u32) {
    let pal_idx = (bus.rb(scene) & 0x0f) as u32;
    let pal = bus.rl(0x44c7c + (pal_idx << 2));
    request_palette(bus, pal, 0);
}

fn ending_montage_card(bus: &Bus, d5: u16) -> (u32, bool) {
    if ending_player(bus).is_some_and(|fighter| fighter as u16 == (d5 >> 2) + 2) {
        if let Some(block) = arcade_text::stand_in_card(bus.data_bank) {
            return (block, true);
        }
    }
    (bus.rl(0x44ca4 + u32::from(d5)), false)
}

/// 0x44ab4..0x44ae6: run one sub-step (`d4 = 1` or `0`) of ending scene `d1`:
/// scroll bg to `*(0x44c90 + 2 * (byte >> 4))` (`0x3b682`), draw the 2x2-tile
/// block `*(0x44ca4 + 4 * (byte & 0x0f))` (`0x3bc3e`), and sleep 120 frames
/// (`0x3b56a` into `0x44ae8`).
fn ending_scene_step(bus: &mut Bus, at: &mut Flow, d1: u16, d4: u16, byte: u8) -> Yield {
    bus.ww(0xffc682, 0);
    let scroll_off = ((byte >> 4) as u32) << 1;
    let d5 = ((byte & 0x0f) as u16) << 2;
    let (block, stand_in) = ending_montage_card(bus, d5);
    let scroll = if stand_in {
        request_palette(bus, 0x1ae68, 0);
        0x0900
    } else {
        bus.rw(0x44c90 + scroll_off)
    };
    add_bg_scroll(bus, scroll);
    text_block_2x2(bus, block);
    *at = Flow::EndingSceneHold { d1, d4, d5 };
    Yield::Sleep(bus.rw(0x3b56c) as u8)
}

/// 0x44b00..0x44b1e: stop the montage tune and clear the screen before
/// the congratulatory message. World Tour enters the ending here.
fn ending_message_start(bus: &mut Bus, at: &mut Flow) -> Yield {
    bus.queue_sound(0xff);
    gfx_off(bus, 0xe8);
    clear_text_layer(bus);
    *at = Flow::EndingMessageSetup;
    Yield::Sleep(1)
}

fn ending_player(bus: &Bus) -> Option<TournamentFighter> {
    (bus.preset != crate::config::RulesetPreset::Faithful1987 && bus.rb(MODE_DEMO) != 0)
        .then_some(bus.roster.p1)
}

fn boss_ending(bus: &Bus) -> Option<(LayerView, u32)> {
    let fighter = ending_player(bus)?;
    let block = arcade_text::epilogue(bus.data_bank, fighter)?;
    let view = bus.backgrounds?.view(&ending_key(fighter.name()))?;
    Some((view, block))
}

/// 0x44b6c..0x44b76: step the congratulatory message typewriter (`0x44d60`,
/// `D4 = 5`, `TEXT_SOUND = 0x000d` via `0x3bb98`); when finished, hold 120
/// frames (`0x3b56a` at `0x44b72` into `0x44b78`).
fn ending_message_type(bus: &mut Bus, at: &mut Flow, mut writer: TextWriter) -> Yield {
    match writer.run(bus) {
        Some(frames) => {
            *at = Flow::EndingMessageTyping { writer };
            Yield::Sleep(frames)
        }
        None => {
            *at = Flow::EndingMessageHold;
            Yield::Sleep(bus.rw(0x3b56c) as u8)
        }
    }
}

/// 0x44bcc..0x44bd2: start typing staff block `7 - d1` (`0x44f5c + 4 * (7 -
/// d1)`, `d1 = 7..=0`) with typewriter delay `D4 = 2` (`0x3bb98`).
fn ending_staff_block(bus: &mut Bus, at: &mut Flow, d1: u16) -> Yield {
    let a2 = 0x44f5c + 4 * (7 - d1) as u32;
    let block = bus.rl(a2);
    let writer = TextWriter::new(bus, block, 2);
    ending_staff_type(bus, at, d1, writer)
}

/// 0x44bce..0x44bd8: step the staff block typewriter (`0x3bb98`); when
/// finished, hold 120 frames (`0x3b56a` at `0x44bd4` into `0x44bda`).
fn ending_staff_type(bus: &mut Bus, at: &mut Flow, d1: u16, mut writer: TextWriter) -> Yield {
    match writer.run(bus) {
        Some(frames) => {
            *at = Flow::EndingStaffTyping { d1, writer };
            Yield::Sleep(frames)
        }
        None => {
            *at = Flow::EndingStaffHold { d1 };
            Yield::Sleep(bus.rw(0x3b56c) as u8)
        }
    }
}

/// 0x3f1bc..0x3f1fe: start game over after continue expiry. Sound 0x3f and
/// event 10 accompany the first absolute effect; its table-specified number
/// of run/sleep cycles begins with D3 = 9 further entries.
fn game_over_start(bus: &mut Bus, at: &mut Flow) -> Yield {
    bus.queue_sound(0x3f);
    request_event(bus, 10);
    let template = bus.rl(0x3f278);
    let effect_ticks = bus.rw(0x3f27c);
    bus.spawn_effect_absolute(template, 0);
    bus.run_effects();
    *at = Flow::GameOverCycle {
        index: 0,
        effect_ticks,
        entries_left: 9,
    };
    Yield::Sleep(1)
}

/// 0x3f204..0x3f24c: one run/sleep cycle of the current game-over effect
/// entry. At its end spawn the next entry, applying its layer mask and blank
/// continue digits when requested, or enter the drain when D3 reaches zero.
fn game_over_cycle(
    bus: &mut Bus,
    at: &mut Flow,
    index: u16,
    effect_ticks: u16,
    entries_left: u8,
) -> Yield {
    let effect_ticks = effect_ticks.wrapping_sub(1);
    if effect_ticks != 0 {
        bus.run_effects();
        *at = Flow::GameOverCycle {
            index,
            effect_ticks,
            entries_left,
        };
        return Yield::Sleep(1);
    }
    let index = index + 1;
    let entry = 0x3f278 + ((index as u32) << 3);
    let mask = bus.rw(entry + 6);
    if mask != 0 {
        gfx_off(bus, mask as u8);
        for offset in [0x738, 0x73c, 0x7b8, 0x7bc, 0x7c4] {
            bus.wl(0x800000 + offset, 0x003c003c);
        }
    }
    bus.spawn_effect_absolute(bus.rl(entry), 0);
    let entries_left = entries_left.wrapping_sub(1);
    if entries_left == 0 {
        // 0x3f24e: the drain always performs this last run before sleeping.
        bus.run_effects();
        *at = Flow::GameOverDrain;
        return Yield::Sleep(1);
    }
    bus.run_effects();
    *at = Flow::GameOverCycle {
        index,
        effect_ticks: bus.rw(entry + 4),
        entries_left,
    };
    Yield::Sleep(1)
}

/// 0x3f25a..0x3f272: drain every live effect, then sleep once more before
/// waiting for event 10 to finish its palette cycle and hold.
fn game_over_drain(bus: &mut Bus, at: &mut Flow) -> Yield {
    if bus.live_effect_count() != 0 {
        bus.run_effects();
        *at = Flow::GameOverDrain;
    } else {
        *at = Flow::GameOverAwaitEvent;
    }
    Yield::Sleep(1)
}

/// 0x3f2d0..0x3f2ea: versus game over checks P1 with scene byte 2, then P2
/// with scene byte 1. A qualifying score enters the name-entry screen; two
/// non-qualifying scores are cleared before the shared screen.
fn versus_game_over(bus: &mut Bus, at: &mut Flow) -> Yield {
    bus.wb(SCENE_MODE, 2);
    if let Some(y) = ranking_check(bus, 0xff86b0, RankingReturn::GameOverP1, at) {
        return y;
    }
    versus_game_over_p2(bus, at)
}

/// 0x3f2e0..0x3f2ea: P2's ranking check at the end of a versus game over.
fn versus_game_over_p2(bus: &mut Bus, at: &mut Flow) -> Yield {
    bus.wb(SCENE_MODE, 1);
    if let Some(y) = ranking_check(bus, 0xff86da, RankingReturn::GameOverP2, at) {
        return y;
    }
    game_over_screen(bus, at)
}

/// 0x3f2f0..0x3f326: the screen shared by versus game over and continuation
/// limit expiry: layers off, fg scroll to 0xc00, palette 0x1a268 at 0x200,
/// sound 0x95, a one-frame sleep, then sprites on and the 240-frame hold.
fn game_over_screen(bus: &mut Bus, at: &mut Flow) -> Yield {
    gfx_off(bus, 0xa8);
    bus.ww(0xffc680, 0);
    add_fg_scroll(bus, 0xc00);
    request_palette(bus, 0x1a268, 0x200);
    bus.queue_sound(0x95);
    *at = Flow::GameOverPalette;
    Yield::Sleep(1)
}

/// 0x3f33c..0x3f372: reset the game state. With credits left (`0x3f358:
/// cmpi.l #1, ($65a,A5); bcc $3dec2`), branch directly to `start_screen_entry`
/// (`0x3dec2`, F45); otherwise kill slot 2 and enter the attract join at
/// `0x3ccae` (`Flow::AttractJoin`: spawn slot 3 at `0x3f374` and enter
/// `0x3ccc2` `attract_loop_head`, F66).
fn game_over_reset(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    bus.toggle_state.satsui_awakening = None;
    bus.wb(MODE_DEMO, 0);
    for addr in [0xffc00a, 0xffc002, 0xffc008, 0xffc009, 0xffc004, 0xffc003] {
        bus.wb(addr, 0);
    }
    let credits = bus.rl(0xff8000 + bus.rw(0x3f35e) as u32);
    if credits >= bus.rl(0x3f35a) {
        return start_screen_entry(scheduler, bus, io, at);
    }
    scheduler.kill(bus, 2);
    scheduler.spawn(bus, 3, 0x3f374);
    attract_loop_head(bus, at)
}

/// 0x3efda..0x3f046: enter the continue screen. Checks the continuation
/// limit (`0xffc000 <= 0xffc003` signed -> `0x3f2f0`'s shared screen); otherwise
/// turns off bg + fg (`0x60`), clears winner and round, sets scene byte `8`,
/// clears the text layer and object regions, scrolls fg to `0x0a80` and bg
/// to `0x0600`, requests palette `0x1a268` at offset `0x200`, and sleeps 1
/// frame (`0x3f046`).
fn continue_start(bus: &mut Bus, at: &mut Flow) -> Yield {
    bus.toggle_state.satsui_awakening = None;
    if (bus.rb(0xffc000) as i8) <= (bus.rb(0xffc003) as i8) {
        return game_over_screen(bus, at);
    }
    gfx_off(bus, 0x60);
    bus.wb(0xffc009, 0);
    bus.wb(0xffc00a, 0);
    bus.wb(SCENE_MODE, 8);
    clear_text_layer(bus);
    clear_objects(bus);
    bus.ww(0xffc680, 0);
    bus.ww(0xffc682, 0);
    add_fg_scroll(bus, 0x0a80);
    add_bg_scroll(bus, 0x0600);
    request_palette(bus, 0x1a268, 0x200);
    *at = Flow::ContinuePalette1;
    Yield::Sleep(1)
}

/// 0x3f092..0x3f170: one pass of the continue countdown loop: blinks
/// `CONTINUE` (`0x15b04`, drawn when `0xff8e20 == 0x0909` and erased when
/// `0xff8e20 == 0x0303`), draws the 2x2 big tens and ones digits from
/// `0x15ab4` (`0xff8e1e` / `0xff8e1f`, attr `0x9000`) and the two small
/// fractional digits at `0xff8e20` (`0x3bdf2`), checks for a credit and the
/// active player's start button (`0x3f166`: queues the boot sounds `0x3b7a8`
/// and sleeps 1 frame into `0x3f172`), and checks for countdown expiry
/// `0xff8e23 == 1`, which enters the game-over sequence.
fn continue_frame(bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    // If 0x3f070's immediate byte was zeroed in a negative-control test so
    // counter 1 was not started before 0x3f08c's sleep, start it here (one
    // pass late).
    if bus.rb(0x3f073) == 0
        && bus.rw(0xff8e1e) == 0
        && bus.rw(0xff8e20) == 0
        && bus.rb(0xff8e23) == 1
    {
        bus.wb(TIMER_FLAGS, 2);
        bus.wb(0xff8e23, 0);
    }
    if bus.rw(0xff8e20) == 0x0909 {
        text_block(bus, 0x15b04);
    }
    if bus.rw(0xff8e20) == 0x0303 {
        erase_block(bus, 0x15b04);
    }
    let d1 = (bus.rb(0xff8e1e) as u32) << 3;
    bus.wl(0x800738, bus.rl(0x15ab4 + d1) | 0x9000_9000);
    bus.wl(0x8007b8, bus.rl(0x15ab4 + d1 + 4) | 0x9000_9000);
    let d1 = (bus.rb(0xff8e1f) as u32) << 3;
    bus.wl(0x80073c, bus.rl(0x15ab4 + d1) | 0x9000_9000);
    bus.wl(0x8007bc, bus.rl(0x15ab4 + d1 + 4) | 0x9000_9000);
    draw_digits(bus, 0xff8e20, 0x8007c4, 0x9000, 1);
    if bus.rl(0xff865a) >= 1 {
        let mask = if bus.rb(GAME_MODE) == 2 { 1 } else { 2 };
        if (io.system as u8) & mask == 0 {
            queue_boot_sounds(bus);
            *at = Flow::ContinueAccept;
            return Yield::Sleep(1);
        }
    }
    if bus.rb(0xff8e23) == 1 {
        return game_over_start(bus, at);
    }
    Yield::Sleep(1)
}

/// 0x3ccc2..0x3cce6: the attract loop head: disable layers 0xe8 (0x3b690),
/// clear the text layer (0x3bdb0), request palette 0x1a268 at offset 0x200,
/// and sleep 1 frame (0x3b51a) into 0x3ccec.
fn attract_loop_head(bus: &mut Bus, at: &mut Flow) -> Yield {
    gfx_off(bus, 0xe8);
    clear_text_layer(bus);
    request_palette(bus, 0x1a268, 0x200);
    *at = Flow::AttractStart;
    Yield::Sleep(1)
}

/// 0x3ce8c..0x3ceb8: end the attract demo round and return to the attract
/// loop head at 0x3ccc2 (ticket #127, F55): queue sound 0x3f (0x3b7c8),
/// restore 0xffc007 from 0xff8f28, clear all three object regions (0x3b9bc /
/// 0x3b9c8 / 0x3b9d4), kill slot 2 (0x3ceb2 -> 0x3b230), and jump to 0x3ccc2.
fn attract_demo_end(scheduler: &mut Scheduler, bus: &mut Bus, at: &mut Flow) -> Yield {
    bus.queue_sound(0x3f);
    bus.wb(0xffc007, bus.rb(0xff8f28));
    clear_objects(bus);
    scheduler.kill(bus, 2);
    attract_loop_head(bus, at)
}

/// 0x3eba2: record `winner`'s round win. Outside demo play (0x3ce8c) its
/// wins count up and its round markers are drawn; the round result (scene
/// event 7) is requested with the winner recorded (0xff8e38); the winner
/// starts its victory pose (move 0x0c, 0x441b0), a human winner's input
/// handler running once; then the victory loop (0x3ebf0).
fn record_win(
    scheduler: &mut Scheduler,
    bus: &mut Bus,
    io: &Io,
    at: &mut Flow,
    winner: u32,
) -> Yield {
    if bus.rb(MODE_DEMO) == 0 {
        return attract_demo_end(scheduler, bus, at);
    }
    bus.wb(winner + 8, bus.rb(winner + 8).wrapping_add(1));
    round_markers(bus, winner);
    bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) | 0x20);
    bus.wb(0xff8e36, 7);
    bus.wl(WINNER, winner);
    let pose = if bus.dramatic_rules().is_some()
        && winner == P1_STRUCT && bus.toggle_state.dramatic.health[0] == 0
    {
        P2_STRUCT
    } else {
        winner
    };
    let me = bus.rw(pose + 0x10);
    bus.start_move(me, pose, 0x0c);
    if (pose != CPU_STRUCT || bus.rl(CPU_STRUCT + 0x12) == 0x1605c) && !bus.boss_on_struct(pose) {
        bus.input_handler(pose);
    }
    *at = Flow::Victory;
    settle_frame(bus, io)
}

/// The time-over and double-KO loops' frame (0x3ed72 / 0x3eca6): the stage
/// drift, the object dispatcher, effects, camera requests and the TIME
/// digits (0x3beb4), before the loop's sleep.
fn outcome_frame(bus: &mut Bus) {
    bus.drift_stage();
    bus.dispatch_fighters();
    bus.run_effects();
    bus.resolve_camera();
    draw_time_digits(bus);
}

/// 0x3ed4a: one pass of the time-over hold: the counter down; at zero the
/// judgement (0x3ed98); else the fighters' frame while either is busy,
/// then the sleep.
fn time_over_hold(
    scheduler: &mut Scheduler,
    bus: &mut Bus,
    io: &Io,
    at: &mut Flow,
    left: u16,
) -> Yield {
    let left = left.saturating_sub(1);
    let demon = &bus.toggle_state.raging_demon;
    let demon_settling = demon.active || demon.victory_pose;
    if left == 0 && !demon_settling {
        return time_over_judge(scheduler, bus, io, at);
    }
    if demon_settling || fighters_busy(bus) {
        outcome_frame(bus);
    }
    *at = Flow::TimeOverHold { left };
    speed_sleep(bus, io)
}

/// 0x3ed98: the TIME digits copied to 0xff8e3c, TIME OVER erased, and the
/// two fighters' health points (0x3c0b0) compared: the higher wins
/// (0x3eba0 / 0x3eba2); equal is a draw — outside demo play (0x3ce8c) the
/// draw text (event 8), then the round's end.
fn time_over_judge(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    for i in 0..3 {
        bus.wb(TIME_COPY + i, bus.rb(TIME_DIGITS + i));
    }
    erase_block(bus, 0x14f74);
    if let Some(rules) = bus.world_tour_rules() {
        use crate::world_tour::native::{MissionKind, RuleModifier};
        match rules.mission {
            MissionKind::TimeAttack { .. } | MissionKind::SurvivalGauntlet { .. } => {
                return record_win(scheduler, bus, io, at, P2_STRUCT);
            }
            MissionKind::RuleModifierTrial(RuleModifier::ProjectileDefense) => {
                let winner = bus.rl(WINNER);
                return record_win(scheduler, bus, io, at, winner);
            }
            _ => {}
        }
    }
    let (this, other, d2, d0) = if bus.dramatic_rules().is_some() {
        use crate::dramatic::{ALLY_SUB_HP, BOSS_SUB_HP};

        let [ally1, boss, ally2] = bus.toggle_state.dramatic.health;
        let team = u32::from(ally1.max(ally2)) * u32::from(BOSS_SUB_HP);
        let boss = u32::from(boss) * u32::from(ALLY_SUB_HP);
        (P1_STRUCT, CPU_STRUCT, team, boss)
    } else {
        let this = bus.rl(0xffe438);
        let other = bus.rl(0xffe43c);
        let (this_hp, other_hp) = match (bus.era_health(this), bus.era_health(other)) {
            (Some((a, a_max)), Some((b, b_max))) => (a * b_max, b * a_max),
            _ => (u32::from(health_points(bus, this)), u32::from(health_points(bus, other))),
        };
        (this, other, this_hp, other_hp)
    };
    if d2 < d0 {
        return record_win(scheduler, bus, io, at, other);
    }
    if d2 > d0 {
        return record_win(scheduler, bus, io, at, this);
    }
    if bus.rb(MODE_DEMO) == 0 {
        return attract_demo_end(scheduler, bus, at);
    }
    request_event(bus, 8);
    *at = Flow::AwaitDraw;
    speed_sleep(bus, io)
}

/// 0x3eca2: the double KO: scene mode 0, one frame of the fighters' work
/// with the TIME digits, then the sleep; the loop goes round while either
/// fighter is busy (0x3ecca).
fn double_ko_start(bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    bus.wb(SCENE_MODE, 0);
    outcome_frame(bus);
    *at = Flow::DoubleKoSettle;
    speed_sleep(bus, io)
}

/// 0x3ecf0: one pass of the double KO text's hold: the counter down; at
/// zero the text is erased and, outside demo play (0x3ce8c), the round
/// ends (0x3edf6); else the sleep.
fn double_ko_hold(
    scheduler: &mut Scheduler,
    bus: &mut Bus,
    io: &Io,
    at: &mut Flow,
    left: u16,
) -> Yield {
    let left = left - 1;
    if left == 0 {
        erase_block(bus, 0x14f56);
        if bus.rb(MODE_DEMO) == 0 {
            return attract_demo_end(scheduler, bus, at);
        }
        return after_round(bus, io, at);
    }
    *at = Flow::DoubleKoHold { left };
    speed_sleep(bus, io)
}

/// 0x3ec40 / 0x3edf6: the round counter up; a third round played goes to
/// the three-rounds rule (0x3ee04), else the next round's wipe (event 6,
/// 0x3ec4e) is requested.
fn after_round(bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    if bus.world_tour_rules().is_some_and(|rules| rules.single_round()) {
        bus.wb(0xffc009, 0);
        return match_end(bus, io, at);
    }
    bus.wb(0xffc00a, bus.rb(0xffc00a).wrapping_add(1));
    if bus.rb(0xffc00a) == 3 {
        return three_rounds(bus, io, at);
    }
    request_event(bus, 6);
    *at = Flow::AwaitWipe;
    speed_sleep(bus, io)
}

fn start_satsui_awakening(bus: &mut Bus) {
    use TournamentFighter::{Ryu, Sagat};

    let player = match bus.rb(GAME_MODE) {
        2 => AwakeningSide::Player,
        4 => AwakeningSide::PlayerTwo,
        _ => return,
    };
    if bus.roster.mode != PlayMode::Arcade
        || !bus.roster.full_cast
        || !bus.era_rules.enabled
        || bus.data_bank.is_none()
        || bus.rb(MODE_DEMO) == 0
        || bus.evil_ryu_playtest.is_some()
        || bus.dramatic_rules().is_some()
        || bus.world_tour_rules().is_some()
        || bus.toggle_state.satsui_awakening.is_some()
        || bus.rb(0xffc003) != 4
        || bus.rb(0xffc004) != 1
        || bus.rb(0xffc00a) != 2
        || bus.rb(player.fighter_struct() + 8) != 1
        || bus.rb(CPU_STRUCT + 8) != 1
    {
        return;
    }

    let side = match (bus.roster.p1, bus.arcade_opponent()) {
        (Ryu, Some(Sagat)) => player,
        (Sagat, Some(Ryu)) => AwakeningSide::Cpu,
        _ => return,
    };
    let Some(fighter) = bus.era_state(side.fighter_struct()) else { return };
    bus.toggle_state.satsui_awakening = Some(SatsuiAwakening {
        side,
        profile: EvilRyuPlaytest::for_awakening(fighter.style),
        surge_pending: true,
    });
}

/// 0x3ee04: three rounds played: the fighter with more wins takes the
/// match (0x3ec6e / 0x3ec70: 0xffc009 = 1 P1, 2 P2, 4 CPU); equal wins
/// leave no winner (0xffc009 = 0), which the one-player modes treat as the
/// CPU's match win and versus as a game over (0x3ee26).
fn three_rounds(bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    let this = bus.rl(0xffe438);
    let other = bus.rl(0xffe43c);
    let (d2, d0) = (bus.rb(this + 8), bus.rb(other + 8));
    if bus.toggles.real_second_timer_and_draws && d2 == d0 {
        // Sudden Death / Draw decider round (#97, O6): keep round index at 2
        // (valid 3rd-round announcement/result/draw table index) and start a
        // decider round instead of forcing a CPU win / player Game Over.
        bus.wb(0xffc00a, 2);
        request_event(bus, 6);
        *at = Flow::AwaitWipe;
        return speed_sleep(bus, io);
    }
    let who = if d2 == d0 {
        0
    } else {
        match if d2 < d0 { other } else { this } {
            0xff86b0 => 1,
            0xff86da => 2,
            _ => 4,
        }
    };
    bus.wb(0xffc009, who);
    match_end(bus, io, at)
}

/// The stage the roster picked for this Local or Training fight, when the
/// machine holds the extended background bank (ADR-0009, ticket #198): the
/// overhaul-only stage gates read its scroll words, pages, palettes and
/// theme in place of the 1987 country tables, and nothing writes work RAM
/// in their place (ADR-0004). `None`, every gate closed, in every other
/// play mode, with no stage picked, and on every machine without the bank:
/// the attract, the faithful cabinet and Faithful 1987 fights.
fn picked_stage(bus: &Bus) -> Option<StageView> {
    if !matches!(bus.roster.mode, PlayMode::Versus | PlayMode::Training) {
        return None;
    }
    match bus.roster.stage? {
        StagePick::Bank(index) => bus.backgrounds?.stage(index),
    }
}

/// The VS screen built for this front-end pair in the extended
/// background bank (ADR-0009, first amendment; ticket #215): Player 1's
/// bust and name on the left, his opponent's on the right. Arcade resolves
/// the current ladder opponent, including Ryu standing in for the player's
/// own slot (#395), rather than using the roster's fixed Player 2 pick.
/// The overhaul-only gate scrolls to its page and loads its palette, and
/// writes no work RAM in their place (ADR-0004). `None` keeps the 1987
/// branch for Faithful 1987, Ryu against Ken (the ROM's own screen), demo
/// play, and every machine without the bank.
fn picked_vs(bus: &Bus) -> Option<LayerView> {
    if bus.preset == crate::config::RulesetPreset::Faithful1987 {
        return None;
    }
    let opponent = match bus.roster.mode {
        PlayMode::Versus | PlayMode::Training => bus.roster.p2,
        PlayMode::Arcade if bus.rb(MODE_DEMO) != 0 && matches!(bus.rb(GAME_MODE), 2 | 4) => {
            bus.arcade_opponent()?
        }
        _ => return None,
    };
    let bank = bus.backgrounds?;
    if let Some(playtest) = bus.evil_ryu_playtest {
        if let Some(view) = bank.view(&vs_key(playtest.display_name(), opponent.name())) {
            return Some(view);
        }
    }
    bank.view(&vs_key(bus.roster.p1.name(), opponent.name()))
}

/// Whether the country intro (events 2 and 3) is skipped: in every fight
/// the stage gates serve, so the stage screen leads straight to the VS
/// screen (ticket #193; design section 4, rule 8). The intro would show
/// the hidden flag's country, which the picked stage never matches. An
/// overhaul-only gate on the translated path, with nothing written to work
/// RAM (ADR-0004); closed, the intro runs as in 1987, in Arcade, in
/// Faithful 1987 and wherever no stage is picked.
fn skips_country_intro(bus: &Bus) -> bool {
    picked_stage(bus).is_some()
}

/// 0x437d8: the stage position for this opponent: the scroll copies
/// cleared, fg_scroll from the country's table (+0xc0) and bg_scroll from
/// its second table, camera 0xc0, the drift position 0. The stage gate
/// gives the picked stage's words instead, and puts both layers on its
/// page; the camera and the drift then move them as they move a 1987
/// stage.
fn stage_reset(bus: &mut Bus) {
    let picked = picked_stage(bus);
    let c = ((sext(bus.rb(0xffc006)) as u32) << 2) & 0xffff;
    let fg_table = bus.rl(0x437ee + c);
    let bg_table = bus.rl(0x437fe + c);
    bus.ww(0xffc680, 0);
    bus.ww(0xffc682, 0);
    let d2 =
        ((((sext(bus.rb(0xffc003)) << 1) as u8).wrapping_add(bus.rb(0xffc004)) as u16) << 1) as u32;
    let fg_word = picked.map_or_else(|| bus.rw(fg_table + d2), |s| s.fg.scroll);
    add_fg_scroll(bus, fg_word.wrapping_add(0xc0));
    bus.ww(0xffc00c, 0xc0);
    let bg = picked.map_or_else(|| bus.rw(bg_table + d2), |s| s.bg.scroll);
    add_bg_scroll(bus, bg);
    bus.ww(0xffc00e, 0);
    if let Some(stage) = picked {
        *bus.fg_page = stage.fg.page;
        *bus.bg_page = stage.bg.page;
    }
}

fn sext(b: u8) -> u16 {
    b as i8 as i16 as u16
}

/// 0x3b674: fg_scroll copy (0xffc680) += d, and the register written.
///
/// The 1987 code clears the copy before every absolute scroll, so an add
/// to a cleared copy also puts the layer back on the ROM tilemap's page
/// (ADR-0009): a page an overhaul gate chose lasts while the layer only
/// moves relative to it (the camera, the drift) and ends at the next
/// screen the 1987 code scrolls to.
fn add_fg_scroll(bus: &mut Bus, d: u16) {
    let copy = bus.rw(0xffc680);
    if copy == 0 {
        *bus.fg_page = 0;
    }
    let v = copy.wrapping_add(d);
    bus.ww(0xffc680, v);
    *bus.fg_scroll = v;
}

/// 0x3b682: bg_scroll copy (0xffc682) += d, and the register written; an
/// add to a cleared copy puts the layer back on the ROM's page, as in
/// [`add_fg_scroll`].
fn add_bg_scroll(bus: &mut Bus, d: u16) {
    let copy = bus.rw(0xffc682);
    if copy == 0 {
        *bus.bg_page = 0;
    }
    let v = copy.wrapping_add(d);
    bus.ww(0xffc682, v);
    *bus.bg_scroll = v;
}

/// 0x3b690: gfxctrl copy (0xff8682) &= !mask, and the register written.
fn gfx_off(bus: &mut Bus, mask: u8) {
    let v = bus.rb(0xff8682) & !mask;
    bus.wb(0xff8682, v);
    *bus.gfxctrl = v;
    *bus.flip_screen = v & 4 != 0;
}

/// 0x3b644: gfxctrl copy |= mask, and the register written.
fn gfx_on(bus: &mut Bus, mask: u8) {
    let v = bus.rb(0xff8682) | mask;
    bus.wb(0xff8682, v);
    *bus.gfxctrl = v;
    *bus.flip_screen = v & 4 != 0;
}

/// Presentation metadata for one native stage-music queue slot. The native
/// byte written to 68000 work RAM never depends on the installed music pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StageMusic {
    pub cue: Option<crate::theme::ThemeCue>,
    pub legacy: u8,
    pub event: u32,
}

/// 0x42332: the native stage music for this opponent. Overhaul playback
/// carries a semantic request beside the native sound-latch write.
fn stage_music(bus: &mut Bus) {
    use crate::theme::{ThemeCue, ThemeId};
    use sf_video::background_bank::{EXTENDED_THEME_FIRST, FALLBACK_THEME};

    let c = ((sext(bus.rb(0xffc006)) as u32) << 2) & 0xffff;
    let table = bus.rl(0x42358 + c);
    let d0 =
        ((((sext(bus.rb(0xffc003)) << 1) as u8).wrapping_add(bus.rb(0xffc004)) as u16) << 1) as u32;
    let rom_cmd = bus.rw(table + d0);
    if bus.preset == crate::config::RulesetPreset::Faithful1987 || bus.rb(MODE_DEMO) == 0 {
        bus.queue_sound(rom_cmd);
        return;
    }

    let bank = bus.backgrounds;
    let authored = if matches!(bus.roster.mode, PlayMode::Versus | PlayMode::Training) {
        let index = bus.roster.stage.map(|StagePick::Bank(index)| index);
        index.and_then(|index| bank?.stage_record(index)).map(|stage| stage.theme)
    } else {
        None
    };
    let stage_cmd = authored.unwrap_or(rom_cmd);
    let fallback = if stage_cmd < EXTENDED_THEME_FIRST { stage_cmd } else { FALLBACK_THEME };
    let id = bus.toggle_state.selected_theme.map(|theme| theme.id)
        .or_else(|| {
            (bus.dramatic_rules().is_some() || bus.world_tour_rules().is_some())
                .then(|| ThemeId::from(bus.roster.p2))
        })
        .or_else(|| {
            if bus.roster.mode == PlayMode::Arcade {
                bus.arcade_opponent().map(ThemeId::from)
            } else {
                None
            }
        })
        .or_else(|| u8::try_from(stage_cmd).ok().and_then(ThemeId::from_command));
    let command = id.map_or(stage_cmd, |theme| u16::from(theme.command()));
    let legacy = u8::try_from(command).ok()
        .filter(|&command| u16::from(command) < EXTENDED_THEME_FIRST || bank.is_some_and(|b| b.can_play(command)))
        .unwrap_or(fallback as u8);
    if id == Some(ThemeId::Ryu) && bus.roster.mode == PlayMode::Arcade && legacy != ThemeId::Ryu.command() {
        bus.toggle_state.missing_ryu_theme.get_or_insert(fallback);
    }
    let cue = id.map(|id| ThemeCue { id, fallback: fallback as u8 });
    bus.queue_stage_music(fallback, cue, legacy);
}

/// 0x3e78c..0x3e81c (and its one-player twins): the round's HUD and the
/// fighters: the texts, scores, top score and names; full health for both
/// fighters; their starting moves (the placement entries 0x44114..); the
/// health bars and round markers. Returns false for a game mode whose path
/// is not translated.
fn round_hud(bus: &mut Bus) -> bool {
    struct Layout {
        texts: &'static [u32],
        scores: &'static [u32],
        names: (u32, u32),
        a: u32,
        b: u32,
        a_place: (u16, Option<u8>),
        b_place: (u16, Option<u8>),
    }
    let layout = match bus.rb(GAME_MODE) {
        2 => Layout {
            texts: &[0x14ef0, 0x14f06, 0x14ed2, 0x14edc],
            scores: &[0xff86b0],
            names: (0x14f2c, 0x14f3c),
            a: 0xff86b0,
            b: 0xffc664,
            a_place: (0, None),
            b_place: (0x1c0, Some(4)),
        },
        4 => Layout {
            texts: &[0x14ef0, 0x14f06, 0x14ee6, 0x14edc],
            scores: &[0xff86da],
            names: (0x14f34, 0x14f3c),
            a: 0xff86da,
            b: 0xffc664,
            a_place: (0, None),
            b_place: (0x40, Some(2)),
        },
        _ => Layout {
            texts: &[0x14ef0, 0x14f06, 0x14ed2, 0x14edc, 0x14ee6],
            scores: &[0xff86b0, 0xff86da],
            names: (0x14f2c, 0x14f34),
            a: 0xff86b0,
            b: 0xff86da,
            a_place: (0, None),
            b_place: (0, None),
        },
    };
    for t in layout.texts {
        text_block(bus, *t);
    }
    for ps in layout.scores {
        bus.show_score(*ps);
    }
    bus.show_digits(0xffc012, 0x1bc, 7); // 0x3be42: the top score
                                         // 0x3be1e / 0x3be26: the two name plates (7 cells, attr 0x4000).
    for (cell, name) in [(0x2acu32, layout.names.0), (0x3ac, layout.names.1)] {
        for i in 0..7 {
            bus.ww(0x800000 + cell + 2 * i, 0x4000 | bus.rb(name + i) as u16);
        }
    }
    let dramatic = bus.dramatic_rules().is_some();
    if dramatic {
        bus.toggle_state.dramatic.reset_round();
    }
    if bus.evil_ryu_playtest.is_some() || bus.toggle_state.satsui_awakening.is_some() {
        bus.toggle_state.ashura_motion = [[0; 2]; 2];
        bus.toggle_state.raging_demon = Default::default();
    }
    bus.bind_roster_tables();
    bus.era_begin_round();
    for (ps, place) in [(layout.a, layout.a_place), (layout.b, layout.b_place)] {
        let (x, facing) = bus.roster_walk_in(ps).map_or(place, |(x, f)| (x, Some(f)));
        bus.wb(ps + 6, 4);
        bus.wb(ps + 7, 0xb);
        let me = bus.rw(ps + 0x10);
        if let Some(f) = facing {
            bus.wb(0xffe000 + me as u32 + 0x37, f);
        }
        bus.start_move_at(me, ps, 0, x);
    }
    if dramatic {
        bus.wb(P2_STRUCT + 6, 4);
        bus.wb(P2_STRUCT + 7, 0xb);
        bus.set_eb(DRAMATIC_ALLY2, 0x37, 2);
        bus.start_move_at(DRAMATIC_ALLY2, P2_STRUCT, 0, 0);
        bus.position_dramatic_ally();
    }
    health_bars(bus);
    bus.world_tour_apply_starting_hp();
    round_markers(bus, layout.a);
    round_markers(bus, layout.b);
    true
}

/// 0x3bf22: draw both empty health bars: 0x1525, twelve 0x1120, 0x1125 at
/// 0x8002ba and 0x8003ba.
fn health_bars(bus: &mut Bus) {
    for start in [0x8002bau32, 0x8003ba] {
        bus.ww(start, 0x1525);
        for i in 0..12 {
            bus.ww(start + 2 + 2 * i, 0x1120);
        }
        bus.ww(start + 26, 0x1125);
    }
}

/// Either fighter's move in progress (+9 bit 7).
fn fighters_busy(bus: &Bus) -> bool {
    bus.rb(0xffe409) & 0x80 != 0 || bus.rb(0xffea09) & 0x80 != 0
        || (bus.dramatic_rules().is_some() && bus.eb(DRAMATIC_ALLY2, 9) & 0x80 != 0)
}

/// The KO and victory loops' frame (0x3eb44 / 0x3ebf0): the stage drift,
/// the object dispatcher, effects and camera requests, then a sleep.
fn settle_frame(bus: &mut Bus, io: &Io) -> Yield {
    bus.drift_stage();
    bus.dispatch_fighters();
    bus.run_effects();
    bus.resolve_camera();
    speed_sleep(bus, io)
}

/// 0x3b5aa: one frame, or six with the Speed DIP set to slow or during the
/// KO slow motion (0xffc00b).
fn speed_sleep(bus: &Bus, io: &Io) -> Yield {
    if io.dsw1 & 0x1000 != 0 && bus.rb(SLOW_MOTION) == 0 {
        Yield::Sleep(1)
    } else {
        Yield::Sleep(6)
    }
}

/// 0x3bed2: draw a player's round markers: for each win (+8) a 2 x 2 block
/// of tiles 0x1100.. from the video offset at +0xc, 4 bytes apart.
fn round_markers(bus: &mut Bus, ps: u32) {
    let wins = bus.rb(ps + 8) as u16;
    let base = bus.rw(ps + 0xc);
    for k in 0..wins {
        let at = base.wrapping_add(4 * k);
        for r in 0..2u16 {
            for c in 0..2u16 {
                bus.ww(
                    0x800000 + at.wrapping_add(0x80 * r + 2 * c) as u32,
                    0x1100 + 0x10 * r + c,
                );
            }
        }
    }
}

/// The screen-event task (slot 2, 0x3f7b0): the poll, and the round result
/// (scene event 7, 0x3fcd4) for a CPU win.
fn screen(bus: &mut Bus, io: &Io, at: &mut Screen) -> Yield {
    match *at {
        Screen::Poll => {
            // 0x3f7b4: a requested scene event runs its handler.
            if bus.rb(SCENE_MODE) & 0x20 == 0 {
                return Yield::Sleep(1);
            }
            let n = bus.rb(0xff8e36);
            match n {
                6 => {
                    // 0x3fc8a: the next-round wipe (tile 0x4a).
                    *at = Screen::Wipe {
                        column: 0,
                        d2: 0x170,
                        tile: 0x4a,
                        announce: false,
                    };
                    return screen(bus, io, at);
                }
                5 => {
                    // 0x3fba2: the round announcement: the text and
                    // background layers off; outside the 0xa0 scene the
                    // screen is cleared (0x422ba) and the task sleeps 6.
                    gfx_off(bus, 0x60);
                    if bus.rb(SCENE_MODE) != 0xa0 {
                        for i in 0..0x800u32 {
                            bus.ww(0x800000 + 2 * i, 0x4a);
                        }
                        *at = Screen::AnnounceCleared;
                        return Yield::Sleep(6);
                    }
                    *at = Screen::AnnounceCleared;
                    return screen(bus, io, at);
                }
                7 => {}
                0xb => return win_screen_start(bus, at),
                2 => {
                    // 0x3f808..0x3f80e: stage 0 skips the travel map
                    // (0x3f812, ticket #83, F43) and goes directly to the
                    // country screen (event 3, 0x3f8e4).
                    if bus.rb(0xffc003) != 0 {
                        return travel_map_start(bus, at);
                    }
                    return country_intro_start(bus, io, at);
                }
                3 => return country_intro_start(bus, io, at),
                4 => return opponent_intro_start(bus, io, at),
                8 => {
                    // 0x3fef2: the draw text by round (table 0x3ff18),
                    // typed (3 frames a cell) and held 120 frames; the
                    // next wipe clears it.
                    let text = bus.rl(0x3ff18 + ((bus.rb(0xffc00a) as u32) << 2));
                    let writer = TextWriter::new(bus, text, 3);
                    *at = Screen::DrawTyping { writer };
                    return screen(bus, io, at);
                }
                9 => {
                    // 0x3ff2c: the slow motion: the request consumed, the
                    // fight loop slowed (0xffc00b) for 60 + 30 frames.
                    bus.wb(0xff8e36, 0);
                    bus.wb(SLOW_MOTION, 0xff);
                    *at = Screen::SlowMotion { first: true };
                    return Yield::Sleep(60);
                }
                10 => {
                    *at = Screen::GameOverPalette { next: 0 };
                    return screen(bus, io, at);
                }
                _ => {
                    let handler = bus.rl(0x3f7d8 + ((n.wrapping_sub(1) as u16) << 2) as u32);
                    bus.note(handler);
                    return Yield::Halt;
                }
            }
            // 0x3fcd4: the result text by round and winner: the CPU's
            // from the table 0x3fd7a (0x3fd30); a human winner's (0x3fcf4
            // P1: 0x3fd52, 0x3fd12 P2: 0x3fd66) after sound 0x35.
            let winner = bus.rl(WINNER);
            let table = match winner {
                0xffc664 => 0x3fd7a,
                0xff86da => 0x3fd66,
                _ => 0x3fd52,
            };
            if winner != 0xffc664 {
                bus.queue_sound(0x35);
            }
            let text = bus.rl(table + ((bus.rb(0xffc00a) as u32) << 2));
            let writer = TextWriter::new(bus, text, 3);
            *at = Screen::ResultTyping { writer, text };
            screen(bus, io, at)
        }
        Screen::ResultTyping { mut writer, text } => match writer.run(bus) {
            Some(frames) => {
                *at = Screen::ResultTyping { writer, text };
                Yield::Sleep(frames)
            }
            None => {
                if bus.rl(WINNER) == 0xffc664 {
                    // 0x3fd40: hold the text 120 frames.
                    *at = Screen::ResultHeld { text };
                    Yield::Sleep(120)
                } else {
                    // 0x3fd8e: hold it 30 frames, then the bonus screen.
                    *at = Screen::ResultTyped { text };
                    Yield::Sleep(30)
                }
            }
        },
        Screen::ResultTyped { text } => {
            // 0x3fd94: the result text erased and the bonus block
            // (0x1561c) shown. The TIME copy is drawn (at first only when
            // it is not zero); the winner's vitality (0x3c0b0: cells * 4 +
            // remainder) becomes its digits (0x3bf5a) and is drawn; the
            // bonus digits are cleared; TIME and vitality are drawn again,
            // then a 30-frame wait.
            erase_block(bus, text);
            text_block(bus, 0x1561c);
            if !time_copy_zero(bus) {
                draw_time_copy(bus);
            }
            let ps = bus.rl(WINNER);
            let points = health_points(bus, ps);
            bus.wl(VITALITY, 0);
            binary_to_digits(bus, points, VITALITY + 4);
            draw_vitality(bus);
            for i in 0..5 {
                bus.wb(BONUS + i, 0);
            }
            draw_time_copy(bus);
            draw_vitality(bus);
            *at = Screen::TallyStart;
            Yield::Sleep(30)
        }
        Screen::TallyStart | Screen::TallyTime => tally_time(bus, at),
        Screen::TallyVitality => tally_vitality(bus, at),
        Screen::WinText { text } => {
            // 0x4014c (0x4006c in versus): the text erased; the objects
            // cleared, the sprites off and the text layer cleared
            // (0x3bdb0). In the one-player modes a human winner (0xffc009
            // & 3) gets the text 0x158e4 with sound 0x9e (the last
            // opponent: 0x159b2, 0xba) over the background scrolled to the
            // country table's word for this opponent (0x4018a), held 180 +
            // 30 frames; the CPU's win (0x40240) shows 0x15848 with sound
            // 0x9d over its table's position (0x40250) for 240. In versus
            // the background goes to the winner's word of 0x400d8 with the
            // palette 0x1ae68 requested and sound 0x9c, then after a frame
            // the text 0x15a78 is held 120 frames.
            erase_block(bus, text);
            if bus.world_tour_rules().is_some() {
                // The mission already showed the result. Its roster has no
                // corresponding entry in the Arcade or versus win-card table.
                bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & !0x20);
                *at = Screen::Poll;
                return Yield::Sleep(1);
            }
            clear_objects(bus);
            gfx_off(bus, 0x40);
            clear_text_layer(bus);
            if !matches!(bus.rb(GAME_MODE), 2 | 4) {
                let who = sext(bus.rb(0xffc009));
                let d1 = (((who & 0xff00) | (who as u8).wrapping_sub(1) as u16) << 2) >> 1;
                bus.ww(0xffc682, 0);
                let d = bus.rw(0x400d8u32.wrapping_add(d1 as i16 as i32 as u32));
                add_bg_scroll(bus, d);
                bus.wl(0xff8634, 0x1ae68);
                bus.ww(0xff8638, 0);
                bus.wb(PALETTE_REQUEST, 0xff);
                bus.queue_sound(0x9c);
                *at = Screen::VersusWinCard { shown: false };
                return Yield::Sleep(1);
            }
            let country = ((sext(bus.rb(0xffc006)) as u32) << 2) & 0xffff;
            let opponent = opponent_index(bus);
            let at_word = |table: u32| table.wrapping_add((opponent << 1) as i16 as i32 as u32);
            if bus.rb(0xffc009) & 3 != 0 {
                let table = bus.rl(0x4018a + country);
                let finale = opponent as u8 == 9;
                let (text, sound) = if finale {
                    (0x159b2, 0xba)
                } else {
                    (0x158e4, 0x9e)
                };
                let player = ending_player(bus).filter(|_| bus.roster.mode == PlayMode::Arcade);
                let quote = player.and_then(|fighter| arcade_text::quote(bus.data_bank, fighter, finale));
                bus.queue_sound(sound);
                text_block(bus, quote.unwrap_or(text));
                bus.ww(0xffc682, 0);
                let stand_in = player.is_some_and(|fighter| {
                    !fighter.is_shoto() && bus.arcade_opponent() == Some(TournamentFighter::Ryu)
                });
                let d = if stand_in {
                    request_palette(bus, 0x1ae68, 0);
                    0x0900
                } else {
                    bus.rw(at_word(table))
                };
                add_bg_scroll(bus, d);
                *at = Screen::WinCard { held: false };
                Yield::Sleep(180)
            } else {
                let table = bus.rl(0x40250 + country);
                bus.ww(0xffc682, 0);
                let d = bus.rw(at_word(table));
                add_bg_scroll(bus, d);
                text_block(bus, 0x15848);
                bus.queue_sound(0x9d);
                *at = Screen::WinCardCpu;
                Yield::Sleep(240)
            }
        }
        Screen::WinCard { held: false } => {
            *at = Screen::WinCard { held: true };
            Yield::Sleep(30)
        }
        Screen::VersusWinCard { shown: false } => {
            // 0x400c2: the winner's text, held 120 frames.
            text_block(bus, 0x15a78);
            *at = Screen::VersusWinCard { shown: true };
            Yield::Sleep(120)
        }
        Screen::VersusWinCard { shown: true } => {
            // 0x402ee: the event is done.
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & 0xdf);
            *at = Screen::Poll;
            Yield::Sleep(1)
        }
        Screen::WinCard { held: true } | Screen::WinCardCpu => {
            // 0x402ee: the event is done.
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & 0xdf);
            *at = Screen::Poll;
            Yield::Sleep(1)
        }
        Screen::OpponentIntro { stage: 0 } => {
            // 0x3fb28: the 120-frame hold.
            *at = Screen::OpponentIntro { stage: 1 };
            Yield::Sleep(120)
        }
        Screen::OpponentIntro { stage: 1 } => {
            // 0x3fb2e: the 30-frame hold.
            *at = Screen::OpponentIntro { stage: 2 };
            Yield::Sleep(30)
        }
        Screen::VersusIntro { stage: 0 } => {
            *at = Screen::VersusIntro { stage: 1 };
            Yield::Sleep(30)
        }
        Screen::VersusIntro { stage: 1 } => {
            // 0x3fa4c: the stage palette request and its sleep.
            request_stage_palette(bus);
            *at = Screen::VersusIntro { stage: 2 };
            speed_sleep(bus, io)
        }
        Screen::TravelMapSpawn { a1 } => {
            // 0x3f87c..0x3f88e: spawn the travel-map plane effect at
            // `rl(a1)` (`0x42f1e`), run effects once (`0x42d22`), and sleep
            // 1 frame (`0x3b51a` into `0x3f890`).
            let template = bus.rl(a1);
            bus.spawn_effect_absolute(template, 0);
            bus.run_effects();
            *at = Screen::TravelMapLoop;
            Yield::Sleep(1)
        }
        Screen::TravelMapLoop => {
            // 0x3f890..0x3f89e: while any effect remains live (`0x4329c`),
            // run effects (`0x42d22`) and sleep 1 frame (`0x3b51a` into
            // `0x3f890`); when the plane despawns, hold 60 frames (`0x3b562`
            // into `0x3f8a0`).
            if bus.live_effect_count() != 0 {
                bus.run_effects();
                *at = Screen::TravelMapLoop;
                Yield::Sleep(1)
            } else {
                *at = Screen::TravelMapEnd;
                Yield::Sleep(60)
            }
        }
        Screen::TravelMapEnd => {
            // 0x3f8a0: `bra.w $3f8e4` into the country intro (event 3).
            country_intro_start(bus, io, at)
        }
        Screen::CountryIntro { a1, stage: 0 } => {
            // 0x3f93c..0x3f958: queue the country voice and draw its text
            // block, then hold 60 frames (0x3b562: move.w #$3c, D0).
            let d2 = bus.rw(a1 + 2) as u32;
            bus.queue_sound(bus.rw(0x3f992 + (d2 << 1)));
            text_block(bus, bus.rl(0x3f97e + (d2 << 2)));
            *at = Screen::CountryIntro { a1, stage: 1 };
            Yield::Sleep(bus.rw(0x3b564) as u8)
        }
        Screen::CountryIntro { a1, stage: 1 } => {
            // 0x3f95e: hold 30 frames (0x3b552).
            *at = Screen::CountryIntro { a1, stage: 2 };
            Yield::Sleep(30)
        }
        Screen::CountryIntro { a1, .. } => {
            // 0x3f964..0x3f97a: erase the country text block, clear scene
            // bit 0x20, and return to the poll (0x3f7d0).
            let d2 = bus.rw(a1 + 2) as u32;
            erase_block(bus, bus.rl(0x3f97e + (d2 << 2)));
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & 0xdf);
            *at = Screen::Poll;
            Yield::Sleep(1)
        }
        Screen::DrawTyping { mut writer } => match writer.run(bus) {
            Some(frames) => {
                *at = Screen::DrawTyping { writer };
                Yield::Sleep(frames)
            }
            None => {
                *at = Screen::DrawHeld;
                Yield::Sleep(120)
            }
        },
        Screen::DrawHeld => {
            // 0x3ff0e: the event is done.
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & 0xdf);
            *at = Screen::Poll;
            Yield::Sleep(1)
        }
        Screen::OpponentIntro { .. } | Screen::VersusIntro { .. } => {
            // 0x3fb34: the scroll layers off, the text layer on; done.
            gfx_off(bus, 0x60);
            gfx_on(bus, 8);
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & 0xdf);
            *at = Screen::Poll;
            Yield::Sleep(1)
        }
        Screen::ResultHeld { text } => {
            // 0x3fd46: erase it, then wait 30 frames (0x3feda).
            erase_block(bus, text);
            *at = Screen::ResultSettle;
            Yield::Sleep(30)
        }
        Screen::ResultSettle => {
            // 0x3fee0: the event is done; back to the poll (0x3f7d0).
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & 0xdf);
            *at = Screen::Poll;
            Yield::Sleep(1)
        }
        Screen::Wipe {
            column,
            d2,
            tile,
            announce,
        } => {
            // 0x3fc9e: one column of 28 cells (0x80 apart) per wake, 2-frame
            // sleeps, 49 columns from 0x170 leftwards.
            if column <= 0x30 {
                for r in 0..28u32 {
                    bus.ww(0x800000 + d2 as u32 + 0x80 * r, tile);
                }
                *at = Screen::Wipe {
                    column: column + 1,
                    d2: d2.wrapping_sub(2),
                    tile,
                    announce,
                };
                return Yield::Sleep(2);
            }
            if announce {
                // 0x3fc2e: the layers back on; sound 0x27, a 2-frame sleep.
                gfx_on(bus, 0x60);
                bus.queue_sound(0x27);
                *at = Screen::AnnounceSound { last: false };
                return Yield::Sleep(2);
            }
            // 0x3fcc0: sound 0x3f; the event is done.
            bus.queue_sound(0x3f);
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & 0xdf);
            *at = Screen::Poll;
            Yield::Sleep(1)
        }
        Screen::AnnounceCleared => {
            // 0x3fbc2: the round's text (table 0x3fc62 by round: text,
            // optional sound, sound), then a 60-frame hold.
            let entry = 0x3fc62 + ((bus.rb(0xffc00a) as u32) << 3);
            text_block(bus, bus.rl(entry));
            let first = bus.rw(entry + 4);
            if first != 0 {
                bus.queue_sound(first);
            }
            bus.queue_sound(bus.rw(entry + 6));
            *at = Screen::AnnounceHeld;
            Yield::Sleep(60)
        }
        Screen::AnnounceHeld => {
            // 0x3fbf8: wipe the announcement with blank tiles (0x3c).
            *at = Screen::Wipe {
                column: 0,
                d2: 0x170,
                tile: 0x3c,
                announce: true,
            };
            screen(bus, io, at)
        }
        Screen::SlowMotion { first: true } => {
            *at = Screen::SlowMotion { first: false };
            Yield::Sleep(30)
        }
        Screen::SlowMotion { first: false } => {
            // 0x3ff42: back to normal speed; the event bit clears unless
            // another event was requested meanwhile (then the poll runs it).
            bus.wb(SLOW_MOTION, 0);
            if bus.rb(0xff8e36) == 0 {
                bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & 0xdf);
            }
            *at = Screen::Poll;
            Yield::Sleep(1)
        }
        Screen::GameOverPalette { next } => {
            if next < 25 {
                // 0x3ff6e..0x3ff82: 25 palette words at 0xb00022, each
                // followed by its table-specified sleep.
                let entry = 0x3ffb8 + ((next as u32) << 2);
                let [hi, lo] = bus.rw(entry).to_be_bytes();
                bus.palette[0x22] = hi;
                bus.palette[0x23] = lo;
                *at = Screen::GameOverPalette { next: next + 1 };
                return Yield::Sleep(bus.rw(entry + 2) as u8);
            }
            // 0x3ff86..0x3ffa8: clear and scroll fg, enable sprites, queue
            // sound 0x95, and hold using the immediate at 0x3b574.
            bus.ww(0xffc680, 0);
            add_fg_scroll(bus, 0xc00);
            gfx_on(bus, 0x40);
            bus.queue_sound(0x95);
            *at = Screen::GameOverHeld;
            Yield::Sleep(bus.rw(0x3b574) as u8)
        }
        Screen::GameOverHeld => {
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & 0xdf);
            *at = Screen::Poll;
            Yield::Sleep(1)
        }
        Screen::AnnounceSound { last } => {
            if !last {
                // 0x3fc48: sound 0x3f, another 2-frame sleep.
                bus.queue_sound(0x3f);
                *at = Screen::AnnounceSound { last: true };
                return Yield::Sleep(2);
            }
            // 0x3fc58: the event is done.
            bus.wb(SCENE_MODE, bus.rb(SCENE_MODE) & 0xdf);
            *at = Screen::Poll;
            Yield::Sleep(1)
        }
    }
}

/// The opponent index as the original computes it into a word: the stage
/// (0xffc003, sign-extended) doubled, the fight (0xffc004) added as a byte.
fn opponent_index(bus: &Bus) -> u16 {
    let d = sext(bus.rb(0xffc003)) << 1;
    (d & 0xff00) | (d as u8).wrapping_add(bus.rb(0xffc004)) as u16
}

/// 0x3fda8 / 0x3fdf8: the TIME copy's test — its first two digits as a
/// word plus the third as a byte — is zero only with all three digits
/// zero.
fn time_copy_zero(bus: &Bus) -> bool {
    let w = bus.rw(TIME_COPY);
    ((w & 0xff00) | (w as u8).wrapping_add(bus.rb(TIME_COPY + 2)) as u16) == 0
}

/// 0x3c0b0: a player's health as points: cells (+7) * 4 + the remainder
/// (+6).
fn health_points(bus: &Bus, ps: u32) -> u16 {
    (bus.rb(ps + 7) as u16)
        .wrapping_mul(4)
        .wrapping_add(bus.rb(ps + 6) as u16)
}

/// 0x3ba12: add the `digits`-digit decimal number ending just before `src`
/// onto the one ending just before `dst`, one digit per byte, the carry
/// (the X flag) through the digits.
fn decimal_add(bus: &mut Bus, dst: u32, src: u32, digits: u32) {
    bus.decimal_add(dst, src, digits);
}

/// 0x3bf5a: a binary word as decimal digits ending just before `end`,
/// added nibble by nibble from the table 0x3bfbc (10 bytes a row: a digit
/// count, five digits, a shift, a mask): the row's digits are added once
/// per unit of the masked, shifted nibble. The low nibble's row has no
/// shift and skips the count's decrement, so it adds one time more than
/// its nibble, as the original does. A zero value adds nothing.
fn binary_to_digits(bus: &mut Bus, value: u16, end: u32) {
    if value == 0 {
        return;
    }
    let mut row = 0x3bfbc;
    for _ in 0..4 {
        let mut d0 = value & bus.rw(row + 8);
        if d0 != 0 {
            let shift = bus.rw(row + 6);
            if shift != 0 {
                d0 >>= shift;
                d0 = d0.wrapping_sub(1);
            }
            let digits = bus.rb(row) as u32 + 1;
            let mut n = d0 as i32;
            loop {
                decimal_add(bus, end, row + 6, digits);
                n -= 1;
                if n < 0 {
                    break;
                }
            }
        }
        row += 10;
    }
}

/// 0x3bde6 (through 0x3bdfa): draw `n + 1` digit bytes from `src` as cells
/// `attr | digit` at `cell`, the leading zeros as the blank tile 0x3c; the
/// last digit is always drawn.
fn draw_digits_blanked(bus: &mut Bus, src: u32, cell: u32, attr: u16, n: u16) {
    let (mut a0, mut a1, mut d1) = (src, cell, n as i32);
    let mut d2;
    loop {
        d2 = bus.rb(a0) as u16;
        a0 += 1;
        if d2 != 0 {
            break;
        }
        bus.ww(a1, 0x3c);
        a1 += 2;
        d1 -= 1;
        if d1 == 0 {
            d2 = bus.rb(a0) as u16;
            a0 += 1;
            break;
        }
    }
    loop {
        bus.ww(a1, attr | d2);
        a1 += 2;
        d1 -= 1;
        if d1 < 0 {
            break;
        }
        d2 = bus.rb(a0) as u16;
        a0 += 1;
    }
}

/// 0x3bdf2 (through 0x3be0c): draw `n + 1` digit bytes from `src` as cells
/// `attr | digit` at `cell`.
fn draw_digits(bus: &mut Bus, src: u32, cell: u32, attr: u16, n: u16) {
    for i in 0..=n as u32 {
        let v = attr | bus.rb(src + i) as u16;
        bus.ww(cell + 2 * i, v);
    }
}

/// 0x422ec: the TIME copy's three digits at 0x80063e.
fn draw_time_copy(bus: &mut Bus) {
    draw_digits_blanked(bus, TIME_COPY, 0x80063e, 0x9000, 2);
}

/// 0x42302: the vitality's last three digits at 0x80073e.
fn draw_vitality(bus: &mut Bus) {
    draw_digits_blanked(bus, VITALITY + 1, 0x80073e, 0x9000, 2);
}

/// 0x42318: the bonus's five digits at 0x800846.
fn draw_bonus(bus: &mut Bus) {
    draw_digits(bus, BONUS, 0x800846, 0x9000, 4);
}

/// 0x3fdf8: the time tally: while the TIME copy is not zero, one tick —
/// the score and bonus additions, one second off, the TIME copy and bonus
/// redrawn — then the vitality tally.
fn tally_time(bus: &mut Bus, at: &mut Screen) -> Yield {
    if time_copy_zero(bus) {
        return tally_vitality(bus, at);
    }
    tally_add(bus);
    decimal_down(bus, TIME_COPY + 3);
    draw_time_copy(bus);
    tally_redraw(bus);
    *at = Screen::TallyTime;
    Yield::Sleep(3)
}

/// 0x3fe64: the vitality tally: while the vitality digits are not all zero
/// (0x3bada), one tick with one off the vitality; then sound 0x3f and the
/// closing 30-frame wait (0x3fed0).
fn tally_vitality(bus: &mut Bus, at: &mut Screen) -> Yield {
    if (0..4).all(|i| bus.rb(VITALITY + i) == 0) {
        bus.queue_sound(0x3f);
        *at = Screen::ResultSettle;
        return Yield::Sleep(30);
    }
    tally_add(bus);
    decimal_down(bus, VITALITY + 4);
    draw_vitality(bus);
    tally_redraw(bus);
    *at = Screen::TallyVitality;
    Yield::Sleep(3)
}

/// A tally tick's additions (0x3fe06 / 0x3fe72): the digits before 0x3fef1
/// (10) onto the winner's 7-digit score (0x3ba0e) and onto the 4-digit
/// bonus (0x3ba12).
fn tally_add(bus: &mut Bus) {
    let ps = bus.rl(WINNER);
    decimal_add(bus, ps + 0x29, 0x3fef1, 7);
    decimal_add(bus, BONUS + 4, 0x3fef1, 4);
}

/// After a tick (0x3fe3a / 0x3fea6): the bonus digits, the winner's score
/// and the top score redrawn, then sound 0xa.
fn tally_redraw(bus: &mut Bus) {
    draw_bonus(bus);
    let ps = bus.rl(WINNER);
    bus.show_score(ps);
    let me = bus.rw(ps + 0x10);
    bus.check_hiscore(me);
    bus.queue_sound(0xa);
}

/// Event 0xb (0x4001c): the match result. The bonus block is erased and
/// the match text shown for 60 frames, by game mode and winner (0xffc009):
/// the tables 0x4010c (mode 2), 0x40140 (mode 4) and 0x40064 (versus, by
/// winner − 1).
fn win_screen_start(bus: &mut Bus, at: &mut Screen) -> Yield {
    erase_block(bus, 0x1561c);
    let who = sext(bus.rb(0xffc009));
    let (table, idx) = match bus.rb(GAME_MODE) {
        2 => (0x4010cu32, (if who == 1 { who } else { who >> 1 }) << 2),
        4 => (0x40140, (who >> 1) << 2),
        _ => (
            0x40064,
            ((who & 0xff00) | (who as u8).wrapping_sub(1) as u16) << 2,
        ),
    };
    let text = bus.rl(table.wrapping_add(idx as i16 as i32 as u32));
    text_block(bus, text);
    *at = Screen::WinText { text };
    Yield::Sleep(60)
}

/// 0x436b8..0x436e6: request the country palette (0x436e8 by country, then
/// by stage) at offset 0x200; the caller yields 0x436e0's speed_sleep. It
/// is the fight's foreground palette, so the stage gate gives the picked
/// stage's at every call site (the country intro gate, #193, still makes
/// this request when it skips the intro).
fn request_country_palette(bus: &mut Bus) {
    let country = ((sext(bus.rb(0xffc006)) as u32) << 2) & 0xffff;
    let stage = (bus.rb(0xffc003) as u32) << 2;
    let rom = bus.rl(bus.rl(0x436e8 + country) + stage);
    let src = picked_stage(bus).map_or(rom, |s| s.fg.palette);
    bus.wl(0xff8634, src);
    bus.ww(0xff8638, 0x200);
    bus.wb(PALETTE_REQUEST, bus.rb(PALETTE_REQUEST) | 0xff);
}

/// Event 2 when `STAGE_NO != 0` (0x3f812..0x3f8e2, ticket #83, F43): the
/// travel map between countries. Looks up the starting country's plane
/// template table in `0x3f822` (`0x3f8a4` Japan, `0x3f8b4` USA, `0x3f8c4`
/// China, `0x3f8d4` England) and indexes it by `(STAGE_NO - 1) * 4`, turns off
/// bg + tx (`0x28`) and turns on fg (`0x40`), scrolls fg to `0x1980`, requests
/// palette `0x1a268` at offset `0x200`, and sleeps 120 frames (`0x3b56a` at
/// `0x3f876` into `0x3f87c`).
fn travel_map_start(bus: &mut Bus, at: &mut Screen) -> Yield {
    let country = ((sext(bus.rb(0xffc006)) as u32) << 2) & 0xffff;
    let table = bus.rl(0x3f822u32.wrapping_add(country as i16 as i32 as u32));
    let d1 = sext(bus.rb(0xffc003));
    let d1 = (((d1 & 0xff00) | (d1 as u8).wrapping_sub(1) as u16) << 2) as i16 as i32 as u32;
    let a1 = table.wrapping_add(d1);
    gfx_off(bus, 0x28);
    gfx_on(bus, 0x40);
    bus.ww(0xffc680, 0);
    add_fg_scroll(bus, 0x1980);
    request_palette(bus, 0x1a268, 0x200);
    *at = Screen::TravelMapSpawn { a1 };
    Yield::Sleep(bus.rw(0x3b56c) as u8)
}

/// Event 3 (0x3f8e4..0x3f97a): the country screen. Clears the text layer
/// (0x3bdb0), looks up the 4-byte entry `(fg_scroll, country_id)` at
/// `0x3f8fa[country][stage]`, turns off bg (0x20) and turns on fg + tx
/// (0x48), scrolls fg to `(A1)`, and requests the country palette at offset
/// 0x200 (0x436b8, with its 1-frame speed_sleep at 0x436e0). The skip gate
/// ([`skips_country_intro`]) clears the text layer, requests that palette
/// and shows nothing else.
fn country_intro_start(bus: &mut Bus, io: &Io, at: &mut Screen) -> Yield {
    clear_text_layer(bus);
    if skips_country_intro(bus) {
        // The gate: nothing of the intro shows. Only the fight's foreground
        // palette is still requested (the intro is where the 1987 flow loads
        // it), and the event ends after the sleep that loads it, as the
        // round result's does.
        request_country_palette(bus);
        *at = Screen::ResultSettle;
        return speed_sleep(bus, io);
    }
    let country = ((sext(bus.rb(0xffc006)) as u32) << 2) & 0xffff;
    let stage = ((sext(bus.rb(0xffc003)) as u32) << 2) & 0xffff;
    let table = bus.rl(0x3f8fa + country);
    let a1 = table.wrapping_add(stage as i16 as i32 as u32);
    gfx_off(bus, 0x20);
    gfx_on(bus, 0x48);
    bus.ww(0xffc680, 0);
    add_fg_scroll(bus, bus.rw(a1));
    request_country_palette(bus);
    *at = Screen::CountryIntro { a1, stage: 0 };
    speed_sleep(bus, io)
}

/// 0x43748: request the stage palette (0x43778 by country, then by stage)
/// at offset 0; the vblank loads it. The original ends with the fight
/// loop's sleep, which the caller yields. The stage gate gives the picked
/// stage's background palette at every call site.
fn request_stage_palette(bus: &mut Bus) {
    let country = ((sext(bus.rb(0xffc006)) as u32) << 2) & 0xffff;
    let stage = (bus.rb(0xffc003) as u32) << 2;
    let rom = bus.rl(bus.rl(0x43778 + country) + stage);
    let src = picked_stage(bus).map_or(rom, |s| s.bg.palette);
    bus.wl(0xff8634, src);
    bus.ww(0xff8638, 0);
    bus.wb(PALETTE_REQUEST, bus.rb(PALETTE_REQUEST) | 0xff);
}

/// Event 4 (0x3f9ec): the next opponent's stage intro. In the one-player
/// modes (0x3fa56 / 0x3fac6 → 0x3fae6): sound 0x33, the text and sprite
/// layers off, the background scrolled to the country table's word for
/// this opponent (0x3fa66 / 0x3fad6), the background on, the stage palette
/// requested (0x43748, with its sleep), then 120 + 30 frames; at the end
/// the scroll layers go off and the text layer on (0x3fb34). In versus
/// (0x3fa00) the background goes to 0x780 with the palette 0x1ae68, the
/// holds come first and the stage palette request after them. The VS gate
/// gives the picked pair's screen, page and palette instead; Arcade's
/// built pairs use that same hold and stage-palette restoration path.
fn opponent_intro_start(bus: &mut Bus, io: &Io, at: &mut Screen) -> Yield {
    let country = ((sext(bus.rb(0xffc006)) as u32) << 2) & 0xffff;
    let vs = picked_vs(bus);
    match bus.rb(GAME_MODE) {
        mode @ (2 | 4) if vs.is_none() => {
            let table = bus.rl(if mode == 2 { 0x3fa66 } else { 0x3fad6 } + country);
            bus.queue_sound(0x33);
            let d2 = opponent_index(bus) << 1;
            gfx_off(bus, 0x48);
            bus.ww(0xffc682, 0);
            let d = bus.rw(table.wrapping_add(d2 as i16 as i32 as u32));
            add_bg_scroll(bus, d);
            gfx_on(bus, 0x20);
            request_stage_palette(bus);
            *at = Screen::OpponentIntro { stage: 0 };
            speed_sleep(bus, io)
        }
        _ => {
            gfx_off(bus, 0x48);
            bus.ww(0xffc682, 0);
            add_bg_scroll(bus, vs.map_or(0x780, |v| v.scroll));
            if let Some(v) = vs {
                *bus.bg_page = v.page;
            }
            bus.queue_sound(0x33);
            bus.wl(0xff8634, vs.map_or(0x1ae68, |v| v.palette));
            bus.ww(0xff8638, 0);
            bus.wb(PALETTE_REQUEST, 0xff);
            gfx_on(bus, 0x20);
            *at = Screen::VersusIntro { stage: 0 };
            Yield::Sleep(120)
        }
    }
}

/// 0x3ea66..0x3eaf4: one frame of the fight loop: the input handlers and
/// (one-player modes) the CPU decision by game mode, the stage drift, the
/// object dispatcher, effects, camera requests, and the TIME digits; then
/// the loop sleeps (0x3b5aa: one frame, or six with the Speed DIP off or in
/// the KO slow motion). In Training, Training's refill runs before the TIME
/// digits ([`Bus::training_refill`], ticket #243).
///
/// The original skips the sleep when a vblank interrupted the frame's work
/// (0xff8632 cleared mid-frame: slowdown). The native frame never overruns,
/// so it always sleeps (disclosed; modelling overruns needs the 68000's
/// cycle timing).
fn fight_frame(bus: &mut Bus, io: &Io, at: &mut Flow) -> Yield {
    if bus.era_freeze_tick() {
        return Yield::Sleep(1);
    }
    bus.wb(FRAME_PENDING, 0xff);
    bus.drive_fight_inputs();
    if bus.era_freeze_tick() {
        return Yield::Sleep(1);
    }
    if bus.rb(SCENE_MODE) == 0x10 {
        // 0x3eac4 -> 0x423b8..0x423d8: stop music (0x3f), play challenger
        // fanfare (0x3e), draw A NEW CHALLENger ENTERS THE RING! (0x15ba2),
        // and sleep 120 frames (0x3b56e) into 0x423de.
        bus.queue_sound(0x3f);
        bus.queue_sound(0x3e);
        text_block(bus, 0x15ba2);
        *at = Flow::ChallengerHold1;
        return Yield::Sleep(120);
    }
    bus.drift_stage();
    bus.dispatch_fighters();
    bus.run_effects();
    bus.resolve_camera();
    // Training's refill (ticket #243), once a fight frame for both
    // fighters, after this frame's hits and blocks; nothing outside Training.
    bus.training_refill();
    bus.era_sync_frame();
    draw_time_digits(bus);
    if io.dsw1 & 0x1000 != 0 && bus.rb(SLOW_MOTION) == 0 {
        Yield::Sleep(1)
    } else {
        Yield::Sleep(6)
    }
}

/// 0x3beb4: draw the three TIME digits (0xff8e18) as text cells at 0x800e1c
/// (attr 0x3000), through the digit writer 0x3be0c.
fn draw_time_digits(bus: &mut Bus) {
    for i in 0..3 {
        let v = 0x3000 | bus.rb(TIME_DIGITS + i) as u16;
        bus.ww(0x800e1c + 2 * i, v);
    }
}

/// 0x3b6c6: clear the attack-button latch groups' state bytes.
fn reset_button_latches(bus: &mut Bus) {
    for (group, offs) in [
        (0xff8690, &[4u32, 3, 2, 0, 1][..]),
        (0xff86a0, &[4, 3, 2, 0, 1][..]),
        (0xff8698, &[4, 3, 2, 0, 1][..]),
        (0xff86a8, &[4, 3, 2, 0, 1][..]),
    ] {
        for o in offs {
            bus.wb(group + o, 0);
        }
    }
}

/// 0x3bba0: write a text block from ROM: a video-RAM byte offset (word),
/// rows and columns (bytes), an attribute (word), then one word per cell —
/// the cell code or'ed with the attribute, or 0xffff and a count: that many
/// blank cells (0x103c), continuing over row ends. Rows are 0x80 bytes
/// apart. This entry writes without the typewriter delay (D4 = 0); the
/// typewriter sound word (0xff868c) is cleared at the end.
pub(crate) fn text_block(bus: &mut Bus, block: u32) {
    let mut writer = TextWriter::new(bus, block, 0);
    let done = writer.run(bus);
    debug_assert!(done.is_none());
}

/// The text-block writer (0x3bba0 / 0x3bb98) as a resumable cursor. With a
/// typewriter delay (D4) every written cell (not the blank fills) is
/// followed by a sleep of that many frames (0x3bbfe: the typewriter sound
/// 0xff868c, if set, is queued first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TextWriter {
    a0: u32,
    d0: u16,
    row_start: u16,
    rows: u16,
    cols: u16,
    col: u16,
    attr: u16,
    filling: bool,
    count: u16,
    delay: u8,
    finished: bool,
}

impl TextWriter {
    /// The block header: a video-RAM byte offset, rows and columns (bytes),
    /// an attribute.
    pub(crate) fn new(bus: &Bus, block: u32, delay: u8) -> Self {
        let d0 = bus.rw(block);
        TextWriter {
            a0: block + 6,
            d0,
            row_start: d0,
            rows: bus.rb(block + 2) as u16,
            cols: bus.rb(block + 3) as u16,
            col: bus.rb(block + 3) as u16,
            attr: bus.rw(block + 4),
            filling: false,
            count: 0,
            delay,
            finished: false,
        }
    }

    /// Write cells until a typewriter sleep (returns its frames) or the end
    /// of the block (returns None, the typewriter sound word cleared).
    pub(crate) fn run(&mut self, bus: &mut Bus) -> Option<u8> {
        loop {
            if self.finished {
                bus.ww(TEXT_SOUND, 0);
                return None;
            }
            let mut sleep = false;
            if !self.filling {
                let d3 = bus.rw(self.a0);
                self.a0 += 2;
                if d3 == 0xffff {
                    self.count = bus.rw(self.a0);
                    self.a0 += 2;
                    self.filling = true;
                } else {
                    bus.ww(0x800000 + self.d0 as u32, d3 | self.attr);
                    if self.delay != 0 {
                        let snd = bus.rw(TEXT_SOUND);
                        if snd != 0 {
                            bus.queue_sound(snd);
                        }
                        sleep = true;
                    }
                }
            }
            if self.filling {
                bus.ww(0x800000 + self.d0 as u32, 0x103c);
                self.count = self.count.wrapping_sub(1);
                self.filling = self.count != 0;
            }
            self.d0 = self.d0.wrapping_add(2);
            self.col -= 1;
            if self.col == 0 {
                self.rows -= 1;
                if self.rows == 0 {
                    self.finished = true;
                } else {
                    self.row_start = self.row_start.wrapping_add(0x80);
                    self.d0 = self.row_start;
                    self.col = self.cols;
                }
            }
            if sleep {
                return Some(self.delay);
            }
        }
    }
}

/// 0x3bd00: blank a text block's cells (0x103c) by its header.
fn erase_block(bus: &mut Bus, block: u32) {
    let (offset, rows, cols) = (
        bus.rw(block),
        bus.rb(block + 2) as u16,
        bus.rb(block + 3) as u16,
    );
    for r in 0..rows {
        for c in 0..cols {
            bus.ww(
                0x800000 + offset.wrapping_add(0x80 * r + 2 * c) as u32,
                0x103c,
            );
        }
    }
}

/// 0x3bc3e..0x3bcfe: write a 2x2-tile text block from ROM (`D4 = 0`): a
/// video-RAM byte offset (word), rows and columns of 2x2 cells (bytes), an
/// attribute (word), then one word per 2x2 cell — `d3 | attr` expanded into
/// `(d3, d3 + 1)` on the upper row and `(d3 + 0x10, d3 + 0x11)` on the lower
/// row (`+0x80`), or `0xffff` followed by a count of blank 2x2 cells
/// (`0x103c` in all four words), continuing across row ends (`+0x100` per
/// row of 2x2 cells, `+4` per column).
pub(crate) fn text_block_2x2(bus: &mut Bus, block: u32) {
    let mut a0 = block + 6;
    let mut row_start = bus.rw(block);
    let mut d0 = row_start;
    let mut rows = bus.rb(block + 2) as u16;
    let cols = bus.rb(block + 3) as u16;
    let mut col = cols;
    let attr = bus.rw(block + 4);
    let mut filling = false;
    let mut count = 0u16;
    loop {
        if !filling {
            let d3 = bus.rw(a0);
            a0 += 2;
            if d3 == 0xffff {
                count = bus.rw(a0);
                a0 += 2;
                filling = true;
            } else {
                let t = d3 | attr;
                bus.ww(0x800000 + d0 as u32, t);
                bus.ww(0x800000 + d0.wrapping_add(2) as u32, t.wrapping_add(1));
                let lo = d0.wrapping_add(0x80);
                let t_lo = t.wrapping_add(0x10);
                bus.ww(0x800000 + lo as u32, t_lo);
                bus.ww(0x800000 + lo.wrapping_add(2) as u32, t_lo.wrapping_add(1));
            }
        }
        if filling {
            bus.ww(0x800000 + d0 as u32, 0x103c);
            bus.ww(0x800000 + d0.wrapping_add(2) as u32, 0x103c);
            let lo = d0.wrapping_add(0x80);
            bus.ww(0x800000 + lo as u32, 0x103c);
            bus.ww(0x800000 + lo.wrapping_add(2) as u32, 0x103c);
            count = count.wrapping_sub(1);
            filling = count != 0;
        }
        d0 = d0.wrapping_add(4);
        col -= 1;
        if col == 0 {
            rows -= 1;
            if rows == 0 {
                return;
            }
            row_start = row_start.wrapping_add(0x100);
            d0 = row_start;
            col = cols;
        }
    }
}

/// 0x3bd3e..0x3bd94: blank a 2x2-tile text block's cells (`0x103c` in all four
/// words of each 2x2 cell, `+4` per column, `+0x100` per row) by its header.
fn erase_block_2x2(bus: &mut Bus, block: u32) {
    let (offset, rows, cols) = (
        bus.rw(block),
        bus.rb(block + 2) as u16,
        bus.rb(block + 3) as u16,
    );
    for r in 0..rows {
        for c in 0..cols {
            let d0 = offset.wrapping_add(0x100 * r + 4 * c);
            bus.ww(0x800000 + d0 as u32, 0x103c);
            bus.ww(0x800000 + d0.wrapping_add(2) as u32, 0x103c);
            let lo = d0.wrapping_add(0x80);
            bus.ww(0x800000 + lo as u32, 0x103c);
            bus.ww(0x800000 + lo.wrapping_add(2) as u32, 0x103c);
        }
    }
}

/// Slot 3 (0x3f388 loop): in attract mode (`0xff8003 == 0`, 0x3f3a4..0x3f40e)
/// draws the copyright line (`0x14e9c`) every frame and blinks `INSERT COIN`
/// (`0x1457a`) 20 frames on and 20 frames off through `0xff8f20` and `D2`; in
/// a game (`0xff8003 == 0xff`), runs the challenger watch through the
/// continuation word at `0xff8f24`.
fn watch(bus: &mut Bus, io: &Io, d2: &mut u16) -> Yield {
    match bus.rb(MODE_DEMO) {
        0 => {
            // 0x3f3a4..0x3f3bc
            text_block(bus, 0x14e9c);
            bus.wl(WATCH_STATE, 0x3f41e);
            return match bus.rl(WATCH_NEXT) {
                0x3f3be => {
                    text_block(bus, 0x1457a);
                    *d2 = 0x14;
                    bus.wl(WATCH_NEXT, 0x3f3d8);
                    Yield::Sleep(1)
                }
                0x3f3d8 | 0x3f402 => {
                    let on = bus.rl(WATCH_NEXT) == 0x3f3d8;
                    *d2 = d2.wrapping_sub(1);
                    if *d2 == 0 {
                        if on {
                            erase_block(bus, 0x1457a);
                            bus.wl(WATCH_NEXT, 0x3f402);
                        } else {
                            text_block(bus, 0x1457a);
                            bus.wl(WATCH_NEXT, 0x3f3d8);
                        }
                        *d2 = 0x14;
                    } else {
                        bus.wl(WATCH_NEXT, if on { 0x3f3d8 } else { 0x3f402 });
                    }
                    Yield::Sleep(1)
                }
                other => {
                    bus.note(other);
                    Yield::Halt
                }
            };
        }
        0xff => {}
        _ => return Yield::Sleep(1),
    }
    // 0x3f410
    bus.wl(WATCH_NEXT, 0x3f3be);
    let mut at = bus.rl(WATCH_STATE);
    loop {
        match at {
            0x3f41e => {
                if bus.rb(SCENE_MODE) != 0x80 {
                    return Yield::Sleep(1);
                }
                *d2 = match bus.rb(GAME_MODE) {
                    2 => 2,
                    4 => 1,
                    _ => return Yield::Sleep(1),
                };
                bus.wl(WATCH_STATE, 0x3f454);
                at = 0x3f454;
            }
            0x3f454 => {
                if bus.rw(CREDITS_LONG) | bus.rw(CREDITS) != 0 {
                    bus.wl(WATCH_STATE, 0x3f47e);
                    at = 0x3f47e;
                    continue;
                }
                if bus.rb(SCENE_MODE) != 0x80 {
                    bus.wl(WATCH_STATE, 0x3f41e);
                }
                return Yield::Sleep(1);
            }
            0x3f47e => {
                if bus.rb(SCENE_MODE) != 0x80 {
                    bus.wl(WATCH_STATE, 0x3f41e);
                    return Yield::Sleep(1);
                }
                if io.system & *d2 != 0 {
                    return Yield::Sleep(1);
                }
                // 0x3f4a2..0x3f4aa: buy-in limit (0xffc001) vs STAGE_NO
                // (0xffc003); signed `ble $3f4d2` refuses the join when
                // `buyin <= stage`.
                if (bus.rb(0xffc001) as i8) <= (bus.rb(0xffc003) as i8) {
                    bus.wl(WATCH_STATE, 0x3f41e);
                    return Yield::Sleep(1);
                }
                // 0x3f4ae..0x3f4c0: SCENE_MODE = 0x10, WATCH_STATE = 0x3f4c6,
                // take 1 credit (0x3bb74 at 0xff865e), and fall through into
                // 0x3f4c6.
                bus.wb(SCENE_MODE, 0x10);
                bus.wl(WATCH_STATE, 0x3f4c6);
                decimal_down(bus, 0xff865e);
                at = 0x3f4c6;
            }
            0x3f4c6 => {
                // 0x3f4c6..0x3f4da: wait while SCENE_MODE bit 4 is set; when
                // cleared, reset WATCH_STATE to 0x3f41e and sleep 1 frame.
                if bus.rb(SCENE_MODE) & 0x10 == 0 {
                    bus.wl(WATCH_STATE, 0x3f41e);
                }
                return Yield::Sleep(1);
            }
            other => {
                bus.note(other);
                return Yield::Halt;
            }
        }
    }
}

/// Slot 7 (0x4813e loop): the random-state pass (`Bus::step_random`), one
/// frame's sleep, the coin slots (0x4827c..0x48324), 9-credit lockout and
/// service switch (0x481be..0x4820c), the attract start check (0x48212..0x48248),
/// the coin counter pulses (0x4824e..0x4826e, 0x48326..0x4834c), and a
/// six-frame sleep.
fn random(scheduler: &mut Scheduler, bus: &mut Bus, io: &Io, at: &mut Random) -> Yield {
    match *at {
        Random::Pass => {
            bus.step_random();
            *at = Random::Coins;
            Yield::Sleep(1)
        }
        Random::Coins => {
            // 0x4819e..0x481ba: coin slots 1-4 (0x4827c).
            for slot in COIN_SLOTS {
                coin_slot_step(bus, slot);
            }
            // 0x481be..0x481e2: 9-credit lockout and service switch check.
            if bus.rw(CREDITS) >= 9 {
                coin_ctrl_off(bus, 0x7070);
                random_after_service(scheduler, bus, at)
            } else if io.system & 4 == 0 {
                *at = Random::ServiceDebounce;
                Yield::Sleep(6)
            } else {
                random_after_service(scheduler, bus, at)
            }
        }
        Random::ServiceDebounce => {
            // 0x481e8..0x4820c: after 6-frame debounce sleep, re-read
            // service1 (SYSTEM bit 2 low); if still held, award 1 credit
            // (0x3bb5c at 0xff865e), queue sound 0x0f (0x3b802) and sleep
            // 30 frames (0x3b552).
            if io.system & 4 == 0 {
                decimal_up(bus, 0xff865e);
                bus.queue_sound_unfiltered(0x0f);
                *at = Random::ServiceCooldown;
                Yield::Sleep(30)
            } else {
                random_after_service(scheduler, bus, at)
            }
        }
        Random::ServiceCooldown => random_after_service(scheduler, bus, at),
        Random::AttractStarted => random_pulses_from(bus, at, 3, COIN_SLOTS[0]),
        Random::Pulse { d1, d2, a1 } => {
            // 0x48344..0x4834c, 0x48262..0x4826a: finish the 6-frame counter
            // pulse (0x3b6a2) and continue the 0x48256 dbra loop.
            coin_ctrl_off(bus, d2);
            if d1 == 0 {
                *at = Random::Pass;
                Yield::Sleep(6)
            } else {
                random_pulses_from(bus, at, d1 - 1, a1 + 8)
            }
        }
    }
}

/// 0x4827c..0x48324: one coin slot's step.
fn coin_slot_step(bus: &mut Bus, slot: u32) {
    if bus.rb(slot + 1) == 0 {
        // 0x48286..0x482a6
        if bus.rw(CREDITS) < 9 && bus.rw(COIN_CONTROL) & 0x01e0 == 0 {
            coin_ctrl_or(bus, 0x7070);
        }
        return;
    }
    // 0x482a8..0x48324: pending coin (+1 != 0).
    if bus.rw(CREDITS) >= 9 {
        return;
    }
    if bus.rw(COIN_CONTROL) & 0x01e0 == 0 {
        coin_ctrl_or(bus, 0x7070);
    }
    bus.wb(slot + 1, bus.rb(slot + 1).wrapping_sub(1));
    bus.wb(slot + 4, bus.rb(slot + 4).wrapping_add(1));
    bus.wb(slot + 6, bus.rb(slot + 6).wrapping_add(1));
    bus.queue_sound_unfiltered(0x0f);
    if bus.rb(slot + 2) != bus.rb(slot + 4) {
        return;
    }
    bus.wb(slot + 4, 0);
    let award = bus.rb(slot + 3) as i8 as i16;
    if award > 0 {
        for _ in 0..award {
            decimal_up(bus, 0xff865e);
            if bus.rw(CREDITS) >= 9 {
                coin_ctrl_off(bus, 0x7070);
                break;
            }
        }
    }
}

/// 0x48212..0x4826a: attract-mode credit check (`0x48228..0x48248`) and coin
/// counter pulse loop (`0x4824e..0x4826a`).
fn random_after_service(scheduler: &mut Scheduler, bus: &mut Bus, at: &mut Random) -> Yield {
    if bus.rb(MODE_DEMO) == 0 && bus.rl(CREDITS_LONG) != 0 {
        // 0x48228..0x48248: copy 0xff8f28 to 0xffc007, kill slot 2 (0x3b230),
        // spawn slot 1 at 0x3dec2 (0x3b20a), and sleep 1 frame (0x3b51a)
        // resuming at 0x4824e.
        bus.wb(0xffc007, bus.rb(0xff8f28));
        scheduler.kill(bus, 2);
        scheduler.spawn(bus, 1, 0x3dec2);
        *at = Random::AttractStarted;
        return Yield::Sleep(1);
    }
    random_pulses_from(bus, at, 3, COIN_SLOTS[0])
}

/// 0x48256..0x4826a, 0x48326..0x4834c: iterate coin slots from `(start_d1,
/// start_a1)` down to `d1 == 0`, pulsing any slot with pending counter count
/// (`+6 != 0`) for 6 frames through 0x3b654 / 0x3b6a2.
fn random_pulses_from(bus: &mut Bus, at: &mut Random, start_d1: u16, start_a1: u32) -> Yield {
    let mut a1 = start_a1;
    for k in 0..=start_d1 {
        let d1 = start_d1 - k;
        let d2 = bus.rw(0x48274 + ((d1 as u32) << 1));
        if bus.rb(a1 + 6) != 0 {
            bus.wb(a1 + 6, bus.rb(a1 + 6).wrapping_sub(1));
            coin_ctrl_or(bus, d2);
            *at = Random::Pulse { d1, d2, a1 };
            return Yield::Sleep(6);
        }
        a1 += 8;
    }
    *at = Random::Pass;
    Yield::Sleep(6)
}

/// One frame's sound-latch write (0x3b138): the byte, and the 68000 cycles
/// from the vblank handler's first instruction (0x3b052) to the write's bus
/// cycle — the standard instruction costs of the path the handler took,
/// which the pinned MAME reproduces exactly (ticket #35,
/// docs/reference/latch-timing.md). The interrupt's acceptance latency
/// before the first instruction is not part of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatchWrite {
    pub value: u8,
    pub cycles: u32,
}

/// Cycles from the start of 0x3b138 `move.w D1,($1c,A6)`'s block
/// (0x3b122) to the write's bus cycle, as the reference's write tap logs
/// it: the block's instructions (58) plus the move's write (measured, 14).
const WRITE_OFFSET: u32 = 58 + 14;

/// 0x3b362: the palette load's cycles when the system port's bit 6 is high
/// (one copy): movea.l #,A1 12, clr.b 16, movea.l (d16,A5) 16, move.w
/// (d16,A5) 12, bra 10; movem.l D1/A0,-(A7) 24, move.w #$7f,D2 8; 128 x
/// (move.l (A0)+,(A1,D1.w) 26, addq.w 4) with dbra 127 x 10 + 14; movem.l
/// (A7)+ 28, move.w (d16,A6) 12, andi.w 8, beq.s not taken 8, rts 16.
const PALETTE_LOAD_CYCLES: u32 = 66 + 32 + 128 * 30 + 127 * 10 + 14 + 72;

/// 0x3b052..0x3b194: the vblank interrupt's work, after the button latches
/// and before the sound pop (which the machines run through the bus
/// already): the frame-counter byte; both coin inputs' debounce (0x3b31a);
/// a requested palette load (0x3b362); the IN1 copy, the sound-queue pop;
/// then the frame-done flag cleared, the vblank flag set, the frame word
/// counted, and the sleeping slots counted down.
/// Returns the sound-latch byte with the handler's cycles to its write:
/// each block below adds the 68000 costs of the original instructions the
/// translation stands for (branch taken 10, not taken 12; byte-displacement
/// branches 10 / 8).
pub fn vblank(bus: &mut Bus, io: &Io) -> LatchWrite {
    let cycles = vblank_prefix(bus, io);
    if io.dsw1 & 0x4000 == 0 {
        // Other bounded machines have not translated the Freeze spin.
        bus.note(0x3b100);
    }
    vblank_suffix(bus, io, cycles).latch
}

/// 0x3b052..0x3b108: the vblank work before the Freeze spin. Keeping this
/// prefix separate lets the one-player machine suspend the original handler
/// at 0x3b100 without running its suffix or the scheduler.
pub(crate) fn vblank_prefix(bus: &mut Bus, io: &Io) -> u32 {
    // 0x3b052: movem.l D0-D7/A0-A6,-(A7) 128, addq.b #1,($2,A5) 16.
    let mut cycles = 144;
    bus.wb(FRAME_BYTE, bus.rb(FRAME_BYTE).wrapping_add(1));
    cycles += bus.vblank_buttons(io.in0, io.in1);
    for (slot, mask, enable) in [(COIN_SLOTS[0], 1u16, 0x0010u16), (COIN_SLOTS[1], 2, 0x0020)] {
        // 0x3b0cc / 0x3b0da: lea 8, move.w #,D0 8, jsr $3b31a.l 20.
        // MAME's coin_w (0xc00011, low byte of 0xc00010; sf-mame0289.cpp:110-116)
        // calls coin_lockout_w(0/1, ~data & 0x10/0x20), keeping the active-low
        // IN0 coin bit high (released) while locked out.
        let released = (io.in0 & mask != 0) || (*bus.coin_ctrl & enable == 0);
        cycles += 36 + coin_debounce(bus, slot, released);
    }
    // 0x3b0e8: move.w ($c,A6),D0 12, andi.w #$80,D0 8, bne $3b194 not taken 12.
    cycles += 32;
    if io.system & 0x80 != 0 {
        // Active high: the frame's remaining vblank work is skipped.
        bus.note(0x3b0f0);
    }
    // 0x3b0f4: move.b ($633,A5),D0 12, beq $3b100 taken 10 / not taken 12 + bsr 18.
    cycles += 12;
    if bus.rb(PALETTE_REQUEST) != 0 {
        // 0x3b362: a requested palette load: 0x200 bytes from the ROM
        // pointer 0xff8634 into the palette at the offset 0xff8638, the
        // request consumed. The original repeats the copy, toggling
        // gfxctrl bit 1, while the system port's bit 6 is low; the
        // fixtures' port keeps it high.
        cycles += 12 + 18 + PALETTE_LOAD_CYCLES;
        bus.wb(PALETTE_REQUEST, 0);
        let src = bus.rl(0xff8634);
        let off = bus.rw(0xff8638);
        upload_palette(bus, src, off);
        if io.system & 0x40 == 0 {
            bus.note(0x3b378);
        }
    } else {
        cycles += 10;
    }
    // 0x3b100: move.w ($8,A6),D0 12, andi.w #$4000,D0 8, beq.s $3b100 not taken 8.
    cycles += 28;
    cycles
}

pub(crate) struct VblankSuffix {
    pub latch: LatchWrite,
    /// Cycles from this write's bus cycle to a held vblank handler's first
    /// fetch, including this handler's post-write work and RTE.
    pub held_entry_cycles: u32,
}

/// 0x3b10a..0x3b198: resume the interrupted vblank suffix with the cycles
/// accumulated before its final Freeze-DIP read. Also account the path to
/// the held interrupt accepted after this handler's RTE.
pub(crate) fn vblank_suffix(bus: &mut Bus, io: &Io, mut cycles: u32) -> VblankSuffix {
    // 0x3b10a: move.w ($2,A6),$ff868e.l 24; cmpi.b #-1,($3,A5) 16; bne
    // $3b122: in a game (the byte is 0xff) not taken 12 + movea.l
    // #$800110,A1 12 (dead: A1 is reloaded at 0x3b12c), else taken 10.
    bus.latch_in1(io.in1);
    cycles += 40 + if bus.rb(MODE_DEMO) == 0xff { 24 } else { 10 };
    // 0x3b122: move.b D0,($1a,A6) 12, nop x3 12, lea ($662,A5),A1 8, move.w
    // ($65e,A5),D0 12, move.w (A1,D0.w),D1 14, then the write.
    cycles += WRITE_OFFSET;
    let latch = bus.pop_sound();

    // Two cycles remain in the 16-cycle move at 0x3b138 after the tapped
    // write position represented by WRITE_OFFSET. Then 0x3b13c: cmpi.w 8;
    // beq 10 when idle / 12 when a command was popped.
    let mut held_entry_cycles = 2 + 8 + if latch == 0xff { 10 } else { 12 };
    if latch != 0xff {
        // 0x3b144..0x3b156: clear the queue word (26), advance/wrap the
        // index (4 + 8 + branch 10, or 8 + clr.w 4), then store it (16).
        held_entry_cycles += 26 + 4 + 8 + if bus.rw(0xff865e) == 0 { 8 + 4 } else { 10 } + 16;
    }
    // 0x3b15a..0x3b170: clr.b 16, move.b # 16, addq.w abs.l 24,
    // lea 8, clr.w 4, move.w # 8.
    held_entry_cycles += 16 + 16 + 24 + 8 + 4 + 8;

    bus.wb(FRAME_PENDING, 0);
    bus.wb(VBLANK_FLAG, 0x88);
    bus.ww(FRAME_WORD, bus.rw(FRAME_WORD).wrapping_add(1));
    for k in 0..SLOT_COUNT {
        let slot = SLOTS + k * 0x10;
        // 0x3b174: cmpi.b #,indexed 18; bne 10 / 12; addi.w 8;
        // dbra 10, or 14 on the last slot.
        held_entry_cycles += 18;
        if bus.rb(slot) == 8 {
            // subq.b #,indexed 18; bne 10 / 12; move.b #,indexed 18
            // when the countdown wakes the slot.
            held_entry_cycles += 12 + 18;
            let n = bus.rb(slot + 1).wrapping_sub(1);
            bus.wb(slot + 1, n);
            if n == 0 {
                held_entry_cycles += 12 + 18;
                bus.wb(slot, 2);
            } else {
                held_entry_cycles += 10;
            }
        } else {
            held_entry_cycles += 10;
        }
        held_entry_cycles += 8 + if k + 1 == SLOT_COUNT { 14 } else { 10 };
    }
    // 0x3b194: movem.l (A7)+,15 registers 132; rte 20; the already-held
    // level-1 exception entry is MAME's standard 44 cycles.
    held_entry_cycles += 132 + 20 + 44;
    VblankSuffix {
        latch: LatchWrite {
            value: latch,
            cycles,
        },
        held_entry_cycles,
    }
}

/// 0x3b31a: one coin input's debounce: while pressed (bit low) the press
/// counter (+0) counts up to 0x64; on release a press of at least two
/// frames (the original compares the word +0/+1 with 2) adds a pending coin
/// (+1), a held-down 0x64 does not, and the counter clears. Returns the
/// routine's cycles: move.w ($0,A6),D1 12, and.w D0,D1 4, beq $3b350
/// (pressed: taken 10), then the branch's instructions.
fn coin_debounce(bus: &mut Bus, slot: u32, released: bool) -> u32 {
    let count = bus.rb(slot);
    if !released {
        // 0x3b350: cmpi.b #$64,($0,A1) 16, blt $3b35c taken 10 + addq.b
        // #1,($0,A1) 16 + rts 16; saturated: not taken 12 + rts 16.
        if (count as i8) < 0x64 {
            bus.wb(slot, count.wrapping_add(1));
            return 26 + 16 + 10 + 16 + 16;
        }
        return 26 + 16 + 12 + 16;
    }
    // 0x3b324: ori.b #0,($0,A1) 20, bne $3b330: no press counted, not
    // taken 12 + rts 16.
    if count == 0 {
        return 28 + 20 + 12 + 16;
    }
    // 0x3b330: cmpi.b #$64,($0,A1) 16, beq $3b34a (a held-down 0x64: taken
    // 10); cmpi.w #2,($0,A1) 16, blt $3b34a (a short press: taken 10); addi.b
    // #1,($1,A1) 20; 0x3b34a: clr.b ($0,A1) 16, rts 16.
    let cycles = if count == 0x64 {
        28 + 20 + 10 + 16 + 10
    } else if (bus.rw(slot) as i16) >= 2 {
        bus.wb(slot + 1, bus.rb(slot + 1).wrapping_add(1));
        28 + 20 + 10 + 16 + 12 + 16 + 12 + 20
    } else {
        28 + 20 + 10 + 16 + 12 + 16 + 10
    };
    bus.wb(slot, 0);
    cycles + 16 + 16
}

#[cfg(test)]
#[path = "tasks/arcade_tests.rs"]
mod arcade_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fighter::Program;
    use crate::machine::MachineState;

    #[test]
    fn bonus_japan_player_pointer_and_palette_tail_branch_on_game_mode() {
        let mut rom = vec![0u8; 0x60000];
        for i in 0..32u8 {
            rom[0x41518 + i as usize] = 0x10 + i;
            rom[0x41538 + i as usize] = 0x80 + i;
        }
        // Valid 1x1 text block headers at 0x14786, 0x147a2, 0x147c0 so
        // bonus_japan_start can run over synthetic ROM bytes.
        for block in [0x14786usize, 0x147a2, 0x147c0] {
            rom[block + 2] = 1;
            rom[block + 3] = 1;
        }
        rom[0x414bc..0x414bf].copy_from_slice(&[0xff, 0xff, 0xff]);

        let mut program = Program::power_on(rom);
        let mut state = MachineState::new(
            vec![0; 0x1000],
            vec![0; 0x2000],
            vec![0; 0x800],
            0,
            0,
            0,
            false,
            0xff,
            0,
        );

        for copy in [BonusCopy::Japan, BonusCopy::Stage2] {
            // Mode 2 (1P on P1 side): 0xff8f14 = 0xff86b0, palette tail from 0x41518.
            {
                let mut bus = program.bus(&mut state);
                bus.wb(GAME_MODE, 2);
                let mut at = Flow::StageStart;
                let y = bonus_japan_start(&mut bus, &mut at, copy);
                assert!(matches!(y, Yield::Sleep(1)));
                assert_eq!(at, Flow::BonusJapanPalette { copy });
                assert_eq!(bus.rl(0xff8f14), 0xff86b0);
                bonus_load_palette_tail(&mut bus);
            }
            assert_eq!(
                &state.paletteram[0x3e0..0x400],
                &(0..32u8).map(|i| 0x10 + i).collect::<Vec<_>>()[..],
            );

            // Mode 4 (1P on P2 side): 0xff8f14 = 0xff86da, palette tail from 0x41538.
            {
                let mut bus = program.bus(&mut state);
                bus.wb(GAME_MODE, 4);
                let mut at = Flow::StageStart;
                let y = bonus_japan_start(&mut bus, &mut at, copy);
                assert!(matches!(y, Yield::Sleep(1)));
                assert_eq!(at, Flow::BonusJapanPalette { copy });
                assert_eq!(bus.rl(0xff8f14), 0xff86da);
                bonus_load_palette_tail(&mut bus);
            }
            assert_eq!(
                &state.paletteram[0x3e0..0x400],
                &(0..32u8).map(|i| 0x80 + i).collect::<Vec<_>>()[..],
            );
        }
    }
}
