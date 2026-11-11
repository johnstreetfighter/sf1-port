//! Ticket #204 (`VO1`): the vendored RetroArch shaders. Every `.slangp`
//! under `shaders/` resolves with nothing missing: each `shaderN`, texture
//! and `#reference` target exists, and each `#include` inside a `.slang`
//! source (and inside the headers it pulls in) exists too. The build copies
//! the folder beside the game executable.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn shaders_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shaders")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn unquote(value: &str) -> &str {
    value.trim().trim_matches('"').trim()
}

/// The files a preset line names, relative to the preset's own folder.
fn preset_refs(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("#reference") {
            refs.push(unquote(rest).to_string());
            continue;
        }
        let line = line.split('#').next().unwrap().trim();
        let Some((key, value)) = line.split_once('=') else { continue };
        let (key, value) = (key.trim(), unquote(value));
        let is_shader = key.starts_with("shader") && key[6..].chars().all(|c| c.is_ascii_digit()) && key.len() > 6;
        let is_file = [".slang", ".slangp", ".png", ".tga", ".jpg", ".bmp"]
            .iter()
            .any(|ext| value.to_ascii_lowercase().ends_with(ext));
        if is_shader || is_file {
            refs.push(value.to_string());
        }
    }
    refs
}

fn include_refs(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix('#')?.trim_start().strip_prefix("include")?;
            let rest = rest.trim_start().strip_prefix('"')?;
            Some(rest[..rest.find('"')?].to_string())
        })
        .collect()
}

/// Walks one file and everything it references; pushes a message per miss.
fn check(file: &Path, seen: &mut BTreeSet<PathBuf>, problems: &mut Vec<String>) {
    let key = fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    if !seen.insert(key) {
        return;
    }
    let dir = file.parent().unwrap();
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let text = match ext.as_str() {
        "png" | "tga" | "jpg" | "bmp" => return,
        _ => String::from_utf8_lossy(&fs::read(file).unwrap()).into_owned(),
    };
    let refs = if ext == "slangp" { preset_refs(&text) } else { include_refs(&text) };
    for r in refs {
        let target = dir.join(&r);
        if !target.is_file() {
            problems.push(format!("{}: {r} is missing", file.display()));
        } else {
            check(&target, seen, problems);
        }
    }
}

#[test]
fn every_vendored_preset_resolves_with_nothing_missing() {
    let root = shaders_root();
    assert!(root.is_dir(), "shaders/ is not at {}", root.display());
    let mut files = Vec::new();
    walk(&root, &mut files);
    let presets: Vec<&PathBuf> = files
        .iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("slangp"))
        .collect();
    assert!(
        presets.len() >= 7,
        "expected the Guest, Royale and Geom presets, found {}",
        presets.len()
    );

    let mut seen = BTreeSet::new();
    let mut problems = Vec::new();
    for preset in &presets {
        check(preset, &mut seen, &mut problems);
    }
    assert!(problems.is_empty(), "unresolved references:\n{}", problems.join("\n"));
}

#[test]
fn the_chosen_presets_are_all_vendored() {
    let root = shaders_root();
    for name in [
        "crt-guest-advanced",
        "crt-guest-advanced-fast",
        "crt-guest-advanced-fastest",
        "crt-guest-advanced-ntsc",
        "crt-royale",
        "crt-royale-fast",
        "crt-geom",
    ] {
        let path = root.join("crt").join(format!("{name}.slangp"));
        assert!(path.is_file(), "{} is not vendored", path.display());
    }
}

#[test]
fn the_readme_names_the_source_and_the_pinned_commit() {
    let readme = fs::read_to_string(shaders_root().join("README.md")).unwrap();
    assert!(readme.contains("https://github.com/libretro/slang-shaders"));
    assert!(readme.contains("53deb2efcb79f4fc3e94e9602d5974b4b64e8f94"));
    for license in ["GPL v2-or-later", "MIT"] {
        assert!(readme.contains(license), "README does not name {license}");
    }
}

#[test]
fn the_build_copies_shaders_beside_the_executable() {
    // The test executable sits in target/<profile>/deps; sf-game.exe and the
    // copied folder sit one level up.
    let exe = std::env::current_exe().unwrap();
    let profile = exe.parent().unwrap().parent().unwrap();
    let copied = profile.join("shaders").join("crt").join("crt-guest-advanced.slangp");
    assert!(copied.is_file(), "build.rs did not copy shaders/ to {}", copied.display());
}
