//! Copies the vendored shaders and embeds the Windows executable icon.
//!
//! The game looks for its shader files next to the executable, so every
//! build copies the repository's `shaders/` into the profile folder
//! (`target/release/shaders` for a release build). The copy is rebuilt when
//! anything under `shaders/` changes.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let target = env::var("TARGET").expect("TARGET");
    if target.ends_with("-windows-msvc") {
        embed_windows_icon(&manifest, &out, &target);
    }

    let source = manifest.join("..").join("..").join("shaders");
    println!("cargo:rerun-if-changed={}", source.display());
    if !source.is_dir() {
        println!(
            "cargo:warning=shaders/ not found at {}; nothing copied",
            source.display()
        );
        return;
    }
    // OUT_DIR is target/<profile>/build/sf-game-<hash>/out: three levels up
    // is the profile folder that holds sf-game.exe.
    let profile = out
        .ancestors()
        .nth(3)
        .expect("OUT_DIR has a profile folder above it")
        .to_path_buf();
    let target = profile.join("shaders");
    if let Err(e) = copy_tree(&source, &target) {
        panic!("copying shaders/ to {}: {e}", target.display());
    }
}

fn embed_windows_icon(manifest: &Path, out: &Path, target: &str) {
    let assets = manifest.join("assets");
    println!("cargo:rerun-if-changed=assets/sf-game.ico");
    println!("cargo:rerun-if-changed=assets/sf-game.rc");

    let mut compiler =
        find_msvc_tools::find(target, "rc.exe").unwrap_or_else(|| Command::new("rc.exe"));
    let resource = out.join("sf-game.res");
    let status = compiler
        .current_dir(assets)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg("sf-game.rc")
        .status()
        .expect("run rc.exe from the Windows SDK to embed the game icon");
    assert!(
        status.success(),
        "compiling the Windows icon failed: {status}"
    );
    println!("cargo:rustc-link-arg-bin=sf-game={}", resource.display());
}

fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else {
            // Skip files already identical, so repeated builds touch nothing.
            let same = match (fs::metadata(entry.path()), fs::metadata(&dest)) {
                (Ok(a), Ok(b)) => a.len() == b.len() && fs::read(entry.path())? == fs::read(&dest)?,
                _ => false,
            };
            if !same {
                fs::copy(entry.path(), &dest)?;
            }
        }
    }
    Ok(())
}
