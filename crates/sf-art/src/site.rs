//! The review page (spec #164, AP4): one fighter's moves playing in a real
//! fight frame next to his ROM art, with approve and reject on every pose.
//! It is served on localhost only and never published (grilling Q8).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sf_engine::roster::TournamentFighter;
use sf_video::art::{Art, Catalogue};
use sf_video::art_review::Review;
use sf_video::render::{render_frame, Frame, HEIGHT, WIDTH};
use sf_video::romset::Assets;

use crate::frame::fight_frame;

/// Everything the page serves, rendered up front.
pub struct Site {
    catalogue: Catalogue,
    /// Frames as BMP files, by path.
    files: BTreeMap<String, Vec<u8>>,
    review: Review,
    review_path: PathBuf,
}

/// A 24-bit BMP of a frame.
pub fn bmp(frame: &Frame) -> Vec<u8> {
    let row = WIDTH * 3;
    let size = 54 + row * HEIGHT;
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(WIDTH as i32).to_le_bytes());
    out.extend_from_slice(&(HEIGHT as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in (0..HEIGHT).rev() {
        for x in 0..WIDTH {
            let p = frame.argb[y * WIDTH + x];
            out.extend_from_slice(&[p as u8, (p >> 8) as u8, (p >> 16) as u8]);
        }
    }
    out
}

/// The opponent a fighter is reviewed against: Ken for Ryu, Ryu for
/// everyone else.
pub fn opponent(fighter: TournamentFighter) -> TournamentFighter {
    if fighter == TournamentFighter::Ryu {
        TournamentFighter::Ken
    } else {
        TournamentFighter::Ryu
    }
}

impl Site {
    /// Render every pose of `fighter`'s catalogue into a real fight frame,
    /// both facings, and load his review file (pending for anything new).
    pub fn build(rom: &Assets, art: &Art, fighter: TournamentFighter, review_path: PathBuf) -> Result<Site, String> {
        let name = fighter.name().to_ascii_lowercase();
        let catalogue = art.catalogue(&name).ok_or_else(|| format!("no recipes for {name}"))?.clone();
        let banked = rom.with_sprite_bank(&art.bank);
        let ff = fight_frame(rom, fighter, opponent(fighter))?;
        let mut files = BTreeMap::new();
        files.insert("rom.bmp".to_owned(), bmp(&render_frame(&banked, &ff.scene)));
        for pose in &catalogue.poses {
            for (left, suffix) in [(false, ""), (true, "-left")] {
                let frame = render_frame(&banked, &ff.with_pose(pose, left));
                files.insert(format!("pose/{}{suffix}.bmp", pose.name), bmp(&frame));
            }
        }
        let old = Review::load(&review_path);
        let review = Review::for_catalogue(&catalogue, old.as_ref());
        review.save(&review_path)?;
        Ok(Site { catalogue, files, review, review_path })
    }

    pub fn review(&self) -> &Review {
        &self.review
    }

    /// The page itself.
    pub fn page(&self) -> String {
        let moves = serde_json::json!(self
            .catalogue
            .moves
            .iter()
            .map(|m| serde_json::json!({ "name": m.name, "poses": m.poses, "timing": m.timing, "note": m.note }))
            .collect::<Vec<_>>());
        let notes: BTreeMap<&str, &str> = self.catalogue.poses.iter().map(|p| (p.name.as_str(), p.note.as_str())).collect();
        PAGE.replace("__FIGHTER__", &self.catalogue.fighter)
            .replace("__MOVES__", &moves.to_string())
            .replace("__NOTES__", &serde_json::to_string(&notes).unwrap())
    }

    /// Answer one request: `(status, content type, body)`.
    pub fn handle(&mut self, method: &str, path: &str, body: &str) -> (u16, &'static str, Vec<u8>) {
        match (method, path) {
            ("GET", "/") => (200, "text/html; charset=utf-8", self.page().into_bytes()),
            ("GET", "/state") => (200, "application/json", serde_json::to_vec(&self.state()).unwrap()),
            ("POST", "/decide") => {
                let field = |k: &str| {
                    body.split('&').find_map(|kv| kv.strip_prefix(&format!("{k}="))).map(|v| v.replace("%20", " "))
                };
                let (Some(pose), Some(verdict)) = (field("pose"), field("verdict")) else {
                    return (400, "text/plain", b"pose and verdict, please".to_vec());
                };
                match self.review.decide(&pose, verdict == "approve") {
                    Ok(_) => match self.review.save(&self.review_path) {
                        Ok(()) => (200, "application/json", serde_json::to_vec(&self.state()).unwrap()),
                        Err(e) => (500, "text/plain", e.into_bytes()),
                    },
                    Err(e) => (404, "text/plain", e.into_bytes()),
                }
            }
            ("GET", p) => match self.files.get(p.trim_start_matches('/')) {
                Some(f) => (200, "image/bmp", f.clone()),
                None => (404, "text/plain", b"not found".to_vec()),
            },
            _ => (405, "text/plain", b"method not allowed".to_vec()),
        }
    }

    fn state(&self) -> serde_json::Value {
        serde_json::json!({ "poses": self.review.poses, "approved": self.review.approved() })
    }

    /// Serve until the process ends (see [`serve`]).
    pub fn serve(self, listener: TcpListener) {
        serve(self, listener);
    }
}

/// A page that answers requests: `(status, content type, body)`.
pub trait Responder {
    fn respond(&mut self, method: &str, path: &str, body: &str) -> (u16, &'static str, Vec<u8>);
}

impl Responder for Site {
    fn respond(&mut self, method: &str, path: &str, body: &str) -> (u16, &'static str, Vec<u8>) {
        self.handle(method, path, body)
    }
}

/// Serve until the process ends. Each connection gets its own thread (a
/// browser opens connections it may not use at once), and the page is
/// locked only while a request is answered.
pub fn serve<R: Responder + Send + 'static>(page: R, listener: TcpListener) {
    let page = Arc::new(Mutex::new(page));
    for stream in listener.incoming().flatten() {
        let page = Arc::clone(&page);
        std::thread::spawn(move || {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
            let _ = answer(&page, stream);
        });
    }
}

fn answer<R: Responder>(site: &Mutex<R>, stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next().unwrap_or("").to_owned(), parts.next().unwrap_or("/").to_owned());
    let mut length = 0usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header)?;
        if header.trim().is_empty() {
            break;
        }
        if let Some(v) = header.to_ascii_lowercase().strip_prefix("content-length:") {
            length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;
    let (status, kind, out) = site.lock().unwrap().respond(&method, &path, &String::from_utf8_lossy(&body));
    let mut stream = stream;
    write!(stream, "HTTP/1.1 {status} OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", out.len())?;
    stream.write_all(&out)
}

const PAGE: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Art review</title>
<style>
:root { --bg: #16161c; --fg: #e8e8ee; --dim: #9a9aa8; --ok: #4caf50; --no: #e05252; --card: #22222b; }
body { background: var(--bg); color: var(--fg); font: 15px/1.45 system-ui, sans-serif; margin: 0 auto; padding: 16px; max-width: 1240px; }
h1 { font-size: 20px; margin: 4px 0 2px; } .dim { color: var(--dim); }
.move { background: var(--card); border-radius: 8px; padding: 12px; margin: 14px 0; }
.frames { display: flex; gap: 10px; flex-wrap: wrap; }
figure { margin: 0; } figcaption { color: var(--dim); font-size: 13px; }
img.screen { width: 576px; max-width: 100%; image-rendering: pixelated; border-radius: 4px; display: block; }
table { border-collapse: collapse; margin-top: 10px; width: 100%; } td { padding: 4px 8px; border-top: 1px solid #33333d; vertical-align: top; }
button { font: inherit; border: 0; border-radius: 6px; padding: 4px 12px; cursor: pointer; margin-right: 6px; }
.approve { background: var(--ok); color: #fff; } .reject { background: var(--no); color: #fff; }
.tag { font-size: 12px; padding: 1px 8px; border-radius: 10px; background: #33333d; }
.tag.approved { background: var(--ok); } .tag.rejected { background: var(--no); }
#status { font-weight: 600; } label { color: var(--dim); font-size: 13px; }
</style></head><body>
<h1>Art review: <span id="fighter"></span></h1>
<p class="dim">Each move plays in a real fight frame, drawn by the game. On the left is his own ROM art in the same frame. This page is local and never published.</p>
<p id="status"></p>
<label><input type="checkbox" id="left"> facing left</label>
<div id="moves"></div>
<script>
const FIGHTER = "__FIGHTER__", MOVES = __MOVES__, NOTES = __NOTES__;
document.getElementById("fighter").textContent = FIGHTER;
let state = { poses: {}, approved: false };
const facing = () => document.getElementById("left").checked ? "-left" : "";
function render() {
  const root = document.getElementById("moves"); root.innerHTML = "";
  for (const m of MOVES) {
    const div = document.createElement("div"); div.className = "move";
    div.innerHTML = `<h2>${m.name}</h2><p class="dim">${m.note}</p>
      <div class="frames"><figure><img class="screen" src="/rom.bmp"><figcaption>ROM: his own art in this fight</figcaption></figure>
      <figure><img class="screen anim"><figcaption>the move, at game speed (${m.timing.join(", ")} frames per pose)</figcaption></figure></div>
      <table></table>`;
    const table = div.querySelector("table");
    for (const p of m.poses) {
      const r = state.poses[p] || {}; const tr = document.createElement("tr");
      tr.innerHTML = `<td><b>${p}</b><br><span class="dim">${NOTES[p] || ""}</span></td>
        <td><span class="tag ${r.verdict}">${r.verdict || "pending"}</span> ${r.next && r.next !== "none" ? "<span class=tag>next: " + r.next + "</span>" : ""} ${r.rejections ? "<span class=dim>rejected " + r.rejections + "×</span>" : ""}</td>
        <td style="white-space:nowrap"><button class="approve">Approve</button><button class="reject">Reject</button></td>`;
      tr.querySelector(".approve").onclick = () => decide(p, "approve");
      tr.querySelector(".reject").onclick = () => decide(p, "reject");
      table.appendChild(tr);
    }
    root.appendChild(div);
    play(div.querySelector(".anim"), m);
  }
  const n = Object.values(state.poses), ok = n.filter(p => p.verdict === "approved").length;
  document.getElementById("status").textContent = state.approved
    ? `${FIGHTER} approved: all ${n.length} poses.` : `${ok} of ${n.length} poses approved.`;
}
const timers = [];
const cache = {};
function load(src) {
  if (!cache[src]) cache[src] = new Promise(ok => { const im = new Image(); im.onload = () => ok(src); im.src = src; });
  return cache[src];
}
async function play(img, m) {
  const srcs = m.poses.map(p => `/pose/${p}${facing()}.bmp`);
  await Promise.all(srcs.map(load));
  let i = 0;
  const step = () => { img.src = srcs[i]; const t = m.timing[i] * 1000 / 60; i = (i + 1) % srcs.length; timers.push(setTimeout(step, t)); };
  step();
}
async function decide(pose, verdict) {
  const r = await fetch("/decide", { method: "POST", body: `pose=${encodeURIComponent(pose)}&verdict=${verdict}` });
  if (r.ok) { state = await r.json(); timers.forEach(clearTimeout); timers.length = 0; render(); } else { alert(await r.text()); }
}
document.getElementById("left").onchange = () => { timers.forEach(clearTimeout); timers.length = 0; render(); };
fetch("/state").then(r => r.json()).then(s => { state = s; render(); });
</script></body></html>
"#;
