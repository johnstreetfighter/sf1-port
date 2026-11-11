#!/usr/bin/env python3
"""Generate the translation coverage ledger from every pinned reference scenario (ticket #51 / F11).

Usage:
    python tools/reference/coverage_ledger.py [--probe] [--check]
"""

from __future__ import annotations

import argparse
import collections
import concurrent.futures
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
from typing import Any

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools" / "reference"))
from lua_gen import generate_cfg  # noqa: E402
from run_scenario import BASELINE_OPTIONS  # noqa: E402
from scenario import DIP_FIELDS, Scenario, compute_input_events, load_scenario  # noqa: E402

SCENARIOS_DIR = REPO_ROOT / "tools" / "reference" / "scenarios"
LOCAL_DIR = REPO_ROOT / "tools" / "reference" / "local"
COVERAGE_DIR = LOCAL_DIR / "coverage"
MAME_DIR = LOCAL_DIR / "mame"
MAME_EXE = MAME_DIR / "mame.exe"
ROMS_DIR = LOCAL_DIR / "roms"
LEDGER_MD = REPO_ROOT / "docs" / "reference" / "coverage-ledger.md"
LEDGER_JSON = COVERAGE_DIR / "ledger.json"

RUST_SOURCES = [
    ("crates/sf-engine/src/fighter.rs", "maincpu"),
    ("crates/sf-engine/src/tasks.rs", "maincpu"),
    ("crates/sf-audio/src/music.rs", "audiocpu"),
    ("crates/sf-audio/src/samples.rs", "audio2"),
]

# Original program routine intervals: (start_addr, end_addr, cite_addr, description, gap_doc)
# - If cite_addr is cited by a translated Rust function in the corresponding crate,
#   the routine's executed addresses map to the citing Rust function(s).
# - If cite_addr is not cited by any Rust function, the routine is untranslated;
#   if gap_doc is present and verified in docs/, it is a documented gap, otherwise
#   its executed addresses are listed as executed, untranslated addresses that no doc names.
MAINCPU_ROUTINES: list[tuple[int, int, int, str, str | None]] = [
    (0x3B000, 0x3B026, 0x3B000, "reset RAM clear and self-test check", None),
    (0x3B02A, 0x3B02A, 0x4834E, "reset self-test jump to 0x4834e", None),
    (0x3B030, 0x3B04E, 0x3B000, "reset boot init and slot-0 spawn", None),
    (0x3B052, 0x3B058, 0x3B052, "vblank entry", None),
    (0x3B05A, 0x3B0C8, 0x3B05A, "vblank_buttons (P1/P2 punch/kick release latch calls)", None),
    (0x3B0CC, 0x3B108, 0x3B052, "vblank coin debounce calls, SYSTEM bit 7 check, palette check, Freeze DIP check", None),
    (0x3B10A, 0x3B12A, 0x3B10A, "latch_in1 (copy IN1 to 0xff868e)", None),
    (0x3B12C, 0x3B170, 0x3B12C, "pop_sound (pop sound command ring to latch)", None),
    (0x3B172, 0x3B198, 0x3B052, "vblank frame counter increment and sleeping-slot countdown", None),
    (0x3B19A, 0x3B1E6, 0x3B19A, "Scheduler::run (cooperative slot scan loop)", None),
    (0x3B1E8, 0x3B208, 0x3B1E8, "enter_new_task and ready-slot resume", None),
    (0x3B20A, 0x3B22E, 0x3B20A, "Scheduler::spawn (trap #0)", None),
    (0x3B230, 0x3B24C, 0x3B230, "Scheduler::kill (trap #1)", None),
    (0x3B24E, 0x3B286, 0x3B24E, "Scheduler::apply_yield (trap #2 sleep)", None),
    (0x3B29A, 0x3B2B2, 0x3B29A, "Scheduler::apply_yield (trap #3 task exit)", None),
    (0x3B2B8, 0x3B2BC, 0x3B2B8, "current_slot_address", None),
    (0x3B2BE, 0x3B2C8, 0x3B2BE, "slot_address", None),
    (0x3B2E4, 0x3B318, 0x3B2E4, "decode_coinage", None),
    (0x3B31A, 0x3B360, 0x3B31A, "coin_debounce", None),
    (0x3B362, 0x3B38A, 0x3B362, "vblank palette upload service", None),
    (0x3B38C, 0x3B3FE, 0x3B3AE, "button_latch", None),
    (0x3B402, 0x3B53E, 0x3B454, "latch_cycles and trap #2 sleep duration wrappers", None),
    (0x3B552, 0x3B5A8, 0x3B582, "multi-frame sleep wrappers 0x3b552..0x3b5a8", None),
    (0x3B5AA, 0x3B5C4, 0x3B5AA, "speed_sleep", None),
    (0x3B5C8, 0x3B5F6, 0x3B5C8, "service_screen_init", None),
    (0x3B5F8, 0x3B642, 0x3B5F8, "upload_palette", None),
    (0x3B644, 0x3B652, 0x3B644, "gfx_on", None),
    (0x3B654, 0x3B662, 0x3B654, "coin_ctrl_or", None),
    (0x3B664, 0x3B672, 0x3B664, "aux_ctrl_or", None),
    (0x3B674, 0x3B680, 0x3B674, "add_fg_scroll", None),
    (0x3B682, 0x3B68E, 0x3B682, "add_bg_scroll", None),
    (0x3B690, 0x3B6A0, 0x3B690, "gfx_off", None),
    (0x3B6A2, 0x3B6B2, 0x3B6A2, "coin_ctrl_off", None),
    (0x3B6B4, 0x3B6C4, 0x3B6B4, "aux_ctrl_off", None),
    (0x3B6C6, 0x3B728, 0x3B6C6, "reset_button_latches", None),
    (0x3B77A, 0x3B7A6, 0x3B77A, "boot_init", None),
    (0x3B7A8, 0x3B7C6, 0x3B7A8, "queue_boot_sounds", None),
    (0x3B7C8, 0x3B800, 0x3B7C8, "queue_sound", None),
    (0x3B802, 0x3B826, 0x3B802, "queue_sound_unfiltered", None),
    (0x3B828, 0x3B850, 0x3B828, "effect_flag_spawn", None),
    (0x3B852, 0x3B8BA, 0x3B852, "swap_regions", None),
    (0x3B8BC, 0x3B8FE, 0x3B8BC, "copy_grid and clear buffer positions", None),
    (0x3B900, 0x3B912, 0x3B904, "fill_words_4", None),
    (0x3B914, 0x3B932, 0x3B914, "fill_words_16", None),
    (0x3B934, 0x3B96A, 0x3B934, "clear_words_4 and clear_words_8", None),
    (0x3B96C, 0x3B9BA, 0x3B96C, "clear_bytes_3", None),
    (0x3B9BC, 0x3B9C6, 0x3B9BC, "clear_objects", None),
    (0x3B9C8, 0x3B9EE, 0x3B9C8, "clear_fighters", None),
    (0x3B9F0, 0x3BA0C, 0x3B9F0, "add_score", None),
    (0x3BA0E, 0x3BA48, 0x3BA12, "decimal_add", None),
    (0x3BA84, 0x3BAC0, 0x3BA84, "check_hiscore", None),
    (0x3BADA, 0x3BAF4, 0x3BADA, "tally_vitality BCD add helper", None),
    (0x3BAF6, 0x3BAFC, 0x3BAF6, "compare_bytes_3", None),
    (0x3BAFE, 0x3BB04, 0x3BAFE, "compare_bytes_7", None),
    (0x3BB06, 0x3BB0C, 0x3BB06, "compare_bytes_15", None),
    (0x3BB0E, 0x3BB32, 0x3BB0E, "compare_bytes", None),
    (0x3BB48, 0x3BB5A, 0x3BB48, "round_timer digit pair writer", None),
    (0x3BB5C, 0x3BB72, 0x3BB5C, "decimal_up", None),
    (0x3BB74, 0x3BB9E, 0x3BB74, "decimal_down", None),
    (0x3BBA0, 0x3BC3C, 0x3BBA0, "text_block and TextWriter typewriter", None),
    (0x3BC3E, 0x3BCFE, 0x3BC3E, "text_block_2x2", None),
    (0x3BD00, 0x3BD3C, 0x3BD00, "erase_block", None),
    (0x3BD3E, 0x3BD94, 0x3BD3E, "erase_block_2x2", None),
    (0x3BD96, 0x3BD98, 0x3BD96, "erase_credit_digits", None),
    (0x3BD9A, 0x3BDE4, 0x3BDB0, "clear_text_layer", None),
    (0x3BDE6, 0x3BDF0, 0x3BDE6, "draw_digits_blanked", None),
    (0x3BDF2, 0x3BDF8, 0x3BDF2, "draw_digits", None),
    (0x3BDFA, 0x3BE0A, 0x3BDE6, "draw_digits_blanked leading-zero loop", None),
    (0x3BE0C, 0x3BE1C, 0x3BE0C, "show_digits", None),
    (0x3BE1E, 0x3BE88, 0x3BE6A, "show_score", None),
    (0x3BE8A, 0x3BEB2, 0x3BE8A, "draw_credit_digits", None),
    (0x3BEB4, 0x3BED0, 0x3BEB4, "draw_time_digits", None),
    (0x3BED2, 0x3BF20, 0x3BED2, "round_markers", None),
    (0x3BF22, 0x3BF58, 0x3BF22, "health_bars", None),
    (0x3BF5A, 0x3BFBA, 0x3BF5A, "binary_to_digits", None),
    (0x3BFFC, 0x3C032, 0x3BFFC, "write_hex_word and write_hex_byte", None),
    (0x3C034, 0x3C0AE, 0x3C034, "damage", None),
    (0x3C0B0, 0x3C0C2, 0x3C0B0, "health_points", None),
    (0x3C0C4, 0x3C110, 0x3C0C4, "boot (slot 0 palette uploads)", None),
    (0x3C112, 0x3C20E, 0x3C112, "boot_setup_chunk", None),
    (0x3C210, 0x3CCBE, 0x3C210, "boot warning screen 0x3c210..0x3c290 and boot-to-attract join 0x3ccae..0x3ccbc", None),
    (0x3CCC0, 0x3CCE6, 0x3CCC2, "attract loop head 0x3ccc2..0x3cce6", None),
    (0x3CCEC, 0x3CD5E, 0x3CD14, "attract_scroll_loop", None),
    (0x3CD60, 0x3CE2E, 0x3CD60, "attract_title_block and 300-frame title wait", None),
    (0x3CE30, 0x3CE52, 0x3CE30, "attract ranking call 0x3ce30..0x3ce4e", None),
    (0x3CE54, 0x3CE8A, 0x3CE54, "attract demo setup 0x3ce54..0x3ce8a", None),
    (0x3CE8C, 0x3CEBA, 0x3CE8C, "attract demo round-end return 0x3ce8c..0x3ceba", None),
    (0x3DEC2, 0x3DF4A, 0x3DEC2, "start_screen_entry", None),
    (0x3DF4C, 0x3E04A, 0x3DF4C, "start_screen_frame", None),
    (0x3E04C, 0x3E1E2, 0x3E04C, "country_select_start, country_select_arm and country_select_frame", None),
    (0x3E1E4, 0x3E370, 0x3E1E4, "country_select4_start, country_select4_arm and country_select4_frame", None),
    (0x3E372, 0x3E596, 0x3E372, "country_select_exit", None),
    (0x3E59C, 0x3E5C4, 0x3E59C, "flow StageStart", None),
    (0x3E5C6, 0x3E5D4, 0x3E5C6, "stage start demo branch 0x3e5c6..0x3e5d4", None),
    (0x3E5D6, 0x3E6C8, 0x3E5D6, "place_players", None),
    (0x3E6CA, 0x3E6FA, 0x3E6CA, "next_opponent", None),
    (0x3E6FE, 0x3E712, 0x3E6FE, "next_opponent demo branch 0x3e6fe..0x3e712", None),
    (0x3E714, 0x3E9B8, 0x3E78C, "round_hud", None),
    (0x3E9BC, 0x3EA3C, 0x3E9BC, "intro_frame", None),
    (0x3EA3E, 0x3EA64, 0x3EA3E, "round_start", None),
    (0x3EA66, 0x3EAF8, 0x3EA66, "fight_frame", None),
    (0x3EAFA, 0x3EB20, 0x3EAFA, "fight_checks", None),
    (0x3EB22, 0x3EB9E, 0x3EB22, "flow KO and settle_frame", None),
    (0x3EBA0, 0x3EC3E, 0x3EBA2, "record_win and victory wait", None),
    (0x3EC40, 0x3EC6C, 0x3EC40, "after_round", None),
    (0x3EC6E, 0x3ECA0, 0x3EC6E, "three_rounds", None),
    (0x3ECA2, 0x3ECEE, 0x3ECA2, "double_ko_start and outcome_frame", None),
    (0x3ECF0, 0x3ED48, 0x3ECF0, "double_ko_hold", None),
    (0x3ED4A, 0x3ED96, 0x3ED4A, "time_over_hold", None),
    (0x3ED98, 0x3EE24, 0x3ED98, "time_over_judge", None),
    (0x3EE26, 0x3EF94, 0x3EE26, "match_end", None),
    (0x3EF96, 0x3EFC2, 0x3EF96, "country's-end dispatch 0x3ef96..0x3efc2", None),
    (0x3EFC4, 0x3EFD8, 0x3EFC4, "stage_step", None),
    (0x3EFDA, 0x3F06E, 0x3EFDA, "continue_start", None),
    (0x3F070, 0x3F1BA, 0x3F092, "continue_frame and continue acceptance 0x3f166..0x3f1b8", None),
    (0x3F1BC, 0x3F372, 0x3F1BC, "game_over_start, game_over_cycle, game_over_drain, versus_game_over, game_over_screen and game_over_reset", None),
    (0x3F374, 0x3F386, 0x3F374, "watch task entry 0x3f374..0x3f386", None),
    (0x3F388, 0x3F4DC, 0x3F388, "watch (including challenger join 0x3f4a2..0x3f4da)", None),
    (0x3F4DE, 0x3F7AE, 0x3F4E2, "round_timer", None),
    (0x3F7B0, 0x3F810, 0x3F7B0, "screen event poll and event-2 stage-0 check", None),
    (0x3F812, 0x3F8E2, 0x3F812, "travel_map_start and travel map event 2 0x3f812..0x3f8e2", None),
    (0x3F8E4, 0x3F9E8, 0x3F8E4, "country_intro_start", None),
    (0x3F9EC, 0x3FB50, 0x3F9EC, "opponent_intro_start", None),
    (0x3FBA2, 0x3FDA6, 0x3FCD4, "screen events 5, 6, 7 (announce, wipe, round result)", None),
    (0x3FDA8, 0x3FDF6, 0x3FDA8, "time_copy_zero", None),
    (0x3FDF8, 0x3FE04, 0x3FDF8, "tally_time", None),
    (0x3FE06, 0x3FE38, 0x3FE06, "tally_add", None),
    (0x3FE3A, 0x3FE62, 0x3FE3A, "tally_redraw", None),
    (0x3FE64, 0x3FEEE, 0x3FE64, "tally_vitality", None),
    (0x3FEF0, 0x4001A, 0x3FCD4, "screen events 8 and 9 (draw and projectile slow motion)", None),
    (0x4001C, 0x402A0, 0x4001C, "win_screen_start (event 0xb)", None),
    (0x402EE, 0x4030A, 0x402F8, "input_handler entry", None),
    (0x4030C, 0x4035E, 0x4030C, "demo input reader 0x4030c..0x4035e", None),
    (0x40360, 0x403D2, 0x40360, "track_commands", None),
    (0x403D4, 0x40416, 0x402F8, "input_handler special move trigger", None),
    (0x40418, 0x4050A, 0x40418, "keep_facing", None),
    (0x4050C, 0x4052A, 0x4050C, "input_tail", None),
    (0x4052C, 0x405C4, 0x4052C, "input_neutral_dir", None),
    (0x405C6, 0x406EC, 0x405C6, "input_right, opponent_attacks_high, start_walk_attack", None),
    (0x406EE, 0x40810, 0x406EE, "input_left", None),
    (0x40812, 0x40892, 0x40812, "input_down", None),
    (0x40894, 0x4098C, 0x40894, "input_down_right, opponent_attacks_low, down_forward_punch, start_low_kick, start_low_punch", None),
    (0x4098E, 0x40AE2, 0x4098E, "input_down_left", None),
    (0x40AE4, 0x40B1E, 0x40AE4, "track_command", None),
    (0x40B20, 0x40B70, 0x40B20, "count_tracker", None),
    (0x40B72, 0x40B8C, 0x40B72, "pick_by_strength", None),
    (0x40B8E, 0x40BD2, 0x40B8E, "pick_preloaded", None),
    (0x40BD4, 0x40D2E, 0x40BD4, "cpu_decide", None),
    (0x40D30, 0x412A4, 0x40D30, "Japan bonus stage: bonus_japan_start, bonus_japan_meter_frame, bonus_japan_strike, bonus_japan_windup, bonus_japan_judge, bonus_japan_break and bonus_japan_cheer_b 0x40d30..0x412a4", None),
    (0x4141A, 0x4159C, 0x4141A, "bonus stage helpers: bonus_place_grid, bonus_update_sprites, bonus_draw_gauge, bonus_load_palette_tail and bonus_resolve_power 0x4141a..0x4159c", None),
    (0x41740, 0x419FC, 0x41740, "stage-1 10-second bonus: bonus_stage1_start, bonus_stage1_frame and bonus_stage1_settle_step 0x41740..0x419fc", None),
    (0x41A02, 0x41F5C, 0x41A02, "stage-2 power/timing bonus: bonus_japan_start, bonus_japan_meter_frame, bonus_japan_strike, bonus_japan_windup, bonus_japan_judge, bonus_japan_break and bonus_japan_cheer_b 0x41a02..0x41f5c", None),
    (0x41FE6, 0x422B8, 0x41FE6, "stage-3 10-second bonus: bonus_stage1_start, bonus_stage1_frame and bonus_stage1_settle_step 0x41fe6..0x422b8", None),
    (0x422BA, 0x422EA, 0x422D0, "request_event and event wait helpers", None),
    (0x422EC, 0x42300, 0x422EC, "draw_time_copy", None),
    (0x42302, 0x42316, 0x42302, "draw_vitality", None),
    (0x42318, 0x42330, 0x42318, "draw_bonus", None),
    (0x42332, 0x42358, 0x42332, "stage_music", None),
    (0x423B8, 0x42420, 0x423B8, "challenger join sequence 0x423b8..0x4241c", None),
    (0x42422, 0x42484, 0x42422, "ranking_check", None),
    (0x42486, 0x425C2, 0x42486, "name_entry_start, name_entry_arm and name_entry_frame 0x42486..0x425c2", None),
    (0x425C4, 0x4268A, 0x425C4, "name entry ranking insertion and display 0x425c4..0x4268a", None),
    (0x4268C, 0x4274E, 0x4268C, "attract_ranking_start and attract_ranking_wait 0x4268c..0x4274e", None),
    (0x42750, 0x427E0, 0x42750, "marked-entry ranking blink loop 0x42750..0x427e0", None),
    (0x427E2, 0x429F0, 0x427E2, "cursor_move_and_button and cursor_commit_letter 0x427e2..0x429f0", None),
    (0x429F2, 0x42A1A, 0x429F2, "dispatch_order", None),
    (0x42A1C, 0x42ACE, 0x42A1C, "run_player", None),
    (0x42AD0, 0x42B32, 0x42AD0, "run_fighter", None),
    (0x42B34, 0x42BE0, 0x42B34, "buffer_air_strength", None),
    (0x42BE2, 0x42C48, 0x42BE2, "animate", None),
    (0x42C4A, 0x42D20, 0x42C4A, "end_move and animate frame/collision continuation", None),
    (0x42D22, 0x42D44, 0x42D22, "run_effects", None),
    (0x42D46, 0x42D6C, 0x42D46, "run_effect", None),
    (0x42D6E, 0x42DD0, 0x42D6E, "effect_collisions and run_effect stream control", None),
    (0x42DD2, 0x42DD6, 0x42DD2, "run_effect stream-call dispatch", None),
    (0x42DD8, 0x42E1C, 0x42D46, "run_effect stream 0xfe/0xfd and frame load", None),
    (0x42E1E, 0x42EB2, 0x42E1E, "spawn_effect", None),
    (0x42EB4, 0x42F1C, 0x42EB4, "transform_effect", None),
    (0x42F1E, 0x42FC4, 0x42F1E, "spawn_effect_absolute", None),
    (0x42FC6, 0x430B4, 0x42FC6, "run_effect frame load and slot finder 0x4304c", None),
    (0x430B6, 0x43142, 0x430B6, "effect_vs_fighter", None),
    (0x43144, 0x431C4, 0x43144, "effect_vs_attack", None),
    (0x431C6, 0x43232, 0x431C6, "effect_vs_projectile", None),
    (0x43234, 0x43266, 0x43234, "effect_centre", None),
    (0x43268, 0x4329A, 0x43268, "effect_offscreen", None),
    (0x4329C, 0x432C2, 0x4329C, "live_effect_count", None),
    (0x432C4, 0x43346, 0x432C4, "travel-map plane stream routines: plane_loop_start, plane_loop_next, plane_flip and plane_sound 0x432c4..0x43338", None),
    (0x43348, 0x433B8, 0x43348, "Geki shuriken effect handlers 0x43348..0x433b8", None),
    (0x433BA, 0x43474, 0x433BA, "fireball_hits_fighter", None),
    (0x43476, 0x434D6, 0x43476, "effect_handler", None),
    (0x434E4, 0x43651, 0x434E4, "stage-1 and stage-3 bonus stack break effect handlers 0x434e4..0x43651", None),
    (0x43652, 0x436B6, 0x43652, "Sagat Tiger Shot projectile collision handler 0x43652..0x436b6", None),
    (0x436B8, 0x436E8, 0x436B8, "request_country_palette", None),
    (0x43748, 0x43778, 0x43748, "request_stage_palette", None),
    (0x437D8, 0x43848, 0x437D8, "stage_reset", None),
    (0x438EA, 0x4392A, 0x438EA, "drift_stage", None),
    (0x4392C, 0x43AA6, 0x4392C, "resolve_camera", None),
    (0x43AA8, 0x43B54, 0x43AA8, "camera_room", None),
    (0x43B56, 0x43B68, 0x43B56, "shift_grid_x", None),
    (0x43B6A, 0x43BD8, 0x43B6A, "place_frame and half_extent", None),
    (0x43BDA, 0x43BE6, 0x43BDA, "center_y", None),
    (0x43BE8, 0x43C16, 0x43BE8, "center_x", None),
    (0x43C18, 0x43C80, 0x43C18, "clamp_y", None),
    (0x43C82, 0x43C98, 0x43C82, "shift_grid_y", None),
    (0x43C9A, 0x43D7A, 0x43C9A, "clamp_x", None),
    (0x43D7C, 0x43E80, 0x43D7C, "push_apart", None),
    (0x43E82, 0x43FA6, 0x43E82, "judge", None),
    (0x43FA8, 0x44058, 0x43FA8, "face_left and mirror_codes", None),
    (0x4405A, 0x44112, 0x4405A, "face_right", None),
    (0x44114, 0x441C2, 0x441B0, "start_move", None),
    (0x441C4, 0x442E0, 0x441C4, "start_move_at", None),
    (0x442E2, 0x443C4, 0x442E2, "load_frame", None),
    (0x443C6, 0x4445C, 0x443C6, "write_grid", None),
    (0x4445E, 0x44504, 0x4445E, "write_grid_flipped", None),
    (0x44506, 0x44518, 0x44506, "park_from", None),
    (0x4451A, 0x4485E, 0x4451A, "react", None),
    (0x44860, 0x4487E, 0x44860, "setup_cpu_opponent", None),
    (0x44880, 0x44A3E, 0x44880, "ai_table_copies", None),
    (0x44A40, 0x44C1C, 0x44A40, "ending sequence: ending_start, ending_scene_outer, ending_scene_step, ending_message_type, ending_staff_block and ending_staff_type 0x44a40..0x44c1c", None),
    (0x452E0, 0x453B4, 0x452F4, "jump_attack (Ryu)", None),
    (0x453B6, 0x4562E, 0x453CA, "jump_attack (Ken) and fighter stream_call routines", None),
    (0x45630, 0x45718, 0x45630, "cpu_follow_up", None),
    (0x4571A, 0x45732, 0x4571A, "cpu_react", None),
    (0x45734, 0x457F8, 0x45734, "cpu_face", None),
    (0x457FA, 0x45948, 0x457FA, "cpu_attack", None),
    (0x4594A, 0x45A78, 0x4596E, "Geki stream calls and teleport 0x4594a..0x45a78", "docs/reference/match-win.md; docs/planning/arcade-flow-batch.md (F27)"),
    (0x45A7A, 0x45AEE, 0x45A7A, "stream_call Retsu and Sagat helpers 0x45a7a..0x45aee", None),
    (0x45AF0, 0x45D24, 0x45AF0, "hit_basic (standing punches)", None),
    (0x45D26, 0x45EF8, 0x45D26, "hit_strength (crouching and jumping punches)", None),
    (0x45EFA, 0x45F58, 0x45EFA, "HitFn::Shoryuken", None),
    (0x45F5A, 0x46028, 0x45F5A, "hit_tatsumaki", None),
    (0x4602A, 0x460B6, 0x4602A, "hit_basic (standing light kick)", None),
    (0x460B8, 0x4624E, 0x460B8, "hit_redirect (standing medium/heavy kicks)", None),
    (0x46250, 0x46470, 0x46250, "hit_strength (crouching/jumping kicks) and sweep", None),
    (0x46472, 0x46648, 0x4656E, "Geki hit functions 0x46492..0x46648", "docs/reference/milestone-audit.md; docs/reference/match-win.md; docs/planning/arcade-flow-batch.md (F27)"),
    (0x4664A, 0x46926, 0x4664A, "HitFn::Cpu (Retsu hit functions 0x4664a..0x468de)", None),
    (0x4692E, 0x46AE4, 0x4692E, "HitFn::Cpu (Mike hit functions 0x4692e..0x46a78)", None),
    (0x46AE6, 0x46E54, 0x46AE6, "HitFn::Cpu (Joe hit functions 0x46ae6..0x46de8)", None),
    (0x46E56, 0x47232, 0x46E56, "HitFn::Cpu (Lee hit functions 0x46e56..0x471c6)", None),
    (0x47234, 0x475A2, 0x47234, "HitFn::Cpu (Gen hit functions 0x47234..0x47536)", None),
    (0x475A4, 0x47910, 0x475A4, "HitFn::Cpu (Birdie hit functions 0x475a4..0x478a6)", None),
    (0x47914, 0x47C14, 0x47914, "HitFn::Cpu (Eagle hit functions 0x47914..0x47ba8)", None),
    (0x47C16, 0x47EA8, 0x47C16, "HitFn::Cpu (Adon hit functions 0x47c16..0x47e3c)", None),
    (0x47EAA, 0x4813D, 0x47EAA, "HitFn::Cpu (Sagat hit functions 0x47eaa..0x480d0)", None),
    (0x4813E, 0x4820E, 0x4813E, "random and step_random", None),
    (0x48210, 0x48226, 0x48212, "random_after_service", None),
    (0x48228, 0x4824C, 0x48228, "random_after_service attract start with credits 0x48228..0x4824c", None),
    (0x4824E, 0x48272, 0x48256, "random_pulses_from", None),
    (0x4827C, 0x48324, 0x4827C, "coin_slot_step", None),
    (0x48326, 0x4834C, 0x48326, "random_pulses_from coin-counter pulse", None),
    (0x4834E, 0x483F6, 0x4834E, "ram_test_start and ram_test_finish 0x4834e..0x483f2", None),
    (0x483F8, 0x48642, 0x483F8, "service_input and service_input_loop 0x483f8..0x48642", None),
    (0x48644, 0x487F6, 0x48644, "service_menu, service_menu_draw and service_menu_step_cursor 0x48644..0x487f6", None),
    (0x487F8, 0x48856, 0x487F8, "service_crosshatch_init and service_item 0x487f8..0x48856", None),
    (0x48858, 0x488F6, 0x48858, "service_char_test1_init and service_item 0x48858..0x488f6", None),
    (0x488F8, 0x4898E, 0x488F8, "service_color_bar_init and service_item 0x488f8..0x4898e", None),
    (0x48990, 0x48B68, 0x48990, "service_char_rom_init, service_char_rom_loop, char_rom_update_page, char_rom_update_color and char_rom_apply_mask 0x48990..0x48b68", None),
    (0x48B6A, 0x48DA2, 0x48B6A, "service_sprite_test_init, service_sprite_test_loop, sprite_test_update_page, sprite_test_update_color and sprite_test_row_headers 0x48b6a..0x48da2", None),
    (0x48DA4, 0x490F6, 0x48DA4, "service_scroll_test_init_fg, service_scroll_test_init_bg, service_scroll_test_fg_step, service_scroll_test_bg_step, scroll_test_step_input, scroll_test_draw_fg_coords and scroll_test_draw_bg_coords 0x48da4..0x490f6", None),
    (0x490F8, 0x4929A, 0x490F8, "service_output_test_init, output_test_reset_outputs and output_test_apply_step 0x490f8..0x4929a", None),
    (0x4929C, 0x492D5, 0x4929C, "erase_service_menu 0x4929c..0x492d5", None),
]

AUDIOCPU_ROUTINES: list[tuple[int, int, int, str, str | None]] = [
    (0x0000, 0x0008, 0x0000, "reset entry into 0x0070", None),
    (0x0009, 0x000B, 0x0009, "idle loop (ei; jr 0x0009)", None),
    (0x0018, 0x001D, 0x0018, "rst 0x18 (HL += A)", None),
    (0x0020, 0x0026, 0x0020, "rst 0x20 (A = (HL + A))", None),
    (0x0028, 0x002E, 0x0028, "rst 0x28 (DE = table[A])", None),
    (0x0030, 0x0033, 0x0030, "rst 0x30 (inline table jump)", None),
    (0x0038, 0x0065, 0x0038, "IM 1 timer interrupt handler", None),
    (0x0066, 0x006F, 0x0066, "NMI latch handler", None),
    (0x0070, 0x009C, 0x0070, "power-on boot setup", None),
    (0x009D, 0x00B8, 0x009D, "mute channel operators", None),
    (0x00B9, 0x00C9, 0x00B9, "key off channel", None),
    (0x00CA, 0x00E1, 0x00CA, "rearm Timer A", None),
    (0x00E2, 0x00FB, 0x00E2, "rearm Timer B", None),
    (0x00FC, 0x010D, 0x00FC, "YM2151 busy wait and register write", None),
    (0x010E, 0x0154, 0x010E, "8x8 unsigned multiply (mul8)", None),
    (0x0155, 0x017C, 0x0155, "16/8 unsigned divide (div16)", None),
    (0x017D, 0x019C, 0x017D, "8-bit fractional multiply (frac8)", None),
    (0x019D, 0x0227, 0x019D, "command queue poll and dispatch", None),
    (0x0228, 0x023F, 0x0228, "load tune channel table", None),
    (0x0240, 0x02A4, 0x0240, "start FM effect", None),
    (0x02A5, 0x02BA, 0x02A5, "special command 0x30..0x3f dispatch", None),
    (0x02BB, 0x032A, 0x02BB, "stop FM effect slot 0/1", None),
    (0x032B, 0x03A3, 0x032B, "re-establish music channel after effect", None),
    (0x03A4, 0x0411, 0x03A7, "stop music / pop saved music", None),
    (0x0412, 0x0423, 0x0412, "discard saved music", None),
    (0x0424, 0x0452, 0x0424, "music tick and fade", None),
    (0x0453, 0x047B, 0x0453, "tick music channels", None),
    (0x047C, 0x048B, 0x047C, "single music channel tick", None),
    (0x048C, 0x04A6, 0x048C, "channel duration countdown", None),
    (0x04A7, 0x04CD, 0x04A7, "channel gate countdown", None),
    (0x04CE, 0x04D1, 0x04CE, "next music event entry", None),
    (0x04D2, 0x0507, 0x04D2, "read music stream events", None),
    (0x0508, 0x0550, 0x0508, "music note event", None),
    (0x0551, 0x059A, 0x0551, "music key on", None),
    (0x059B, 0x05FF, 0x059B, "music operator levels", None),
    (0x0600, 0x064A, 0x0600, "music key scale", None),
    (0x064B, 0x06F4, 0x064B, "level curve lookup", None),
    (0x06F5, 0x0800, 0x06F5, "nibble multiply table lookup", None),
    (0x0801, 0x0860, 0x0801, "set note duration", None),
    (0x0861, 0x08E3, 0x0861, "music stream opcode 0x00..0x12", None),
    (0x08E4, 0x0923, 0x08E4, "load music patch", None),
    (0x0924, 0x09F7, 0x0924, "write patch operator registers", None),
    (0x09F8, 0x09FF, 0x09F8, "write channel register (write_ch)", None),
    (0x0A00, 0x0A0F, 0x0A00, "key off music channel (key_off_ch)", None),
    (0x0A10, 0x0A19, 0x0A10, "check channel owned by effect", None),
    (0x0A1A, 0x0A9A, 0x0A1A, "read byte from stream (read_byte) and tables", None),
    (0x0A9B, 0x0AB8, 0x0A9B, "FM effects player outer tick", None),
    (0x0AB9, 0x0AFD, 0x0AB9, "FM effect slot tick", None),
    (0x0AFE, 0x0B48, 0x0AFE, "FM effect pitch/level slide", None),
    (0x0B49, 0x0B9E, 0x0B49, "FM effect stream events", None),
    (0x0B9F, 0x0BFB, 0x0B9F, "FM effect key on", None),
    (0x0BFC, 0x0E61, 0x0BFC, "FM effect operator levels and tables", None),
    (0x0E62, 0x0EBB, 0x0E75, "FM effect opcode dispatch", None),
    (0x0EBC, 0x0F5D, 0x0EBC, "FM effect patch load", None),
    (0x0F5E, 0x0F9F, 0x0F7C, "FM effect opcodes 0x02..0x05", None),
    (0x0FA0, 0x0FFD, 0x0FA0, "FM effect stream byte reader (effect_byte)", None),
]

AUDIO2_ROUTINES: list[tuple[int, int, int, str, str | None]] = [
    (0x0000, 0x002B, 0x0000, "power-on initialization and idle loop", None),
    (0x0038, 0x0045, 0x0038, "IM 1 tick interrupt", None),
    (0x0046, 0x008C, 0x0046, "idle poll for new latch command", None),
    (0x008D, 0x012A, 0x008D, "start sample command on channel 0/1", None),
    (0x012B, 0x01B1, 0x012B, "play active ADPCM nibbles", None),
]

ROUTINES_BY_CPU = {
    "maincpu": MAINCPU_ROUTINES,
    "audiocpu": AUDIOCPU_ROUTINES,
    "audio2": AUDIO2_ROUTINES,
}

# Spin-loop instruction PC sets for the zero-heuristic MAME probe optimization:
# A tap is only removed after 100% of the instructions in its spin loop have been recorded.
SPIN_M_PCS = [
    0x3B19A, 0x3B19E, 0x3B1A4, 0x3B1A6, 0x3B1AC, 0x3B1B0, 0x3B1B6, 0x3B1BA,
    0x3B1C0, 0x3B1C4, 0x3B1CA, 0x3B1CE, 0x3B1D0, 0x3B1D2,
]
SPIN_A_PCS = [0x0009, 0x000A]
SPIN_2_28_PCS = [0x0028, 0x002A]
SPIN_2_38_PCS = [
    0x0038, 0x0039, 0x003B, 0x003D, 0x0041, 0x0043, 0x0044, 0x0045,
    0x0046, 0x0047, 0x004A,
]
SPIN_2_65_PCS = [
    0x0065, 0x0066, 0x0068, 0x0069, 0x006B, 0x006F, 0x0071, 0x0074,
    0x0075, 0x0076, 0x0077, 0x0079, 0x007B, 0x007C, 0x007F, 0x0080,
    0x0082, 0x0083, 0x0084, 0x0085, 0x0087, 0x0088, 0x0089, 0x008A,
    0x008B, 0x008C,
]


def _lua_pc_list(pcs: list[int]) -> str:
    return "{" + ", ".join(f"0x{p:x}" for p in pcs) + "}"


def build_lua_probe(s: Scenario, out_tsv_path: pathlib.Path) -> str:
    frames_target = int(s.duration_frames)
    events = compute_input_events(s)
    lua_events = []
    for ev in events:
        lua_events.append(
            f'{{frame={int(ev["frame"])}, port={json.dumps(ev["port"])}, '
            f'mask=0x{int(ev["mask"]):04x}, state={int(ev["state"])}}}'
        )
    events_table_str = "{\n  " + ",\n  ".join(lua_events) + "\n}"

    lua_dip_changes = []
    previous = {"DSW1": s.dsw1, "DSW2": s.dsw2}
    for change in s.dip_changes:
        fields = []
        for spec in DIP_FIELDS:
            old_word = previous[spec.port]
            new_word = change.dsw1 if spec.port == "DSW1" else change.dsw2
            if (old_word & spec.mask) != (new_word & spec.mask):
                fields.append(
                    f'{{port={json.dumps(spec.port)}, mask=0x{spec.mask:04x}, '
                    f'value=0x{new_word & spec.mask:04x}}}'
                )
        lua_dip_changes.append(
            f'{{frame={change.frame}, changes={{' + ", ".join(fields) + "}}"
        )
        previous = {"DSW1": change.dsw1, "DSW2": change.dsw2}
    dip_changes_table_str = "{\n  " + ",\n  ".join(lua_dip_changes) + "\n}"

    return f"""
local frames_target = {frames_target}
local out_path = {json.dumps(str(out_tsv_path.resolve()))}
local input_events = {events_table_str}
local dip_changes = {dip_changes_table_str}

local frame = 0
local main_seen = {{}}
local audio_seen = {{}}
local audio2_seen = {{}}

local spin_m_pcs = {_lua_pc_list(SPIN_M_PCS)}
local spin_a_pcs = {_lua_pc_list(SPIN_A_PCS)}
local spin_2_28_pcs = {_lua_pc_list(SPIN_2_28_PCS)}
local spin_2_38_pcs = {_lua_pc_list(SPIN_2_38_PCS)}
local spin_2_65_pcs = {_lua_pc_list(SPIN_2_65_PCS)}

local function all_seen(seen, list)
  for i = 1, #list do
    if seen[list[i]] == nil then return false end
  end
  return true
end

local maincpu = manager.machine.devices[":maincpu"]
local audiocpu = manager.machine.devices[":audiocpu"]
local audio2 = manager.machine.devices[":audio2"]

local m_curpc = maincpu.state["CURPC"]
local a_curpc = audiocpu.state["CURPC"]
local a2_curpc = audio2.state["CURPC"]

local m_mem = maincpu.spaces["program"]
local a_mem = audiocpu.spaces["program"]
local a2_mem = audio2.spaces["program"]

_G.KEEP_TAPS = {{}}

local tap_m_spin = nil
-- A program-space read may be an opcode operand or data, not the current
-- instruction's opcode. Record CURPC itself; requiring offset == CURPC loses
-- nearly all 68000 instructions. The range check excludes the reset-time 0.
local function install_m_taps()
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = m_mem:install_read_tap(0x3b000, 0x3b199, "cov_m1", function(offset, data, mask)
    local pc = m_curpc.value
    if pc >= 0x3b000 and pc <= 0x492d5 and main_seen[pc] == nil then main_seen[pc] = frame end
  end)
  tap_m_spin = m_mem:install_read_tap(0x3b19a, 0x3b1d5, "cov_m_spin", function(offset, data, mask)
    local pc = m_curpc.value
    if pc >= 0x3b000 and pc <= 0x492d5 and main_seen[pc] == nil then
      main_seen[pc] = frame
      if all_seen(main_seen, spin_m_pcs) and tap_m_spin ~= nil then
        tap_m_spin:remove()
        tap_m_spin = nil
      end
    end
  end)
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = tap_m_spin
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = m_mem:install_read_tap(0x3b1d6, 0x492d5, "cov_m2", function(offset, data, mask)
    local pc = m_curpc.value
    if pc >= 0x3b000 and pc <= 0x492d5 and main_seen[pc] == nil then main_seen[pc] = frame end
  end)
end

local tap_a_spin = nil
local function install_a_taps()
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = a_mem:install_read_tap(0x0000, 0x0008, "cov_a1", function(offset, data, mask)
    local pc = a_curpc.value
    if offset == pc and audio_seen[pc] == nil then audio_seen[pc] = frame end
  end)
  tap_a_spin = a_mem:install_read_tap(0x0009, 0x000b, "cov_a_spin", function(offset, data, mask)
    local pc = a_curpc.value
    if offset == pc and audio_seen[pc] == nil then
      audio_seen[pc] = frame
      if all_seen(audio_seen, spin_a_pcs) and tap_a_spin ~= nil then
        tap_a_spin:remove()
        tap_a_spin = nil
      end
    end
  end)
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = tap_a_spin
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = a_mem:install_read_tap(0x000c, 0x0ffd, "cov_a2", function(offset, data, mask)
    local pc = a_curpc.value
    if offset == pc and audio_seen[pc] == nil then audio_seen[pc] = frame end
  end)
end

local tap_2_28 = nil
local tap_2_38 = nil
local tap_2_65 = nil
local function install_2_taps()
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = a2_mem:install_read_tap(0x0000, 0x0027, "cov_2_1", function(offset, data, mask)
    local pc = a2_curpc.value
    if offset == pc and audio2_seen[pc] == nil then audio2_seen[pc] = frame end
  end)
  tap_2_28 = a2_mem:install_read_tap(0x0028, 0x002b, "cov_2_28", function(offset, data, mask)
    local pc = a2_curpc.value
    if offset == pc and audio2_seen[pc] == nil then
      audio2_seen[pc] = frame
      if all_seen(audio2_seen, spin_2_28_pcs) and tap_2_28 ~= nil then
        tap_2_28:remove()
        tap_2_28 = nil
      end
    end
  end)
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = tap_2_28
  tap_2_38 = a2_mem:install_read_tap(0x0038, 0x004b, "cov_2_38", function(offset, data, mask)
    local pc = a2_curpc.value
    if offset == pc and audio2_seen[pc] == nil then
      audio2_seen[pc] = frame
      if all_seen(audio2_seen, spin_2_38_pcs) and tap_2_38 ~= nil then
        tap_2_38:remove()
        tap_2_38 = nil
      end
    end
  end)
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = tap_2_38
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = a2_mem:install_read_tap(0x004c, 0x0064, "cov_2_4c", function(offset, data, mask)
    local pc = a2_curpc.value
    if offset == pc and audio2_seen[pc] == nil then audio2_seen[pc] = frame end
  end)
  tap_2_65 = a2_mem:install_read_tap(0x0065, 0x008c, "cov_2_65", function(offset, data, mask)
    local pc = a2_curpc.value
    if offset == pc and audio2_seen[pc] == nil then
      audio2_seen[pc] = frame
      if all_seen(audio2_seen, spin_2_65_pcs) and tap_2_65 ~= nil then
        tap_2_65:remove()
        tap_2_65 = nil
      end
    end
  end)
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = tap_2_65
  _G.KEEP_TAPS[#_G.KEEP_TAPS + 1] = a2_mem:install_read_tap(0x008d, 0x01b1, "cov_2_8d", function(offset, data, mask)
    local pc = a2_curpc.value
    if offset == pc and audio2_seen[pc] == nil then audio2_seen[pc] = frame end
  end)
end

install_m_taps()
install_a_taps()
install_2_taps()

local ioport = manager.machine.ioport
local ports = {{}}
for k, v in pairs(ioport.ports) do ports[k] = v end

local function find_field(port_tag, mask)
  local p = ports[":" .. port_tag]
  if not p then return nil end
  for _, f in pairs(p.fields) do
    if f.mask == mask then return f end
  end
  return nil
end

local next_dip_change = 1
local function apply_dip_changes(f)
  while next_dip_change <= #dip_changes and dip_changes[next_dip_change].frame <= f do
    for _, change in ipairs(dip_changes[next_dip_change].changes) do
      local field = assert(find_field(change.port, change.mask))
      field.user_value = change.value
    end
    next_dip_change = next_dip_change + 1
  end
end

local event_idx = 1
local function apply_events_for_frame(f)
  while event_idx <= #input_events and input_events[event_idx].frame <= f do
    local ev = input_events[event_idx]
    local fld = find_field(ev.port, ev.mask)
    if fld then
      if ev.state == 1 then
        fld:set_value(1)
      else
        fld:clear_value()
      end
    end
    event_idx = event_idx + 1
  end
end

emu.register_frame_done(function()
  local _keep = #_G.KEEP_TAPS
  if frame >= frames_target then
    local f = assert(io.open(out_path, "wb"))
    f:write("cpu\\tpc\\tfirst_frame\\n")
    local function dump(name, tbl, fmt)
      local pcs = {{}}
      for k, _ in pairs(tbl) do pcs[#pcs + 1] = k end
      table.sort(pcs)
      for _, pc in ipairs(pcs) do
        f:write(string.format("%s\\t" .. fmt .. "\\t%d\\n", name, pc, tbl[pc]))
      end
    end
    dump("maincpu", main_seen, "0x%05x")
    dump("audiocpu", audio_seen, "0x%04x")
    dump("audio2", audio2_seen, "0x%04x")
    f:close()
    manager.machine:exit()
    return
  end
  apply_dip_changes(frame)
  apply_events_for_frame(frame)
  frame = frame + 1
end)
"""


def _run_single_probe(job: tuple[pathlib.Path, str]) -> tuple[str, str]:
    COVERAGE_DIR.mkdir(parents=True, exist_ok=True)
    scen_path, run_tag = job
    s = load_scenario(scen_path)
    scen_id = s.scenario_id
    out_tsv = COVERAGE_DIR / f"{scen_id}-{run_tag}.tsv"
    if out_tsv.exists():
        out_tsv.unlink()
    lua_path = COVERAGE_DIR / f"_tmp_{scen_id}_{run_tag}.lua"
    cfg_dir = COVERAGE_DIR / f"_cfg_{scen_id}_{run_tag}"
    cfg_dir.mkdir(parents=True, exist_ok=True)
    opts = list(BASELINE_OPTIONS)
    if s.dipswitches_explicit:
        opts = [o for o in opts if o != "-noreadconfig"]
        (cfg_dir / f"{s.machine_set}.cfg").write_text(generate_cfg(s), encoding="utf-8")
    lua_path.write_text(build_lua_probe(s, out_tsv), encoding="utf-8")
    try:
        cmd = [
            str(MAME_EXE.resolve()),
            s.machine_set,
            "-rompath",
            str(ROMS_DIR.resolve()),
            *opts,
            "-cfg_directory",
            str(cfg_dir.resolve()),
            "-autoboot_delay",
            "0",
            "-autoboot_script",
            str(lua_path.resolve()),
        ]
        res = subprocess.run(cmd, cwd=str(MAME_DIR), capture_output=True, text=True)
        if res.returncode != 0 or not out_tsv.exists():
            raise RuntimeError(
                f"MAME probe failed for {scen_id} ({run_tag}): rc={res.returncode}\n{res.stderr[-400:]}"
            )
    finally:
        if lua_path.exists():
            lua_path.unlink()
        if cfg_dir.exists():
            for child in cfg_dir.rglob("*"):
                if child.is_file():
                    child.unlink()
            for child in sorted(cfg_dir.rglob("*"), reverse=True):
                if child.is_dir():
                    child.rmdir()
            cfg_dir.rmdir()
    return scen_id, run_tag


def ensure_probe_runs(force_rerun: bool = False, max_workers: int = 8) -> list[dict[str, Any]]:
    """Ensure run-a and run-b coverage TSVs exist for all scenarios and verify run-a == run-b."""
    scen_files = sorted(SCENARIOS_DIR.glob("*.json"))
    jobs: list[tuple[pathlib.Path, str]] = []
    for sp in scen_files:
        s = load_scenario(sp)
        scen_id = s.scenario_id
        for tag in ("run-a", "run-b"):
            out_tsv = COVERAGE_DIR / f"{scen_id}-{tag}.tsv"
            if force_rerun or not out_tsv.exists():
                jobs.append((sp, tag))

    if jobs:
        COVERAGE_DIR.mkdir(parents=True, exist_ok=True)
        with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as ex:
            futures = [ex.submit(_run_single_probe, job) for job in jobs]
            for fut in concurrent.futures.as_completed(futures):
                fut.result()

    scenario_summaries: list[dict[str, Any]] = []
    for sp in scen_files:
        s = load_scenario(sp)
        scen_id = s.scenario_id
        path_a = COVERAGE_DIR / f"{scen_id}-run-a.tsv"
        path_b = COVERAGE_DIR / f"{scen_id}-run-b.tsv"
        if not path_a.exists() or not path_b.exists():
            raise FileNotFoundError(f"Missing coverage run for {scen_id}: {path_a} / {path_b}")
        bytes_a = path_a.read_bytes()
        bytes_b = path_b.read_bytes()
        sha_a = hashlib.sha256(bytes_a).hexdigest()
        sha_b = hashlib.sha256(bytes_b).hexdigest()
        identical = bytes_a == bytes_b and sha_a == sha_b
        if not identical:
            raise AssertionError(f"Coverage runs differ for scenario {scen_id}")
        if path_a.stat().st_ino != path_b.stat().st_ino:
            path_b.unlink()
            os.link(path_a, path_b)
        counts = {"maincpu": 0, "audiocpu": 0, "audio2": 0}
        for line in bytes_a.decode("utf-8").splitlines()[1:]:
            cpu = line.split("\t", 1)[0]
            counts[cpu] += 1
        scenario_summaries.append(
            {
                "id": scen_id,
                "file": sp.name,
                "frames": int(s.duration_frames),
                "identical": identical,
                "sha256": sha_a,
                "maincpu_pcs": counts["maincpu"],
                "audiocpu_pcs": counts["audiocpu"],
                "audio2_pcs": counts["audio2"],
            }
        )
    return scenario_summaries


def _find_code_addrs(text: str, cpu: str) -> list[int]:
    if cpu == "maincpu":
        raw = [int(x, 16) for x in re.findall(r"0x([34][0-9a-fA-F]{4})\b", text)]
        return [a for a in raw if 0x3B000 <= a < 0x492D6]
    if cpu == "audiocpu":
        raw = [int(x, 16) for x in re.findall(r"0x(0[0-9a-fA-F]{3})\b", text)]
        for rm in re.findall(r"rst\s+0x([0-9a-fA-F]{2})\b", text):
            raw.append(int(rm, 16))
        return [a for a in raw if 0x0000 <= a <= 0x0FFD]
    raw = [int(x, 16) for x in re.findall(r"0x(0[01][0-9a-fA-F]{2})\b", text)]
    return [a for a in raw if 0x0000 <= a <= 0x01B1]


# Metadata for dynamic and static note() / note_unrecovered() sites across the codebase.
NOTE_SITE_DISPOSITIONS: dict[tuple[str, str, str], tuple[list[int], str]] = {
    ("fighter.rs", "park_from", "0x44506"): (
        [],
        "Unreachable under MAME: defensive loop-bound guard (`d0 > 0x600`); caller `0x437fc` (`stage_reset`) passes `d0 = 0x100` and `0x44506` steps `d0` by `0x40` up to `0x600`.",
    ),
    ("fighter.rs", "animate", "routine"): (
        [],
        "Deferred (open decision 8 in #1): dynamic fighter stream-call dispatch (`0xfe` control byte at `0x42c20`); all scenario-reached targets (`0x4141a`, `0x414f0`, `0x41558` in F29 #69 / F58 #130; `0x4596e`, `0x45a34` in F27 #67; `0x4558a` in F62 #134; `0x45ace` in F63 #135) are translated, and static translation of unreached move-stream pointers is deferred under #1 open decision 8.",
    ),
    ("fighter.rs", "judge", "f"): (
        [],
        "Deferred (open decision 8 in #1): dynamic hit-function dispatch (`+0x28` at `0x43f9e`); all scenario-reached entries across Ryu, Ken, and all 10 CPU opponents (F27 #67, F28 #68, F44 #84, F53 #125, F57 #129, F59 #131, F60 #132, F62 #134, F63 #135) are translated, and unreached CPU hit-function entries in `0x46472..0x480d0` are deferred under #1 open decision 8.",
    ),
    ("fighter.rs", "effect_handler", "h"): (
        [],
        "Deferred (open decision 8 in #1): dynamic effect-handler dispatch (`+0x14` / `+0x18`); all scenario-reached handlers (`0x433ba` in F9 #49, `0x43348` in F27 #67, `0x434e4..0x4360a` in F52 #124 / F61 #133, `0x43652` in F63 #135) are translated, and unreached effect-template handler entries (such as `0x43396`) are deferred under #1 open decision 8.",
    ),
    ("fighter.rs", "input_handler", "0x4030c"): (
        [0x4030C],
        "Attract demo scripted input reader (`0x4030c`); reached in `sf-attract-01` and `sf-attract-demo-01` (F48, F55).",
    ),
    ("tasks.rs", "enter_new_task", "entry"): (
        [],
        "Unreachable under MAME: dynamic new-task entry fallback (`0x3b1e8`); all 14 task entries in the ROM — normal-play tasks (`0x3c0c4`, `0x3ccae`, `0x3dec2`, `0x3f7b0`, `0x3f374`, `0x4813e`), service input/DIP/sound test (`0x483f8`, F73 #145), service menu (`0x48644`, F74 #146), and service items 1..7 (`0x487f8`, `0x488f8`, `0x48858`, `0x48990`, `0x48b6a`, `0x48da4`, `0x490f8`, F74 #146, F75 #147, F76 #148) — are translated.",
    ),
    ("tasks.rs", "run", "0x3b1d6"): (
        [],
        "Unreachable under MAME: scheduler invariant guard when an active RAM slot (`state == 4`) has no corresponding Rust `Task` state.",
    ),
    ("tasks.rs", "run", "0x3b1f0"): (
        [],
        "Unreachable under MAME: scheduler invariant guard when a state-1 (new) RAM slot does not hold `Task::New`.",
    ),
    ("tasks.rs", "run", "*entry"): (
        [],
        "Unreachable under MAME: Rust enum-exhaustiveness guard on `Task::New` inside `Task::run`; `Scheduler::run` always resolves `Task::New` via `enter_new_task` (`0x3b1e8`) before invoking `Task::run`.",
    ),
    ("tasks.rs", "service_input", "0x3b602"): (
        [0x3B602],
        "Unreachable under MAME: service palette-upload repeat (`0x3b602`) when `SYSTEM` bit 6 is low; MAME hardwires `SYSTEM` bit 6 high via `IPT_UNKNOWN` (`sf-mame0289.cpp:561`).",
    ),
    ("tasks.rs", "boot", "0x3b602"): (
        [0x3B602],
        "Unreachable under MAME: boot palette-upload repeat (`0x3b602`) when `SYSTEM` bit 6 is low; MAME hardwires `SYSTEM` bit 6 high via `IPT_UNKNOWN` (`sf-mame0289.cpp:561`).",
    ),
    ("tasks.rs", "boot", "0x3c210"): (
        [0x3C210],
        "Boot warning screen (`0x3c210..0x3c290`); reached at frame 8 in power-on scenarios (F15).",
    ),
    ("tasks.rs", "flow", "0x3e5c6"): (
        [0x3E5C6],
        "Attract demo stage-start branch (`0x3e5c6`); reached in `sf-attract-01` and `sf-attract-demo-01` (F48, F55).",
    ),
    ("tasks.rs", "flow", "target"): (
        [],
        "Unreachable under MAME: country-end per-stage dispatch fallback (`0x3ef96`, table `0x3efb0`); all five ROM table entries — stage 0 (`0x40d30`, F29 #69), stage 1 (`0x41740`, F52 #124), stage 2 (`0x41a02`, F58 #130), stage 3 (`0x41fe6`, F61 #133), and stage 4 (`0x44a40`, F64 #136) — are translated.",
    ),
    ("tasks.rs", "bonus_japan_meter_frame", "cont"): (
        [],
        "Unreachable under MAME: dynamic power-meter continuation fallback (`0x40f1e` / `0x41cf0`); all ten ROM continuation states (`0x40f20..0x40ffe` and `0x41bf2..0x41cd0`) are translated (F29 #69, F58 #130).",
    ),
    ("tasks.rs", "attract_ranking_start", "0x42750"): (
        [0x42750],
        "Marked ranking entry path (`0x42750`) when a ranking entry has `+4 == 0xff`; translated and covered by F50 (#122).",
    ),
    ("tasks.rs", "attract_ranking_wait", "0x3ce54"): (
        [0x3CE54],
        "Attract demo setup (`0x3ce54..0x3ceba`) after the ranking screen exits; reached in `sf-attract-01`, `sf-attract-demo-01`, and `sf-music-03` (F48, F55).",
    ),
    ("tasks.rs", "next_opponent", "0x3e6fe"): (
        [0x3E6FE],
        "Attract demo `next_opponent` branch (`0x3e6fe`); reached in `sf-attract-01` and `sf-attract-demo-01` (F48, F55).",
    ),
    ("fighter.rs", "run_effect", "target"): (
        [],
        "Unreachable under MAME: dynamic effect stream-call fallback (`0xff` control byte at `0x42dd2`); all effect stream-call targets in the ROM (`0x427e2`, `0x42920` in F42 #82; `0x432c4`, `0x432e0`, `0x43306`, `0x4331a` in F43 #83) are translated.",
    ),
    ("tasks.rs", "ranking_check", "0x42486"): (
        [0x42486],
        "Name-entry setup (`0x42486`) after a qualifying score records its ranking-entry pointer at `0xff8e66` (F26, F42, F50).",
    ),
    ("tasks.rs", "name_entry_frame", "0x425c4"): (
        [0x425C4],
        "Name-entry completion, ranking insertion and return to game (`0x425c4`); reached in `sf-name-entry-01` and `sf-name-entry-02` (F50).",
    ),
    ("tasks.rs", "game_over_reset", "0x3ccae"): (
        [0x3CCAE],
        "Attract return (`0x3ccae`) when no credits remain after game over; reached in `sf-doubleko-01`, `sf-doubleko-02`, `sf-match-end-01`, and `sf-music-03` (F25).",
    ),
    ("tasks.rs", "record_win", "0x3ce8c"): (
        [0x3CE8C],
        "Attract demo round-end cleanup (`0x3ce8c`); translated and covered by `sf-attract-demo-01` (F55 #127).",
    ),
    ("tasks.rs", "time_over_judge", "0x3ce8c"): (
        [0x3CE8C],
        "Attract demo time-over round-end cleanup (`0x3ce8c`); translated by F55 (#127).",
    ),
    ("tasks.rs", "double_ko_hold", "0x3ce8c"): (
        [0x3CE8C],
        "Attract demo double-KO round-end cleanup (`0x3ce8c`); translated by F55 (#127).",
    ),
    ("tasks.rs", "screen", "handler"): (
        [],
        "Unreachable under MAME: dynamic screen-event handler fallback (`0x3f7b0`, table `0x3f7d8`); all ten non-null ROM event handlers (`2..=11`: `0x3f808`, `0x3f8e4`, `0x3f9ec`, `0x3fba2`, `0x3fc8a`, `0x3fcd4`, `0x3fef2`, `0x3ff2c`, `0x3ff5a`, `0x4001c`) are translated (F2–F5, F8, F10, F16, F18, F25, F43), and entry 1 (`0x00000`) is never requested.",
    ),
    ("tasks.rs", "watch", "other"): (
        [],
        "Unreachable under MAME: dynamic watch-task continuation fallback (`0xff8f20` / `0xff8f24`); all five attract states (`0x3f3a4..0x3f40e`) and all four game states (`0x3f41e`, `0x3f454`, `0x3f47e`, `0x3f4c6`) are translated (F34 #74, F41 #81).",
    ),
    ("tasks.rs", "vblank_prefix", "0x3b0f0"): (
        [],
        "Unreachable under MAME: vblank skip when `SYSTEM` bit 7 is high (`0x3b0f0` branch taken to `0x3b194`); MAME hardwires `SYSTEM` bit 7 low via `IPT_UNKNOWN` (`sf-mame0289.cpp:562`).",
    ),
    ("tasks.rs", "vblank_prefix", "0x3b378"): (
        [0x3B378],
        "Unreachable under MAME: vblank palette-upload repeat (`0x3b378`) when `SYSTEM` bit 6 is low; MAME hardwires `SYSTEM` bit 6 high via `IPT_UNKNOWN` (`sf-mame0289.cpp:561`).",
    ),
    ("tasks.rs", "vblank", "0x3b100"): (
        [],
        "Unreachable under MAME: guard for bounded callers that call `vblank` directly with Freeze On; `FightMachine` (`CpuMachine`) translates the Freeze spin via `vblank_prefix` / `vblank_suffix`, verified against `sf-freeze-01` (F21 #61).",
    ),
    ("music.rs", "raw_tick", "0x000a"): (
        [],
        "Unreachable under MAME: extension-only raw-score catch-up guard after 256 events without a positive span; raw channels are initialized only by opcode `0x0a` in the extended image above the original ROM, which MAME does not load (SP15 #267).",
    ),
    ("music.rs", "raw_opcode", "0x000a"): (
        [],
        "Unreachable under MAME: extension-only raw-score sub-opcode guard; `opcode` calls `raw_opcode` only with an extended image and stream pointer in its extension range. Original ROM opcode `0x0a` keeps its separate legacy guard (SP15 #267).",
    ),
    ("music.rs", "special", "0x02a5"): (
        [],
        "Unreachable under MAME: special command index `>= 5` past the 5-entry table at `0x02a7`; `0x01e2` only routes commands `0x30..0x3f` here, which map to indices `0..4`.",
    ),
    ("music.rs", "effect_opcode", "0x0e62"): (
        [],
        "Unreachable under MAME: FM effect stream opcode `> 5` at `0x0e62`; no ROM FM effect stream contains an opcode byte in `0x06..=0x1f`.",
    ),
    ("music.rs", "div16", "0x0159"): (
        [],
        "Unreachable under MAME: divide-by-zero early return (`DE == 0`) at `0x0159`; all ROM tempo divisors passed to `0x0155` are non-zero.",
    ),
    ("music.rs", "opcode", "0x0000"): (
        [],
        "Unreachable under MAME: music stream opcode `0x0a` (null pointer `0x0000` in table `0x0865`); no ROM music stream contains opcode `0x0a`.",
    ),
    ("music.rs", "opcode", "0x09e7"): (
        [],
        "Unreachable under MAME: second-level nested music sub-phrase call (`0x0f` at `0x09e7` when return pointer `ix+0x12..0x13` is already non-zero); ROM tunes only nest one level deep (F23 #63).",
    ),
}


def parse_rust_sources(
    rust_root: pathlib.Path = REPO_ROOT,
) -> tuple[dict[str, list[dict[str, Any]]], list[dict[str, Any]]]:
    """Parse translated functions and note()/note_unrecovered() sites from the Rust sources."""
    fns_by_cpu: dict[str, list[dict[str, Any]]] = {
        "maincpu": [],
        "audiocpu": [],
        "audio2": [],
    }
    note_sites: list[dict[str, Any]] = []

    for rel_path, cpu in RUST_SOURCES:
        full_path = rust_root / rel_path
        lines = full_path.read_text(encoding="utf-8").splitlines()
        short_file = pathlib.Path(rel_path).name

        i = 0
        while i < len(lines):
            if lines[i].strip().startswith("#[cfg(test)]"):
                break
            m = re.match(r"^(\s*)(?:pub(?:\(crate\))?\s+)?fn\s+([a-zA-Z0-9_]+)", lines[i])
            if m and len(m.group(1)) <= 4 and not lines[i].strip().endswith(";"):
                indent = len(m.group(1))
                fn_name = m.group(2)
                start_doc = i
                while start_doc > 0 and (
                    lines[start_doc - 1].strip().startswith("///")
                    or lines[start_doc - 1].strip().startswith("//")
                    or lines[start_doc - 1].strip().startswith("#[")
                ):
                    start_doc -= 1
                if "{" in lines[i]:
                    end_fn = i
                    if not re.search(r"\{\s*\}\s*$", lines[i]):
                        end_fn = i + 1
                        while end_fn < len(lines):
                            if re.match(r"^" + (" " * indent) + r"\}\s*$", lines[end_fn]):
                                break
                            end_fn += 1
                else:
                    end_fn = i + 1
                    while end_fn < len(lines) and not re.match(
                        r"^" + (" " * indent) + r"\}\s*$", lines[end_fn]
                    ):
                        end_fn += 1

                doc_text = "\n".join(lines[start_doc:i])
                doc_clean = re.sub(r"called\s+from\s+[^.;)]*", "", doc_text, flags=re.IGNORECASE)
                doc_clean = re.sub(
                    r"four\s+callers\s+are\s+[^.;)]*", "", doc_clean, flags=re.IGNORECASE
                )

                fn_notes: set[int] = set()
                body_lines: list[str] = []
                for k in range(i, min(end_fn + 1, len(lines))):
                    ln = lines[k]
                    for nm in re.finditer(r"\b(note(?:_unrecovered)?)\(([^)]+)\)", ln):
                        call_kind = nm.group(1)
                        arg = nm.group(2).strip()
                        if fn_name in ("note", "note_unrecovered") and "addr" in arg:
                            continue
                        m_hex = re.match(r"^0x([0-9a-fA-F]+)$", arg)
                        if m_hex:
                            fn_notes.add(int(m_hex.group(1), 16))
                        note_sites.append(
                            {
                                "file": rel_path,
                                "short_file": short_file,
                                "line": k + 1,
                                "fn": fn_name,
                                "cpu": cpu,
                                "call": f"{call_kind}({arg})",
                                "arg": arg,
                            }
                        )
                    if "note(" in ln or "note_unrecovered(" in ln:
                        continue
                    if (
                        k + 1 < len(lines)
                        and ("note(" in lines[k + 1] or "note_unrecovered(" in lines[k + 1])
                        and ln.strip().startswith("//")
                    ):
                        continue
                    body_lines.append(ln)

                doc_addrs = _find_code_addrs(doc_clean, cpu)
                primary = doc_addrs[0] if doc_addrs else None
                all_addrs = set(doc_addrs + _find_code_addrs("\n".join(body_lines), cpu))
                cited_addrs = sorted(a for a in all_addrs if a not in fn_notes or a == primary)

                if cited_addrs:
                    fns_by_cpu[cpu].append(
                        {
                            "file": rel_path,
                            "short_file": short_file,
                            "line": i + 1,
                            "name": fn_name,
                            "primary": primary if primary is not None else cited_addrs[0],
                            "doc_addrs": doc_addrs,
                            "cited_addrs": cited_addrs,
                        }
                    )
                i = end_fn + 1
            else:
                i += 1

    return fns_by_cpu, note_sites


def verify_gap_doc_mentions(
    repo_root: pathlib.Path, start_addr: int, end_addr: int, cite_addr: int, gap_doc: str | None
) -> bool:
    """Verify that at least one doc path in gap_doc exists and mentions the routine or its range."""
    if not gap_doc:
        return False
    doc_paths = re.findall(r"(docs/[a-zA-Z0-9_/\-]+\.md)", gap_doc)
    if not doc_paths:
        return False
    tokens = {f"0x{cite_addr:x}", f"0x{start_addr:x}", f"0x{end_addr:x}"}
    for dp in doc_paths:
        p = repo_root / dp
        if p.exists():
            txt = p.read_text(encoding="utf-8").lower()
            if any(tok in txt for tok in tokens):
                return True
    return False


def load_coverage_data() -> tuple[
    dict[str, dict[int, set[str]]],
    dict[str, dict[int, tuple[int, str]]],
]:
    exec_pcs: dict[str, dict[int, set[str]]] = {
        "maincpu": collections.defaultdict(set),
        "audiocpu": collections.defaultdict(set),
        "audio2": collections.defaultdict(set),
    }
    first_frame: dict[str, dict[int, tuple[int, str]]] = {
        "maincpu": {},
        "audiocpu": {},
        "audio2": {},
    }
    for p in sorted(COVERAGE_DIR.glob("*-run-a.tsv")):
        sc = p.name[: -len("-run-a.tsv")]
        for line in p.read_text(encoding="utf-8").splitlines()[1:]:
            cpu, pc_s, frame_s = line.split("\t")
            pc = int(pc_s, 16)
            fr = int(frame_s)
            exec_pcs[cpu][pc].add(sc)
            if pc not in first_frame[cpu] or (fr, sc) < first_frame[cpu][pc]:
                first_frame[cpu][pc] = (fr, sc)
    return exec_pcs, first_frame


def _fmt_addr(cpu: str, addr: int) -> str:
    return f"0x{addr:05x}" if cpu == "maincpu" else f"0x{addr:04x}"


def build_ledger(
    rust_root: pathlib.Path = REPO_ROOT,
    scenario_summaries: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    if scenario_summaries is None:
        scenario_summaries = ensure_probe_runs(force_rerun=False)
    exec_pcs, first_frame = load_coverage_data()
    fns_by_cpu, raw_notes = parse_rust_sources(rust_root)

    # Index which Rust functions cite which addresses (excluding fixture-state loaders from_fixture/positions)
    addr_to_fns: dict[str, dict[int, list[dict[str, Any]]]] = {
        "maincpu": collections.defaultdict(list),
        "audiocpu": collections.defaultdict(list),
        "audio2": collections.defaultdict(list),
    }
    for cpu, fns in fns_by_cpu.items():
        for fn in fns:
            if fn["name"] in ("from_fixture", "positions"):
                continue
            for a in fn["cited_addrs"]:
                addr_to_fns[cpu][a].append(fn)

    # Map each routine interval and each executed PC
    routine_entries: dict[str, list[dict[str, Any]]] = {
        "maincpu": [],
        "audiocpu": [],
        "audio2": [],
    }
    address_ledger: dict[str, dict[str, Any]] = {
        "maincpu": {},
        "audiocpu": {},
        "audio2": {},
    }
    documented_gaps: list[dict[str, Any]] = []
    undocumented_untranslated: list[dict[str, Any]] = []

    # Track scenarios and executed PCs per Rust function
    fn_exec_pcs: dict[tuple[str, int, str], set[int]] = collections.defaultdict(set)
    fn_scenarios: dict[tuple[str, int, str], set[str]] = collections.defaultdict(set)

    for cpu in ("maincpu", "audiocpu", "audio2"):
        for a1, a2, cite_addr, desc, gap_doc in ROUTINES_BY_CPU[cpu]:
            pcs_in_range = sorted(pc for pc in exec_pcs[cpu] if a1 <= pc <= a2)
            scenarios_in_range: set[str] = set()
            for pc in pcs_in_range:
                scenarios_in_range |= exec_pcs[cpu][pc]

            citing_fns = addr_to_fns[cpu].get(cite_addr, [])
            # Prefer functions whose primary citation is cite_addr or whose cited_addrs fall in a1..=a2
            primary_fns = [
                f for f in citing_fns if f["primary"] == cite_addr or (a1 <= f["primary"] <= a2)
            ]
            chosen_fns = primary_fns if primary_fns else citing_fns
            translated = len(chosen_fns) > 0

            fn_labels = [f"{f['short_file']}:{f['line']} {f['name']}" for f in chosen_fns]
            for f in chosen_fns:
                key = (f["file"], f["line"], f["name"])
                fn_exec_pcs[key].update(pcs_in_range)
                fn_scenarios[key].update(scenarios_in_range)
            # Also credit any function whose primary address lies inside [a1, a2]
            for f in fns_by_cpu[cpu]:
                if a1 <= f["primary"] <= a2:
                    key = (f["file"], f["line"], f["name"])
                    fn_exec_pcs[key].update(pcs_in_range)
                    fn_scenarios[key].update(scenarios_in_range)

            first_hit = None
            if pcs_in_range:
                fr0, sc0 = min(first_frame[cpu][pc] for pc in pcs_in_range)
                first_hit = {"scenario": sc0, "frame": fr0}

            doc_verified = verify_gap_doc_mentions(REPO_ROOT, a1, a2, cite_addr, gap_doc)
            entry = {
                "cpu": cpu,
                "start": _fmt_addr(cpu, a1),
                "end": _fmt_addr(cpu, a2),
                "cite_addr": _fmt_addr(cpu, cite_addr),
                "description": desc,
                "translated": translated,
                "rust_routines": fn_labels,
                "executed_pc_count": len(pcs_in_range),
                "executed_pcs": [_fmt_addr(cpu, pc) for pc in pcs_in_range],
                "scenario_count": len(scenarios_in_range),
                "scenarios": sorted(scenarios_in_range),
                "first_hit": first_hit,
                "gap_doc": gap_doc,
                "gap_doc_verified": doc_verified,
            }
            routine_entries[cpu].append(entry)

            for pc in pcs_in_range:
                fr_pc, sc_pc = first_frame[cpu][pc]
                address_ledger[cpu][_fmt_addr(cpu, pc)] = {
                    "routine_range": f"{_fmt_addr(cpu, a1)}..{_fmt_addr(cpu, a2)}",
                    "cite_addr": _fmt_addr(cpu, cite_addr),
                    "translated": translated,
                    "rust_routines": fn_labels,
                    "scenario_count": len(exec_pcs[cpu][pc]),
                    "first_scenario": sc_pc,
                    "first_frame": fr_pc,
                    "gap_doc": gap_doc if not translated else None,
                }

            if not translated and pcs_in_range:
                if gap_doc and doc_verified:
                    documented_gaps.append(entry)
                else:
                    undocumented_untranslated.append(entry)

    # Build translated routine summary table
    translated_routines_table: list[dict[str, Any]] = []
    for cpu in ("maincpu", "audiocpu", "audio2"):
        for f in fns_by_cpu[cpu]:
            key = (f["file"], f["line"], f["name"])
            pcs = fn_exec_pcs.get(key, set())
            scs = fn_scenarios.get(key, set())
            # If a helper shares code inline with another routine, also check its cited_addrs directly
            if not pcs:
                for ca in f["cited_addrs"]:
                    for a1, a2, cite_addr, _, _ in ROUTINES_BY_CPU[cpu]:
                        if a1 <= ca <= a2:
                            for pc in exec_pcs[cpu]:
                                if a1 <= pc <= a2:
                                    pcs.add(pc)
                                    scs |= exec_pcs[cpu][pc]
            translated_routines_table.append(
                {
                    "cpu": cpu,
                    "file": f["file"],
                    "short_file": f["short_file"],
                    "line": f["line"],
                    "name": f["name"],
                    "primary_addr": _fmt_addr(cpu, f["primary"]),
                    "cited_addrs": [_fmt_addr(cpu, a) for a in f["cited_addrs"]],
                    "executed_pc_count": len(pcs),
                    "scenario_count": len(scs),
                    "scenarios": sorted(scs),
                }
            )

    # Evaluate each note() / note_unrecovered() site
    note_site_table: list[dict[str, Any]] = []
    for ns in raw_notes:
        key = (ns["short_file"], ns["fn"], ns["arg"])
        target_addrs, disp = NOTE_SITE_DISPOSITIONS.get(
            key, ([], "Dynamic or guard report site.")
        )
        cpu = ns["cpu"]
        reached_scs: set[str] = set()
        first_pair: tuple[int, str] | None = None
        reached_targets: list[str] = []
        for ta in target_addrs:
            if ta in exec_pcs[cpu]:
                reached_targets.append(_fmt_addr(cpu, ta))
                reached_scs |= exec_pcs[cpu][ta]
                fp = first_frame[cpu][ta]
                if first_pair is None or fp < first_pair:
                    first_pair = fp
        reached = len(reached_scs) > 0
        note_site_table.append(
            {
                "file": ns["file"],
                "short_file": ns["short_file"],
                "line": ns["line"],
                "fn": ns["fn"],
                "cpu": cpu,
                "call": ns["call"],
                "target_addrs": [_fmt_addr(cpu, ta) for ta in target_addrs],
                "reached": reached,
                "reached_targets": reached_targets,
                "scenario_count": len(reached_scs),
                "scenarios": sorted(reached_scs),
                "first_hit": (
                    {"scenario": first_pair[1], "frame": first_pair[0]}
                    if first_pair is not None
                    else None
                ),
                "disposition": disp,
            }
        )

    cpu_totals: dict[str, dict[str, int]] = {}
    normal_play_cpu_totals: dict[str, dict[str, int]] = {}
    service_mode_scenarios = {
        sc["id"]
        for sc in scenario_summaries
        if sc["id"] == "sf-dip-smoke-03"
        or sc["id"].startswith("sf-sound-test-")
        or sc["id"].startswith("sf-service-")
    }
    for cpu in ("maincpu", "audiocpu", "audio2"):
        total_exec = len(address_ledger[cpu])
        trans_exec = sum(1 for v in address_ledger[cpu].values() if v["translated"])
        untrans_exec = total_exec - trans_exec
        cpu_totals[cpu] = {
            "executed_pcs": total_exec,
            "translated_pcs": trans_exec,
            "untranslated_pcs": untrans_exec,
        }
        norm_pcs = [
            pc
            for pc, scs in exec_pcs[cpu].items()
            if (scs - service_mode_scenarios) and _fmt_addr(cpu, pc) in address_ledger[cpu]
        ]
        norm_trans = sum(
            1 for pc in norm_pcs if address_ledger[cpu][_fmt_addr(cpu, pc)]["translated"]
        )
        normal_play_cpu_totals[cpu] = {
            "executed_pcs": len(norm_pcs),
            "translated_pcs": norm_trans,
            "untranslated_pcs": len(norm_pcs) - norm_trans,
        }

    return {
        "scenario_count": len(scenario_summaries),
        "normal_play_scenario_count": len(scenario_summaries) - len(service_mode_scenarios),
        "service_mode_scenario_count": len(service_mode_scenarios),
        "scenarios": scenario_summaries,
        "cpu_totals": cpu_totals,
        "normal_play_cpu_totals": normal_play_cpu_totals,
        "routines_by_cpu": routine_entries,
        "translated_routines": translated_routines_table,
        "note_sites": note_site_table,
        "documented_gaps": documented_gaps,
        "undocumented_untranslated": undocumented_untranslated,
        "address_ledger": address_ledger,
    }


def verify_negative_control() -> dict[str, Any]:
    """Acceptance Criterion 4 (C-NEG): removing one translated routine's citation from the Rust source
    makes the ledger list that routine's executed addresses as untranslated."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp_root = pathlib.Path(tmp)
        for rel_path, _ in RUST_SOURCES:
            src = REPO_ROOT / rel_path
            dst = tmp_root / rel_path
            dst.parent.mkdir(parents=True, exist_ok=True)
            txt = src.read_text(encoding="utf-8")
            if rel_path.endswith("fighter.rs"):
                # Remove cpu_decide's 0x40bd4 citation
                txt = txt.replace("0x40bd4", "0x00000")
            dst.write_text(txt, encoding="utf-8")

        neg_ledger = build_ledger(rust_root=tmp_root)
        undoc = neg_ledger["undocumented_untranslated"]
        assert len(undoc) == 1, f"Expected 1 undocumented untranslated routine, got {len(undoc)}"
        entry = undoc[0]
        assert entry["cite_addr"] == "0x40bd4", f"Unexpected cite_addr: {entry['cite_addr']}"
        assert entry["executed_pc_count"] == 93, f"Expected 93 PCs, got {entry['executed_pc_count']}"
        for pc_hex in entry["executed_pcs"]:
            addr_info = neg_ledger["address_ledger"]["maincpu"][pc_hex]
            assert not addr_info["translated"], f"Expected {pc_hex} to be marked untranslated"
        neg_res = {
            "removed_citation": "0x40bd4 (Bus::cpu_decide in crates/sf-engine/src/fighter.rs)",
            "untranslated_range": f"{entry['start']}..{entry['end']}",
            "untranslated_pc_count": entry["executed_pc_count"],
            "first_pc": entry["executed_pcs"][0],
            "last_pc": entry["executed_pcs"][-1],
            "executed_pcs": entry["executed_pcs"],
        }
        neg_md = render_markdown(neg_ledger, neg_res)
        for pc_hex in entry["executed_pcs"]:
            assert pc_hex in neg_md.split("## 6. Executed, untranslated addresses that no doc names")[1], (
                f"Expected {pc_hex} in section 6 of negative-control markdown"
            )
        return neg_res


def _fmt_scenarios_full(scs: list[str], total_scenarios: int = 64) -> str:
    if not scs:
        return "none (0)"
    if len(scs) == total_scenarios:
        return f"all {total_scenarios} scenarios ({', '.join(scs)})"
    return f"{len(scs)} ({', '.join(scs)})"


def render_markdown(ledger: dict[str, Any], neg_result: dict[str, Any]) -> str:
    totals = ledger["cpu_totals"]
    m_tot = totals["maincpu"]
    a_tot = totals["audiocpu"]
    a2_tot = totals["audio2"]
    all_exec = m_tot["executed_pcs"] + a_tot["executed_pcs"] + a2_tot["executed_pcs"]
    all_trans = m_tot["translated_pcs"] + a_tot["translated_pcs"] + a2_tot["translated_pcs"]
    all_untrans = m_tot["untranslated_pcs"] + a_tot["untranslated_pcs"] + a2_tot["untranslated_pcs"]
    scen_total = ledger["scenario_count"]
    norm_totals = ledger.get("normal_play_cpu_totals", totals)
    nm_tot = norm_totals["maincpu"]
    na_tot = norm_totals["audiocpu"]
    na2_tot = norm_totals["audio2"]
    norm_exec = nm_tot["executed_pcs"] + na_tot["executed_pcs"] + na2_tot["executed_pcs"]
    norm_trans = nm_tot["translated_pcs"] + na_tot["translated_pcs"] + na2_tot["translated_pcs"]
    norm_untrans = nm_tot["untranslated_pcs"] + na_tot["untranslated_pcs"] + na2_tot["untranslated_pcs"]
    norm_scen_count = ledger.get("normal_play_scenario_count", scen_total)

    lines: list[str] = [
        "# Translation coverage ledger",
        "",
        "Evidence class: **TOOL** (ticket #51, F11).",
        "",
        "This document is generated by one command (`python tools/reference/coverage_ledger.py`)",
        "from the MAME 0.289 PC-coverage probe of the 68000 (`:maincpu`) and both Z80s",
        "(`:audiocpu` and `:audio2`) over every pinned reference scenario in",
        f"`tools/reference/scenarios/*.json` ({scen_total} scenarios, {scen_total * 2} probe runs) and the Rust",
        "sources (`crates/sf-engine/src/fighter.rs`, `crates/sf-engine/src/tasks.rs`,",
        "`crates/sf-audio/src/music.rs`, `crates/sf-audio/src/samples.rs`).",
        "The machine-readable ledger is written to",
        "`tools/reference/local/coverage/ledger.json`.",
        "",
        "Code: `tools/reference/coverage_ledger.py`, `tools/reference/tests/test_coverage_ledger.py`,",
        "`crates/sf-engine/tests/coverage_ledger.rs`.",
        "",
        "## 1. Probe reproducibility and summary",
        "",
        f"- **Pinned scenarios probed:** {scen_total} (`{scen_total * 2}` MAME 0.289 runs: `run-a` and `run-b` per scenario).",
        f"- **Two runs per scenario identical:** all {scen_total} / {scen_total} scenarios match byte-for-byte (`0` differences).",
        f"- **Normal-play (`Service Mode = Off`, {norm_scen_count} scenarios, `F1–F71` pre-test-mode scope):** `{norm_exec}` executed PCs (`{nm_tot['executed_pcs']}` `:maincpu`, `{na_tot['executed_pcs']}` `:audiocpu`, `{na2_tot['executed_pcs']}` `:audio2`), `{norm_trans}` translated, **`{norm_untrans}` untranslated**.",
        "",
        "| CPU | Program range | Executed instruction PCs | Translated (cited in Rust) | Untranslated (documented gaps) | Untranslated (unnamed by any doc) |",
        "| --- | --- | --- | --- | --- | --- |",
        f"| `:maincpu` (68000) | `0x3b000..0x492d5` | {m_tot['executed_pcs']} | {m_tot['translated_pcs']} | {m_tot['untranslated_pcs']} | {sum(e['executed_pc_count'] for e in ledger['undocumented_untranslated'] if e['cpu'] == 'maincpu')} |",
        f"| `:audiocpu` (Z80 #1) | `0x0000..0x0ffd` | {a_tot['executed_pcs']} | {a_tot['translated_pcs']} | {a_tot['untranslated_pcs']} | 0 |",
        f"| `:audio2` (Z80 #2) | `0x0000..0x01b1` | {a2_tot['executed_pcs']} | {a2_tot['translated_pcs']} | {a2_tot['untranslated_pcs']} | 0 |",
        f"| **Total** | | **{all_exec}** | **{all_trans}** | **{all_untrans}** | **{sum(e['executed_pc_count'] for e in ledger['undocumented_untranslated'])}** |",
        "",
        "### 1.1 Per-scenario probe runs (`run-a == run-b`)",
        "",
        "| Scenario | File | Frames | `:maincpu` PCs | `:audiocpu` PCs | `:audio2` PCs | `run-a == run-b` | Coverage TSV SHA-256 |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for s in ledger["scenarios"]:
        lines.append(
            f"| `{s['id']}` | `{s['file']}` | {s['frames']} | {s['maincpu_pcs']} | {s['audiocpu_pcs']} | {s['audio2_pcs']} | {'identical' if s['identical'] else 'MISMATCH'} | `{s['sha256'][:16]}…` |"
        )

    lines.extend(
        [
            "",
            "## 2. Executed original addresses mapped to the Rust that cites them",
            "",
            "Each row covers one routine interval of the original program that was executed",
            "in at least one pinned scenario (`executed_pc_count > 0`), listing every executed",
            "instruction PC in that interval and mapping it to the Rust routine(s) (`file:line symbol`)",
            "that cite its entry address, or to `untranslated` with the repository doc that names the gap.",
            "",
        ]
    )

    for cpu, title in (
        ("maincpu", "### 2.1 `:maincpu` (68000)"),
        ("audiocpu", "### 2.2 `:audiocpu` (music Z80)"),
        ("audio2", "### 2.3 `:audio2` (samples Z80)"),
    ):
        lines.extend(
            [
                title,
                "",
                "| Routine range | Executed span (PCs) | Cited entry | Status | Rust citation(s) / Gap doc | Scenarios (first frame) | Executed original addresses |",
                "| --- | --- | --- | --- | --- | --- | --- |",
            ]
        )
        for r in ledger["routines_by_cpu"][cpu]:
            if r["executed_pc_count"] == 0:
                continue
            exec_span = f"`{r['executed_pcs'][0]}..{r['executed_pcs'][-1]}` ({r['executed_pc_count']})"
            status = "translated" if r["translated"] else "**untranslated**"
            if r["translated"]:
                cite_col = ", ".join(f"`{fn}`" for fn in r["rust_routines"])
            else:
                cite_col = f"{r['description']} — `{r['gap_doc'] or 'NONE'}`"
            fh = r["first_hit"]
            sc_col = f"{r['scenario_count']} (`{fh['scenario']}@{fh['frame']}`)" if fh else "0"
            addr_list = ", ".join(f"`{p}`" for p in r["executed_pcs"])
            lines.append(
                f"| `{r['start']}..{r['end']}` | {exec_span} | `{r['cite_addr']}` | {status} | {cite_col} | {sc_col} | {addr_list} |"
            )
        lines.append("")

    lines.extend(
        [
            "## 3. Translated routines mapped to the scenarios that execute them",
            "",
            "| CPU | Rust routine | Primary citation | Cited addresses | Executed PCs | Scenarios executing routine |",
            "| --- | --- | --- | --- | --- | --- |",
        ]
    )
    for tr in ledger["translated_routines"]:
        cites = ", ".join(f"`{c}`" for c in tr["cited_addrs"])
        lines.append(
            f"| `{tr['cpu']}` | `{tr['short_file']}:{tr['line']} {tr['name']}` | `{tr['primary_addr']}` | {cites} | {tr['executed_pc_count']} | {_fmt_scenarios_full(tr['scenarios'], scen_total)} |"
        )

    lines.extend(
        [
            "",
            "## 4. `note()` and `note_unrecovered()` report sites",
            "",
            "| Site (`file:line fn`) | Call | Target address(es) | Status | Scenarios reaching site (first hit) | Disposition |",
            "| --- | --- | --- | --- | --- | --- |",
        ]
    )
    for ns in ledger["note_sites"]:
        targets = ", ".join(f"`{t}`" for t in ns["target_addrs"]) if ns["target_addrs"] else "*(guard)*"
        status = "**reached**" if ns["reached"] else "unreached"
        if ns["reached"] and ns["first_hit"]:
            fh = ns["first_hit"]
            sc_col = (
                f"{ns['scenario_count']} (`{fh['scenario']}@{fh['frame']}` on "
                f"{', '.join(f'`{t}`' for t in ns['reached_targets'])}: "
                f"{', '.join(ns['scenarios'])})"
            )
        else:
            sc_col = "0"
        lines.append(
            f"| `{ns['short_file']}:{ns['line']} {ns['fn']}` | `{ns['call']}` | {targets} | {status} | {sc_col} | {ns['disposition']} |"
        )

    lines.extend(
        [
            "",
            "## 5. Documented gaps reproduced on the tree of the day",
            "",
            "Every executed, untranslated routine interval on the tree of the day corresponds",
            "to a documented gap in `docs/reference/*.md` and `docs/planning/arcade-flow-batch.md`",
            "(either reached during MAME's pre-fixture setup frames `0..2389` before the fight",
            "fixtures start, or at the documented stop boundary of a scenario):",
            "",
            "| CPU | Routine range | Executed span | Executed PCs | First reached | Routine description | Documenting reference |",
            "| --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for g in ledger["documented_gaps"]:
        fh = g["first_hit"]
        exec_span = f"`{g['executed_pcs'][0]}..{g['executed_pcs'][-1]}`"
        lines.append(
            f"| `{g['cpu']}` | `{g['start']}..{g['end']}` | {exec_span} | {g['executed_pc_count']} | `{fh['scenario']}@{fh['frame']}` | {g['description']} | `{g['gap_doc']}` |"
        )

    lines.extend(
        [
            "",
            "## 6. Executed, untranslated addresses that no doc names",
            "",
        ]
    )
    if not ledger["undocumented_untranslated"]:
        lines.append("None (`0` addresses across `:maincpu`, `:audiocpu`, and `:audio2`).")
    else:
        for u in ledger["undocumented_untranslated"]:
            lines.append(
                f"- `{u['cpu']}` `{u['start']}..{u['end']}` ({u['executed_pc_count']} PCs): {', '.join(u['executed_pcs'])}"
            )

    lines.extend(
        [
            "",
            "## 7. Verification and negative control (`C-NEG`)",
            "",
            "- **One-command generation:** `python tools/reference/coverage_ledger.py` verifies all",
            f"  {scen_total} scenario coverage pairs (`run-a.tsv == run-b.tsv`), parses the Rust sources and",
            "  reference docs, writes `tools/reference/local/coverage/ledger.json` and",
            "  `docs/reference/coverage-ledger.md`, and runs the negative control.",
            "- **Negative control (`C-NEG`):** removing the `0x40bd4` citation from",
            f"  `{neg_result['removed_citation']}` makes the ledger classify",
            f"  `{neg_result['untranslated_range']}` (`{neg_result['first_pc']}..{neg_result['last_pc']}`,",
            f"  `{neg_result['untranslated_pc_count']}` executed PCs) as untranslated and list all",
            f"  `{neg_result['untranslated_pc_count']}` executed addresses under section 6",
            "  (executed, untranslated addresses that no doc names).",
            "- **Automated test:** `crates/sf-engine/tests/coverage_ledger.rs` verifies",
            "  Criteria 1–4 as part of `cargo test --workspace --release`.",
            "",
            "## 8. Not covered",
            "",
            "- Unexecuted routine intervals in `MAINCPU_ROUTINES` that no current pinned scenario reaches:",
            "  - `0x3bb06..0x3bb0c` (`compare_bytes_15`, which has no caller in the original ROMs and is unreachable under MAME).",
            "- MAME's coverage probe records instruction execution on the reference emulator,",
            "  not native comparison; each routine's native equivalence is verified by its slice's",
            "  `C-NAT` comparison tests.",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", action="store_true", help="Force re-running MAME coverage probes")
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify that docs/reference/coverage-ledger.md is up to date and pass all checks",
    )
    args = parser.parse_args()

    summaries = ensure_probe_runs(force_rerun=args.probe)
    ledger = build_ledger(rust_root=REPO_ROOT, scenario_summaries=summaries)
    neg_result = verify_negative_control()

    if ledger["undocumented_untranslated"]:
        print(
            f"ERROR: found {len(ledger['undocumented_untranslated'])} undocumented untranslated routines!",
            file=sys.stderr,
        )
        return 1

    md_text = render_markdown(ledger, neg_result)

    if args.check:
        if not LEDGER_MD.exists():
            print(f"ERROR: {LEDGER_MD} does not exist", file=sys.stderr)
            return 1
        current = LEDGER_MD.read_text(encoding="utf-8")
        if current != md_text:
            print(f"ERROR: {LEDGER_MD} is out of date; re-run coverage_ledger.py", file=sys.stderr)
            return 1
        print(
            f"OK: {ledger['scenario_count']} scenarios verified identical, "
            f"maincpu={ledger['cpu_totals']['maincpu']}, "
            f"audiocpu={ledger['cpu_totals']['audiocpu']}, "
            f"audio2={ledger['cpu_totals']['audio2']}, "
            f"C-NEG={neg_result['untranslated_pc_count']} PCs caught."
        )
        return 0

    COVERAGE_DIR.mkdir(parents=True, exist_ok=True)
    LEDGER_JSON.write_text(json.dumps(ledger, indent=2), encoding="utf-8")
    LEDGER_MD.write_text(md_text, encoding="utf-8")
    print(
        f"Generated {LEDGER_MD.relative_to(REPO_ROOT)} and {LEDGER_JSON.relative_to(REPO_ROOT)} "
        f"({ledger['scenario_count']} scenarios, "
        f"maincpu={ledger['cpu_totals']['maincpu']['executed_pcs']} PCs, "
        f"audiocpu={ledger['cpu_totals']['audiocpu']['executed_pcs']} PCs, "
        f"audio2={ledger['cpu_totals']['audio2']['executed_pcs']} PCs, "
        f"C-NEG={neg_result['untranslated_pc_count']} PCs)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
