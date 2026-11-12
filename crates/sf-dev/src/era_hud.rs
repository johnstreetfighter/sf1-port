//! Native local fight with developer controls for previewing the Era HUD.

use minifb::Key;
use sf_engine::era_preview::{EraPreview, PreviewEra};
use sf_engine::front::{fight::FightEvent, PlayerInputs};
use sf_host::input::LauncherPads;
use sf_host::window::{frame_buffer, open_window, poll_key_edge, present};
use sf_video::callouts::Side;
use sf_video::era_hud::GaugeArt;
use sf_video::romset::{load_assets, Assets};

use crate::{load_active_bindings, resolve_local, session::read_with_focus, MANIFEST, ROMS};

pub fn run(args: &[String]) -> i32 {
    let pick = |i: usize, default| match args.get(i) {
        None => Some(default),
        Some(slug) => PreviewEra::from_slug(slug),
    };
    let (Some(p1), Some(p2)) = (pick(0, PreviewEra::Sf6), pick(1, PreviewEra::Alpha)) else {
        let slugs: Vec<_> = PreviewEra::ALL.iter().map(|era| era.slug()).collect();
        eprintln!("era-hud: the eras are {}", slugs.join(", "));
        return 2;
    };
    let art = match args.get(2) {
        None => GaugeArt::Ruled,
        Some(slug) => match GaugeArt::from_slug(slug) {
            Some(art) => art,
            None => {
                eprintln!("era-hud: the gauge arts are ruled, blocks, plate");
                return 2;
            }
        },
    };
    let assets = match load_assets(resolve_local(ROMS), resolve_local(MANIFEST)) {
        Ok(assets) => assets,
        Err(error) => {
            eprintln!("era-hud: {error}");
            return 1;
        }
    };
    match show(&assets, [p1, p2], art) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("era-hud: {error:?}");
            1
        }
    }
}

fn show(assets: &Assets, mut eras: [PreviewEra; 2], art: GaugeArt) -> Result<(), FightEvent> {
    let mut preview = EraPreview::new(assets, eras[0], eras[1])?;
    preview.set_gauge_art(art);
    let bindings = load_active_bindings();
    let mut pads = LauncherPads::open();
    let mut window = open_window(&title(eras));
    let mut buffer = frame_buffer();
    let mut art_held = false;
    let mut cycle_held = [false; 2];
    let mut fixture_held = [false; 2];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let mut changed = false;
        for (index, key) in [Key::F5, Key::F6].into_iter().enumerate() {
            if poll_key_edge(&mut window, key, &mut cycle_held[index]) {
                eras[index] = eras[index].next();
                changed = true;
            }
        }
        if changed {
            preview.reset(assets, eras[0], eras[1])?;
            window.set_title(&title(eras));
        }
        for (side, key) in Side::BOTH.into_iter().zip([Key::F7, Key::F8]) {
            if poll_key_edge(&mut window, key, &mut fixture_held[side.index()]) {
                preview.fixture(side);
            }
        }
        if poll_key_edge(&mut window, Key::Tab, &mut art_held) {
            preview.set_gauge_art(preview.fight().gauge_art().next());
        }

        let focused = window.is_active();
        pads.poll(focused);
        let keys = [&bindings.p1, &bindings.p2];
        let pad_bindings = [&bindings.p1_pad, &bindings.p2_pad];
        let input = [0, 1].map(|side| {
            let keyboard = read_with_focus(keys[side], focused, |key| window.is_key_down(key));
            let held = pads.player(side, keyboard, pad_bindings[side]);
            PlayerInputs {
                directions: held.directions,
                buttons: held.buttons,
            }
        });
        let output = preview.step(assets, input);
        present(&mut window, &mut buffer, &output.frame);
        match output.event {
            None => {}
            Some(FightEvent::MatchOver) => preview.reset(assets, eras[0], eras[1])?,
            Some(error) => return Err(error),
        }
    }
    Ok(())
}

fn title([p1, p2]: [PreviewEra; 2]) -> String {
    format!(
        "sf-dev Era HUD | P1 {} vs P2 {} | F5/F6 style, F7/F8 call-out, Tab gauge art, Esc quits",
        p1.slug(),
        p2.slug()
    )
}
