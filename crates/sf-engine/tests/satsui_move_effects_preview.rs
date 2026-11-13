//! Verify the Raging Demon accent in the native Evil Ryu playtest.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use std::path::Path;

use sf_engine::combo::throw::THROW_MOVE;
use sf_engine::config::{EvilRyuPlaytest, MatchConfig};
use sf_engine::fighter::SF6_RAGING_DEMON;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs};
use sf_engine::roster::TournamentFighter::{Ryu, Sagat};
use sf_video::romset::load_assets;
use support::{load_fighter_art, pad, LP, MANIFEST, RIGHT, ROMS};

#[test]
fn raging_demon_adds_ink_to_the_glide_and_native_blackout() {
    if !Path::new(ROMS).is_dir() || !Path::new("../../assets/sf1-port-art").is_dir() {
        eprintln!("skipping: local ROM set or art pack not present");
        return;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROMs assemble");
    let sagat_art = load_fighter_art(&assets, Sagat);
    let mut config = MatchConfig::definitive();
    config.evil_ryu_playtest = Some(EvilRyuPlaytest::Sf6);
    let mut session = GameSession::new(assets, config)
        .with_sprite_bank(&sagat_art.bank)
        .with_player_tables(&sagat_art.catalogues);
    session.start_evil_ryu_playtest();
    for _ in 0..1500 {
        if session.fight_machine().map(flow_position) == Some(0x3eafa) {
            break;
        }
        session.step(FrameInputs::default());
    }
    assert_eq!(session.fight_machine().map(flow_position), Some(0x3eafa));

    let assets = session.fight_assets().unwrap();
    let mut current = session.fight().unwrap().clone();
    let bank = current.machine().data_bank().unwrap();
    assert!(bank.throw_entry(Ryu).is_some());
    assert!(bank.throw_entry(Sagat).is_some());
    let mut ready = current.save_netplay_snapshot(0);
    let ryu = &mut ready.machine.toggle_state.era.fighters[0];
    ryu.super_meter = 144;
    ryu.sub_hp = ryu.max_sub_hp / 4;
    // Let the normal move and collision runners handle the idle opponent.
    ready.setup.config.p2_fair_cpu = false;
    ready.machine.match_config.p2_fair_cpu = false;
    ready.machine.fair_cpu = None;
    current.restore_netplay_snapshot(&ready);
    let idle = PlayerInputs::default();
    for _ in 0..90 {
        let position = current.machine().diagnostic_telemetry();
        if position.p2.center_x - position.p1.center_x <= 75 {
            break;
        }
        current.step(assets, pad(RIGHT, None), idle, false);
    }
    current.step(assets, idle, idle, false);

    let mut preview = current.clone();
    assert_eq!(current.move_effects_preview(), Some(false));
    preview.toggle_move_effects();
    assert_eq!(preview.move_effects_preview(), Some(true));

    let command = [
        pad(0, Some(LP)),
        idle,
        pad(0, Some(LP)),
        pad(RIGHT, None),
        pad(0, Some(3)),
        pad(0, Some(2)),
    ];
    let mut saw_glide = false;
    let mut saw_rush = false;
    let mut saw_throw = false;
    let mut saw_blackout = false;
    let mut saw_blackout_accent = false;
    for input in command.into_iter().chain(std::iter::repeat_n(idle, 95)) {
        let before = current.step(assets, input, idle, false);
        let after = preview.step(assets, input, idle, false);
        assert_eq!(current.state_checksum(), preview.state_checksum());
        assert_eq!(current.fight_read(), preview.fight_read());

        let different = before.frame.argb != after.frame.argb;
        if current.fight_read().fighters[0].move_id == SF6_RAGING_DEMON {
            saw_rush = true;
            saw_glide |= different;
        }
        saw_throw |= saw_rush && current.fight_read().fighters[0].move_id == THROW_MOVE;
        // The blackout leaves only the native spark on the solid backdrop.
        let pixels = &before.frame.argb;
        if saw_throw
            && pixels
                .iter()
                .filter(|&&pixel| pixel != pixels[0])
                .take(1500)
                .count()
                < 1500
        {
            saw_blackout = true;
            saw_blackout_accent |= different;
        }
        if saw_glide && saw_blackout_accent {
            break;
        }
    }
    assert!(saw_rush, "Raging Demon command activated");
    assert!(saw_glide, "Raging Demon rush has a move accent");
    assert!(saw_throw, "Raging Demon grabbed the opponent");
    assert!(saw_blackout, "Raging Demon reached the blackout");
    assert!(saw_blackout_accent, "blackout hits add preview pixels");
}
