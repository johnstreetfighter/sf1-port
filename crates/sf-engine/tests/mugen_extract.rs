//! Offline M.U.G.E.N extractor (`#114`, `M1`).

use sf_engine::mugen_extract::{
    bridge_normals, decode_lz5, hardware_tile_from_indexed, missing_states, parse_act, parse_air,
    parse_cmd, parse_cns, parse_def, parse_sff, pose_keyframes, quantize_fixed, step_throw,
    throw_binds, validate_hardware_tile, write_sff_v1, write_sff_v2, Limb, NormalDef, Posture,
    Sprite, Strength, CPU_CAST, RECOIL_TILES,
};
use sf_engine::roster::TournamentFighter;

const AIR: &str = r#"
[Begin Action 10]
Clsn2: 2
 Clsn2[0] = -20, -80, 20, 0
 Clsn2[1] = -10, -40, 10, -10
Clsn1: 1
 Clsn1[0] = -15, -70, 30, -20
10,1, 0,0, 4
[Begin Action 820]
820,2, 12,-8, 3, 1
820,3, 4,-20, 3, 2
820,2, -6,-4, 3, 3
"#;

#[test]
fn parsers_boxes_fixed_point_and_six_button_bridge() {
    let def =
        parse_def("[Info]\nname = \"Retsu\"\n[Files]\nsprite = retsu.sff\nanim = retsu.air\n")
            .unwrap();
    assert_eq!(def.name, "Retsu");
    assert_eq!(def.sprite, "retsu.sff");

    let actions = parse_air(AIR).unwrap();
    let crouch = actions.iter().find(|a| a.id == 10).unwrap();
    let frame = &crouch.frames[0];
    assert_eq!(frame.hit.len(), 1);
    assert_eq!(frame.hit[0].depth, 1);
    assert!(frame.hurt.len() == 2 && frame.hurt.iter().all(|b| b.depth == 2));
    let union = frame.hurt_union.unwrap();
    assert_eq!(union.depth, 2);
    assert!(union.half_w >= frame.hurt[0].half_w);

    let motions = parse_cns("[Statedef 200]\n[State 200, vel]\ntype = VelSet\nx = 4.5\ny = -2.25\n[State 200, grav]\ntype = VelAdd\ny = .56\n").unwrap();
    assert_eq!(motions[0].vel_x, quantize_fixed(4.5));
    assert_eq!(motions[0].vel_x, 294_912);
    assert_eq!(motions[0].vel_y, quantize_fixed(-2.25));
    assert_eq!(motions[0].accel_y, quantize_fixed(0.56));

    let cmds =
        parse_cmd("[Command]\nname = \"Kiai\"\ncommand = ~D, DF, F, x\ntime = 12\n").unwrap();
    assert_eq!(cmds[0].name, "Kiai");
    assert_eq!(cmds[0].time, 12);

    let mut act = vec![0u8; 768];
    act[0] = 10;
    act[1] = 20;
    act[2] = 30;
    let palette = parse_act(&act).unwrap();
    assert_eq!(palette[0], [10, 20, 30]);

    let source = vec![
        normal(Posture::Stand, Limb::Punch, Strength::Light, 4, 3, 8, 20),
        normal(Posture::Stand, Limb::Punch, Strength::Heavy, 8, 5, 12, 60),
        normal(Posture::Stand, Limb::Kick, Strength::Light, 5, 3, 8, 22),
        normal(Posture::Stand, Limb::Kick, Strength::Heavy, 9, 5, 14, 64),
        normal(Posture::Crouch, Limb::Punch, Strength::Light, 3, 2, 7, 18),
        normal(Posture::Crouch, Limb::Punch, Strength::Heavy, 7, 4, 11, 50),
        normal(Posture::Jump, Limb::Kick, Strength::Light, 4, 6, 6, 24),
        normal(Posture::Jump, Limb::Kick, Strength::Heavy, 6, 8, 8, 58),
    ];
    let rom = [normal(
        Posture::Stand,
        Limb::Punch,
        Strength::Medium,
        6,
        4,
        9,
        40,
    )];
    let bridged = bridge_normals(&source, &rom);
    let mp = bridged
        .iter()
        .find(|n| n.limb == Limb::Punch && n.strength == Strength::Medium)
        .unwrap();
    assert_eq!(mp.damage, 40);
    let mk = bridged
        .iter()
        .find(|n| n.limb == Limb::Kick && n.strength == Strength::Medium)
        .unwrap();
    assert_eq!(mk.damage, (22 + 64) / 2);
    assert!(bridged.iter().any(|n| n.strength == Strength::Light));
    assert!(bridged.iter().any(|n| n.strength == Strength::Heavy));
    let crouch_mp = bridged
        .iter()
        .find(|n| n.posture == Posture::Crouch && n.strength == Strength::Medium)
        .unwrap();
    assert_eq!(crouch_mp.damage, (18 + 50) / 2);
    let jump_mk = bridged
        .iter()
        .find(|n| {
            n.posture == Posture::Jump && n.limb == Limb::Kick && n.strength == Strength::Medium
        })
        .unwrap();
    assert_eq!(jump_mk.damage, (24 + 58) / 2);

    assert_eq!(missing_states(&actions), vec!["jump", "block"]);
}

#[test]
fn sff_v1_pcx_and_v2_lz5_and_png_roundtrip() {
    let pixels: Vec<u8> = (0..256)
        .map(|i| if i % 5 == 0 { 0 } else { (i % 14) as u8 + 1 })
        .collect();
    let sprite = Sprite {
        group: 10,
        image: 1,
        width: 16,
        height: 16,
        axis_x: 8,
        axis_y: 15,
        pixels: pixels.clone(),
    };
    let v1 = parse_sff(&write_sff_v1(&[sprite.clone()])).unwrap();
    assert_eq!(v1[0].pixels, pixels);
    assert_eq!(v1[0].group, 10);

    let lz = decode_lz5(&[0x01, b'A', 0x00, 0x01], 4).unwrap();
    assert_eq!(lz, b"AAAA");
    let v2 = parse_sff(&write_sff_v2(
        &sprite,
        4,
        &sf_engine::mugen_extract::encode_lz5(&pixels),
    ))
    .unwrap();
    assert_eq!(v2[0].pixels, pixels);

    let png = sf_engine::mugen_extract::encode_png_indexed(&pixels, 16, 16);
    let from_png = parse_sff(&write_sff_v2(&sprite, 10, &png)).unwrap();
    assert_eq!(from_png[0].pixels, pixels);

    let keys = pose_keyframes(&parse_air(AIR).unwrap(), &v1);
    assert_eq!(keys[0].axis_x, 8);
    assert_eq!(keys[0].axis_y, 0);

    let mut indexed = [0u8; 256];
    indexed[0] = 1;
    let mut rgba = [[0u8; 4]; 16];
    rgba[1] = [255, 128, 16, 255];
    let tile = hardware_tile_from_indexed(&indexed, &rgba);
    validate_hardware_tile(&tile).unwrap();
    assert_eq!(tile.pixels[0], 1);
    assert_eq!(tile.pixels[1], 15);
    assert_eq!(tile.palette[1] & 0xf000, 0);
}

#[test]
fn ten_cpu_fighters_throw_with_existing_recoil_tiles() {
    let actions = parse_air(AIR).unwrap();
    let thrown = actions.iter().find(|a| a.id == 820).unwrap();
    for fighter in CPU_CAST {
        let binds = throw_binds(fighter, thrown);
        let frames = step_throw(&binds);
        assert_eq!(frames.len(), thrown.frames.len());
        for (frame, element) in frames.iter().zip(&thrown.frames) {
            assert_eq!(frame.dx, element.x as i16);
            assert_eq!(frame.dy, element.y as i16);
            assert_eq!(frame.flip_x, element.flip_x);
            assert_eq!(frame.flip_y, element.flip_y);
            assert!(
                RECOIL_TILES.contains(&frame.tile),
                "new tile {:04x}",
                frame.tile
            );
        }
    }
    let _ = TournamentFighter::Ryu;
}

fn normal(
    posture: Posture,
    limb: Limb,
    strength: Strength,
    startup: i32,
    active: i32,
    recovery: i32,
    damage: i32,
) -> NormalDef {
    NormalDef {
        posture,
        limb,
        strength,
        startup,
        active,
        recovery,
        damage,
    }
}
