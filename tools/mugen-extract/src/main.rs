//! Offline extractor (`#114`, `M1`). Reads a directory of text fixtures and
//! prints the parsed def, action count, and command count. It does not load
//! a M.U.G.E.N runtime.

use std::env;
use std::fs;
use std::path::Path;

use sf_engine::mugen_extract::{parse_air, parse_cmd, parse_def};

fn main() {
    let dir = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: mugen-extract <package-dir>");
        std::process::exit(2);
    });
    let dir = Path::new(&dir);
    let mut def_name = String::from("(no def)");
    let mut actions = 0usize;
    let mut commands = 0usize;
    let entries = fs::read_dir(dir).unwrap_or_else(|err| {
        eprintln!("cannot read {}: {err}", dir.display());
        std::process::exit(1);
    });
    for entry in entries.flatten() {
        let path = entry.path();
        let text = fs::read_to_string(&path).unwrap_or_default();
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("def") => {
                if let Ok(def) = parse_def(&text) {
                    def_name = def.name;
                }
            }
            Some("air") => {
                if let Ok(parsed) = parse_air(&text) {
                    actions = parsed.len();
                }
            }
            Some("cmd") => {
                if let Ok(parsed) = parse_cmd(&text) {
                    commands = parsed.len();
                }
            }
            _ => {}
        }
    }
    println!("{def_name}: {actions} actions, {commands} commands");
}
