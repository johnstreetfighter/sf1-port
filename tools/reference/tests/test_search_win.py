import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import scenario
import search_win


class SearchWinTest(unittest.TestCase):
    def test_shoryuken_strategy_generates_valid_non_overlapping_inputs(self):
        spec = search_win.StrategySpec(
            move="shoryuken_hp",
            start_frame=5860,
            end_frame=6100,
            period=60,
            walk_frames=20,
            move_offset=22,
        )
        events = search_win.generate_strategy_inputs(spec)
        self.assertGreater(len(events), 0)
        # First cycle at 5860 should match sf-win-01's cycle 57 shape:
        # walk at 5860 (20f), F at 5882 (3f), D at 5885 (11f), DF at 5888 (8f), HP at 5889 (4f).
        first_five = [(e["frame"], e["press"], e["hold_frames"]) for e in events[:5]]
        self.assertEqual(
            [
                (5860, "p1_right", 20),
                (5882, "p1_right", 3),
                (5885, "p1_down", 11),
                (5888, "p1_right", 8),
                (5889, "p1_hp", 4),
            ],
            first_five,
        )
        # Must validate through scenario.parse_scenario with no overlapping presses.
        base = json.loads(
            (Path(__file__).resolve().parent.parent / "scenarios" / "sf_win_01.json").read_text(
                encoding="utf-8"
            )
        )
        extended = search_win.extend_scenario_dict(
            base,
            scenario_id="sf-win-02-test",
            description="test",
            divergence_frame=5860,
            segment_inputs=events,
            extra_inputs=[],
            duration_frames=6200,
            checkpoints=[5860, 6100],
        )
        parsed = scenario.parse_scenario(extended)
        self.assertEqual("sf-win-02-test", parsed.scenario_id)

    def test_other_moves_in_strategy_family_validate_cleanly(self):
        base = json.loads(
            (Path(__file__).resolve().parent.parent / "scenarios" / "sf_win_01.json").read_text(
                encoding="utf-8"
            )
        )
        for move in (
            "shoryuken_hp",
            "shoryuken_mp",
            "shoryuken_lp",
            "hadoken_hp",
            "tatsumaki_hk",
            "walk_hp",
            "walk_hk",
            "crouch_hk",
            "jump_hk",
            "block_high",
            "block_low",
            "idle",
        ):
            spec = search_win.StrategySpec(
                move=move,
                start_frame=5860,
                end_frame=6100,
                period=60,
                walk_frames=18,
                move_offset=22,
            )
            events = search_win.generate_strategy_inputs(spec)
            extended = search_win.extend_scenario_dict(
                base,
                scenario_id=f"test-{move}",
                description="test",
                divergence_frame=5860,
                segment_inputs=events,
                extra_inputs=[],
                duration_frames=6200,
                checkpoints=[],
            )
            parsed = scenario.parse_scenario(extended)
            self.assertEqual(f"test-{move}", parsed.scenario_id)

    def test_extend_scenario_preserves_prefix_before_divergence_frame(self):
        base = json.loads(
            (Path(__file__).resolve().parent.parent / "scenarios" / "sf_win_01.json").read_text(
                encoding="utf-8"
            )
        )
        prefix = [e for e in base["inputs"] if e["frame"] < 5860]
        spec = search_win.StrategySpec(
            move="hadoken_hp",
            start_frame=5870,
            end_frame=6000,
            period=60,
            walk_frames=15,
            move_offset=20,
        )
        seg = search_win.generate_strategy_inputs(spec)
        extended = search_win.extend_scenario_dict(
            base,
            scenario_id="sf-win-02-prefix-check",
            description="test",
            divergence_frame=5860,
            segment_inputs=seg,
            extra_inputs=[{"frame": 9000, "press": "p1_hp", "hold_frames": 6, "comment": "bonus"}],
            duration_frames=9500,
            checkpoints=[5860],
        )
        ext_prefix = [e for e in extended["inputs"] if e["frame"] < 5860]
        self.assertEqual(prefix, ext_prefix)

    def test_probe_lua_never_writes_emulated_memory(self):
        base = json.loads(
            (Path(__file__).resolve().parent.parent / "scenarios" / "sf_win_01.json").read_text(
                encoding="utf-8"
            )
        )
        s = scenario.parse_scenario(base)
        lua_code = search_win.generate_probe_lua(
            s,
            out_dir="/tmp/probe",
            start_frame=5860,
            target_stage=1,
            target_fight=0,
            stop_at_next_fight_loop=True,
        )
        for forbidden in ("write_u8", "write_u16", "write_u32", "write_i8", "write_i16", "write_i32", "writev_"):
            self.assertNotIn(forbidden, lua_code)
        self.assertIn("read_u8", lua_code)
        self.assertIn("set_value", lua_code)

    def test_parse_probe_outcome_extracts_milestones_and_verdict(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "probe.json"
            sample = {
                "scenario_id": "sf-win-02",
                "start_frame": 5860,
                "final_frame": 10350,
                "stop_reason": "target_fight_loop_reached",
                "stage": 1,
                "fight": 0,
                "winner": 0,
                "p1_wins": 0,
                "cpu_wins": 0,
                "p1_health": 48,
                "cpu_health": 48,
                "rounds": [
                    {
                        "round": 0,
                        "frame": 6500,
                        "p1_wins": 1,
                        "cpu_wins": 0,
                        "p1_health": 40,
                        "cpu_health": 0,
                    },
                    {
                        "round": 1,
                        "frame": 8100,
                        "p1_wins": 2,
                        "cpu_wins": 0,
                        "p1_health": 36,
                        "cpu_health": 0,
                    },
                ],
                "milestones": {
                    "geki_events_0xb_6_first_frame": 8600,
                    "geki_events_0xb_6_last_frame": 8968,
                    "bonus_start_frame": 8970,
                    "stage_step_frame": 9600,
                    "next_fight_round1_frame": 10349,
                    "next_fight_loop_frame": 10350,
                },
            }
            p.write_text(json.dumps(sample), encoding="utf-8")
            outcome = search_win.load_probe_outcome(p)
            self.assertTrue(outcome.match_won)
            self.assertEqual(8970, outcome.bonus_start_frame)
            self.assertEqual(9600, outcome.stage_step_frame)
            self.assertEqual(10349, outcome.next_fight_round1_frame)
            self.assertEqual(10350, outcome.next_fight_loop_frame)

    def test_iter_strategy_candidates_yields_deterministic_family_with_default_first(self):
        cands = list(
            search_win.iter_strategy_candidates(
                start_frame=5860,
                end_frame=8420,
                moves=("shoryuken_hp", "hadoken_hp"),
                periods=(60, 50),
                walk_frames_list=(20, 15),
                phase_offsets=(0, 10),
            )
        )
        self.assertEqual(2 * 2 * 2 * 2, len(cands))
        self.assertEqual(
            search_win.StrategySpec(
                move="shoryuken_hp",
                start_frame=5860,
                end_frame=8420,
                period=60,
                walk_frames=20,
                move_offset=22,
            ),
            cands[0],
        )

    def test_sf_win_02_scenario_definition_and_captures(self):
        scenarios_dir = Path(__file__).resolve().parent.parent / "scenarios"
        win01_raw = json.loads((scenarios_dir / "sf_win_01.json").read_text(encoding="utf-8"))
        win02_raw = json.loads((scenarios_dir / "sf_win_02.json").read_text(encoding="utf-8"))
        win01 = scenario.parse_scenario(win01_raw)
        win02 = scenario.parse_scenario(win02_raw)

        self.assertEqual("sf-win-02", win02.scenario_id)
        self.assertEqual(10842, win02.duration_frames)
        self.assertEqual(list(range(5860, 10842)), win02.checkpoints)

        # Every input event before Geki's round-1 start (5860, the divergence frame
        # where the Geki strategy segment begins) — and in fact through 6219 (where
        # cycle 63 at 6220 starts after sf-win-01's last input at 6189) — must be
        # identical between sf-win-01 and sf-win-02.
        ev01 = [e for e in scenario.compute_input_events(win01) if e["frame"] < 6220]
        ev02 = [e for e in scenario.compute_input_events(win02) if e["frame"] < 6220]
        self.assertEqual(ev01, ev02)

        captures_dir = Path(__file__).resolve().parent.parent / "local" / "captures"
        cap_a = captures_dir / "win-02-run-a"
        if not cap_a.is_dir():
            return

        win01_cap = captures_dir / "win-01-run-a"
        if win01_cap.is_dir():
            f01 = (win01_cap / "frames.tsv").read_text(encoding="utf-8").splitlines()
            f02 = (cap_a / "frames.tsv").read_text(encoding="utf-8").splitlines()
            # Header + frames 0..=6219 (including divergence frame 5860).
            self.assertEqual(f01[:6221], f02[:6221])
            for fn in (5860, 6000, 6219):
                for suffix in (
                    f"ram_{fn:06d}_work.bin",
                    f"ram_{fn:06d}_video.bin",
                    f"ram_{fn:06d}_palette.bin",
                    f"ram_{fn:06d}_music.bin",
                    f"pixels_{fn:06d}.rgba",
                ):
                    self.assertEqual(
                        (win01_cap / suffix).read_bytes(),
                        (cap_a / suffix).read_bytes(),
                        f"shared checkpoint mismatch at {suffix}",
                    )

        def read_wram(fn: int) -> bytes:
            return (cap_a / f"ram_{fn:06d}_work.bin").read_bytes()

        def slot1_resume(w: bytes) -> int:
            if w[0x16] != 8:
                return 0
            usp = int.from_bytes(w[0x1E:0x22], "big")
            off = usp - 0xFF8000
            if int.from_bytes(w[off + 60 : off + 64], "big") != 0x3B5A8:
                return 0
            return int.from_bytes(w[off + 64 : off + 68], "big")

        # 5860: Geki round-1 start at 0x3ea3e.
        w5860 = read_wram(5860)
        self.assertEqual(0x3EA3E, slot1_resume(w5860))
        self.assertEqual(0, w5860[0x4003])
        self.assertEqual(1, w5860[0x4004])

        # 8908..=9277: 0xffc009 = 1 and 0xffc004 = 1 during Geki's events 0xb and 6.
        for fn in range(8908, 9278):
            w = read_wram(fn)
            self.assertEqual(1, w[0x4009], f"frame {fn}: expected 0xffc009 = 1")
            self.assertEqual(1, w[0x4004], f"frame {fn}: expected 0xffc004 = 1")

        # 9278: flow task inside 0x40d30 (resume 0x40dd4) with 0xffc004 = 0.
        w9278 = read_wram(9278)
        self.assertEqual(0x40DD4, slot1_resume(w9278))
        self.assertEqual(0, w9278[0x4004])
        self.assertEqual(0x40, w9278[0x4008])

        # 10029: 0xffc003 = 1 after 0x3efc4.
        self.assertEqual(0, read_wram(10028)[0x4003])
        self.assertEqual(1, read_wram(10029)[0x4003])

        # 10840: flow task at 0x3ea3e for 0x1734c; 10841: first fight-loop frame at 0x3eafa.
        w10840 = read_wram(10840)
        self.assertEqual(0x3EA3E, slot1_resume(w10840))
        self.assertEqual(1, w10840[0x4003])
        self.assertEqual(0, w10840[0x4004])
        w10841 = read_wram(10841)
        self.assertEqual(0x3EAFA, slot1_resume(w10841))
        self.assertEqual(0x80, w10841[0x4008])

    def test_sf_win_03_scenario_definition_and_captures(self):
        scenarios_dir = Path(__file__).resolve().parent.parent / "scenarios"
        win02_raw = json.loads((scenarios_dir / "sf_win_02.json").read_text(encoding="utf-8"))
        win03_raw = json.loads((scenarios_dir / "sf_win_03.json").read_text(encoding="utf-8"))
        win02 = scenario.parse_scenario(win02_raw)
        win03 = scenario.parse_scenario(win03_raw)

        self.assertEqual("sf-win-03", win03.scenario_id)
        self.assertEqual(15545, win03.duration_frames)
        self.assertEqual(list(range(10840, 15545)), win03.checkpoints)

        # Joe's strategy begins at his round-1 start. Everything before that
        # divergence frame must remain identical to sf-win-02.
        ev02 = [e for e in scenario.compute_input_events(win02) if e["frame"] < 10840]
        ev03 = [e for e in scenario.compute_input_events(win03) if e["frame"] < 10840]
        self.assertEqual(ev02, ev03)

        captures_dir = Path(__file__).resolve().parent.parent / "local" / "captures"
        cap_a = captures_dir / "win-03-run-a"
        win02_cap = captures_dir / "win-02-run-a"
        if not cap_a.is_dir() or not win02_cap.is_dir():
            return

        f02 = (win02_cap / "frames.tsv").read_text(encoding="utf-8").splitlines()
        f03 = (cap_a / "frames.tsv").read_text(encoding="utf-8").splitlines()
        # Header + frames 0..=10840, including the divergence-frame checkpoint.
        self.assertEqual(f02[:10842], f03[:10842])
        for suffix in (
            "ram_010840_work.bin",
            "ram_010840_video.bin",
            "ram_010840_palette.bin",
            "ram_010840_music.bin",
            "pixels_010840.rgba",
        ):
            self.assertEqual(
                (win02_cap / suffix).read_bytes(),
                (cap_a / suffix).read_bytes(),
                f"shared checkpoint mismatch at {suffix}",
            )


    def test_sf_win_04_scenario_definition_and_captures(self):
        scenarios_dir = Path(__file__).resolve().parent.parent / "scenarios"
        win03_raw = json.loads((scenarios_dir / "sf_win_03.json").read_text(encoding="utf-8"))
        win04_raw = json.loads((scenarios_dir / "sf_win_04.json").read_text(encoding="utf-8"))
        win03 = scenario.parse_scenario(win03_raw)
        win04 = scenario.parse_scenario(win04_raw)

        self.assertEqual("sf-win-04", win04.scenario_id)
        self.assertEqual(19381, win04.duration_frames)
        self.assertEqual(list(range(13711, 19381)), win04.checkpoints)

        # Mike's strategy begins after his round-1 fixture. Everything before
        # that divergence frame must remain identical to sf-win-03.
        ev03 = [e for e in scenario.compute_input_events(win03) if e["frame"] < 13712]
        ev04 = [e for e in scenario.compute_input_events(win04) if e["frame"] < 13712]
        self.assertEqual(ev03, ev04)

        captures_dir = Path(__file__).resolve().parent.parent / "local" / "captures"
        cap_a = captures_dir / "win-04-run-a"
        win03_cap = captures_dir / "win-03-run-a"
        if not cap_a.is_dir() or not win03_cap.is_dir():
            return

        f03 = (win03_cap / "frames.tsv").read_text(encoding="utf-8").splitlines()
        f04 = (cap_a / "frames.tsv").read_text(encoding="utf-8").splitlines()
        # Header + frames 0..=13712, including the divergence-frame checkpoint.
        self.assertEqual(f03[:13714], f04[:13714])
        for fn in (13711, 13712):
            for suffix in (
                f"ram_{fn:06d}_work.bin",
                f"ram_{fn:06d}_video.bin",
                f"ram_{fn:06d}_palette.bin",
                f"ram_{fn:06d}_music.bin",
                f"pixels_{fn:06d}.rgba",
            ):
                self.assertEqual(
                    (win03_cap / suffix).read_bytes(),
                    (cap_a / suffix).read_bytes(),
                    f"shared checkpoint mismatch at {suffix}",
                )

    def test_sf_win_05_scenario_definition_and_captures(self):
        scenarios_dir = Path(__file__).resolve().parent.parent / "scenarios"
        win04_raw = json.loads((scenarios_dir / "sf_win_04.json").read_text(encoding="utf-8"))
        win05_raw = json.loads((scenarios_dir / "sf_win_05.json").read_text(encoding="utf-8"))
        win04 = scenario.parse_scenario(win04_raw)
        win05 = scenario.parse_scenario(win05_raw)

        self.assertEqual("sf-win-05", win05.scenario_id)
        self.assertEqual(22267, win05.duration_frames)
        self.assertEqual(list(range(19379, 22267)), win05.checkpoints)

        # Lee's strategy begins after his round-1 fixture (19379). Everything
        # before divergence frame 19380 must remain identical to sf-win-04.
        ev04 = [e for e in scenario.compute_input_events(win04) if e["frame"] < 19380]
        ev05 = [e for e in scenario.compute_input_events(win05) if e["frame"] < 19380]
        self.assertEqual(ev04, ev05)

        captures_dir = Path(__file__).resolve().parent.parent / "local" / "captures"
        cap_a = captures_dir / "win-05-run-a"
        win04_cap = captures_dir / "win-04-run-a"
        if not cap_a.is_dir() or not win04_cap.is_dir():
            return

        f04 = (win04_cap / "frames.tsv").read_text(encoding="utf-8").splitlines()
        f05 = (cap_a / "frames.tsv").read_text(encoding="utf-8").splitlines()
        # Header + frames 0..=19380, including the divergence-frame checkpoint.
        self.assertEqual(f04[:19382], f05[:19382])
        for fn in (19379, 19380):
            for suffix in (
                f"ram_{fn:06d}_work.bin",
                f"ram_{fn:06d}_video.bin",
                f"ram_{fn:06d}_palette.bin",
                f"ram_{fn:06d}_music.bin",
                f"pixels_{fn:06d}.rgba",
            ):
                self.assertEqual(
                    (win04_cap / suffix).read_bytes(),
                    (cap_a / suffix).read_bytes(),
                    f"shared checkpoint mismatch at {suffix}",
                )

    def test_sf_win_06_scenario_definition_and_captures(self):
        scenarios_dir = Path(__file__).resolve().parent.parent / "scenarios"
        win05_raw = json.loads((scenarios_dir / "sf_win_05.json").read_text(encoding="utf-8"))
        win06_raw = json.loads((scenarios_dir / "sf_win_06.json").read_text(encoding="utf-8"))
        win05 = scenario.parse_scenario(win05_raw)
        win06 = scenario.parse_scenario(win06_raw)

        self.assertEqual("sf-win-06", win06.scenario_id)
        self.assertEqual(26373, win06.duration_frames)
        self.assertEqual(list(range(22265, 26373)), win06.checkpoints)

        # Gen's strategy begins after his round-1 fixture (22265). Everything
        # before divergence frame 22266 must remain identical to sf-win-05.
        ev05 = [e for e in scenario.compute_input_events(win05) if e["frame"] < 22266]
        ev06 = [e for e in scenario.compute_input_events(win06) if e["frame"] < 22266]
        self.assertEqual(ev05, ev06)

        captures_dir = Path(__file__).resolve().parent.parent / "local" / "captures"
        cap_a = captures_dir / "win-06-run-a"
        win05_cap = captures_dir / "win-05-run-a"
        if not cap_a.is_dir() or not win05_cap.is_dir():
            return

        f05 = (win05_cap / "frames.tsv").read_text(encoding="utf-8").splitlines()
        f06 = (cap_a / "frames.tsv").read_text(encoding="utf-8").splitlines()
        # Header + frames 0..=22266, including the divergence-frame checkpoint.
        self.assertEqual(f05[:22268], f06[:22268])
        for fn in (22265, 22266):
            for suffix in (
                f"ram_{fn:06d}_work.bin",
                f"ram_{fn:06d}_video.bin",
                f"ram_{fn:06d}_palette.bin",
                f"ram_{fn:06d}_music.bin",
                f"pixels_{fn:06d}.rgba",
            ):
                self.assertEqual(
                    (win05_cap / suffix).read_bytes(),
                    (cap_a / suffix).read_bytes(),
                    f"shared checkpoint mismatch at {suffix}",
                )

    def test_sf_win_07_scenario_definition_and_captures(self):
        scenarios_dir = Path(__file__).resolve().parent.parent / "scenarios"
        win06_raw = json.loads((scenarios_dir / "sf_win_06.json").read_text(encoding="utf-8"))
        win07_raw = json.loads((scenarios_dir / "sf_win_07.json").read_text(encoding="utf-8"))
        win06 = scenario.parse_scenario(win06_raw)
        win07 = scenario.parse_scenario(win07_raw)

        self.assertEqual("sf-win-07", win07.scenario_id)
        self.assertEqual(29428, win07.duration_frames)
        self.assertEqual(list(range(26371, 29428)), win07.checkpoints)

        # Birdie's strategy begins after his round-1 fixture (26371). Everything
        # before divergence frame 26372 must remain identical to sf-win-06.
        ev06 = [e for e in scenario.compute_input_events(win06) if e["frame"] < 26372]
        ev07 = [e for e in scenario.compute_input_events(win07) if e["frame"] < 26372]
        self.assertEqual(ev06, ev07)

        captures_dir = Path(__file__).resolve().parent.parent / "local" / "captures"
        cap_a = captures_dir / "win-07-run-a"
        win06_cap = captures_dir / "win-06-run-a"
        if not cap_a.is_dir() or not win06_cap.is_dir():
            return

        f06 = (win06_cap / "frames.tsv").read_text(encoding="utf-8").splitlines()
        f07 = (cap_a / "frames.tsv").read_text(encoding="utf-8").splitlines()
        # Header + frames 0..=26372, including the divergence-frame checkpoint.
        self.assertEqual(f06[:26374], f07[:26374])
        for fn in (26371, 26372):
            for suffix in (
                f"ram_{fn:06d}_work.bin",
                f"ram_{fn:06d}_video.bin",
                f"ram_{fn:06d}_palette.bin",
                f"ram_{fn:06d}_music.bin",
                f"pixels_{fn:06d}.rgba",
            ):
                self.assertEqual(
                    (win06_cap / suffix).read_bytes(),
                    (cap_a / suffix).read_bytes(),
                    f"shared checkpoint mismatch at {suffix}",
                )

    def test_sf_win_08_scenario_definition_and_captures(self):
        scenarios_dir = Path(__file__).resolve().parent.parent / "scenarios"
        win07_raw = json.loads((scenarios_dir / "sf_win_07.json").read_text(encoding="utf-8"))
        win08_raw = json.loads((scenarios_dir / "sf_win_08.json").read_text(encoding="utf-8"))
        win07 = scenario.parse_scenario(win07_raw)
        win08 = scenario.parse_scenario(win08_raw)

        self.assertEqual("sf-win-08", win08.scenario_id)
        self.assertEqual(34185, win08.duration_frames)
        self.assertEqual(list(range(29426, 34185)), win08.checkpoints)

        # Eagle's strategy begins after his round-1 fixture (29426). Everything
        # before divergence frame 29427 must remain identical to sf-win-07.
        ev07 = [e for e in scenario.compute_input_events(win07) if e["frame"] < 29427]
        ev08 = [e for e in scenario.compute_input_events(win08) if e["frame"] < 29427]
        self.assertEqual(ev07, ev08)

        captures_dir = Path(__file__).resolve().parent.parent / "local" / "captures"
        cap_a = captures_dir / "win-08-run-a"
        win07_cap = captures_dir / "win-07-run-a"
        if not cap_a.is_dir() or not win07_cap.is_dir():
            return

        f07 = (win07_cap / "frames.tsv").read_text(encoding="utf-8").splitlines()
        f08 = (cap_a / "frames.tsv").read_text(encoding="utf-8").splitlines()
        # Header + frames 0..=29427, including the divergence-frame checkpoint.
        self.assertEqual(f07[:29429], f08[:29429])
        for fn in (29426, 29427):
            for suffix in (
                f"ram_{fn:06d}_work.bin",
                f"ram_{fn:06d}_video.bin",
                f"ram_{fn:06d}_palette.bin",
                f"ram_{fn:06d}_music.bin",
                f"pixels_{fn:06d}.rgba",
            ):
                self.assertEqual(
                    (win07_cap / suffix).read_bytes(),
                    (cap_a / suffix).read_bytes(),
                    f"shared checkpoint mismatch at {suffix}",
                )

    def test_sf_win_09_scenario_definition_and_captures(self):
        scenarios_dir = Path(__file__).resolve().parent.parent / "scenarios"
        win08_raw = json.loads((scenarios_dir / "sf_win_08.json").read_text(encoding="utf-8"))
        win09_raw = json.loads((scenarios_dir / "sf_win_09.json").read_text(encoding="utf-8"))
        win08 = scenario.parse_scenario(win08_raw)
        win09 = scenario.parse_scenario(win09_raw)

        self.assertEqual("sf-win-09", win09.scenario_id)
        self.assertEqual(37155, win09.duration_frames)
        self.assertEqual(list(range(34183, 37155)), win09.checkpoints)

        # Adon's strategy begins after his round-1 fixture (34183). Everything
        # before divergence frame 34185 must remain identical to sf-win-08.
        ev08 = [e for e in scenario.compute_input_events(win08) if e["frame"] < 34185]
        ev09 = [e for e in scenario.compute_input_events(win09) if e["frame"] < 34185]
        self.assertEqual(ev08, ev09)

        captures_dir = Path(__file__).resolve().parent.parent / "local" / "captures"
        cap_a = captures_dir / "win-09-run-a"
        win08_cap = captures_dir / "win-08-run-a"
        if not cap_a.is_dir() or not win08_cap.is_dir():
            return

        f08 = (win08_cap / "frames.tsv").read_text(encoding="utf-8").splitlines()
        f09 = (cap_a / "frames.tsv").read_text(encoding="utf-8").splitlines()
        # Header + frames 0..=34184, including the divergence-frame checkpoint.
        self.assertEqual(f08[:34186], f09[:34186])
        for fn in (34183, 34184):
            for suffix in (
                f"ram_{fn:06d}_work.bin",
                f"ram_{fn:06d}_video.bin",
                f"ram_{fn:06d}_palette.bin",
                f"ram_{fn:06d}_music.bin",
                f"pixels_{fn:06d}.rgba",
            ):
                self.assertEqual(
                    (win08_cap / suffix).read_bytes(),
                    (cap_a / suffix).read_bytes(),
                    f"shared checkpoint mismatch at {suffix}",
                )

    def test_sf_win_10_scenario_definition_and_captures(self):
        scenarios_dir = Path(__file__).resolve().parent.parent / "scenarios"
        win09_raw = json.loads((scenarios_dir / "sf_win_09.json").read_text(encoding="utf-8"))
        win10_raw = json.loads((scenarios_dir / "sf_win_10.json").read_text(encoding="utf-8"))
        win09 = scenario.parse_scenario(win09_raw)
        win10 = scenario.parse_scenario(win10_raw)

        self.assertEqual("sf-win-10", win10.scenario_id)
        self.assertEqual(43619, win10.duration_frames)
        self.assertEqual(list(range(37153, 43619)), win10.checkpoints)

        # Sagat's strategy begins after his round-1 fixture (37153). Everything
        # before divergence frame 37155 must remain identical to sf-win-09.
        ev09 = [e for e in scenario.compute_input_events(win09) if e["frame"] < 37155]
        ev10 = [e for e in scenario.compute_input_events(win10) if e["frame"] < 37155]
        self.assertEqual(ev09, ev10)

        captures_dir = Path(__file__).resolve().parent.parent / "local" / "captures"
        cap_a = captures_dir / "win-10-run-a"
        win09_cap = captures_dir / "win-09-run-a"
        if not cap_a.is_dir() or not win09_cap.is_dir():
            return

        f09 = (win09_cap / "frames.tsv").read_text(encoding="utf-8").splitlines()
        f10 = (cap_a / "frames.tsv").read_text(encoding="utf-8").splitlines()
        # Header + frames 0..=37154, including the divergence-frame checkpoint.
        self.assertEqual(f09[:37156], f10[:37156])
        for fn in (37153, 37154):
            for suffix in (
                f"ram_{fn:06d}_work.bin",
                f"ram_{fn:06d}_video.bin",
                f"ram_{fn:06d}_palette.bin",
                f"ram_{fn:06d}_music.bin",
                f"pixels_{fn:06d}.rgba",
            ):
                self.assertEqual(
                    (win09_cap / suffix).read_bytes(),
                    (cap_a / suffix).read_bytes(),
                    f"shared checkpoint mismatch at {suffix}",
                )


if __name__ == "__main__":
    unittest.main()
