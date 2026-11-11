//! Acceptance test for ticket #23: the CPU opponent's AI-table setup
//! (0x44880, with the move-table pointer 0x44860).
//!
//! - Retsu (stage 0, fight 0, the fixture's difficulty and country): the
//!   setup over the fixture's work RAM, with the AI tables and the CPU
//!   structure's move-table pointer wiped first, reproduces the reference's
//!   tables (0xffc058..0xffc664) and pointer exactly, as they stand both at
//!   the fixture (2389) and later in the round (2460).
//! - Every opponent (10), difficulty (4) and variant (2): each copy reads
//!   inside that opponent's own 0x300e-byte block of AI data (the blocks at
//!   0x1c048 + 0x300e k), at the layout's offsets (ranges +0x80, limits
//!   +0x86, moves +0x8e.., stunned +0x20e.., rolls +0x30e / +0x38e,
//!   reactions +0x40e / +0x80e, attack records +0xc0e / +0x180e / +0x240e),
//!   never past the next table. How many of the eight settings give
//!   different tables is a property of the data: every opponent varies
//!   except the last (stage 4, fight 1), whose rows are all the same.
//!
//! Consumes local-only data; skips on a clean checkout.

use std::path::Path;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const CPU: &str = "../../tools/reference/local/captures/cpu-01-run-a";

fn inputs() -> Option<(Vec<u8>, Vec<u8>)> {
    if !Path::new(CPU).is_dir() || !Path::new(ROMS).is_dir() {
        return None;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).ok()?;
    let work = std::fs::read(Path::new(CPU).join("ram_002389_work.bin")).ok()?;
    Some((rom, work))
}

const TABLES: std::ops::Range<usize> = 0x4058..0x4664;
const CPU_MOVES: std::ops::Range<usize> = 0x4664 + 0x12..0x4664 + 0x16;

#[test]
fn retsus_tables_match_the_reference_exactly() {
    let Some((rom, mut work)) = inputs() else {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    };
    let (stage, fight, country, difficulty) =
        (work[0x4003], work[0x4004], work[0x4006], work[0x4007]);
    assert_eq!((stage, fight), (0, 0), "the fixed fight's first opponent");
    let reference = work.clone();
    work[TABLES].fill(0);
    work[CPU_MOVES].fill(0);
    let (native, _) =
        sf_engine::run::cpu_opponent_setup(rom, &work, stage, fight, difficulty, country).unwrap();
    assert_eq!(native[TABLES], reference[TABLES], "AI tables as at 2389");
    assert_eq!(
        native[CPU_MOVES], reference[CPU_MOVES],
        "Retsu's move table 0x16e6c"
    );
    let later = std::fs::read(Path::new(CPU).join("ram_002433_work.bin")).unwrap();
    assert_eq!(native[TABLES], later[TABLES], "unchanged through the round");
}

#[test]
fn every_opponent_difficulty_and_variant_reads_inside_its_own_block() {
    let Some((rom, work)) = inputs() else {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    };
    // Layout offsets inside a block and the next table's offset (the
    // bound a copy must not cross), by destination.
    let layout = |dst: u32| -> (u32, u32) {
        match dst {
            0xffc068 => (0x80, 0x86),
            0xffc058 => (0x00, 0x80),
            0xffc06e => (0x86, 0x8e),
            0xffc074 => (0x8e, 0x20e),
            0xffc0a4 => (0x20e, 0x30e),
            0xffc0c4 => (0x30e, 0x38e),
            0xffc0d4 => (0x38e, 0x40e),
            0xffc0e4 => (0x40e, 0x80e),
            0xffc164 => (0x80e, 0xc0e),
            0xffc1e4 => (0xc0e, 0x180e),
            0xffc364 => (0x180e, 0x240e),
            0xffc4e4 => (0x240e, 0x300e),
            _ => panic!("unexpected destination {dst:#x}"),
        }
    };
    let mut blocks = std::collections::BTreeSet::new();
    let mut checked = 0;
    let mut counts = Vec::new();
    for stage in 0..5u8 {
        for fight in 0..2u8 {
            let mut images = Vec::new();
            for difficulty in 0..4u8 {
                for variant in 0..2u8 {
                    let (native, copies) = sf_engine::run::cpu_opponent_setup(
                        rom.clone(),
                        &work,
                        stage,
                        fight,
                        difficulty,
                        variant << 1,
                    )
                    .unwrap();
                    // The block starts where the range classes start (offset 0).
                    let classes = copies.iter().find(|c| c.1 == 0xffc058).unwrap().0;
                    let base = classes - (difficulty as u32 * 16 + variant as u32 * 64);
                    assert_eq!((base - 0x1c048) % 0x300e, 0, "block base {base:#x}");
                    blocks.insert(base);
                    for (src, dst, words) in &copies {
                        let (start, end) = layout(*dst);
                        let (lo, hi) = (*src - base, *src - base + 2 * words);
                        assert!(lo >= start && hi <= end, "stage {stage} fight {fight} d{difficulty} v{variant}: {dst:#x} reads {lo:#x}..{hi:#x}, table {start:#x}..{end:#x}");
                    }
                    images.push(native[TABLES].to_vec());
                    checked += 1;
                }
            }
            let distinct: std::collections::BTreeSet<_> = images.iter().collect();
            counts.push(distinct.len());
        }
    }
    assert_eq!(checked, 80);
    assert_eq!(blocks.len(), 10, "ten distinct opponent blocks");
    eprintln!("distinct table images per opponent (of 8 difficulty/variant settings): {counts:?}");
    // The data, not the translation, decides how many settings differ: the
    // last opponent's rows are the same at every difficulty and variant.
    assert_eq!(counts[9], 1, "the last opponent is difficulty-invariant");
    assert!(
        counts[..9].iter().all(|&c| c > 1),
        "the other opponents vary"
    );
}
