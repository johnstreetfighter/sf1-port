"""The listening sheet (EH9, #225): one HTML page in the local archive.

Every button plays the ROM's own "FIGHT!" first, as the cabinet's chip plays
it, and then the take or preview, encoded to 4 kHz MSM5205 ADPCM and decoded
by the same chip model: the maintainer judges each one on what the player will hear,
right after the 1987 voice. Served by pipeline.py listen, its Save button
writes the selected voice's picks file. Opened as a file, it offers a
download instead.
"""

import html
import json

STYLE = """
:root { --bg: #f6f4ef; --fg: #1d1b18; --muted: #6b665d; --line: #d9d3c7; --accent: #a3261b; --card: #fffdf8; }
@media (prefers-color-scheme: dark) { :root { --bg: #16140f; --fg: #ece7dc; --muted: #a39d90; --line: #3a362e; --accent: #ff7a6b; --card: #1f1c16; } }
body { background: var(--bg); color: var(--fg); font: 15px/1.45 system-ui, sans-serif; margin: 0 auto; max-width: 1000px; padding: 16px; }
h1 { font-size: 22px; } h2 { font-size: 18px; margin-top: 32px; border-bottom: 1px solid var(--line); }
.muted { color: var(--muted); } code { font-size: 13px; }
table { border-collapse: collapse; width: 100%; } td, th { border-bottom: 1px solid var(--line); padding: 6px; text-align: left; vertical-align: top; }
button { background: var(--card); color: var(--fg); border: 1px solid var(--line); border-radius: 6px; padding: 4px 10px; cursor: pointer; }
button:hover { border-color: var(--accent); } label { margin-right: 10px; white-space: nowrap; }
.picked { color: var(--accent); font-weight: 600; } textarea { width: 100%; height: 120px; font: 12px monospace; background: var(--card); color: var(--fg); }
"""

SCRIPT = """
const fight = document.getElementById('fight');
let current = null;
function play(src) {
  if (current) { current.pause(); }
  fight.pause();
  const take = new Audio(src);
  current = take;
  if (!fight.src) { take.play(); return; }
  fight.currentTime = 0;
  fight.onended = () => setTimeout(() => { if (current === take) take.play(); }, 250);
  fight.play();
}
function alone(src) { if (current) current.pause(); fight.pause(); current = new Audio(src); current.play(); }
const {picks, picksFile} = SHEET_DATA;
function render() {
  document.getElementById('picks').value = JSON.stringify(picks, null, 2) + "\\n";
  document.querySelectorAll('input[type=radio]').forEach(r => {
    const p = picks.lines[r.name];
    r.checked = !!p && p.take_key === r.value;
    r.parentElement.classList.toggle('picked', r.checked);
  });
}
function choose(stem, take, key) { picks.lines[stem] = {take: take, take_key: key}; render(); }
async function save() {
  const out = document.getElementById('saved');
  if (location.protocol.startsWith('http')) {
    const r = await fetch('/picks', {method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(picks)});
    out.textContent = r.ok ? 'Saved to tools/voice/' + picksFile + '.' : 'Not saved: ' + await r.text();
  } else {
    const a = document.createElement('a');
    a.href = URL.createObjectURL(new Blob([JSON.stringify(picks, null, 2) + "\\n"], {type: 'application/json'}));
    a.download = picksFile; a.click();
    out.textContent = 'Downloaded. Move it to tools/voice/' + picksFile + '.';
  }
}
render();
"""


def _e(s):
    return html.escape(str(s), quote=True)


def build(data):
    """Render the voice, takes, picks and commands with paths relative to the sheet.
    rounds: [{"round": n, "previews": [{"n": i, "heard": rel or None, "save": "command"}]}].
    lines: [{"stem", "text", "prompt", "takes": [{"take", "key", "heard": rel or None, "seconds"}]}]."""
    out = ["<!doctype html><html lang='en'><head><meta charset='utf-8'>",
           "<meta name='viewport' content='width=device-width, initial-scale=1'>",
           "<title>Announcer listening sheet</title>", f"<style>{STYLE}</style></head><body>",
           "<h1>The announcer: listening sheet</h1>",
           "<p class='muted'>EH9 (#225). Each button plays the ROM's own FIGHT! and then the take, both through the "
           "cabinet's MSM5205 at 4 kHz. Everything here is encoded ADPCM: what the player hears.</p>"]
    fight = data.get("fight")
    if fight:
        out.append(f"<audio id='fight' preload='auto' src='{_e(fight)}'></audio>"
                   f"<p><button onclick=\"alone('{_e(fight)}')\">The ROM's FIGHT! alone</button></p>")
    else:
        out.append("<audio id='fight'></audio><p><strong>The ROM's FIGHT! is missing</strong> (no ROM set at "
                   "tools/reference/local/roms/sf): takes play alone.</p>")

    v = data["voice"]
    out.append("<h2>The voice</h2>")
    out.append(f"<p>Voice: <code>{_e(v.get('voice_id') or 'not saved yet')}</code>"
               + (f" ({_e(v['picked'])})" if v.get("picked") else "") + "</p>")
    out.append(f"<p class='muted'>Designed from: {_e(v['description'])}</p>")
    for rnd in data["rounds"]:
        out.append(f"<p>Round {rnd['round']}: ")
        for p in rnd["previews"]:
            if p["heard"]:
                out.append(f"<button onclick=\"play('{_e(p['heard'])}')\">FIGHT! then preview {p['n']}</button> ")
        out.append("</p><p class='muted'>To keep one: " + "; ".join(f"<code>{_e(p['save'])}</code>" for p in rnd["previews"]) + "</p>")
    if not data["rounds"]:
        out.append(f"<p class='muted'>No design round yet. <code>{_e(data['design_command'])}</code></p>")

    out.append("<h2>The lines</h2><table><tr><th>Line</th><th>Takes (FIGHT! first)</th><th>Pick</th></tr>")
    for line in data["lines"]:
        plays, radios = [], []
        for t in line["takes"]:
            if t["heard"]:
                plays.append(f"<button onclick=\"play('{_e(t['heard'])}')\">take {t['take']}</button>"
                             f" <span class='muted'>{t['seconds']:.2f}s</span>")
                radios.append(f"<label><input type='radio' name='{_e(line['stem'])}' value='{_e(t['key'])}' "
                              f"onchange=\"choose('{_e(line['stem'])}', {t['take']}, '{_e(t['key'])}')\"> {t['take']}</label>")
        out.append(f"<tr><td><strong>{_e(line['text'])}</strong><br><code class='muted'>{_e(line['stem'])}</code>"
                   f" <span class='muted'>sent: {_e(line['prompt'])}</span></td>"
                   f"<td>{' '.join(plays) or '<span class=muted>not voiced yet</span>'}</td><td>{''.join(radios)}</td></tr>")
    out.append("</table>")
    out.append("<h2>Picks</h2><p><button onclick='save()'>Save picks</button> <span id='saved' class='muted'></span></p>"
               "<textarea id='picks' readonly></textarea>"
               f"<p class='muted'>Build the voice pack with <code>{_e(data['pack_command'])}</code>.</p>")
    script_data = json.dumps({"picks": data["picks"], "picksFile": data["picks_file"]}).replace("<", "\\u003c")
    out.append(f"<script>{SCRIPT.replace('SHEET_DATA', script_data)}</script></body></html>")
    return "\n".join(out) + "\n"
