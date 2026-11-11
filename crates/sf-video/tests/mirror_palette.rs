//! Ticket #250 (`SP16`): the mirror palettes' data format. No ROM needed.

use sf_video::mirror_palette::{argb, colour_text, committed, for_fighter, parse_colour, MirrorPalette, FILES, PENS};

fn file(from: u8, slot: u8) -> serde_json::Value {
    serde_json::json!({ "fighter": "ryu", "note": "a note", "from": from, "slot": slot, "pens": vec!["#112233"; PENS] })
}

fn parse(v: &serde_json::Value) -> Result<MirrorPalette, String> {
    MirrorPalette::parse(&v.to_string())
}

#[test]
fn the_twelve_committed_files_parse_and_each_names_its_own_fighter() {
    assert_eq!(committed().len(), 12);
    for (name, _) in FILES {
        let p = for_fighter(name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(p.fighter, name);
        assert!(p.slot != p.from && p.slot < 16 && p.from < 16);
        assert_eq!(for_fighter(&name.to_uppercase()), Some(p), "a name matches in any case");
    }
    assert!(for_fighter("akuma").is_none());
}

#[test]
fn colours_are_twelve_bit_in_the_text_the_palette_command_prints() {
    // `#664400` is the hardware's word 0x640 as `sf-art palette` prints it.
    assert_eq!(parse_colour("#664400"), Ok(0x640));
    assert_eq!(colour_text(0x640), "#664400");
    assert_eq!(argb(0x640), 0xff66_4400);
    for word in [0x000u16, 0xfff, 0x123, 0xabc] {
        assert_eq!(parse_colour(&colour_text(word)), Ok(word));
    }
    assert!(parse_colour("#665400").unwrap_err().contains("12-bit"), "a colour the hardware cannot show");
    assert!(parse_colour("664400").is_err());
    assert!(parse_colour("#6644").is_err());
    assert!(parse_colour("#66440g").is_err());
}

#[test]
fn a_file_is_checked_for_what_the_engine_relies_on() {
    let ok = parse(&file(0, 14)).unwrap();
    assert_eq!((ok.from, ok.slot, ok.pens[3]), (0, 14, 0x123));

    let bad = |v: serde_json::Value, why: &str| {
        let e = parse(&v).unwrap_err();
        assert!(e.contains(why), "{e} should mention {why}");
    };
    bad(file(0, 16), "0 to 15");
    bad(file(16, 14), "0 to 15");
    bad(file(13, 14), "sparks");
    bad(file(0, 13), "sparks");
    bad(file(3, 3), "stands in");
    let mut short = file(0, 14);
    short["pens"] = serde_json::json!(["#112233"]);
    bad(short, "pens");
    let mut odd = file(0, 14);
    odd["pens"] = serde_json::json!(vec!["#112234"; PENS]);
    bad(odd, "12-bit");
    assert!(MirrorPalette::parse("{").is_err());
    let mut extra = file(0, 14);
    extra["swaps"] = serde_json::json!([]);
    assert!(parse(&extra).is_err(), "unknown fields (the old effect swaps) are refused");
}

#[test]
fn the_digest_changes_with_any_colour_slot_or_palette() {
    let base = parse(&file(0, 14)).unwrap();
    assert_eq!(base.digest(), parse(&file(0, 14)).unwrap().digest());
    assert_eq!(base.digest().len(), 8);
    let mut one = file(0, 14);
    one["pens"][7] = serde_json::json!("#112244");
    for other in [file(0, 15), file(1, 14), one] {
        assert_ne!(base.digest(), parse(&other).unwrap().digest());
    }
    // The note is for people: it is not the colours.
    let mut noted = file(0, 14);
    noted["note"] = serde_json::json!("another note");
    assert_eq!(base.digest(), parse(&noted).unwrap().digest());
}
