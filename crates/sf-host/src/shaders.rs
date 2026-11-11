//! The look catalogue (spec #203, ADR-0010): the looks `CRT LOOK` lists,
//! `OFF` first, then the built-in looks, each a RetroArch `.slangp` preset
//! in the shaders folder beside the executable, then the drop-in looks
//! (ticket #210): every `.slangp` under `%APPDATA%\sf1-port\shaders\`,
//! named by its file; and, between the two, the player's saved looks
//! (ticket #213): `.slangp` files in the drop-ins' `saved` folder, written
//! by the tuning screen's `SAVE AS` ([`save_look`]). The engine sees only
//! the ids and names ([`Catalogue::names`]); the window loads the presets
//! ([`crate::look`]).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use sf_engine::front::settings::{LookName, DEFAULT_LOOK, LOOK_OFF};
use sf_engine::front::tune::{is_saved, name_problem, SaveLook, SAVED_PREFIX};

/// A built-in look: its id in `settings.toml`, its name in the menus, and
/// its preset relative to the shaders folder.
pub struct BuiltIn {
    pub id: &'static str,
    pub name: &'static str,
    pub preset: &'static str,
}

/// The built-in looks, in list order after `OFF` (ticket #208: names,
/// tuning and order proposed, for the user's eye). Each holds the ship
/// bar in spec #203 when measured offscreen at true 4:3 on the 780M.
pub const BUILT_IN: [BuiltIn; 4] = [
    BuiltIn {
        id: DEFAULT_LOOK,
        name: "ARCADE CABINET",
        preset: "looks/arcade-cabinet.slangp",
    },
    BuiltIn {
        id: "consumer_tv",
        name: "CONSUMER TV",
        preset: "looks/consumer-tv.slangp",
    },
    BuiltIn {
        id: "pro_monitor",
        name: "PRO MONITOR",
        preset: "looks/pro-monitor.slangp",
    },
    BuiltIn {
        id: "curved_tube",
        name: "CURVED TUBE",
        preset: "looks/curved-tube.slangp",
    },
];

/// The shaders folder: `shaders` beside the executable, where the build
/// puts the vendored folder.
pub fn shaders_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("shaders")
}

/// The drop-in folder: `shaders` in the app-data folder, beside
/// `settings.toml`. The saved looks' `saved` folder inside it is not
/// searched for drop-ins.
pub fn drop_in_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("shaders")
}

/// The folder in [`drop_in_dir`] that holds the saved looks (VO10).
const SAVED: &str = "saved";

/// A saved look's file, its ids and names as the list has them.
const EXTENSION: &str = ".slangp";

/// How deep under the drop-in folder presets are looked for: deep enough
/// for RetroArch's own layout (`crt/crt-royale.slangp` beside `include/`
/// and `blurs/`, all inside a folder of the player's).
const DROP_IN_DEPTH: usize = 6;

/// Every `.slangp` under `dir` (not in its `saved` folder), as paths
/// relative to it with `/` between folders, sorted without regard to case.
/// A folder that can't be read is skipped.
pub fn find_drop_ins(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, rel: &str, depth: usize, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                if depth < DROP_IN_DEPTH && !(rel.is_empty() && name.eq_ignore_ascii_case(SAVED)) {
                    walk(&entry.path(), &path, depth + 1, out);
                }
            } else if Path::new(&name).extension().is_some_and(|e| e.eq_ignore_ascii_case("slangp")) {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, "", 0, &mut out);
    out.sort_by_key(|p| p.to_lowercase());
    out
}

/// A drop-in's name in the menus: `text` (its file name without
/// `.slangp`, or its whole relative path) upper-cased, `_` as a space, and
/// what the ROM font can't draw dropped.
fn menu_name(text: &str) -> String {
    let drawn: String = text
        .to_uppercase()
        .chars()
        .map(|c| if c == '_' { ' ' } else { c })
        .filter(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || " .-/()'!?+,:".contains(*c))
        .collect();
    let name = drawn.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        "DROP-IN".to_owned()
    } else {
        name
    }
}

/// The saved looks' file names (`NAME.slangp`) in `drop_ins`' `saved`
/// folder, sorted without regard to case. Only that folder, not what is
/// inside it.
pub fn find_saved(drop_ins: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(drop_ins.join(SAVED)) else { return Vec::new() };
    let mut out: Vec<String> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|k| k.is_file()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.len() > EXTENSION.len() && n.to_ascii_lowercase().ends_with(EXTENSION))
        .collect();
    out.sort_by_key(|n| n.to_lowercase());
    out
}

/// What a saved look is built on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Base {
    /// A built-in look, by its id: the preset moves with the game.
    BuiltIn(String),
    /// A preset under the drop-in folder, by its path relative to it.
    DropIn(String),
}

/// A saved look: its base and the parameters it sets away from the base's
/// values, in order.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedLook {
    pub base: Base,
    pub values: Vec<(String, f32)>,
}

/// How a saved look names a built-in base. It can't be a path, since the
/// game's folder moves; [`loadable`] turns it into one when the look loads.
const BUILT_IN_REF: &str = "builtin:";

/// A parameter id as a `#pragma parameter` can spell one.
fn param_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The `.slangp` text of `look`, in RetroArch's format: a `#reference` to
/// the base (a drop-in base by its path from the `saved` folder, so it
/// stays valid while that folder stays beside it), then the parameters set.
/// A built-in base is referenced by its id, which RetroArch itself can't
/// open (nobody asked for that).
pub fn slangp_text(look: &SavedLook) -> String {
    let reference = match &look.base {
        Base::BuiltIn(id) => format!("{BUILT_IN_REF}{id}"),
        Base::DropIn(rel) => format!("../{rel}"),
    };
    let mut out = format!("#reference \"{reference}\"\n\n# A look saved from the tuning screen.\n");
    for (id, value) in look.values.iter().filter(|(id, v)| param_id(id) && v.is_finite()) {
        out.push_str(&format!("{id} = \"{value}\"\n"));
    }
    out
}

/// A saved look from its `.slangp` text ([`slangp_text`] reversed).
pub fn parse_slangp(text: &str) -> Result<SavedLook, String> {
    let mut base = None;
    let mut values: Vec<(String, f32)> = Vec::new();
    for line in text.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("#reference") {
            let reference = rest.trim().trim_matches('"');
            if base.is_none() {
                base = Some(match reference.strip_prefix(BUILT_IN_REF) {
                    Some(id) if !id.is_empty() => Base::BuiltIn(id.to_owned()),
                    Some(_) => return Err("the #reference names no built-in look".to_owned()),
                    None => match reference.strip_prefix("../") {
                        Some(rel) if !rel.is_empty() && !rel.split('/').any(|part| part == ".." || part.contains(':')) => Base::DropIn(rel.to_owned()),
                        _ => return Err(format!("the #reference \"{reference}\" is not a built-in look or a path beside the saved folder")),
                    },
                });
            }
        } else if let Some((key, value)) = line.split_once('=').filter(|_| !line.starts_with('#')) {
            let (key, value) = (key.trim(), value.trim().trim_matches('"'));
            let number: f32 = value.parse().map_err(|_| format!("\"{value}\" is not a number for {key}"))?;
            if !param_id(key) || !number.is_finite() {
                return Err(format!("{key} is not a parameter this game can save"));
            }
            match values.iter_mut().find(|(id, _)| id == key) {
                Some(set) => set.1 = number,
                None => values.push((key.to_owned(), number)),
            }
        }
    }
    base.map(|base| SavedLook { base, values }).ok_or_else(|| "no #reference".to_owned())
}

/// Save `request` in `drop_ins`' `saved` folder as `NAME.slangp` and give
/// its id. The base is the tuned look's own (a saved look's base, with its
/// values under the new ones, so a saved look never builds on another
/// file); the parameters are the base's values with the request's set.
/// Nothing but the one file is written, and never in the built-in looks'
/// folder.
pub fn save_look(drop_ins: &Path, request: &SaveLook) -> Result<String, String> {
    if let Some(problem) = name_problem(&request.name) {
        return Err(format!("\"{}\": {}", request.name, problem.to_lowercase()));
    }
    let (base, mut values) = if let Some(built) = BUILT_IN.iter().find(|b| b.id == request.look) {
        (Base::BuiltIn(built.id.to_owned()), Vec::new())
    } else if is_saved(&request.look) {
        let path = drop_ins.join(&request.look);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let tuned = parse_slangp(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        (tuned.base, tuned.values)
    } else if request.look == LOOK_OFF {
        return Err("OFF has no parameters to save".to_owned());
    } else {
        (Base::DropIn(request.look.clone()), Vec::new())
    };
    for (id, value) in &request.overrides {
        match values.iter_mut().find(|(have, _)| have == id) {
            Some(set) => set.1 = *value,
            None => values.push((id.clone(), *value)),
        }
    }
    let dir = drop_ins.join(SAVED);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    // A saved look's name is its identity whatever the case (the menus show
    // it upper-cased, and Windows keeps a replaced file's old spelling), so
    // one saved under the name in another case is replaced under its own
    // file name, and the id given is the one the list holds.
    let wanted = format!("{}{EXTENSION}", request.name);
    let file = find_saved(drop_ins).into_iter().find(|f| f.to_lowercase() == wanted.to_lowercase()).unwrap_or(wanted);
    let (path, partial) = (dir.join(&file), dir.join(format!("{}.part", request.name)));
    // Written whole beside the file, then renamed over it, so a failed write
    // never leaves half a look (or loses the look it replaces).
    std::fs::write(&partial, slangp_text(&SavedLook { base, values })).map_err(|e| format!("{}: {e}", partial.display()))?;
    std::fs::rename(&partial, &path).map_err(|e| {
        let _ = std::fs::remove_file(&partial);
        format!("{}: {e}", path.display())
    })?;
    Ok(format!("{SAVED_PREFIX}{file}"))
}

/// The start of the folders [`loadable`] writes in the system's temporary
/// folder.
const TEMP_PREFIX: &str = "sf1-port-looks-";

/// How many presets [`loadable`] has written: each in a folder of its own.
static TEMP_LOADS: AtomicUsize = AtomicUsize::new(0);

/// The preset to load for `preset`, a catalogue entry's: itself, unless it
/// is a saved look on a built-in base, whose `#reference` names the base by
/// id. That one is written as a preset in a folder of its own in the
/// system's temporary folder, with the reference made a path to `root`'s
/// (the shaders folder now beside the executable), and its path given;
/// [`release`] removes it once loaded. The saved file itself never holds a
/// path into the game's folder, so it loads wherever the game has been
/// moved.
pub fn loadable(preset: &Path, root: &Path) -> Result<PathBuf, String> {
    let Ok(text) = std::fs::read_to_string(preset) else { return Ok(preset.to_path_buf()) };
    let Some(first) = text.lines().find(|l| l.trim_start().starts_with("#reference")) else { return Ok(preset.to_path_buf()) };
    let Some(id) = first.trim_start().strip_prefix("#reference").and_then(|r| r.trim().trim_matches('"').strip_prefix(BUILT_IN_REF)) else {
        return Ok(preset.to_path_buf());
    };
    let built = BUILT_IN.iter().find(|b| b.id == id).ok_or_else(|| format!("{}: no built-in look \"{id}\"", preset.display()))?;
    let target = root.join(built.preset).to_string_lossy().replace('\\', "/");
    let resolved = text.replacen(first, &format!("#reference \"{target}\""), 1);
    let load = TEMP_LOADS.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("{TEMP_PREFIX}{}-{load}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(preset.file_name().unwrap_or_default());
    std::fs::write(&path, resolved).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Remove the folder [`loadable`] wrote `preset` in, once the look has
/// loaded (librashader reads every file of a preset as its chain loads, so
/// the chain no longer needs it). Any other preset is left alone.
pub fn release(preset: &Path) {
    let Some(dir) = preset.parent() else { return };
    let ours = dir.parent() == Some(std::env::temp_dir().as_path())
        && dir.file_name().is_some_and(|n| n.to_string_lossy().starts_with(TEMP_PREFIX));
    if ours {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Remove [`loadable`]'s folders in `temp` last changed `age` or more ago:
/// what a game that stopped mid-load (or an older build) left behind. A
/// folder lives only while its look loads, so an hour-old one is no
/// running game's. Gives how many went.
pub fn sweep_temp(temp: &Path, age: Duration) -> usize {
    let Ok(entries) = std::fs::read_dir(temp) else { return 0 };
    entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(TEMP_PREFIX) && e.file_type().is_ok_and(|k| k.is_dir()))
        .filter(|e| e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t.elapsed().is_ok_and(|old| old >= age)))
        .filter(|e| std::fs::remove_dir_all(e.path()).is_ok())
        .count()
}

/// One look in the list: `preset` is `None` for `OFF`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub preset: Option<PathBuf>,
}

/// The looks in list order.
#[derive(Clone, Debug)]
pub struct Catalogue {
    looks: Vec<Entry>,
    /// How many of `looks`, after the built-ins, are saved looks.
    saved: usize,
}

impl Catalogue {
    /// `OFF` and the built-in looks, their presets under `root`. A preset
    /// missing from `root` is still listed: loading it fails, and the look
    /// falls back to `OFF` with the error.
    pub fn built_in(root: &Path) -> Catalogue {
        let mut looks = vec![Entry {
            id: LOOK_OFF.to_owned(),
            name: "OFF".to_owned(),
            preset: None,
        }];
        looks.extend(BUILT_IN.iter().map(|b| Entry {
            id: b.id.to_owned(),
            name: b.name.to_owned(),
            preset: Some(root.join(b.preset)),
        }));
        Catalogue { looks, saved: 0 }
    }

    /// `OFF`, the built-in looks under `root`, the player's saved looks
    /// ([`find_saved`]), then the drop-ins under `drop_ins`
    /// ([`find_drop_ins`]). A saved look's id is `saved/` and its file name;
    /// a drop-in's is its path relative to `drop_ins` (what `crt_look`
    /// stores) and its name its file name;
    /// a name another look already has becomes the drop-in's relative path,
    /// and one still taken (a drop-in at the folder's top) gets `(DROP-IN)`.
    /// A drop-in that doesn't load fails as a built-in would: `OFF`, with
    /// its first error on the notice line.
    pub fn discover(root: &Path, drop_ins: &Path) -> Catalogue {
        let mut catalogue = Catalogue::built_in(root);
        for file in find_saved(drop_ins) {
            let stem = &file[..file.len() - EXTENSION.len()];
            let mut name = menu_name(stem);
            if catalogue.looks.iter().any(|l| l.name == name) {
                name = menu_name(&format!("{SAVED}/{stem}"));
            }
            catalogue.looks.push(Entry {
                id: format!("{SAVED_PREFIX}{file}"),
                name,
                preset: Some(drop_ins.join(SAVED).join(&file)),
            });
            catalogue.saved += 1;
        }
        let found = find_drop_ins(drop_ins);
        let stem = |rel: &str| menu_name(Path::new(rel).file_stem().map_or(rel.into(), |s| s.to_string_lossy()).as_ref());
        let names: Vec<String> = found.iter().map(|rel| stem(rel)).collect();
        for (rel, name) in found.iter().zip(&names) {
            let taken = names.iter().filter(|n| *n == name).count() > 1 || catalogue.looks.iter().any(|l| &l.name == name);
            // find_drop_ins found it by its `.slangp`, in any case.
            let mut name = if taken { menu_name(&rel[..rel.len() - ".slangp".len()]) } else { name.clone() };
            // A drop-in at the top of the folder has no folder to tell it
            // apart, so a name still taken gets `(DROP-IN)`, then a number:
            // the same each run, since the list is in path order.
            let plain = name.clone();
            for n in 1.. {
                if !catalogue.looks.iter().any(|l| l.name == name) {
                    break;
                }
                name = if n == 1 { format!("{plain} (DROP-IN)") } else { format!("{plain} (DROP-IN {n})") };
            }
            catalogue.looks.push(Entry {
                id: rel.clone(),
                name,
                preset: Some(drop_ins.join(rel)),
            });
        }
        catalogue
    }

    /// The saved looks' ids, in list order.
    pub fn saved(&self) -> Vec<&str> {
        self.looks.iter().skip(1 + BUILT_IN.len()).take(self.saved).map(|l| l.id.as_str()).collect()
    }

    /// The drop-ins' ids, in list order.
    pub fn drop_ins(&self) -> Vec<&str> {
        self.looks.iter().skip(1 + BUILT_IN.len() + self.saved).map(|l| l.id.as_str()).collect()
    }

    pub fn looks(&self) -> &[Entry] {
        &self.looks
    }

    /// The ids and names, for the session.
    pub fn names(&self) -> Vec<LookName> {
        self.looks.iter().map(|l| LookName { id: l.id.clone(), name: l.name.clone() }).collect()
    }

    pub fn find(&self, id: &str) -> Option<&Entry> {
        self.looks.iter().find(|l| l.id == id)
    }

    /// `id` if the list holds it, else [`DEFAULT_LOOK`] and a line for the
    /// log saying why (a settings file naming a look that is gone).
    pub fn known_or_default(&self, id: &str) -> (String, Option<String>) {
        if self.find(id).is_some() {
            return (id.to_owned(), None);
        }
        let line = format!("crt_look \"{id}\" is not a known look; using \"{DEFAULT_LOOK}\"");
        (DEFAULT_LOOK.to_owned(), Some(line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_shaders() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shaders")
    }

    #[test]
    fn the_list_is_off_then_the_built_in_looks() {
        let c = Catalogue::built_in(Path::new("x"));
        let names: Vec<_> = c.names().into_iter().map(|l| (l.id, l.name)).collect();
        let expected = [
            ("off", "OFF"),
            ("arcade_cabinet", "ARCADE CABINET"),
            ("consumer_tv", "CONSUMER TV"),
            ("pro_monitor", "PRO MONITOR"),
            ("curved_tube", "CURVED TUBE"),
        ]
        .map(|(id, name)| (id.to_owned(), name.to_owned()));
        assert_eq!(names, expected);
        assert_eq!(c.find("off").unwrap().preset, None);
        assert_eq!(c.find("arcade_cabinet").unwrap().preset, Some(Path::new("x").join("looks/arcade-cabinet.slangp")));
    }

    #[test]
    fn an_unknown_look_falls_back_to_arcade_cabinet_with_a_log_line() {
        let c = Catalogue::built_in(Path::new("x"));
        assert_eq!(c.known_or_default("off"), ("off".to_owned(), None));
        assert_eq!(c.known_or_default("arcade_cabinet"), ("arcade_cabinet".to_owned(), None));
        let (id, line) = c.known_or_default("my-deleted-look.slangp");
        assert_eq!(id, "arcade_cabinet");
        assert!(line.unwrap().contains("my-deleted-look.slangp"));
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sf1-port-shaders-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(dir: &Path, rel: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "shaders = 1\n").unwrap();
    }

    #[test]
    fn drop_ins_follow_the_built_in_looks_by_path_named_by_their_files() {
        let dir = scratch("order");
        for rel in [
            "zebra.slangp",
            "Neon_Glow.slangp",
            "crt/crt-royale.slangp",
            "crt/shaders/crt-royale/pass.slang",
            "saved/mine.slangp",
            "notes.txt",
            "a/b/arcade cabinet.slangp",
            "one/dup.slangp",
            "two/DUP.SLANGP",
            "deep/1/2/3/4/5/6/too-deep.slangp",
        ] {
            touch(&dir, rel);
        }
        let c = Catalogue::discover(Path::new("x"), &dir);
        let names: Vec<_> = c.names().into_iter().map(|l| (l.id, l.name)).collect();
        let mut expected: Vec<(String, String)> =
            Catalogue::built_in(Path::new("x")).names().into_iter().map(|l| (l.id, l.name)).collect();
        expected.extend(
            [
                // The saved folder's look comes before the drop-ins.
                ("saved/mine.slangp", "MINE"),
                // Its name is a built-in look's, so it shows its path.
                ("a/b/arcade cabinet.slangp", "A/B/ARCADE CABINET"),
                ("crt/crt-royale.slangp", "CRT-ROYALE"),
                ("Neon_Glow.slangp", "NEON GLOW"),
                // Two drop-ins of one name each show their path.
                ("one/dup.slangp", "ONE/DUP"),
                ("two/DUP.SLANGP", "TWO/DUP"),
                ("zebra.slangp", "ZEBRA"),
            ]
            .map(|(id, name)| (id.to_owned(), name.to_owned())),
        );
        assert_eq!(names, expected, "OFF, the built-ins, the saved look, then the drop-ins; not too deep");
        assert_eq!(c.find("crt/crt-royale.slangp").unwrap().preset, Some(dir.join("crt/crt-royale.slangp")));
        assert_eq!(c.drop_ins().len(), 6);
        // No drop-in folder: the built-ins alone.
        let none = Catalogue::discover(Path::new("x"), &dir.join("absent"));
        assert_eq!(none.names(), Catalogue::built_in(Path::new("x")).names());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Review finding 4: a drop-in at the folder's top whose name is taken
    /// (a built-in's, a saved look's, another drop-in's) has no folder to
    /// show, so it is told apart with `(DROP-IN)`, the same on every run.
    #[test]
    fn a_top_level_drop_in_named_as_another_look_is_told_apart() {
        let dir = scratch("top-clash");
        for rel in ["Arcade Cabinet.slangp", "neon.slangp", "saved/neon.slangp", "glow_x.slangp", "glow x.slangp", "sub/glow x.slangp"] {
            touch(&dir, rel);
        }
        let names = || -> Vec<(String, String)> {
            Catalogue::discover(Path::new("x"), &dir).names().into_iter().skip(1 + BUILT_IN.len()).map(|l| (l.id, l.name)).collect()
        };
        let expected = [
            ("saved/neon.slangp", "NEON"),
            ("Arcade Cabinet.slangp", "ARCADE CABINET (DROP-IN)"),
            ("glow x.slangp", "GLOW X"),
            ("glow_x.slangp", "GLOW X (DROP-IN)"),
            ("neon.slangp", "NEON (DROP-IN)"),
            ("sub/glow x.slangp", "SUB/GLOW X"),
        ]
        .map(|(id, name)| (id.to_owned(), name.to_owned()));
        assert_eq!(names(), expected);
        assert_eq!(names(), expected, "stable across reads");
        let all = Catalogue::discover(Path::new("x"), &dir).names();
        for (i, a) in all.iter().enumerate() {
            assert!(!all[i + 1..].iter().any(|b| b.name == a.name), "{} is listed twice", a.name);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_drop_in_name_keeps_only_what_the_rom_font_draws() {
        assert_eq!(menu_name("crt-guest-advanced"), "CRT-GUEST-ADVANCED");
        assert_eq!(menu_name("my__look (v2)"), "MY LOOK (V2)");
        assert_eq!(menu_name("éclair~"), "CLAIR");
        assert_eq!(menu_name("~~~"), "DROP-IN");
    }

    #[test]
    fn a_deleted_drop_in_falls_back_to_arcade_cabinet_with_a_log_line() {
        let dir = scratch("deleted");
        touch(&dir, "looks/neon.slangp");
        let c = Catalogue::discover(Path::new("x"), &dir);
        assert_eq!(c.known_or_default("looks/neon.slangp"), ("looks/neon.slangp".to_owned(), None));
        std::fs::remove_file(dir.join("looks/neon.slangp")).unwrap();
        let c = Catalogue::discover(Path::new("x"), &dir);
        let (id, line) = c.known_or_default("looks/neon.slangp");
        assert_eq!(id, DEFAULT_LOOK);
        assert!(line.unwrap().contains("looks/neon.slangp"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_built_in_preset_is_in_the_vendored_shaders() {
        for b in &BUILT_IN {
            let path = repo_shaders().join(b.preset);
            assert!(path.is_file(), "{} has no preset at {}", b.name, path.display());
        }
    }

    // ---- saved looks (ticket #213) --------------------------------------------------

    fn request(look: &str, name: &str, overrides: &[(&str, f32)]) -> SaveLook {
        SaveLook {
            look: look.to_owned(),
            name: name.to_owned(),
            overrides: overrides.iter().map(|&(id, v)| (id.to_owned(), v)).collect(),
        }
    }

    /// Every file under `dir` with its bytes, by relative path.
    fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
        fn walk(dir: &Path, rel: &str, out: &mut Vec<(String, Vec<u8>)>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = format!("{rel}/{}", entry.file_name().to_string_lossy());
                if entry.file_type().unwrap().is_dir() {
                    walk(&entry.path(), &path, out);
                } else {
                    out.push((path, std::fs::read(entry.path()).unwrap()));
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, "", &mut out);
        out.sort();
        out
    }

    #[test]
    fn a_saved_look_writes_and_reads_back_its_base_and_overrides() {
        let built_in = SavedLook {
            base: Base::BuiltIn("arcade_cabinet".into()),
            values: vec![("brightboost".into(), 1.75), ("h_sharp".into(), 6.0), ("deconrr".into(), -0.25), ("tiny".into(), 0.0001)],
        };
        let drop_in = SavedLook { base: Base::DropIn("crt/crt-royale.slangp".into()), values: Vec::new() };
        for look in [built_in, drop_in] {
            let text = slangp_text(&look);
            assert!(text.starts_with("#reference \""), "RetroArch's format: the reference first\n{text}");
            assert_eq!(parse_slangp(&text).as_ref(), Ok(&look), "{text}");
        }
        let text = slangp_text(&SavedLook { base: Base::BuiltIn("consumer_tv".into()), values: vec![("a".into(), 0.4)] });
        assert_eq!(
            text,
            "#reference \"builtin:consumer_tv\"\n\n# A look saved from the tuning screen.\na = \"0.4\"\n",
            "values as RetroArch writes them, quoted"
        );
        // What can't be a parameter is left out, not written.
        let odd = SavedLook {
            base: Base::BuiltIn("arcade_cabinet".into()),
            values: vec![("fine".into(), 1.0), ("not an id".into(), 1.0), ("nan".into(), f32::NAN), ("shader0 = x\n#".into(), 1.0)],
        };
        assert_eq!(parse_slangp(&slangp_text(&odd)).unwrap().values, [("fine".to_owned(), 1.0)]);
    }

    #[test]
    fn a_saved_look_that_is_not_ours_is_refused_with_why() {
        for (text, mention) in [
            ("a = \"1\"\n", "no #reference"),
            ("#reference \"/abs/olute.slangp\"\n", "not a built-in look"),
            ("#reference \"../../escape.slangp\"\n", "not a built-in look"),
            ("#reference \"..\\x.slangp\"\n", "not a built-in look"),
            ("#reference \"builtin:\"\n", "names no built-in look"),
            ("#reference \"builtin:consumer_tv\"\na = \"fast\"\n", "not a number"),
            ("#reference \"builtin:consumer_tv\"\nshader0 = \"1\"\nbad id = \"1\"\n", "not a parameter"),
        ] {
            let error = parse_slangp(text).expect_err(text);
            assert!(error.contains(mention), "{text:?}: {error}");
        }
        // Later lines win, comments are ignored.
        let ok = parse_slangp("# note\n#reference \"builtin:pro_monitor\"\nx = \"1\"\n# x = \"9\"\nx = \"2\"\n").unwrap();
        assert_eq!(ok.values, [("x".to_owned(), 2.0)]);
    }

    #[test]
    fn saved_looks_come_after_the_built_ins_and_before_the_drop_ins() {
        let dir = scratch("saved-order");
        for rel in [
            "zebra.slangp",
            "crt/crt-royale.slangp",
            "saved/neon.slangp",
            "saved/Arcade Cabinet 2.slangp",
            "saved/notes.txt",
            "saved/old.slangp.part",
            "saved/sub/hidden.slangp",
            // A saved look named as a built-in is shown by its folder too.
            "saved/PRO MONITOR.slangp",
        ] {
            touch(&dir, rel);
        }
        let c = Catalogue::discover(Path::new("x"), &dir);
        let names: Vec<_> = c.names().into_iter().map(|l| (l.id, l.name)).collect();
        let mut expected: Vec<(String, String)> =
            Catalogue::built_in(Path::new("x")).names().into_iter().map(|l| (l.id, l.name)).collect();
        expected.extend(
            [
                ("saved/Arcade Cabinet 2.slangp", "ARCADE CABINET 2"),
                ("saved/neon.slangp", "NEON"),
                ("saved/PRO MONITOR.slangp", "SAVED/PRO MONITOR"),
                ("crt/crt-royale.slangp", "CRT-ROYALE"),
                ("zebra.slangp", "ZEBRA"),
            ]
            .map(|(id, name)| (id.to_owned(), name.to_owned())),
        );
        assert_eq!(names, expected, "OFF, built-in, saved, drop-in; the saved folder is not scanned for drop-ins");
        assert_eq!(c.saved(), ["saved/Arcade Cabinet 2.slangp", "saved/neon.slangp", "saved/PRO MONITOR.slangp"]);
        assert_eq!(c.drop_ins(), ["crt/crt-royale.slangp", "zebra.slangp"]);
        assert_eq!(c.find("saved/neon.slangp").unwrap().preset, Some(dir.join("saved").join("neon.slangp")));
        // The ids are ones the settings keep and the engine knows as saved.
        assert!(c.saved().iter().all(|id| is_saved(id)) && !c.drop_ins().iter().any(|id| is_saved(id)));
        let none = Catalogue::discover(Path::new("x"), &dir.join("absent"));
        assert!(none.saved().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A shaders folder like the game's, small: a look on a shader.
    fn fake_shaders(dir: &Path) {
        for b in &BUILT_IN {
            touch(dir, b.preset);
        }
    }

    #[test]
    fn saving_writes_one_file_in_the_saved_folder_and_never_a_built_in() {
        let (root, app) = (scratch("save-root"), scratch("save-app"));
        fake_shaders(&root);
        let drop_ins = drop_in_dir(&app);
        let before = snapshot(&root);
        let id = save_look(&drop_ins, &request("arcade_cabinet", "ARCADE CABINET 2", &[("brightboost", 1.75), ("h_sharp", 6.0)])).unwrap();
        assert_eq!(id, "saved/ARCADE CABINET 2.slangp");
        let files = snapshot(&app);
        assert_eq!(files.len(), 1, "one file, no partial left behind: {:?}", files.iter().map(|f| &f.0).collect::<Vec<_>>());
        assert_eq!(files[0].0, "/shaders/saved/ARCADE CABINET 2.slangp");
        let saved = parse_slangp(&String::from_utf8(files[0].1.clone()).unwrap()).unwrap();
        assert_eq!(saved.base, Base::BuiltIn("arcade_cabinet".into()));
        assert_eq!(saved.values, [("brightboost".to_owned(), 1.75), ("h_sharp".to_owned(), 6.0)]);
        // It is listed, as a saved look, and loadable beside the game's folder.
        let c = Catalogue::discover(&root, &drop_ins);
        assert_eq!(c.saved(), [id.as_str()]);
        let preset = c.find(&id).unwrap().preset.clone().unwrap();
        let resolved = loadable(&preset, &root).unwrap();
        assert!(resolved.is_file());
        release(&resolved);
        assert_eq!(snapshot(&root), before, "the shaders folder, built-in looks included, is untouched");
        // Saved again under the name: replaced, not duplicated.
        save_look(&drop_ins, &request("arcade_cabinet", "ARCADE CABINET 2", &[("brightboost", 2.0)])).unwrap();
        assert_eq!(Catalogue::discover(&root, &drop_ins).saved().len(), 1);
        let text = std::fs::read_to_string(drop_ins.join("saved/ARCADE CABINET 2.slangp")).unwrap();
        assert_eq!(parse_slangp(&text).unwrap().values, [("brightboost".to_owned(), 2.0)]);
        for d in [root, app] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    #[test]
    fn saving_from_a_saved_look_or_a_drop_in_keeps_the_original_base() {
        let app = scratch("save-chain");
        let drop_ins = drop_in_dir(&app);
        touch(&drop_ins, "crt/crt-royale.slangp");
        // A drop-in base is a path from the saved folder, and it finds the file.
        let id = save_look(&drop_ins, &request("crt/crt-royale.slangp", "ROYALE BRIGHT", &[("glow", 0.5)])).unwrap();
        let text = std::fs::read_to_string(drop_ins.join(&id)).unwrap();
        assert!(text.starts_with("#reference \"../crt/crt-royale.slangp\"\n"), "{text}");
        assert!(drop_ins.join(SAVED).join("../crt/crt-royale.slangp").is_file(), "the reference is valid from the saved folder");
        // From a saved look: its base, its values under the new ones.
        let first = save_look(&drop_ins, &request("consumer_tv", "TV ONE", &[("a", 1.0), ("b", 2.0)])).unwrap();
        let second = save_look(&drop_ins, &request(&first, "TV TWO", &[("b", 3.0), ("c", 4.0)])).unwrap();
        let tuned = parse_slangp(&std::fs::read_to_string(drop_ins.join(&second)).unwrap()).unwrap();
        assert_eq!(tuned.base, Base::BuiltIn("consumer_tv".into()), "never a look on a look");
        assert_eq!(tuned.values, [("a".to_owned(), 1.0), ("b".to_owned(), 3.0), ("c".to_owned(), 4.0)]);
        // And the first is as it was.
        let kept = parse_slangp(&std::fs::read_to_string(drop_ins.join(&first)).unwrap()).unwrap();
        assert_eq!(kept.values, [("a".to_owned(), 1.0), ("b".to_owned(), 2.0)]);
        let _ = std::fs::remove_dir_all(&app);
    }

    #[test]
    fn a_save_that_cannot_be_written_says_why_and_writes_nothing() {
        let app = scratch("save-bad");
        let drop_ins = drop_in_dir(&app);
        for (look, name) in [("off", "NEON"), ("arcade_cabinet", ""), ("arcade_cabinet", "CON"), ("arcade_cabinet", "A/B"), ("arcade_cabinet", "..")] {
            assert!(save_look(&drop_ins, &request(look, name, &[])).is_err(), "{look} {name:?}");
        }
        // A saved look that is gone, or not a look of ours.
        assert!(save_look(&drop_ins, &request("saved/gone.slangp", "NEON", &[])).is_err());
        touch(&drop_ins, "saved/foreign.slangp");
        let error = save_look(&drop_ins, &request("saved/foreign.slangp", "NEON", &[])).unwrap_err();
        assert!(error.contains("no #reference"), "{error}");
        assert_eq!(find_saved(&drop_ins), ["foreign.slangp"], "only the file that was there");
        // The folder is a file: the write fails with the path, nothing is left.
        let blocked = scratch("save-blocked");
        std::fs::write(blocked.join("shaders"), "x").unwrap();
        let error = save_look(&drop_in_dir(&blocked), &request("consumer_tv", "NEON", &[])).unwrap_err();
        assert!(error.contains("shaders"), "{error}");
        for d in [app, blocked] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    #[test]
    fn a_saved_look_on_a_built_in_loads_from_wherever_the_game_is() {
        let (app, here, moved) = (scratch("move-app"), scratch("move-here"), scratch("move-there"));
        let drop_ins = drop_in_dir(&app);
        let id = save_look(&drop_ins, &request("pro_monitor", "MY MONITOR", &[("glow", 0.5)])).unwrap();
        let file = drop_ins.join(&id);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("builtin:pro_monitor") && !text.contains(here.to_str().unwrap()), "no path into the game's folder:\n{text}");
        for root in [&here, &moved] {
            fake_shaders(root);
            let resolved = loadable(&file, root).unwrap();
            let body = std::fs::read_to_string(&resolved).unwrap();
            let target = root.join("looks/pro-monitor.slangp").to_string_lossy().replace('\\', "/");
            assert!(body.starts_with(&format!("#reference \"{target}\"\n")), "{body}");
            assert!(body.contains("glow = \"0.5\""), "{body}");
            assert!(Path::new(&target).is_file(), "the base it names is there");
            release(&resolved);
        }
        assert_eq!(std::fs::read_to_string(&file).unwrap(), text, "the saved file does not change with the game's place");
        // A drop-in base needs nothing resolved; an unknown built-in fails.
        let on_drop_in = {
            touch(&drop_ins, "crt/x.slangp");
            drop_ins.join(save_look(&drop_ins, &request("crt/x.slangp", "DROPPED", &[])).unwrap())
        };
        assert_eq!(loadable(&on_drop_in, &here).unwrap(), on_drop_in);
        std::fs::write(&file, "#reference \"builtin:nowhere\"\n").unwrap();
        assert!(loadable(&file, &here).unwrap_err().contains("nowhere"));
        // A file that isn't there is left for the loader to report.
        assert_eq!(loadable(&app.join("absent.slangp"), &here).unwrap(), app.join("absent.slangp"));
        for d in [app, here, moved] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    /// Review finding 6: saving under a saved look's name in another case
    /// replaces that file, and the id given is the one the list holds.
    #[test]
    fn saving_under_a_saved_names_other_case_replaces_it_under_its_listed_id() {
        let (app, root) = (scratch("case-app"), scratch("case-root"));
        fake_shaders(&root);
        let drop_ins = drop_in_dir(&app);
        // The game writes names upper-case; this one was named by hand.
        let look = SavedLook { base: Base::BuiltIn("consumer_tv".into()), values: vec![("a".into(), 1.0)] };
        std::fs::create_dir_all(drop_ins.join(SAVED)).unwrap();
        std::fs::write(drop_ins.join(SAVED).join("Neon.slangp"), slangp_text(&look)).unwrap();
        let first = Catalogue::discover(&root, &drop_ins).saved()[0].to_owned();
        assert_eq!(first, "saved/Neon.slangp");
        let again = save_look(&drop_ins, &request("consumer_tv", "NEON", &[("a", 2.0)])).unwrap();
        let listed = Catalogue::discover(&root, &drop_ins);
        assert_eq!(listed.saved(), [again.as_str()], "one look, listed under the id given");
        assert_eq!(again, first, "the file keeps its own name");
        let text = std::fs::read_to_string(drop_ins.join(&again)).unwrap();
        assert_eq!(parse_slangp(&text).unwrap().values, [("a".to_owned(), 2.0)], "replaced");
        for d in [app, root] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    /// Review finding 5: each resolved preset is written in a temporary
    /// folder of its own, which [`release`] removes (and nothing else), and
    /// [`sweep_temp`] clears old ones a stopped game left.
    #[test]
    fn each_resolved_preset_has_its_own_temporary_folder_released_after_loading() {
        let (app, root) = (scratch("temp-app"), scratch("temp-root"));
        fake_shaders(&root);
        let drop_ins = drop_in_dir(&app);
        let id = save_look(&drop_ins, &request("pro_monitor", "TEMP", &[("glow", 0.5)])).unwrap();
        let saved = drop_ins.join(&id);
        let (a, b) = (loadable(&saved, &root).unwrap(), loadable(&saved, &root).unwrap());
        assert_ne!(a.parent(), b.parent(), "one folder per load: no two loads share one");
        assert!(a.parent().unwrap().starts_with(std::env::temp_dir()), "{}", a.display());
        release(&a);
        assert!(!a.parent().unwrap().exists() && b.is_file(), "only its own folder goes");
        release(&b);
        assert!(!b.parent().unwrap().exists());
        // Any other preset is never removed.
        release(&saved);
        touch(&root, "x/y.slangp");
        release(&root.join("x/y.slangp"));
        assert!(saved.is_file() && root.join("x/y.slangp").is_file());
        // The sweep takes old folders of loadable's naming, and only those.
        let temp = scratch("temp-sweep");
        for name in ["sf1-port-looks-1", "sf1-port-looks-2-7", "keep-me"] {
            touch(&temp, &format!("{name}/a.slangp"));
        }
        assert_eq!(sweep_temp(&temp, Duration::from_secs(3600)), 0, "nothing an hour old");
        assert_eq!(sweep_temp(&temp, Duration::ZERO), 2);
        let left: Vec<_> = std::fs::read_dir(&temp).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert_eq!(left, ["keep-me"]);
        for d in [app, root, temp] {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}
