//! Builds libopenmpt from `vendor/libopenmpt` (the official 0.8.10 source
//! release, `libopenmpt-0.8.10+release.makefile.tar.gz` from
//! lib.openmpt.org, trimmed to what the library needs) into a static
//! library. Needs only a C and a C++17 compiler: MSVC on Windows (the
//! toolchain Rust already needs there), gcc or clang elsewhere. No
//! external libraries: zlib, MP3 and Vorbis decoding for compressed modules
//! and samples come from the miniz, minimp3 and stb_vorbis files in
//! `vendor/libopenmpt/include`, as libopenmpt's own makefile falls back to.

use std::path::{Path, PathBuf};

fn sources(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == extension))
        .collect();
    files.sort();
    files
}

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("vendor/libopenmpt");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=vendor/libopenmpt");

    // The vendored source is committed: a missing tree is a broken checkout,
    // or a build that should have turned the `tracker` feature off.
    let missing = |what: &str| -> ! {
        panic!(
            "libopenmpt's vendored source is missing ({what} under {}). Restore it (`git checkout -- crates/sf-openmpt/vendor`),              or build without tracker module support: `cargo build --no-default-features`.",
            root.display()
        )
    };
    for needed in ["common", "soundlib", "sounddsp", "libopenmpt", "src/mpt", "include/miniz/miniz.c", "include/minimp3/minimp3.c", "include/stb_vorbis/stb_vorbis.c"] {
        if !root.join(needed).exists() {
            missing(needed);
        }
    }

    // The directories libopenmpt's makefile compiles (COMMON, SOUNDLIB and
    // LIBOPENMPT sources).
    let mut cpp = Vec::new();
    for dir in [
        "common",
        "soundlib",
        "soundlib/plugins",
        "soundlib/plugins/dmo",
        "sounddsp",
        "libopenmpt",
        "src/openmpt/all",
        "src/openmpt/base",
        "src/openmpt/logging",
        "src/openmpt/random",
        "src/openmpt/fileformat_base",
        "src/openmpt/soundbase",
        "src/openmpt/soundfile_data",
    ] {
        let dir = root.join(dir);
        if dir.is_dir() {
            cpp.extend(sources(&dir, "cpp"));
        }
    }
    if cpp.is_empty() {
        missing("any .cpp file");
    }

    let includes = [root.join("src"), root.join("common"), root.clone(), root.join("include")];

    let mut build = cc::Build::new();
    build.cpp(true).warnings(false).extra_warnings(false).opt_level(2);
    for dir in &includes {
        build.include(dir);
    }
    // C and C++ asserts off, as the library's own release builds have them: a
    // failed assert aborts the whole game, and the Jukebox's fallback is a log line.
    build.define("NDEBUG", None);
    build.define("LIBOPENMPT_BUILD", None).define("MPT_WITH_MINIZ", None).define("MPT_WITH_MINIMP3", None).define("MPT_WITH_STBVORBIS", None);
    build.define("STB_VORBIS_NO_PULLDATA_API", None).define("STB_VORBIS_NO_STDIO", None);
    if build.get_compiler().is_like_msvc() {
        build.flag("/std:c++17").flag("/Zc:__cplusplus").flag("/Zc:preprocessor").flag("/EHsc").flag("/utf-8");
        build.define("_CRT_SECURE_NO_WARNINGS", None).define("UNICODE", None).define("_UNICODE", None);
    } else {
        build.flag("-std=c++17").flag_if_supported("-fvisibility=hidden");
    }
    build.files(&cpp);
    build.compile("openmpt_cpp");

    let mut c = cc::Build::new();
    c.warnings(false).extra_warnings(false).opt_level(2);
    for dir in &includes {
        c.include(dir);
    }
    c.define("NDEBUG", None);
    c.define("LIBOPENMPT_BUILD", None).define("MPT_WITH_MINIZ", None).define("MPT_WITH_MINIMP3", None).define("MPT_WITH_STBVORBIS", None);
    c.define("STB_VORBIS_NO_PULLDATA_API", None).define("STB_VORBIS_NO_STDIO", None);
    if c.get_compiler().is_like_msvc() {
        c.define("_CRT_SECURE_NO_WARNINGS", None);
    }
    for file in ["include/miniz/miniz.c", "include/minimp3/minimp3.c", "include/stb_vorbis/stb_vorbis.c"] {
        c.file(root.join(file));
    }
    c.compile("openmpt_c");
}
