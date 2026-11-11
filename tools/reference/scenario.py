"""Scenario model for reference-baseline captures.

A scenario is the spec-agreed high-level boundary: reference identity,
machine configuration, frame-indexed logical inputs, bounded duration, and
observation settings. No MAME internals live here; the Lua driver generator
translates a scenario into machine-level port writes.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path


class ScenarioError(ValueError):
    """Raised when a scenario file is invalid."""


DSW1_DEFAULT = 0xDFFF
DSW2_DEFAULT = 0xFFFF

LEGACY_DEFAULT_DIPSWITCH_STRINGS = frozenset(
    {
        "defaults",
        "factory defaults",
        "factory defaults; observed readback DSW1=0xdfff, DSW2=0xffff",
    }
)


def _coin_settings(shift: int) -> dict[str, int]:
    entries = [
        (0x0, ("4 Coins/1 Credit", "4C_1C", "4C1C", "4C/1C")),
        (0x1, ("3 Coins/1 Credit", "3C_1C", "3C1C", "3C/1C")),
        (0x2, ("2 Coins/1 Credit", "2C_1C", "2C1C", "2C/1C")),
        (0x3, ("1 Coin/6 Credits", "1C_6C", "1C6C", "1C/6C")),
        (0x4, ("1 Coin/4 Credits", "1C_4C", "1C4C", "1C/4C")),
        (0x5, ("1 Coin/3 Credits", "1C_3C", "1C3C", "1C/3C")),
        (0x6, ("1 Coin/2 Credits", "1C_2C", "1C2C", "1C/2C")),
        (0x7, ("1 Coin/1 Credit", "1C_1C", "1C1C", "1C/1C")),
    ]
    out: dict[str, int] = {}
    for raw, names in entries:
        val = raw << shift
        for name in names:
            out[name] = val
    return out


def _stage_max_settings(shift: int) -> dict[str, int]:
    entries = [
        (0x7, ("5th Stage Maximum", "5th Stage", "5")),
        (0x6, ("4th Stage Maximum", "4th Stage", "4")),
        (0x5, ("3rd Stage Maximum", "3rd Stage", "3")),
        (0x4, ("2nd Stage Maximum", "2nd Stage", "2")),
        (0x3, ("1st Stage Maximum", "1st Stage", "1")),
        (0x2 if shift == 0 else 0x1, ("None",)),
    ]
    out: dict[str, int] = {}
    for raw, names in entries:
        val = raw << shift
        for name in names:
            out[name] = val
    return out


@dataclass(frozen=True)
class DipFieldSpec:
    port: str  # "DSW1" or "DSW2"
    name: str
    mask: int
    defvalue: int
    settings: dict[str, int]


# MAME 0.289 `sf` DIP switch definitions (sf-mame0289.cpp:479-553).
DIP_FIELDS: tuple[DipFieldSpec, ...] = (
    DipFieldSpec("DSW1", "Coin A", 0x0007, 0x0007, _coin_settings(0)),
    DipFieldSpec("DSW1", "Coin B", 0x0038, 0x0038, _coin_settings(3)),
    DipFieldSpec("DSW1", "Unused 1", 0x0040, 0x0040, {"Off": 0x0040, "On": 0x0000}),
    DipFieldSpec("DSW1", "Unused 2", 0x0080, 0x0080, {"Off": 0x0080, "On": 0x0000}),
    DipFieldSpec("DSW1", "Flip Screen", 0x0100, 0x0100, {"Off": 0x0100, "On": 0x0000}),
    DipFieldSpec("DSW1", "Attract Music", 0x0200, 0x0200, {"Off": 0x0000, "On": 0x0200}),
    DipFieldSpec("DSW1", "Unused 3", 0x0400, 0x0400, {"Off": 0x0400, "On": 0x0000}),
    DipFieldSpec("DSW1", "Unused 4", 0x0800, 0x0800, {"Off": 0x0800, "On": 0x0000}),
    DipFieldSpec("DSW1", "Speed", 0x1000, 0x1000, {"Slow": 0x0000, "Normal": 0x1000}),
    DipFieldSpec("DSW1", "Demo Sounds", 0x2000, 0x0000, {"Off": 0x2000, "On": 0x0000}),
    DipFieldSpec("DSW1", "Freeze", 0x4000, 0x4000, {"Off": 0x4000, "On": 0x0000}),
    DipFieldSpec("DSW1", "Service Mode", 0x8000, 0x8000, {"Off": 0x8000, "On": 0x0000}),
    DipFieldSpec("DSW2", "Game Continuation", 0x0007, 0x0007, _stage_max_settings(0)),
    DipFieldSpec(
        "DSW2",
        "Round Time Count",
        0x0018,
        0x0018,
        {"100": 0x0018, "150": 0x0010, "200": 0x0008, "250": 0x0000},
    ),
    DipFieldSpec(
        "DSW2",
        "Difficulty",
        0x0060,
        0x0060,
        {
            "Normal": 0x0060,
            "Easy": 0x0040,
            "Difficult": 0x0020,
            "Very Difficult": 0x0000,
            "Very_Difficult": 0x0000,
        },
    ),
    DipFieldSpec("DSW2", "Buy-In Feature", 0x0380, 0x0380, _stage_max_settings(7)),
    DipFieldSpec(
        "DSW2",
        "Number of Countries Selected",
        0x0400,
        0x0400,
        {"2": 0x0400, "4": 0x0000},
    ),
    DipFieldSpec("DSW2", "Unused 5", 0x0800, 0x0800, {"Off": 0x0800, "On": 0x0000}),
    DipFieldSpec("DSW2", "Unused 6", 0x1000, 0x1000, {"Off": 0x1000, "On": 0x0000}),
    DipFieldSpec("DSW2", "Unused 7", 0x2000, 0x2000, {"Off": 0x2000, "On": 0x0000}),
    DipFieldSpec("DSW2", "Unused 8", 0x4000, 0x4000, {"Off": 0x4000, "On": 0x0000}),
    DipFieldSpec("DSW2", "Unused 9", 0x8000, 0x8000, {"Off": 0x8000, "On": 0x0000}),
)

_NAMED_DIP_LOOKUP: dict[str, DipFieldSpec] = {}
for _spec in DIP_FIELDS:
    if not _spec.name.startswith("Unused"):
        _NAMED_DIP_LOOKUP[_spec.name] = _spec
        _NAMED_DIP_LOOKUP[_spec.name.lower().replace(" ", "_").replace("-", "_")] = _spec
_NAMED_DIP_LOOKUP["countries"] = _NAMED_DIP_LOOKUP["Number of Countries Selected"]
_NAMED_DIP_LOOKUP["Countries"] = _NAMED_DIP_LOOKUP["Number of Countries Selected"]
_NAMED_DIP_LOOKUP["round_time"] = _NAMED_DIP_LOOKUP["Round Time Count"]
_NAMED_DIP_LOOKUP["Round Time"] = _NAMED_DIP_LOOKUP["Round Time Count"]
_NAMED_DIP_LOOKUP["buy_in"] = _NAMED_DIP_LOOKUP["Buy-In Feature"]
_NAMED_DIP_LOOKUP["Buy-In"] = _NAMED_DIP_LOOKUP["Buy-In Feature"]
_NAMED_DIP_LOOKUP["flip"] = _NAMED_DIP_LOOKUP["Flip Screen"]
_NAMED_DIP_LOOKUP["Flip"] = _NAMED_DIP_LOOKUP["Flip Screen"]
_NAMED_DIP_LOOKUP["service"] = _NAMED_DIP_LOOKUP["Service Mode"]
_NAMED_DIP_LOOKUP["Service"] = _NAMED_DIP_LOOKUP["Service Mode"]
_NAMED_DIP_LOOKUP["self_test"] = _NAMED_DIP_LOOKUP["Service Mode"]
_NAMED_DIP_LOOKUP["Self-Test"] = _NAMED_DIP_LOOKUP["Service Mode"]


def _parse_port_word(value: object, port_name: str) -> int:
    if isinstance(value, bool):
        raise ScenarioError(f"DIP port {port_name} word must be a 16-bit integer, got {value!r}")
    if isinstance(value, int):
        word = value
    elif isinstance(value, str):
        try:
            word = int(value, 0)
        except ValueError as exc:
            raise ScenarioError(
                f"DIP port {port_name} word '{value}' is not a valid integer"
            ) from exc
    else:
        raise ScenarioError(
            f"DIP port {port_name} word must be an int or hex string, got {value!r}"
        )
    if word < 0 or word > 0xFFFF:
        raise ScenarioError(
            f"DIP port {port_name} word {value!r} is out of 16-bit range 0x0000..0xffff"
        )
    return word


def parse_dipswitches(
    raw: object, dsw1: int = DSW1_DEFAULT, dsw2: int = DSW2_DEFAULT
) -> tuple[int, int, bool]:
    """Resolve DIP fields into `(dsw1, dsw2, explicit)`, starting from the given words."""
    if raw is None:
        return dsw1, dsw2, False
    if isinstance(raw, str):
        if raw in LEGACY_DEFAULT_DIPSWITCH_STRINGS:
            return dsw1, dsw2, False
        raise ScenarioError(f"unknown dipswitches preset '{raw}'")
    if not isinstance(raw, dict):
        raise ScenarioError(
            f"machine.dipswitches must be a mapping or default string, got {type(raw).__name__}"
        )
    if not raw:
        raise ScenarioError("machine.dipswitches mapping cannot be empty")

    explicit_dsw1_word: int | None = None
    explicit_dsw2_word: int | None = None

    for key, value in raw.items():
        if key in ("dsw1", "DSW1"):
            explicit_dsw1_word = _parse_port_word(value, "DSW1")
            dsw1 = explicit_dsw1_word
        elif key in ("dsw2", "DSW2"):
            explicit_dsw2_word = _parse_port_word(value, "DSW2")
            dsw2 = explicit_dsw2_word

    for key, value in raw.items():
        if key in ("dsw1", "DSW1", "dsw2", "DSW2"):
            continue
        spec = _NAMED_DIP_LOOKUP.get(key)
        if spec is None:
            raise ScenarioError(f"unknown DIP switch '{key}'")
        if isinstance(value, bool):
            raise ScenarioError(
                f"invalid value {value!r} for DIP switch '{key}'"
            )
        setting_key = str(value) if isinstance(value, (int, str)) else None
        if setting_key is None or setting_key not in spec.settings:
            raise ScenarioError(
                f"invalid value {value!r} for DIP switch '{key}' "
                f"(allowed: {', '.join(sorted(spec.settings))})"
            )
        bits = spec.settings[setting_key]
        if spec.port == "DSW1":
            if explicit_dsw1_word is not None and (explicit_dsw1_word & spec.mask) != bits:
                raise ScenarioError(
                    f"DIP switch '{key}' conflicts with explicit DSW1 port word"
                )
            dsw1 = (dsw1 & ~spec.mask) | bits
        else:
            if explicit_dsw2_word is not None and (explicit_dsw2_word & spec.mask) != bits:
                raise ScenarioError(
                    f"DIP switch '{key}' conflicts with explicit DSW2 port word"
                )
            dsw2 = (dsw2 & ~spec.mask) | bits

    if (dsw2 & 0x0007) in (0x0000, 0x0001):
        raise ScenarioError(
            f"DSW2 word 0x{dsw2:04x} uses unlisted Game Continuation bits 0x{dsw2 & 0x0007:04x}"
        )
    if (dsw2 & 0x0380) in (0x0000, 0x0100):
        raise ScenarioError(
            f"DSW2 word 0x{dsw2:04x} uses unlisted Buy-In Feature bits 0x{dsw2 & 0x0380:04x}"
        )

    return dsw1, dsw2, True


@dataclass(frozen=True)
class Control:
    name: str
    port: str
    bit: int


@dataclass(frozen=True)
class InputEvent:
    frame: int
    press: str
    hold_frames: int
    comment: str = ""


@dataclass(frozen=True)
class DipChange:
    frame: int
    dsw1: int
    dsw2: int


@dataclass(frozen=True)
class Scenario:
    scenario_id: str
    format_version: int
    rom_set: str
    mame_version: str
    machine_set: str
    controls: dict[str, Control]
    duration_frames: int
    inputs: list[InputEvent]
    screen_downsample_stride: int
    ram_crc_every_frames: int
    checkpoints: list[int] = field(default_factory=list)
    dsw1: int = DSW1_DEFAULT
    dsw2: int = DSW2_DEFAULT
    dipswitches_explicit: bool = False
    dip_changes: list[DipChange] = field(default_factory=list)


def load_scenario(path: str | Path) -> Scenario:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    return parse_scenario(data)


def _require(data: dict, key: str, context: str):
    if key not in data:
        raise ScenarioError(f"missing required field {context}.{key}")
    return data[key]


def parse_scenario(data: dict) -> Scenario:
    scenario_id = _require(data, "scenario_id", "scenario")
    reference = _require(data, "reference", "scenario")
    machine = _require(data, "machine", "scenario")
    observations = _require(data, "observations", "scenario")

    controls = {
        name: Control(name=name, port=_require(c, "port", f"controls.{name}"), bit=_require(c, "bit", f"controls.{name}"))
        for name, c in _require(data, "controls", "scenario").items()
    }
    inputs = [
        InputEvent(
            frame=_require(e, "frame", "inputs[]"),
            press=_require(e, "press", "inputs[]"),
            hold_frames=_require(e, "hold_frames", "inputs[]"),
            comment=e.get("comment", ""),
        )
        for e in _require(data, "inputs", "scenario")
    ]
    for e in inputs:
        if e.press not in controls:
            raise ScenarioError(
                f"input event at frame {e.frame} presses unknown logical input '{e.press}'"
            )
        if e.hold_frames < 1:
            raise ScenarioError(
                f"input event for '{e.press}' at frame {e.frame} has hold_frames < 1"
            )
    by_input: dict[str, list[InputEvent]] = {}
    for e in inputs:
        by_input.setdefault(e.press, []).append(e)
    for name, events in by_input.items():
        events.sort(key=lambda e: e.frame)
        for prev, cur in zip(events, events[1:]):
            if cur.frame < prev.frame + prev.hold_frames:
                raise ScenarioError(
                    f"overlapping press of '{name}' at frames {prev.frame} and {cur.frame}"
                )
    initial_dsw1, initial_dsw2, dipswitches_explicit = parse_dipswitches(
        machine.get("dipswitches")
    )
    dsw1, dsw2 = initial_dsw1, initial_dsw2
    dip_changes = []
    for raw_change in data.get("dip_changes", []):
        frame = _require(raw_change, "frame", "dip_changes[]")
        if dip_changes and frame <= dip_changes[-1].frame:
            raise ScenarioError("dip_changes frames must be strictly increasing")
        dsw1, dsw2, _ = parse_dipswitches(
            _require(raw_change, "dipswitches", "dip_changes[]"), dsw1, dsw2
        )
        dip_changes.append(DipChange(frame=frame, dsw1=dsw1, dsw2=dsw2))
    return Scenario(
        scenario_id=scenario_id,
        format_version=_require(data, "format_version", "scenario"),
        rom_set=_require(reference, "rom_set", "reference"),
        mame_version=_require(reference, "mame_version", "reference"),
        machine_set=_require(machine, "set", "machine"),
        controls=controls,
        duration_frames=_require(data, "duration_frames", "scenario"),
        inputs=inputs,
        screen_downsample_stride=_require(observations, "screen_downsample_stride", "observations"),
        ram_crc_every_frames=_require(observations, "ram_crc_every_frames", "observations"),
        checkpoints=list(observations.get("checkpoints", [])),
        dsw1=initial_dsw1,
        dsw2=initial_dsw2,
        dipswitches_explicit=dipswitches_explicit,
        dip_changes=dip_changes,
    )


def compute_input_events(s: Scenario) -> list[dict]:
    """Translate frame-indexed logical inputs into MAME field state events.

    Each press expands into a press event (state 1) at its frame and a
    release event (state 0) after hold_frames. The Lua driver resolves
    (port, mask) to an ioport field and calls set_value/clear_value, which
    is the injection path MAME's input system actually honors.
    """
    events = []
    for e in s.inputs:
        control = s.controls[e.press]
        mask = 1 << control.bit
        events.append(
            {"frame": e.frame, "port": control.port, "mask": mask, "state": 1}
        )
        events.append(
            {
                "frame": e.frame + e.hold_frames,
                "port": control.port,
                "mask": mask,
                "state": 0,
            }
        )
    events.sort(key=lambda ev: ev["frame"])
    return events
