//! The extended tune table at launch (ADR-0009, tickets #195 and #199): the
//! committed themes and the music pack's, converted against the player's
//! sound ROM, as the table the music CPU runs with in overhaul fights.
//! - Faithful 1987 gets no table and its music pack is never opened: under
//!   that preset [`load`] returns nothing without looking for the pack.
//! - The pack is looked for beside the ROM set, then beside each other
//!   folder the ROMs are looked for in, as the art pack is. Without one, or
//!   with a score that does not convert, the log says so and the themes it
//!   would have held are missing.
//! - A stage whose theme is missing plays a 1987 stage theme, and [`apply`]
//!   logs a line naming the stage and the theme. The fight always starts.
//! - A theme that fails to convert is logged and left out; there is no
//!   table at all if even the committed ones fail.
//!
//! The music CPU is one object for the whole session while the preset can
//! change between fights, so the table is not part of its program from
//! launch: the launcher hands it over only for a fight under an overhaul
//! preset (`FrameOutput::tune_table`, `Audio::set_tune_table`) and takes it
//! away after the fight.

use std::path::Path;
use std::sync::Arc;

use sf_audio::music_pack::{find_pack, pack_folders, MusicPack};
use sf_audio::tunes::{self, ExtendedTunes};
use sf_engine::config::RulesetPreset;
use sf_engine::front::GameSession;
use sf_video::background_bank::FALLBACK_THEME;
pub use sf_video::background_bank::RYU_THEME;

use crate::log::Log;

/// The themes the music CPU can play in an overhaul fight.
#[derive(Clone, Debug)]
pub struct Themes {
    /// The table's bytes, as `Audio::set_tune_table` takes them.
    pub table: Arc<Vec<u8>>,
    /// The commands the table answers.
    pub commands: Vec<u8>,
}

impl Themes {
    /// The music CPU's program with these themes behind the ROM `audiocpu`
    /// (sf-02.7k): the image an overhaul fight runs.
    pub fn program(&self, audiocpu: &[u8]) -> Vec<u8> {
        let mut program = audiocpu.to_vec();
        program.resize(tunes::ROM_BYTES, 0xff);
        program.extend_from_slice(&self.table);
        program
    }
}

/// The themes for a session whose preset is `preset`, with the music pack
/// beside the ROM folders `rom_folders`; none under Faithful 1987.
pub fn load(preset: RulesetPreset, audiocpu: &[u8], rom_folders: &[&Path], log: &mut Log) -> Option<Themes> {
    load_from(preset, audiocpu, || find_pack(&pack_folders(rom_folders)), log)
}

/// [`load`] with the pack to open given (for tests). `pack` is called only
/// when the preset is an overhaul one: Faithful 1987 reads nothing from the
/// music pack.
pub fn load_from(
    preset: RulesetPreset,
    audiocpu: &[u8],
    pack: impl FnOnce() -> Result<MusicPack, String>,
    log: &mut Log,
) -> Option<Themes> {
    if preset == RulesetPreset::Faithful1987 {
        return None;
    }
    let mut table = match tunes::committed(audiocpu) {
        Ok(table) => table,
        Err(e) => {
            log.line("themes", &format!("no extended tune table: {e}"));
            return None;
        }
    };
    match pack() {
        Ok(pack) => {
            log.line("themes", &format!("the music pack at {}: {} score(s)", pack.folder.display(), pack.len()));
            for why in &pack.unreadable {
                log.line("themes", &format!("music pack: {why}"));
            }
            for why in tunes::add_pack(&mut table, audiocpu, &pack) {
                log.line("themes", &format!("music pack: {why}"));
            }
        }
        Err(why) => log.line("themes", &format!("no music pack: {why}")),
    }
    Some(themes(table, log))
}

fn themes(table: ExtendedTunes, log: &mut Log) -> Themes {
    let commands = table.commands();
    let names: Vec<String> = commands.iter().map(|c| format!("{c:#04x}")).collect();
    log.line("themes", &format!("the extended tune table: {}", names.join(", ")));
    Themes { table: Arc::new(table.image().to_vec()), commands }
}

/// Tell `session` which extended themes the music CPU can play (none, when
/// there are no themes), and log each stage that falls back to a 1987
/// theme for want of its own.
pub fn apply(themes: Option<&Themes>, session: &mut GameSession, log: &mut Log) {
    let commands = themes.map(|t| t.commands.as_slice()).unwrap_or_default();
    let cause = match themes {
        Some(_) => "which the extended tune table (the committed themes and the music pack) does not hold",
        None => "and there is no extended tune table",
    };
    for missing in session.set_playable_themes(commands) {
        log.line(
            "themes",
            &format!(
                "the stage {} names the theme {:#04x}, {cause}: it plays the 1987 theme {FALLBACK_THEME:#04x}",
                missing.stage, missing.theme
            ),
        );
    }
    if !commands.contains(&RYU_THEME) {
        log.line(
            "themes",
            &format!(
                "Ryu's theme ({RYU_THEME:#04x}), {cause}: stand-in fights play the stage's 1987 theme"
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_audio::music_pack::PackScore;

    /// A stand-in for sf-02.7k with the patches the committed theme names.
    fn rom() -> Vec<u8> {
        let mut rom = vec![0u8; tunes::ROM_BYTES];
        for patch in [17usize, 24, 25] {
            rom[0x6b00 + 0x29 * patch + 6] = 0x3d;
        }
        rom
    }

    fn log() -> Log {
        Log::create(&std::env::temp_dir().join(format!("sf1-port-themes-test-{}.log", std::process::id())))
    }

    fn no_pack() -> Result<MusicPack, String> {
        Err("there is no pack".into())
    }

    /// A pack of one original test score, command `command`.
    fn pack(command: u8) -> MusicPack {
        let text = format!("command {command:#04x}\nunit 10\nchannel 0\n  patch 24\n  forever {{ C4:4 E4:4 }}\n");
        MusicPack { folder: "pack".into(), scores: vec![PackScore { file: "a.score".into(), text }], unreadable: Vec::new() }
    }

    #[test]
    fn faithful_1987_never_loads_the_table_and_never_opens_the_pack() {
        let opened = std::cell::Cell::new(false);
        let themes = load_from(
            RulesetPreset::Faithful1987,
            &rom(),
            || {
                opened.set(true);
                Ok(pack(0x41))
            },
            &mut log(),
        );
        assert!(themes.is_none());
        assert!(!opened.get(), "the music pack is not read");
    }

    #[test]
    fn the_overhaul_presets_load_it_behind_the_rom() {
        let rom = rom();
        for preset in [
            RulesetPreset::DefinitiveOverhaul,
            RulesetPreset::ClassicCompetitive,
            RulesetPreset::PureFootsies1987Plus,
        ] {
            let themes = load_from(preset, &rom, no_pack, &mut log()).expect("themes");
            assert_eq!(themes.commands, [0x40], "{preset:?}: the committed test theme");
            let program = themes.program(&rom);
            assert_eq!(&program[..tunes::ROM_BYTES], &rom[..]);
            assert!(program.len() > tunes::ROM_BYTES, "{preset:?}");
        }
    }

    #[test]
    fn the_packs_scores_join_the_committed_themes() {
        let themes = load_from(RulesetPreset::DefinitiveOverhaul, &rom(), || Ok(pack(0x41)), &mut log()).unwrap();
        assert_eq!(themes.commands, [0x40, 0x41]);
    }

    #[test]
    fn a_score_that_will_not_convert_is_left_out_and_logged() {
        let path = std::env::temp_dir().join(format!("sf1-port-themes-bad-score-{}.log", std::process::id()));
        let mut log = Log::create(&path);
        // 0x40 is the committed theme's command: taken.
        let themes = load_from(RulesetPreset::DefinitiveOverhaul, &rom(), || Ok(pack(0x40)), &mut log).unwrap();
        assert_eq!(themes.commands, [0x40]);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("music pack: a.score:") && text.contains("0x40"), "{text}");
    }

    #[test]
    fn an_unreadable_score_is_logged_and_the_others_load() {
        let path = std::env::temp_dir().join(format!("sf1-port-themes-unreadable-{}.log", std::process::id()));
        let mut log = Log::create(&path);
        let mut p = pack(0x41);
        p.unreadable.push("bad.score: stream did not contain valid UTF-8".into());
        let themes = load_from(RulesetPreset::DefinitiveOverhaul, &rom(), || Ok(p), &mut log).unwrap();
        assert_eq!(themes.commands, [0x40, 0x41]);
        assert!(std::fs::read_to_string(&path).unwrap().contains("music pack: bad.score:"));
    }

    #[test]
    fn a_rom_the_themes_cannot_convert_against_has_no_table() {
        let blank = vec![0u8; tunes::ROM_BYTES];
        assert!(load_from(RulesetPreset::DefinitiveOverhaul, &blank, no_pack, &mut log()).is_none());
    }

    #[test]
    fn apply_logs_when_ryus_theme_is_missing_and_not_when_the_pack_holds_it() {
        use sf_engine::config::MatchConfig;
        use sf_video::romset::Assets;

        let empty_assets = || Assets {
            maincpu: vec![0; 0x60000],
            gfx1: Vec::new(),
            gfx2: Vec::new(),
            gfx3: Vec::new(),
            gfx4: Vec::new(),
            tilerom: Vec::new(),
        };
        let rom = rom();

        let missing_path = std::env::temp_dir().join(format!("sf1-port-themes-no-ryu-{}.log", std::process::id()));
        let mut missing_log = Log::create(&missing_path);
        let without_ryu = load_from(RulesetPreset::DefinitiveOverhaul, &rom, no_pack, &mut missing_log).unwrap();
        let mut session = GameSession::new(empty_assets(), MatchConfig::default());
        apply(Some(&without_ryu), &mut session, &mut missing_log);
        let text = std::fs::read_to_string(&missing_path).unwrap();
        assert!(
            text.contains("Ryu's theme (0x41)") && text.contains("stand-in fights play the stage's 1987 theme"),
            "{text}"
        );

        let present_path = std::env::temp_dir().join(format!("sf1-port-themes-with-ryu-{}.log", std::process::id()));
        let mut present_log = Log::create(&present_path);
        let with_ryu = load_from(RulesetPreset::DefinitiveOverhaul, &rom, || Ok(pack(RYU_THEME)), &mut present_log).unwrap();
        let mut session = GameSession::new(empty_assets(), MatchConfig::default());
        apply(Some(&with_ryu), &mut session, &mut present_log);
        let text = std::fs::read_to_string(&present_path).unwrap();
        assert!(!text.contains("Ryu's theme (0x41)"), "{text}");
    }
}
