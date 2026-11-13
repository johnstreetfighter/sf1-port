import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))

import coverage_ledger
from scenario import load_scenario


def has_local_coverage() -> bool:
    return (coverage_ledger.COVERAGE_DIR / "sf-fixed-fight-01-run-a.tsv").is_file()


class CoverageLedgerTest(unittest.TestCase):
    def test_probe_lua_never_writes_emulated_memory_and_parses_rust_sources(self):
        scen_path = coverage_ledger.SCENARIOS_DIR / "sf_fixed_fight_01.json"
        s = load_scenario(scen_path)
        lua_code = coverage_ledger.build_lua_probe(s, pathlib.Path("/tmp/cov.tsv"))
        for forbidden in (
            "write_u8",
            "write_u16",
            "write_u32",
            "write_i8",
            "write_i16",
            "write_i32",
            "writev_",
        ):
            self.assertNotIn(forbidden, lua_code)
        self.assertIn("install_read_tap", lua_code)
        self.assertIn("set_value", lua_code)

        fns_by_cpu, note_sites = coverage_ledger.parse_rust_sources()
        self.assertGreater(len(fns_by_cpu["maincpu"]), 100)
        self.assertGreater(len(fns_by_cpu["audiocpu"]), 40)
        self.assertEqual(len(fns_by_cpu["audio2"]), 5)
        self.assertEqual(len(note_sites), 29)

    def test_probe_applies_inputs_and_records_main_instruction_pcs(self):
        s = load_scenario(coverage_ledger.SCENARIOS_DIR / "sf_hadoken_01.json")
        lua_code = coverage_ledger.build_lua_probe(s, pathlib.Path("/tmp/cov.tsv"))
        self.assertIn('local p = ports[":" .. port_tag]', lua_code)
        self.assertNotIn("if offset == pc and main_seen[pc]", lua_code)
        self.assertIn("if pc >= 0x3b000 and pc <= 0x492d5 and main_seen[pc] == nil then", lua_code)

    def test_probe_applies_timed_dip_changes_before_inputs(self):
        scen_path = coverage_ledger.SCENARIOS_DIR / "sf_freeze_01.json"
        s = load_scenario(scen_path)
        lua_code = coverage_ledger.build_lua_probe(s, pathlib.Path("/tmp/cov.tsv"))
        self.assertIn("local dip_changes = {", lua_code)
        self.assertIn(
            '{frame=2500, changes={{port="DSW1", mask=0x4000, value=0x0000}}}',
            lua_code,
        )
        self.assertIn(
            '{frame=2560, changes={{port="DSW1", mask=0x4000, value=0x4000}}}',
            lua_code,
        )
        self.assertIn("field.user_value = change.value", lua_code)
        self.assertIn("  apply_dip_changes(frame)\n  apply_events_for_frame(frame)", lua_code)

    def test_two_coverage_runs_per_scenario_are_identical(self):
        if not has_local_coverage():
            self.skipTest("local coverage probe data not present")
        summaries = coverage_ledger.ensure_probe_runs(force_rerun=False)
        self.assertEqual(len(summaries), 137)
        for s in summaries:
            self.assertTrue(s["identical"], f"Run A and Run B differ for {s['id']}")
            self.assertGreater(s["maincpu_pcs"], 0)
            self.assertGreater(s["audiocpu_pcs"], 0)
            self.assertGreater(s["audio2_pcs"], 0)

    def test_ledger_maps_all_executed_addresses_and_reproduces_documented_gaps(self):
        if not has_local_coverage():
            self.skipTest("local coverage probe data not present")
        ledger = coverage_ledger.build_ledger()
        self.assertEqual(ledger["scenario_count"], 137)
        self.assertEqual(
            ledger["cpu_totals"]["maincpu"],
            {"executed_pcs": 9287, "translated_pcs": 9287, "untranslated_pcs": 0},
        )
        self.assertEqual(
            ledger["cpu_totals"]["audiocpu"],
            {"executed_pcs": 1394, "translated_pcs": 1394, "untranslated_pcs": 0},
        )
        self.assertEqual(
            ledger["cpu_totals"]["audio2"],
            {"executed_pcs": 223, "translated_pcs": 223, "untranslated_pcs": 0},
        )
        self.assertEqual(
            ledger["normal_play_cpu_totals"]["maincpu"],
            {"executed_pcs": 8377, "translated_pcs": 8377, "untranslated_pcs": 0},
        )
        self.assertEqual(
            ledger["normal_play_cpu_totals"]["audiocpu"],
            {"executed_pcs": 1378, "translated_pcs": 1378, "untranslated_pcs": 0},
        )
        self.assertEqual(
            ledger["normal_play_cpu_totals"]["audio2"],
            {"executed_pcs": 223, "translated_pcs": 223, "untranslated_pcs": 0},
        )
        self.assertEqual(len(ledger["undocumented_untranslated"]), 0)
        self.assertEqual(len(ledger["documented_gaps"]), 0)
        for g in ledger["documented_gaps"]:
            self.assertTrue(g["gap_doc_verified"], f"Unverified gap doc for {g}")
            self.assertFalse(g["translated"])
            self.assertGreater(g["executed_pc_count"], 0)

        # Every note() / note_unrecovered() site is classified as reached or unreached
        self.assertEqual(len(ledger["note_sites"]), 29)
        reached_notes = [n for n in ledger["note_sites"] if n["reached"]]
        unreached_notes = [n for n in ledger["note_sites"] if not n["reached"]]
        self.assertEqual(len(reached_notes), 0)
        self.assertEqual(len(unreached_notes), 29)
        raw_guards = [
            n for n in unreached_notes
            if n["short_file"] == "music.rs" and n["fn"] in ("raw_tick", "raw_opcode")
        ]
        self.assertEqual({n["fn"] for n in raw_guards}, {"raw_tick", "raw_opcode"})
        for guard in raw_guards:
            self.assertTrue(
                guard["disposition"].startswith("Unreachable under MAME: raw-score"),
                guard,
            )

    def test_negative_control_removing_citation_marks_executed_addresses_untranslated(self):
        if not has_local_coverage():
            self.skipTest("local coverage probe data not present")
        neg = coverage_ledger.verify_negative_control()
        self.assertEqual(neg["untranslated_range"], "0x40bd4..0x40d2e")
        self.assertEqual(neg["untranslated_pc_count"], 87)
        self.assertEqual(neg["first_pc"], "0x40bd4")
        self.assertEqual(neg["last_pc"], "0x40d2e")

        # Also test removing an audiocpu routine citation (e.g. 0x0551 key_on in music.rs)
        with tempfile.TemporaryDirectory() as tmp:
            tmp_root = pathlib.Path(tmp)
            for rel_path, _ in coverage_ledger.RUST_SOURCES:
                src = coverage_ledger.REPO_ROOT / rel_path
                dst = tmp_root / rel_path
                dst.parent.mkdir(parents=True, exist_ok=True)
                txt = src.read_text(encoding="utf-8")
                if rel_path.endswith("music.rs"):
                    txt = txt.replace("0x0551", "0x9999")
                dst.write_text(txt, encoding="utf-8")
            neg_audio = coverage_ledger.build_ledger(rust_root=tmp_root)
            undoc = neg_audio["undocumented_untranslated"]
            self.assertEqual(len(undoc), 1)
            self.assertEqual(undoc[0]["cpu"], "audiocpu")
            self.assertEqual(undoc[0]["cite_addr"], "0x0551")
            self.assertEqual(undoc[0]["executed_pc_count"], 34)


if __name__ == "__main__":
    unittest.main()
