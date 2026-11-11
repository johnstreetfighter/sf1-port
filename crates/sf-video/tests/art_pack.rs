//! Ticket #169 (`AP5`): the art pack and generated poses at the seam.
//!
//! - **The pack.** A pose written to the pack comes back pen for pen, as an
//!   indexed PNG with pen 15 transparent. The manifest is checked: a
//!   missing, damaged, changed or odd pack is refused with its reason.
//! - **Generated poses.** A recipe names its pack file by CRC-32. The pose
//!   loads into the bank from the pack, with any ROM parts drawn over it.
//! - **Fallbacks.** A fighter keeps his ROM art, with the reason, when a
//!   generated pose of his is missing from the pack or is not the approved
//!   file, or when his poses are not all approved. The others still load.
//!
//! The pack tests need nothing; the seam tests need the local ROMs and skip
//! without them. No generated pixels are used: the test packs hold the
//! ROM's own pens.

use std::path::{Path, PathBuf};

use sf_video::art::{load_art, load_art_from, rom_only, ArtSources, Grid, Pens, Source, TRANSPARENT};
use sf_video::art_pack::{decode_png, encode_png, write_pose, ArtPack};
use sf_video::art_review::Review;
use sf_video::romset::{crc32, load_assets, Assets};
use sf_video::sprite_bank::ROM_SPRITE_TILES;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const RYU: &str = include_str!("../../../art/recipes/ryu.json");
const RYU_REVIEW: &str = include_str!("../../../art/review/ryu.json");

const COLOURS: [u32; 16] = [
    0xff000000, 0xff664400, 0xff996644, 0xffcc9977, 0xffddaa88, 0xffddccaa, 0xffbbbbbb, 0xff777777, 0xff999999,
    0xff555555, 0xff444444, 0xff440044, 0xff660099, 0xff990088, 0xffbb00bb, 0xff111111,
];

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sf1-port-pack-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Every pen, transparent ones included, in a 20x18 pattern.
fn pattern() -> Pens {
    let (width, height) = (20, 18);
    Pens { width, height, pens: (0..width * height).map(|i| ((i * 7 + i / width) % 16) as u8).collect() }
}

fn rom() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).unwrap())
}

fn manifest_entry(dir: &Path, file: &str, bytes: &[u8]) {
    let text = format!(
        "{{ \"format\": 1, \"files\": {{ \"{file}\": {{ \"size\": {}, \"crc32\": \"{:08x}\" }} }} }}",
        bytes.len(),
        crc32(bytes)
    );
    std::fs::create_dir_all(dir.join(file).parent().unwrap()).unwrap();
    std::fs::write(dir.join(file), bytes).unwrap();
    std::fs::write(dir.join("manifest.json"), text).unwrap();
}

#[test]
fn a_pose_comes_back_from_the_pack_pen_for_pen() {
    let dir = scratch("roundtrip");
    let pens = pattern();
    let crc = write_pose(&dir, "sagat", "crouch", &pens, &COLOURS).unwrap();
    let other = Pens { width: 3, height: 2, pens: vec![1, 2, 3, 15, 15, 4] };
    write_pose(&dir, "sagat", "kick", &other, &COLOURS).unwrap();
    let pack = ArtPack::open(&dir).unwrap();
    assert_eq!(pack.len(), 2, "a second pose keeps the first");
    let back = pack.pose("sagat", "crouch").unwrap();
    assert_eq!((back.crc32, &back.pens), (crc, &pens));
    assert_eq!(pack.pose("sagat", "kick").unwrap().pens, other);
    assert!(pack.pose("ryu", "crouch").is_none());

    // The file is an indexed PNG in his colours, pen 15 fully clear and
    // every other pen fully opaque.
    let bytes = std::fs::read(dir.join("sagat/crouch.png")).unwrap();
    assert_eq!(crc32(&bytes), crc);
    let reader = png::Decoder::new(std::io::Cursor::new(&bytes)).read_info().unwrap();
    let info = reader.info();
    assert_eq!(info.color_type, png::ColorType::Indexed);
    assert_eq!(&info.palette.as_ref().unwrap()[..6], &[0x00, 0x00, 0x00, 0x66, 0x44, 0x00]);
    let trns = info.trns.as_ref().unwrap();
    assert!(trns[..15].iter().all(|&a| a == 255) && trns[15] == 0);

    // An editor may save the pens at 4 bits a pixel: they read the same.
    let mut four = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut four, pens.width as u32, pens.height as u32);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Four);
        enc.set_palette(vec![0u8; 48]);
        let packed: Vec<u8> = pens.pens.chunks(pens.width).flat_map(|row| row.chunks(2).map(|p| p[0] << 4 | p.get(1).copied().unwrap_or(0)).collect::<Vec<_>>()).collect();
        enc.write_header().unwrap().write_image_data(&packed).unwrap();
    }
    assert_eq!(decode_png(&four).unwrap(), pens);
    assert_eq!(decode_png(&encode_png(&pens, &COLOURS).unwrap()).unwrap(), pens);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_missing_damaged_or_odd_pack_is_refused_with_its_reason() {
    let refused = |dir: &Path| ArtPack::open(dir).unwrap_err();
    let dir = scratch("refused");
    assert!(refused(&dir).contains("manifest.json"), "a missing pack");

    write_pose(&dir, "sagat", "crouch", &pattern(), &COLOURS).unwrap();
    let file = dir.join("sagat/crouch.png");
    let mut bytes = std::fs::read(&file).unwrap();
    let last = bytes.len() - 20;
    bytes[last] ^= 0xff;
    std::fs::write(&file, &bytes).unwrap();
    assert!(refused(&dir).contains("does not match the manifest"), "a damaged file: {}", refused(&dir));

    std::fs::write(dir.join("manifest.json"), "{ \"format\": 2, \"files\": {} }").unwrap();
    assert!(refused(&dir).contains("format 2"));
    std::fs::write(dir.join("manifest.json"), "{ \"format\": 1, \"files\": ").unwrap();
    assert!(refused(&dir).contains("manifest.json:"), "an unreadable manifest");

    manifest_entry(&dir, "notes.txt", b"hello");
    assert!(refused(&dir).contains("not a <fighter>/<pose>.png file"));

    // An RGBA picture is not a pack pose, even listed with its true size.
    let mut rgba = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut rgba, 2, 2);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&[255; 16]).unwrap();
    }
    manifest_entry(&dir, "sagat/crouch.png", &rgba);
    assert!(refused(&dir).contains("not an indexed one"));

    // Nor is a pixel past the 16 pens.
    let mut bytes = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut bytes, 2, 1);
        enc.set_color(png::ColorType::Indexed);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_palette(vec![0u8; 17 * 3]);
        enc.write_header().unwrap().write_image_data(&[1, 16]).unwrap();
    }
    manifest_entry(&dir, "sagat/crouch.png", &bytes);
    assert!(refused(&dir).contains("past the 16 pens"));
    let _ = std::fs::remove_dir_all(dir);
}

/// A test fighter: one ROM pose, and one generated pose from the pack with
/// a ROM part drawn over it.
fn tester(crc: &str) -> String {
    format!(
        r#"{{ "fighter": "tester",
  "poses": [
    {{ "name": "stance", "rom": {{ "code": "0x0063", "large": true, "rows": 2, "cols": 1 }}, "anchor": [-32, -96] }},
    {{ "name": "made", "pack": {{ "crc32": "{crc}" }},
      "parts": [{{ "rom": {{ "code": "0x0480", "large": true, "rows": 1, "cols": 1 }}, "crop": [8, 8, 12, 10], "at": [4, 6] }}],
      "anchor": [-32, -96] }}
  ],
  "moves": [{{ "name": "m", "poses": ["stance", "made"] }}] }}"#
    )
}

/// A pack holding the tester's generated pose: Ryu's stance pens, standing
/// in for generated ones.
fn tester_pack(rom: &Assets, name: &str) -> (PathBuf, ArtPack, Pens, String) {
    let dir = scratch(name);
    let pens = Pens::from_grid(&rom.gfx3, &Grid { code: 0x63, large: true, rows: 2, cols: 1 });
    let crc = write_pose(&dir, "tester", "made", &pens, &COLOURS).unwrap();
    (dir.clone(), ArtPack::open(&dir).unwrap(), pens, format!("{crc:08x}"))
}

#[test]
fn a_generated_pose_loads_from_the_pack_with_rom_parts_over_it() {
    let Some(rom) = rom() else { return };
    let (dir, pack, base, crc) = tester_pack(&rom, "generated");
    let recipe = tester(&crc);
    let ryu = rom_only(RYU).unwrap();
    let sources = ArtSources { recipes: &[&ryu, &recipe], reviews: None, pack: Ok(&pack) };
    let art = load_art_from(&rom, &sources).unwrap();
    assert!(art.kept.is_empty(), "{:?}", art.kept);
    let made = art.catalogue("tester").unwrap().pose("made").unwrap().clone();
    assert_eq!((made.source.clone(), made.pack.as_deref()), (Source::Pack, Some(crc.as_str())));
    assert!(made.grid.code as usize >= ROM_SPRITE_TILES, "in the bank");
    assert!(art.describe().contains("tester (1 move; 1 ROM pose, 0 rebuilt, 1 generated)"), "{}", art.describe());

    // The pack's pens, with the part drawn over them where it is opaque.
    let part = Pens::from_grid(&rom.gfx3, &Grid { code: 0x480, large: true, rows: 1, cols: 1 });
    let mut want = base.clone();
    for y in 0..10 {
        for x in 0..12 {
            let pen = part.at(8 + x, 8 + y);
            if pen != TRANSPARENT {
                want.pens[(6 + y) * want.width + 4 + x] = pen;
            }
        }
    }
    assert_ne!(want, base, "the part changes something");
    let built = &art.built.iter().find(|(f, n, _)| f == "tester" && n == "made").unwrap().2;
    assert_eq!(built, &want);

    // The bank holds exactly those pens at the pose's grid.
    let drawn = Pens::from_grid(&rom.with_sprite_bank(&art.bank).gfx3, &made.grid);
    for y in 0..drawn.height {
        for x in 0..drawn.width {
            let expect = if x < want.width && y < want.height { want.at(x, y) } else { TRANSPARENT };
            assert_eq!(drawn.at(x, y), expect, "({x}, {y})");
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_fighter_whose_generated_pose_is_not_ready_keeps_his_rom_art() {
    let Some(rom) = rom() else { return };
    let (dir, pack, _, crc) = tester_pack(&rom, "fallback");
    let ryu = rom_only(RYU).unwrap();
    let ryu_alone = load_art(&rom, &[&ryu]).unwrap();
    let kept = |recipe: &str, pack: Result<&ArtPack, &str>| {
        let art = load_art_from(&rom, &ArtSources { recipes: &[&ryu, recipe], reviews: None, pack }).unwrap();
        assert!(art.catalogue("tester").is_none());
        assert!(art.catalogue("ryu").is_some(), "Ryu still loads");
        assert_eq!(art.bank, ryu_alone.bank, "and the bank holds only his");
        assert_eq!(art.kept.len(), 1);
        art.kept[0].clone()
    };
    let (who, why) = kept(&tester(&crc), Err("there is no sf1-port-art folder beside the ROMs"));
    assert_eq!(who, "tester");
    assert_eq!(why, "his generated pose made needs the art pack, and there is no sf1-port-art folder beside the ROMs");

    // A pack from before the pose was made: out of date.
    let older = scratch("fallback-older");
    write_pose(&older, "tester", "other", &pattern(), &COLOURS).unwrap();
    let older_pack = ArtPack::open(&older).unwrap();
    assert!(kept(&tester(&crc), Ok(&older_pack)).1.contains("the art pack has no made for him (the pack is out of date)"));

    // A pack whose file is not the approved one: mismatched.
    let why = kept(&tester("0badf00d"), Ok(&pack)).1;
    assert!(why.contains(&format!("is not the approved one (CRC-32 {crc}; the recipe wants 0badf00d)")), "{why}");
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(older);
}

#[test]
fn only_fighters_whose_poses_are_all_approved_load() {
    let Some(rom) = rom() else { return };
    let (dir, pack, _, crc) = tester_pack(&rom, "approved");
    let recipe = tester(&crc);
    let ryu = rom_only(RYU).unwrap();
    let load = |reviews: &[&str]| {
        load_art_from(&rom, &ArtSources { recipes: &[&ryu, &recipe], reviews: Some(reviews), pack: Ok(&pack) }).unwrap()
    };
    let catalogue = load_art_from(&rom, &ArtSources { recipes: &[&recipe], reviews: None, pack: Ok(&pack) })
        .unwrap()
        .catalogue("tester")
        .unwrap()
        .clone();
    let review = |approve: &[&str]| {
        let mut r = Review::for_catalogue(&catalogue, None);
        for p in approve {
            r.decide(p, true).unwrap();
        }
        serde_json::to_string(&r).unwrap()
    };

    let art = load(&[RYU_REVIEW]);
    assert!(art.catalogue("ryu").is_some(), "Ryu's committed review approves his set");
    assert_eq!(art.kept, [("tester".to_owned(), "his new poses have not been reviewed".to_owned())]);

    let half = review(&["stance"]);
    assert_eq!(load(&[RYU_REVIEW, &half]).kept[0].1, "1 of his 2 new poses are not approved yet");

    let all = review(&["stance", "made"]);
    let art = load(&[RYU_REVIEW, &all]);
    assert!(art.kept.is_empty() && art.catalogue("tester").is_some());

    // An approval given on another file of the pose does not carry over.
    let stale = all.replace(&crc, "0badf00d");
    assert_ne!(stale, all);
    assert_eq!(load(&[RYU_REVIEW, &stale]).kept[0].1, "1 of his 2 new poses are not approved yet");

    // A broken review file is a reason to keep the ROM's graphics.
    let broken = load_art_from(&rom, &ArtSources { recipes: &[&ryu], reviews: Some(&["{"]), pack: Ok(&pack) });
    assert!(broken.unwrap_err().starts_with("a review file:"));
    let _ = std::fs::remove_dir_all(dir);
}

/// Ticket #171 (`AP7`): a fighter whose set comes in halves (Sagat's ground
/// set, then his air set and specials) stays on his ROM art in the game
/// until the recipe says nothing is still to come, however much of it is
/// approved. The review page, which reads no reviews, still builds him.
#[test]
fn a_fighter_whose_set_is_not_finished_keeps_his_rom_art() {
    let Some(rom) = rom() else { return };
    let (dir, pack, _, crc) = tester_pack(&rom, "unfinished");
    let recipe = tester(&crc).replacen(r#""fighter": "tester","#, r#""fighter": "tester", "still_to_come": "his air set","#, 1);
    assert!(recipe.contains("still_to_come"));
    let for_review = load_art_from(&rom, &ArtSources { recipes: &[&recipe], reviews: None, pack: Ok(&pack) }).unwrap();
    assert!(for_review.kept.is_empty(), "built for review: {:?}", for_review.kept);
    let catalogue = for_review.catalogue("tester").unwrap();
    let mut all = Review::for_catalogue(catalogue, None);
    for p in ["stance", "made"] {
        all.decide(p, true).unwrap();
    }
    let all = serde_json::to_string(&all).unwrap();
    let art = load_art_from(&rom, &ArtSources { recipes: &[&recipe], reviews: Some(&[&all]), pack: Ok(&pack) }).unwrap();
    assert!(art.catalogue("tester").is_none() && art.bank.is_empty());
    assert_eq!(art.kept, [("tester".to_owned(), "his set is not finished, with his air set still to come".to_owned())]);
    let _ = std::fs::remove_dir_all(dir);
}
