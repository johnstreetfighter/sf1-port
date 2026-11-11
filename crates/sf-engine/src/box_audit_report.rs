//! The ROM box audit's report (spec #246, `SP3` #249): the Markdown of
//! `docs/reference/rom-box-audit.md` and the list of pictures that go beside
//! it, written from the measurements of [`crate::box_audit`].
//!
//! The text is generated, so every number in it comes from the ROM and moves
//! with the code that measures it. `tests/rom_box_audit.rs` regenerates the
//! file (`UPDATE_ROM_BOX_AUDIT=1`) or compares it with the committed one.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::box_audit::*;
use crate::roster::TournamentFighter;

/// The folder, beside the report, the pictures go in.
pub const PICTURE_DIR: &str = "rom-box-audit";

/// One picture the report refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PictureRequest {
    /// The file name inside [`PICTURE_DIR`].
    pub file: String,
    pub fighter: TournamentFighter,
    /// Index into the fighter's audited moves.
    pub move_index: usize,
    /// Index into the move's frames.
    pub frame: usize,
}

/// The finished report.
#[derive(Clone, Debug)]
pub struct Report {
    pub markdown: String,
    pub pictures: Vec<PictureRequest>,
}

/// The pictures' scale: 3 picture pixels per grid pixel for a pose up to 96
/// pixels across, 2 for a larger one, so a picture stays about 300 pixels
/// across and a few kilobytes.
pub fn picture_scale(rec: &FrameRecord) -> usize {
    if rec.width().max(rec.height()) > 96 {
        2
    } else {
        3
    }
}

fn is_kit(kind: MoveKind) -> bool {
    !matches!(kind, MoveKind::HitReaction | MoveKind::KnockOut)
}

fn short_kind(m: &MoveEntry) -> String {
    let classes: Vec<&str> = ["near", "middle", "far"]
        .into_iter()
        .filter(|c| m.notes.iter().any(|n| n.starts_with(c)))
        .map(|c| &c[..1])
        .collect();
    match m.kind {
        MoveKind::Table => "table".to_string(),
        MoveKind::JumpAttack => "jump".to_string(),
        MoveKind::CpuAttack => format!("CPU attack {}", classes.join("")),
        MoveKind::CpuReaction => "CPU reaction".to_string(),
        MoveKind::Recoil => "recoil".to_string(),
        MoveKind::HitReaction => "hit".to_string(),
        MoveKind::KnockOut => "knock-out".to_string(),
    }
}

fn display_name(m: &MoveEntry) -> String {
    let slot = m.slot.map(|s| format!("[{s:#04x}] ")).unwrap_or_default();
    format!("{slot}{}", m.name)
}

fn stream_text(m: &MoveEntry) -> String {
    if m.stream == 0 {
        "header".to_string()
    } else {
        format!("`{:#07x}`", m.stream)
    }
}

fn dir_letter(d: usize) -> char {
    ['F', 'B', 'U', 'D'][d]
}

fn startup_text(m: &MoveAudit) -> String {
    match (m.startup, m.startup_by_strength, m.spawn) {
        (Some(s), Some(k), _) => format!("{s} ({}/{} M/H)", k[1], k[2]),
        (Some(s), None, _) => s.to_string(),
        (None, _, Some(sp)) => format!("spawn {sp}"),
        _ => "-".to_string(),
    }
}

/// The audited moves of every fighter, with their indices.
fn kit(audits: &[FighterAudit]) -> Vec<(usize, usize, &MoveAudit)> {
    let mut out = Vec::new();
    for (fi, fa) in audits.iter().enumerate() {
        for (mi, m) in fa.moves.iter().enumerate() {
            if is_kit(m.entry.kind) {
                out.push((fi, mi, m));
            }
        }
    }
    out
}

fn picture_file(fa: &FighterAudit, m: &MoveAudit, frame: usize) -> String {
    let id = match m.entry.slot {
        Some(s) => format!("slot{s:02x}"),
        None => format!("s{:05x}", m.entry.stream),
    };
    format!("{}-{id}-f{frame}.png", fa.fighter.name().to_lowercase())
}

fn fmt_q(v: &[f64], digits: usize) -> String {
    match quartiles(v) {
        None => "-".to_string(),
        Some([a, b, c]) => {
            let max = v.iter().cloned().fold(f64::MIN, f64::max);
            let min = v.iter().cloned().fold(f64::MAX, f64::min);
            format!("{} | {:.d$} | {:.d$} | {:.d$} | {:.d$} | {:.d$}", v.len(), min, a, b, c, max, d = digits)
        }
    }
}

/// Write the report for `audits` (every fighter, in roster order).
pub fn render(audits: &[FighterAudit], command: &str) -> Report {
    let mut o = String::new();
    let mut pictures: Vec<PictureRequest> = Vec::new();
    let kit = kit(audits);
    let all_frames: usize = audits.iter().flat_map(|f| &f.moves).filter(|m| is_kit(m.entry.kind)).map(|m| m.frames.len()).sum();
    let count_kind = |k: MoveKind| kit.iter().filter(|(_, _, m)| m.entry.kind == k).count();
    let react_streams: usize = audits.iter().flat_map(|f| &f.moves).filter(|m| !is_kit(m.entry.kind)).count();
    let attack_moves: Vec<_> = kit.iter().filter(|(_, _, m)| m.startup.is_some()).collect();
    let reach: Vec<f64> = attack_moves.iter().filter_map(|(_, _, m)| m.reach).map(|r| r as f64).collect();
    let over: Vec<f64> = attack_moves.iter().filter_map(|(_, _, m)| m.worst_overreach.map(|w| w.0 as f64)).collect();
    let startup: Vec<f64> = attack_moves.iter().filter_map(|(_, _, m)| m.startup).map(|s| s as f64).collect();
    let active: Vec<f64> = attack_moves.iter().map(|(_, _, m)| m.active_ticks as f64).collect();
    let loose: Vec<f64> = kit.iter().filter_map(|(_, _, m)| m.loose_overreach.map(|w| w.0 as f64)).collect();
    let excess: Vec<f64> = kit.iter().filter_map(|(_, _, m)| m.uncovered_excess.map(|w| w.0 * 100.0)).collect();
    let stance: Vec<f64> = audits.iter().map(|a| a.stance_share * 100.0).collect();
    // Tukey's outer fence (Q3 + 1.5 IQR) and a percentile, for the thresholds' reasons.
    let fence = |v: &[f64]| quartiles(v).map_or(0.0, |[a, _, c]| c + 1.5 * (c - a));
    let pct = |v: &[f64], p: f64| {
        let mut w = v.to_vec();
        w.sort_by(|a, b| a.partial_cmp(b).unwrap());
        w.get(((w.len().max(1) - 1) as f64 * p) as usize).copied().unwrap_or(0.0)
    };

    let _ = writeln!(o, "# ROM box audit\n");
    let _ = writeln!(
        o,
        "Generated by `crates/sf-engine/tests/rom_box_audit.rs` (issue #249, `SP3` of spec #246) from the program ROM and the sprite ROM, with the measuring code in `crates/sf-engine/src/box_audit.rs`. It is read only: no box, move or behaviour anywhere is changed, and every adjustment it may lead to lives in the Balanced roster layer, tuned in SP20 after the maintainer has seen this report. Regenerate it with:\n\n```\n{command}\n```\n"
    );
    let _ = writeln!(
        o,
        "The audit measures every ROM move the SF1+ fighters will use: the 31 table moves of Ryu and Ken (with their jump attacks and blocked-kick recoils), and every stream each of the ten bosses' 1987 CPU can start, plus each boss's own move table. Each frame's **attack boxes** are measured against the pixels the frame draws, its **hurtboxes** against the drawn body, and each move's **startup and reach** against the numbers in `docs/research/sf1plus-reference-numbers.md`. Outliers are listed with their numbers and a picture of the frame's boxes over its pose.\n"
    );

    // ---- Summary -----------------------------------------------------------
    let _ = writeln!(o, "## Summary\n");
    let flagged: Vec<_> = kit.iter().filter(|(_, _, m)| !m.findings.is_empty()).collect();
    let total_findings: usize = kit.iter().map(|(_, _, m)| m.findings.len()).sum();
    let _ = writeln!(
        o,
        "- **Covered:** {} moves of the 12 fighters ({} table moves, {} Ryu/Ken jump attacks, {} boss CPU attack streams, {} boss CPU reaction streams, {} blocked-kick recoils), {} distinct frame records; {} of the moves have an attack box or throw a projectile. Their {} hit-reaction and knock-out streams are measured too and summarised in their own section.",
        kit.len(),
        count_kind(MoveKind::Table),
        count_kind(MoveKind::JumpAttack),
        count_kind(MoveKind::CpuAttack),
        count_kind(MoveKind::CpuReaction),
        count_kind(MoveKind::Recoil),
        all_frames,
        kit.iter().filter(|(_, _, m)| m.has_attack()).count(),
        react_streams
    );
    let _ = writeln!(
        o,
        "- **Outliers:** {} findings on {} moves, each pictured. By kind: {}.",
        total_findings,
        flagged.len(),
        FindingKind::ALL
            .iter()
            .map(|k| format!(
                "{} {}",
                kit.iter().flat_map(|(_, _, m)| &m.findings).filter(|f| f.kind == *k).count(),
                k.label()
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(
        o,
        "- **How the measurements were checked:** the model of a move (which frame is on screen each tick, its flags, its raw boxes, its grid, where the diagnostic overlay draws its boxes on that grid, and for every attack tick the attack box's forward edge in pixels from the stance centre, steps included) was compared tick by tick with the translated engine running the same move, on every tick of every move below (`the_model_of_every_move_agrees_with_the_engine` in `tests/rom_box_audit.rs`), and the strength-dependent startups of the crouching punches too. They agree exactly.\n"
    );

    // Headlines.
    let _ = writeln!(o, "### What stands out\n");
    let worst = |k: FindingKind| {
        let all = kit
            .iter()
            .flat_map(|(fi, _, m)| m.findings.iter().filter(move |f| f.kind == k).map(move |f| (*fi, *m, f)));
        if k == FindingKind::FastStartup {
            all.min_by_key(|(_, _, f)| f.value)
        } else {
            all.max_by_key(|(_, _, f)| f.value)
        }
    };
    for k in FindingKind::ALL {
        match worst(k) {
            Some((fi, m, f)) => {
                let _ = writeln!(
                    o,
                    "- **{}** ({}): the {} is {} {} {}: {}.",
                    capitalise(k.label()),
                    kit.iter().flat_map(|(_, _, m)| &m.findings).filter(|f| f.kind == k).count(),
                    if k == FindingKind::FastStartup { "fastest" } else { "worst" },
                    audits[fi].fighter.name(),
                    m.entry.name,
                    stream_text(&m.entry),
                    f.detail
                );
            }
            None => {
                let _ = writeln!(o, "- **{}**: none.", capitalise(k.label()));
            }
        }
    }
    {
        let frames: Vec<&FrameAudit> =
            audits.iter().flat_map(|a| &a.moves).filter(|m| !is_kit(m.entry.kind)).flat_map(|m| &m.frames).collect();
        let none = frames.iter().filter(|f| f.point_hurtbox).count();
        let _ = writeln!(
            o,
            "- **Hit reactions and knock-outs** have a zero-size (point) loose hurtbox on {none} of their {} frames and the invulnerable flag on {} of them. In 1987 the invulnerable flag is what keeps a reacting fighter from being hit again (the judge skips a flagged target); under the modern rules (true combos) a reacting fighter must be hittable, so the reaction frames need real boxes written for them, not only the flag cleared.",
            frames.len(),
            frames.iter().filter(|f| f.rec.invulnerable()).count()
        );
    }
    {
        let quirks: Vec<String> = audits
            .iter()
            .filter(|a| a.stance_tight_out > 0)
            .map(|a| format!("{} ({} px)", a.fighter.name(), a.stance_tight_out))
            .collect();
        if !quirks.is_empty() {
            let _ = writeln!(
                o,
                "- **Critical boxes outside the hurtbox, fighter-wide:** the stance of {} already has its critical box sticking out of its hurtbox, so that figure is each such fighter's baseline for the finding of the same name.",
                quirks.join(", ")
            );
        }
    }
    let _ = writeln!(
        o,
        "- **Hurtboxes in general:** the 1987 hurtbox is a torso box, not an outline. In their stances the twelve fighters leave {} of their drawn pixels outside the loose hurtbox (arms, legs, anything extended), so \"drawn body not covered\" is flagged against each fighter's own stance, not against zero.\n",
        {
            let s: Vec<f64> = audits.iter().map(|a| a.stance_share * 100.0).collect();
            format!(
                "{:.0}% to {:.0}%",
                s.iter().cloned().fold(f64::MAX, f64::min),
                s.iter().cloned().fold(f64::MIN, f64::max)
            )
        }
    );

    // ---- Scope ---------------------------------------------------------------
    let _ = writeln!(o, "## What is covered, and the open question\n");
    let _ = writeln!(
        o,
        "- **Ryu and Ken:** every move in their tables (slots `0x00` to `0x1e`: intro, stance, walks, blocks, crouches, jumps, win, the standing and crouching normals, sweep, Shoryuken, Hadoken, Tatsumaki), their seven jump-attack streams each (from the apex tables `0x45386`/`0x4539e` and `0x4545c`/`0x45474`) and their three blocked-kick recoils."
    );
    let _ = writeln!(
        o,
        "- **The ten bosses:** their move tables (slots `0x00` to `0x0c`) and **every stream the 1987 CPU can start**: the attack records (the near, middle and far tables of every difficulty and variant image, `0x4044`, `0x4048`, `0x404c` of the per-opponent arrays) and the reaction records (airborne and ground, `0x403c`, `0x4040`, the CPU's guards, jumps and the like). A reaction stream with an active frame is an attack and is measured as one."
    );
    let _ = writeln!(
        o,
        "- **Open question (not decided here):** which of a boss's streams become his player moves is for SP4 to SP12 to decide, and the project has not decided it yet. This audit therefore covers **all** of a fighter's attack streams, so the answer cannot leave an unmeasured move behind. Each row says which range class of the CPU's records (`n` near, `m` middle, `f` far) starts the stream and which button the roster binds it to today (`L`, `M`, `H` punch or kick; informational only).\n"
    );
    let _ = writeln!(
        o,
        "- **Startup counts differ by where a stream starts.** A Ryu or Ken table move, and a boss's table slot, starts with its header pose and its startup counts that pose's ticks. A boss's CPU attack stream has no header: it starts on its first frame, and many of them start *on an active frame* (startup 1). The 1987 CPU begins an attack the moment its decision picks one, so the wind-up a player's move needs is not in these streams; a player-facing version of such a move may need one, and that is a design question for the move's ticket, not a defect of the ROM.\n"
    );

    // ---- Method ----------------------------------------------------------------
    let _ = writeln!(o, "## Method\n");
    let _ = writeln!(
        o,
        "**What a frame is.** A move is a 0x30-byte header in its fighter's move table (the first pose and the move's data) and a stream of 0x20-byte frame records and control entries (`fighter::load_frame`, `animate`). A record holds a pose (a grid of 16 px sprites, or 32 px sprites when the attribute is `0x400` or more, at most 24 entries), how long it lasts, its boxes and its flags. The audit walks every stream the way `animate` does, keeping both the *primary path* (the order the move plays in when nothing interrupts it: gotos followed, the Tatsumaki's spin loop run its five times, Geki's teleport taking its first choice) and every other reachable frame (the other teleport choices, the CPU follow-up's resume)."
    );
    let _ = writeln!(
        o,
        "\n**Where the boxes sit on the drawing.** `place_frame` (`0x43b6a`) always puts the pose's centre on the fighter's centre, and every box is an offset from that centre: a box is centred at `dx` to the right and `dy` above it, with half sizes `hw` and `hh`, and the judge (`0x43e82`) compares them inclusively. A left-facing frame negates the `dx` bytes and mirrors the grid, so the audit works on the pose facing right. The drawn pixels are decoded from the sprite ROM exactly as the renderer does (pen 15 transparent, big sprites as 2x2 tiles, tile code inversion). The diagnostic overlay's reading of the boxes on the engine's own grid agrees with this placement on every tick checked."
    );
    let _ = writeln!(
        o,
        "\n**Attack boxes** (`+0x22..+0x25`, live while the frame's flags bit 7 is set). *Overreach* is how far the box's outer edge lies past the farthest drawn pixel in its own band: the rows the box spans for forward and back, the columns it spans for up and down. A box with no drawn pixel inside it at all is *floating*. *Reach* is `x_offset + dx + hw`: the pixels from the fighter's stance centre to the box's forward edge. The frame's x-offset (`+0x26`) counts because `load_frame` writes it into the grid's first sprite, and the judge measures from the grid's centre, which moves with it (a stance's offset is 0). *Travel* is how far the frames' own steps (`+0x15`, applied on load and on each repeat) carry the fighter over the whole move: lunges, jumps and turns; it is reported apart and is not part of reach. The pictures draw the attack box in red (its inside tinted), the loose hurtbox in cyan, the tight (critical) box in yellow, the pose's edge in grey and its centre as a green cross; an attack box on a frame whose active bit is clear (it can never hit) is an orange outline; pens are drawn as greys because the game sets the palette at run time."
    );
    let _ = writeln!(
        o,
        "\n**Hurtboxes.** A hit must touch the *loose* box (`+0x1e..+0x21`); touching the *tight* box (`+0x1a..+0x1d`) as well makes it a critical hit. *Drawing outgrows the hurtbox* compares the share of drawn pixels outside the loose box with the fighter's own stance. *Hurtbox past the drawing* is the loose box's overreach, measured like an attack box's. *Critical box outside the hurtbox* is how far the tight box sticks out of the loose box (an attack cannot score a critical through that part alone, since it must touch the loose box first; one that also touches the loose box still can). *Point hurtbox* is a frame whose loose box has size zero. The judge has no empty-box test, so that is a single point at the box's offset from the centre, which an attack box can still touch (and a zero tight box makes a hit on that point critical); the fighter is hard to hit, not untouchable. Only the invulnerable flag (flags bit 3) makes a frame unhittable."
    );
    let _ = writeln!(
        o,
        "\n**Startup and active ticks.** A frame lasts `(repeat + 1) * (duration + 1)` ticks. Startup counts the first active frame (the research's convention: a move that hits on its fifth tick has startup 5) and so is `1 +` the ticks of every frame before it, counted from the tick the move starts, not from the button press (the 1987 game starts a move one frame after the button is released). The crouching punches that call `0x455fc` hold their first frames 0, 6 or 9 ticks longer for a light, medium or heavy button, shown as `7 (13/16 M/H)`. A projectile move shows the tick its fireball spawns (`spawn N`). *Active* counts every tick of a frame with the attack bit set; in the game a hit clears the bit, so a connecting move is active for fewer ticks than shown. *Total* is the primary path's length, which for a CPU stream ends at the follow-up call (`0x45630`)."
    );

    // ---- Thresholds ----------------------------------------------------------
    let _ = writeln!(o, "\n## Thresholds\n");
    let _ = writeln!(
        o,
        "The thresholds are constants in `box_audit.rs`, chosen on the art's own grain (a sprite tile is 16 px) and on the ROM's own spread (the table below), and the research's numbers where it has them. A move below a threshold is not an error, only unremarkable; the per-fighter tables show the numbers for every move.\n"
    );
    let _ = writeln!(o, "| Finding | Flagged when | Why this number |");
    let _ = writeln!(o, "| --- | --- | --- |");
    let _ = writeln!(o, "| Attack box that never goes live | a move none of whose frames sets the active bit (flags bit 7), though a frame carries an attack box and a hit function | the move can never hit |");
    let _ = writeln!(o, "| Attack box over nothing | an active attack box holds no drawn pixel | nothing is drawn where the move hits |");
    let _ = writeln!(o, "| Attack box past the drawing | overreach of {OVERREACH_FLAG} px or more | a whole sprite tile; Tukey's outer fence for the per-move figures below is {:.0} px and their median {:.0} px |", fence(&over), quartiles(&over).map_or(0.0, |q| q[1]));
    let _ = writeln!(o, "| Hurtbox past the drawing | the loose box lies {BODY_OVERREACH_FLAG} px or more past the drawn pixels | a tile and a half; the 90th percentile of the per-move figures below is {:.0} px |", pct(&loose, 0.9));
    let _ = writeln!(o, "| Drawing outgrows the hurtbox | the share of drawn pixels outside the loose box is {:.0} points or more above the fighter's stance | about the 90th percentile of the per-move figures below ({:.0} points) |", BODY_EXCESS_FLAG * 100.0, pct(&excess, 0.9));
    let _ = writeln!(o, "| Critical box outside the hurtbox | the tight box sticks {TIGHT_OUTSIDE_FLAG} px or more further out of the loose box than in the fighter's stance | a tile; an attack must touch the loose box first, so it cannot score a critical through that part alone |");
    let _ = writeln!(o, "| Point hurtbox mid-move | an attack move shows a zero-size loose hurtbox for {POINT_HURTBOX_FLAG} ticks or more | the fighter can be hit only at one point for that long |");
    let _ = writeln!(o, "| Faster than the research | a melee startup of {FAST_STARTUP} or less, or a projectile that spawns before tick {FAST_PROJECTILE_STARTUP} | the research's fastest normal is {FASTEST_RESEARCH_NORMAL} (USF4 Adon and Gen, CvS2 Eagle, Alpha 3 Gen's close jab), light normals 4 to 5, projectiles 12 to 16 (section 5.2); a startup of {FASTEST_RESEARCH_NORMAL} is at the floor and only shown |");
    let _ = writeln!(o, "| Long reach | reach of {LONG_REACH} px or more from the stance centre | six sprite tiles, a little under Tukey's outer fence for the ROM's attack moves ({:.0} px: third quartile plus 1.5 interquartile ranges). The research's only pixel figure for reach is Super Turbo's throw range, 48 px from the axis (3.4); the median attack here reaches {:.0} px |", fence(&reach), quartiles(&reach).map_or(0.0, |q| q[1]));

    // Population figures.
    let _ = writeln!(o, "\n### The ROM's own spread\n");
    let _ = writeln!(o, "Over the {} moves that have an attack box (reach, overreach, startup) and the moves with a hurtbox (the others).\n", attack_moves.len());
    let _ = writeln!(o, "| Measure | n | min | Q1 | median | Q3 | max |");
    let _ = writeln!(o, "| --- | --- | --- | --- | --- | --- | --- |");
    let _ = writeln!(o, "| reach, px from the stance centre (x-offset + dx + hw) | {} |", fmt_q(&reach, 0));
    let _ = writeln!(o, "| attack overreach, px (worst frame) | {} |", fmt_q(&over, 0));
    let _ = writeln!(o, "| startup, ticks | {} |", fmt_q(&startup, 0));
    let _ = writeln!(o, "| active ticks | {} |", fmt_q(&active, 0));
    let _ = writeln!(o, "| hurtbox overreach, px (worst frame) | {} |", fmt_q(&loose, 0));
    let _ = writeln!(o, "| drawing outside the hurtbox, points above the stance (worst frame) | {} |", fmt_q(&excess, 0));
    let _ = writeln!(o, "| stance: drawn pixels outside the hurtbox, % | {} |", fmt_q(&stance, 0));

    // ---- Outliers by kind --------------------------------------------------------
    let _ = writeln!(o, "\n## Outliers\n");
    let _ = writeln!(
        o,
        "Each row is one move and the frame that crosses the threshold; the picture shows that frame's boxes over its pose (legend above). Rows are in order of the measure, largest first.\n"
    );
    let mut seen_pictures: BTreeMap<String, ()> = BTreeMap::new();
    for kind in FindingKind::ALL {
        let mut rows: Vec<(usize, usize, &MoveAudit, &Finding)> = Vec::new();
        for &(fi, mi, m) in &kit {
            for f in m.findings.iter().filter(|f| f.kind == kind) {
                rows.push((fi, mi, m, f));
            }
        }
        rows.sort_by(|a, b| b.3.value.cmp(&a.3.value).then(a.0.cmp(&b.0)).then(a.1.cmp(&b.1)));
        let _ = writeln!(o, "### {} ({})\n", capitalise(kind.label()), rows.len());
        if rows.is_empty() {
            let _ = writeln!(o, "None.\n");
            continue;
        }
        let _ = writeln!(o, "| Fighter | Move | Stream | Finding | Picture |");
        let _ = writeln!(o, "| --- | --- | --- | --- | --- |");
        for (fi, mi, m, f) in rows {
            let fa = &audits[fi];
            let file = picture_file(fa, m, f.frame);
            if seen_pictures.insert(file.clone(), ()).is_none() {
                pictures.push(PictureRequest { file: file.clone(), fighter: fa.fighter, move_index: mi, frame: f.frame });
            }
            let _ = writeln!(
                o,
                "| {} | {} | {} | {} | <img src=\"{PICTURE_DIR}/{file}\" width=\"220\"> |",
                fa.fighter.name(),
                display_name(&m.entry),
                stream_text(&m.entry),
                f.detail
            );
        }
        let _ = writeln!(o);
    }

    // ---- Reference checks --------------------------------------------------------
    let _ = writeln!(o, "## Against the reference numbers\n");
    let _ = writeln!(o, "### Startup tiers\n");
    let _ = writeln!(
        o,
        "Melee moves by the research's modern-level tiers (section 5.2, first active frame counted). The research proposes these numbers for the modern level; the 1987 moves were never meant to match them, so a count in a different tier is a character, not a fault. Moves that start on their first tick (`1`) are boss CPU streams that begin on an active frame (see the open question above).\n"
    );
    let _ = writeln!(o, "| Fighter | at or below 3 | light 4-5 | medium 6-8 | heavy 9-12 | slower than 12 | fastest | slowest |");
    let _ = writeln!(o, "| --- | --- | --- | --- | --- | --- | --- | --- |");
    for fa in audits {
        let st: Vec<u32> = fa.moves.iter().filter(|m| is_kit(m.entry.kind)).filter_map(|m| m.startup).collect();
        let n = |lo: u32, hi: u32| st.iter().filter(|&&s| s >= lo && s <= hi).count();
        let _ = writeln!(
            o,
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            fa.fighter.name(),
            n(0, 3),
            n(4, 5),
            n(6, 8),
            n(9, 12),
            n(13, 9999),
            st.iter().min().map(|x| x.to_string()).unwrap_or_default(),
            st.iter().max().map(|x| x.to_string()).unwrap_or_default()
        );
    }
    let _ = writeln!(o, "\n### Reach\n");
    let _ = writeln!(
        o,
        "Reach is pixels from the fighter's stance centre to the attack box's forward edge (x-offset + dx + hw), over each fighter's attack moves; the distance a move also carries the fighter (travel) is in the per-fighter tables. The research has no pixel reach for normals (its SF6 \"range\" is in other units); its one pixel figure is Super Turbo's throw, 48 px from the axis, and the 1987 game has no throws. For a rough shape, SF6 Sagat's longest normal is about 1.7 times his shortest (5HP 2.17 against 5LP 1.26, section 1.2); the last column is each fighter's longest reach over his shortest.\n"
    );
    let _ = writeln!(o, "| Fighter | moves | shortest | median | longest | longest / shortest | {LONG_REACH} px or more |");
    let _ = writeln!(o, "| --- | --- | --- | --- | --- | --- | --- |");
    for fa in audits {
        let r: Vec<i32> = fa.moves.iter().filter(|m| is_kit(m.entry.kind)).filter_map(|m| m.reach).collect();
        if r.is_empty() {
            continue;
        }
        let rf: Vec<f64> = r.iter().map(|&x| x as f64).collect();
        let (lo, hi) = (*r.iter().min().unwrap(), *r.iter().max().unwrap());
        let _ = writeln!(
            o,
            "| {} | {} | {} | {:.0} | {} | {:.2} | {} |",
            fa.fighter.name(),
            r.len(),
            lo,
            quartiles(&rf).unwrap()[1],
            hi,
            hi as f64 / lo.max(1) as f64,
            r.iter().filter(|&&x| x >= LONG_REACH).count()
        );
    }

    // Projectiles.
    let _ = writeln!(o, "\n### Projectiles\n");
    let _ = writeln!(
        o,
        "The effect a move spawns has its own box and sprite (`effect_vs_fighter`, `0x430b6`): the box is `hw` by `hh` half sizes about the sprite's centre, and the effect moves `speed` pixels a tick. The overreach is measured as for an attack box. The research's projectile speeds are in SF6 units and cannot be compared; its startup is 12 to 16 for the spawn.\n"
    );
    let _ = writeln!(o, "| Fighter | Move | Spawns on tick | Template | Speed px/tick | Box half w x h per effect frame | Overreach per frame |");
    let _ = writeln!(o, "| --- | --- | --- | --- | --- | --- | --- |");
    for &(fi, _, m) in &kit {
        for p in &m.projectiles {
            let boxes: Vec<String> = p.frames.iter().map(|f| format!("{}x{}", f.1, f.0)).collect();
            let over: Vec<String> = p.frames.iter().map(|f| f.2.worst().map(|w| w.0.to_string()).unwrap_or_else(|| "floating".into())).collect();
            let _ = writeln!(
                o,
                "| {} | {} | {} | `{:#07x}` | {} | {} | {} |",
                audits[fi].fighter.name(),
                m.entry.name,
                m.spawn.unwrap_or(0),
                p.template,
                p.speed,
                boxes.join(" "),
                over.join(" ")
            );
        }
    }

    // 1987 claims.
    let _ = writeln!(o, "\n### The research's notes on the 1987 game, against the ROM\n");
    let _ = writeln!(
        o,
        "The research (section 2) took these from SuperCombo's pages on the SF1++ hack and marked them as leads to check against the ROM, not values. Counting convention: the pages count frames before the first active frame, the audit counts the first active frame too (startup = pages' number + 1).\n"
    );
    let find = |f: TournamentFighter, slot: u8| -> Option<&MoveAudit> {
        audits.iter().find(|a| a.fighter == f)?.moves.iter().find(|m| m.entry.slot == Some(slot))
    };
    let _ = writeln!(o, "| Research note | ROM measure | Reading |");
    let _ = writeln!(o, "| --- | --- | --- |");
    if let (Some(r), Some(k)) = (find(TournamentFighter::Ryu, 0x15), find(TournamentFighter::Ken, 0x15)) {
        let (rs, ks) = (r.startup.unwrap_or(0), k.startup.unwrap_or(0));
        let ok = rs == 22 && ks == 27;
        let _ = writeln!(
            o,
            "| Medium kick startup: Ryu 21, Ken 26 | Ryu startup {rs}, Ken {ks} ({} and {} frames before the active one) | {} |",
            rs - 1,
            ks - 1,
            if ok { "matches" } else { "differs" }
        );
    }
    if let Some(g) = audits.iter().find(|a| a.fighter == TournamentFighter::Geki) {
        let best = g.moves.iter().filter(|m| is_kit(m.entry.kind)).filter_map(|m| m.startup.map(|s| (s, m))).min_by_key(|(s, _)| *s);
        if let Some((s, m)) = best {
            let _ = writeln!(
                o,
                "| Geki: \"fastest attack comes out on frame 10\" (and, on the same page, 9) | Geki's fastest attack stream is `{:#07x}`, startup {s} ({} frames before the active one) | {} |",
                m.entry.stream,
                s - 1,
                if s - 1 == 10 || s - 1 == 9 { "matches one of the two figures" } else { "differs from both figures" }
            );
        }
    }
    if let (Some(r), Some(k)) = (find(TournamentFighter::Ryu, 9), find(TournamentFighter::Ken, 9)) {
        let t = |m: &MoveAudit| m.frames[0].rec.ticks();
        let _ = writeln!(
            o,
            "| Jump: \"an awful nine frame jump squat\" (Ryu and Ken) | the jump's header pose lasts Ryu {} ticks, Ken {} | {} |",
            t(r),
            t(k),
            if t(r) == 9 && t(k) == 9 { "matches" } else { "differs" }
        );
    }
    if let (Some(r), Some(k)) = (find(TournamentFighter::Ryu, 0x19), find(TournamentFighter::Ken, 0x19)) {
        let first_inv = |m: &MoveAudit| m.timeline().into_iter().find(|&(i, _)| m.walk.frames[i].invulnerable()).map(|(_, t)| t);
        let _ = writeln!(
            o,
            "| Shoryuken: Ryu's is fully invincible, Ken's \"is vulnerable for the first 4 frames\" | the invulnerable flag first shows on tick {} for Ryu and tick {} for Ken; the first active frame is tick {} for both | {} |",
            first_inv(r).map(|t| t.to_string()).unwrap_or("never".into()),
            first_inv(k).map(|t| t.to_string()).unwrap_or("never".into()),
            r.startup.unwrap_or(0),
            if first_inv(r) == Some(1) && first_inv(k) == Some(5) { "matches (Ken's first 4 ticks have no flag; the `symmetric_shotos_and_blockable_dp` toggle patches this)" } else { "see the numbers" }
        );
    }
    let after_last_active = |m: &MoveAudit| -> u32 {
        let tl = m.timeline();
        let last = tl.iter().rev().find(|&&(i, _)| m.walk.frames[i].attack_active());
        match last {
            Some(&(i, at)) => m.total_ticks + 1 - (at + m.walk.frames[i].ticks()),
            None => 0,
        }
    };
    if let (Some(r), Some(k)) = (find(TournamentFighter::Ryu, 0x1b), find(TournamentFighter::Ken, 0x1b)) {
        let _ = writeln!(
            o,
            "| Tatsumaki end lag: Ryu 28, Ken 24 | ticks after the last active frame: Ryu {}, Ken {} | {} |",
            after_last_active(r),
            after_last_active(k),
            if after_last_active(r) == 28 && after_last_active(k) == 24 {
                "matches"
            } else if after_last_active(r) == 27 && after_last_active(k) == 23 {
                "one tick less for both: the same four-tick gap between them, so a counting convention, not a different move"
            } else {
                "differs"
            }
        );
    }
    if let (Some(r), Some(k)) = (find(TournamentFighter::Ryu, 0x1a), find(TournamentFighter::Ken, 0x1a)) {
        let after = |m: &MoveAudit| m.total_ticks + 1 - m.spawn.unwrap_or(0);
        let _ = writeln!(
            o,
            "| Hadoken end lag: on whiff Ryu 16, Ken 30; on hit Ryu 37, Ken 94; on block Ryu 7, Ken 19 | ticks from the fireball's spawn to the stream's end: Ryu {}, Ken {} (the spawn is on tick {} for both) | not comparable: the page does not say what its lag counts from, and hit lag includes the slow motion (`hadoken.md`) |",
            after(r),
            after(k),
            r.spawn.unwrap_or(0)
        );
    }
    if let Some(l) = find(TournamentFighter::Lee, 4) {
        let mut spans: Vec<String> = Vec::new();
        let mut tick = 1;
        for &i in &l.walk.path {
            let rec = &l.walk.frames[i];
            spans.push(format!(
                "ticks {}-{}: hurtbox centre {} px {} the grid's, half height {}",
                tick,
                tick + rec.ticks() - 1,
                rec.loose.dy.unsigned_abs(),
                if rec.loose.dy < 0 { "below" } else { "above" },
                rec.loose.hh
            ));
            tick += rec.ticks();
        }
        let _ = writeln!(
            o,
            "| Lee: for the first 16 frames of his block \"his hurtbox is lowered dramatically\" | {} | {} |",
            spans.join("; "),
            if l.walk.path.first().is_some_and(|&i| l.walk.frames[i].ticks() == 16 && l.walk.frames[i].loose.dy < 0) {
                "matches: the first 16 ticks have the hurtbox's centre well below the pose's centre"
            } else {
                "see the numbers"
            }
        );
    }

    // ---- Every move ---------------------------------------------------------------
    let _ = writeln!(o, "\n## Every move\n");
    let _ = writeln!(
        o,
        "One row per move. *Startup*, *Active/Total* and *Reach* are as defined above; *Past* is the attack box's largest overreach in px and its direction (F forward, B back, U up, D down); *Body* is the largest points by which the drawing outgrows the hurtbox over the fighter's stance (negative: better covered than the stance); *Tight* is the critical box's largest overhang in px; *Travel* is the net px the frame steps carry the fighter (lunges, jumps); *Point hurt* is the ticks with a zero-size (point) hurtbox; *Flags* are the findings (`{}`). `[0x..]` is the table slot.\n",
        FindingKind::ALL.iter().map(|k| k.slug()).collect::<Vec<_>>().join("`, `")
    );
    for fa in audits {
        let _ = writeln!(
            o,
            "### {}\n\nStance: {:.0}% of the drawing outside the loose hurtbox.\n",
            fa.fighter.name(),
            fa.stance_share * 100.0
        );
        let _ = writeln!(o, "| Move | Kind | Stream | Startup | Active/Total | Reach | Past | Body | Travel | Tight | Point hurt | Flags |");
        let _ = writeln!(o, "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
        for m in fa.moves.iter().filter(|m| is_kit(m.entry.kind)) {
            let past = m
                .worst_overreach
                .map(|(v, d, _)| format!("{v} {}", dir_letter(d)))
                .unwrap_or_else(|| "-".to_string());
            let body = m.uncovered_excess.map(|(e, _)| format!("{:+.0}", e * 100.0)).unwrap_or_else(|| "-".into());
            let tight = m.tight_out.map(|(v, _)| v.to_string()).unwrap_or_else(|| "-".into());
            let nohurt = if m.point_hurtbox_ticks > 0 { m.point_hurtbox_ticks.to_string() } else { "-".into() };
            let at = if m.startup.is_some() { format!("{}/{}", m.active_ticks, m.total_ticks) } else { format!("-/{}", m.total_ticks) };
            let flags = m.findings.iter().map(|f| f.kind.slug()).collect::<Vec<_>>().join(" ");
            let note = match m.walk.end {
                PathEnd::Broken => " (stream pointer lands inside a record: unusable in the ROM)",
                PathEnd::Cap => " (path cap)",
                _ if m.entry.kind == MoveKind::Table
                    && m.entry.stream == 0
                    && m.frames.first().is_some_and(|f| f.point_hurtbox)
                    && !matches!(m.entry.slot, Some(0) | Some(0x0c)) =>
                {
                    " (unused slot: no stream, zero-size hurtbox)"
                }
                _ => "",
            };
            let _ = writeln!(
                o,
                "| {}{note} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                display_name(&m.entry),
                short_kind(&m.entry),
                stream_text(&m.entry),
                startup_text(m),
                at,
                m.reach.map(|r| r.to_string()).unwrap_or_else(|| "-".into()),
                past,
                body,
                m.travel.map(|t| format!("{t:+}")).unwrap_or_else(|| "-".into()),
                tight,
                nohurt,
                flags
            );
        }
        let _ = writeln!(o);
    }

    // ---- Reactions ------------------------------------------------------------------
    let _ = writeln!(o, "## Hit reactions and knock-outs\n");
    let _ = writeln!(
        o,
        "A fighter's hit and knock-out streams are outside the kit a player drives and are not flagged. In 1987 a fighter is invulnerable for his whole reaction, so their hurtboxes only matter once the modern rules make the reaction hittable (combos). The table shows what the combos would meet: how many of each fighter's reaction frames have a zero-size (point) loose hurtbox.\n"
    );
    let _ = writeln!(o, "| Fighter | hit streams | knock-out streams | frames | frames with a point hurtbox | invulnerable flag |");
    let _ = writeln!(o, "| --- | --- | --- | --- | --- | --- |");
    for fa in audits {
        let hit = fa.moves.iter().filter(|m| m.entry.kind == MoveKind::HitReaction);
        let ko = fa.moves.iter().filter(|m| m.entry.kind == MoveKind::KnockOut);
        let frames: Vec<&FrameAudit> = fa.moves.iter().filter(|m| !is_kit(m.entry.kind)).flat_map(|m| &m.frames).collect();
        let _ = writeln!(
            o,
            "| {} | {} | {} | {} | {} | {} |",
            fa.fighter.name(),
            hit.count(),
            ko.count(),
            frames.len(),
            frames.iter().filter(|f| f.point_hurtbox).count(),
            frames.iter().filter(|f| f.rec.invulnerable()).count()
        );
    }

    // ---- Limits ------------------------------------------------------------------------
    let _ = writeln!(o, "\n## Limits\n");
    let _ = writeln!(
        o,
        "- **Boxes are measured against pixels, not against intent.** A hurtbox that leaves an extended arm out is the 1987 design (\"the 1987 box is a torso box\"); it is flagged only where it is unusual for the ROM itself.\n- **Poses are drawn in greys.** The palette is set at run time; the pens' brightness order is kept, the colours are not.\n- **Frames at the ends of a stream.** The CPU follow-up call (`0x45630`) ends a stream's primary path; what follows is the CPU's next decision, which is not a frame of the move.\n- **Random choices.** Geki's teleport picks one of three frame groups at random (`0x45a34`); the primary path takes the first and the other two are measured as frames but not as timing.\n- **Joe's crouch slot is broken in the ROM.** His move 5's stream pointer lands inside a record; the audit shows its header pose only (see `boss-ladder-report.md`).\n- **Projectile boxes** are measured against the effect's own sprite; where a projectile starts relative to the thrower is not.\n- **No hit is simulated.** The audit reads the boxes; it does not play the moves against each other."
    );
    Report { markdown: o, pictures }
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
