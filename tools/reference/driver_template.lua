-- Reference scenario driver for the pinned MAME baseline (US set 1, "sf").
-- Generated runs embed a CONFIG table in place of the config marker below.
-- Memory map constants are documented from src/mame/capcom/sf.cpp @ mame0289:
--   main 68000 space: 0xff8000-0xffdfff work RAM, 0xffe000-0xffffff object RAM,
--   0x800000-0x800fff video RAM, 0xb00000-0xb007ff palette RAM,
--   0xc0001d sound command latch (write-only from the main CPU).
--   audio Z80 ("audiocpu") space: 0xc000-0xc7ff RAM, 0xc800 latch read.
local CONFIG = @@CONFIG@@

local WORK_RAM_START = 0xff8000
local WORK_RAM_END = 0xffffff -- includes object RAM, one contiguous 32 KiB range
local VIDEO_RAM_START = 0x800000
local VIDEO_RAM_END = 0x800fff
local PALETTE_RAM_START = 0xb00000
local PALETTE_RAM_END = 0xb007ff
local MUSIC_RAM_START = 0xc000
local MUSIC_RAM_END = 0xc7ff
local SOUND_LATCH_ADDR = 0xc800

local screen = manager.machine.screens[":screen"]
local main_space = manager.machine.devices[":maincpu"].spaces["program"]
local audio_space = manager.machine.devices[":audiocpu"].spaces["program"]
local ports = manager.machine.ioport.ports

-- Resolve each referenced (port, mask) pair to an ioport field. Field-level
-- set_value/clear_value is the injection path MAME's input system honors;
-- writing the port's live value directly gets overwritten before the game
-- reads it (observed: injected coin/start had no effect in calibration).
local field_cache = {}
local function resolve_field(port, mask)
	local key = port .. ":" .. mask
	if field_cache[key] == nil then
		local found = nil
		for name, field in pairs(ports[":" .. port].fields) do
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

-- CRC32 (IEEE 802.3 polynomial), matching zlib/binascii so capture files can
-- be re-checked from Python.
local crc_table = {}
for b = 0, 255 do
	local c = b
	for _ = 1, 8 do
		if c & 1 ~= 0 then c = 0xEDB88320 ~ (c >> 1) else c = c >> 1 end
	end
	crc_table[b] = c
end
local function crc32_update(crc, byte)
	return crc_table[(crc ~ byte) & 0xFF] ~ (crc >> 8)
end
local function crc32_string(buf)
	local c = 0xFFFFFFFF
	for i = 1, #buf do
		c = crc32_update(c, string.byte(buf, i))
	end
	return (~c) & 0xFFFFFFFF
end

local function read_range_string(space, first, last)
	local bytes = {}
	for a = first, last do
		bytes[#bytes + 1] = string.char(space:read_u8(a))
	end
	return table.concat(bytes)
end

local function crc32_range(space, first, last)
	local c = 0xFFFFFFFF
	for a = first, last do
		c = crc32_update(c, space:read_u8(a))
	end
	return (~c) & 0xFFFFFFFF
end

local frames_file = assert(io.open(CONFIG.out_dir .. "/frames.tsv", "wb"))
local checkpoints_file = assert(io.open(CONFIG.out_dir .. "/checkpoints.tsv", "wb"))
local meta_file = assert(io.open(CONFIG.out_dir .. "/meta.tsv", "wb"))

frames_file:write("frame\tscreen_hash\tlatch\tlatch_changed\tram_crc\tmusic_crc\n")
checkpoints_file:write("frame\tscreen_crc32\tpixels_file\n")

local line_buffer = {}
local function log_frame(line)
	line_buffer[#line_buffer + 1] = line
	if #line_buffer >= 120 then
		frames_file:write(table.concat(line_buffer))
		line_buffer = {}
	end
end

local function dump_checkpoint(fn)
	local pixels = manager.machine.video:snapshot_pixels()
	local name = string.format("pixels_%06d.rgba", fn)
	local f = assert(io.open(CONFIG.out_dir .. "/" .. name, "wb"))
	f:write(pixels)
	f:close()
	local screen_crc = crc32_string(pixels)
	checkpoints_file:write(string.format("%d\t%08x\t%s\n", fn, screen_crc, name))
	checkpoints_file:flush()

	local regions = {
		{ "work", main_space, WORK_RAM_START, WORK_RAM_END },
		{ "video", main_space, VIDEO_RAM_START, VIDEO_RAM_END },
		{ "palette", main_space, PALETTE_RAM_START, PALETTE_RAM_END },
		{ "music", audio_space, MUSIC_RAM_START, MUSIC_RAM_END },
	}
	for _, region in ipairs(regions) do
		local label, space, first, last = region[1], region[2], region[3], region[4]
		local dump = read_range_string(space, first, last)
		local rf = assert(io.open(
			CONFIG.out_dir .. "/" .. string.format("ram_%06d_%s.bin", fn, label), "wb"))
		rf:write(dump)
		rf:close()
	end
end

local prev_latch = nil
local frames_seen = 0
local finished = false

emu.register_frame_done(function()
	if finished then
		return
	end
	local fn = screen:frame_number()
	if fn >= CONFIG.duration_frames then
		finished = true
		if #line_buffer > 0 then
			frames_file:write(table.concat(line_buffer))
			line_buffer = {}
		end
		frames_file:close()
		checkpoints_file:close()
		meta_file:write(string.format("dsw1\t%04x\n", ports[":DSW1"]:read()))
		meta_file:write(string.format("dsw2\t%04x\n", ports[":DSW2"]:read()))
		meta_file:write(string.format("screen_width\t%d\n", screen.width))
		meta_file:write(string.format("screen_height\t%d\n", screen.height))
		meta_file:write(string.format("frames_executed\t%d\n", frames_seen))
		meta_file:close()
		print(string.format("[driver] finished at frame %d", fn))
		manager.machine:exit()
		return
	end

	apply_inputs(fn)

	local stride = CONFIG.stride
	local acc = 0x811c9dc5
	for y = 0, 223, stride do
		for x = 0, 383, stride do
			acc = (acc ~ screen:pixel(x, y)) & 0xFFFFFFFF
			acc = (acc * 0x01000193) & 0xFFFFFFFF
		end
	end

	-- readv is a no-side-effect (debugger-style) read: polling the latch must
	-- not consume a pending command.
	local latch = audio_space:readv_u8(SOUND_LATCH_ADDR)
	local latch_changed = (prev_latch ~= nil and latch ~= prev_latch) and 1 or 0
	prev_latch = latch

	local ram_crc = "-"
	local music_crc = "-"
	if fn % CONFIG.ram_crc_every == 0 then
		ram_crc = string.format("%08x", crc32_range(main_space, WORK_RAM_START, WORK_RAM_END))
		music_crc = string.format("%08x", crc32_range(audio_space, MUSIC_RAM_START, MUSIC_RAM_END))
	end

	log_frame(string.format("%d\t%08x\t%02x\t%d\t%s\t%s\n",
		fn, acc, latch, latch_changed, ram_crc, music_crc))

	if CONFIG.checkpoints[fn] then
		dump_checkpoint(fn)
	end

	frames_seen = frames_seen + 1
	if fn % 600 == 0 then
		print(string.format("[driver] frame %d / %d", fn, CONFIG.duration_frames))
	end
end)

print(string.format("[driver] scenario %s armed for %d frames",
	CONFIG.scenario_id, CONFIG.duration_frames))
