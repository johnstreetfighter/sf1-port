//! The portrait select for a play mode, framed like every front-end screen,
//! with the twelve tournament fighters' VS-screen busts drawn at their own
//! size (pixel art is never resampled: a large frame holds a whole bust, a
//! tile a window onto its face).
//!
//! Arcade (one player): the large frame on the left shows the bust under
//! the cursor, the twelve tiles stand in three columns on the right, and
//! the ten bosses are greyed with the caption `SF1+`. Local (two players)
//! is laid out like the 1987 VS screen: P1's bust on the left facing
//! right, P2's on the right facing left, the names and `VS` between them,
//! and the twelve tiles in two rows of six below. Both players pick at
//! once: P1's cursor is yellow, P2's blue; they may rest on one tile;
//! confirm opens a side's Style card when era selection is active, then
//! locks the pick; the screen moves on the moment both are locked.
//!
//! Tiles a side may not pick are skipped by its cursor and, when no side
//! may pick them, drawn greyed: the bosses in Arcade, the ban list's
//! fighters, everyone but Ryu and Ken under Faithful 1987 (where the 1987
//! game binds Ryu to P1 and Ken to P2).

use sf_video::busts::Bust;
use sf_video::compositor::{blit_window, darken, ink, outline, RomFont, SelectPortrait};
use sf_video::render::Frame;

use super::draw::framed;
use super::fight::{first_opponent_flag, FightKind};
use super::style_card::{render_badge, StyleCard};
use super::Transition;
use crate::config::{MatchConfig, RulesetPreset};
use crate::era::{EraFamily, StylePick};
use crate::roster::{MenuInput, TournamentFighter};

/// Where a mode's pieces sit, in pixels inside the gold frame (which spans
/// 40..344 by 16..216; nothing is drawn on its foot row, 200..208).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    /// Grid columns (rows are 12 / cols).
    cols: usize,
    /// First tile's outline origin, the step between tiles, a tile's size.
    origin: (usize, usize),
    pitch: (usize, usize),
    tile: (usize, usize),
    /// The large frames' outlines (x, y, w, h): one in Arcade, P1's and
    /// P2's in Local. A bust is at most 98 pixels square, and the frames
    /// keep two pixels for a locked side's double outline, so every bust
    /// fits whole.
    panels: [(usize, usize, usize, usize); 2],
}

/// Arcade: the large frame left, three columns of four tiles right.
const ONE_PLAYER: Layout = Layout {
    cols: 3,
    origin: (168, 32),
    pitch: (56, 40),
    tile: (48, 36),
    panels: [(52, 28, 104, 104), (0, 0, 0, 0)],
};

/// Local: P1's and P2's frames on either side, two rows of six tiles below.
const TWO_PLAYERS: Layout = Layout {
    cols: 6,
    origin: (48, 136),
    pitch: (48, 32),
    tile: (46, 30),
    panels: [(48, 28, 102, 102), (234, 28, 102, 102)],
};

/// Who picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Picks {
    /// P1 and P2 at once (Local).
    Two,
    /// P1 only.
    One,
    /// Training: P1 picks a fighter, then the opponent (the CPU, or the
    /// dummy) with the same cursor.
    Sequential { cpu: bool },
}

/// The select screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectScreen {
    picks: Picks,
    /// Per side, the tiles that side may pick (before the mirror rule).
    allowed: [[bool; 12]; 2],
    /// Per tile, the caption drawn on it when greyed.
    caption: [&'static str; 12],
    cursors: [usize; 2],
    locked: [Option<TournamentFighter>; 2],
    /// What-If availability when era selection is active.
    styles: Option<bool>,
    dramatic_styles: bool,
    preferred_eras: [EraFamily; 2],
    cards: [Option<StyleCard>; 2],
    /// The cursor that moved last (Arcade's large frame follows P1's).
    last_moved: usize,
}

impl SelectScreen {
    fn open(picks: Picks, allowed: [[bool; 12]; 2], caption: [&'static str; 12]) -> Self {
        let first = |side: usize, from: usize| {
            (0..12).map(|i| (from + i) % 12).find(|&s| allowed[side][s]).unwrap_or(0)
        };
        SelectScreen {
            picks,
            allowed,
            caption,
            cursors: [first(0, 0), first(1, 1)],
            locked: [None, None],
            styles: None,
            dramatic_styles: false,
            preferred_eras: [EraFamily::Sf1Plus; 2],
            cards: [None, None],
            last_moved: 0,
        }
    }

    fn with_style_cards(mut self, config: &MatchConfig) -> Self {
        self.styles = config.era_select_enabled().then_some(config.roster.what_if_styles);
        self
    }

    pub(super) fn with_preferred_eras(mut self, eras: [EraFamily; 2]) -> Self {
        self.preferred_eras = eras;
        self
    }

    pub(super) fn preferred_eras(&self) -> [EraFamily; 2] {
        self.preferred_eras
    }

    /// Per side, what `config` and `player_tables` let that side pick: the
    /// ban list, the extended data bank's 1987 player tables (ADR-0014: a
    /// boss needs the art pack's poses), and under a preset without the
    /// full cast (Faithful 1987) the 1987 game's own binding, Ryu for P1
    /// and Ken for P2.
    fn allowed(config: &MatchConfig, player_tables: &[TournamentFighter]) -> [[bool; 12]; 2] {
        std::array::from_fn(|side| std::array::from_fn(|slot| {
            Self::fighter_available(config, player_tables, side, TournamentFighter::ALL[slot])
        }))
    }

    pub(super) fn fighter_available(
        config: &MatchConfig,
        player_tables: &[TournamentFighter],
        side: usize,
        fighter: TournamentFighter,
    ) -> bool {
        let bound = config.roster.full_cast || fighter == [TournamentFighter::Ryu, TournamentFighter::Ken][side];
        let playable = fighter.is_shoto() || player_tables.contains(&fighter);
        bound && playable && config.roster.ban.allows(fighter, side as u8, config.toggles.balanced_roster)
    }

    /// Local: P1 and P2 pick at once.
    pub fn local(config: &MatchConfig, player_tables: &[TournamentFighter]) -> Self {
        Self::open(Picks::Two, Self::allowed(config, player_tables), [""; 12])
            .with_style_cards(config)
    }

    /// Training: P1's fighter, then the opponent. Against the CPU under
    /// Faithful 1987 the opponent can only be one a flag brings first (the
    /// 1987 one-player game's ladder decides Capcom's CPU;
    /// [`first_opponent_flag`]). Under an overhaul preset the fair CPU plays
    /// the opponent through Player 2's pad (SP21), so he must be a fighter
    /// the 1987 player code runs: Ryu, Ken, or a boss in `player_tables`
    /// (the extended data bank's, ADR-0014). Either way the rest are greyed
    /// with `SF1+`. Against a dummy, any fighter P2 may be.
    pub fn training(config: &MatchConfig, cpu: bool, player_tables: &[TournamentFighter]) -> Self {
        let mut allowed = Self::allowed(config, player_tables);
        let mut caption = [""; 12];
        if cpu {
            let fair = config.preset != RulesetPreset::Faithful1987;
            for (slot, f) in TournamentFighter::ALL.iter().enumerate() {
                let reachable = if fair {
                    allowed[1][slot] && (f.is_shoto() || player_tables.contains(f))
                } else {
                    first_opponent_flag(*f).is_some_and(|flag| config.roster.four_country || flag % 2 == 0)
                };
                allowed[1][slot] = reachable;
                if !reachable {
                    caption[slot] = "SF1+";
                }
            }
        }
        Self::open(Picks::Sequential { cpu }, allowed, caption)
            .with_style_cards(config)
    }

    /// Arcade: under an overhaul preset with the art pack, every fighter
    /// [`Self::allowed`] opens for Player 1; without a boss's 1987 player
    /// table his tile is greyed with `SF1+`.
    pub fn arcade(config: &MatchConfig, player_tables: &[TournamentFighter]) -> Self {
        Self::single(config, player_tables, 0).with_style_cards(config)
    }

    pub(super) fn single(config: &MatchConfig, player_tables: &[TournamentFighter], side: usize) -> Self {
        let allowed = [Self::allowed(config, player_tables)[side], [false; 12]];
        let caption = std::array::from_fn(|i| {
            let f = TournamentFighter::ALL[i];
            if config.roster.full_cast && !f.is_shoto() && !player_tables.contains(&f) {
                "SF1+"
            } else {
                ""
            }
        });
        Self::open(Picks::One, allowed, caption)
    }

    pub(super) fn dramatic(config: &MatchConfig, player_tables: &[TournamentFighter], side: usize) -> Self {
        let mut select = Self::single(config, player_tables, side).with_style_cards(config);
        select.dramatic_styles = true;
        select
    }

    pub fn name(&self) -> &'static str {
        "select"
    }

    /// Whether both players pick here (Local).
    pub fn two_players(&self) -> bool {
        self.picks == Picks::Two
    }

    /// The fight this select leads to.
    pub fn kind(&self) -> FightKind {
        match self.picks {
            Picks::Two => FightKind::Local,
            Picks::One => FightKind::Arcade,
            Picks::Sequential { cpu: true } => FightKind::TrainingCpu,
            Picks::Sequential { cpu: false } => FightKind::TrainingDummy,
        }
    }

    /// Whether the screen shows two sides (Local and Training).
    fn two_sides(&self) -> bool {
        self.picks != Picks::One
    }

    /// The side the cursor picks for now: both at once in Local; in
    /// Training P1's until it is locked, then the opponent's.
    fn active_side(&self) -> usize {
        match self.picks {
            Picks::Sequential { .. } if self.locked[0].is_some() => 1,
            _ => 0,
        }
    }

    fn layout(&self) -> Layout {
        if self.two_sides() {
            TWO_PLAYERS
        } else {
            ONE_PLAYER
        }
    }

    /// Tile `slot`'s outline: x, y, width, height.
    pub fn tile_rect(&self, slot: usize) -> (usize, usize, usize, usize) {
        let l = self.layout();
        let slot = slot % 12;
        let (x, y) = (
            l.origin.0 + (slot % l.cols) * l.pitch.0,
            l.origin.1 + (slot / l.cols) * l.pitch.1,
        );
        (x, y, l.tile.0, l.tile.1)
    }

    /// The large frame of `side` (Arcade has only P1's): x, y, width,
    /// height.
    pub fn panel_rect(&self, side: usize) -> (usize, usize, usize, usize) {
        self.layout().panels[side]
    }

    /// Where a tile shows its fighter's name when it has no bust.
    pub fn name_origin(&self, slot: usize) -> (usize, usize) {
        let (x, y, _, h) = self.tile_rect(slot);
        (x + 2, y + h / 2 - 4)
    }

    /// The fighter under `side`'s cursor.
    pub fn fighter(&self, side: usize) -> TournamentFighter {
        TournamentFighter::ALL[self.cursors[side]]
    }

    pub(super) fn focus(&mut self, fighter: TournamentFighter) {
        if let Some(slot) = TournamentFighter::ALL.iter().position(|&candidate| candidate == fighter)
            .filter(|&slot| self.may_pick(0, slot)) {
            self.cursors[0] = slot;
        }
    }

    pub(super) fn restore_open_style(&mut self, pick: StylePick) {
        if let Some(what_if) = self.styles.filter(|_| self.cards[0].is_some()) {
            if let Some(card) = StyleCard::from_pick(self.fighter(0), pick, what_if) {
                self.cards[0] = Some(card);
            }
        }
    }

    /// `side`'s locked fighter, once picked.
    pub fn locked(&self, side: usize) -> Option<TournamentFighter> {
        self.locked[side]
    }

    /// The locked Style card pick. Presets without a card use their fixed style.
    pub fn locked_style(&self, side: usize) -> Option<StylePick> {
        self.locked(side)?;
        self.cards[side].as_ref().map(StyleCard::pick)
    }

    /// Apply a replicated cursor/lock only through the same eligibility table
    /// used by local input. The session checks the sender's assigned side.
    pub(crate) fn set_remote_pick(&mut self, side: usize, fighter: TournamentFighter, locked: bool, style: Option<StylePick>) -> Option<Transition> {
        let slot = TournamentFighter::ALL.iter().position(|candidate| *candidate == fighter)?;
        if side >= 2 || !self.may_pick(side, slot) { return None; }
        let card = match (self.styles, locked, style) {
            (Some(what_if), true, Some(pick)) => Some(StyleCard::from_pick(fighter, pick, what_if)?),
            (None, _, None) | (_, false, None) => None,
            _ => return None,
        };
        self.cursors[side] = slot;
        self.locked[side] = locked.then_some(fighter);
        self.cards[side] = card;
        self.last_moved = side;
        Some(match (self.locked[0], self.locked[1]) {
            (Some(p1), Some(p2)) => Transition::Fighters(p1, p2),
            _ if locked => Transition::Picked(fighter),
            _ => Transition::CursorMoved,
        })
    }

    /// Whether `slot`'s tile is greyed: no side picking now may pick it.
    pub fn greyed(&self, slot: usize) -> bool {
        match self.picks {
            Picks::Two => (0..2).all(|side| !self.allowed[side][slot]),
            _ => !self.allowed[self.active_side()][slot],
        }
    }

    /// Whether `side` may land on, and lock, `slot` now.
    fn may_pick(&self, side: usize, slot: usize) -> bool {
        self.allowed[side][slot]
    }

    /// The next tile `side` may pick from its cursor in steps of (`dc`,
    /// `dr`), if any.
    fn next_live(&self, side: usize, dc: isize, dr: isize) -> Option<usize> {
        let cols = self.layout().cols;
        let rows = 12 / cols;
        let at = self.cursors[side];
        let (mut c, mut r) = ((at % cols) as isize, (at / cols) as isize);
        loop {
            c += dc;
            r += dr;
            if !(0..cols as isize).contains(&c) || !(0..rows as isize).contains(&r) {
                return None;
            }
            let slot = r as usize * cols + c as usize;
            if self.may_pick(side, slot) {
                return Some(slot);
            }
            if dr != 0 {
                if let Some(s) = (0..cols).map(|cc| r as usize * cols + cc).find(|&s| self.may_pick(side, s)) {
                    return Some(s);
                }
            }
        }
    }

    /// One input from the merged menu cursor (Arcade and Training).
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        let side = self.active_side();
        if let Picks::Sequential { .. } = self.picks {
            if side == 1 && self.cards[1].is_none() && input == MenuInput::Back {
                return self.apply_side(0, input);
            }
        }
        self.apply_side(side, input)
    }

    /// One input for `side`. Up and down move between rows, left and right
    /// along a row, skipping tiles the side may not pick; confirm or Start
    /// locks; back unlocks, or leaves when nothing is locked.
    pub fn apply_side(&mut self, side: usize, input: MenuInput) -> Option<Transition> {
        if side == 1 && self.picks == Picks::One {
            return None;
        }
        if self.locked[side].is_some() {
            return (input == MenuInput::Back).then(|| {
                self.locked[side] = None;
                self.cards[side] = None;
                self.last_moved = side;
                Transition::Unpicked
            });
        }
        if let Some(card) = &mut self.cards[side] {
            return match input {
                MenuInput::Confirm | MenuInput::Start => {
                    self.preferred_eras[side] = card.pick().family;
                    Some(self.lock(side))
                }
                MenuInput::Back => {
                    self.cards[side] = None;
                    Some(Transition::Unpicked)
                }
                _ => card.change(input).then_some(Transition::CursorMoved),
            };
        }
        let next = match input {
            MenuInput::Left => self.next_live(side, -1, 0),
            MenuInput::Right => self.next_live(side, 1, 0),
            MenuInput::Up => self.next_live(side, 0, -1),
            MenuInput::Down => self.next_live(side, 0, 1),
            MenuInput::Confirm | MenuInput::Start => {
                if !self.may_pick(side, self.cursors[side]) {
                    return None;
                }
                if let Some(what_if) = self.styles {
                    let open = if self.dramatic_styles { StyleCard::open_dramatic } else { StyleCard::open };
                    self.cards[side] = Some(open(self.fighter(side), self.preferred_eras[side], what_if));
                    return Some(Transition::StyleOpened);
                }
                return Some(self.lock(side));
            }
            MenuInput::Back => {
                return (self.locked == [None, None] && self.cards.iter().all(Option::is_none)).then_some(Transition::Back);
            }
            _ => None,
        };
        next.map(|at| {
            self.cursors[side] = at;
            self.last_moved = side;
            Transition::CursorMoved
        })
    }

    fn lock(&mut self, side: usize) -> Transition {
        let fighter = self.fighter(side);
        if self.picks == Picks::One && self.cards[side].is_none() {
            return Transition::Fighter(fighter);
        }
        self.locked[side] = Some(fighter);
        self.last_moved = side;
        match (self.picks, self.locked) {
            (Picks::One, _) => Transition::Fighter(fighter),
            (_, [Some(p1), Some(p2)]) => Transition::Fighters(p1, p2),
            _ => Transition::Picked(fighter),
        }
    }

    /// The select in the front end's framed style: `PLAYER SELECT` in the
    /// ROM's header font inside the gold frame, the large frames, the
    /// tiles, and the names in the ROM's 8×8 font.
    pub fn render(&self, font: &RomFont<'_>, busts: &[Option<Bust>; 12]) -> Frame {
        self.render_named(font, busts, "PLAYER SELECT")
    }

    pub(super) fn render_named(&self, font: &RomFont<'_>, busts: &[Option<Bust>; 12], title: &str) -> Frame {
        let mut frame = framed(font, title, "");
        let argb = &mut frame.argb;
        let gold = font.pen(ink::GOLD_A, 2);
        let cursor_color = [font.pen(ink::YELLOW, 0), font.pen(ink::P2_BLUE, 2)];
        let side_ink = [ink::YELLOW, ink::P2_BLUE];
        let sides = if self.two_sides() { 2 } else { 1 };

        // The large frames: each side's fighter at the bust's own size,
        // centred (Local: P1 mirrored to face P2, as on the VS screen). A
        // locked side's frame is drawn in its colour, twice as thick.
        for side in 0..sides {
            let (px, py, pw, ph) = self.panel_rect(side);
            let locked = self.locked[side].is_some();
            let color = if locked { cursor_color[side] } else { gold };
            for i in 0..if locked { 2 } else { 1 } {
                outline(argb, px + i, py + i, pw - 2 * i, ph - 2 * i, color);
            }
            let shown = self.cursors[side];
            match &busts[shown] {
                Some(b) => {
                    let facing = if self.two_sides() && side == 0 { Some(b.mirrored()) } else { None };
                    let b = facing.as_ref().unwrap_or(b);
                    let (sx, sy) = (
                        (b.width as isize - (pw - 4) as isize) / 2,
                        (b.height as isize - (ph - 4) as isize) / 2,
                    );
                    blit_window(argb, px + 2, py + 2, pw - 4, ph - 4, &portrait(b), sx, sy);
                }
                None => {
                    let name = TournamentFighter::ALL[shown].name().to_ascii_uppercase();
                    font.text_px(argb, px + (pw - 8 * name.len()) / 2, py + ph / 2 - 4, &name, ink::BLUE);
                }
            }
            if !locked {
                if let Some(card) = &self.cards[side] {
                    card.render(argb, font, self.panel_rect(side), side_ink[side]);
                }
            }
        }

        // Who is on which fighter: under the frame in Arcade; between the
        // two frames in Local and Training, with VS in the header font.
        if self.two_sides() {
            let (mid_l, mid_r) = (self.panel_rect(0).0 + self.panel_rect(0).2, self.panel_rect(1).0);
            let centre = |len: usize| mid_l + (mid_r - mid_l - 8 * len) / 2;
            let labels = match self.picks {
                Picks::Sequential { cpu: true } => ["P1", "CPU"],
                Picks::Sequential { cpu: false } => ["P1", "DUMMY"],
                _ => ["P1", "P2"],
            };
            for side in 0..2 {
                let name = self.fighter(side).name().to_ascii_uppercase();
                let y = [36, 92][side];
                font.text_px(argb, centre(labels[side].len()), y, labels[side], side_ink[side]);
                let color = if self.locked[side].is_some() { side_ink[side] } else { ink::BLUE };
                font.text_px(argb, centre(name.len()), y + 12, &name, color);
                if let Some(card) = &self.cards[side] {
                    render_badge(argb, mid_l, y + 22, mid_r - mid_l, card.pick(), font.pen(side_ink[side], 0));
                }
            }
            let vs_row = if self.styles.is_some() { 9 } else { 8 };
            font.header(argb, (mid_l + mid_r) / 16 - 2, vs_row, "VS", ink::BLUE);
        } else {
            let (px, py, pw, ph) = self.panel_rect(0);
            let name = self.fighter(0).name().to_ascii_uppercase();
            font.text_px(argb, px, py + ph + 12, &name, ink::BLUE);
            if let Some(card) = &self.cards[0] {
                render_badge(argb, px, py + ph + 24, pw, card.pick(), font.pen(side_ink[0], 0));
            }
        }

        // The twelve tiles.
        for slot in 0..12 {
            let (x, y, w, h) = self.tile_rect(slot);
            let greyed = self.greyed(slot);
            outline(argb, x, y, w, h, if greyed { font.pen(ink::DIM, 2) } else { gold });
            let (iw, ih) = (w - 2, h - 2);
            match &busts[slot] {
                Some(b) => {
                    // A window onto the face: centred across, its middle 40%
                    // of the way down the bust.
                    let sx = (b.width as isize - iw as isize) / 2;
                    let sy = (b.height as isize * 2 / 5 - ih as isize / 2).max(0);
                    blit_window(argb, x + 1, y + 1, iw, ih, &portrait(b), sx, sy);
                }
                None => {
                    let name: String = TournamentFighter::ALL[slot]
                        .name()
                        .to_ascii_uppercase()
                        .chars()
                        .take(iw / 8)
                        .collect();
                    let (nx, ny) = self.name_origin(slot);
                    font.text_px(argb, nx, ny, &name, if greyed { ink::DIM } else { ink::BLUE });
                }
            }
            if greyed {
                darken(argb, x + 1, y + 1, iw, ih);
                let caption = self.caption[slot];
                font.text_px(argb, x + w - 2 - 8 * caption.len(), y + h - 10, caption, ink::YELLOW);
            }
        }

        // The cursors: P1's on its tile; P2's on its own, or inside P1's
        // when they share one. A locked cursor is twice as thick. Training
        // shows the opponent's cursor once P1's fighter is locked.
        for side in 0..sides {
            if side == 1 && self.active_side() == 0 && self.picks != Picks::Two {
                continue;
            }
            let (x, y, w, h) = self.tile_rect(self.cursors[side]);
            let inset = if side == 1 && self.cursors[1] == self.cursors[0] { 2 } else { 0 };
            let width = if self.locked[side].is_some() { 2 } else { 1 };
            for i in inset..inset + width {
                outline(argb, x + i, y + i, w - 2 * i, h - 2 * i, cursor_color[side]);
            }
        }
        frame
    }
}

fn portrait(b: &Bust) -> SelectPortrait<'_> {
    SelectPortrait {
        argb: &b.argb,
        width: b.width,
        height: b.height,
    }
}
