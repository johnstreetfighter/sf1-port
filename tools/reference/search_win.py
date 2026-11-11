"""Reference-side winning-script search tool (ticket #46 / F6).

Extends a pinned scenario's input prefix fight by fight using a parametrized
strategy family, runs the candidate in the pinned MAME 0.289 build with a
read-only Lua observation probe (never writing emulated memory), and reads
the fight outcome (`0xffc009`, `0xffc003` / `0xffc004`, round wins, health,
and flow-task resume addresses).
"""

from __future__ import annotations

import argparse
import copy
import json
import subprocess
import sys
import tempfile
import time
from dataclasses import asdict, dataclass, field
from pathlib import Path

from lua_gen import write_cfg
from scenario import Scenario, compute_input_events, load_scenario, parse_scenario
from run_scenario import BASELINE_OPTIONS, HERE


@dataclass(frozen=True)
class StrategySpec:
    """Parametrized P1 input strategy over `[start_frame, end_frame]`."""

    move: str = "shoryuken_hp"
    start_frame: int = 5860
    end_frame: int = 8500
    period: int = 60
    walk_frames: int = 20
    move_offset: int = 22
    facing: str = "right"
    button_hold: int = 4


VALID_MOVES = (
    "shoryuken_hp",
    "shoryuken_mp",
    "shoryuken_lp",
    "hadoken_hp",
    "hadoken_mp",
    "hadoken_lp",
    "tatsumaki_hk",
    "tatsumaki_mk",
    "tatsumaki_lk",
    "walk_hp",
    "walk_hk",
    "crouch_hk",
    "jump_hk",
    "block_high",
    "block_low",
    "idle",
)


def _move_events(
    cycle_idx: int,
    base_frame: int,
    move: str,
    dir_f: str,
    dir_b: str,
    button_hold: int,
) -> list[dict]:
    if move.startswith("shoryuken_"):
        btn = "p1_" + move.split("_", 1)[1]
        label = f"Shoryuken {btn.upper().replace('P1_', '')} ({cycle_idx})"
        return [
            {
                "frame": base_frame,
                "press": dir_f,
                "hold_frames": 3,
                "comment": f"{label}: F",
            },
            {
                "frame": base_frame + 3,
                "press": "p1_down",
                "hold_frames": 11,
                "comment": f"{label}: D",
            },
            {
                "frame": base_frame + 6,
                "press": dir_f,
                "hold_frames": 8,
                "comment": f"{label}: DF",
            },
            {
                "frame": base_frame + 7,
                "press": btn,
                "hold_frames": button_hold,
                "comment": f"{label}: button",
            },
        ]
    if move.startswith("hadoken_"):
        btn = "p1_" + move.split("_", 1)[1]
        label = f"Hadoken {btn.upper().replace('P1_', '')} ({cycle_idx})"
        return [
            {
                "frame": base_frame,
                "press": "p1_down",
                "hold_frames": 6,
                "comment": f"{label}: D",
            },
            {
                "frame": base_frame + 3,
                "press": dir_f,
                "hold_frames": 9,
                "comment": f"{label}: DF->F",
            },
            {
                "frame": base_frame + 7,
                "press": btn,
                "hold_frames": button_hold,
                "comment": f"{label}: button",
            },
        ]
    if move.startswith("tatsumaki_"):
        btn = "p1_" + move.split("_", 1)[1]
        label = f"Tatsumaki {btn.upper().replace('P1_', '')} ({cycle_idx})"
        return [
            {
                "frame": base_frame,
                "press": "p1_down",
                "hold_frames": 6,
                "comment": f"{label}: D",
            },
            {
                "frame": base_frame + 3,
                "press": dir_b,
                "hold_frames": 9,
                "comment": f"{label}: DB->B",
            },
            {
                "frame": base_frame + 7,
                "press": btn,
                "hold_frames": button_hold,
                "comment": f"{label}: button",
            },
        ]
    if move == "walk_hp":
        return [
            {
                "frame": base_frame,
                "press": "p1_hp",
                "hold_frames": button_hold,
                "comment": f"standing HP ({cycle_idx})",
            }
        ]
    if move == "walk_hk":
        return [
            {
                "frame": base_frame,
                "press": "p1_hk",
                "hold_frames": button_hold,
                "comment": f"standing HK ({cycle_idx})",
            }
        ]
    if move == "crouch_hk":
        return [
            {
                "frame": base_frame,
                "press": "p1_down",
                "hold_frames": 12,
                "comment": f"crouch HK ({cycle_idx}): D",
            },
            {
                "frame": base_frame + 2,
                "press": "p1_hk",
                "hold_frames": button_hold,
                "comment": f"crouch HK ({cycle_idx}): button",
            },
        ]
    if move == "jump_hk":
        return [
            {
                "frame": base_frame,
                "press": "p1_up",
                "hold_frames": 6,
                "comment": f"jump HK ({cycle_idx}): U",
            },
            {
                "frame": base_frame,
                "press": dir_f,
                "hold_frames": 6,
                "comment": f"jump HK ({cycle_idx}): F",
            },
            {
                "frame": base_frame + 12,
                "press": "p1_hk",
                "hold_frames": button_hold,
                "comment": f"jump HK ({cycle_idx}): button",
            },
        ]
    if move == "block_high":
        return [
            {
                "frame": base_frame,
                "press": dir_b,
                "hold_frames": 20,
                "comment": f"block high ({cycle_idx})",
            }
        ]
    if move == "block_low":
        return [
            {
                "frame": base_frame,
                "press": dir_b,
                "hold_frames": 20,
                "comment": f"block low ({cycle_idx}): B",
            },
            {
                "frame": base_frame,
                "press": "p1_down",
                "hold_frames": 20,
                "comment": f"block low ({cycle_idx}): D",
            },
        ]
    if move == "idle":
        return []
    raise ValueError(f"unknown strategy move '{move}'")


def generate_strategy_inputs(spec: StrategySpec, start_cycle_index: int = 57) -> list[dict]:
    """Generate frame-indexed logical input dicts for `spec`."""
    if spec.move not in VALID_MOVES:
        raise ValueError(f"unknown strategy move '{spec.move}'")
    if spec.period < 25:
        raise ValueError(f"period {spec.period} is too short (< 25)")
    if spec.walk_frames < 0 or spec.walk_frames >= spec.move_offset:
        raise ValueError(
            f"walk_frames ({spec.walk_frames}) must be in 0..{spec.move_offset - 1}"
        )

    dir_f = "p1_right" if spec.facing == "right" else "p1_left"
    dir_b = "p1_left" if spec.facing == "right" else "p1_right"

    out: list[dict] = []
    cycle = start_cycle_index
    t = spec.start_frame
    while t <= spec.end_frame:
        if spec.walk_frames > 0 and spec.move not in ("block_high", "block_low", "idle"):
            if t + spec.walk_frames <= spec.end_frame:
                out.append(
                    {
                        "frame": t,
                        "press": dir_f,
                        "hold_frames": spec.walk_frames,
                        "comment": f"close in ({cycle})",
                    }
                )
        move_t = t + spec.move_offset
        if move_t <= spec.end_frame:
            for ev in _move_events(
                cycle, move_t, spec.move, dir_f, dir_b, spec.button_hold
            ):
                if ev["frame"] + ev["hold_frames"] <= spec.end_frame + 20:
                    out.append(ev)
        t += spec.period
        cycle += 1
    return out


def iter_strategy_candidates(
    *,
    start_frame: int,
    end_frame: int,
    moves: tuple[str, ...] = ("shoryuken_hp", "hadoken_hp", "tatsumaki_hk", "crouch_hk", "jump_hk"),
    periods: tuple[int, ...] = (60, 50, 70, 45),
    walk_frames_list: tuple[int, ...] = (20, 15, 10, 0),
    phase_offsets: tuple[int, ...] = (0, 10, 20, 30),
    move_offset: int = 22,
    facing: str = "right",
):
    """Yield deterministic `StrategySpec` candidates in priority order."""
    for move in moves:
        for period in periods:
            for walk_frames in walk_frames_list:
                if walk_frames >= move_offset:
                    continue
                for phase in phase_offsets:
                    yield StrategySpec(
                        move=move,
                        start_frame=start_frame + phase,
                        end_frame=end_frame,
                        period=period,
                        walk_frames=walk_frames,
                        move_offset=move_offset,
                        facing=facing,
                    )


def extend_scenario_dict(
    base_data: dict,
    *,
    scenario_id: str,
    description: str,
    divergence_frame: int,
    segment_inputs: list[dict],
    extra_inputs: list[dict],
    duration_frames: int,
    checkpoints: list[int],
) -> dict:
    """Build a scenario dict keeping `base_data` inputs strictly before `divergence_frame`."""
    out = copy.deepcopy(base_data)
    out["scenario_id"] = scenario_id
    out["description"] = description
    out["duration_frames"] = duration_frames
    prefix = [e for e in base_data["inputs"] if e["frame"] < divergence_frame]
    combined = prefix + copy.deepcopy(segment_inputs) + copy.deepcopy(extra_inputs)
    combined.sort(key=lambda e: (e["frame"], e["press"]))
    out["inputs"] = combined
    out["observations"]["checkpoints"] = list(checkpoints)
    return out


def _lua_string(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def generate_probe_lua(
    s: Scenario,
    *,
    out_dir: str,
    start_frame: int,
    target_stage: int,
    target_fight: int,
    stop_at_next_fight_loop: bool = True,
    stop_on_cpu_win: bool = True,
) -> str:
    """Generate a read-only MAME Lua driver that logs fight outcomes and milestones."""
    events = compute_input_events(s)
    event_lines = [
        f"    {{ frame = {ev['frame']}, port = {_lua_string(ev['port'])}, "
        f"mask = 0x{ev['mask']:04x}, state = {ev['state']} }},"
        for ev in events
    ]
    events_lua = "\n".join(event_lines)
    stop_loop_lua = "true" if stop_at_next_fight_loop else "false"
    stop_cpu_lua = "true" if stop_on_cpu_win else "false"

    return f"""-- Read-only winning-script search probe for MAME 0.289 ("sf").
-- Applies logical inputs via ioport fields and reads outcome state from
-- main_space without ever writing emulated memory.
local CONFIG = {{
  scenario_id = {_lua_string(s.scenario_id)},
  duration_frames = {s.duration_frames},
  start_frame = {start_frame},
  target_stage = {target_stage},
  target_fight = {target_fight},
  stop_at_next_fight_loop = {stop_loop_lua},
  stop_on_cpu_win = {stop_cpu_lua},
  out_dir = {_lua_string(out_dir)},
  input_events = {{
{events_lua}
  }},
}}

local screen = manager.machine.screens[":screen"]
local main_space = manager.machine.devices[":maincpu"].spaces["program"]
local ports = manager.machine.ioport.ports

local field_cache = {{}}
local function resolve_field(port, mask)
	local key = port .. ":" .. mask
	if field_cache[key] == nil then
		local found = nil
		for _, field in pairs(ports[":" .. port].fields) do
			if field.mask == mask then
				found = field
				break
			end
		end
		assert(found ~= nil, "no ioport field for port " .. port .. " mask " .. mask)
		field_cache[key] = found
	end
	return field_cache[key]
end

local input_events = CONFIG.input_events
local next_event = 1
local function apply_inputs(fn)
	while next_event <= #input_events and input_events[next_event].frame <= fn do
		local ev = input_events[next_event]
		local field = resolve_field(ev.port, ev.mask)
		if ev.state == 1 then
			field:set_value(1)
		else
			field:clear_value()
		end
		next_event = next_event + 1
	end
end

local function slot1_resume()
	local state = main_space:read_u8(0xff8016)
	if state ~= 8 then
		return 0
	end
	local usp = main_space:read_u32(0xff801e)
	if usp < 0xff8000 or usp > 0xffdf00 then
		return 0
	end
	if main_space:read_u32(usp + 60) ~= 0x3b5a8 then
		return 0
	end
	return main_space:read_u32(usp + 64)
end

local rounds = {{}}
local prev_p1_wins = nil
local prev_cpu_wins = nil
local prev_round = nil
local geki_events_first = nil
local geki_events_last = nil
local bonus_start_frame = nil
local stage_step_frame = nil
local next_fight_round1_frame = nil
local next_fight_loop_frame = nil
local finished = false

local function write_probe_json(fn, stop_reason)
	local stage = main_space:read_u8(0xffc003)
	local fight = main_space:read_u8(0xffc004)
	local winner = main_space:read_u8(0xffc009)
	local p1_wins = main_space:read_u8(0xff86b8)
	local cpu_wins = main_space:read_u8(0xffc66c)
	local p1_health = main_space:read_u8(0xff86b7) * 4 + main_space:read_u8(0xff86b6)
	local cpu_health = main_space:read_u8(0xffc66b) * 4 + main_space:read_u8(0xffc66a)

	local f = assert(io.open(CONFIG.out_dir .. "/probe.json", "wb"))
	f:write("{{\\n")
	f:write(string.format('  "scenario_id": "%s",\\n', CONFIG.scenario_id))
	f:write(string.format('  "start_frame": %d,\\n', CONFIG.start_frame))
	f:write(string.format('  "final_frame": %d,\\n', fn))
	f:write(string.format('  "stop_reason": "%s",\\n', stop_reason))
	f:write(string.format('  "stage": %d,\\n', stage))
	f:write(string.format('  "fight": %d,\\n', fight))
	f:write(string.format('  "winner": %d,\\n', winner))
	f:write(string.format('  "p1_wins": %d,\\n', p1_wins))
	f:write(string.format('  "cpu_wins": %d,\\n', cpu_wins))
	f:write(string.format('  "p1_health": %d,\\n', p1_health))
	f:write(string.format('  "cpu_health": %d,\\n', cpu_health))
	f:write('  "rounds": [\\n')
	for i, r in ipairs(rounds) do
		local comma = (i < #rounds) and "," or ""
		f:write(string.format(
			'    {{"round": %d, "frame": %d, "p1_wins": %d, "cpu_wins": %d, "p1_health": %d, "cpu_health": %d}}%s\\n',
			r.round, r.frame, r.p1_wins, r.cpu_wins, r.p1_health, r.cpu_health, comma
		))
	end
	f:write('  ],\\n')
	f:write('  "milestones": {{\\n')
	f:write(string.format('    "geki_events_0xb_6_first_frame": %s,\\n', geki_events_first and tostring(geki_events_first) or "null"))
	f:write(string.format('    "geki_events_0xb_6_last_frame": %s,\\n', geki_events_last and tostring(geki_events_last) or "null"))
	f:write(string.format('    "bonus_start_frame": %s,\\n', bonus_start_frame and tostring(bonus_start_frame) or "null"))
	f:write(string.format('    "stage_step_frame": %s,\\n', stage_step_frame and tostring(stage_step_frame) or "null"))
	f:write(string.format('    "next_fight_round1_frame": %s,\\n', next_fight_round1_frame and tostring(next_fight_round1_frame) or "null"))
	f:write(string.format('    "next_fight_loop_frame": %s\\n', next_fight_loop_frame and tostring(next_fight_loop_frame) or "null"))
	f:write('  }}\\n')
	f:write("}}\\n")
	f:close()
end

emu.register_frame_done(function()
	if finished then
		return
	end
	local fn = screen:frame_number()
	if fn >= CONFIG.duration_frames then
		finished = true
		write_probe_json(fn, "duration_reached")
		manager.machine:exit()
		return
	end

	apply_inputs(fn)

	if fn < CONFIG.start_frame then
		return
	end

	local stage = main_space:read_u8(0xffc003)
	local fight = main_space:read_u8(0xffc004)
	local scene = main_space:read_u8(0xffc008)
	local winner = main_space:read_u8(0xffc009)
	local round = main_space:read_u8(0xffc00a)
	local p1_wins = main_space:read_u8(0xff86b8)
	local cpu_wins = main_space:read_u8(0xffc66c)
	local p1_health = main_space:read_u8(0xff86b7) * 4 + main_space:read_u8(0xff86b6)
	local cpu_health = main_space:read_u8(0xffc66b) * 4 + main_space:read_u8(0xffc66a)
	local event_id = main_space:read_u8(0xff8e36)
	local s1_res = slot1_resume()

	if prev_p1_wins == nil then
		prev_p1_wins = p1_wins
		prev_cpu_wins = cpu_wins
		prev_round = round
	else
		if p1_wins > prev_p1_wins or cpu_wins > prev_cpu_wins then
			rounds[#rounds + 1] = {{
				round = prev_round,
				frame = fn,
				p1_wins = p1_wins,
				cpu_wins = cpu_wins,
				p1_health = p1_health,
				cpu_health = cpu_health,
			}}
			prev_p1_wins = p1_wins
			prev_cpu_wins = cpu_wins
		end
		prev_round = round
	end

	if winner == 1 and fight == 1 and (scene & 0x20) ~= 0 and (event_id == 0x0b or event_id == 0x06) then
		if geki_events_first == nil then
			geki_events_first = fn
		end
		geki_events_last = fn
	end

	if bonus_start_frame == nil and fight == 0 and (scene == 0x40 or s1_res == 0x44a60) and (s1_res == 0x40dd4 or s1_res == 0x41814 or s1_res == 0x41aa6 or s1_res == 0x420ba or s1_res == 0x420cc or s1_res == 0x44a60) then
		bonus_start_frame = fn
	end

	if stage_step_frame == nil and stage == CONFIG.target_stage then
		stage_step_frame = fn
	end

	if next_fight_round1_frame == nil and stage == CONFIG.target_stage and fight == CONFIG.target_fight and s1_res == 0x3ea3e then
		next_fight_round1_frame = fn
	end

	if next_fight_round1_frame ~= nil and next_fight_loop_frame == nil and stage == CONFIG.target_stage and fight == CONFIG.target_fight and s1_res == 0x3eafa and scene == 0x80 then
		next_fight_loop_frame = fn
		if CONFIG.stop_at_next_fight_loop then
			finished = true
			write_probe_json(fn, "target_fight_loop_reached")
			manager.machine:exit()
			return
		end
	end

	if CONFIG.stop_on_cpu_win and (winner == 4 or scene == 0x08) then
		finished = true
		write_probe_json(fn, "cpu_match_win")
		manager.machine:exit()
		return
	end
end)
"""


@dataclass(frozen=True)
class ProbeOutcome:
    scenario_id: str
    start_frame: int
    final_frame: int
    stop_reason: str
    stage: int
    fight: int
    winner: int
    p1_wins: int
    cpu_wins: int
    p1_health: int
    cpu_health: int
    rounds: list[dict] = field(default_factory=list)
    geki_events_0xb_6_first_frame: int | None = None
    geki_events_0xb_6_last_frame: int | None = None
    bonus_start_frame: int | None = None
    stage_step_frame: int | None = None
    next_fight_round1_frame: int | None = None
    next_fight_loop_frame: int | None = None

    @property
    def match_won(self) -> bool:
        return (
            any(r.get("p1_wins") == 2 for r in self.rounds)
            or self.bonus_start_frame is not None
            or self.next_fight_round1_frame is not None
        )


def load_probe_outcome(path: str | Path) -> ProbeOutcome:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    m = data.get("milestones", {})
    return ProbeOutcome(
        scenario_id=data["scenario_id"],
        start_frame=data["start_frame"],
        final_frame=data["final_frame"],
        stop_reason=data["stop_reason"],
        stage=data["stage"],
        fight=data["fight"],
        winner=data["winner"],
        p1_wins=data["p1_wins"],
        cpu_wins=data["cpu_wins"],
        p1_health=data["p1_health"],
        cpu_health=data["cpu_health"],
        rounds=list(data.get("rounds", [])),
        geki_events_0xb_6_first_frame=m.get("geki_events_0xb_6_first_frame"),
        geki_events_0xb_6_last_frame=m.get("geki_events_0xb_6_last_frame"),
        bonus_start_frame=m.get("bonus_start_frame"),
        stage_step_frame=m.get("stage_step_frame"),
        next_fight_round1_frame=m.get("next_fight_round1_frame"),
        next_fight_loop_frame=m.get("next_fight_loop_frame"),
    )


def run_candidate(
    scenario_data: dict,
    *,
    start_frame: int,
    target_stage: int = 1,
    target_fight: int = 0,
    stop_at_next_fight_loop: bool = True,
    stop_on_cpu_win: bool = True,
    mame_exe: Path = HERE / "local" / "mame" / "mame.exe",
    rompath: Path = HERE / "local" / "roms",
    work_dir: Path | None = None,
) -> ProbeOutcome:
    """Run a candidate scenario dict in MAME with the read-only probe and return its `ProbeOutcome`."""
    s = parse_scenario(scenario_data)
    temp_ctx = tempfile.TemporaryDirectory() if work_dir is None else None
    out_dir = Path(temp_ctx.name).resolve() if temp_ctx is not None else work_dir.resolve()
    try:
        out_dir.mkdir(parents=True, exist_ok=True)
        driver_path = out_dir / "probe_driver.lua"
        driver_path.write_text(
            generate_probe_lua(
                s,
                out_dir=out_dir.as_posix(),
                start_frame=start_frame,
                target_stage=target_stage,
                target_fight=target_fight,
                stop_at_next_fight_loop=stop_at_next_fight_loop,
                stop_on_cpu_win=stop_on_cpu_win,
            ),
            encoding="utf-8",
        )
        options = list(BASELINE_OPTIONS)
        if s.dipswitches_explicit:
            write_cfg(s, out_dir)
            options = [opt for opt in options if opt != "-noreadconfig"]
        cap_seconds = int(s.duration_frames / 60 * 3) + 30
        cmd = [
            str(mame_exe),
            s.machine_set,
            "-rompath",
            str(rompath),
            *options,
            "-cfg_directory",
            str(out_dir / "cfg"),
            "-snapshot_directory",
            str(out_dir / "snap"),
            "-autoboot_script",
            str(driver_path),
            "-autoboot_delay",
            "0",
            "-seconds_to_run",
            str(cap_seconds),
        ]
        proc = subprocess.run(cmd, capture_output=True, text=True)
        if proc.returncode != 0:
            raise RuntimeError(f"MAME exited with {proc.returncode}: {proc.stderr}")
        probe_path = out_dir / "probe.json"
        if not probe_path.is_file():
            raise RuntimeError("probe.json was not written by probe_driver.lua")
        return load_probe_outcome(probe_path)
    finally:
        if temp_ctx is not None:
            temp_ctx.cleanup()


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--base",
        default=HERE / "scenarios" / "sf_win_01.json",
        type=Path,
        help="base scenario JSON providing prefix inputs",
    )
    parser.add_argument(
        "--divergence-frame",
        type=int,
        default=5860,
        help="keep base inputs strictly before this frame",
    )
    parser.add_argument("--move", default="shoryuken_hp", choices=VALID_MOVES)
    parser.add_argument("--start-frame", type=int, default=5860)
    parser.add_argument("--end-frame", type=int, default=8500)
    parser.add_argument("--period", type=int, default=60)
    parser.add_argument("--walk-frames", type=int, default=20)
    parser.add_argument("--move-offset", type=int, default=22)
    parser.add_argument("--duration-frames", type=int, default=12000)
    parser.add_argument("--target-stage", type=int, default=1)
    parser.add_argument("--target-fight", type=int, default=0)
    parser.add_argument(
        "--bonus-press-frame",
        type=int,
        default=None,
        help="optional fixed p1_hp press frame during bonus stage",
    )
    parser.add_argument(
        "--search",
        action="store_true",
        help="search over the strategy family until a winning candidate is found",
    )
    parser.add_argument(
        "--max-candidates",
        type=int,
        default=16,
        help="maximum candidates to evaluate in --search mode",
    )
    parser.add_argument("--mame", type=Path, default=HERE / "local" / "mame" / "mame.exe")
    parser.add_argument("--rompath", type=Path, default=HERE / "local" / "roms")
    parser.add_argument("--out-json", type=Path, default=None, help="write probe outcome JSON")
    args = parser.parse_args(argv)

    base_data = json.loads(args.base.read_text(encoding="utf-8"))
    extra = []
    if args.bonus_press_frame is not None:
        extra.append(
            {
                "frame": args.bonus_press_frame,
                "press": "p1_hp",
                "hold_frames": 6,
                "comment": "bonus-1 plank break press",
            }
        )

    if args.search:
        candidates = list(
            iter_strategy_candidates(
                start_frame=args.start_frame,
                end_frame=args.end_frame,
                move_offset=args.move_offset,
            )
        )[: args.max_candidates]
    else:
        candidates = [
            StrategySpec(
                move=args.move,
                start_frame=args.start_frame,
                end_frame=args.end_frame,
                period=args.period,
                walk_frames=args.walk_frames,
                move_offset=args.move_offset,
            )
        ]

    for idx, spec in enumerate(candidates):
        seg = generate_strategy_inputs(spec)
        cand = extend_scenario_dict(
            base_data,
            scenario_id=f"sf-win-search-candidate-{idx}",
            description="candidate",
            divergence_frame=args.divergence_frame,
            segment_inputs=seg,
            extra_inputs=extra,
            duration_frames=args.duration_frames,
            checkpoints=[],
        )
        started = time.monotonic()
        outcome = run_candidate(
            cand,
            start_frame=args.divergence_frame,
            target_stage=args.target_stage,
            target_fight=args.target_fight,
            mame_exe=args.mame,
            rompath=args.rompath,
        )
        elapsed = time.monotonic() - started
        result_dict = asdict(outcome)
        result_dict["strategy"] = asdict(spec)
        result_dict["wall_seconds"] = round(elapsed, 2)
        print(json.dumps(result_dict, indent=2))
        if outcome.match_won:
            if args.out_json is not None:
                args.out_json.write_text(json.dumps(result_dict, indent=2), encoding="utf-8")
            return 0
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
