import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import scenario


def minimal_scenario_dict():
    return {
        "scenario_id": "sf-fixed-fight-01",
        "format_version": 1,
        "reference": {"rom_set": "sf", "mame_version": "0.289"},
        "machine": {"set": "sf"},
        "controls": {
            "coin1": {"port": "IN0", "bit": 0},
            "start1": {"port": "SYSTEM", "bit": 0},
        },
        "duration_frames": 1200,
        "inputs": [
            {"frame": 300, "press": "coin1", "hold_frames": 12},
        ],
        "observations": {
            "screen_downsample_stride": 4,
            "ram_crc_every_frames": 10,
            "checkpoints": [600],
        },
    }


class LoadScenarioTest(unittest.TestCase):
    def write_scenario(self, data):
        tmp = tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False, encoding="utf-8"
        )
        json.dump(data, tmp)
        tmp.close()
        return Path(tmp.name)

    def test_minimal_valid_scenario_loads(self):
        path = self.write_scenario(minimal_scenario_dict())
        s = scenario.load_scenario(path)
        self.assertEqual("sf-fixed-fight-01", s.scenario_id)
        self.assertEqual("sf", s.machine_set)
        self.assertEqual(1200, s.duration_frames)
        self.assertEqual(1, len(s.inputs))
        self.assertEqual("coin1", s.inputs[0].press)
        self.assertEqual([600], s.checkpoints)
    def test_missing_required_field_is_named_in_error(self):
        data = minimal_scenario_dict()
        del data["duration_frames"]
        path = self.write_scenario(data)
        with self.assertRaises(scenario.ScenarioError) as ctx:
            scenario.load_scenario(path)
        self.assertIn("duration_frames", str(ctx.exception))
    def test_unknown_logical_input_is_named_in_error(self):
        data = minimal_scenario_dict()
        data["inputs"].append({"frame": 400, "press": "hadouken", "hold_frames": 12})
        path = self.write_scenario(data)
        with self.assertRaises(scenario.ScenarioError) as ctx:
            scenario.load_scenario(path)
        self.assertIn("hadouken", str(ctx.exception))
    def test_overlapping_press_of_same_input_is_rejected(self):
        data = minimal_scenario_dict()
        data["inputs"].append({"frame": 305, "press": "coin1", "hold_frames": 12})
        path = self.write_scenario(data)
        with self.assertRaises(scenario.ScenarioError) as ctx:
            scenario.load_scenario(path)
        self.assertIn("coin1", str(ctx.exception))

    def test_nonpositive_hold_is_rejected(self):
        data = minimal_scenario_dict()
        data["inputs"][0]["hold_frames"] = 0
        path = self.write_scenario(data)
        with self.assertRaises(scenario.ScenarioError) as ctx:
            scenario.load_scenario(path)
        self.assertIn("hold_frames", str(ctx.exception))

    def test_default_dipswitches_resolve_to_dfff_ffff_and_implicit(self):
        s_omitted = scenario.parse_scenario(minimal_scenario_dict())
        self.assertEqual(0xDFFF, s_omitted.dsw1)
        self.assertEqual(0xFFFF, s_omitted.dsw2)
        self.assertFalse(s_omitted.dipswitches_explicit)

        for label in (
            "defaults",
            "factory defaults; observed readback DSW1=0xdfff, DSW2=0xffff",
        ):
            data = minimal_scenario_dict()
            data["machine"]["dipswitches"] = label
            s = scenario.parse_scenario(data)
            self.assertEqual(0xDFFF, s.dsw1)
            self.assertEqual(0xFFFF, s.dsw2)
            self.assertFalse(s.dipswitches_explicit)

    def test_port_word_dipswitches_accept_hex_strings_and_ints(self):
        data = minimal_scenario_dict()
        data["machine"]["dipswitches"] = {"dsw1": "0xdfda", "dsw2": "0xfbff"}
        s = scenario.parse_scenario(data)
        self.assertEqual(0xDFDA, s.dsw1)
        self.assertEqual(0xFBFF, s.dsw2)
        self.assertTrue(s.dipswitches_explicit)

        data["machine"]["dipswitches"] = {"DSW1": 0x5FFF, "DSW2": 0xFFFF}
        s2 = scenario.parse_scenario(data)
        self.assertEqual(0x5FFF, s2.dsw1)
        self.assertEqual(0xFFFF, s2.dsw2)
        self.assertTrue(s2.dipswitches_explicit)

    def test_named_mame_dip_fields_resolve_masks_and_settings(self):
        data = minimal_scenario_dict()
        data["machine"]["dipswitches"] = {
            "Coin A": "2C_1C",
            "Coin B": "1C_6C",
            "Number of Countries Selected": "4",
        }
        s = scenario.parse_scenario(data)
        self.assertEqual(0xDFDA, s.dsw1)
        self.assertEqual(0xFBFF, s.dsw2)
        self.assertTrue(s.dipswitches_explicit)

        data["machine"]["dipswitches"] = {"Flip Screen": "On"}
        s_flip = scenario.parse_scenario(data)
        self.assertEqual(0xDEFF, s_flip.dsw1)
        self.assertEqual(0xFFFF, s_flip.dsw2)

        data["machine"]["dipswitches"] = {"Service Mode": "On"}
        s_service = scenario.parse_scenario(data)
        self.assertEqual(0x5FFF, s_service.dsw1)
        self.assertEqual(0xFFFF, s_service.dsw2)

    def test_explicit_factory_default_fields_equal_dfff_ffff(self):
        data = minimal_scenario_dict()
        data["machine"]["dipswitches"] = {
            "Coin A": "1 Coin/1 Credit",
            "Coin B": "1 Coin/1 Credit",
            "Flip Screen": "Off",
            "Attract Music": "On",
            "Speed": "Normal",
            "Demo Sounds": "On",
            "Freeze": "Off",
            "Service Mode": "Off",
            "Game Continuation": "5th Stage Maximum",
            "Round Time Count": "100",
            "Difficulty": "Normal",
            "Buy-In Feature": "5th Stage Maximum",
            "Number of Countries Selected": "2",
        }
        s = scenario.parse_scenario(data)
        self.assertEqual(0xDFFF, s.dsw1)
        self.assertEqual(0xFFFF, s.dsw2)
        self.assertTrue(s.dipswitches_explicit)

    def test_timed_dip_changes_are_cumulative_and_preserve_other_fields(self):
        data = minimal_scenario_dict()
        data["machine"]["dipswitches"] = {"Speed": "Slow"}
        data["dip_changes"] = [
            {"frame": 2500, "dipswitches": {"Freeze": "On"}},
            {"frame": 2560, "dipswitches": {"Freeze": "Off"}},
        ]
        s = scenario.parse_scenario(data)
        self.assertEqual(
            [(2500, 0x8FFF, 0xFFFF), (2560, 0xCFFF, 0xFFFF)],
            [(change.frame, change.dsw1, change.dsw2) for change in s.dip_changes],
        )

    def test_timed_dip_changes_require_strictly_increasing_frames(self):
        for frames in ((2500, 2500), (2560, 2500)):
            data = minimal_scenario_dict()
            data["dip_changes"] = [
                {"frame": frame, "dipswitches": {"Freeze": "On"}} for frame in frames
            ]
            with self.assertRaises(scenario.ScenarioError, msg=frames) as ctx:
                scenario.parse_scenario(data)
            self.assertIn("strictly increasing", str(ctx.exception))

    def test_unknown_dip_name_is_rejected(self):
        data = minimal_scenario_dict()
        data["machine"]["dipswitches"] = {"Free Play": "On"}
        with self.assertRaises(scenario.ScenarioError) as ctx:
            scenario.parse_scenario(data)
        self.assertIn("Free Play", str(ctx.exception))

    def test_out_of_range_or_unknown_dip_values_are_rejected(self):
        for bad in (
            {"dsw1": 0x10000},
            {"dsw1": -1},
            {"dsw2": "0x10000"},
            {"dsw2": "0xfff8"},  # unlisted Game Continuation 0x0000
            {"dsw2": "0xfd7f"},  # unlisted Buy-In Feature 0x0100
            {"Flip Screen": "Maybe"},
            {"Number of Countries Selected": "3"},
            {"Game Continuation": "0"},
            "unknown-preset",
        ):
            data = minimal_scenario_dict()
            data["machine"]["dipswitches"] = bad
            with self.assertRaises(scenario.ScenarioError, msg=repr(bad)):
                scenario.parse_scenario(data)


class InputEventsTest(unittest.TestCase):
    def test_single_press_emits_press_and_release(self):
        s = scenario.parse_scenario(minimal_scenario_dict())
        events = scenario.compute_input_events(s)
        self.assertEqual(
            [
                {"frame": 300, "port": "IN0", "mask": 0x0001, "state": 1},
                {"frame": 312, "port": "IN0", "mask": 0x0001, "state": 0},
            ],
            events,
        )

    def test_events_are_sorted_across_inputs_and_ports(self):
        data = minimal_scenario_dict()
        data["inputs"].append({"frame": 290, "press": "start1", "hold_frames": 10})
        s = scenario.parse_scenario(data)
        events = scenario.compute_input_events(s)
        self.assertEqual(
            [
                {"frame": 290, "port": "SYSTEM", "mask": 0x0001, "state": 1},
                {"frame": 300, "port": "IN0", "mask": 0x0001, "state": 1},
                {"frame": 300, "port": "SYSTEM", "mask": 0x0001, "state": 0},
                {"frame": 312, "port": "IN0", "mask": 0x0001, "state": 0},
            ],
            events,
        )


if __name__ == "__main__":
    unittest.main()
