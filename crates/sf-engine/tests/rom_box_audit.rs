//! Ticket #249 (`SP3`): the ROM box audit.
//!
//! Measures every ROM move the SF1+ fighters will use (Ryu's and Ken's table
//! moves, jump attacks and recoils, and every stream each boss's 1987 CPU can
//! start, plus each boss's move table): attack boxes against the pixels the
//! frame draws, hurtboxes against the drawn body, and startup and reach
//! against the research's numbers (`docs/research/sf1plus-reference-numbers.md`).
//! The measuring code is `sf_engine::box_audit`; the report text is
//! `sf_engine::box_audit_report`. Nothing here changes a box or a behaviour.
//!
//! Tests:
//! - `the_model_of_every_move_agrees_with_the_engine`: for every audited move
//!   the model's frame on each tick, its flags, raw boxes, grid and the boxes
//!   the diagnostic overlay reads on that grid equal what the translated
//!   engine holds after running the move tick by tick; so does the attack
//!   box's reach measured from the stance centre (the frame's x-offset, dx and
//!   half width, plus the steps the frames have carried the fighter); the
//!   strength-dependent startups too.
//! - `the_audit_covers_every_fighters_moves`: the shape of the audit (what is
//!   covered, no unknown stream routine, the one broken slot).
//! - `the_committed_report_is_current`: the report and its pictures are what
//!   the audit produces now. To rewrite them:
//!   `UPDATE_ROM_BOX_AUDIT=1 cargo test --release -p sf-engine --test rom_box_audit`
//!   (PowerShell: `$env:UPDATE_ROM_BOX_AUDIT=1; cargo test ...`).
//!
//! Every test skips without the local ROM set.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sf_engine::box_audit::*;
use sf_engine::box_audit_report::{self, PICTURE_DIR};
use sf_engine::fighter::{Program, P1_STRUCT, P2_STRUCT};
use sf_engine::machine::MachineState;
use sf_engine::roster::TournamentFighter;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const REPORT: &str = "../../docs/reference/rom-box-audit.md";
const REPORT_DIR: &str = "../../docs/reference";
const COMMAND: &str = "UPDATE_ROM_BOX_AUDIT=1 cargo test --release -p sf-engine --test rom_box_audit the_committed_report_is_current";

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn audit_all(a: &Assets) -> Vec<FighterAudit> {
    TournamentFighter::ALL.iter().map(|&f| audit_fighter(&a.maincpu, &a.gfx3, f)).collect()
}

// ---- The model against the engine ---------------------------------------------

/// What the engine holds after one tick of a move.
struct Tick {
    frame: u32,
    flags: u8,
    boxes: [u8; 16],
    /// Entry 0's code and attr, and the grid's rows and columns minus one.
    grid: (u16, u16, u8, u8),
    /// The loose, tight and attack boxes as the diagnostic overlay reads them
    /// (`overlay::extract_diagnostic_telemetry`), in grid pixels from the
    /// grid's top-left: (x0, y0, x1, y1).
    seen: [Option<(i32, i32, i32, i32)>; 3],
    /// The attack box's forward edge in pixels from the stance centre (the
    /// fighter's centre with the stance showing, before the move started),
    /// as the overlay reads it.
    reach_from_stance: Option<i32>,
}

/// Run the translated engine over one move, tick by tick: a move table slot
/// from its header, or a stream started the way the CPU's attack records
/// start one (the runner loads its first frame on the next tick). `strength`
/// is the button level (1 to 3) the move was started with, and `start` the
/// stance centre's x: the screen clamp (`clamp_x`) holds a fighter between
/// `0x60` and `0x180`, so a move that carries him far needs room to do it.
fn engine_trace(rom: &[u8], table: u32, slot: Option<u8>, stream: Option<u32>, strength: u8, start: u16, ticks: u32) -> Vec<Tick> {
    let mut prog = Program::power_on(rom.to_vec());
    let mut state = MachineState::new(vec![0; 0x1000], vec![0; 0x2000], vec![0; 0x800], 0, 0, 0, false, 0, 0);
    let mut bus = prog.bus(&mut state);
    let (me, other) = (0x400u16, 0xa00u16);
    let ps = P1_STRUCT;
    bus.wl(ps + 0xe, (other as u32) << 16 | me as u32);
    bus.ww(ps + 0x10, me);
    bus.wl(ps + 0x12, table);
    bus.wl(P2_STRUCT + 0xe, (me as u32) << 16 | other as u32);
    bus.wl(P2_STRUCT + 0x12, table);
    bus.set_eb(me, 0x37, 2);
    bus.set_el(me, 0x38, ps);
    bus.set_eb(other, 0x37, 4);
    bus.set_el(other, 0x38, P2_STRUCT);
    // The other fighter is far away and cannot be hit, so nothing connects.
    bus.set_eb(other, 8, 2);
    bus.set_eb(other, 0xf, 0x08);
    bus.set_ew(other, 6, 0x300);
    bus.set_ew(other, 4, 0x80);
    // Geki's teleport picks one of three frame groups by a random nibble:
    // take a roll that picks the first, the one the primary path follows.
    let roll = (0..16usize).find(|&r| rom[0x45a6a + r] == 0).unwrap_or(0);
    bus.wb(0xff8e4e, roll as u8);
    // The stance centre, the fighter's own position (the stance's x-offset is 0).
    bus.start_move_at(me, ps, 1, start);
    let stance_centre = bus.center_x(me) as i32;
    bus.start_move_at(me, ps, slot.unwrap_or(1), start);
    bus.set_eb(me, 0x34, strength);
    if let Some(s) = stream {
        bus.run_fighter(me, other);
        bus.set_eb(me, 0x34, strength);
        bus.set_eb(me, 9, 0xc2);
        bus.set_eb(me, 0xa, 0x0d);
        bus.set_el(me, 0x2c, s);
        bus.set_eb(me, 0x33, 0);
        bus.set_eb(me, 0xb, 0);
        bus.set_eb(me, 0xe, 0);
    }
    let mut out = Vec::new();
    for _ in 0..ticks {
        // A frame flagged 0x10 or 0x20 trades the two fighters' object
        // regions (0x3b828) and clears the flag: follow the fighter.
        let (cur, oth) = bus.player_entities(ps);
        bus.run_fighter(cur, oth);
        let (cur, _) = bus.player_entities(ps);
        let mut boxes = [0u8; 16];
        for (i, b) in boxes.iter_mut().enumerate() {
            *b = bus.eb(cur, 0x16 + i as u32);
        }
        let tele = sf_engine::overlay::extract_diagnostic_telemetry(rom, &*bus.work, &*bus.obj, false);
        let f = if tele.p1.entity_offset == cur { tele.p1 } else { tele.p2 };
        let (gx, gy) = (bus.ew(cur, 6) as i32, bus.ew(cur, 4) as i32);
        // Positions are 16-bit words: a jump above the screen's top wraps.
        let wrap = |v: i32| (v + 0x8000).rem_euclid(0x10000) - 0x8000;
        let rel = |b: &sf_engine::overlay::BoundingBox| {
            (wrap(b.left - gx), wrap(b.top - gy), wrap(b.right - gx), wrap(b.bottom - gy))
        };
        out.push(Tick {
            frame: bus.el(cur, 0x2c).wrapping_sub(0x20),
            flags: bus.eb(cur, 0xf),
            boxes,
            grid: (bus.ew(cur, 0), bus.ew(cur, 2), bus.eb(cur, 0xc), bus.eb(cur, 0xd)),
            seen: [Some(rel(&f.loose_hurtbox)), Some(rel(&f.tight_hurtbox)), f.attack_hitbox.as_ref().map(rel)],
            reach_from_stance: f.attack_hitbox.as_ref().map(|b| wrap(b.right - stance_centre)),
        });
    }
    out
}

fn start_of(m: &MoveAudit) -> (Option<u8>, Option<u32>) {
    match m.entry.slot {
        Some(s) => (Some(s), None),
        None => (None, Some(m.entry.stream)),
    }
}

#[test]
fn the_model_of_every_move_agrees_with_the_engine() {
    let Some(a) = assets() else { return };
    let rom = &a.maincpu;
    let mut bad = Vec::new();
    let mut ticks_checked = 0usize;
    for fa in audit_all(&a) {
        let f = fa.fighter;
        for m in &fa.moves {
            // The one stream the ROM cannot run: a pointer into a record.
            if m.walk.end == PathEnd::Broken {
                continue;
            }
            let (slot, stream) = start_of(m);
            let travels = (1..=m.total_ticks).map(|t| m.travel_at(t));
            let back = (-travels.clone().min().unwrap_or(0)).max(0) as u16;
            let start = 0xc0 + back;
            let tr = engine_trace(rom, f.move_table(), slot, stream, 1, start, m.total_ticks);
            assert_eq!(tr.len() as u32, m.total_ticks);
            for (i, t) in tr.iter().enumerate() {
                let tick = i as u32 + 1;
                let Some(fi) = m.frame_at(tick) else {
                    bad.push(format!("{} {} tick {tick}: the model's path ended", f.name(), m.entry.name));
                    break;
                };
                let rec = &m.walk.frames[fi];
                ticks_checked += 1;
                let mut b = [0u8; 16];
                b[..4].copy_from_slice(&rec.push);
                let sp = |x: &BoxSpec| [x.dy as u8, x.hh, x.dx as u8, x.hw];
                b[4..8].copy_from_slice(&sp(&rec.tight));
                b[8..12].copy_from_slice(&sp(&rec.loose));
                b[12..16].copy_from_slice(&sp(&rec.attack));
                let (w, h) = (rec.width(), rec.height());
                let want_seen = [
                    Some(rec.loose.rect(w, h)),
                    Some(rec.tight.rect(w, h)),
                    (rec.attack_active() && rec.attack.hw > 0 && rec.attack.hh > 0).then(|| rec.attack.rect(w, h)),
                ];
                // The engine's own frame address is only a stream frame's
                // (a header's `+0x2c` is its stream); the swap flags 0x10 and
                // 0x20 are cleared when the fighters trade regions.
                let ok = (rec.header || t.frame == rec.addr)
                    && (t.flags == rec.flags || t.flags == rec.flags & 0xcf)
                    && t.boxes == b
                    && t.grid == (rec.code, rec.attr, rec.rows, rec.cols)
                    && t.seen == want_seen
                    // Reach from the stance centre: the frame's x-offset, dx and
                    // half width, plus what the frame steps have carried the
                    // fighter by this tick.
                    && t.reach_from_stance == want_seen[2].map(|_| rec.reach() + m.travel_at(tick));
                if !ok {
                    bad.push(format!(
                        "{} {} tick {tick}: engine frame {:05x} flags {:02x} boxes {:02x?} reach {:?}; model frame {:05x} flags {:02x} boxes {:02x?} reach {} + travel {}",
                        f.name(),
                        m.entry.name,
                        t.frame,
                        t.flags,
                        t.boxes,
                        t.reach_from_stance,
                        rec.addr,
                        rec.flags,
                        b,
                        rec.reach(),
                        m.travel_at(tick)
                    ));
                    break;
                }
            }
        }
    }
    // The crouching punches that hold their first frames longer for a
    // medium or heavy button.
    let mut by_strength = 0usize;
    for fa in audit_all(&a) {
        for m in fa.moves.iter().filter(|m| m.startup_by_strength.is_some()) {
            for (k, &want) in m.startup_by_strength.unwrap().iter().enumerate() {
                let (slot, stream) = start_of(m);
                let tr = engine_trace(rom, fa.fighter.move_table(), slot, stream, k as u8 + 1, 0x100, want + 2);
                let got = tr.iter().position(|t| t.flags & 0x80 != 0).map(|p| p as u32 + 1);
                by_strength += 1;
                if got != Some(want) {
                    bad.push(format!("{} {} strength {}: engine startup {got:?}, model {want}", fa.fighter.name(), m.entry.name, k + 1));
                }
            }
        }
    }
    for b in bad.iter().take(20) {
        eprintln!("{b}");
    }
    assert!(bad.is_empty(), "{} moves differ from the engine", bad.len());
    assert!(ticks_checked > 20_000, "only {ticks_checked} ticks checked");
    assert!(by_strength >= 6, "only {by_strength} strength startups checked");
}

// ---- The shape of the audit ---------------------------------------------------

#[test]
fn the_audit_covers_every_fighters_moves() {
    let Some(a) = assets() else { return };
    let audits = audit_all(&a);
    assert_eq!(audits.len(), 12);
    for fa in &audits {
        let f = fa.fighter;
        let kit: Vec<&MoveAudit> = fa.moves.iter().filter(|m| m.entry.kind != MoveKind::HitReaction && m.entry.kind != MoveKind::KnockOut).collect();
        // Every fighter has his stance and an attack.
        assert!(kit.iter().any(|m| m.entry.slot == Some(1)), "{f:?}: stance");
        assert!(kit.iter().any(|m| m.startup.is_some()), "{f:?}: an attack");
        // 13 hit and 11 knock-out streams, some shared between entries.
        assert!(fa.moves.iter().any(|m| m.entry.kind == MoveKind::HitReaction), "{f:?}: hit streams");
        assert!(fa.moves.iter().any(|m| m.entry.kind == MoveKind::KnockOut), "{f:?}: knock-out streams");
        for m in &fa.moves {
            assert!(m.walk.unknown_calls.is_empty(), "{f:?} {}: unknown stream routines {:x?}", m.entry.name, m.walk.unknown_calls);
            assert!(!m.frames.is_empty(), "{f:?} {}: no frames", m.entry.name);
            assert_ne!(m.walk.end, PathEnd::Cap, "{f:?} {}: a path that never ends", m.entry.name);
            if m.walk.end == PathEnd::Broken {
                // Joe's crouch: the slot's stream pointer lands inside the
                // record at 0xe350 (docs/reference/art-inventory.md).
                assert_eq!((f, m.entry.slot), (TournamentFighter::Joe, Some(5)), "an unwalkable stream: {f:?} {}", m.entry.name);
            }
        }
        if f.is_shoto() {
            // All 31 table slots, seven jump attacks and three recoils.
            assert_eq!(kit.iter().filter(|m| m.entry.kind == MoveKind::Table).count(), 31, "{f:?}");
            assert_eq!(kit.iter().filter(|m| m.entry.kind == MoveKind::JumpAttack).count(), 7, "{f:?}");
            assert_eq!(kit.iter().filter(|m| m.entry.kind == MoveKind::Recoil).count(), 3, "{f:?}");
        } else {
            assert!(kit.iter().filter(|m| m.entry.kind == MoveKind::Table).count() >= 10, "{f:?}: his move table");
            assert!(kit.iter().filter(|m| m.entry.kind == MoveKind::CpuAttack).count() >= 4, "{f:?}: his CPU attack streams");
        }
    }
    // Reference points of the measurement itself.
    let ryu = &audits[0];
    let lp = ryu.moves.iter().find(|m| m.entry.slot == Some(0x0d)).unwrap();
    assert_eq!(lp.startup, Some(4), "Ryu's standing light punch: header 3 ticks, active on tick 4");
    let mk = ryu.moves.iter().find(|m| m.entry.slot == Some(0x15)).unwrap();
    assert_eq!(mk.startup, Some(22));
    // The Hadoken has no melee box but throws a projectile on tick 12.
    let hado = ryu.moves.iter().find(|m| m.entry.slot == Some(0x1a)).unwrap();
    assert_eq!((hado.startup, hado.spawn), (None, Some(12)));
    assert_eq!(hado.projectiles.len(), 1);
    // The sweep's active frame holds no drawn pixel in its box: a finding.
    let sweep = ryu.moves.iter().find(|m| m.entry.slot == Some(0x18)).unwrap();
    assert!(sweep.findings.iter().any(|f| f.kind == FindingKind::AttackFloating));
}

// ---- The report -------------------------------------------------------------------

fn write_png(path: &Path, pic: &Picture) {
    let file = std::fs::File::create(path).expect("create a picture");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), pic.width as u32, pic.height as u32);
    enc.set_color(png::ColorType::Indexed);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Best);
    enc.set_palette(PICTURE_PALETTE.iter().flatten().copied().collect::<Vec<u8>>());
    let mut w = enc.write_header().expect("png header");
    w.write_image_data(&pic.pixels).expect("png data");
}

fn read_png(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let file = std::fs::File::open(path).ok()?;
    let dec = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    Some((info.width, info.height, buf))
}

#[test]
fn the_committed_report_is_current() {
    let Some(a) = assets() else { return };
    let audits = audit_all(&a);
    let report = box_audit_report::render(&audits, COMMAND);
    let dir: PathBuf = Path::new(REPORT_DIR).join(PICTURE_DIR);
    let update = std::env::var_os("UPDATE_ROM_BOX_AUDIT").is_some();

    // The pictures: one per flagged frame, kept small.
    let mut pictures = Vec::new();
    for req in &report.pictures {
        let fa = audits.iter().find(|f| f.fighter == req.fighter).unwrap();
        let m = &fa.moves[req.move_index];
        let rec = &m.frames[req.frame].rec;
        let pose = Pose::of_frame(&a.gfx3, rec);
        let pic = draw_picture(&pose, rec, box_audit_report::picture_scale(rec));
        pictures.push((req.file.clone(), pic));
    }
    if update {
        std::fs::create_dir_all(&dir).expect("picture folder");
        for entry in std::fs::read_dir(&dir).expect("picture folder").flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
        for (file, pic) in &pictures {
            write_png(&dir.join(file), pic);
        }
        std::fs::write(REPORT, &report.markdown).expect("write the report");
        eprintln!("wrote {REPORT} and {} pictures", pictures.len());
    }

    let committed = std::fs::read_to_string(REPORT).unwrap_or_default().replace("\r\n", "\n");
    assert!(
        committed == report.markdown,
        "docs/reference/rom-box-audit.md is out of date; regenerate it with:\n{COMMAND}"
    );
    let expected: BTreeSet<String> = pictures.iter().map(|(f, _)| f.clone()).collect();
    let present: BTreeSet<String> = std::fs::read_dir(&dir)
        .map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    assert_eq!(present, expected, "the pictures beside the report are not the ones it refers to; regenerate it");
    let mut total = 0usize;
    for (file, pic) in &pictures {
        let (w, h, px) = read_png(&dir.join(file)).unwrap_or_else(|| panic!("{file} does not decode"));
        assert_eq!((w as usize, h as usize), (pic.width, pic.height), "{file}");
        assert_eq!(&px, &pic.pixels, "{file} is not the picture the audit draws now");
        let size = std::fs::metadata(dir.join(file)).unwrap().len() as usize;
        assert!(size < 20_000, "{file} is {size} bytes; pictures are kept small");
        total += size;
    }
    eprintln!("{} pictures, {} bytes", pictures.len(), total);
}
