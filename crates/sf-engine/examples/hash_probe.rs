//! Debug aid: hash a reference capture's checkpoint dumps as the native
//! renderer sees them, with the palette of the same or the next frame, to
//! settle which palette a frame's screen hash reflects (local-only data).
//!
//!   cargo run -p sf-engine --release --example hash_probe -- <capture> <frame>

use std::path::Path;

use sf_engine::capture::screen_hash;
use sf_video::scene::SceneState;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = Path::new(&args[1]);
    let frame: u32 = args[2].parse().unwrap();
    let assets = sf_video::romset::load_assets(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
    )
    .expect("ROM regions assemble");
    let read = |f: u32, suffix: &str| {
        std::fs::read(dir.join(format!("ram_{f:06}_{suffix}.bin"))).expect("dump")
    };
    let work = read(frame, "work");
    let reg = |a: usize| u16::from_be_bytes([work[a], work[a + 1]]);
    let scene = |palette: Vec<u8>| SceneState {
        videoram: read(frame, "video"),
        objectram: work[0x6000..0x8000].to_vec(),
        paletteram: palette,
        bg_scroll: reg(0x4682),
        fg_scroll: reg(0x4680),
        active: work[0x682],
        flip_screen: false,
        bg_page: 0,
        fg_page: 0,
    };
    for (label, f) in [
        ("same frame's palette", frame),
        ("next frame's palette", frame + 1),
    ] {
        let hash = screen_hash(&sf_video::render::render_frame(
            &assets,
            &scene(read(f, "palette")),
        ));
        println!("render(S_{frame}) with the {label}: {hash:08x}");
    }
}
