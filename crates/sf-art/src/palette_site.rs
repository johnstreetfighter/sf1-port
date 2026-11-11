//! The mirror palettes' review page (spec #246, SP16), in the style of the
//! pose review page (`site.rs`): all twelve fighters on one page, each with
//! his normal palette beside his mirror palette on a few poses, real frames
//! of a Local mirror fight with the gate on, and approve or
//! reject with a reason. Served on localhost only, never published.
//! Verdicts go to `art/review/<fighter>-palette.json`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sf_engine::roster::TournamentFighter;
use sf_video::art::Art;
use sf_video::mirror_palette::{self, colour_text, MirrorPalette};
use sf_video::romset::Assets;

use crate::palettes::{frame_png, mirror_colours, mirror_frames, palette_colours, pose_pairs, review_path, rom_palette, PaletteReview};
use crate::site::Responder;

struct Entry {
    fighter: TournamentFighter,
    mirror: &'static MirrorPalette,
    review: PaletteReview,
    review_path: PathBuf,
    /// Pictures: `(label, path)`, in page order.
    poses: Vec<(String, String)>,
    frames: Vec<(String, String)>,
    /// Why a part could not be drawn, shown on the page.
    problems: Vec<String>,
    /// His pens, normal and mirror, as the page's swatches.
    swatches: serde_json::Value,
}

/// Everything the page serves, drawn up front.
pub struct PaletteSite {
    entries: Vec<Entry>,
    files: BTreeMap<String, Vec<u8>>,
}

impl PaletteSite {
    /// Draw every fighter's pictures and load (or start) his verdict file in
    /// `art/review`. `art` holds the catalogues the poses come from; without
    /// it (or for a fighter it lacks) the page still shows swatches
    /// and the fight frames.
    pub fn build(rom: &Assets, art: Option<&Art>, art_dir: &std::path::Path) -> Result<PaletteSite, String> {
        let banked = art.map(|a| rom.with_sprite_bank(&a.bank));
        let mut entries = Vec::new();
        let mut files = BTreeMap::new();
        for fighter in TournamentFighter::ALL {
            let name = fighter.name().to_ascii_lowercase();
            let mirror = mirror_palette::for_fighter(&name).ok_or_else(|| format!("no mirror palette for {name}"))?;
            let path = review_path(art_dir, &name);
            let review = PaletteReview::current(PaletteReview::load(&path), mirror);
            review.save(&path)?;
            let swatches = serde_json::json!({
                "from": mirror.from, "slot": mirror.slot,
                "normal": rom_palette(rom, mirror.from)[..15].iter().map(|w| colour_text(*w)).collect::<Vec<_>>(),
                "mirror": mirror.pens.iter().map(|w| colour_text(*w)).collect::<Vec<_>>(),
            });
            let mut entry = Entry { fighter, mirror, review, review_path: path, poses: vec![], frames: vec![], problems: vec![], swatches };
            let catalogue = art.and_then(|a| a.catalogue(&name));
            let (normal, mirrored) = (palette_colours(rom, fighter.sprite_palette()), mirror_colours(rom, mirror));
            match (&banked, catalogue) {
                (Some(banked), Some(c)) => {
                    for p in pose_pairs(banked, c, &normal, &mirrored) {
                        let at = format!("img/{name}/pose-{}.png", p.label);
                        files.insert(at.clone(), p.png);
                        entry.poses.push((p.label.replace('_', " "), at));
                    }
                }
                _ => entry.problems.push("no pose catalogue, so no poses are drawn".to_owned()),
            }
            match mirror_frames(rom, fighter) {
                Ok(frames) => {
                    for (label, frame) in frames {
                        let at = format!("img/{name}/fight-{}.png", label.replace(' ', "-"));
                        files.insert(at.clone(), frame_png(&frame));
                        entry.frames.push((label, at));
                    }
                }
                Err(e) => entry.problems.push(format!("the mirror fight could not be played: {e}")),
            }
            entries.push(entry);
        }
        Ok(PaletteSite { entries, files })
    }

    /// The pictures, by the path the page asks for.
    pub fn files(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.files
    }

    pub fn review(&self, fighter: &str) -> Option<&PaletteReview> {
        self.entries.iter().find(|e| e.review.fighter == fighter).map(|e| &e.review)
    }

    /// Every fighter's review state, for the page's refresh.
    fn state(&self) -> serde_json::Value {
        let approved = self.entries.iter().filter(|e| e.review.approved_for(e.mirror)).count();
        serde_json::json!({
            "approved": approved,
            "fighters": self.entries.iter().map(|e| (e.review.fighter.clone(), serde_json::json!(e.review))).collect::<BTreeMap<_, _>>(),
        })
    }

    /// The page itself.
    pub fn page(&self) -> String {
        let fighters = serde_json::json!(self
            .entries
            .iter()
            .map(|e| serde_json::json!({
                "name": e.review.fighter,
                "title": e.fighter.name(),
                "note": e.mirror.note,
                "swatches": e.swatches,
                "poses": e.poses, "frames": e.frames, "problems": e.problems,
            }))
            .collect::<Vec<_>>());
        PAGE.replace("__FIGHTERS__", &fighters.to_string())
    }

    pub fn handle(&mut self, method: &str, path: &str, body: &str) -> (u16, &'static str, Vec<u8>) {
        match (method, path) {
            ("GET", "/") => (200, "text/html; charset=utf-8", self.page().into_bytes()),
            ("GET", "/state") => (200, "application/json", serde_json::to_vec(&self.state()).unwrap()),
            ("POST", "/decide") => {
                let form = parse_form(body);
                let (Some(fighter), Some(verdict)) = (form.get("fighter"), form.get("verdict")) else {
                    return (400, "text/plain", b"fighter and verdict, please".to_vec());
                };
                let reason = form.get("reason").map(String::as_str).unwrap_or("");
                let Some(entry) = self.entries.iter_mut().find(|e| &e.review.fighter == fighter) else {
                    return (404, "text/plain", format!("no fighter named {fighter}").into_bytes());
                };
                if let Err(e) = entry.review.decide(entry.mirror, verdict == "approve", reason) {
                    return (400, "text/plain", e.into_bytes());
                }
                match entry.review.save(&entry.review_path) {
                    Ok(()) => (200, "application/json", serde_json::to_vec(&self.state()).unwrap()),
                    Err(e) => (500, "text/plain", e.into_bytes()),
                }
            }
            ("GET", p) => match self.files.get(p.trim_start_matches('/')) {
                Some(f) => (200, "image/png", f.clone()),
                None => (404, "text/plain", b"not found".to_vec()),
            },
            _ => (405, "text/plain", b"method not allowed".to_vec()),
        }
    }
}

impl Responder for PaletteSite {
    fn respond(&mut self, method: &str, path: &str, body: &str) -> (u16, &'static str, Vec<u8>) {
        self.handle(method, path, body)
    }
}

/// `a=b&c=d` with `+` and `%XX` decoded.
pub fn parse_form(body: &str) -> BTreeMap<String, String> {
    body.split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (decode(k), decode(v)))
        .collect()
}

fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if bytes.get(i + 1..i + 3).is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)) => {
                out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

const PAGE: &str = r##"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Mirror palettes</title>
<style>
:root { --bg: #16161c; --fg: #e8e8ee; --dim: #9a9aa8; --ok: #4caf50; --no: #e05252; --card: #22222b; }
body { background: var(--bg); color: var(--fg); font: 15px/1.45 system-ui, sans-serif; margin: 0 auto; padding: 16px; max-width: 1240px; }
h1 { font-size: 20px; margin: 4px 0 2px; } h2 { font-size: 18px; margin: 0 0 4px; } h3 { font-size: 14px; color: var(--dim); margin: 14px 0 4px; font-weight: 600; }
.dim { color: var(--dim); }
nav a { color: var(--fg); background: var(--card); padding: 2px 10px; border-radius: 10px; margin-right: 6px; text-decoration: none; font-size: 13px; display: inline-block; margin-bottom: 4px; }
nav a.approved { border-bottom: 3px solid var(--ok); } nav a.rejected { border-bottom: 3px solid var(--no); }
.fighter { background: var(--card); border-radius: 8px; padding: 12px; margin: 14px 0; }
.row { display: flex; gap: 12px; flex-wrap: wrap; align-items: flex-end; }
figure { margin: 0; } figcaption { color: var(--dim); font-size: 13px; }
img { image-rendering: pixelated; border-radius: 4px; display: block; max-width: 100%; }
img.frame { width: 576px; }
.chips { display: flex; gap: 2px; align-items: center; margin: 2px 0; } .chips b { width: 90px; font-weight: 400; color: var(--dim); font-size: 12px; }
.chip { width: 30px; height: 22px; border: 1px solid #00000066; font-size: 9px; color: #fff; text-shadow: 0 0 2px #000, 0 0 2px #000; text-align: center; line-height: 22px; }
button { font: inherit; border: 0; border-radius: 6px; padding: 4px 12px; cursor: pointer; margin-right: 6px; }
.approve { background: var(--ok); color: #fff; } .reject { background: var(--no); color: #fff; }
textarea { width: 100%; max-width: 640px; background: #16161c; color: var(--fg); border: 1px solid #44444f; border-radius: 6px; padding: 6px; font: inherit; box-sizing: border-box; }
.tag { font-size: 12px; padding: 1px 8px; border-radius: 10px; background: #33333d; margin-left: 8px; }
.tag.approved { background: var(--ok); } .tag.rejected { background: var(--no); }
.problem { color: #e0b252; } #status { font-weight: 600; } ul.history { margin: 4px 0; padding-left: 18px; color: var(--dim); font-size: 13px; }
</style></head><body>
<h1>Mirror palettes</h1>
<p class="dim">Each fighter's second palette, worn by P2 in a Local mirror match. On each pose the left is his normal palette and the right his mirror; only his body is recoloured (projectiles and effects keep their colours); the fight frames are real, with P1 in the normal palette and P2 in the mirror, drawn by the game. A verdict holds for the colours it was given on: change them and the fighter is pending again. This page is local and never published.</p>
<p id="status"></p>
<nav id="nav"></nav>
<div id="fighters"></div>
<script>
const FIGHTERS = __FIGHTERS__;
let state = { approved: 0, fighters: {} };
const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const chips = (label, colours) => `<div class="chips"><b>${esc(label)}</b>${colours.map((c, i) => `<div class="chip" style="background:${c}" title="pen ${i} ${c}">${i}</div>`).join("")}</div>`;
function tag(name) {
  const r = state.fighters[name];
  if (!r) return "";
  return `<span class="tag ${r.verdict}">${r.verdict}</span>`;
}
function render() {
  const root = document.getElementById("fighters"), nav = document.getElementById("nav");
  nav.innerHTML = FIGHTERS.map(f => `<a href="#${f.name}" class="${(state.fighters[f.name] || {}).verdict}">${esc(f.title)}</a>`).join("");
  document.getElementById("status").textContent = `${state.approved} of ${FIGHTERS.length} mirror palettes approved.`;
  root.innerHTML = "";
  for (const f of FIGHTERS) {
    const r = state.fighters[f.name] || {};
    const div = document.createElement("div"); div.className = "fighter"; div.id = f.name;
    div.innerHTML = `<h2>${esc(f.title)} ${tag(f.name)}</h2><p class="dim">${esc(f.note)}</p>
      <h3>His body: sprite palette ${f.swatches.from}, drawn in palette ${f.swatches.slot} for P2</h3>${chips("normal", f.swatches.normal)}${chips("mirror", f.swatches.mirror)}
      ${f.problems.map(p => `<p class="problem">${esc(p)}</p>`).join("")}
      ${f.poses.length ? `<h3>Poses: normal, then mirror</h3><div class="row">${f.poses.map(p => `<figure><img src="/${p[1]}"><figcaption>${esc(p[0])}</figcaption></figure>`).join("")}</div>` : ""}
      ${f.frames.length ? `<h3>A Local mirror fight, P1 on the left in the normal palette, P2 on the right in the mirror</h3><div class="row">${f.frames.map(p => `<figure><img class="frame" src="/${p[1]}"><figcaption>${esc(p[0])}</figcaption></figure>`).join("")}</div>` : ""}
      <h3>Verdict</h3>
      <p>${r.verdict === "rejected" ? `Rejected: ${esc(r.reason)}` : r.verdict === "approved" ? `Approved${r.reason ? ": " + esc(r.reason) : ""}.` : "Pending."}</p>
      <textarea rows="2" data-name="${f.name}" placeholder="Why (required to reject)"></textarea>
      <p><button class="approve">Approve</button><button class="reject">Reject</button></p>
      ${(r.history || []).length ? `<ul class="history">${r.history.map(h => `<li>${h.verdict}${h.reason ? ": " + esc(h.reason) : ""}</li>`).join("")}</ul>` : ""}`;
    div.querySelector(".approve").onclick = () => decide(f.name, "approve", div.querySelector("textarea").value);
    div.querySelector(".reject").onclick = () => decide(f.name, "reject", div.querySelector("textarea").value);
    root.appendChild(div);
  }
}
async function decide(fighter, verdict, reason) {
  const r = await fetch("/decide", { method: "POST", body: `fighter=${encodeURIComponent(fighter)}&verdict=${verdict}&reason=${encodeURIComponent(reason)}` });
  if (r.ok) { state = await r.json(); const y = window.scrollY; render(); window.scrollTo(0, y); } else { alert(await r.text()); }
}
fetch("/state").then(r => r.json()).then(s => { state = s; render(); if (location.hash) document.querySelector(location.hash)?.scrollIntoView(); });
</script></body></html>
"##;
