//! The Jukebox (ADR-0009, design section 6a, ticket #200 `ST8`): which
//! external track, if any, plays in a real fight, and when the host starts
//! and stops it.
//!
//! - **Keys.** A track is set for the whole game (`global`), for a stage
//!   (by its record's name in the background bank: `retsu`, `test-stage`)
//!   or for a fighter (by name: `Ken`, `Evil Ryu`). Keys are names, not the
//!   1987 stage byte and the twelve fighters, so every stage record and
//!   every fighter, challengers included, can have one with no change here.
//!   Names compare without regard to case.
//! - **Order.** Character, then stage, then global. When both fighters have
//!   a character track, the stage picker's wins (ST2 records the picker:
//!   Player 1 on the first match, the loser after).
//! - **Files.** A track is a file name, read from the `jukebox` folder beside
//!   the ROM set (an absolute path is used as it is). The engine never
//!   touches the disk: the host finds the file, reads its loop points and
//!   plays it, and a file it cannot play leaves the native theme playing.
//! - **Cues.** [`TrackCue::Prepare`] as a fight starts, [`TrackCue::Play`] on the frame the fight latches its stage
//!   theme (the track replaces that tune), [`TrackCue::Stop`] when the
//!   driver would stop its music: the stops `0x00`, `0x35`, `0x3d` and
//!   `0x3f`, and the win jingle `0x37` that replaces the theme. The cue
//!   machine is the session's ([`crate::front::GameSession`]); the rules
//!   live here so they can be read in one place.
//! - **`NATIVE`** (ticket #202, `ST10`). A stage or a fighter can be set to
//!   the native theme itself ([`Pick::Native`]), which counts as a choice at
//!   its level: it beats the levels below it, the global track included, and
//!   the file keeps it as `"retsu" = "NATIVE"`. A stage or fighter with no
//!   entry at all follows the next level down.
//! - **The settings file** holds the tracks in three tables
//!   ([`Jukebox::to_toml`], [`Section`]); the Jukebox screen
//!   ([`crate::front::jukebox`]) writes them.
//! - **Formats.** [`TrackFormats`] is the list of file extensions the
//!   Jukebox plays, which the host supplies (OGG, WAV and FLAC always,
//!   tracker modules only in a build that has the player); the host lists
//!   the `jukebox` folder with it and the screen shows only what it names.

use std::collections::BTreeMap;

/// The music CPU's special commands that stop the music (`0x00`: everything;
/// `0x35`, `0x3d`, `0x3f` at `0x03a4`): `docs/reference/music-stops.md`.
pub const STOPS: [u8; 4] = [0x00, 0x35, 0x3d, 0x3f];

/// The match-win jingle: a tune of the driver's highest priority, which
/// replaces the stage theme where it plays.
pub const WIN_JINGLE: u8 = 0x37;

/// The `[jukebox]` table's tables in the settings file.
pub const GLOBAL_TABLE: &str = "jukebox";
pub const STAGE_TABLE: &str = "jukebox.stages";
pub const FIGHTER_TABLE: &str = "jukebox.fighters";

/// The word the settings file and the Jukebox screen use for the native
/// theme (no case, no padding). A file really named `NATIVE` is written
/// `./NATIVE`.
pub const NATIVE: &str = "NATIVE";

/// The file extensions of the audio formats every build plays, lower case,
/// without the dot. Tracker modules are not here: the host adds them (and
/// so the Jukebox screen offers them) only when it was built with a
/// tracker player.
pub const TRACK_EXTENSIONS: [&str; 3] = ["ogg", "wav", "flac"];

/// The file extensions the host plays: the one list the host's folder
/// listing and the Jukebox screen filter by. The default is the audio
/// formats of [`TRACK_EXTENSIONS`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackFormats {
    extensions: Vec<String>,
}

impl Default for TrackFormats {
    fn default() -> Self {
        TrackFormats::new(TRACK_EXTENSIONS.iter().map(|e| (*e).to_owned()))
    }
}

impl TrackFormats {
    /// The formats named by `extensions` (no dot; case is not regarded).
    pub fn new(extensions: impl IntoIterator<Item = String>) -> Self {
        let mut extensions: Vec<String> = extensions.into_iter().map(|e| e.to_ascii_lowercase()).collect();
        extensions.sort();
        extensions.dedup();
        TrackFormats { extensions }
    }

    /// Whether `file` (a name in the `jukebox` folder) is of a format the
    /// host plays, by its extension and without regard to case. A name with a
    /// leading dot is a hidden or resource file, never a track.
    pub fn accepts(&self, file: &str) -> bool {
        let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
        !name.starts_with('.')
            && name.rsplit_once('.').is_some_and(|(stem, ext)| !stem.is_empty() && self.extensions.iter().any(|e| ext.eq_ignore_ascii_case(e)))
    }

    /// Whether the list has formats beyond [`TRACK_EXTENSIONS`]: tracker
    /// modules.
    pub fn has_trackers(&self) -> bool {
        self.extensions.iter().any(|e| !TRACK_EXTENSIONS.contains(&e.as_str()))
    }
}

/// What a stage or a fighter is set to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// The native theme, even where a lower level has a track.
    Native,
    /// A file, as the settings name it.
    File(String),
}

impl Pick {
    /// The pick the settings file's value `text` says: `NATIVE` (no case,
    /// no padding) or a file.
    pub fn parse(text: &str) -> Pick {
        if text.trim().eq_ignore_ascii_case(NATIVE) {
            Pick::Native
        } else {
            Pick::File(text.to_owned())
        }
    }

    /// The file, if this is one.
    pub fn file(&self) -> Option<&str> {
        match self {
            Pick::File(file) => Some(file),
            Pick::Native => None,
        }
    }
}

/// Whether the driver stops its music, or ends it with a jingle, on `latch`.
pub fn ends_music(latch: u8) -> bool {
    STOPS.contains(&latch) || latch == WIN_JINGLE
}

/// What a key is compared by: its text, trimmed and in lower case.
fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Which of the three levels a track was set at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrackSource {
    /// A fighter's track (the key as set).
    Character(String),
    /// A stage's track (the key as set).
    Stage(String),
    Global,
}

impl TrackSource {
    /// For the log: `the track set for Ken`.
    pub fn describe(&self) -> String {
        match self {
            TrackSource::Character(name) => format!("the track set for the fighter {name}"),
            TrackSource::Stage(name) => format!("the track set for the stage {name}"),
            TrackSource::Global => "the global track".to_owned(),
        }
    }
}

/// The track a fight plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The file as the settings name it.
    pub file: String,
    pub source: TrackSource,
}

/// What the host does with the track, on the frame it comes with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrackCue {
    /// A fight has started and this is the track its theme will start (the
    /// stage or fighters it is for are known): the host may decode the file
    /// now, so the theme's frame does not wait for it. Nothing is heard.
    Prepare(Resolved),
    /// Start the track from its beginning (again, if it was playing: the
    /// round's theme starts over each round). If the host cannot play the
    /// file it says so and leaves the native theme playing.
    Play(Resolved),
    /// The driver would stop its music here: stop the track.
    Stop,
}

/// The cue machine: what the host is told on each frame of a session, from
/// that frame's sound latch (the session's, one per front-end frame).
#[derive(Clone, Debug)]
pub struct Cues {
    /// The last frame's latch, so a command held for several frames is one
    /// command (the driver ignores a latch equal to the last it took).
    last: u8,
    /// A track was started and not yet stopped.
    on: bool,
    /// The track last announced for the running fight.
    prepared: Option<Resolved>,
}

impl Default for Cues {
    fn default() -> Self {
        Cues { last: IDLE, on: false, prepared: None }
    }
}

/// The latch's idle value, between commands.
const IDLE: u8 = 0xff;
/// The stop-all command, the first a fight sends.
const STOP_ALL: u8 = 0x00;

impl Cues {
    /// The cue for a frame whose latch is `latch`. `fighting` says a fight
    /// under an overhaul preset is up (never in the attract, the cabinet or
    /// a Faithful 1987 fight); `track(None)` is the track the fight will
    /// have now and `track(Some(command))` the track for `command` if it is
    /// the fight's stage theme.
    ///
    /// A new command (not idle, not the last frame's) that is the stage
    /// theme starts the track; one that ends the driver's music stops it;
    /// so does leaving the fight. Whenever the track the fight will have
    /// changes (a fight starting, Arcade's next opponent) it is announced
    /// ahead, unless the theme is on this very frame.
    pub fn step(&mut self, latch: u8, fighting: bool, track: impl Fn(Option<u8>) -> Option<Resolved>) -> Option<TrackCue> {
        let new = latch != IDLE && latch != self.last;
        self.last = latch;
        if !fighting || (new && ends_music(latch)) {
            // A fight starts with the stop-all command: its track is to be
            // announced afresh (the stage or fighters may have changed).
            if !fighting || latch == STOP_ALL {
                self.prepared = None;
            }
            return std::mem::take(&mut self.on).then_some(TrackCue::Stop);
        }
        let coming = track(None);
        if new {
            if let Some(play) = track(Some(latch)) {
                self.prepared = coming;
                self.on = true;
                return Some(TrackCue::Play(play));
            }
        }
        if coming != self.prepared {
            self.prepared = coming.clone();
            return coming.map(TrackCue::Prepare);
        }
        None
    }

    /// Whether a track was started and not stopped.
    pub fn playing(&self) -> bool {
        self.on
    }
}

/// The tracks a player set.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Jukebox {
    global: Option<String>,
    stages: BTreeMap<String, Pick>,
    fighters: BTreeMap<String, Pick>,
}

impl Jukebox {
    pub fn is_empty(&self) -> bool {
        self.global.is_none() && self.stages.is_empty() && self.fighters.is_empty()
    }

    pub fn global(&self) -> Option<&str> {
        self.global.as_deref()
    }

    /// The file set for a stage; `None` for a stage on `NATIVE` or with no
    /// entry ([`Self::stage_pick`] tells them apart).
    pub fn stage(&self, stage: &str) -> Option<&str> {
        self.stage_pick(stage).and_then(Pick::file)
    }

    /// The file set for a fighter; `None` for one on `NATIVE` or with no
    /// entry ([`Self::fighter_pick`] tells them apart).
    pub fn fighter(&self, fighter: &str) -> Option<&str> {
        self.fighter_pick(fighter).and_then(Pick::file)
    }

    /// What a stage is set to, if it has an entry.
    pub fn stage_pick(&self, stage: &str) -> Option<&Pick> {
        self.stages.get(&key(stage))
    }

    /// What a fighter is set to, if it has an entry.
    pub fn fighter_pick(&self, fighter: &str) -> Option<&Pick> {
        self.fighters.get(&key(fighter))
    }

    /// Set the track for every fight that has no stage or fighter entry.
    pub fn set_global(&mut self, file: &str) {
        self.global = Some(file.to_owned());
    }

    pub fn set_stage(&mut self, stage: &str, file: &str) {
        self.stages.insert(key(stage), Pick::File(file.to_owned()));
    }

    pub fn set_fighter(&mut self, fighter: &str, file: &str) {
        self.fighters.insert(key(fighter), Pick::File(file.to_owned()));
    }

    /// Set a stage to the native theme, over the global track.
    pub fn set_stage_native(&mut self, stage: &str) {
        self.stages.insert(key(stage), Pick::Native);
    }

    /// Set a fighter to the native theme, over the stage's and the global
    /// track.
    pub fn set_fighter_native(&mut self, fighter: &str) {
        self.fighters.insert(key(fighter), Pick::Native);
    }

    pub fn clear_global(&mut self) {
        self.global = None;
    }

    /// Remove a stage's entry: it follows the global track again.
    pub fn clear_stage(&mut self, stage: &str) {
        self.stages.remove(&key(stage));
    }

    /// Remove a fighter's entry: it follows the stage's and the global track
    /// again.
    pub fn clear_fighter(&mut self, fighter: &str) {
        self.fighters.remove(&key(fighter));
    }

    /// The track for a fight on `stage` between `picker`'s fighter (the side
    /// at the controls of the stage screen) and `other`'s: a fighter's
    /// entry first, the picker's before the other's, then the stage's, then
    /// the global track. The first entry found decides, and `NATIVE` is an
    /// entry: it plays the native theme whatever the levels below hold.
    /// `None` plays the native theme.
    pub fn resolve(&self, stage: Option<&str>, picker: Option<&str>, other: Option<&str>) -> Option<Resolved> {
        for name in [picker, other].into_iter().flatten() {
            match self.fighter_pick(name) {
                Some(Pick::File(file)) => return Some(Resolved { file: file.clone(), source: TrackSource::Character(name.to_owned()) }),
                Some(Pick::Native) => return None,
                None => {}
            }
        }
        if let Some(name) = stage {
            match self.stage_pick(name) {
                Some(Pick::File(file)) => return Some(Resolved { file: file.clone(), source: TrackSource::Stage(name.to_owned()) }),
                Some(Pick::Native) => return None,
                None => {}
            }
        }
        self.global.as_ref().map(|file| Resolved { file: file.clone(), source: TrackSource::Global })
    }

    /// The tables of the settings file, empty tables left out (so a player
    /// who never sets a track has a file with none).
    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        if let Some(file) = &self.global {
            out.push_str(&format!("\n[{GLOBAL_TABLE}]\nglobal = {}\n", quoted(file)));
        }
        for (table, tracks) in [(STAGE_TABLE, &self.stages), (FIGHTER_TABLE, &self.fighters)] {
            if tracks.is_empty() {
                continue;
            }
            out.push_str(&format!("\n[{table}]\n"));
            for (name, pick) in tracks {
                out.push_str(&format!("{} = {}\n", quoted(name), quoted(pick.file().unwrap_or(NATIVE))));
            }
        }
        out
    }

    /// One `name = "file"` line of table `section`. A key set twice (names
    /// that differ only in case are the same key) is not an error: the later
    /// line wins, and the warning says so.
    pub fn read_line(&mut self, section: Section, name: &str, file: &str) -> Result<Option<String>, String> {
        if file.trim().is_empty() {
            return Err(format!("the track for '{name}' names no file"));
        }
        let pick = Pick::parse(file);
        let (what, before) = match section {
            Section::Global => match name {
                // `NATIVE` is no track: nothing is below the global level to override.
                "global" => ("global", std::mem::replace(&mut self.global, pick.file().map(str::to_owned))),
                _ => return Err(format!("unknown key '{name}' in [{GLOBAL_TABLE}]")),
            },
            Section::Stages if key(name).is_empty() => return Err("a stage track names no stage".to_owned()),
            Section::Stages => ("stage", self.stages.insert(key(name), pick).map(|p| p.file().unwrap_or(NATIVE).to_owned())),
            Section::Fighters if key(name).is_empty() => return Err("a fighter track names no fighter".to_owned()),
            Section::Fighters => ("fighter", self.fighters.insert(key(name), pick).map(|p| p.file().unwrap_or(NATIVE).to_owned())),
        };
        Ok(before.map(|old| format!("the {what} '{name}' is set twice (keys that differ only in case are one key): '{file}' replaces '{old}'")))
    }

    /// The stage and fighter names set that match none of `stages` and
    /// `fighters` (the stage records and the fighters the game has), each as
    /// a line for the log. Names compare without regard to case.
    pub fn unmatched<'a>(&self, stages: impl IntoIterator<Item = &'a str>, fighters: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        let stages: Vec<String> = stages.into_iter().map(key).collect();
        let fighters: Vec<String> = fighters.into_iter().map(key).collect();
        let mut out = Vec::new();
        out.extend(self.stages.keys().filter(|k| !stages.contains(k)).map(|k| format!("the track set for the stage '{k}' matches no stage record, so no fight uses it")));
        out.extend(self.fighters.keys().filter(|k| !fighters.contains(k)).map(|k| format!("the track set for the fighter '{k}' matches no fighter, so no fight uses it")));
        out
    }
}

/// The settings file's Jukebox tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Global,
    Stages,
    Fighters,
}

impl Section {
    /// The table a `[header]` line names, if it is one of the Jukebox's.
    pub fn from_header(header: &str) -> Option<Section> {
        match header.trim() {
            GLOBAL_TABLE => Some(Section::Global),
            STAGE_TABLE => Some(Section::Stages),
            FIGHTER_TABLE => Some(Section::Fighters),
            _ => None,
        }
    }
}

/// `text` as a quoted string, `\` and `"` escaped.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if c == '\\' || c == '"' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracks() -> Jukebox {
        let mut j = Jukebox::default();
        j.set_global("any.ogg");
        j.set_stage("Retsu", "stage.ogg");
        j.set_fighter("Ken", "ken.flac");
        j.set_fighter("Evil Ryu", "evil.wav");
        j
    }

    #[test]
    fn character_beats_stage_beats_global() {
        let j = tracks();
        let from = |r: Option<Resolved>| r.map(|r| r.file);
        assert_eq!(from(j.resolve(Some("retsu"), Some("Ken"), Some("Ryu"))).as_deref(), Some("ken.flac"));
        assert_eq!(from(j.resolve(Some("retsu"), Some("Ryu"), Some("Joe"))).as_deref(), Some("stage.ogg"));
        assert_eq!(from(j.resolve(Some("geki"), Some("Ryu"), Some("Joe"))).as_deref(), Some("any.ogg"));
        assert_eq!(from(j.resolve(None, None, None)).as_deref(), Some("any.ogg"));
        assert_eq!(Jukebox::default().resolve(Some("retsu"), Some("Ken"), None), None);
    }

    #[test]
    fn native_is_an_entry_that_beats_the_levels_below_it() {
        let from = |r: Option<Resolved>| r.map(|r| r.file);
        let mut j = tracks();
        // A stage on NATIVE plays the native theme over the global track.
        j.set_stage_native("geki");
        assert_eq!(from(j.resolve(Some("geki"), Some("Ryu"), Some("Joe"))), None);
        assert_eq!(from(j.resolve(Some("mike"), Some("Ryu"), Some("Joe"))).as_deref(), Some("any.ogg"), "other stages follow the global track");
        // A fighter's track still beats the stage's NATIVE: character first.
        assert_eq!(from(j.resolve(Some("geki"), Some("Ken"), Some("Joe"))).as_deref(), Some("ken.flac"));
        // A fighter on NATIVE beats the stage's track and the global one.
        j.set_fighter_native("Joe");
        assert_eq!(from(j.resolve(Some("retsu"), Some("Ryu"), Some("Joe"))), None);
        assert_eq!(from(j.resolve(Some("mike"), Some("Joe"), None)), None);
        // Of two fighters' entries the picker's decides, NATIVE or a file.
        assert_eq!(from(j.resolve(Some("mike"), Some("Joe"), Some("Ken"))), None);
        assert_eq!(from(j.resolve(Some("mike"), Some("Ken"), Some("Joe"))).as_deref(), Some("ken.flac"));
        // Clearing the entry hands the fight back to the next level down.
        j.clear_fighter("Joe");
        j.clear_stage("geki");
        assert_eq!(from(j.resolve(Some("geki"), Some("Ryu"), Some("Joe"))).as_deref(), Some("any.ogg"));
        // NATIVE has no file: the file accessors say nothing, the picks tell it apart.
        j.set_stage_native("geki");
        assert_eq!((j.stage("geki"), j.stage_pick("geki")), (None, Some(&Pick::Native)));
        assert_eq!(j.stage_pick("mike"), None);
    }

    #[test]
    fn native_is_kept_in_the_settings_text() {
        let mut j = Jukebox::default();
        j.set_global("all.ogg");
        j.set_stage_native("Geki");
        j.set_fighter_native("Ken");
        j.set_fighter("Ryu", "ryu.ogg");
        let text = j.to_toml();
        assert!(text.contains("\"geki\" = \"NATIVE\"") && text.contains("\"ken\" = \"NATIVE\""), "{text}");
        let mut back = Jukebox::default();
        assert_eq!(back.read_line(Section::Global, "global", "all.ogg"), Ok(None));
        assert_eq!(back.read_line(Section::Stages, "geki", " native "), Ok(None), "no case, no padding");
        assert_eq!(back.read_line(Section::Fighters, "ken", "NATIVE"), Ok(None));
        assert_eq!(back.read_line(Section::Fighters, "ryu", "ryu.ogg"), Ok(None));
        assert_eq!(back, j);
        // A key set twice says what it replaces, NATIVE included.
        let warning = back.read_line(Section::Stages, "Geki", "g.ogg").unwrap().expect("a warning");
        assert!(warning.contains("NATIVE") && warning.contains("g.ogg"), "{warning}");
        // The global level has nothing under it to override: NATIVE there is no track.
        back.read_line(Section::Global, "global", "native").unwrap();
        assert_eq!(back.global(), None);
        // A file really named NATIVE is written with a path.
        assert_eq!(Pick::parse("./NATIVE"), Pick::File("./NATIVE".into()));
    }

    #[test]
    fn only_the_formats_the_host_plays_are_tracks() {
        let formats = TrackFormats::default();
        for file in ["a.ogg", "B.OGG", "mix one.wav", "x.Flac", "sub/y.ogg", r"sub\y.wav", "a.b.flac"] {
            assert!(formats.accepts(file), "{file}");
        }
        for file in ["a.mp3", "a.ogg.txt", "ogg", ".ogg", "a.", "", "notes.txt", ".hidden.ogg", "sub/.x.ogg", "cover.png", "song.xm", "a.MOD"] {
            assert!(!formats.accepts(file), "{file}");
        }
        assert!(!formats.has_trackers());
        assert_eq!(TRACK_EXTENSIONS, ["ogg", "wav", "flac"], "the audio formats every build plays");
    }

    #[test]
    fn a_build_with_a_tracker_player_adds_the_modules_to_the_formats() {
        let formats = TrackFormats::new(["ogg", "WAV", "flac", "xm", "mod", "it"].map(String::from));
        for file in ["a.ogg", "song.XM", "tune.mod", "x.it", "sub/y.IT"] {
            assert!(formats.accepts(file), "{file}");
        }
        for file in ["a.s3m", "a.mp3", ".song.xm"] {
            assert!(!formats.accepts(file), "{file}");
        }
        assert!(formats.has_trackers());
    }

    #[test]
    fn the_picker_breaks_a_tie_between_two_character_tracks() {
        let j = tracks();
        let of = |picker, other| j.resolve(Some("retsu"), Some(picker), Some(other)).unwrap();
        assert_eq!(of("Ken", "Evil Ryu").file, "ken.flac");
        assert_eq!(of("Evil Ryu", "Ken").file, "evil.wav");
        // With one character track, it is that one whoever picked.
        assert_eq!(of("Ryu", "Evil Ryu").file, "evil.wav");
        assert_eq!(of("Evil Ryu", "Ryu").file, "evil.wav");
    }

    #[test]
    fn keys_compare_without_regard_to_case_and_name_what_set_them() {
        let j = tracks();
        let r = j.resolve(None, Some("EVIL RYU"), None).unwrap();
        assert_eq!(r.source, TrackSource::Character("EVIL RYU".into()));
        assert_eq!(j.stage("RETSU"), Some("stage.ogg"));
    }

    fn global() -> Resolved {
        Resolved { file: "a.ogg".into(), source: TrackSource::Global }
    }

    /// The cue for each of `frames` (latch, fighting) with a global track
    /// whose stage theme is `0x29`.
    fn cues(frames: &[(u8, bool)]) -> Vec<Option<TrackCue>> {
        let mut machine = Cues::default();
        let track = |latch: Option<u8>| latch.is_none_or(|l| l == 0x29).then(global);
        frames.iter().map(|&(latch, fighting)| machine.step(latch, fighting, track)).collect()
    }

    #[test]
    fn the_theme_starts_the_track_and_each_stop_or_the_jingle_ends_it() {
        let play = Some(TrackCue::Play(global()));
        let prepare = Some(TrackCue::Prepare(global()));
        let stop = Some(TrackCue::Stop);
        // Announced on the fight's first frame, started by the theme, and a
        // command held for several frames is one command.
        assert_eq!(cues(&[(0xff, true), (0x29, true), (0x29, true), (0xff, true)]), [prepare.clone(), play.clone(), None, None]);
        // The theme on the fight's very first frame is not swallowed by the announcement.
        assert_eq!(cues(&[(0x29, true), (0xff, true)]), [play.clone(), None]);
        // The theme is not any command: another tune does not start it.
        assert_eq!(cues(&[(0xff, true), (0x33, true), (0x40, true)]), [prepare.clone(), None, None]);
        // Every stop ends it, once; so does the win jingle where it replaces the theme.
        for end in [0x35, 0x3d, 0x3f, 0x37] {
            assert_eq!(cues(&[(0xff, true), (0x29, true), (end, true), (end, true), (0xff, true)]), [prepare.clone(), play.clone(), stop.clone(), None, None], "{end:#04x}");
        }
        // The stop-all command a fight starts with stops it and has the next fight announced afresh.
        assert_eq!(cues(&[(0xff, true), (0x29, true), (0x00, true), (0xff, true)]), [prepare.clone(), play.clone(), stop.clone(), prepare.clone()]);
        // A stop with nothing playing says nothing; the next round's theme plays again.
        assert_eq!(cues(&[(0xff, true), (0x3f, true), (0x29, true), (0x35, true), (0xff, true), (0x29, true)]), [prepare.clone(), None, play.clone(), stop.clone(), None, play.clone()]);
        // Leaving the fight stops it, and outside a fight nothing is said.
        assert_eq!(cues(&[(0x29, false), (0xff, true), (0x29, true), (0xff, false), (0x29, false)]), [None, prepare, play, stop, None]);
    }

    #[test]
    fn a_new_track_for_the_fight_is_announced_again() {
        // Arcade's next opponent has a track of his own: announced when it
        // changes, once.
        let mut machine = Cues::default();
        let step = |m: &mut Cues, latch, ladder: u8| {
            m.step(latch, true, |l| {
                let file = if ladder == 0 { "retsu.ogg" } else { "geki.ogg" };
                l.is_none_or(|c| c == 0x29).then(|| Resolved { file: file.into(), source: TrackSource::Global })
            })
        };
        let named = |c: Option<TrackCue>| match c {
            Some(TrackCue::Prepare(r)) => format!("prepare {}", r.file),
            Some(TrackCue::Play(r)) => format!("play {}", r.file),
            Some(TrackCue::Stop) => "stop".to_owned(),
            None => "-".to_owned(),
        };
        let got: Vec<String> = [(0xff, 0), (0xff, 0), (0x29, 0), (0x35, 0), (0xff, 1), (0xff, 1), (0x29, 1)].into_iter().map(|(l, k)| named(step(&mut machine, l, k))).collect();
        assert_eq!(got, ["prepare retsu.ogg", "-", "play retsu.ogg", "stop", "prepare geki.ogg", "-", "play geki.ogg"]);
    }

    #[test]
    fn a_key_set_twice_warns_and_names_that_match_nothing_are_listed() {
        let mut j = Jukebox::default();
        assert_eq!(j.read_line(Section::Stages, "Retsu", "a.ogg"), Ok(None));
        let warning = j.read_line(Section::Stages, "RETSU", "b.ogg").unwrap().expect("a warning");
        assert!(warning.contains("RETSU") && warning.contains("b.ogg") && warning.contains("a.ogg"), "{warning}");
        assert_eq!(j.stage("retsu"), Some("b.ogg"), "the later line wins");
        j.set_stage("nowhere", "c.ogg");
        j.set_fighter("Nobody", "d.ogg");
        j.set_fighter("ken", "e.ogg");
        let lines = j.unmatched(["Retsu", "Geki"], ["Ryu", "Ken"]);
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].contains("nowhere") && lines[1].contains("nobody"), "{lines:?}");
    }

    #[test]
    fn the_stops_and_the_jingle_end_music() {
        for latch in [0x00, 0x35, 0x3d, 0x3f, 0x37] {
            assert!(ends_music(latch), "{latch:#04x}");
        }
        for latch in [0xff, 0x29, 0x33, 0x40, 0x0f] {
            assert!(!ends_music(latch), "{latch:#04x}");
        }
    }

    #[test]
    fn the_tables_round_trip_through_the_settings_text() {
        let mut j = tracks();
        j.set_stage("test-stage", "a \"quoted\" \\ name.ogg");
        let mut back = Jukebox::default();
        let mut section = None;
        for line in j.to_toml().lines().map(str::trim).filter(|l| !l.is_empty()) {
            if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = Section::from_header(header);
                assert!(section.is_some(), "{header}");
                continue;
            }
            let (name, file) = line.split_once(" = ").unwrap();
            let unquote = |s: &str| match s.strip_prefix('"') {
                Some(rest) => rest.strip_suffix('"').unwrap().replace("\\\"", "\"").replace("\\\\", "\\"),
                None => s.to_owned(),
            };
            assert_eq!(back.read_line(section.unwrap(), &unquote(name), &unquote(file)).unwrap(), None);
        }
        assert_eq!(back, j);
        assert_eq!(Jukebox::default().to_toml(), "", "no tracks, no tables");
    }
}
