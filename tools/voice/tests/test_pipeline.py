"""Headless tests for the announcer's voice pipeline (EH9, #225).

    python -m unittest discover -s tools/voice/tests

No network and no cost: a fake vendor stands in for ElevenLabs, and the key
tests run the real el.py against a mocked urlopen with a made-up key. Most
tests stand in for the codec and the ffmpeg step too; the end-to-end test
uses the real `sf-audio` binary and ffmpeg and skips without them.
"""

import array
import contextlib
import io
import json
import math
import pathlib
import re
import shutil
import sys
import tempfile
import unittest
import urllib.error
import wave
import zlib
from unittest import mock

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import audio  # noqa: E402
import el  # noqa: E402
import pipeline  # noqa: E402

REPO = HERE.parents[2]
SECRET = "sk_test_THIS-IS-NOT-A-REAL-KEY-0123456789abcdef"


def tone_pcm(text, rate=24000):
    """A fake take: 0.2 s of silence, a tone of 50 ms a character, 0.2 s of silence."""
    out = array.array("h", [0] * int(0.2 * rate))
    n = int(0.05 * rate) * len(text)
    out.extend(int(12000 * math.sin(2 * math.pi * 220 * i / rate)) for i in range(n))
    out.extend([0] * int(0.2 * rate))
    return out.tobytes()


class FakeVendor:
    def __init__(self, count=0, limit=40000, fail_after=None):
        self.count, self.limit, self.fail_after = count, limit, fail_after
        self.calls, self.batches, self.designs, self.saved = [], [], [], []

    def subscription(self):
        return {"character_count": self.count, "character_limit": self.limit}

    @contextlib.contextmanager
    def batch(self, name, note):
        self.batches.append(name)
        yield {}

    def tts(self, voice_id, body):
        if self.fail_after is not None and len(self.calls) >= self.fail_after:
            raise RuntimeError("the vendor fell over")
        self.calls.append((voice_id, body))
        self.count += len(body["text"])
        return tone_pcm(body["text"])

    def design(self, body):
        self.designs.append(body)
        self.count += len(body["text"])
        return [(tone_pcm("preview" * (i + 1)), f"gen{i}", 1.0) for i in range(3)], body["text"]

    def save_voice(self, body):
        self.saved.append(body)
        return "VoiceAAAAAAAAAAAA"


class FakeCodec:
    """Stands in for `sf-audio`: a .vox of one byte per two samples, the 4 kHz WAV as the heard file."""

    def __init__(self):
        self.encoded, self.packs = [], []

    def ready(self):
        return True

    def encode(self, wav_4k, vox, heard):
        _, s = audio.read_wav(wav_4k)
        data = bytes((len(s) + 1) // 2)
        pathlib.Path(vox).write_bytes(data)
        shutil.copyfile(wav_4k, heard)
        self.encoded.append(str(wav_4k))
        return {"nibbles": len(s), "crc32": f"{zlib.crc32(data):08x}", "seconds": len(s) / 4000}

    def rom_sample(self, command, out):
        audio.write_wav(out, 48000, [0] * 480)
        return {"command": command, "nibbles": 40, "seconds": 0.01}

    def pack(self, plan, folder):
        lines = json.loads(pathlib.Path(plan).read_text(encoding="utf-8"))["lines"]
        self.packs.append(lines)
        pathlib.Path(folder).mkdir(parents=True, exist_ok=True)
        (pathlib.Path(folder) / "manifest.json").write_text(json.dumps({"lines": sorted(lines)}), encoding="utf-8")
        return {"lines": len(lines)}


def fast_prepare(raw, out_4k, spec=audio.PROCESS):
    rate, s = audio.read_wav(raw)
    audio.write_wav(out_4k, 4000, s[::6])
    return {"seconds_raw": len(s) / rate, "seconds": len(s) / rate}


def quiet(*_):
    pass


class Fixture(unittest.TestCase):
    """A temp repo holding the real line list and speaker (combos capped at 6 to keep it small)."""

    combo_cap = 6

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = pathlib.Path(self.tmp.name)
        (root / "tools" / "voice").mkdir(parents=True)
        lines = json.loads((REPO / "tools" / "voice" / "lines.json").read_text(encoding="utf-8"))
        lines["combo_cap"] = self.combo_cap
        lines["lines"] = [x for x in lines["lines"]
                          if not x["stem"].startswith("combo_") or int(x["stem"][6:]) <= self.combo_cap]
        (root / "tools" / "voice" / "lines.json").write_text(json.dumps(lines), encoding="utf-8")
        # The speaker as it stood before any voice was designed, so each test
        # starts undesigned whatever the repository's voice is now.
        speaker = json.loads((REPO / "tools" / "voice" / "announcer.json").read_text(encoding="utf-8"))
        speaker["voice"]["voice_id"] = None
        speaker["voice"]["design"].pop("picked", None)
        (root / "tools" / "voice" / "announcer.json").write_text(json.dumps(speaker), encoding="utf-8")
        self.paths = pipeline.Paths(root, roms=root / "roms", binary=root / "no-binary")
        self.codec = FakeCodec()
        patcher = mock.patch.object(pipeline.audio, "prepare", fast_prepare)
        patcher.start()
        self.addCleanup(patcher.stop)

    def tearDown(self):
        self.tmp.cleanup()

    def designed(self, voice_id="VoiceAAAAAAAAAAAA"):
        self.edit_speaker(lambda v: v.update(voice_id=voice_id))

    def edit_speaker(self, change):
        record = json.loads(self.paths.speaker.read_text(encoding="utf-8"))
        change(record["voice"])
        self.paths.speaker.write_text(json.dumps(record), encoding="utf-8")

    def edit_line(self, stem, prompt):
        doc = json.loads(self.paths.lines.read_text(encoding="utf-8"))
        next(x for x in doc["lines"] if x["stem"] == stem)["prompt"] = prompt
        self.paths.lines.write_text(json.dumps(doc), encoding="utf-8")

    def run_pipeline(self, vendor=None, max_credits=10000, **kw):
        return pipeline.run(self.paths, vendor=vendor or FakeVendor(), codec=self.codec, max_credits=max_credits,
                            log=quiet, **kw)

    def actions(self):
        return [(i.line.stem, i.take, i.action) for i in pipeline.plan(self.paths).items]

    @property
    def n_lines(self):
        return 11 + self.combo_cap - 4


class StatusTest(Fixture):
    def test_status_costs_nothing_and_touches_no_network(self):
        def no_network(*a, **k):
            raise AssertionError("status touched the network")

        with mock.patch("urllib.request.urlopen", no_network), mock.patch("socket.socket.connect", no_network), \
                mock.patch.object(el, "_key", side_effect=AssertionError("status read the key")):
            s = pipeline.status(self.paths)
            self.assertEqual(s["voice"], "not designed yet")
            self.assertEqual(s["actions"], {"waiting": 3 * self.n_lines})
            self.assertEqual(s["credits_planned"], 0)
            self.designed()
            self.run_pipeline()
            s = pipeline.status(self.paths)
            self.assertEqual(s["actions"], {"keep": 3 * self.n_lines})
            # And the command on the real repository's content.
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                pipeline.main(["status"])
            self.assertIn('"first_full_run"', out.getvalue())

    def test_status_reports_what_a_run_would_spend(self):
        self.designed()
        s = pipeline.status(self.paths)
        chars = sum(len(i.line.prompt) for i in pipeline.plan(self.paths).items)
        self.assertEqual(s["credits_planned"], chars)
        self.assertEqual(s["actions"], {"generate": 3 * self.n_lines})

    def test_the_first_full_run_estimate(self):
        e = pipeline.estimate(pipeline.Paths(REPO))
        lines = json.loads((REPO / "tools" / "voice" / "lines.json").read_text(encoding="utf-8"))["lines"]
        preview = json.loads((REPO / "tools" / "voice" / "announcer.json").read_text(encoding="utf-8"))["voice"]["design"]["preview_text"]
        chars = 3 * sum(len(x["prompt"]) for x in lines)
        self.assertEqual(e["lines"], 22)
        self.assertEqual(e["takes"], 66)
        self.assertEqual(e["design_round_credits"], len(preview))
        self.assertEqual(e["take_characters"], chars)
        self.assertEqual(e["credits_planned_at_list_price"], len(preview) + chars)
        self.assertEqual(e["credits_at_highfall_measured_rate"], len(preview) + math.ceil(chars * 0.40))


class RunTest(Fixture):
    def test_nothing_is_voiced_before_the_voice_is_designed(self):
        vendor = FakeVendor()
        summary = self.run_pipeline(vendor)
        self.assertEqual(vendor.calls, [])
        self.assertEqual(summary["waiting"], 3 * self.n_lines)

    def test_a_first_run_voices_three_takes_of_every_line_and_a_rerun_does_nothing(self):
        self.designed()
        vendor = FakeVendor()
        summary = self.run_pipeline(vendor)
        self.assertEqual(len(vendor.calls), 3 * self.n_lines)
        self.assertEqual(len(summary["encoded"]), 3 * self.n_lines)
        seeds = {body["seed"] for _, body in vendor.calls if body["text"] == "SUPER!"}
        self.assertEqual(len(seeds), 3, "the three takes are sent with three seeds")
        again = FakeVendor()
        summary = self.run_pipeline(again)
        self.assertEqual(again.calls, [])
        self.assertEqual(again.batches, [], "a run with nothing to voice never opens a ledger batch")
        self.assertEqual(summary["generated"], [])
        self.assertEqual(summary["encoded"], [])

    def test_every_take_is_archived_with_its_prompt_settings_and_seed_and_encoded(self):
        self.designed()
        self.run_pipeline()
        voice = json.loads(self.paths.speaker.read_text(encoding="utf-8"))["voice"]
        for item in pipeline.plan(self.paths).items:
            raw = self.paths.take_file(item.line.stem, item.key)
            side = json.loads(raw.with_suffix(".json").read_text(encoding="utf-8"))
            self.assertEqual(side["prompt"], item.line.prompt)
            self.assertEqual(side["request"], {"text": item.line.prompt, "model_id": voice["model"],
                                               "voice_settings": voice["settings"], "seed": voice["seed"] + item.take})
            self.assertEqual(side["voice_id"], voice["voice_id"])
            self.assertEqual(side["take_key"], item.key)
            d = pipeline.derived(raw)
            for k in ("wav4k", "vox", "heard", "proc"):
                self.assertTrue(d[k].exists(), f"{item.line.stem} {k}")
            self.assertTrue(str(raw).startswith(str(self.paths.archive)), "takes live in the local archive")

    def test_a_changed_prompt_revoices_that_line_alone_and_keeps_its_old_takes(self):
        self.designed()
        self.run_pipeline()
        old = sorted((self.paths.takes / "burnout").glob("*.wav"))
        self.edit_line("burnout", "[shouting] BURNOUT!")
        why = {i.why for i in pipeline.plan(self.paths).items if i.action == "generate"}
        self.assertEqual(why, {"changed: prompt"})
        vendor = FakeVendor()
        summary = self.run_pipeline(vendor)
        self.assertEqual(sorted(summary["generated"]), ["burnout/1", "burnout/2", "burnout/3"])
        for p in old:
            self.assertTrue(p.exists(), "old takes are never deleted")
        # Going back costs nothing: the old takes are still archived.
        self.edit_line("burnout", "BURNOUT!")
        back = FakeVendor()
        self.run_pipeline(back)
        self.assertEqual(back.calls, [])

    def test_a_new_voice_revoices_every_line(self):
        self.designed()
        self.run_pipeline()
        self.designed("VoiceBBBBBBBBBBBB")
        vendor = FakeVendor()
        self.run_pipeline(vendor)
        self.assertEqual(len(vendor.calls), 3 * self.n_lines)
        self.assertEqual({v for v, _ in vendor.calls}, {"VoiceBBBBBBBBBBBB"})

    def test_a_processing_change_encodes_again_without_spending(self):
        self.designed()
        self.run_pipeline()
        with mock.patch.dict(audio.PROCESS, {"peak_db": -3.0}):
            self.assertEqual({a for _, _, a in self.actions()}, {"process"})
            vendor = FakeVendor()
            summary = self.run_pipeline(vendor, max_credits=None)
            self.assertEqual(vendor.calls, [])
            self.assertEqual(len(summary["encoded"]), 3 * self.n_lines)
            self.assertEqual({a for _, _, a in self.actions()}, {"keep"})

    def test_a_paid_run_needs_a_budget(self):
        self.designed()
        vendor = FakeVendor()
        with self.assertRaises(SystemExit) as e:
            self.run_pipeline(vendor, max_credits=None)
        self.assertIn("--max-credits", str(e.exception))
        self.assertEqual(vendor.calls, [])

    def test_a_run_stops_at_its_budget_and_the_credit_cap(self):
        self.designed()
        vendor = FakeVendor()
        summary = self.run_pipeline(vendor, max_credits=20)
        self.assertLessEqual(sum(len(b["text"]) for _, b in vendor.calls), 20)
        self.assertTrue(summary["deferred"])
        capped = FakeVendor(count=13990)
        self.run_pipeline(capped, max_credits=10000)
        self.assertLessEqual(capped.count, pipeline.STOP_AT)
        limited = FakeVendor(count=0, limit=15)
        self.run_pipeline(limited, max_credits=10000)
        self.assertLessEqual(limited.count, 15)

    def test_a_stopped_run_keeps_every_take_it_paid_for(self):
        self.designed()
        vendor = FakeVendor(fail_after=4)
        with self.assertRaises(RuntimeError):
            self.run_pipeline(vendor)
        archived = list(self.paths.takes.rglob("*.wav"))
        self.assertEqual(len(archived), 4)
        again = FakeVendor()
        self.run_pipeline(again)
        self.assertEqual(len(again.calls), 3 * self.n_lines - 4, "a resumed run pays only for the rest")


class DesignTest(Fixture):
    def test_a_design_round_archives_three_previews_with_its_request(self):
        vendor = FakeVendor()
        with self.assertRaises(SystemExit):
            pipeline.design(self.paths, vendor=vendor, codec=self.codec, max_credits=None, log=quiet)
        with self.assertRaises(SystemExit):
            pipeline.design(self.paths, vendor=vendor, codec=self.codec, max_credits=10, log=quiet)
        self.assertEqual(vendor.designs, [], "a round the budget can't cover is never sent")
        n = pipeline.design(self.paths, vendor=vendor, codec=self.codec, max_credits=1000, log=quiet)
        self.assertEqual(n, 1)
        record = json.loads((self.paths.design / "round1.json").read_text(encoding="utf-8"))
        spec = json.loads(self.paths.speaker.read_text(encoding="utf-8"))["voice"]["design"]
        self.assertEqual(record["request"]["voice_description"], spec["description"])
        self.assertEqual(record["request"]["text"], spec["preview_text"])
        self.assertEqual(len(record["previews"]), 3)
        for i in (1, 2, 3):
            self.assertTrue((self.paths.design / f"round1_preview{i}.heard.wav").exists(), "previews are encoded too")
        self.assertNotIn("reference_audio", json.dumps(vendor.designs), "designed from text; nothing is cloned")
        pipeline.design(self.paths, vendor=vendor, codec=self.codec, max_credits=1000, log=quiet)
        self.assertNotEqual(vendor.designs[0]["seed"], vendor.designs[1]["seed"], "each round is a new draw")

    def test_saving_a_preview_makes_it_the_voice_and_every_line_is_voiced(self):
        vendor = FakeVendor()
        pipeline.design(self.paths, vendor=vendor, codec=self.codec, max_credits=1000, log=quiet)
        pipeline.save_voice(self.paths, 2, vendor=vendor, log=quiet)
        self.assertEqual(vendor.saved[0]["generated_voice_id"], "gen1")
        v = json.loads(self.paths.speaker.read_text(encoding="utf-8"))["voice"]
        self.assertEqual(v["voice_id"], "VoiceAAAAAAAAAAAA")
        self.assertIn("round 1, preview 2", v["design"]["picked"])
        self.assertEqual({a for _, _, a in self.actions()}, {"generate"})


class SheetAndPackTest(Fixture):
    def setUp(self):
        super().setUp()
        self.paths.rom_set.mkdir(parents=True)  # the fake codec "extracts" FIGHT!
        self.designed()
        self.run_pipeline()

    def test_the_sheet_plays_fight_then_each_encoded_take(self):
        page = self.paths.sheet.read_text(encoding="utf-8")
        self.assertIn("<audio id='fight' preload='auto' src='rom/fight.heard.wav'>", page)
        self.assertIn("fight.onended", page)
        heard = re.findall(r"play\('([^']+)'\)", page)
        self.assertEqual(len(heard), 3 * self.n_lines)
        for h in heard:
            self.assertTrue(h.endswith(".heard.wav"), f"{h}: only encoded takes are judged")
            self.assertTrue((self.paths.archive / h).exists())

    def test_picks_are_recorded_and_checked(self):
        pipeline.pick(self.paths, "super", 2, log=quiet)
        picks = json.loads(self.paths.picks.read_text(encoding="utf-8"))
        key = next(i.key for i in pipeline.plan(self.paths).items if i.line.stem == "super" and i.take == 2)
        self.assertEqual(picks["lines"]["super"], {"take": 2, "take_key": key})
        with self.assertRaises(ValueError):
            pipeline.check_picks(self.paths, {"lines": {"fireball": {"take": 1, "take_key": key}}})
        with self.assertRaises(ValueError):
            pipeline.check_picks(self.paths, {"lines": {"perfect": {"take": 1, "take_key": "0" * 64}}})

    def test_build_pack_writes_the_picked_lines_and_reports_the_rest(self):
        with self.assertRaises(SystemExit):
            pipeline.build_pack(self.paths, codec=self.codec, log=quiet)
        pipeline.pick(self.paths, "super", 1, log=quiet)
        pipeline.pick(self.paths, "combo_5", 3, log=quiet)
        pipeline.pick(self.paths, "burnout", 2, log=quiet)
        self.edit_line("burnout", "[shouting] BURNOUT!")  # its pick is no longer one of its takes
        summary = pipeline.build_pack(self.paths, codec=self.codec, log=quiet)
        self.assertEqual(summary["written"], ["combo_5", "super"])
        self.assertEqual(summary["stale"], ["burnout"])
        self.assertIn("perfect", summary["unpicked"])
        plan = self.codec.packs[-1]
        self.assertTrue(plan["super"]["vox"].endswith(".vox"))
        self.assertEqual(pathlib.Path(plan["super"]["vox"]).parent.name, "super")


class KeyTest(Fixture):
    """The real el.py against a mocked network, with a made-up key."""

    def fake_urlopen(self):
        state = {"count": 0, "seen": []}

        class Response(io.BytesIO):
            def __enter__(self):
                return self

            def __exit__(self, *a):
                return False

        def urlopen(req, timeout=None):
            state["seen"].append(req.full_url)
            assert req.get_header("Xi-api-key") == SECRET, "the key goes in the xi-api-key header"
            if req.full_url.endswith("/v1/user/subscription"):
                return Response(json.dumps({"tier": "starter", "character_count": state["count"],
                                            "character_limit": 40000}).encode())
            if "/v1/text-to-speech/" in req.full_url:
                text = json.loads(req.data)["text"]
                state["count"] += len(text)
                return Response(tone_pcm(text))
            raise AssertionError(req.full_url)

        return urlopen, state

    def test_the_key_never_reaches_output_files_or_errors(self):
        self.designed()
        urlopen, state = self.fake_urlopen()
        out, err = io.StringIO(), io.StringIO()
        with mock.patch.object(el, "_key", return_value=SECRET), mock.patch.object(el.urllib.request, "urlopen", urlopen), \
                mock.patch.object(el.time, "sleep", lambda s: None), contextlib.redirect_stdout(out), \
                contextlib.redirect_stderr(err):
            vendor = pipeline.ElevenLabs(self.paths)
            pipeline.run(self.paths, vendor=vendor, codec=self.codec, max_credits=10000)
            self.assertTrue(any("text-to-speech" in u for u in state["seen"]))

            def refused(req, timeout=None):
                raise urllib.error.HTTPError(req.full_url, 401, "Unauthorized", {}, io.BytesIO(b'{"detail":"invalid key"}'))

            with mock.patch.object(el.urllib.request, "urlopen", refused):
                with self.assertRaises(SystemExit) as e:
                    el.request("GET", "/v1/user/subscription")
        self.assertNotIn(SECRET, str(e.exception))
        self.assertNotIn(SECRET, out.getvalue() + err.getvalue())
        self.assertIn("[ledger]", out.getvalue(), "the paid batch is ledgered")
        for f in pathlib.Path(self.tmp.name).rglob("*"):
            if f.is_file():
                self.assertNotIn(SECRET.encode(), f.read_bytes(), f"the key is in {f}")
        ledger = json.loads(self.paths.ledger.read_text(encoding="utf-8"))
        self.assertEqual(ledger[0]["credits"], state["count"])

    def test_only_el_reads_the_key(self):
        for f in (REPO / "tools" / "voice").glob("*.py"):
            text = f.read_text(encoding="utf-8")
            if f.name != "el.py":
                self.assertNotIn("ELEVENLABS_API_KEY\"", text.replace("'", '"'), f.name)
                self.assertNotIn("winreg", text, f.name)
                self.assertIsNone(re.search(r"(?<!\w)_key\(", text), f.name)


class ContentTest(unittest.TestCase):
    """The real line list and speaker."""

    def test_the_lines_are_his_lines_spelled_out(self):
        doc = json.loads((REPO / "tools" / "voice" / "lines.json").read_text(encoding="utf-8"))
        lines, takes = pipeline.load_lines(pipeline.Paths(REPO))
        self.assertEqual(takes, 3)
        self.assertEqual(doc["combo_cap"], 15)
        self.assertEqual(len(lines), 22)
        # The stems are the Rust ones (sf_video::announcer::AnnouncerLine::file_stem).
        src = (REPO / "crates" / "sf-video" / "src" / "announcer.rs").read_text(encoding="utf-8")
        rust = re.findall(r'AnnouncerLine::\w+ => "(\w+)"', src)
        self.assertEqual(rust, pipeline.WORDS)
        self.assertIn(f"COMBO_MIN_HITS: u16 = {pipeline.COMBO_MIN};", src)
        words = ["Five", "Six", "Seven", "Eight", "Nine", "Ten", "Eleven", "Twelve", "Thirteen", "Fourteen", "Fifteen"]
        combos = {x.stem: x.text for x in lines if x.stem.startswith("combo_")}
        self.assertEqual(combos, {f"combo_{n}": f"{w} hit combo!" for n, w in zip(range(5, 16), words)})
        for x in lines:
            self.assertEqual(x.prompt, x.text.upper())

    def test_the_voice_is_designed_from_the_design_docs_description(self):
        v = json.loads((REPO / "tools" / "voice" / "announcer.json").read_text(encoding="utf-8"))["voice"]
        design = (REPO / "docs" / "design" / "2026-11-04-era-hud-and-callouts-design.md").read_text(encoding="utf-8")
        self.assertIn(v["design"]["description"], design.replace("\n  > ", " "))
        self.assertEqual(v["vendor"], "elevenlabs")
        self.assertGreaterEqual(len(v["design"]["preview_text"]), 100, "Voice Design needs 100 characters or more")
        # Either not designed yet, or a saved voice that names the preview it was kept from.
        if v["voice_id"] is not None:
            self.assertTrue(v["voice_id"], "a saved voice has an id")
            self.assertIn("picked", v["design"], "a saved voice records the preview it was kept from")

    def test_authoring_scripts_do_not_duplicate_pack_audio(self):
        for pattern in ("*.wav", "*.vox", "*.mp3", "*.ogg"):
            found = [p for p in (REPO / "tools" / "voice").rglob(pattern)]
            self.assertEqual(found, [], pattern)


@unittest.skipUnless(shutil.which("ffmpeg") and audio.binary_path(REPO).exists(),
                     "needs ffmpeg and `cargo build --release -p sf-audio`")
class EndToEndTest(Fixture):
    """Real ffmpeg and the real Rust codec: takes to a pack VoiceStore accepts."""

    def setUp(self):
        super().setUp()
        mock.patch.stopall()  # the real prepare
        self.paths.binary = audio.binary_path(REPO)
        self.codec = pipeline._codec(self.paths)

    def test_takes_become_a_voice_pack(self):
        self.designed()
        self.run_pipeline()
        item = next(i for i in pipeline.plan(self.paths).items if i.line.stem == "super" and i.take == 1)
        d = pipeline.derived(self.paths.take_file("super", item.key))
        rate, s = audio.read_wav(d["wav4k"])
        self.assertEqual(rate, 4000)
        self.assertAlmostEqual(max(abs(x) for x in s) / 32767, 10 ** (-2 / 20), places=2)
        self.assertLess(len(s) / 4000, 0.4 + 6 * 0.05, "the silence is trimmed")
        proc = json.loads(d["proc"].read_text(encoding="utf-8"))
        self.assertEqual(proc["nibbles"], len(s))
        self.assertEqual(d["vox"].stat().st_size, (len(s) + 1) // 2)
        with wave.open(str(d["heard"])) as w:
            self.assertEqual((w.getframerate(), w.getnframes()), (48000, 12 * len(s)))
        for stem in ("super", "combo_6"):
            pipeline.pick(self.paths, stem, 1, log=quiet)
        pipeline.build_pack(self.paths, codec=self.codec, log=quiet)
        manifest = json.loads((self.paths.pack / "manifest.json").read_text(encoding="utf-8"))
        self.assertEqual((manifest["format"], manifest["sample_rate"]), (1, 4000))
        self.assertEqual(sorted(manifest["lines"]), ["combo_6", "super"])
        vox = (self.paths.pack / "super.vox").read_bytes()
        self.assertEqual(vox, d["vox"].read_bytes(), "the pack holds the take the maintainer heard")
        self.assertEqual(manifest["lines"]["super"]["crc32"], f"{zlib.crc32(vox):08x}")
        self.assertTrue((self.paths.archive / "pack-plan.json").exists())


if __name__ == "__main__":
    unittest.main()
