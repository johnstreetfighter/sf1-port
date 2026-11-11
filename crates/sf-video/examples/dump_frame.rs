//! Render one checkpoint frame of the reference capture and dump raw ARGB
//! bytes (little-endian u32 per pixel, 384x224) for visual diffing.
//!
//! Scene state per frame is the recovered register state documented in
//! docs/reference/first-frame.md.
//!
//! cargo run -p sf-video --example dump_frame -- <frame> <out.argb>

use sf_video::render::{render_frame, HEIGHT, WIDTH};
use sf_video::romset::load_assets;
use sf_video::scene::SceneState;

fn main() {
    let frame: u32 = std::env::args()
        .nth(1)
        .expect("usage: dump_frame <frame> <out.argb>")
        .parse()
        .expect("frame number");
    let out = std::env::args()
        .nth(2)
        .expect("usage: dump_frame <frame> <out.argb>");

    let capture = "../../tools/reference/local/captures/fight-run-a";
    let video = std::fs::read(format!("{capture}/ram_{frame:06}_video.bin")).unwrap();
    let palette = std::fs::read(format!("{capture}/ram_{frame:06}_palette.bin")).unwrap();
    let work = std::fs::read(format!("{capture}/ram_{frame:06}_work.bin")).unwrap();
    let assets = load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    )
    .unwrap();

    // Write-only register state per frame, recovered from register taps.
    let (bg_scroll, fg_scroll, active) = match frame {
        790 | 794 | 1500 => (0, 0, 0x08),
        2280 => (0x1800, 0x1d40, 0xe8),
        other => panic!("no recovered scene state for frame {other}"),
    };
    let scene = SceneState {
        videoram: video,
        objectram: work[0x6000..0x8000].to_vec(),
        paletteram: palette,
        bg_scroll,
        fg_scroll,
        active,
        flip_screen: false,
        bg_page: 0,
        fg_page: 0,
    };

    let frame = render_frame(&assets, &scene);
    let mut bytes = Vec::with_capacity(WIDTH * HEIGHT * 4);
    for px in &frame.argb {
        bytes.extend_from_slice(&px.to_le_bytes());
    }
    std::fs::write(&out, &bytes).unwrap();
    eprintln!("wrote {out} ({}x{} ARGB)", WIDTH, HEIGHT);
}
