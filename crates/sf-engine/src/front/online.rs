//! The socket-free online lobby for current tournament fights.

use super::Transition;
use super::TypedKey;
use crate::roster::MenuInput;
use crate::netplay::{decode_room_code, Link, Room, RoomKind};
use sf_video::compositor::{fallback_text, ink, RomFont};
use sf_video::render::Frame;
use super::draw::{fit, framed_wide_at};

pub const NET_ENTRY_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.: ";
const ENTRY_MAX: usize = 32;
const BROWSE_ROOM_START: usize = 4;
const DIRECT_JOIN_ROW: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnlineAction {
    Refresh,
    Join { addr: String, kind: RoomKind },
    Host { port: u16, public: bool, wired_only: bool, kind: RoomKind },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LobbyView {
    Browse,
    Host,
    Direct,
}

#[derive(Clone, Debug)]
pub struct OnlineScreen {
    view: LobbyView,
    cursor: usize,
    wired_only: bool,
    join_kind: RoomKind,
    host_kind: RoomKind,
    rooms: Vec<Room>,
    port: u16,
    public: bool,
    entry: String,
    at: usize,
    editing: bool,
    notice: String,
    room_code: Option<String>,
    actions: Vec<OnlineAction>,
}

impl OnlineScreen {
    pub fn open(wired_only: bool, port: u16) -> Self {
        Self {
            view: LobbyView::Browse, cursor: 0, wired_only, rooms: Vec::new(),
            join_kind: RoomKind::Versus, host_kind: RoomKind::Versus,
            port: port.max(1024), public: true,
            entry: String::new(), at: 0, editing: false,
            notice: String::new(), room_code: None, actions: Vec::new(),
        }
    }

    pub fn view(&self) -> LobbyView { self.view }
    pub fn name(&self) -> &'static str { "online" }

    pub fn wired_only(&self) -> bool { self.wired_only }
    pub fn typing(&self) -> bool { self.editing }
    pub fn entry(&self) -> &str { &self.entry }
    pub fn notice(&self) -> &str { &self.notice }
    pub fn room_code(&self) -> Option<&str> { self.room_code.as_deref() }

    pub(super) fn room_code_for_copy(&self) -> Option<&str> {
        match self.view {
            LobbyView::Browse => self.selected_room().map(|room| room.code.as_str()),
            LobbyView::Host => self.room_code(),
            LobbyView::Direct => None,
        }
    }

    pub(super) fn accepts_room_code_paste(&self) -> bool {
        self.view == LobbyView::Direct
    }

    pub(super) fn paste_room_code(&mut self, text: Option<&str>) {
        if !self.accepts_room_code_paste() {
            return;
        }
        let Some(text) = text else {
            self.set_notice("NO ROOM CODE ON CLIPBOARD");
            return;
        };
        let text = text.trim();
        if text.len() > ENTRY_MAX {
            self.set_notice("INVALID CODE OR IPV4 ADDRESS");
            return;
        }
        let code = text.to_ascii_uppercase();
        if decode_room_code(&code).is_none() {
            self.set_notice("INVALID CODE OR IPV4 ADDRESS");
            return;
        }
        self.entry = code;
        self.at = self.entry.len();
        self.editing = false;
        self.cursor = DIRECT_JOIN_ROW;
        self.set_notice("ROOM CODE PASTED");
    }

    pub fn set_bound(&mut self, room_code: impl Into<String>) {
        self.room_code = Some(room_code.into());
        self.view = LobbyView::Host;
        self.cursor = 0;
        self.editing = false;
        self.notice = "WAITING FOR CHALLENGER...".into();
    }
    pub fn set_notice(&mut self, notice: impl Into<String>) { self.notice = notice.into(); }
    pub fn drain_actions(&mut self) -> Vec<OnlineAction> { std::mem::take(&mut self.actions) }

    fn join(&mut self) -> Option<Transition> {
        self.join_code(&self.entry.clone(), self.join_kind)
    }

    fn join_code(&mut self, code: &str, kind: RoomKind) -> Option<Transition> {
        let Some((ip, port)) = decode_room_code(code.trim()) else {
            self.notice = "INVALID CODE OR IPV4 ADDRESS".into();
            return Some(Transition::ValueChanged);
        };
        self.actions.push(OnlineAction::Join {
            addr: format!("{}.{}.{}.{}:{port}", ip[0], ip[1], ip[2], ip[3]),
            kind,
        });
        self.editing = false;
        self.notice = "CONNECTING...".into();
        Some(Transition::ValueChanged)
    }

    pub fn type_key(&mut self, key: TypedKey) -> Option<Transition> {
        if !self.editing { return None; }
        match key {
            TypedKey::Char(ch) => {
                let ch = ch.to_ascii_uppercase();
                if !NET_ENTRY_CHARS.contains(ch) || self.entry.len() >= ENTRY_MAX { return None; }
                self.entry.insert(self.at, ch);
                self.at += 1;
            }
            TypedKey::Erase if self.at > 0 => { self.at -= 1; self.entry.remove(self.at); }
            TypedKey::EraseAhead if self.at < self.entry.len() => { self.entry.remove(self.at); }
            TypedKey::Left => self.at = self.at.saturating_sub(1),
            TypedKey::Right => self.at = (self.at + 1).min(self.entry.len()),
            TypedKey::Home => self.at = 0,
            TypedKey::End => self.at = self.entry.len(),
            TypedKey::Enter => return self.join(),
            TypedKey::Escape => self.editing = false,
            _ => return None,
        }
        Some(Transition::ValueChanged)
    }

    fn listed_rooms(&self) -> impl Iterator<Item = &Room> {
        self.rooms.iter().filter(|room| !self.wired_only || room.link == Link::Ethernet)
    }

    fn selected_room(&self) -> Option<&Room> {
        let index = self.cursor.checked_sub(BROWSE_ROOM_START)?;
        self.listed_rooms().nth(index)
    }

    pub fn visible_rooms(&self) -> Vec<&Room> {
        self.listed_rooms().collect()
    }

    pub fn set_rooms(&mut self, rooms: Vec<Room>) {
        self.rooms = rooms;
        if self.view == LobbyView::Browse && self.cursor >= BROWSE_ROOM_START
            && self.visible_rooms().is_empty() {
            self.cursor = 2;
        }
        self.cursor = self.cursor.min(self.rows() - 1);
    }

    fn rows(&self) -> usize {
        match self.view {
            LobbyView::Browse => BROWSE_ROOM_START + self.visible_rooms().len(),
            LobbyView::Host => 5,
            LobbyView::Direct => 4,
        }
    }

    fn draw_row(&self, frame: &mut Frame, font: &RomFont<'_>, index: usize, row: usize, label: &str, value: &str) {
        let color = if self.cursor == index { ink::YELLOW } else { ink::BLUE };
        if self.cursor == index { font.text(&mut frame.argb, 3, row, ">", color); }
        font.text(&mut frame.argb, 5, row, label, color);
        font.text(&mut frame.argb, 20, row, value, color);
    }

    pub fn render(&self, font: &RomFont<'_>) -> Frame {
        self.render_with_title(font, 0)
    }

    pub(super) fn render_with_title(&self, font: &RomFont<'_>, title_offset: usize) -> Frame {
        let mut frame = framed_wide_at(font, "ONLINE LOBBY", "", title_offset);
        let view = match self.view {
            LobbyView::Browse => "< BROWSE ROOMS >",
            LobbyView::Host => "< HOST MATCH >",
            LobbyView::Direct => "< DIRECT JOIN >",
        };
        self.draw_row(&mut frame, font, 0, 4, "MODE", view);
        let on_off = |on| if on { "ON" } else { "OFF" };
        match self.view {
            LobbyView::Browse => {
                self.draw_row(&mut frame, font, 1, 6, "WIRED ONLY", on_off(self.wired_only));
                self.draw_row(&mut frame, font, 2, 7, "REFRESH ROOMS", "");
                self.draw_row(&mut frame, font, 3, 9, "JOIN AS",
                    if self.join_kind == RoomKind::Spectate { "SPECTATOR" } else { "PLAYER" });
                for (x, text) in [(16, "CODE"), (94, "HOST"), (150, "MODE"), (176, "RULESET"),
                    (234, "PING"), (270, "RBF"), (294, "JIT"), (326, "LINK")] {
                    fallback_text(&mut frame.argb, x, 88, text, 0xff90_90a0, 1);
                }
                let rooms = self.visible_rooms();
                let first = self.cursor.saturating_sub(BROWSE_ROOM_START).saturating_sub(5);
                for (shown, (index, room)) in rooms.iter().enumerate().skip(first).take(6).enumerate() {
                    let y = 100 + shown * 12;
                    let color = if self.cursor == index + BROWSE_ROOM_START { 0xffff_d438 } else { 0xff90_a8ef };
                    let preset = match room.preset {
                        crate::config::RulesetPreset::DefinitiveOverhaul => "DEFINITIVE",
                        crate::config::RulesetPreset::Faithful1987 => "FAITHFUL",
                        crate::config::RulesetPreset::ClassicCompetitive => "CLASSIC",
                        crate::config::RulesetPreset::PureFootsies1987Plus => "FOOTSIES",
                        crate::config::RulesetPreset::Custom => "CUSTOM",
                    };
                    let mode = match (room.kind, room.in_match) {
                        (RoomKind::Dramatic, false) => "2V1",
                        (RoomKind::Versus, false) => "1V1",
                        _ => "SPEC",
                    };
                    for (x, text) in [
                        (16, fit(&room.code, 18)), (94, fit(&room.host, 12)), (150, mode.into()),
                        (176, preset.into()),
                        (234, if room.ping_ms == 0 { "--MS".into() } else { format!("{}MS", room.ping_ms) }),
                        (270, format!("{}F", room.rollback_frames.clamp(1, 8))),
                        (294, if room.ping_ms == 0 { "--MS".into() } else { format!("{}MS", room.jitter_ms) }),
                        (326, if room.link == Link::Ethernet { "[ETH]" } else { "[WIFI]" }.into()),
                    ] { fallback_text(&mut frame.argb, x, y, &text, color, 1); }
                }
                if cfg!(windows) && self.selected_room().is_some() {
                    font.text(&mut frame.argb, 3, 21, "CTRL+C COPY ROOM CODE", ink::DIM);
                }
                if self.notice.is_empty() {
                    font.text(&mut frame.argb, 3, 22, &format!("{} ROOMS FOUND", rooms.len()), ink::BLUE);
                }
            }
            LobbyView::Host => {
                self.draw_row(&mut frame, font, 1, 7, "ROOM KIND",
                    if self.host_kind == RoomKind::Dramatic { "DRAMATIC 2V1" } else { "1V1 VERSUS" });
                self.draw_row(&mut frame, font, 2, 10, "VISIBILITY",
                    if self.public { "PUBLIC LAN+LOBBY" } else { "PRIVATE CODE ONLY" });
                self.draw_row(&mut frame, font, 3, 13, "WIRED ONLY", on_off(self.wired_only));
                self.draw_row(&mut frame, font, 4, 16, "OPEN ROOM", "");
                if let Some(code) = &self.room_code {
                    font.text(&mut frame.argb, 5, 19, "ROOM CODE", ink::BLUE);
                    font.text(&mut frame.argb, 20, 19, code, ink::YELLOW);
                    if cfg!(windows) {
                        font.text(&mut frame.argb, 20, 20, "CTRL+C COPY", ink::DIM);
                    }
                }
            }
            LobbyView::Direct => {
                self.draw_row(&mut frame, font, 1, 8, "CODE / IP", "");
                font.text(&mut frame.argb, 4, 10, &self.entry, ink::YELLOW);
                if self.editing {
                    let x = (4 + self.at) * 8;
                    let color = font.pen(ink::YELLOW, 0);
                    for pixel in 0..7 { frame.argb[87 * 384 + x + pixel] = color; }
                }
                let role = match self.join_kind {
                    RoomKind::Spectate => "SPECTATOR",
                    RoomKind::Dramatic => "DRAMATIC ALLY 2",
                    RoomKind::Versus => "PLAYER",
                };
                self.draw_row(&mut frame, font, 2, 13, "JOIN AS", role);
                self.draw_row(&mut frame, font, DIRECT_JOIN_ROW, 15, "JOIN", "");
                font.text(&mut frame.argb, 4, 18, "ROOM CODE OR IPV4:PORT", ink::DIM);
                if cfg!(windows) {
                    font.text(&mut frame.argb, 4, 19, "CTRL+V PASTE", ink::DIM);
                }
            }
        }
        if !self.notice.is_empty() {
            font.text(&mut frame.argb, 3, 22, &fit(&self.notice, 42), ink::BLUE);
        }
        font.text_centred(&mut frame.argb, 23,
            if self.editing { "UP/DN CHAR  L/R PLACE  CONFIRM DONE" }
            else { "UP/DN SELECT  L/R CHANGE  CONFIRM" }, ink::DIM);
        frame
    }

    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        if self.editing {
            match input {
                MenuInput::Left => self.at = self.at.saturating_sub(1),
                MenuInput::Right => self.at = (self.at + 1).min(self.entry.len()),
                MenuInput::Up | MenuInput::Down => {
                    if self.at == self.entry.len() {
                        if self.entry.len() == ENTRY_MAX { return None; }
                        self.entry.push(' ');
                    }
                    let alphabet = NET_ENTRY_CHARS.as_bytes();
                    let old = alphabet.iter().position(|&ch| ch == self.entry.as_bytes()[self.at]).unwrap_or(0);
                    let next = if input == MenuInput::Up { (old + 1) % alphabet.len() }
                        else { (old + alphabet.len() - 1) % alphabet.len() };
                    self.entry.replace_range(self.at..self.at + 1, &char::from(alphabet[next]).to_string());
                }
                MenuInput::Start if self.at > 0 => { self.at -= 1; self.entry.remove(self.at); }
                MenuInput::Confirm => { self.editing = false; self.cursor = DIRECT_JOIN_ROW; }
                MenuInput::Back => self.editing = false,
                _ => return None,
            }
            return Some(Transition::ValueChanged);
        }
        if self.view == LobbyView::Browse && self.cursor == 2
            && matches!(input, MenuInput::Confirm | MenuInput::Start) {
            self.actions.push(OnlineAction::Refresh);
            return Some(Transition::ValueChanged);
        }
        if matches!((self.view, self.cursor), (LobbyView::Browse, 3) | (LobbyView::Direct, 2))
            && matches!(input, MenuInput::Left | MenuInput::Right | MenuInput::Confirm) {
            let choices: &[RoomKind] = if self.view == LobbyView::Direct {
                &[RoomKind::Versus, RoomKind::Spectate, RoomKind::Dramatic]
            } else {
                &[RoomKind::Versus, RoomKind::Spectate]
            };
            let current = choices.iter().position(|&kind| kind == self.join_kind).unwrap_or(0);
            let step = if input == MenuInput::Left { choices.len() - 1 } else { 1 };
            self.join_kind = choices[(current + step) % choices.len()];
            return Some(Transition::ValueChanged);
        }
        if self.view == LobbyView::Browse && self.cursor >= BROWSE_ROOM_START
            && matches!(input, MenuInput::Confirm | MenuInput::Start) {
            let selected = self.selected_room().map(|room| {
                let kind = if self.join_kind == RoomKind::Spectate || room.in_match || room.kind == RoomKind::Spectate {
                    RoomKind::Spectate
                } else {
                    room.kind
                };
                (room.code.clone(), kind)
            });
            if let Some((code, kind)) = selected { return self.join_code(&code, kind); }
        }
        if self.view == LobbyView::Host {
            match (self.cursor, input) {
                (1, MenuInput::Left | MenuInput::Right | MenuInput::Confirm) => {
                    self.host_kind = if self.host_kind == RoomKind::Versus { RoomKind::Dramatic } else { RoomKind::Versus };
                    return Some(Transition::ValueChanged);
                }
                (2, MenuInput::Left | MenuInput::Right | MenuInput::Confirm) => {
                    self.public = !self.public;
                    return Some(Transition::ValueChanged);
                }
                (3, MenuInput::Left | MenuInput::Right | MenuInput::Confirm) => {
                    self.wired_only = !self.wired_only;
                    return Some(Transition::ValueChanged);
                }
                (4, MenuInput::Confirm | MenuInput::Start) => {
                    self.actions.push(OnlineAction::Host {
                        port: self.port, public: self.public, wired_only: self.wired_only, kind: self.host_kind,
                    });
                    self.notice = "OPENING ROOM...".into();
                    return Some(Transition::ValueChanged);
                }
                _ => {}
            }
        }
        if self.view == LobbyView::Direct && self.cursor == 1
            && matches!(input, MenuInput::Confirm) {
            self.editing = true;
            self.at = self.entry.len();
            return Some(Transition::CursorMoved);
        }
        if self.view == LobbyView::Direct && self.cursor == DIRECT_JOIN_ROW
            && matches!(input, MenuInput::Confirm | MenuInput::Start) {
            return self.join();
        }
        match input {
            MenuInput::Up => self.cursor = (self.cursor + self.rows() - 1) % self.rows(),
            MenuInput::Down => self.cursor = (self.cursor + 1) % self.rows(),
            MenuInput::Left | MenuInput::Right | MenuInput::Confirm
                if self.view == LobbyView::Browse && self.cursor == 1 => {
                    self.wired_only = !self.wired_only;
                }
            _ => {}
        }
        if matches!(input, MenuInput::Up | MenuInput::Down) { return Some(Transition::CursorMoved); }
        if self.view == LobbyView::Browse && self.cursor == 1
            && matches!(input, MenuInput::Left | MenuInput::Right | MenuInput::Confirm) {
                return Some(Transition::ValueChanged);
            }
        if self.cursor != 0 && !matches!(input, MenuInput::Back) { return None; }
        self.view = match (self.view, input) {
            (LobbyView::Browse, MenuInput::Right) | (LobbyView::Direct, MenuInput::Left) => LobbyView::Host,
            (LobbyView::Host, MenuInput::Right) | (LobbyView::Browse, MenuInput::Left) => LobbyView::Direct,
            (LobbyView::Direct, MenuInput::Right) | (LobbyView::Host, MenuInput::Left) => LobbyView::Browse,
            (_, MenuInput::Back) => return Some(Transition::Back),
            _ => return None,
        };
        Some(Transition::CursorMoved)
    }
}
