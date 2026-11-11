import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import run_audio_probe

from test_scenario import minimal_scenario_dict


class AudioProbeDipTest(unittest.TestCase):
    def test_explicit_service_dip_is_applied_and_verified(self):
        data = minimal_scenario_dict()
        data["duration_frames"] = 10
        data["inputs"] = []
        data["observations"]["checkpoints"] = []
        data["machine"]["dipswitches"] = {"Service Mode": "On"}
        with tempfile.TemporaryDirectory() as tmp:
            scenario_path = Path(tmp) / "scenario.json"
            scenario_path.write_text(json.dumps(data), encoding="utf-8")
            out_dir = Path(tmp) / "capture"

            def fake_run(cmd, capture_output, text):
                (out_dir / "meta.tsv").write_text(
                    "dsw1\t5fff\ndsw2\tffff\n", encoding="utf-8"
                )
                return subprocess.CompletedProcess(cmd, 0, "", "")

            with mock.patch.object(run_audio_probe.subprocess, "run", side_effect=fake_run) as run:
                result = run_audio_probe.main(
                    [str(scenario_path), "--out", str(out_dir), "--no-wav"]
                )

            self.assertEqual(0, result)
            self.assertNotIn("-noreadconfig", run.call_args.args[0])
            self.assertIn(
                '<port tag=":DSW1" type="DIPSWITCH" mask="32768" defvalue="32768" value="0" />',
                (out_dir / "cfg" / "sf.cfg").read_text(encoding="utf-8"),
            )


if __name__ == "__main__":
    unittest.main()
