import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import lua_gen
import scenario

from test_scenario import minimal_scenario_dict


class GenerateDriverTest(unittest.TestCase):
    def test_driver_embeds_input_events_and_no_placeholder_remains(self):
        s = scenario.parse_scenario(minimal_scenario_dict())
        lua = lua_gen.generate_driver(s, out_dir="C:/tmp/out")
        self.assertNotIn("@@CONFIG@@", lua)
        self.assertIn(
            '{ frame = 300, port = "IN0", mask = 0x0001, state = 1 }', lua
        )
        self.assertIn(
            '{ frame = 312, port = "IN0", mask = 0x0001, state = 0 }', lua
        )
        self.assertTrue(lua.index("local CONFIG =") < lua.index("input_events"))
        self.assertIn('"sf-fixed-fight-01"', lua)
        self.assertIn("1200", lua)

    def test_every_legacy_implicit_dip_scenario_regenerates_byte_identical_driver(self):
        scenarios_dir = Path(__file__).resolve().parent.parent / "scenarios"
        template = lua_gen.TEMPLATE_PATH.read_text(encoding="utf-8")
        implicit_count = 0
        for path in sorted(scenarios_dir.glob("*.json")):
            s = scenario.load_scenario(path)
            if s.dipswitches_explicit or s.dip_changes:
                continue
            implicit_count += 1
            actual = lua_gen.generate_driver(s, out_dir="C:/tmp/out")
            self.assertNotIn("CONFIG.dsw1", actual, msg=path.name)
            self.assertTrue(actual.startswith(template.split("@@CONFIG@@")[0]), msg=path.name)
            self.assertTrue(actual.endswith(template.split("@@CONFIG@@")[1]), msg=path.name)
        self.assertGreaterEqual(implicit_count, 52)

    def test_explicit_dipswitches_embed_dsw_and_frame0_and_end_checks(self):
        data = minimal_scenario_dict()
        data["machine"]["dipswitches"] = {
            "Coin A": "2C_1C",
            "Coin B": "1C_6C",
            "Number of Countries Selected": "4",
        }
        s = scenario.parse_scenario(data)
        lua = lua_gen.generate_driver(s, out_dir="C:/tmp/out")
        self.assertIn("dsw1 = 0xdfda,", lua)
        self.assertIn("dsw2 = 0xfbff,", lua)
        self.assertIn("fn == 0", lua)
        self.assertIn("CONFIG.dsw1", lua)
        self.assertIn("CONFIG.dsw2", lua)

    def test_timed_dip_changes_generate_writes_and_next_frame_readbacks(self):
        data = minimal_scenario_dict()
        data["duration_frames"] = 3000
        data["dip_changes"] = [
            {"frame": 2500, "dipswitches": {"Freeze": "On"}},
            {"frame": 2560, "dipswitches": {"Freeze": "Off"}},
        ]
        s = scenario.parse_scenario(data)
        lua = lua_gen.generate_driver(s, out_dir="C:/tmp/out")
        self.assertIn("dip_changes = {", lua)
        self.assertIn("frame = 2500, dsw1 = 0x9fff, dsw2 = 0xffff", lua)
        self.assertIn("frame = 2560, dsw1 = 0xdfff, dsw2 = 0xffff", lua)
        self.assertIn('port = "DSW1", mask = 0x4000, value = 0x0000', lua)
        self.assertIn('port = "DSW1", mask = 0x4000, value = 0x4000', lua)
        self.assertIn("field.user_value = value", lua)
        self.assertIn("change_frame\\treadback_frame\\tdsw1\\tdsw2\\n", lua)
        self.assertIn("readback_frame = fn + 1", lua)
        self.assertTrue(lua.index("log_dip_readbacks(fn)") < lua.index("apply_dip_changes(fn)"))

    def test_timed_dip_readbacks_verify_each_change_and_final_metadata(self):
        data = minimal_scenario_dict()
        data["duration_frames"] = 3000
        data["dip_changes"] = [
            {"frame": 2500, "dipswitches": {"Freeze": "On"}},
            {"frame": 2560, "dipswitches": {"Freeze": "Off"}},
        ]
        s = scenario.parse_scenario(data)
        with tempfile.TemporaryDirectory() as tmp:
            readbacks = Path(tmp) / "dip_readbacks.tsv"
            readbacks.write_text(
                "change_frame\treadback_frame\tdsw1\tdsw2\n"
                "2500\t2501\t9fff\tffff\n"
                "2560\t2561\tdfff\tffff\n",
                encoding="utf-8",
            )
            lua_gen.verify_dip_readbacks(s, readbacks)
            meta = Path(tmp) / "meta.tsv"
            meta.write_text("dsw1\tdfff\ndsw2\tffff\n", encoding="utf-8")
            lua_gen.verify_readback(s, meta)

    def test_generate_cfg_emits_mameconfig_for_requested_dip_fields(self):
        data = minimal_scenario_dict()
        data["machine"]["dipswitches"] = {
            "Coin A": "2C_1C",
            "Coin B": "1C_6C",
            "Number of Countries Selected": "4",
        }
        s = scenario.parse_scenario(data)
        cfg = lua_gen.generate_cfg(s)
        self.assertIn('<mameconfig version="10">', cfg)
        self.assertIn('<port tag=":DSW1" type="DIPSWITCH" mask="7" defvalue="7" value="2" />', cfg)
        self.assertIn('<port tag=":DSW1" type="DIPSWITCH" mask="56" defvalue="56" value="24" />', cfg)
        self.assertIn('<port tag=":DSW2" type="DIPSWITCH" mask="1024" defvalue="1024" value="0" />', cfg)

    def test_unapplied_dip_request_fails_readback_check(self):
        data = minimal_scenario_dict()
        data["machine"]["dipswitches"] = {"Flip Screen": "On"}
        s = scenario.parse_scenario(data)
        with tempfile.TemporaryDirectory() as tmp:
            meta = Path(tmp) / "meta.tsv"
            meta.write_text(
                "dsw1\tdfff\ndsw2\tffff\nscreen_width\t384\nscreen_height\t224\nframes_executed\t21\n",
                encoding="utf-8",
            )
            with self.assertRaises(scenario.ScenarioError):
                lua_gen.verify_readback(s, meta)

            meta.write_text(
                "dsw1\tdeff\ndsw2\tffff\nscreen_width\t384\nscreen_height\t224\nframes_executed\t21\n",
                encoding="utf-8",
            )
            lua_gen.verify_readback(s, meta)

    def test_mame_runner_applies_and_verifies_timed_dip_changes(self):
        here = Path(__file__).resolve().parent.parent
        mame_exe = here / "local" / "mame" / "mame.exe"
        rompath = here / "local" / "roms"
        if not mame_exe.is_file() or not rompath.is_dir():
            self.skipTest("local MAME binary or ROMs not available")
        from types import SimpleNamespace
        import run_scenario

        data = minimal_scenario_dict()
        data["duration_frames"] = 10
        data["inputs"] = []
        data["observations"]["checkpoints"] = []
        data["dip_changes"] = [
            {"frame": 2, "dipswitches": {"Freeze": "On"}},
            {"frame": 4, "dipswitches": {"Freeze": "Off"}},
            {"frame": 6, "dipswitches": {"Freeze": "On"}},
        ]
        with tempfile.TemporaryDirectory() as tmp:
            scenario_path = Path(tmp) / "scenario.json"
            scenario_path.write_text(json.dumps(data), encoding="utf-8")
            out_dir = Path(tmp) / "capture"
            result = run_scenario.run(
                SimpleNamespace(
                    scenario=scenario_path,
                    out=out_dir,
                    mame=mame_exe,
                    rompath=rompath,
                )
            )
            self.assertEqual(0, result)
            self.assertEqual(
                "change_frame\treadback_frame\tdsw1\tdsw2\n"
                "2\t3\t9fff\tffff\n"
                "4\t5\tdfff\tffff\n"
                "6\t7\t9fff\tffff",
                (out_dir / "dip_readbacks.tsv").read_text(encoding="utf-8").strip(),
            )

    def test_mame_driver_aborts_at_frame0_when_dip_request_not_applied(self):
        here = Path(__file__).resolve().parent.parent
        mame_exe = here / "local" / "mame" / "mame.exe"
        rompath = here / "local" / "roms"
        if not mame_exe.is_file() or not rompath.is_dir():
            self.skipTest("local MAME binary or ROMs not available")
        import subprocess

        data = minimal_scenario_dict()
        data["duration_frames"] = 2
        data["inputs"] = []
        data["observations"]["checkpoints"] = []
        data["machine"]["dipswitches"] = {"Flip Screen": "On"}
        s = scenario.parse_scenario(data)
        with tempfile.TemporaryDirectory() as tmp:
            out_dir = Path(tmp)
            driver_path = out_dir / "generated_driver.lua"
            driver_path.write_text(
                lua_gen.generate_driver(s, out_dir.as_posix()), encoding="utf-8"
            )
            # Intentionally do NOT call write_cfg(s, out_dir) so the DIP request is unapplied.
            proc = subprocess.run(
                [
                    str(mame_exe),
                    s.machine_set,
                    "-rompath",
                    str(rompath),
                    "-video",
                    "none",
                    "-sound",
                    "none",
                    "-nothrottle",
                    "-skip_gameinfo",
                    "-noreadconfig",
                    "-nowriteconfig",
                    "-cfg_directory",
                    str(out_dir / "cfg"),
                    "-autoboot_script",
                    str(driver_path),
                    "-autoboot_delay",
                    "0",
                    "-seconds_to_run",
                    "5",
                ],
                capture_output=True,
                text=True,
            )
            with self.assertRaises(scenario.ScenarioError):
                lua_gen.verify_readback(s, out_dir / "meta.tsv")


if __name__ == "__main__":
    unittest.main()
