"""The announcer's voice pipeline (EH9, #225).

    python tools/voice/pipeline.py status                         # what a run would do; no network, no cost
    python tools/voice/pipeline.py design --max-credits N         # one Voice Design round: three previews (paid)
    python tools/voice/pipeline.py design --save P [--round R]    # keep preview P as the voice (free)
    python tools/voice/pipeline.py run --max-credits N [--stop-at N]   # voice new or changed takes (paid)
    python tools/voice/pipeline.py sheet                          # rebuild the listening sheet (free)
    python tools/voice/pipeline.py listen [--port 8765]           # serve the sheet; its Save writes picks.json
    python tools/voice/pipeline.py pick <stem> <take>             # or record one pick by hand
    python tools/voice/pipeline.py build-pack                     # write sf1-port-voice from the picks

In the repository (tools/voice/): the line list (lines.json), the speaker with
the voice's description and settings (announcer.json), the maintainer's picks
(picks.json) and the credit ledger (ledger.json). The playable audio and
authoring takes live in assets/sf1-port-voice. The pack's archive/ folder
contains takes, previews and locally generated reference audio
(docs/reference/announcer.md).

A run:

1. Works out each take's key, a hash of everything the vendor is sent (the
   prompt, voice id, model, settings, seed and output format). Take t of a
   line is sent with the voice's seed + t, so the three takes differ. A take
   whose key is archived costs nothing; anything else is generated, in line
   order, until the run's credit budget runs out.
2. Archives every take the moment it arrives, before anything else touches
   it: archive/takes/<stem>/<key>.wav with a .json sidecar holding the
   prompt, every setting, the seed and the key. Takes are never deleted.
3. Processes each take (free, from its archived original): trims the
   silence, downsamples to 4 kHz mono 16-bit, normalises the peak
   (audio.PROCESS), encodes it to 4-bit MSM5205 ADPCM with the repo's own
   encoder and decodes it back for listening. Changing audio.PROCESS
   re-processes every take; nothing is paid again.
4. Writes the listening sheet: every take after the ROM's own FIGHT!.

The API key is read only by el.py, from the ELEVENLABS_API_KEY user
environment variable, and never printed or written. A paid command needs
--max-credits: it plans at the list price of 1 credit a character and stops
short of that budget, of --stop-at (default 14,000, under the key's 15,000
cap) and of the plan's limit, reading the vendor's counter before it spends.
"""

import argparse
import datetime
import hashlib
import json
import math
import os
import pathlib
import sys
from dataclasses import dataclass, field

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import audio  # noqa: E402
import sheet  # noqa: E402

REPO = pathlib.Path(__file__).resolve().parents[2]
FORMAT = "pcm_24000"  # the best PCM the Starter tier gets
STOP_AT = 14000
CREDITS_PER_CHAR = 1.0  # planning ceiling: the vendor's list price
# Rates measured on a Starter plan (2026-10-20): Voice Design charges
# 1 credit per preview-text character per round, for all three previews; saving
# a voice is free; text to speech charged 0.39 to 0.40 credits per prompt
# character, tags included. Starter was $6 a month for 40,000 credits there.
MEASURED_TTS_PER_CHAR = 0.40
STARTER_USD, STARTER_CREDITS = 6.0, 40000
WORDS = ["super", "perfect", "perfect_parry", "counter", "punish_counter", "crush_counter", "alpha_counter",
         "reversal", "first_attack", "burnout", "custom_combo"]  # sf_video::announcer::AnnouncerLine::WORDS
COMBO_MIN = 5  # sf_video::announcer::COMBO_MIN_HITS


class Paths:
    def __init__(self, repo=REPO, roms=None, binary=None):
        self.repo = pathlib.Path(repo)
        self.tool = self.repo / "tools" / "voice"
        self.lines = self.tool / "lines.json"
        self.speaker = self.tool / "announcer.json"
        self.picks = self.tool / "picks.json"
        self.ledger = self.tool / "ledger.json"
        self.roms = pathlib.Path(roms) if roms else self.repo / "tools" / "reference" / "local" / "roms"
        self.rom_set = self.roms / "sf"
        self.rom_manifest = self.repo / "tools" / "reference" / "rom-manifest.json"
        self.pack = self.repo / "assets" / "sf1-port-voice"
        self.archive = self.pack / "archive"
        self.design = self.archive / "design"
        self.takes = self.archive / "takes"
        self.fight = self.archive / "rom" / "fight.heard.wav"
        self.sheet = self.archive / "sheet.html"
        self.binary = pathlib.Path(binary) if binary else audio.binary_path(self.repo)

    def take_file(self, stem, key):
        return self.takes / stem / f"{key[:16]}.wav"


def derived(raw):
    """A take's (or preview's) processed files, beside its archived original."""
    raw = pathlib.Path(raw)
    base = raw.with_suffix("")
    return {"wav4k": base.with_name(base.name + ".4k.wav"), "vox": base.with_name(base.name + ".vox"),
            "heard": base.with_name(base.name + ".heard.wav"), "proc": base.with_name(base.name + ".proc.json")}


def digest(obj):
    return hashlib.sha256(json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()).hexdigest()


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


def read_json(path):
    return json.loads(pathlib.Path(path).read_text(encoding="utf-8"))


def write_json(path, data):
    path = pathlib.Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    os.replace(tmp, path)


# --- Content ------------------------------------------------------------------------

@dataclass(frozen=True)
class Line:
    stem: str
    text: str
    prompt: str


def load_lines(paths):
    """(lines, takes per line). Refused unless every stem is one of his lines,
    each once, with the combo counts running from five to the cap."""
    doc = read_json(paths.lines)
    lines = [Line(x["stem"], x["text"], x.get("prompt", x["text"])) for x in doc["lines"]]
    stems = [x.stem for x in lines]
    if len(set(stems)) != len(stems):
        raise SystemExit(f"{paths.lines}: a stem is listed twice")
    cap = doc["combo_cap"]
    want = WORDS + [f"combo_{n}" for n in range(COMBO_MIN, cap + 1)]
    if sorted(stems) != sorted(want):
        raise SystemExit(f"{paths.lines}: the lines must be exactly {want}")
    for x in lines:
        if not x.text.strip() or not x.prompt.strip():
            raise SystemExit(f"{paths.lines}: {x.stem} has no text")
    return lines, doc["takes"]


def load_speaker(paths):
    return read_json(paths.speaker)


def load_picks(paths):
    return read_json(paths.picks) if paths.picks.exists() else {"format": 1, "lines": {}}


def request_for(line, voice, take):
    """The text-to-speech body for take `take` (1-based) of a line."""
    return {"text": line.prompt, "model_id": voice["model"], "voice_settings": dict(voice["settings"]),
            "seed": voice["seed"] + take}


def take_key(line, voice, take):
    return digest({"vendor": voice["vendor"], "voice_id": voice["voice_id"], "request": request_for(line, voice, take),
                   "output_format": FORMAT})


def proc_key(key):
    return digest({"take": key, "process": audio.PROCESS})


# --- Planning ----------------------------------------------------------------------------

@dataclass
class Item:
    line: Line
    take: int
    action: str  # generate, process, keep, waiting
    why: str = ""
    key: str = ""

    @property
    def chars(self):
        return len(self.line.prompt)

    @property
    def cost(self):
        return math.ceil(self.chars * CREDITS_PER_CHAR) if self.action == "generate" else 0


@dataclass
class Plan:
    voice: dict
    items: list
    takes: int
    rounds: list = field(default_factory=list)

    def by(self, *actions):
        return [i for i in self.items if i.action in actions]


def _sidecars(paths, stem, take):
    folder = paths.takes / stem
    found = []
    for p in folder.glob("*.json") if folder.exists() else []:
        if p.name.endswith(".proc.json"):
            continue
        s = read_json(p)
        if s.get("take") == take:
            found.append(s)
    return sorted(found, key=lambda s: s.get("made", ""))


def _changed(paths, line, voice, take):
    """Why a take is not archived, in words."""
    old = _sidecars(paths, line.stem, take)
    if not old:
        return "new line"
    last, body = old[-1], request_for(line, voice, take)
    what = [w for w, a, b in [("prompt", last["request"]["text"], body["text"]),
                              ("voice", last["voice_id"], voice["voice_id"]),
                              ("model", last["request"]["model_id"], body["model_id"]),
                              ("settings", last["request"]["voice_settings"], body["voice_settings"]),
                              ("seed", last["request"]["seed"], body["seed"])] if a != b]
    return "changed: " + ", ".join(what or ["request"])


def processed(raw):
    """Whether a take's processed files are current."""
    d = derived(raw)
    if not all(d[k].exists() for k in ("wav4k", "vox", "heard", "proc")):
        return False
    return read_json(d["proc"]).get("proc_key") == proc_key(read_json(pathlib.Path(raw).with_suffix(".json"))["take_key"])


def design_rounds(paths):
    found = paths.design.glob("round*.json") if paths.design.exists() else []
    rounds = sorted((int(p.stem[5:]), p) for p in found if p.stem[5:].isdigit())
    return [read_json(p) | {"round": n} for n, p in rounds]


def plan(paths):
    lines, takes = load_lines(paths)
    voice = load_speaker(paths)["voice"]
    items = []
    for line in lines:
        for t in range(1, takes + 1):
            if not voice.get("voice_id"):
                items.append(Item(line, t, "waiting", "the voice isn't designed and saved yet (pipeline.py design)"))
                continue
            key = take_key(line, voice, t)
            raw = paths.take_file(line.stem, key)
            if not raw.exists():
                items.append(Item(line, t, "generate", _changed(paths, line, voice, t), key))
            elif not processed(raw):
                items.append(Item(line, t, "process", "not yet encoded at its current processing", key))
            else:
                items.append(Item(line, t, "keep", "", key))
    return Plan(voice, items, takes, design_rounds(paths))


def estimate(paths):
    """A first full run's credits: one Voice Design round and every take of every line."""
    lines, takes = load_lines(paths)
    design = load_speaker(paths)["voice"]["design"]
    preview = len(design["preview_text"])
    chars = sum(len(x.prompt) for x in lines) * takes
    measured = preview + math.ceil(chars * MEASURED_TTS_PER_CHAR)
    listed = preview + math.ceil(chars * CREDITS_PER_CHAR)
    per_credit = STARTER_USD / STARTER_CREDITS
    return {"design_round_credits": preview, "lines": len(lines), "takes": len(lines) * takes,
            "take_characters": chars, "credits_planned_at_list_price": listed,
            "credits_at_highfall_measured_rate": measured,
            "usd_share_of_starter_plan": {"at_list_price": round(listed * per_credit, 2),
                                          "at_measured_rate": round(measured * per_credit, 2)}}


# --- The vendor ------------------------------------------------------------------------------

class ElevenLabs:
    """The real vendor, through el.py (which alone reads the API key)."""

    def __init__(self, paths, log=print):
        import el
        self.el = el
        self.ledger = el.Ledger(paths.ledger, log=log)

    def subscription(self):
        return self.el.subscription()

    def batch(self, name, note):
        return self.ledger.batch(name, note)

    def tts(self, voice_id, body):
        return self.el.tts(voice_id, body)

    def design(self, body):
        return self.el.design(body)

    def save_voice(self, body):
        return self.el.save_voice(body)


def _budget(vendor, max_credits, stop_at, log):
    sub = vendor.subscription()
    ceiling = min(sub["character_limit"], stop_at)
    room = min(ceiling - sub["character_count"], max_credits)
    log(f"credits: {sub['character_count']} of {sub['character_limit']} used; stopping at {ceiling}; "
        f"this run may spend at most {max(room, 0)}")
    return room


def _need_budget(max_credits, planned, what):
    if max_credits is None:
        raise SystemExit(f"{what} costs money: give it a budget with --max-credits N "
                         f"(it plans at most {planned} credits at the list price)")


# --- Processing (free) -----------------------------------------------------------------------

def process(paths, raw, codec, key):
    d = derived(raw)
    info = audio.prepare(raw, d["wav4k"])
    enc = codec.encode(d["wav4k"], d["vox"], d["heard"])
    write_json(d["proc"], {"take_key": key, "proc_key": proc_key(key), "process": audio.PROCESS,
                           "nibbles": enc["nibbles"], "crc32": enc["crc32"], "seconds": enc["seconds"],
                           "seconds_raw": info["seconds_raw"], "made": now()})


def ensure_fight(paths, codec, log=print):
    """The ROM's FIGHT! (samples command 0x94) as its chip plays it, once. Free."""
    if paths.fight.exists():
        return True
    if not paths.rom_set.is_dir() or not codec.ready():
        log("  the ROM's FIGHT! is not extracted (needs the ROM set and the sf-audio binary)")
        return False
    info = codec.rom_sample("0x94", paths.fight)
    write_json(paths.fight.with_suffix(".json"), info | {"what": "the ROM's FIGHT!, samples-CPU command 0x94"})
    log(f"  extracted the ROM's FIGHT! ({info['nibbles']} nibbles)")
    return True


def _codec(paths):
    return audio.Codec(paths.binary, paths.rom_set, paths.rom_manifest, paths.repo)


# --- Running ----------------------------------------------------------------------------------

def run(paths, vendor=None, codec=None, max_credits=None, stop_at=STOP_AT, log=print):
    """Voices every new or changed take and processes every take not yet
    encoded. Returns a summary of what happened."""
    p = plan(paths)
    codec = codec or _codec(paths)
    paid = p.by("generate")
    chosen, deferred = [], []
    if paid:
        _need_budget(max_credits, sum(i.cost for i in paid), "This run")
        vendor = vendor or ElevenLabs(paths, log)
        room, spend = _budget(vendor, max_credits, stop_at, log), 0
        for item in paid:
            if spend + item.cost > room:
                break
            chosen.append(item)
            spend += item.cost
        deferred = paid[len(chosen):]
    if chosen:
        chars = sum(i.chars for i in chosen)
        with vendor.batch(f"voice: {len(chosen)} takes", f"{chars} prompt characters"):
            for item in chosen:
                body = request_for(item.line, p.voice, item.take)
                pcm = vendor.tts(p.voice["voice_id"], body)
                raw = paths.take_file(item.line.stem, item.key)
                audio.write_wav(raw, 24000, pcm)
                # The take and its sidecar are saved before the next paid call, so a stopped run loses nothing.
                write_json(raw.with_suffix(".json"), {
                    "stem": item.line.stem, "take": item.take, "text": item.line.text, "prompt": item.line.prompt,
                    "vendor": p.voice["vendor"], "voice_id": p.voice["voice_id"], "request": body,
                    "output_format": FORMAT, "take_key": item.key, "seconds": round(len(pcm) / 2 / 24000, 3),
                    "made": now()})
                log(f"  generated {item.line.stem} take {item.take} ({item.why})")
    done = []
    for item in chosen + p.by("process"):
        process(paths, paths.take_file(item.line.stem, item.key), codec, item.key)
        done.append(f"{item.line.stem}/{item.take}")
        log(f"  encoded {item.line.stem} take {item.take}")
    if deferred:
        log(f"stopped at this run's credit budget: {len(deferred)} takes (at most {sum(i.cost for i in deferred)} "
            "credits) wait for the next run")
    write_sheet(paths, codec, log)
    return {"generated": [f"{i.line.stem}/{i.take}" for i in chosen], "encoded": done,
            "deferred": [f"{i.line.stem}/{i.take}" for i in deferred],
            "kept": len(p.by("keep")), "waiting": len(p.by("waiting"))}


# --- Designing the voice -----------------------------------------------------------------------

def design(paths, vendor=None, codec=None, max_credits=None, stop_at=STOP_AT, log=print):
    """One Voice Design round from the speaker's description: three previews,
    archived with the request, then encoded for the sheet."""
    record = load_speaker(paths)
    spec = record["voice"]["design"]
    cost = math.ceil(len(spec["preview_text"]) * CREDITS_PER_CHAR)
    _need_budget(max_credits, cost, "A design round")
    vendor = vendor or ElevenLabs(paths, log)
    if _budget(vendor, max_credits, stop_at, log) < cost:
        raise SystemExit(f"a design round costs {cost} credits, more than this run may spend")
    n = len(design_rounds(paths)) + 1
    body = {"voice_description": spec["description"], "model_id": spec["model"], "text": spec["preview_text"],
            "loudness": spec["loudness"], "guidance_scale": spec["guidance_scale"], "seed": spec["seed"] + n}
    # Saved inside the batch: the ledger's poll runs after, and a paid result must never be lost to it.
    with vendor.batch(f"design round {n}", f"{spec['model']}, {len(spec['preview_text'])} preview characters"):
        previews, text = vendor.design(body)
        out = []
        for i, (pcm, generated_id, secs) in enumerate(previews, 1):
            wav = paths.design / f"round{n}_preview{i}.wav"
            audio.write_wav(wav, 24000, pcm)
            out.append({"file": wav.name, "generated_voice_id": generated_id, "duration_secs": secs})
        write_json(paths.design / f"round{n}.json", {"request": body, "output_format": FORMAT, "text": text,
                                                     "previews": out, "made": now()})
    codec = codec or _codec(paths)
    for prev in out:
        raw = paths.design / prev["file"]
        process(paths, raw, codec, digest({"design": n, "file": prev["file"]}))
    log(f"design round {n}: {len(out)} previews in {paths.design}")
    write_sheet(paths, codec, log)
    return n


def save_voice(paths, preview, round_=None, vendor=None, log=print):
    """Keep preview `preview` of round `round_` (default: the latest) as the
    voice: free. Every take's key changes, so the next run voices every line."""
    rounds = design_rounds(paths)
    if not rounds:
        raise SystemExit("no design round yet: run `pipeline.py design --max-credits N` first")
    chosen = next((r for r in rounds if r["round"] == round_), None) if round_ else rounds[-1]
    if chosen is None:
        raise SystemExit(f"there is no design round {round_}")
    prev = chosen["previews"][preview - 1]
    record = load_speaker(paths)
    v = record["voice"]
    body = {"voice_name": "sf1-port - the announcer", "voice_description": v["design"]["description"],
            "generated_voice_id": prev["generated_voice_id"]}
    vendor = vendor or ElevenLabs(paths, log)
    v["voice_id"] = vendor.save_voice(body)
    v["design"]["picked"] = f"round {chosen['round']}, preview {preview} (archive/design/{prev['file']})"
    write_json(paths.speaker, record)
    log(f"the announcer's voice is now {v['voice_id']} ({v['design']['picked']})")
    return v["voice_id"]


# --- The listening sheet and picks -----------------------------------------------------------

def _rel(paths, path):
    return pathlib.Path(os.path.relpath(path, paths.archive)).as_posix()


def write_sheet(paths, codec=None, log=print):
    codec = codec or _codec(paths)
    has_fight = ensure_fight(paths, codec, log)
    p = plan(paths)
    voice = p.voice
    rounds = []
    for r in p.rounds:
        previews = []
        for i, prev in enumerate(r["previews"], 1):
            heard = derived(paths.design / prev["file"])["heard"]
            previews.append({"n": i, "heard": _rel(paths, heard) if heard.exists() else None,
                             "save": f"python tools/voice/pipeline.py design --save {i} --round {r['round']}"})
        rounds.append({"round": r["round"], "previews": previews})
    lines = {}
    for item in p.items:
        entry = lines.setdefault(item.line.stem, {"stem": item.line.stem, "text": item.line.text,
                                                  "prompt": item.line.prompt, "takes": []})
        heard, secs = None, 0.0
        if item.action == "keep":
            raw = paths.take_file(item.line.stem, item.key)
            heard = _rel(paths, derived(raw)["heard"])
            secs = read_json(derived(raw)["proc"])["seconds"]
        entry["takes"].append({"take": item.take, "key": item.key, "heard": heard, "seconds": secs})
    page = sheet.build({"fight": _rel(paths, paths.fight) if has_fight else None,
                        "voice": {"voice_id": voice.get("voice_id"), "picked": voice["design"].get("picked"),
                                  "description": voice["design"]["description"]},
                        "rounds": rounds, "lines": list(lines.values()), "picks": load_picks(paths)})
    paths.sheet.parent.mkdir(parents=True, exist_ok=True)
    paths.sheet.write_text(page, encoding="utf-8")
    return paths.sheet


def check_picks(paths, picks):
    """picks.json as the maintainer saved it, or why not: every stem one of the lines,
    every take an archived, encoded take of that line."""
    if not isinstance(picks, dict) or not isinstance(picks.get("lines"), dict):
        raise ValueError('picks must be {"format": 1, "lines": {...}}')
    lines = {x.stem for x in load_lines(paths)[0]}
    for stem, pick in picks["lines"].items():
        if stem not in lines:
            raise ValueError(f"{stem} is not one of the lines")
        key = pick.get("take_key", "")
        raw = paths.take_file(stem, key) if len(key) >= 16 else None
        if raw is None or not raw.exists() or not processed(raw):
            raise ValueError(f"{stem}: no encoded take {key[:16]}")
    return {"format": 1, "lines": {s: {"take": p["take"], "take_key": p["take_key"]}
                                   for s, p in sorted(picks["lines"].items())}}


def pick(paths, stem, take, log=print):
    p = plan(paths)
    item = next((i for i in p.items if i.line.stem == stem and i.take == take), None)
    if item is None or item.action != "keep":
        raise SystemExit(f"{stem} take {take} is not voiced and encoded yet")
    picks = load_picks(paths)
    picks["lines"][stem] = {"take": take, "take_key": item.key}
    write_json(paths.picks, check_picks(paths, picks))
    log(f"picked {stem} take {take}")


def build_pack(paths, codec=None, log=print):
    """Writes sf1-port-voice from the picks with VoiceStore::write. A line
    with no pick, or whose pick is not one of its current takes, is left
    out (the game does not call a line the pack lacks) and reported."""
    codec = codec or _codec(paths)
    p = plan(paths)
    picks = load_picks(paths)["lines"]
    current = {}
    for i in p.items:
        current.setdefault(i.line.stem, {})[i.key] = i
    plan_lines, unpicked, stale = {}, [], []
    for stem, takes in current.items():
        chosen = picks.get(stem)
        if not chosen:
            unpicked.append(stem)
            continue
        item = takes.get(chosen["take_key"])
        if item is None or item.action != "keep":
            stale.append(stem)
            continue
        raw = paths.take_file(stem, item.key)
        plan_lines[stem] = {"vox": str(derived(raw)["vox"]), "nibbles": read_json(derived(raw)["proc"])["nibbles"]}
    if not plan_lines:
        raise SystemExit("no line is picked yet: pick takes on the listening sheet first")
    plan_file = paths.archive / "pack-plan.json"
    write_json(plan_file, {"lines": plan_lines})
    for old in paths.pack.glob("*.vox") if paths.pack.exists() else []:
        if old.stem not in plan_lines:
            old.unlink()
    info = codec.pack(plan_file, paths.pack)
    log(f"wrote {info['lines']} lines to {paths.pack}")
    if unpicked:
        log(f"  not picked yet (left out): {', '.join(unpicked)}")
    if stale:
        log(f"  picked take is not a current take (left out; pick again): {', '.join(stale)}")
    return {"written": sorted(plan_lines), "unpicked": unpicked, "stale": stale}


# --- The command ---------------------------------------------------------------------------------

def status(paths):
    """What a run would do. Reads local files only: no network, no key, no cost."""
    p = plan(paths)
    counts = {}
    for i in p.items:
        counts[i.action] = counts.get(i.action, 0) + 1
    picks = load_picks(paths)["lines"]
    keys = {(i.line.stem, i.key) for i in p.items if i.action == "keep"}
    stems = sorted({i.line.stem for i in p.items})
    return {
        "voice": p.voice.get("voice_id") or "not designed yet",
        "design_rounds": len(p.rounds),
        "lines": len(stems), "takes_per_line": p.takes,
        "actions": counts,
        "credits_planned": sum(i.cost for i in p.items),
        "credits_planned_once_designed": sum(i.chars for i in p.items if i.action in ("waiting", "generate")),
        "picks": {"picked": sum(1 for s in stems if s in picks and (s, picks[s]["take_key"]) in keys),
                  "not_picked": sum(1 for s in stems if s not in picks),
                  "stale": sum(1 for s in stems if s in picks and (s, picks[s]["take_key"]) not in keys)},
        "first_full_run": estimate(paths),
        "ready": {"sf-audio binary": paths.binary.exists(), "ffmpeg": bool(audio.shutil.which("ffmpeg")),
                  "ROM set (for FIGHT!)": paths.rom_set.is_dir(), "voice pack": (paths.pack / "manifest.json").exists()},
    }


def listen(paths, port, log=print):
    import functools
    import http.server

    write_sheet(paths, log=log)

    class Handler(http.server.SimpleHTTPRequestHandler):
        def do_POST(self):
            if self.path != "/picks":
                self.send_error(404)
                return
            try:
                picks = check_picks(paths, json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
            except (ValueError, KeyError, TypeError) as e:
                self.send_response(400)
                self.end_headers()
                self.wfile.write(str(e).encode())
                return
            write_json(paths.picks, picks)
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"saved")
            log(f"saved {len(picks['lines'])} picks to {paths.picks}")

    server = http.server.ThreadingHTTPServer(("127.0.0.1", port),
                                             functools.partial(Handler, directory=str(paths.archive)))
    log(f"listening sheet: http://127.0.0.1:{port}/sheet.html (Ctrl+C to stop)")
    server.serve_forever()


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("command", choices=["status", "design", "run", "sheet", "listen", "pick", "build-pack"])
    ap.add_argument("args", nargs="*", help="pick: <stem> <take>")
    ap.add_argument("--max-credits", type=int)
    ap.add_argument("--stop-at", type=int, default=STOP_AT)
    ap.add_argument("--save", type=int, help="design: keep this preview as the voice")
    ap.add_argument("--round", type=int, help="design --save: of this round (default the latest)")
    ap.add_argument("--port", type=int, default=8765)
    a = ap.parse_args(argv)
    paths = Paths()
    if a.command == "status":
        s = status(paths)
        print(json.dumps(s, indent=2))
        for i in plan(paths).items:
            if i.action not in ("keep", "waiting"):
                print(f"  {i.action:9} {i.line.stem:15} take {i.take}  {i.why}")
    elif a.command == "design":
        if a.save:
            save_voice(paths, a.save, a.round)
        else:
            design(paths, max_credits=a.max_credits, stop_at=a.stop_at)
    elif a.command == "run":
        print(json.dumps(run(paths, max_credits=a.max_credits, stop_at=a.stop_at), indent=2))
    elif a.command == "sheet":
        print(write_sheet(paths))
    elif a.command == "listen":
        listen(paths, a.port)
    elif a.command == "pick":
        if len(a.args) != 2:
            ap.error("pick needs <stem> <take>")
        pick(paths, a.args[0], int(a.args[1]))
    elif a.command == "build-pack":
        print(json.dumps(build_pack(paths), indent=2))


if __name__ == "__main__":
    main()
