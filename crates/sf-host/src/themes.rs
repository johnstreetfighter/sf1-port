//! Load the local native score library and the legacy tune table. Faithful
//! 1987 never opens the music pack. Overhaul sessions bind semantic match
//! cues to their scores before pushing the timed latch to the music CPU.
//! The legacy table continues to serve the title tune and raw callers.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sf_audio::music_pack::{find_pack, pack_folders, MusicPack, MUSIC_FOLDER};
use sf_audio::score::{self, CompiledScore, NativeScore};
use sf_audio::tunes::{self, ExtendedTunes};
use sf_engine::config::RulesetPreset;
use sf_engine::front::{sound, GameSession};
use sf_engine::theme::{ThemeCue, ThemeId};
use sf_video::background_bank::FALLBACK_THEME;
pub use sf_video::background_bank::RYU_THEME;

use crate::log::Log;

/// The themes the music CPU can play under an overhaul preset.
#[derive(Clone, Debug)]
pub struct Themes {
    /// The table's bytes, as `Audio::set_tune_table` takes them.
    pub table: Arc<Vec<u8>>,
    /// Commands available from the legacy table or the separate library.
    pub commands: Vec<u8>,
    scores: BTreeMap<ThemeId, Arc<NativeScore>>,
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
    load_with_fallbacks(preset, audiocpu, rom_folders, &[], log)
}

/// Read the ROM-adjacent pack first. Fill in missing character scores from
/// the first repository asset root that has a music pack.
pub fn load_with_fallbacks(
    preset: RulesetPreset,
    audiocpu: &[u8],
    rom_folders: &[&Path],
    asset_roots: &[PathBuf],
    log: &mut Log,
) -> Option<Themes> {
    load_packs(preset, audiocpu, || {
        let mut packs = vec![find_pack(&pack_folders(rom_folders))];
        if let Some(folder) = asset_roots.iter().map(|root| root.join(MUSIC_FOLDER)).find(|folder| folder.is_dir()) {
            packs.push(MusicPack::open(&folder));
        }
        packs
    }, log)
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
    load_packs(preset, audiocpu, || [pack()], log)
}

fn load_packs<Packs: IntoIterator<Item = Result<MusicPack, String>>>(
    preset: RulesetPreset,
    audiocpu: &[u8],
    packs: impl FnOnce() -> Packs,
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
    let mut scores = BTreeMap::new();
    for text in tunes::COMMITTED {
        match score::compile(text, audiocpu) {
            Ok(compiled) if compiled.command == ThemeId::Test.command() => {
                scores.insert(ThemeId::Test, compiled.score);
            }
            Ok(_) => {}
            Err(error) => log.line("themes", &format!("committed score: {error}")),
        }
    }
    for pack in packs() {
        match pack {
            Ok(pack) => {
                log.line("themes", &format!("the music pack at {}: {} score(s)", pack.folder.display(), pack.len()));
                add_pack(&mut table, &mut scores, audiocpu, pack, log);
            }
            Err(why) => log.line("themes", &format!("no music pack: {why}")),
        }
    }

    let mut commands = table.commands();
    commands.extend(scores.keys().map(|id| id.command()));
    commands.sort_unstable();
    commands.dedup();
    let legacy: Vec<_> = table.commands().into_iter().map(|c| format!("{c:#04x}")).collect();
    log.line("themes", &format!("the extended tune table: {}", legacy.join(", ")));
    log.line("themes", &format!("the separate score library: {} score(s)", scores.len()));
    Some(Themes { table: Arc::new(table.image().to_vec()), commands, scores })
}

fn registered_identity(score: &CompiledScore) -> Result<Option<ThemeId>, String> {
    let command = score.command;
    let id = ThemeId::from_command(command);
    if command == ThemeId::Test.command() {
        return Err(format!("{command:#04x} is reserved for the original test score"));
    }
    if let Some(slug) = &score.theme {
        let expected = ThemeId::from_slug(slug).filter(|id| id.is_extended())
            .ok_or_else(|| format!("unknown extended theme identity {slug:?}"))?;
        if id != Some(expected) {
            return Err(format!("theme {slug:?} uses command {:#04x}, not {command:#04x}", expected.command()));
        }
    }
    Ok(id)
}

fn add_pack(
    table: &mut ExtendedTunes,
    library: &mut BTreeMap<ThemeId, Arc<NativeScore>>,
    rom: &[u8],
    pack: MusicPack,
    log: &mut Log,
) {
    for why in pack.unreadable {
        log.line("themes", &format!("music pack: {why}"));
    }
    let mut compiled = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    for source in pack.scores {
        let result = score::compile(&source.text, rom).map_err(|e| e.to_string())
            .and_then(|score| registered_identity(&score).map(|id| (id, score)));
        match result {
            Ok((id, score)) => {
                if compiled.contains_key(&score.command) {
                    ambiguous.insert(score.command);
                } else {
                    compiled.insert(score.command, (source, id, score));
                    continue;
                }
                log.line("themes", &format!("music pack: {}: duplicate command {:#04x}", source.file, score.command));
            }
            Err(why) => log.line("themes", &format!("music pack: {}: {why}", source.file)),
        }
    }
    // Reserve the old window for the title and existing raw-command callers
    // before mirroring named character scores for legacy tools.
    let (legacy, named): (Vec<_>, Vec<_>) = compiled.into_values().partition(|(_, id, _)| id.is_none());
    for (source, id, score) in legacy.into_iter().chain(named) {
        if ambiguous.contains(&score.command) {
            log.line("themes", &format!("music pack: {}: ambiguous command {:#04x}", source.file, score.command));
            continue;
        }
        if let Some(id) = id {
            let Entry::Vacant(slot) = library.entry(id) else { continue };
            slot.insert(score.score);
        }
        let result = score::convert(&source.text, rom).map_err(|e| e.to_string())
            .and_then(|score| score.add_to(table).map_err(|e| e.to_string()));
        if let Err(error) = result {
            if id.is_none() {
                log.line("themes", &format!("music pack: {}: {error}", source.file));
            }
        }
    }
}

/// An output latch and the exact in-memory score bound to it, if any.
pub struct ThemeFrame {
    pub latch: u8,
    pub score: Option<Arc<NativeScore>>,
}

/// Resolve timed semantic cues without changing match selection or rotation.
#[derive(Default)]
pub struct ThemeDelivery {
    missing: BTreeSet<(ThemeId, u8)>,
}

impl ThemeDelivery {
    pub fn resolve(&mut self, themes: Option<&Themes>, cue: ThemeCue, log: &mut Log) -> ThemeFrame {
        let command = cue.id.command();
        if !cue.id.is_extended() {
            return ThemeFrame { latch: command, score: None };
        }
        if let Some(score) = themes.and_then(|themes| themes.scores.get(&cue.id)) {
            return ThemeFrame { latch: command, score: Some(score.clone()) };
        }
        if self.missing.insert((cue.id, cue.fallback)) {
            log.line("themes", &format!(
                "no usable score for {} ({command:#04x}); playing the stage's 1987 theme {:#04x}",
                cue.id.slug(), cue.fallback
            ));
        }
        ThemeFrame { latch: cue.fallback, score: None }
    }
}

/// Tell `session` which extended themes the music CPU can play (none, when
/// there are no themes). Log when the title or a stage's score is missing.
pub fn apply(themes: Option<&Themes>, session: &mut GameSession, log: &mut Log) {
    let commands = themes.map(|t| t.commands.as_slice()).unwrap_or_default();
    let has_title = commands.contains(&sound::TITLE_THEME);
    session.set_title_theme_available(has_title);
    if !has_title {
        log.line("themes", &format!("no title theme ({:#04x}) in the music pack", sound::TITLE_THEME));
    }
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
    fn registered_cues_bind_a_score_and_fall_back_to_the_actual_stage() {
        use sf_engine::theme::{ThemeCue, ThemeId};

        let themes = load_from(RulesetPreset::DefinitiveOverhaul, &rom(), || Ok(pack(0x41)), &mut log()).unwrap();
        let mut delivery = ThemeDelivery::default();
        let ryu = delivery.resolve(Some(&themes), ThemeCue { id: ThemeId::Ryu, fallback: 0x31 }, &mut log());
        assert_eq!(ryu.latch, 0x41);
        assert!(ryu.score.is_some());

        let evil = delivery.resolve(Some(&themes), ThemeCue { id: ThemeId::EvilRyu, fallback: 0x31 }, &mut log());
        assert_eq!(evil.latch, 0x31, "a missing Evil Ryu score cannot play Ryu");
        assert!(evil.score.is_none());

        let gen = delivery.resolve(None, ThemeCue { id: ThemeId::Gen, fallback: 0x31 }, &mut log());
        assert_eq!(gen.latch, 0x2f, "the selected native identity owns its ROM theme");
        assert!(gen.score.is_none());
    }

    #[test]
    fn the_registry_rejects_mismatched_named_scores_and_duplicate_commands() {
        use sf_audio::music_pack::PackScore;

        let mut entries = pack(0x41);
        entries.scores[0].text.insert_str(0, "theme evil-ryu\n");
        entries.scores.push(PackScore { file: "b.score".into(), text: pack(0x52).scores[0].text.clone() });
        entries.scores.push(PackScore { file: "c.score".into(), text: pack(0x52).scores[0].text.clone() });
        let themes = load_from(RulesetPreset::DefinitiveOverhaul, &rom(), || Ok(entries), &mut log()).unwrap();
        assert_eq!(themes.commands, [0x40]);
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
