//! Tier 2 SF1+ combat session (`#101`, `O10`).
//!
//! Data-driven state tables for the six Japan and USA fighters. The 1987
//! machine is not on this path: a session starts only when the caller builds
//! one. Faithful rules still select the 1987 special-damage and input-lockout
//! settings when a session is opened from that preset.

use crate::config::{MatchConfig, RulesetPreset};
use crate::roster::TournamentFighter;
use sf_video::asset_pack::{
    encode_adpcm, simple_fm_patch, validate_adpcm, validate_fm_patch, validate_tile, AdpcmClip,
    FmPatch, IndexedTile, Rgba,
};

pub const HEALTH_MAX: u16 = 1000;
pub const HUD_CELLS: u16 = 11;
pub const THROW_TECH_FRAMES: u8 = 7;
pub const CLASSIC_SPECIAL_DAMAGE: u16 = 420;
const STAGE_MIN: i32 = 16;
const STAGE_MAX: i32 = 368;
const CORNER_MARGIN: i32 = 24;
const WAKEUP_PUSH: i32 = 32;
const ATTACK_REACH: i32 = 48;
const THROW_REACH: i32 = 36;

/// Match toggles for the SF1+ session (`#101`, `O10`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sf1PlusRules {
    /// Specials deal a flat 42% of the 1,000 HP bar instead of the table value.
    pub classic_1987_special_damage: bool,
    /// Keep attack inputs entered during hitstop and apply them when it ends.
    pub disable_freeze_input_lockout: bool,
    /// Classic SF2 throws: the 7-frame tech window does not open.
    pub untechable_sf2_throws: bool,
    /// Leave a thrown fighter in the corner so the throw can loop.
    pub allow_corner_throw_loops: bool,
}

// Canonical gameplay schema: explicit tags and exhaustive fields (NP1).
crate::canonical::canonical_struct!(Sf1PlusRules, "Sf1PlusRules.v1", {
    classic_1987_special_damage, disable_freeze_input_lockout, untechable_sf2_throws, allow_corner_throw_loops,
});


impl Sf1PlusRules {
    pub const fn for_preset(preset: RulesetPreset) -> Self {
        let faithful = matches!(preset, RulesetPreset::Faithful1987);
        Self {
            classic_1987_special_damage: faithful,
            disable_freeze_input_lockout: !faithful,
            untechable_sf2_throws: false,
            allow_corner_throw_loops: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EraStyle {
    /// Shorter hitstun, no normal-to-special cancels, stand-only guard.
    Sf1987,
    /// Table hitstun, cancels, and high/low guard.
    Sf1Plus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Posture {
    Stand,
    Crouch,
    Jump,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Normal,
    CommandNormal,
    Throw,
    Special,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guard {
    High,
    Low,
    Mid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    Lp,
    Mp,
    Hp,
    Lk,
    Mk,
    Hk,
    CmdHp,
    Throw,
    QcfP,
    QcfK,
    DpP,
    DpK,
    QcbK,
    QcbP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveDef {
    pub name: &'static str,
    pub kind: MoveKind,
    pub posture: Posture,
    command: Command,
    pub startup: u8,
    pub active: u8,
    pub recovery: u8,
    pub hitstun: u8,
    pub blockstun: u8,
    pub pushback: i16,
    pub cancel_window: u8,
    pub damage: u16,
    pub guard: Guard,
    pub audio_latch: u8,
    pub tile: usize,
    pub anim_len: u8,
}

impl MoveDef {
    pub fn total_frames(self) -> u8 {
        self.startup + self.active + self.recovery
    }
}

#[derive(Debug, Clone)]
pub struct FighterKit {
    pub fighter: TournamentFighter,
    pub moves: Vec<MoveDef>,
    pub tiles: Vec<IndexedTile>,
    pub voice: AdpcmClip,
    pub patch: FmPatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pad {
    pub left: bool,
    pub right: bool,
    pub down: bool,
    pub up: bool,
    pub lp: bool,
    pub mp: bool,
    pub hp: bool,
    pub lk: bool,
    pub mk: bool,
    pub hk: bool,
}

impl MoveDef {
    /// Input sequence that starts this move from idle, facing right.
    pub fn rehearsal(self) -> Vec<Pad> {
        let quiet = Pad::default();
        let mut seq = Vec::new();
        match self.posture {
            Posture::Jump => {
                let mut up = quiet;
                up.up = true;
                seq.push(up);
                seq.push(quiet);
                seq.push(quiet);
            }
            Posture::Crouch => {}
            Posture::Stand => {}
        }
        match self.command {
            Command::QcfP
            | Command::QcfK
            | Command::DpP
            | Command::DpK
            | Command::QcbK
            | Command::QcbP => {
                seq.extend(motion(self.command));
            }
            _ => seq.push(press(self)),
        }
        seq
    }
}

fn press(mv: MoveDef) -> Pad {
    let mut pad = Pad::default();
    if mv.posture == Posture::Crouch {
        pad.down = true;
    }
    match mv.command {
        Command::Lp => pad.lp = true,
        Command::Mp => pad.mp = true,
        Command::Hp | Command::CmdHp => pad.hp = true,
        Command::Lk => pad.lk = true,
        Command::Mk => pad.mk = true,
        Command::Hk => pad.hk = true,
        Command::Throw => {
            pad.right = true;
            pad.lp = true;
            pad.lk = true;
        }
        Command::QcfP
        | Command::QcfK
        | Command::DpP
        | Command::DpK
        | Command::QcbK
        | Command::QcbP => {}
    }
    if mv.command == Command::CmdHp {
        pad.right = true;
    }
    pad
}

fn motion(command: Command) -> Vec<Pad> {
    let punch = Pad {
        lp: true,
        ..Pad::default()
    };
    let kick = Pad {
        lk: true,
        ..Pad::default()
    };
    let down = Pad {
        down: true,
        ..Pad::default()
    };
    let fwd = Pad {
        right: true,
        ..Pad::default()
    };
    let back = Pad {
        left: true,
        ..Pad::default()
    };
    let df = Pad {
        down: true,
        right: true,
        ..Pad::default()
    };
    match command {
        Command::QcfP => vec![down, df, fwd, punch],
        Command::QcfK => vec![down, df, fwd, kick],
        Command::DpP => vec![fwd, down, df, punch],
        Command::DpK => vec![fwd, down, df, kick],
        Command::QcbK => vec![down, back, kick],
        Command::QcbP => vec![down, back, punch],
        _ => vec![punch],
    }
}

pub fn hud_cells(health: u16) -> u8 {
    let health = health.min(HEALTH_MAX);
    if health == 0 {
        0
    } else {
        ((u32::from(health) * u32::from(HUD_CELLS) + 999) / 1000) as u8
    }
}

pub fn kit_for(fighter: TournamentFighter) -> FighterKit {
    let rows = move_rows(fighter);
    let mut tiles = Vec::new();
    let mut moves = Vec::with_capacity(rows.len());
    for (i, mut row) in rows.into_iter().enumerate() {
        let frames = if row.kind == MoveKind::Special { 4 } else { 1 };
        row.anim_len = frames;
        let tile = tiles.len();
        for frame in 0..frames as usize {
            tiles.push(pose_tile(fighter, i, frame));
        }
        moves.push(MoveDef { tile, ..row });
    }
    let voice = voice_clip(fighter);
    validate_adpcm(&voice).expect("SF1+ voice clip");
    let patch = simple_fm_patch(fighter_channel(fighter), 0x20);
    validate_fm_patch(&patch).expect("SF1+ FM cue");
    for tile in &tiles {
        validate_tile(tile).expect("SF1+ pose tile");
    }
    FighterKit {
        fighter,
        moves,
        tiles,
        voice,
        patch,
    }
}

fn fighter_channel(fighter: TournamentFighter) -> u8 {
    TournamentFighter::ALL
        .iter()
        .position(|f| *f == fighter)
        .unwrap_or(0) as u8
        & 7
}

fn pose_tile(fighter: TournamentFighter, move_index: usize, frame: usize) -> IndexedTile {
    let id = fighter_channel(fighter) as u8;
    let shift = frame % 4;
    let mut pixels = vec![Rgba::TRANSPARENT; 256];
    for y in 2..14 {
        for x in 4..12 {
            let on = (x + y + move_index + frame) % 3 != 0;
            let sx = x + shift;
            if on && sx < 16 {
                pixels[y * 16 + sx] = Rgba {
                    r: 40 + id * 20,
                    g: 30 + ((move_index + frame) as u8 % 7) * 18,
                    b: 20 + id * 12,
                    a: 255,
                };
            }
        }
    }
    sf_video::asset_pack::quantize_sheet(&pixels, 16, 16)
        .expect("pose sheet")
        .pop()
        .expect("one tile")
}

fn voice_clip(fighter: TournamentFighter) -> AdpcmClip {
    let base = 200 + fighter_channel(fighter) as i16 * 80;
    let pcm: Vec<i16> = (0..24).map(|i| base + i * 15).collect();
    encode_adpcm(&pcm).expect("voice")
}

fn move_rows(fighter: TournamentFighter) -> Vec<MoveDef> {
    let mut rows = Vec::new();
    for (posture, base) in [
        (Posture::Stand, 0u8),
        (Posture::Crouch, 1),
        (Posture::Jump, 2),
    ] {
        for (button, command, dmg, high) in [
            (0u8, Command::Lp, 20u16, true),
            (1, Command::Mp, 40, true),
            (2, Command::Hp, 60, true),
            (3, Command::Lk, 22, false),
            (4, Command::Mk, 42, false),
            (5, Command::Hk, 64, false),
        ] {
            let guard = if posture == Posture::Crouch && !high {
                Guard::Low
            } else if high {
                Guard::High
            } else {
                Guard::Mid
            };
            rows.push(blank(
                posture_name(posture, command),
                MoveKind::Normal,
                posture,
                command,
                2 + base,
                3,
                6 + button,
                10 + button,
                8,
                4 + button as i16,
                if button == 0 { 8 } else { 4 },
                dmg + u16::from(base) * 2,
                guard,
                0x81 + button,
            ));
        }
    }
    rows.push(blank(
        "command heavy",
        MoveKind::CommandNormal,
        Posture::Stand,
        Command::CmdHp,
        4,
        3,
        10,
        14,
        10,
        8,
        3,
        70,
        Guard::High,
        0x90,
    ));
    rows.push(blank(
        "ground throw",
        MoveKind::Throw,
        Posture::Stand,
        Command::Throw,
        2,
        1,
        12,
        0,
        0,
        0,
        0,
        120,
        Guard::Mid,
        0x91,
    ));
    rows.extend(specials(fighter));
    rows
}

fn specials(fighter: TournamentFighter) -> Vec<MoveDef> {
    match fighter {
        TournamentFighter::Ryu => vec![
            special("Hadoken", Command::QcfP, 8, 4, 16, 18, 12, 6, 90, 0xa0),
            special("Shoryuken", Command::DpP, 3, 6, 20, 22, 8, 4, 130, 0xa1),
            special("Tatsumaki", Command::QcbK, 6, 8, 14, 16, 10, 2, 100, 0xa2),
        ],
        TournamentFighter::Ken => vec![
            special("Hadoken", Command::QcfP, 8, 4, 16, 18, 12, 6, 90, 0xa3),
            special("Shoryuken", Command::DpP, 2, 8, 22, 24, 8, 3, 140, 0xa4),
            special("Tatsumaki", Command::QcbK, 5, 10, 12, 16, 10, 2, 110, 0xa5),
        ],
        TournamentFighter::Retsu => vec![special(
            "Kiai Shot",
            Command::QcfP,
            10,
            6,
            18,
            16,
            12,
            4,
            80,
            0xa6,
        )],
        TournamentFighter::Geki => vec![
            special("Shuriken", Command::QcfP, 7, 8, 14, 14, 10, 4, 70, 0xa7),
            special(
                "Teleport Strike",
                Command::QcbP,
                4,
                4,
                16,
                18,
                8,
                2,
                100,
                0xa8,
            ),
        ],
        TournamentFighter::Joe => vec![
            special(
                "Rolling Sobat",
                Command::QcbK,
                6,
                5,
                14,
                16,
                10,
                3,
                90,
                0xa9,
            ),
            special(
                "Dynamite Straight",
                Command::DpP,
                5,
                4,
                16,
                18,
                8,
                2,
                110,
                0xaa,
            ),
        ],
        TournamentFighter::Mike => vec![
            special("Bison Rush", Command::QcfP, 6, 6, 12, 16, 10, 4, 100, 0xab),
            special("Iron Upper", Command::DpP, 3, 5, 18, 20, 8, 2, 120, 0xac),
        ],
        TournamentFighter::Lee => vec![
            special("Zesshouohken", Command::QcfP, 7, 4, 16, 16, 10, 4, 90, 0xad),
            special("Double Kick", Command::QcbK, 5, 6, 14, 14, 8, 3, 80, 0xae),
        ],
        TournamentFighter::Gen => vec![
            special("Hyakurenko", Command::QcfP, 4, 8, 12, 14, 8, 2, 70, 0xaf),
            special("Gekiro", Command::DpP, 4, 5, 18, 18, 8, 3, 100, 0xb0),
        ],
        TournamentFighter::Birdie => vec![
            special("Bull Head", Command::QcfP, 8, 4, 16, 18, 12, 6, 100, 0xb1),
            special("Bull Horn", Command::QcbK, 6, 5, 14, 16, 10, 4, 90, 0xb2),
        ],
        TournamentFighter::Eagle => vec![
            special(
                "Manchester Black",
                Command::QcfP,
                6,
                4,
                16,
                16,
                10,
                4,
                90,
                0xb3,
            ),
            special(
                "St. Andrew's Green",
                Command::QcbK,
                7,
                5,
                15,
                16,
                10,
                3,
                95,
                0xb4,
            ),
        ],
        TournamentFighter::Adon => vec![
            special("Jaguar Kick", Command::QcbK, 6, 6, 14, 16, 10, 4, 90, 0xb5),
            special("Rising Jaguar", Command::DpP, 3, 6, 20, 20, 8, 2, 120, 0xb6),
        ],
        TournamentFighter::Sagat => vec![
            special(
                "High Tiger Shot",
                Command::QcfP,
                8,
                6,
                16,
                16,
                12,
                4,
                80,
                0xb7,
            ),
            special(
                "Low Tiger Shot",
                Command::QcfK,
                8,
                6,
                16,
                16,
                12,
                4,
                80,
                0xb8,
            ),
            special("Tiger Knee", Command::DpK, 5, 6, 18, 18, 8, 3, 110, 0xb9),
        ],
    }
}

fn special(
    name: &'static str,
    command: Command,
    startup: u8,
    active: u8,
    recovery: u8,
    hitstun: u8,
    blockstun: u8,
    pushback: i16,
    damage: u16,
    latch: u8,
) -> MoveDef {
    blank(
        name,
        MoveKind::Special,
        Posture::Stand,
        command,
        startup,
        active,
        recovery,
        hitstun,
        blockstun,
        pushback,
        0,
        damage,
        Guard::Mid,
        latch,
    )
}

fn blank(
    name: &'static str,
    kind: MoveKind,
    posture: Posture,
    command: Command,
    startup: u8,
    active: u8,
    recovery: u8,
    hitstun: u8,
    blockstun: u8,
    pushback: i16,
    cancel_window: u8,
    damage: u16,
    guard: Guard,
    audio_latch: u8,
) -> MoveDef {
    MoveDef {
        name,
        kind,
        posture,
        command,
        startup,
        active,
        recovery,
        hitstun,
        blockstun,
        pushback,
        cancel_window,
        damage,
        guard,
        audio_latch,
        tile: 0,
        anim_len: 1,
    }
}

fn posture_name(posture: Posture, command: Command) -> &'static str {
    match (posture, command) {
        (Posture::Stand, Command::Lp) => "stand LP",
        (Posture::Stand, Command::Mp) => "stand MP",
        (Posture::Stand, Command::Hp) => "stand HP",
        (Posture::Stand, Command::Lk) => "stand LK",
        (Posture::Stand, Command::Mk) => "stand MK",
        (Posture::Stand, Command::Hk) => "stand HK",
        (Posture::Crouch, Command::Lp) => "crouch LP",
        (Posture::Crouch, Command::Mp) => "crouch MP",
        (Posture::Crouch, Command::Hp) => "crouch HP",
        (Posture::Crouch, Command::Lk) => "crouch LK",
        (Posture::Crouch, Command::Mk) => "crouch MK",
        (Posture::Crouch, Command::Hk) => "crouch HK",
        (Posture::Jump, Command::Lp) => "jump LP",
        (Posture::Jump, Command::Mp) => "jump MP",
        (Posture::Jump, Command::Hp) => "jump HP",
        (Posture::Jump, Command::Lk) => "jump LK",
        (Posture::Jump, Command::Mk) => "jump MK",
        (Posture::Jump, Command::Hk) => "jump HK",
        _ => "move",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Act {
    Idle,
    Busy { index: usize, frame: u8, hit: bool },
    Knockdown { left: u8, pushed: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    D,
    F,
    B,
    Df,
}

#[derive(Debug, Clone)]
struct Body {
    era: EraStyle,
    health: u16,
    x: i32,
    facing: i8,
    crouch: bool,
    air_left: u8,
    stun: u8,
    blocking: bool,
    freeze: u8,
    act: Act,
    tech_left: u8,
    throwing: bool,
    buffer: Option<Pad>,
    dirs: Vec<Dir>,
    audio: Option<u8>,
    tile: Option<usize>,
}

impl Body {
    fn at(x: i32, facing: i8, era: EraStyle) -> Self {
        Self {
            era,
            health: HEALTH_MAX,
            x,
            facing,
            crouch: false,
            air_left: 0,
            stun: 0,
            blocking: false,
            freeze: 0,
            act: Act::Idle,
            tech_left: 0,
            throwing: false,
            buffer: None,
            dirs: Vec::new(),
            audio: None,
            tile: None,
        }
    }

    fn actionable(&self) -> bool {
        self.stun == 0 && self.tech_left == 0 && matches!(self.act, Act::Idle) && !self.throwing
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatEvent {
    Started(&'static str),
    Hit { damage: u16, hitstun: u8 },
    Blocked { blockstun: u8 },
    Canceled(&'static str),
    ThrowTech,
    ThrowConnect { damage: u16 },
    WakePush,
}

#[derive(Debug, Clone)]
pub struct CombatSession {
    pub rules: Sf1PlusRules,
    p1_kit: FighterKit,
    p2_kit: FighterKit,
    p1: Body,
    p2: Body,
    pub events: Vec<CombatEvent>,
}

impl CombatSession {
    pub fn new(p1: TournamentFighter, p2: TournamentFighter, rules: Sf1PlusRules) -> Self {
        Self::with_eras(p1, p2, rules, EraStyle::Sf1Plus, EraStyle::Sf1Plus)
    }

    pub fn from_config(p1: TournamentFighter, p2: TournamentFighter, config: &MatchConfig) -> Self {
        Self::new(p1, p2, config.sf1plus)
    }

    pub fn with_eras(
        p1: TournamentFighter,
        p2: TournamentFighter,
        rules: Sf1PlusRules,
        p1_era: EraStyle,
        p2_era: EraStyle,
    ) -> Self {
        Self {
            rules,
            p1_kit: kit_for(p1),
            p2_kit: kit_for(p2),
            p1: Body::at(120, 1, p1_era),
            p2: Body::at(156, -1, p2_era),
            events: Vec::new(),
        }
    }

    pub fn p1_health(&self) -> u16 {
        self.p1.health
    }
    pub fn p2_health(&self) -> u16 {
        self.p2.health
    }
    pub fn p2_x(&self) -> i32 {
        self.p2.x
    }
    pub fn p1_stun(&self) -> u8 {
        self.p1.stun
    }
    pub fn p2_stun(&self) -> u8 {
        self.p2.stun
    }
    pub fn p1_audio(&self) -> Option<u8> {
        self.p1.audio
    }
    pub fn p1_tile(&self) -> Option<&IndexedTile> {
        self.p1.tile.map(|i| &self.p1_kit.tiles[i])
    }
    pub fn p1_kit(&self) -> &FighterKit {
        &self.p1_kit
    }

    pub fn place(&mut self, p1_x: i32, p2_x: i32) {
        self.p1.x = p1_x;
        self.p2.x = p2_x;
    }

    pub fn step(&mut self, p1: Pad, p2: Pad) {
        self.cool_side(true);
        self.cool_side(false);
        let p1_pad = self.take_pad(true, p1);
        let p2_pad = self.take_pad(false, p2);
        self.face();
        self.read_dirs(true, p1_pad);
        self.read_dirs(false, p2_pad);
        if self.try_tech(p2_pad) {
            return;
        }
        self.advance_busy(true);
        self.advance_busy(false);
        if self.p1.actionable() {
            self.try_start(true, p1_pad);
        } else {
            self.try_cancel(true, p1_pad);
        }
        if self.p2.actionable() {
            self.try_start(false, p2_pad);
        }
    }

    pub fn play(&mut self, pads: &[Pad]) {
        for pad in pads {
            self.step(*pad, Pad::default());
        }
    }

    fn cool_side(&mut self, p1: bool) {
        let rules = self.rules;
        let body = if p1 { &mut self.p1 } else { &mut self.p2 };
        if body.freeze > 0 {
            body.freeze -= 1;
        }
        if body.stun > 0 {
            body.stun -= 1;
            if body.stun == 0 {
                body.blocking = false;
            }
        }
        if body.air_left > 0 {
            body.air_left -= 1;
        }
        let mut woke = false;
        if let Act::Knockdown { left, pushed } = body.act {
            if left <= 1 {
                if !pushed && cornered(body.x) && !rules.allow_corner_throw_loops {
                    let toward = if body.x < (STAGE_MIN + STAGE_MAX) / 2 {
                        1
                    } else {
                        -1
                    };
                    body.x = (body.x + toward * WAKEUP_PUSH).clamp(STAGE_MIN, STAGE_MAX);
                    woke = true;
                }
                body.act = Act::Idle;
            } else {
                body.act = Act::Knockdown {
                    left: left - 1,
                    pushed,
                };
            }
        }
        if woke {
            self.events.push(CombatEvent::WakePush);
        }
    }

    fn sees(&self, kind: Command) -> bool {
        let d = &self.p1.dirs;
        match kind {
            Command::QcfP | Command::QcfK => {
                d.ends_with(&[Dir::D, Dir::F]) || d.ends_with(&[Dir::D, Dir::Df, Dir::F])
            }
            Command::DpP | Command::DpK => d.ends_with(&[Dir::F, Dir::D, Dir::Df]),
            Command::QcbK | Command::QcbP => d.ends_with(&[Dir::D, Dir::B]),
            _ => false,
        }
    }

    fn take_pad(&mut self, p1: bool, pad: Pad) -> Pad {
        let buffer_inputs = self.rules.disable_freeze_input_lockout;
        let body = if p1 { &mut self.p1 } else { &mut self.p2 };
        if body.freeze > 0 && !buffer_inputs {
            return Pad::default();
        }
        if body.freeze > 0 && attack_pressed(pad) {
            body.buffer = Some(pad);
        }
        if body.freeze == 0 {
            if let Some(saved) = body.buffer.take() {
                if matches!(body.act, Act::Idle)
                    && body.stun == 0
                    && body.tech_left == 0
                    && !body.throwing
                {
                    return saved;
                }
                body.buffer = Some(saved);
            }
        }
        pad
    }

    fn face(&mut self) {
        if self.p2.x >= self.p1.x {
            self.p1.facing = 1;
            self.p2.facing = -1;
        } else {
            self.p1.facing = -1;
            self.p2.facing = 1;
        }
    }

    fn read_dirs(&mut self, p1: bool, pad: Pad) {
        let body = if p1 { &mut self.p1 } else { &mut self.p2 };
        body.crouch = pad.down && body.air_left == 0;
        if pad.up && body.actionable() {
            body.air_left = 10;
        }
        let forward = if body.facing > 0 { pad.right } else { pad.left };
        let back = if body.facing > 0 { pad.left } else { pad.right };
        let dir = if pad.down && forward {
            Some(Dir::Df)
        } else if pad.down && back {
            Some(Dir::B)
        } else if pad.down {
            Some(Dir::D)
        } else if forward {
            Some(Dir::F)
        } else if back {
            Some(Dir::B)
        } else {
            None
        };
        if let Some(dir) = dir {
            if body.dirs.last().copied() != Some(dir) {
                body.dirs.push(dir);
                if body.dirs.len() > 16 {
                    body.dirs.remove(0);
                }
            }
        }
    }

    fn try_tech(&mut self, p2: Pad) -> bool {
        if self.p2.tech_left == 0 || self.rules.untechable_sf2_throws {
            if self.p2.tech_left > 0 {
                self.p2.tech_left -= 1;
            }
            return false;
        }
        let teched = throw_buttons(p2);
        self.p2.tech_left -= 1;
        if teched {
            self.p1.throwing = false;
            self.p1.act = Act::Idle;
            self.p2.act = Act::Idle;
            self.p2.tech_left = 0;
            self.events.push(CombatEvent::ThrowTech);
            return true;
        }
        if self.p2.tech_left == 0 && self.p1.throwing {
            self.connect_throw();
        }
        false
    }

    fn connect_throw(&mut self) {
        let damage = self
            .p1_kit
            .moves
            .iter()
            .find(|m| m.kind == MoveKind::Throw)
            .map(|m| m.damage)
            .unwrap_or(120);
        self.p2.health = self.p2.health.saturating_sub(damage);
        self.p1.throwing = false;
        self.p1.act = Act::Idle;
        self.p2.act = Act::Knockdown {
            left: 16,
            pushed: false,
        };
        self.events.push(CombatEvent::ThrowConnect { damage });
    }

    fn advance_busy(&mut self, p1: bool) {
        let (index, frame, hit) = {
            let body = if p1 { &self.p1 } else { &self.p2 };
            match body.act {
                Act::Busy { index, frame, hit } => (index, frame, hit),
                _ => return,
            }
        };
        let mv = if p1 {
            self.p1_kit.moves[index]
        } else {
            self.p2_kit.moves[index]
        };
        let next = frame + 1;
        let landed = if !hit
            && next >= mv.startup
            && next < mv.startup + mv.active
            && mv.kind != MoveKind::Throw
        {
            self.resolve_hit(p1, mv)
        } else {
            false
        };
        let body = if p1 { &mut self.p1 } else { &mut self.p2 };
        if next >= mv.total_frames() {
            body.act = Act::Idle;
            body.tile = None;
        } else {
            body.act = Act::Busy {
                index,
                frame: next,
                hit: body_hit(hit, landed),
            };
            let span = usize::from(mv.anim_len.max(1));
            body.tile = Some(mv.tile + (usize::from(next) % span));
        }
    }

    fn resolve_hit(&mut self, attacker_p1: bool, mv: MoveDef) -> bool {
        let reach = (self.p1.x - self.p2.x).abs() <= ATTACK_REACH;
        if !reach {
            return false;
        }
        let (attacker_era, defender_crouch, defender_era, defender_back) = if attacker_p1 {
            (
                self.p1.era,
                self.p2.crouch,
                self.p2.era,
                holding_back(&self.p2),
            )
        } else {
            (
                self.p2.era,
                self.p1.crouch,
                self.p1.era,
                holding_back(&self.p1),
            )
        };
        let blocked = defender_back && guard_ok(defender_era, defender_crouch, mv.guard);
        if blocked {
            let stun = mv.blockstun;
            if attacker_p1 {
                self.p2.stun = stun;
                self.p2.blocking = true;
                self.p2.x = (self.p2.x + i32::from(self.p1.facing) * i32::from(mv.pushback))
                    .clamp(STAGE_MIN, STAGE_MAX);
                self.mark_hit(true);
            } else {
                self.p1.stun = stun;
                self.p1.blocking = true;
                self.mark_hit(false);
            }
            self.events.push(CombatEvent::Blocked { blockstun: stun });
            return true;
        }
        let hitstun = match attacker_era {
            EraStyle::Sf1Plus => mv.hitstun,
            EraStyle::Sf1987 => mv.hitstun / 2,
        };
        let damage = if mv.kind == MoveKind::Special && self.rules.classic_1987_special_damage {
            CLASSIC_SPECIAL_DAMAGE
        } else {
            mv.damage
        };
        if attacker_p1 {
            self.p2.health = self.p2.health.saturating_sub(damage);
            self.p2.stun = hitstun;
            self.p2.freeze = 4;
            self.p1.freeze = 4;
            self.p2.x = (self.p2.x + i32::from(self.p1.facing) * i32::from(mv.pushback))
                .clamp(STAGE_MIN, STAGE_MAX);
            self.mark_hit(true);
        } else {
            self.p1.health = self.p1.health.saturating_sub(damage);
            self.p1.stun = hitstun;
            self.mark_hit(false);
        }
        self.events.push(CombatEvent::Hit { damage, hitstun });
        true
    }

    fn mark_hit(&mut self, p1: bool) {
        let body = if p1 { &mut self.p1 } else { &mut self.p2 };
        if let Act::Busy { index, frame, .. } = body.act {
            body.act = Act::Busy {
                index,
                frame,
                hit: true,
            };
        }
    }

    fn try_start(&mut self, p1: bool, pad: Pad) {
        if !p1 {
            return;
        }
        let wanted = self.classify(pad);
        let Some(command) = wanted else { return };
        let posture = self.posture_of(pad);
        if command == Command::Throw {
            if (self.p1.x - self.p2.x).abs() > THROW_REACH || self.p2.air_left > 0 {
                return;
            }
            let index = self
                .p1_kit
                .moves
                .iter()
                .position(|m| m.kind == MoveKind::Throw)
                .unwrap();
            self.begin(index);
            self.p1.throwing = true;
            self.p2.tech_left = if self.rules.untechable_sf2_throws {
                0
            } else {
                THROW_TECH_FRAMES
            };
            if self.p2.tech_left == 0 {
                self.connect_throw();
            }
            return;
        }
        if let Some(index) = self.p1_kit.moves.iter().position(|m| {
            m.command == command
                && (m.posture == posture
                    || m.kind == MoveKind::Special
                    || m.kind == MoveKind::CommandNormal)
        }) {
            self.begin(index);
        }
    }

    fn try_cancel(&mut self, p1: bool, pad: Pad) {
        if !p1 || self.p1.era != EraStyle::Sf1Plus {
            return;
        }
        let Act::Busy { index, frame, hit } = self.p1.act else {
            return;
        };
        let current = self.p1_kit.moves[index];
        if current.kind != MoveKind::Normal && current.kind != MoveKind::CommandNormal {
            return;
        }
        if frame < current.startup || frame >= current.startup + current.cancel_window {
            return;
        }
        let Some(command) = self.classify(pad) else {
            return;
        };
        if !matches!(
            command,
            Command::QcfP
                | Command::QcfK
                | Command::DpP
                | Command::DpK
                | Command::QcbK
                | Command::QcbP
        ) {
            return;
        }
        let Some(next) = self
            .p1_kit
            .moves
            .iter()
            .position(|m| m.command == command && m.kind == MoveKind::Special)
        else {
            return;
        };
        let name = self.p1_kit.moves[next].name;
        self.begin(next);
        let _ = (hit,);
        self.events.push(CombatEvent::Canceled(name));
    }

    fn begin(&mut self, index: usize) {
        let mv = self.p1_kit.moves[index];
        self.p1.act = Act::Busy {
            index,
            frame: 0,
            hit: false,
        };
        self.p1.audio = Some(mv.audio_latch);
        self.p1.tile = Some(mv.tile);
        self.p1.dirs.clear();
        self.p1.buffer = None;
        self.events.push(CombatEvent::Started(mv.name));
    }

    fn posture_of(&self, pad: Pad) -> Posture {
        if self.p1.air_left > 0 {
            Posture::Jump
        } else if pad.down {
            Posture::Crouch
        } else {
            Posture::Stand
        }
    }

    fn classify(&mut self, pad: Pad) -> Option<Command> {
        if throw_buttons(pad) && (self.forward(pad) || self.back(pad)) {
            return Some(Command::Throw);
        }
        if self.sees(Command::QcfP) && (pad.lp || pad.mp || pad.hp) {
            return Some(Command::QcfP);
        }
        if self.sees(Command::QcfK) && (pad.lk || pad.mk || pad.hk) {
            return Some(Command::QcfK);
        }
        if self.sees(Command::DpP) && (pad.lp || pad.mp || pad.hp) {
            return Some(Command::DpP);
        }
        if self.sees(Command::DpK) && (pad.lk || pad.mk || pad.hk) {
            return Some(Command::DpK);
        }
        if self.sees(Command::QcbK) && (pad.lk || pad.mk || pad.hk) {
            return Some(Command::QcbK);
        }
        if self.sees(Command::QcbP) && (pad.lp || pad.mp || pad.hp) {
            return Some(Command::QcbP);
        }
        if self.forward(pad) && pad.hp && !pad.down {
            return Some(Command::CmdHp);
        }
        if pad.lp {
            return Some(Command::Lp);
        }
        if pad.mp {
            return Some(Command::Mp);
        }
        if pad.hp {
            return Some(Command::Hp);
        }
        if pad.lk {
            return Some(Command::Lk);
        }
        if pad.mk {
            return Some(Command::Mk);
        }
        if pad.hk {
            return Some(Command::Hk);
        }
        None
    }

    fn forward(&self, pad: Pad) -> bool {
        if self.p1.facing > 0 {
            pad.right
        } else {
            pad.left
        }
    }
    fn back(&self, pad: Pad) -> bool {
        if self.p1.facing > 0 {
            pad.left
        } else {
            pad.right
        }
    }
}

fn cornered(x: i32) -> bool {
    x <= STAGE_MIN + CORNER_MARGIN || x >= STAGE_MAX - CORNER_MARGIN
}

fn body_hit(was: bool, marked: bool) -> bool {
    was || marked
}

fn attack_pressed(pad: Pad) -> bool {
    pad.lp || pad.mp || pad.hp || pad.lk || pad.mk || pad.hk
}

fn throw_buttons(pad: Pad) -> bool {
    (pad.lp && pad.lk) || (pad.mp && pad.mk)
}

fn holding_back(body: &Body) -> bool {
    body.dirs.last().copied() == Some(Dir::B) || body.blocking
}

fn guard_ok(era: EraStyle, crouch: bool, guard: Guard) -> bool {
    match era {
        EraStyle::Sf1987 => !crouch && guard != Guard::Low,
        EraStyle::Sf1Plus => match guard {
            Guard::High => !crouch,
            Guard::Low => crouch,
            Guard::Mid => true,
        },
    }
}

pub const JAPAN_USA: [TournamentFighter; 6] = [
    TournamentFighter::Ryu,
    TournamentFighter::Ken,
    TournamentFighter::Retsu,
    TournamentFighter::Geki,
    TournamentFighter::Joe,
    TournamentFighter::Mike,
];
