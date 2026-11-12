//! The Main Menu and its Single Player and Multi Player sub-menus: framed
//! lists with the yellow cursor, items that do not exist yet greyed and
//! skipped. The Main Menu's edition logo stands inside the top of its
//! unbroken frame, with the rows under it.

use sf_video::compositor::{ink, RomFont};
use sf_video::render::{Frame, HEIGHT, WIDTH};

use super::attract::TitleLogo;
use super::draw::{centred_col, framed, BLACK, FRAME_COL, FRAME_COLS, FRAME_ROW, FRAME_ROWS};
use super::Transition;
use crate::roster::MenuInput;
use crate::version::EDITION_NAME;

/// Every item of the menu tree (design section 5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem {
    SinglePlayer,
    MultiPlayer,
    Settings,
    Cabinet1987,
    Exit,
    Arcade,
    Training,
    WorldTour,
    DramaticBattle,
    Local,
    Online,
    LocalCoopDramatic,
    Rematch,
    Reselect,
    Menu,
    TrainingCpu,
    TrainingDummy,
    Resume,
    Overlays,
    /// The pause box's `LOOK`: the player's `CRT LOOK`, stepped live.
    Look,
    GaugeArt,
    RestartRound,
    /// Training's `RESET POSITIONS`, in `RestartRound`'s place: the round's
    /// start with the fight's present configuration kept.
    ResetPositions,
    QuitToMenu,
    /// Training's pause-box `TRAINING` row: opens the Training settings box.
    TrainingSettings,
    LivePreset,
    /// Training's `HOLD COMBO`: the last combo count stays up.
    HoldCombo,
    /// Training's `FILL METERS`: every era gauge's resource kept full.
    FillMeters,
    /// The pause box's `MOVES`: the fighter's move list.
    Moves,
    PowerOn,
    Retry,
    BrowseRomFolder,
    BrowseRomArchive,
    RebindP1,
    RebindP2,
    RebindMenu,
    /// The Video tab's `TUNE CRT LOOK >`: the tuning screen.
    TuneLook,
    /// The Sound tab's `JUKEBOX`: the Jukebox screen.
    Jukebox,
}

impl MenuItem {
    /// The row's text, in the ROM font's character set.
    pub const fn label(self) -> &'static str {
        match self {
            MenuItem::SinglePlayer => "SINGLE PLAYER",
            MenuItem::MultiPlayer => "MULTI PLAYER",
            MenuItem::Settings => "SETTINGS",
            MenuItem::Cabinet1987 => "1987",
            MenuItem::Exit => "EXIT",
            MenuItem::Arcade => "ARCADE MODE",
            MenuItem::Training => "TRAINING",
            MenuItem::WorldTour => "WORLD TOUR",
            MenuItem::DramaticBattle => "DRAMATIC BATTLE",
            MenuItem::Local => "LOCAL",
            MenuItem::Online => "ONLINE",
            MenuItem::LocalCoopDramatic => "LOCAL CO-OP DRAMATIC",
            MenuItem::Rematch => "REMATCH",
            MenuItem::Reselect => "RESELECT",
            MenuItem::Menu => "MENU",
            MenuItem::TrainingCpu => "CPU",
            MenuItem::TrainingDummy => "DUMMY",
            MenuItem::Resume => "RESUME",
            MenuItem::Overlays => "OVERLAYS",
            MenuItem::Look => "LOOK",
            MenuItem::GaugeArt => "GAUGE ART",
            MenuItem::RestartRound => "RESTART ROUND",
            MenuItem::ResetPositions => "RESET POSITIONS",
            MenuItem::QuitToMenu => "QUIT TO MENU",
            MenuItem::TrainingSettings => "TRAINING",
            MenuItem::LivePreset => "PRESET",
            MenuItem::HoldCombo => "HOLD COMBO",
            MenuItem::FillMeters => "FILL METERS",
            MenuItem::Moves => "MOVES",
            MenuItem::PowerOn => "POWER ON",
            MenuItem::Retry => "RETRY",
            MenuItem::BrowseRomFolder => "BROWSE FOLDER",
            MenuItem::BrowseRomArchive => "BROWSE ARCHIVE",
            MenuItem::RebindP1 => "REBIND P1",
            MenuItem::RebindP2 => "REBIND P2",
            MenuItem::RebindMenu => "REBIND MENU",
            MenuItem::TuneLook => "TUNE CRT LOOK",
            MenuItem::Jukebox => "JUKEBOX",
        }
    }

    /// Whether the item exists yet. Greyed items are drawn dim and skipped
    /// by the cursor. World Tour stays greyed until its project.
    pub const fn available(self) -> bool {
        matches!(
            self,
            MenuItem::SinglePlayer
                | MenuItem::MultiPlayer
                | MenuItem::Settings
                | MenuItem::Cabinet1987
                | MenuItem::PowerOn
                | MenuItem::Retry
                | MenuItem::BrowseRomFolder
                | MenuItem::BrowseRomArchive
                | MenuItem::RebindP1
                | MenuItem::RebindP2
                | MenuItem::RebindMenu
                | MenuItem::Exit
                | MenuItem::Local
                | MenuItem::Online
                | MenuItem::DramaticBattle
                | MenuItem::LocalCoopDramatic
                | MenuItem::Arcade
                | MenuItem::Training
                | MenuItem::TrainingCpu
                | MenuItem::TrainingDummy
                | MenuItem::Resume
                | MenuItem::Overlays
                | MenuItem::Look
                | MenuItem::GaugeArt
                | MenuItem::RestartRound
                | MenuItem::ResetPositions
                | MenuItem::QuitToMenu
                | MenuItem::TrainingSettings
                | MenuItem::LivePreset
                | MenuItem::HoldCombo
                | MenuItem::FillMeters
                | MenuItem::Moves
                | MenuItem::Rematch
                | MenuItem::Reselect
                | MenuItem::Menu
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuKind {
    Main,
    SinglePlayer,
    MultiPlayer,
    Training,
}

const MAIN: &[MenuItem] = &[
    MenuItem::SinglePlayer,
    MenuItem::MultiPlayer,
    MenuItem::Settings,
    MenuItem::Cabinet1987,
    MenuItem::Exit,
];
const SINGLE: &[MenuItem] = &[
    MenuItem::Arcade,
    MenuItem::Training,
    MenuItem::WorldTour,
    MenuItem::DramaticBattle,
];
const MULTI: &[MenuItem] = &[
    MenuItem::Local,
    MenuItem::Online,
    MenuItem::LocalCoopDramatic,
];
const TRAINING: &[MenuItem] = &[MenuItem::TrainingCpu, MenuItem::TrainingDummy];

/// The first item's row; items follow every second row.
const FIRST_ROW: usize = 8;
const ROW_STEP: usize = 2;
/// Give the full-size logo a tile of space below the top border.
const MAIN_FRAME_ROW: usize = FRAME_ROW - 1;
const SF1_PLUS_SUBTITLE_Y: usize = 15 * 8;
/// Leave room after the subtitle and fit all five choices above the notice.
const MAIN_FIRST_ITEM_Y: usize = 134;
const MAIN_ITEM_STEP: usize = 14;
/// The row of a one-line notice (an untranslated branch, say).
pub const NOTICE_ROW: usize = 21;
/// The Main Menu's notice row: the one free row under its last item.
const MAIN_NOTICE_ROW: usize = 25;
/// The row of a question above the items (Training's `OPPONENT`).
pub const PROMPT_ROW: usize = 5;

/// A framed list menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuScreen {
    kind: MenuKind,
    /// The cursor's item, or None when every item is greyed.
    cursor: Option<usize>,
    dramatic_enabled: bool,
}

impl MenuScreen {
    pub fn main() -> Self {
        Self::open(MenuKind::Main)
    }

    pub fn single_player() -> Self {
        Self::open(MenuKind::SinglePlayer)
    }

    pub fn multi_player() -> Self {
        Self::open(MenuKind::MultiPlayer)
    }

    /// Training's first question: `OPPONENT`, `CPU` or `DUMMY`.
    pub fn training() -> Self {
        Self::open(MenuKind::Training)
    }

    fn open(kind: MenuKind) -> Self {
        let mut screen = MenuScreen { kind, cursor: None, dramatic_enabled: true };
        screen.cursor = screen.items().iter().position(|&item| screen.item_available(item));
        screen
    }

    pub(super) fn with_dramatic_enabled(mut self, enabled: bool) -> Self {
        self.dramatic_enabled = enabled;
        self.cursor = self.cursor.filter(|&row| self.item_available(self.items()[row]))
            .or_else(|| self.items().iter().position(|&item| self.item_available(item)));
        self
    }

    fn item_available(&self, item: MenuItem) -> bool {
        item.available() && (self.dramatic_enabled
            || !matches!(item, MenuItem::DramaticBattle | MenuItem::LocalCoopDramatic))
    }

    /// The menu with its cursor on row `row`, when that row exists and is
    /// live (a saved position); otherwise on its first live row.
    pub fn at(mut self, row: u8) -> Self {
        let row = row as usize;
        if self.items().get(row).is_some_and(|&item| self.item_available(item)) {
            self.cursor = Some(row);
        }
        self
    }

    /// Which of the four menus this is, as the settings file counts them:
    /// Main Menu, Single Player, Multi Player, Training.
    pub fn slot(&self) -> usize {
        match self.kind {
            MenuKind::Main => 0,
            MenuKind::SinglePlayer => 1,
            MenuKind::MultiPlayer => 2,
            MenuKind::Training => 3,
        }
    }

    /// The cursor's row and item, if any row is live.
    pub fn cursor(&self) -> Option<(u8, MenuItem)> {
        self.cursor.map(|at| (at as u8, self.items()[at]))
    }

    /// The screen's name, as the session reports it.
    pub fn name(&self) -> &'static str {
        match self.kind {
            MenuKind::Main => "main menu",
            MenuKind::SinglePlayer => "single player",
            MenuKind::MultiPlayer => "multi player",
            MenuKind::Training => "training",
        }
    }

    /// The header in the frame's top edge; the Main Menu has none.
    fn header(&self) -> &'static str {
        match self.kind {
            MenuKind::Main => "",
            MenuKind::SinglePlayer => "SINGLE PLAYER",
            MenuKind::MultiPlayer => "MULTI PLAYER",
            MenuKind::Training => "TRAINING",
        }
    }

    /// The items, top to bottom.
    pub fn items(&self) -> &'static [MenuItem] {
        match self.kind {
            MenuKind::Main => MAIN,
            MenuKind::SinglePlayer => SINGLE,
            MenuKind::MultiPlayer => MULTI,
            MenuKind::Training => TRAINING,
        }
    }

    /// The top pixel row of item `index`.
    pub fn item_y(&self, index: usize) -> usize {
        if self.kind == MenuKind::Main {
            MAIN_FIRST_ITEM_Y + MAIN_ITEM_STEP * index
        } else {
            (FIRST_ROW + ROW_STEP * index) * 8
        }
    }

    /// The first text column of `item`'s label (labels are centred).
    pub fn item_col(item: MenuItem) -> usize {
        centred_col(item.label())
    }

    /// Apply one input edge. Up and down move among the items that exist,
    /// wrapping; confirm or Start chooses; back leaves.
    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        match input {
            MenuInput::Up | MenuInput::Down => {
                let at = self.cursor?;
                let n = self.items().len();
                let step = if input == MenuInput::Down { 1 } else { n - 1 };
                let mut next = at;
                loop {
                    next = (next + step) % n;
                    if self.item_available(self.items()[next]) || next == at {
                        break;
                    }
                }
                (next != at).then(|| {
                    self.cursor = Some(next);
                    Transition::CursorMoved
                })
            }
            MenuInput::Confirm | MenuInput::Start => {
                self.cursor.map(|at| Transition::Choose(self.items()[at]))
            }
            MenuInput::Back => Some(Transition::Back),
            _ => None,
        }
    }

    /// The framed menu: header (on the Main Menu, the title `logo` instead),
    /// rows (cursor yellow with the ROM's arrow, greyed rows dim), and a
    /// one-line `notice` if any.
    pub fn render(&self, font: &RomFont<'_>, logo: Option<&TitleLogo>, notice: Option<&str>) -> Frame {
        let mut frame = if self.kind == MenuKind::Main {
            let mut argb = vec![BLACK; WIDTH * HEIGHT];
            font.gold_frame(
                &mut argb,
                FRAME_COL,
                MAIN_FRAME_ROW,
                FRAME_COLS,
                FRAME_ROW + FRAME_ROWS - MAIN_FRAME_ROW,
                0..0,
            );
            if let Some(logo) = logo {
                logo.draw(font, &mut argb);
                if let Some(subtitle) = EDITION_NAME.strip_prefix("Street Fighter': ") {
                    draw_sf1_plus_branding(font, &mut argb, subtitle);
                }
            }
            Frame { argb }
        } else {
            framed(font, self.header(), "")
        };
        if self.kind == MenuKind::Training {
            font.text_centred(&mut frame.argb, PROMPT_ROW, "OPPONENT", ink::BLUE);
        }
        if let Some(notice) = notice {
            let row = if self.kind == MenuKind::Main { MAIN_NOTICE_ROW } else { NOTICE_ROW };
            font.text_centred(&mut frame.argb, row, notice, ink::YELLOW);
        }
        for (i, item) in self.items().iter().enumerate() {
            let (x, y) = (Self::item_col(*item) * 8, self.item_y(i));
            let color = if !self.item_available(*item) {
                ink::DIM
            } else if self.cursor == Some(i) {
                font.text_px(&mut frame.argb, x - 16, y, ">", ink::YELLOW);
                ink::YELLOW
            } else {
                ink::BLUE
            };
            font.text_px(&mut frame.argb, x, y, item.label(), color);
        }
        frame
    }
}

fn draw_sf1_plus_branding(font: &RomFont<'_>, argb: &mut [u32], subtitle: &str) {
    // The accent sits just above and to the right of the ROM logo's final R.
    const PRIME_X: usize = 304;
    const PRIME_Y: usize = 41;
    const PRIME: [&str; 13] = [
        "       ##",
        "      ###+",
        "     ####+",
        "     ####+",
        "    ####++",
        "    ###++",
        "   ####+",
        "  ####+",
        "  ###+",
        " ###+",
        " ##+",
        "#+",
        "+",
    ];
    for (dy, row) in PRIME.iter().enumerate() {
        for (dx, pixel) in row.bytes().enumerate() {
            let color = match pixel {
                b'#' => 0xffdd_0000,
                b'+' => 0xffdd_9900,
                _ => continue,
            };
            argb[(PRIME_Y + dy) * WIDTH + PRIME_X + dx] = color;
        }
    }

    // A shallow shear gives the cream subtitle an italic slant at native resolution.
    const PENS: [u32; 3] = [0xffff_f3cb, 0xff7d_4919, 0xffec_d99c];
    let width = subtitle.chars().count() * 8;
    let left = (WIDTH - width - 3) / 2;
    for (i, ch) in subtitle.chars().enumerate() {
        font.glyph_in(argb, left + i * 8, SF1_PLUS_SUBTITLE_Y, ch.to_ascii_uppercase(), PENS);
    }
    for dy in 0..8 {
        let shift = (7 - dy) / 2;
        let start = (SF1_PLUS_SUBTITLE_Y + dy) * WIDTH + left;
        argb.copy_within(start..start + width, start + shift);
        argb[start..start + shift].fill(0xff00_0000);
    }
}
