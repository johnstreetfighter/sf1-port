//! The development preview of two era sessions under the era HUD (`#217`, `EH1`).

use sf_engine::era_preview::{EraPreview, PreviewEra, PreviewSide};
use sf_video::era_hud::{GaugeArt, GaugeRow, GaugeState};
use sf_video::render::{HEIGHT, WIDTH};

#[test]
fn every_pair_of_eras_runs_under_the_hud() {
    for p1 in PreviewEra::ALL {
        for p2 in PreviewEra::ALL {
            let mut preview = EraPreview::new(p1, p2);
            for side in [&mut preview.p1, &mut preview.p2] {
                side.hit();
                side.alt();
                side.spend();
            }
            preview.tick();
            let hud = preview.hud();
            assert_eq!(hud.p1.is_none(), p1 == PreviewEra::Sf1, "{p1:?} vs {p2:?}");
            assert_eq!(hud.p2.is_none(), p2 == PreviewEra::Sf1, "{p1:?} vs {p2:?}");
            let frame = preview.frame(GaugeArt::Ruled, None);
            assert_eq!(frame.argb.len(), WIDTH * HEIGHT);
            // The backdrop, or the backdrop under a super's dim (EH4).
            let backdrop = |p: u32| p == 0xff_18_14_28 || p == 0xff_0c_0a_14;
            let corner_drawn = |x0: usize| (200..216).any(|y| (x0..x0 + 24).any(|x| !backdrop(frame.argb[y * WIDTH + x])));
            assert_eq!(corner_drawn(8), p1 != PreviewEra::Sf1, "{p1:?}'s corner");
            assert_eq!(corner_drawn(WIDTH - 32), p2 != PreviewEra::Sf1, "{p2:?}'s corner");
        }
    }
}

#[test]
fn the_preview_actions_drive_each_era_model() {
    let mut alpha = PreviewSide::new(PreviewEra::Alpha);
    alpha.hit();
    alpha.alt();
    assert!(matches!(alpha.gauge().unwrap().main, GaugeRow::Level { state: GaugeState::Draining, .. }));

    let mut sf3 = PreviewSide::new(PreviewEra::Sf3);
    let length = |side: &PreviewSide| match side.gauge().unwrap().main {
        GaugeRow::Level { length, .. } => length.px(96),
        row => panic!("{row:?}"),
    };
    assert_eq!(length(&sf3), 96);
    sf3.alt();
    assert_eq!(length(&sf3), 64, "Super Art II's shorter bar");

    let mut sf6 = PreviewSide::new(PreviewEra::Sf6);
    for _ in 0..6 {
        sf6.alt();
    }
    assert!(matches!(sf6.gauge().unwrap().main, GaugeRow::Stock { state: GaugeState::Greyed, .. }));
    sf6.alt();
    assert!(matches!(sf6.gauge().unwrap().main, GaugeRow::Stock { full: 6, .. }), "the next round refills");

    let mut st = PreviewSide::new(PreviewEra::St);
    for _ in 0..6 {
        st.hit();
    }
    assert!(matches!(st.gauge().unwrap().main, GaugeRow::Stock { full: 1, .. }));
    st.spend();
    assert!(matches!(st.gauge().unwrap().main, GaugeRow::Stock { full: 0, .. }));
    assert_eq!(PreviewEra::from_slug("sf6"), Some(PreviewEra::Sf6));
    assert_eq!(PreviewEra::Sf6.next(), PreviewEra::Sf1);
}
