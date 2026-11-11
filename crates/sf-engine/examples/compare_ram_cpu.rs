//! Debug aid: run the CPU-decision scene natively and compare object RAM
//! and the translated work-RAM variables with the reference capture's
//! work-RAM dumps frame by frame (local-only data); optionally run past the
//! interval to report the first untranslated branch.
//!
//!   cargo run -p sf-engine --release --example compare_ram_cpu -- \
//!       <reference capture> <last frame>

use std::path::Path;

use sf_engine::cpu::CpuMachine;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let reference = Path::new(&args[1]);
    let last: u32 = args[2].parse().unwrap();
    let rom = sf_video::romset::load_program_rom(
        "tools/reference/local/roms/sf",
        "tools/reference/rom-manifest.json",
    )
    .expect("program ROM");
    let mut m = CpuMachine::from_checkpoint_2389(reference, rom).expect("fixture");
    let vars: &[(&str, usize, usize)] = &[
        ("P1 struct +0..+0x16", 0x6b0, 0x6c6),
        ("P1 struct +0x1f..+0x2a", 0x6cf, 0x6da),
        ("CPU struct 0xffc664", 0x4664, 0x468e),
        ("sound queue 0x65e", 0x65e, 0x682),
        ("rng 0xe4a", 0xe4a, 0xe52),
        ("camera/drift 0x400c", 0x400c, 0x4012),
        ("scroll copies 0x4680", 0x4680, 0x4684),
    ];
    let mut reported = std::collections::BTreeSet::new();
    while m.state.frame < last {
        m.step();
        let frame = m.state.frame;
        if !m.unrecovered().is_empty() && reported.insert("unrecovered") {
            println!(
                "frame {frame}: untranslated branches {:x?}",
                m.unrecovered()
            );
        }
        let Ok(work) = std::fs::read(reference.join(format!("ram_{frame:06}_work.bin"))) else {
            continue;
        };
        let obj = &work[0x6000..0x8000];
        if let Some(i) = (0..0x2000).find(|&i| obj[i] != m.state.objectram[i]) {
            if reported.insert("objectram") {
                println!(
                    "frame {frame}: objectram differs first at 0xff{:04x}: ref {:02x} native {:02x}",
                    0xe000 + i, obj[i], m.state.objectram[i]
                );
            }
        }
        for (name, a, b) in vars {
            if let Some(i) = (*a..*b).find(|&i| work[i] != m.work_ram()[i]) {
                if reported.insert(name) {
                    println!(
                        "frame {frame}: {name} differs first at 0xff{:04x}: ref {:02x} native {:02x}",
                        0x8000 + i, work[i], m.work_ram()[i]
                    );
                }
            }
        }
    }
    if reported.is_empty() {
        println!("object RAM and translated variables identical through frame {last}");
    }
}
