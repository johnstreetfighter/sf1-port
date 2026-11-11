"""Audio observation run: a scenario on the pinned MAME with sound-chip
write taps, optional per-device output streams, and a WAV recording
(tickets #10/#11; docs/reference/hit-sound.md, docs/reference/music-phrase.md).

    python run_audio_probe.py scenarios/<name>.json --out local/captures/<dir> \
        [--no-wav] [--hook-window F0 F1] [--hook-devices ymsnd,msm1,msm2] \n        [--code-trace T0 T1] [--m68k-trace T0 T1 ...]

Appends write taps to the generated driver: the main CPU's sound-latch
write (0xc0001c-d), the samples CPU's I/O writes (MSM5205 ports 0/1, bank
port 2), and the music CPU's YM2151 writes (0xe000-0xe001), each logged with
the machine time (seconds) and the screen frame number to DIR/audio_taps.tsv.
MAME's mixed output goes to DIR/audio.wav (-wavwrite; -sound none keeps the
OSD silent, mixing still runs). With --hook-window, the MSM5205 and YM2151
device output streams (MAME's Lua sound hook: msm1/msm2 at 384 kHz, ymsnd
at its native rate, first channel) for frames F0..F1 go to
DIR/hook_<device>.f32 (little-endian float32) with DIR/hook_index.tsv;
--hook-devices restricts the hooked devices (default all three; a whole
run's YM2151 stream alone stays small).
The generated driver keeps the scenario's inputs and observations, so the
run's capture files are produced as usual. With --code-trace, every
program-space read of the samples CPU's code (0x0000-0x01b0: opcode and
operand fetches) and every read of its latch port (I/O port 1) between the
machine times T0 and T1 (seconds) go to DIR/code_reads.tsv with the machine
time: an instruction-level trace of the samples CPU (ticket #32,
docs/reference/fight-samples.md). With --m68k-trace (repeatable), every
program-space read of the main CPU's vblank handler and its subroutines
(0x3b052-0x3b4ff) inside each machine-time window T0..T1 goes to
DIR/m68k_reads.tsv with the machine time and frame: the handler's
instruction-level trace, analysed by vblank_paths.py (ticket #35,
docs/reference/latch-timing.md).
"""
import argparse, subprocess, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from lua_gen import (  # noqa: E402
    generate_driver,
    verify_dip_readbacks,
    verify_readback,
    write_cfg,
)
from scenario import ScenarioError, load_scenario  # noqa: E402

TAPS = r'''
-- audio probe taps (appended)
audio_probe_out = assert(io.open("%(out)s/audio_taps.tsv", "wb"))
audio_probe_out:write("time\tframe\tsource\taddr\tdata\n")
audio_probe_taps = {}
local function atap(space, first, last, label)
	local scr = manager.machine.screens[":screen"]
	local t = space:install_write_tap(first, last, label, function(offset, data, mask)
		audio_probe_out:write(string.format("%%.9f\t%%d\t%%s\t%%x\t%%x\n",
			manager.machine.time:as_double(), scr:frame_number(), label, offset, data))
	end)
	audio_probe_taps[#audio_probe_taps + 1] = t
end
atap(manager.machine.devices[":maincpu"].spaces["program"], 0xc0001c, 0xc0001d, "latch")
atap(manager.machine.devices[":audio2"].spaces["io"], 0x00, 0x02, "s2io")
atap(manager.machine.devices[":audiocpu"].spaces["program"], 0xe000, 0xe001, "opm")
'''

CODE_TRACE = r'''
-- samples-CPU instruction trace (appended): every program-space read of the
-- code (opcode and operand fetches, 0x0000-0x01b0) and every latch read
-- (I/O port 1) between the machine times T0 and T1, with the machine time.
code_tap_out = assert(io.open("%(out)s/code_reads.tsv", "wb"))
code_tap_out:write("time	source	addr	data
")
local a2prog = manager.machine.devices[":audio2"].spaces["program"]
code_tap = a2prog:install_read_tap(0x0000, 0x01b0, "code", function(offset, data, mask)
	local tm = manager.machine.time:as_double()
	if tm < %(t0)s or tm > %(t1)s then return end
	code_tap_out:write(string.format("%%.9f	code	%%x	%%x
", tm, offset, data))
end)
local a2io = manager.machine.devices[":audio2"].spaces["io"]
ioread_tap = a2io:install_read_tap(0x01, 0x01, "latchread", function(offset, data, mask)
	local tm = manager.machine.time:as_double()
	if tm < %(t0)s or tm > %(t1)s then return end
	code_tap_out:write(string.format("%%.9f	in	%%x	%%x
", tm, offset, data))
end)
'''

HOOK = r'''
-- per-device output streams over a frame window (appended)
audio_hook_files = {}
for tag, dev in pairs(manager.machine.sounds) do
	if %(devices)s then
		dev.hook = true
		audio_hook_files[tag] = assert(io.open("%(out)s/hook_" .. tag:sub(2) .. ".f32", "wb"))
	end
end
audio_hook_index = assert(io.open("%(out)s/hook_index.tsv", "wb"))
audio_hook_index:write("frame\ttime\tdevice\tchannel\tcount\n")
emu.register_sound_update(function(t)
	local scr = manager.machine.screens[":screen"]
	local fn = scr:frame_number()
	if fn < %(f0)d or fn > %(f1)d then return end
	for tag, chans in pairs(t) do
		local f = audio_hook_files[tag]
		if f then
			for ch, buf in ipairs(chans) do
				if ch == 1 then
					local parts = {}
					for i = 1, #buf do parts[i] = string.pack("<f", buf[i]) end
					f:write(table.concat(parts))
					audio_hook_index:write(string.format("%%d\t%%.9f\t%%s\t%%d\t%%d\n", fn, manager.machine.time:as_double(), tag, ch, #buf))
				end
			end
		end
	end
end)
'''

M68K_TRACE = r"""
-- 68000 vblank-handler trace (appended): every program-space read of the
-- handler and its subroutines (0x3b052-0x3b4ff: opcode and operand
-- fetches, the latch-strength tables) inside the listed machine-time
-- windows, with the machine time and the screen frame number.
m68k_tap_out = assert(io.open("%(out)s/m68k_reads.tsv", "wb"))
m68k_tap_out:write("time\tframe\taddr\tdata\n")
local m68k_windows = {%(windows)s}
local mprog = manager.machine.devices[":maincpu"].spaces["program"]
local mscr = manager.machine.screens[":screen"]
m68k_tap = mprog:install_read_tap(0x3b052, 0x3b4ff, "m68k", function(offset, data, mask)
	local tm = manager.machine.time:as_double()
	for _, w in ipairs(m68k_windows) do
		if tm >= w[1] and tm <= w[2] then
			m68k_tap_out:write(string.format("%%.9f\t%%d\t%%x\t%%x\n", tm, mscr:frame_number(), offset, data))
			return
		end
	end
end)
"""

def main(argv):
    ap = argparse.ArgumentParser()
    ap.add_argument("scenario"); ap.add_argument("--out", required=True)
    ap.add_argument("--no-wav", action="store_true")
    ap.add_argument("--hook-devices", default="ymsnd,msm1,msm2",
                    help="comma-separated devices to hook (ymsnd, msm1, msm2); default all")
    ap.add_argument("--hook-window", nargs=2, type=int, metavar=("F0", "F1"),
                    help="record the MSM5205 / YM2151 output streams for frames F0..F1")
    ap.add_argument("--code-trace", nargs=2, type=float, metavar=("T0", "T1"),
                    help="trace the samples CPU's code fetches and latch reads between machine times T0..T1 (s)")
    ap.add_argument("--m68k-trace", nargs=2, type=float, action="append", metavar=("T0", "T1"),
                    help="trace the main CPU's vblank handler (program reads 0x3b052-0x3b4ff) between machine times T0..T1 (s); repeatable")
    a = ap.parse_args(argv)
    s = load_scenario(a.scenario)
    out = Path(a.out).resolve(); out.mkdir(parents=True, exist_ok=True)
    drv = out / "generated_driver.lua"
    lua = generate_driver(s, out.as_posix()) + TAPS % {"out": out.as_posix()}
    if a.hook_window:
        devices = " or ".join('tag == ":%s"' % d.strip() for d in a.hook_devices.split(",") if d.strip())
        lua += HOOK % {"out": out.as_posix(), "f0": a.hook_window[0], "f1": a.hook_window[1], "devices": devices}
    if a.code_trace:
        lua += CODE_TRACE % {"out": out.as_posix(), "t0": repr(a.code_trace[0]), "t1": repr(a.code_trace[1])}
    if a.m68k_trace:
        windows = ", ".join("{%r, %r}" % (t0, t1) for t0, t1 in a.m68k_trace)
        lua += M68K_TRACE % {"out": out.as_posix(), "windows": windows}
    drv.write_text(lua, encoding="utf-8")
    local = HERE / "local"
    cmd = [str(local / "mame" / "mame.exe"), s.machine_set, "-rompath", str(local / "roms"),
           "-video", "none", "-sound", "none", "-nothrottle", "-skip_gameinfo", "-noreadconfig", "-nowriteconfig",
           "-cfg_directory", str(out / "cfg"), "-snapshot_directory", str(out / "snap"),
           "-autoboot_script", str(drv), "-autoboot_delay", "0",
           "-seconds_to_run", str(int(s.duration_frames / 60 * 3) + 30)]
    if s.dipswitches_explicit or s.dip_changes:
        write_cfg(s, out)
        cmd.remove("-noreadconfig")
    if not a.no_wav:
        cmd += ["-wavwrite", str(out / "audio.wav")]
    p = subprocess.run(cmd, capture_output=True, text=True)
    sys.stdout.write(p.stdout[-400:]); sys.stderr.write(p.stderr[-2000:])
    if p.returncode != 0:
        return p.returncode
    try:
        verify_readback(s, out / "meta.tsv")
        if s.dip_changes:
            verify_dip_readbacks(s, out / "dip_readbacks.tsv")
    except ScenarioError as exc:
        print(f"[audio-probe] DIP readback verification failed: {exc}")
        return 1
    return 0

if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
