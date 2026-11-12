//! Check the character art in every authored later-era move stream.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use sf_engine::{
    box_audit::{fighter_moves, walk_stream, FrameRecord, PathEnd, Pose},
    data_bank::{
        in_bank, DataBank, DATA_BANK_BASE, DATA_BANK_END, PLAYER_MOVES, ROUTINE_ALPHA_HOLD_P,
        ROUTINE_ALPHA_REPEAT_K, ROUTINE_ALPHA_WALL_REBOUND, ROUTINE_SF6_TIGER_NEXUS,
    },
    era::EraFamily,
    roster::TournamentFighter,
};
use sf_video::{
    art::{load_art_from, ArtSources, Grid},
    art_pack::ArtPack,
    romset::load_assets,
    sprite_bank::sprite_slot,
};

const FAMILIES: [EraFamily; 6] = [
    EraFamily::Sf2,
    EraFamily::SfZ,
    EraFamily::Sf3,
    EraFamily::Sf4,
    EraFamily::Sf5,
    EraFamily::Sf6,
];

type Shape = (u16, bool, u8, u8);

fn shape(frame: &FrameRecord) -> Shape {
    (frame.code, frame.big(), frame.rows, frame.cols)
}

fn grid(shape: Shape) -> Grid {
    Grid {
        code: shape.0,
        large: shape.1,
        rows: shape.2,
        cols: shape.3,
    }
}

fn frames(rom: &[u8], header: Option<u32>, stream: u32) -> (Vec<FrameRecord>, PathEnd) {
    let first = header.map(|at| FrameRecord::from_header(rom, at).expect("valid header grid"));
    if stream == 0 {
        return (first.into_iter().collect(), PathEnd::End);
    }
    let walk = walk_stream(rom, first, stream);
    (walk.frames, walk.end)
}

fn read_long(rom: &[u8], at: u32) -> u32 {
    u32::from_be_bytes(rom[at as usize..at as usize + 4].try_into().unwrap())
}

fn bank_frames(program: &[u8], header: u32) -> (Vec<FrameRecord>, Vec<(u32, PathEnd)>) {
    let mut result = Vec::new();
    let mut errors = Vec::new();
    let mut seen_frames = BTreeSet::new();
    let mut seen_streams = BTreeSet::new();
    let mut pending = vec![(Some(header), read_long(program, header + 0x2c))];
    while let Some((header, start)) = pending.pop() {
        if !seen_streams.insert(start) {
            continue;
        }
        let first =
            header.map(|at| FrameRecord::from_header(program, at).expect("valid era header grid"));
        if start == 0 {
            result.extend(first);
            continue;
        }
        let walk = walk_stream(program, first, start);
        if matches!(walk.end, PathEnd::Cap | PathEnd::Broken) {
            errors.push((start, walk.end));
        }
        result.extend(
            walk.frames
                .into_iter()
                .filter(|frame| seen_frames.insert(frame.addr)),
        );
        // These authored calls can jump to extra records or a replay loop.
        for (_, routine) in walk
            .calls
            .into_iter()
            .filter(|(_, routine)| in_bank(*routine))
        {
            let target_count = match program[routine as usize] {
                ROUTINE_ALPHA_HOLD_P | ROUTINE_ALPHA_REPEAT_K | ROUTINE_ALPHA_WALL_REBOUND => 1,
                ROUTINE_SF6_TIGER_NEXUS => 3,
                _ => 0,
            };
            for index in 0..target_count {
                pending.push((None, read_long(program, routine + 4 + 4 * index)));
            }
        }
    }
    (result, errors)
}

fn fighter_palette(fighter: TournamentFighter, frame: &FrameRecord) -> bool {
    let palette = frame.attr as u8 & 0xf;
    palette == fighter.sprite_palette()
        // Geki temporarily replaces his whole body grid with his ROM teleport smoke.
        || (fighter == TournamentFighter::Geki && palette == 4
            && matches!(frame.code, 0x11e6 | 0x1326 | 0x1329 | 0x132d))
}

fn own_rom_shapes(rom: &[u8], fighter: TournamentFighter) -> BTreeSet<Shape> {
    fighter_moves(rom, fighter)
        .into_iter()
        .flat_map(|entry| frames(rom, entry.header, entry.stream).0)
        // Unused CPU-boss headers can contain a dummy sprite. Validate
        // against body poses in the fighter's palette and known smoke.
        .filter(|frame| (fighter.is_shoto() || frame.code != 0) && fighter_palette(fighter, frame))
        .map(|frame| shape(&frame))
        .collect()
}

#[test]
fn later_era_move_frames_use_visible_art_for_their_own_fighter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let roms = root.join("tools/reference/local/roms/sf");
    let pack = root.join("tools/reference/local/roms/sf1-port-art");
    if !roms.is_dir() || !pack.is_dir() {
        eprintln!("skipping: local ROMs and approved art pack required");
        return;
    }
    let assets = load_assets(roms, root.join("tools/reference/rom-manifest.json")).unwrap();
    let pack = ArtPack::open(&pack).unwrap();
    let fighters = TournamentFighter::ALL;
    let recipes: Vec<_> = fighters
        .iter()
        .map(|fighter| {
            fs::read_to_string(root.join(format!(
                "art/recipes/{}.json",
                fighter.name().to_ascii_lowercase()
            )))
            .unwrap()
        })
        .collect();
    let reviews: Vec<_> = fighters
        .iter()
        .map(|fighter| {
            fs::read_to_string(root.join(format!(
                "art/review/{}.json",
                fighter.name().to_ascii_lowercase()
            )))
            .unwrap()
        })
        .collect();
    let recipes: Vec<_> = recipes.iter().map(String::as_str).collect();
    let reviews: Vec<_> = reviews.iter().map(String::as_str).collect();
    let art = load_art_from(
        &assets,
        &ArtSources {
            recipes: &recipes,
            reviews: Some(&reviews),
            pack: Ok(&pack),
        },
    )
    .unwrap();
    assert_eq!(art.catalogues.len(), fighters.len(), "{:?}", art.kept);
    let bank = DataBank::build(&assets.maincpu, &art.catalogues);
    assert_eq!(
        bank.fighters().len(),
        fighters.len() - 2,
        "{:?}",
        bank.notes()
    );
    let banked_assets = assets.with_sprite_bank(&art.bank);
    let mut program = assets.maincpu.clone();
    program.resize((DATA_BANK_END + 1) as usize, 0xff);
    for at in DATA_BANK_BASE..=DATA_BANK_END {
        program[at as usize] = bank.rb(at);
    }

    let rom_shapes: BTreeMap<_, _> = fighters
        .into_iter()
        .map(|fighter| (fighter as u8, own_rom_shapes(&assets.maincpu, fighter)))
        .collect();
    let mut failures = Vec::new();
    let mut total_slots = 0;
    let mut total_frames = 0;
    let mut total_poses = 0;
    for fighter in fighters {
        let cat = art.catalogue(&fighter.name().to_ascii_lowercase()).unwrap();
        let own = &rom_shapes[&(fighter as u8)];
        let catalogue_shapes: BTreeSet<_> = cat
            .poses
            .iter()
            .map(|pose| {
                let Grid {
                    code,
                    large,
                    rows,
                    cols,
                } = pose.grid;
                (code, large, rows, cols)
            })
            .collect();
        let mut headers = BTreeMap::<u32, Vec<String>>::new();
        for slot in 0..PLAYER_MOVES {
            for family in FAMILIES {
                if let Some(header) = bank.era_move_header(fighter, family, slot) {
                    headers
                        .entry(header)
                        .or_default()
                        .push(format!("{family:?} {slot:#04x}"));
                }
            }
            if slot >= 0x1f {
                if let Some(header) = bank.move_header(fighter, slot) {
                    headers
                        .entry(header)
                        .or_default()
                        .push(format!("shared {slot:#04x}"));
                }
            }
        }
        assert!(!headers.is_empty(), "{fighter:?} has no later-era moves");
        total_slots += headers.values().map(Vec::len).sum::<usize>();
        let mut unique = BTreeSet::new();
        for (header, slots) in headers {
            let (frames, errors) = bank_frames(&program, header);
            for (stream, end) in errors {
                failures.push(format!(
                    "{fighter:?} {slots:?} {header:#x} -> {stream:#x}: {end:?}"
                ));
            }
            total_frames += frames.len();
            for frame in frames {
                let key = shape(&frame);
                if !unique.insert((key, frame.attr)) {
                    continue;
                }
                let tag = format!(
                    "{fighter:?} {slots:?} {:#x} code {:#06x} attr {:#x} {}x{}",
                    frame.addr,
                    frame.code,
                    frame.attr,
                    frame.width(),
                    frame.height()
                );
                if grid(key)
                    .tiles()
                    .into_iter()
                    .any(|(tile, _, _)| sprite_slot(tile) >= banked_assets.sprite_tiles())
                {
                    failures.push(format!("{tag}: tile is outside the loaded sprite bank"));
                    continue;
                }
                let pose = Pose::of_frame(&banked_assets.gfx3, &frame);
                if pose.drawn_count() == 0 {
                    failures.push(format!("{tag}: fully transparent"));
                } else if !fighter_palette(fighter, &frame) {
                    failures.push(format!("{tag}: palette does not belong to the fighter"));
                } else if !own.contains(&key) && !catalogue_shapes.contains(&key) {
                    let owners: Vec<_> = fighters
                        .into_iter()
                        .filter(|other| rom_shapes[&(*other as u8)].contains(&key))
                        .collect();
                    failures.push(format!(
                        "{tag}: not in fighter ROM poses or catalogue; ROM owners: {owners:?}"
                    ));
                }
            }
        }
        total_poses += unique.len();
    }
    println!("Audited {total_slots} slots, {total_frames} frame records and {total_poses} distinct poses across {} fighters", fighters.len());
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
