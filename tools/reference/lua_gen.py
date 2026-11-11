"""Generate a standalone Lua driver from a scenario.

The generated file embeds the scenario's port schedule and observation
settings as a Lua table in place of the template's @@CONFIG@@ marker, so a
run needs no further input files inside MAME's sandbox.
"""

from __future__ import annotations

from pathlib import Path

from scenario import DIP_FIELDS, Scenario, ScenarioError, compute_input_events

TEMPLATE_PATH = Path(__file__).resolve().parent / "driver_template.lua"
CONFIG_MARKER = "@@CONFIG@@"


def _lua_string(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def _config_lua(s: Scenario, out_dir: str) -> str:
    events = compute_input_events(s)
    lines = ["{"]
    lines.append(f"  scenario_id = {_lua_string(s.scenario_id)},")
    lines.append(f"  rom_set = {_lua_string(s.rom_set)},")
    lines.append(f"  duration_frames = {s.duration_frames},")
    lines.append(f"  stride = {s.screen_downsample_stride},")
    lines.append(f"  ram_crc_every = {s.ram_crc_every_frames},")
    checkpoints = ", ".join(f"[{f}] = true" for f in s.checkpoints)
    lines.append(f"  checkpoints = {{ {checkpoints} }},")
    lines.append(f"  out_dir = {_lua_string(out_dir)},")
    if s.dipswitches_explicit or s.dip_changes:
        lines.append(f"  dsw1 = 0x{s.dsw1:04x},")
        lines.append(f"  dsw2 = 0x{s.dsw2:04x},")
    if s.dip_changes:
        lines.append(f"  final_dsw1 = 0x{s.dip_changes[-1].dsw1:04x},")
        lines.append(f"  final_dsw2 = 0x{s.dip_changes[-1].dsw2:04x},")
        lines.append("  dip_changes = {")
        previous = {"DSW1": s.dsw1, "DSW2": s.dsw2}
        for change in s.dip_changes:
            lines.append(
                f"    {{ frame = {change.frame}, dsw1 = 0x{change.dsw1:04x}, "
                f"dsw2 = 0x{change.dsw2:04x}, changes = {{"
            )
            for spec in DIP_FIELDS:
                old_word = previous[spec.port]
                new_word = change.dsw1 if spec.port == "DSW1" else change.dsw2
                if (old_word & spec.mask) != (new_word & spec.mask):
                    lines.append(
                        f"      {{ port = {_lua_string(spec.port)}, mask = 0x{spec.mask:04x}, "
                        f"value = 0x{new_word & spec.mask:04x} }},"
                    )
            previous = {"DSW1": change.dsw1, "DSW2": change.dsw2}
            lines.append("    } },")
        lines.append("  },")
    lines.append("  input_events = {")
    for ev in events:
        lines.append(
            f"    {{ frame = {ev['frame']}, port = {_lua_string(ev['port'])}, "
            f"mask = 0x{ev['mask']:04x}, state = {ev['state']} }},"
        )
    lines.append("  },")
    lines.append("}")
    return "\n".join(lines)


_FRAME_NUMBER_ANCHOR = "\tlocal fn = screen:frame_number()\n"
_FRAME0_CHECK = (
    "\tlocal fn = screen:frame_number()\n"
    "\tif fn == 0 then\n"
    '\t\tlocal dsw1_f0 = ports[":DSW1"]:read()\n'
    '\t\tlocal dsw2_f0 = ports[":DSW2"]:read()\n'
    "\t\tif dsw1_f0 ~= CONFIG.dsw1 or dsw2_f0 ~= CONFIG.dsw2 then\n"
    "\t\t\tfinished = true\n"
    "\t\t\tframes_file:close()\n"
    "\t\t\tcheckpoints_file:close()\n"
    "\t\t\tmeta_file:close()\n"
    "\t\t\tmanager.machine:exit()\n"
    '\t\t\terror(string.format("frame 0 DIP readback DSW1=%04x DSW2=%04x != requested DSW1=%04x DSW2=%04x",\n'
    "\t\t\t\tdsw1_f0, dsw2_f0, CONFIG.dsw1, CONFIG.dsw2))\n"
    "\t\tend\n"
    "\tend\n"
)

_META_WRITE_ANCHOR = (
    '\t\tmeta_file:write(string.format("dsw1\\t%04x\\n", ports[":DSW1"]:read()))\n'
    '\t\tmeta_file:write(string.format("dsw2\\t%04x\\n", ports[":DSW2"]:read()))\n'
)
_META_WRITE_CHECKED = (
    '\t\tlocal dsw1_end = ports[":DSW1"]:read()\n'
    '\t\tlocal dsw2_end = ports[":DSW2"]:read()\n'
    "\t\tif dsw1_end ~= CONFIG.dsw1 or dsw2_end ~= CONFIG.dsw2 then\n"
    "\t\t\tmeta_file:close()\n"
    "\t\t\tmanager.machine:exit()\n"
    '\t\t\terror(string.format("end DIP readback DSW1=%04x DSW2=%04x != requested DSW1=%04x DSW2=%04x",\n'
    "\t\t\t\tdsw1_end, dsw2_end, CONFIG.dsw1, CONFIG.dsw2))\n"
    "\t\tend\n"
    '\t\tmeta_file:write(string.format("dsw1\\t%04x\\n", dsw1_end))\n'
    '\t\tmeta_file:write(string.format("dsw2\\t%04x\\n", dsw2_end))\n'
)
_META_WRITE_CHECKED_TIMED = _META_WRITE_CHECKED.replace("CONFIG.dsw1", "CONFIG.final_dsw1").replace(
    "CONFIG.dsw2", "CONFIG.final_dsw2"
)

_DIP_HELPER_ANCHOR = "-- CRC32 (IEEE 802.3 polynomial), matching zlib/binascii so capture files can\n"
_DIP_HELPERS = r'''local dip_changes = CONFIG.dip_changes
local next_dip_change = 1
local pending_dip_readbacks = {}
local dip_readbacks_file = assert(io.open(CONFIG.out_dir .. "/dip_readbacks.tsv", "wb"))
dip_readbacks_file:write("change_frame\treadback_frame\tdsw1\tdsw2\n")

local function log_dip_readbacks(fn)
	while #pending_dip_readbacks > 0 and pending_dip_readbacks[1].readback_frame <= fn do
		local pending = table.remove(pending_dip_readbacks, 1)
		dip_readbacks_file:write(string.format("%d\t%d\t%04x\t%04x\n",
			pending.change_frame, fn, ports[":DSW1"]:read(), ports[":DSW2"]:read()))
		dip_readbacks_file:flush()
	end
end

local function apply_dip_changes(fn)
	while next_dip_change <= #dip_changes and dip_changes[next_dip_change].frame <= fn do
		local ev = dip_changes[next_dip_change]
		for _, change in ipairs(ev.changes) do
			local field = resolve_field(change.port, change.mask)
			local value = change.value
			field.user_value = value
		end
		pending_dip_readbacks[#pending_dip_readbacks + 1] = {
			change_frame = ev.frame, readback_frame = fn + 1
		}
		next_dip_change = next_dip_change + 1
	end
end

'''
_DIP_LOG_ANCHOR = "\tif fn >= CONFIG.duration_frames then\n"
_DIP_APPLY_ANCHOR = "\tapply_inputs(fn)\n"
_DIP_CLOSE_ANCHOR = "\t\tframes_file:close()\n\t\tcheckpoints_file:close()\n"


def generate_driver(s: Scenario, out_dir: str) -> str:
    template = TEMPLATE_PATH.read_text(encoding="utf-8")
    if CONFIG_MARKER not in template:
        raise ValueError(f"template {TEMPLATE_PATH} lost its {CONFIG_MARKER} marker")
    rendered = template.replace(CONFIG_MARKER, _config_lua(s, out_dir))
    if s.dipswitches_explicit:
        if _FRAME_NUMBER_ANCHOR not in rendered or _META_WRITE_ANCHOR not in rendered:
            raise ValueError(f"template {TEMPLATE_PATH} lost its DIP readback anchors")
        rendered = rendered.replace(_FRAME_NUMBER_ANCHOR, _FRAME0_CHECK, 1)
        rendered = rendered.replace(_META_WRITE_ANCHOR, _META_WRITE_CHECKED, 1)
    if not s.dip_changes:
        return rendered
    anchors = (
        _DIP_HELPER_ANCHOR,
        _DIP_LOG_ANCHOR,
        _DIP_APPLY_ANCHOR,
        _DIP_CLOSE_ANCHOR,
    )
    if any(anchor not in rendered for anchor in anchors):
        raise ValueError(f"template {TEMPLATE_PATH} lost its timed DIP anchors")
    rendered = rendered.replace(_DIP_HELPER_ANCHOR, _DIP_HELPERS + _DIP_HELPER_ANCHOR, 1)
    rendered = rendered.replace(_DIP_LOG_ANCHOR, "\tlog_dip_readbacks(fn)\n" + _DIP_LOG_ANCHOR, 1)
    rendered = rendered.replace(
        _DIP_APPLY_ANCHOR, "\tapply_dip_changes(fn)\n" + _DIP_APPLY_ANCHOR, 1
    )
    rendered = rendered.replace(
        _DIP_CLOSE_ANCHOR, "\t\tdip_readbacks_file:close()\n" + _DIP_CLOSE_ANCHOR, 1
    )
    if s.dipswitches_explicit:
        rendered = rendered.replace(_META_WRITE_CHECKED, _META_WRITE_CHECKED_TIMED, 1)
    return rendered


def generate_cfg(s: Scenario) -> str:
    """Generate MAME `<machine_set>.cfg` XML applying `s.dsw1` and `s.dsw2` before reset."""
    lines = [
        '<?xml version="1.0"?>',
        '<mameconfig version="10">',
        f'    <system name="{s.machine_set}">',
        "        <input>",
    ]
    for spec in DIP_FIELDS:
        word = s.dsw1 if spec.port == "DSW1" else s.dsw2
        val = word & spec.mask
        lines.append(
            f'            <port tag=":{spec.port}" type="DIPSWITCH" '
            f'mask="{spec.mask}" defvalue="{spec.defvalue}" value="{val}" />'
        )
    lines.extend(
        [
            "        </input>",
            "    </system>",
            "</mameconfig>",
            "",
        ]
    )
    return "\n".join(lines)


def write_cfg(s: Scenario, out_dir: str | Path) -> Path:
    cfg_dir = Path(out_dir) / "cfg"
    cfg_dir.mkdir(parents=True, exist_ok=True)
    cfg_path = cfg_dir / f"{s.machine_set}.cfg"
    cfg_path.write_text(generate_cfg(s), encoding="utf-8")
    return cfg_path


def verify_readback(s: Scenario, meta_path: str | Path) -> None:
    """Verify final port words, including the last scheduled DIP change."""
    path = Path(meta_path)
    if not path.is_file():
        raise ScenarioError(f"meta.tsv missing at {path}")
    kv: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "\t" in line:
            k, v = line.split("\t", 1)
            kv[k.strip()] = v.strip()
    final = s.dip_changes[-1] if s.dip_changes else None
    expected_words = (
        ("dsw1", final.dsw1 if final else s.dsw1),
        ("dsw2", final.dsw2 if final else s.dsw2),
    )
    for key, expected in expected_words:
        actual_str = kv.get(key)
        if actual_str is None:
            raise ScenarioError(f"meta.tsv lacks {key} readback")
        try:
            actual = int(actual_str, 16)
        except ValueError as exc:
            raise ScenarioError(
                f"meta.tsv {key} readback '{actual_str}' is not hex"
            ) from exc
        if actual != expected:
            raise ScenarioError(
                f"meta.tsv {key} readback 0x{actual:04x} does not match requested 0x{expected:04x}"
            )


def verify_dip_readbacks(s: Scenario, readbacks_path: str | Path) -> None:
    """Verify one exact next-frame port readback for every scheduled change."""
    path = Path(readbacks_path)
    if not path.is_file():
        raise ScenarioError(f"dip_readbacks.tsv missing at {path}")
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "change_frame\treadback_frame\tdsw1\tdsw2":
        raise ScenarioError("dip_readbacks.tsv has an invalid header")
    rows = lines[1:]
    if len(rows) != len(s.dip_changes):
        raise ScenarioError(
            f"dip_readbacks.tsv has {len(rows)} rows, expected {len(s.dip_changes)}"
        )
    for change, row in zip(s.dip_changes, rows):
        try:
            change_frame, readback_frame, dsw1, dsw2 = row.split("\t")
            actual = (
                int(change_frame),
                int(readback_frame),
                int(dsw1, 16),
                int(dsw2, 16),
            )
        except ValueError as exc:
            raise ScenarioError(f"invalid dip_readbacks.tsv row {row!r}") from exc
        expected = (change.frame, change.frame + 1, change.dsw1, change.dsw2)
        if actual != expected:
            raise ScenarioError(
                f"DIP readback {actual} does not match requested {expected}"
            )
