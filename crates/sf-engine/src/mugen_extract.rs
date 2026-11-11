//! Offline M.U.G.E.N extraction (`#114`, `M1`).
//!
//! Parses reference `.def`, `.air`, `.cns`, `.cmd`, `.act`, and `.sff` bytes
//! into move tables and pose keyframes. Nothing here runs a `.cns` interpreter,
//! and the fixtures are built in tests rather than committed as character art.

use crate::roster::TournamentFighter;

/// 1987 knockdown and recoil tiles already in the mounted ROM set.
pub const RECOIL_TILES: [u16; 4] = [0x0012, 0x0013, 0x0040, 0x0041];

pub const CPU_CAST: [TournamentFighter; 10] = [
    TournamentFighter::Retsu,
    TournamentFighter::Geki,
    TournamentFighter::Joe,
    TournamentFighter::Mike,
    TournamentFighter::Lee,
    TournamentFighter::Gen,
    TournamentFighter::Birdie,
    TournamentFighter::Eagle,
    TournamentFighter::Adon,
    TournamentFighter::Sagat,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractError {
    pub message: String,
}

impl ExtractError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Box16 {
    pub dx: i32,
    pub dy: i32,
    pub half_w: i32,
    pub half_h: i32,
    pub depth: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RawBox {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl RawBox {
    fn to_center(self, depth: u8) -> Box16 {
        let left = self.x1.min(self.x2);
        let right = self.x1.max(self.x2);
        let bottom = self.y1.min(self.y2);
        let top = self.y1.max(self.y2);
        Box16 {
            dx: left + (right - left) / 2,
            dy: bottom + (top - bottom) / 2,
            half_w: (right - left) / 2,
            half_h: (top - bottom) / 2,
            depth,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimFrame {
    pub group: i32,
    pub image: i32,
    pub x: i32,
    pub y: i32,
    pub time: i32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub hit: Vec<Box16>,
    pub hurt: Vec<Box16>,
    pub hurt_union: Option<Box16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub id: i32,
    pub frames: Vec<AnimFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateMotion {
    pub state: i32,
    pub vel_x: i32,
    pub vel_y: i32,
    pub accel_x: i32,
    pub accel_y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDef {
    pub name: String,
    pub command: String,
    pub time: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefFile {
    pub name: String,
    pub sprite: String,
    pub anim: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    pub group: u16,
    pub image: u16,
    pub width: u16,
    pub height: u16,
    pub axis_x: i16,
    pub axis_y: i16,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoseKeyframe {
    pub action: i32,
    pub group: i32,
    pub image: i32,
    pub axis_x: i32,
    pub axis_y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThrowBind {
    pub fighter: TournamentFighter,
    pub frame: u8,
    pub dx: i16,
    pub dy: i16,
    pub flip_x: bool,
    pub flip_y: bool,
    pub tile: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Light,
    Medium,
    Heavy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limb {
    Punch,
    Kick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Posture {
    Stand,
    Crouch,
    Jump,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalDef {
    pub posture: Posture,
    pub limb: Limb,
    pub strength: Strength,
    pub startup: i32,
    pub active: i32,
    pub recovery: i32,
    pub damage: i32,
}

/// Quantize a MUGEN float velocity into 16.16 fixed point.
pub fn quantize_fixed(value: f64) -> i32 {
    (value * 65536.0).round() as i32
}

pub fn parse_def(text: &str) -> Result<DefFile, ExtractError> {
    let mut name = String::new();
    let mut sprite = String::new();
    let mut anim = String::new();
    for raw in text.lines() {
        let line = strip_comment(raw);
        if let Some((key, value)) = split_kv(line) {
            match key.to_ascii_lowercase().as_str() {
                "name" | "displayname" => name = unquote(value),
                "sprite" => sprite = unquote(value),
                "anim" => anim = unquote(value),
                _ => {}
            }
        }
    }
    if name.is_empty() {
        return Err(ExtractError::new("def has no name"));
    }
    Ok(DefFile { name, sprite, anim })
}

pub fn parse_air(text: &str) -> Result<Vec<Action>, ExtractError> {
    let mut actions = Vec::new();
    let mut current: Option<Action> = None;
    let mut pending_hit: Vec<RawBox> = Vec::new();
    let mut pending_hurt: Vec<RawBox> = Vec::new();
    let mut mode: Option<bool> = None;
    for raw in text.lines() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("[begin action") {
            push_action(&mut actions, current.take());
            let id: i32 = rest
                .trim_matches(|c: char| !c.is_ascii_digit() && c != '-')
                .parse()
                .map_err(|_| ExtractError::new("bad action id"))?;
            current = Some(Action {
                id,
                frames: Vec::new(),
            });
            pending_hit.clear();
            pending_hurt.clear();
            mode = None;
            continue;
        }
        if lower.starts_with("clsn1default") || lower.starts_with("clsn1:") {
            mode = Some(true);
            continue;
        }
        if lower.starts_with("clsn2default") || lower.starts_with("clsn2:") {
            mode = Some(false);
            continue;
        }
        if lower.contains("clsn1[") || lower.contains("clsn2[") {
            let hit = lower.contains("clsn1[");
            let raw_box = parse_box_line(line)?;
            if hit {
                pending_hit.push(raw_box);
            } else {
                pending_hurt.push(raw_box);
            }
            continue;
        }
        if line.contains(',') && current.is_some() && !lower.starts_with("clsn") && mode.is_none()
            || line.contains(',')
                && current.is_some()
                && line
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_digit() || c == '-')
        {
            let frame = parse_frame_line(line, &pending_hit, &pending_hurt)?;
            current.as_mut().unwrap().frames.push(frame);
            pending_hit.clear();
            pending_hurt.clear();
            mode = None;
        }
    }
    push_action(&mut actions, current);
    if actions.is_empty() {
        return Err(ExtractError::new("air has no actions"));
    }
    Ok(actions)
}

fn push_action(actions: &mut Vec<Action>, action: Option<Action>) {
    if let Some(action) = action {
        if !action.frames.is_empty() {
            actions.push(action);
        }
    }
}

fn parse_box_line(line: &str) -> Result<RawBox, ExtractError> {
    let rhs = line.split('=').nth(1).unwrap_or(line);
    let nums = parse_ints(rhs);
    if nums.len() < 4 {
        return Err(ExtractError::new("clsn box needs four numbers"));
    }
    Ok(RawBox {
        x1: nums[0],
        y1: nums[1],
        x2: nums[2],
        y2: nums[3],
    })
}

fn parse_frame_line(
    line: &str,
    hit: &[RawBox],
    hurt: &[RawBox],
) -> Result<AnimFrame, ExtractError> {
    let nums = parse_ints(line);
    if nums.len() < 5 {
        return Err(ExtractError::new(
            "anim element needs group, image, x, y, time",
        ));
    }
    let flip = if nums.len() > 5 { nums[5] } else { 0 };
    let hit_boxes: Vec<Box16> = hit.iter().copied().map(|b| b.to_center(1)).collect();
    let hurt_boxes: Vec<Box16> = hurt.iter().copied().map(|b| b.to_center(2)).collect();
    let hurt_union = union_hurt(hurt);
    Ok(AnimFrame {
        group: nums[0],
        image: nums[1],
        x: nums[2],
        y: nums[3],
        time: nums[4],
        flip_x: flip & 1 != 0,
        flip_y: flip & 2 != 0,
        hit: hit_boxes,
        hurt: hurt_boxes,
        hurt_union,
    })
}

fn union_hurt(hurt: &[RawBox]) -> Option<Box16> {
    if hurt.is_empty() {
        return None;
    }
    let mut acc = hurt[0];
    for box_ in hurt.iter().skip(1) {
        acc.x1 = acc.x1.min(box_.x1.min(box_.x2));
        acc.x2 = acc.x2.max(box_.x1.max(box_.x2));
        acc.y1 = acc.y1.min(box_.y1.min(box_.y2));
        acc.y2 = acc.y2.max(box_.y1.max(box_.y2));
    }
    Some(acc.to_center(2))
}

pub fn parse_cns(text: &str) -> Result<Vec<StateMotion>, ExtractError> {
    let mut states = Vec::new();
    let mut state: Option<i32> = None;
    let mut vel_x = 0i32;
    let mut vel_y = 0i32;
    let mut accel_x = 0i32;
    let mut accel_y = 0i32;
    let mut kind = "";
    let flush = |states: &mut Vec<StateMotion>,
                 state: &mut Option<i32>,
                 vel_x: &mut i32,
                 vel_y: &mut i32,
                 accel_x: &mut i32,
                 accel_y: &mut i32| {
        if let Some(id) = state.take() {
            states.push(StateMotion {
                state: id,
                vel_x: *vel_x,
                vel_y: *vel_y,
                accel_x: *accel_x,
                accel_y: *accel_y,
            });
        }
        *vel_x = 0;
        *vel_y = 0;
        *accel_x = 0;
        *accel_y = 0;
    };
    for raw in text.lines() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("[statedef") {
            flush(
                &mut states,
                &mut state,
                &mut vel_x,
                &mut vel_y,
                &mut accel_x,
                &mut accel_y,
            );
            let id: i32 = rest
                .trim_matches(|c: char| !c.is_ascii_digit() && c != '-')
                .parse()
                .unwrap_or(0);
            state = Some(id);
            kind = "";
            continue;
        }
        if lower.starts_with("[state ") {
            kind = "";
            continue;
        }
        if let Some((key, value)) = split_kv(line) {
            match key.to_ascii_lowercase().as_str() {
                "type" => {
                    kind = if value.eq_ignore_ascii_case("velset") {
                        "vel"
                    } else if value.eq_ignore_ascii_case("veladd") {
                        "add"
                    } else {
                        kind
                    }
                }
                "x" => {
                    let q = quantize_fixed(parse_float(value));
                    if kind == "add" {
                        accel_x = q;
                    } else {
                        vel_x = q;
                    }
                }
                "y" => {
                    let q = quantize_fixed(parse_float(value));
                    if kind == "add" {
                        accel_y = q;
                    } else {
                        vel_y = q;
                    }
                }
                _ => {}
            }
        }
    }
    flush(
        &mut states,
        &mut state,
        &mut vel_x,
        &mut vel_y,
        &mut accel_x,
        &mut accel_y,
    );
    Ok(states)
}

pub fn parse_cmd(text: &str) -> Result<Vec<CommandDef>, ExtractError> {
    let mut out = Vec::new();
    let mut name = String::new();
    let mut command = String::new();
    let mut time = 15i32;
    let mut open = false;
    let flush = |out: &mut Vec<CommandDef>,
                 open: &mut bool,
                 name: &mut String,
                 command: &mut String,
                 time: &mut i32| {
        if *open && !name.is_empty() {
            out.push(CommandDef {
                name: name.clone(),
                command: command.clone(),
                time: *time,
            });
        }
        *open = false;
        name.clear();
        command.clear();
        *time = 15;
    };
    for raw in text.lines() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if line.to_ascii_lowercase().starts_with("[command]") {
            flush(&mut out, &mut open, &mut name, &mut command, &mut time);
            open = true;
            continue;
        }
        if let Some((key, value)) = split_kv(line) {
            match key.to_ascii_lowercase().as_str() {
                "name" => name = unquote(value),
                "command" => command = unquote(value),
                "time" => time = value.parse().unwrap_or(15),
                _ => {}
            }
        }
    }
    flush(&mut out, &mut open, &mut name, &mut command, &mut time);
    Ok(out)
}

pub fn parse_act(bytes: &[u8]) -> Result<[[u8; 3]; 256], ExtractError> {
    let data = if bytes.len() == 768 {
        bytes
    } else if bytes.len() >= 772 {
        &bytes[bytes.len() - 768..]
    } else {
        return Err(ExtractError::new("act palette is not 768 bytes"));
    };
    let mut palette = [[0u8; 3]; 256];
    for (i, slot) in palette.iter_mut().enumerate() {
        slot.copy_from_slice(&data[i * 3..i * 3 + 3]);
    }
    Ok(palette)
}

pub fn parse_sff(bytes: &[u8]) -> Result<Vec<Sprite>, ExtractError> {
    if bytes.len() < 16 || &bytes[0..11] != b"ElecbyteSpr" {
        return Err(ExtractError::new("not an Elecbyte SFF"));
    }
    match bytes.get(12) {
        Some(1) => parse_sff_v1(bytes),
        Some(2) => parse_sff_v2(bytes),
        _ => Err(ExtractError::new("unsupported sff version")),
    }
}

fn parse_sff_v1(bytes: &[u8]) -> Result<Vec<Sprite>, ExtractError> {
    if bytes.len() < 32 {
        return Err(ExtractError::new("sff v1 header is short"));
    }
    let count = u32::from_le_bytes(bytes[20..24].try_into().unwrap()) as usize;
    let mut offset = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
    let mut sprites = Vec::with_capacity(count);
    for _ in 0..count {
        if offset + 32 > bytes.len() {
            return Err(ExtractError::new("sff v1 sprite walks off the file"));
        }
        let next = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let len = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let axis_x = i16::from_le_bytes(bytes[offset + 8..offset + 10].try_into().unwrap());
        let axis_y = i16::from_le_bytes(bytes[offset + 10..offset + 12].try_into().unwrap());
        let group = u16::from_le_bytes(bytes[offset + 12..offset + 14].try_into().unwrap());
        let image = u16::from_le_bytes(bytes[offset + 14..offset + 16].try_into().unwrap());
        let width = u16::from_le_bytes(bytes[offset + 16..offset + 18].try_into().unwrap());
        let height = u16::from_le_bytes(bytes[offset + 18..offset + 20].try_into().unwrap());
        let data = bytes
            .get(offset + 32..offset + 32 + len)
            .ok_or_else(|| ExtractError::new("sff v1 data is short"))?;
        let pixels = if data.first() == Some(&0x0a) {
            decode_pcx(data, width, height)?
        } else {
            data.to_vec()
        };
        sprites.push(Sprite {
            group,
            image,
            width,
            height,
            axis_x,
            axis_y,
            pixels,
        });
        if next == 0 {
            break;
        }
        offset = next;
    }
    Ok(sprites)
}

fn parse_sff_v2(bytes: &[u8]) -> Result<Vec<Sprite>, ExtractError> {
    if bytes.len() < 52 {
        return Err(ExtractError::new("sff v2 header is short"));
    }
    let offset = u32::from_le_bytes(bytes[36..40].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
    let mut sprites = Vec::with_capacity(count);
    for i in 0..count {
        let at = offset + i * 28;
        let node = bytes
            .get(at..at + 28)
            .ok_or_else(|| ExtractError::new("sff v2 node is short"))?;
        let group = u16::from_le_bytes(node[0..2].try_into().unwrap());
        let image = u16::from_le_bytes(node[2..4].try_into().unwrap());
        let width = u16::from_le_bytes(node[4..6].try_into().unwrap());
        let height = u16::from_le_bytes(node[6..8].try_into().unwrap());
        let axis_x = i16::from_le_bytes(node[8..10].try_into().unwrap());
        let axis_y = i16::from_le_bytes(node[10..12].try_into().unwrap());
        let format = node[14];
        let data_off = u32::from_le_bytes(node[16..20].try_into().unwrap()) as usize;
        let data_len = u32::from_le_bytes(node[20..24].try_into().unwrap()) as usize;
        let data = bytes
            .get(data_off..data_off + data_len)
            .ok_or_else(|| ExtractError::new("sff v2 data is short"))?;
        let expected = width as usize * height as usize;
        let pixels = match format {
            0 => data.to_vec(),
            4 => decode_lz5(data, expected)?,
            10 => decode_png_indexed(data)?,
            _ => return Err(ExtractError::new("unsupported sff v2 sprite format")),
        };
        sprites.push(Sprite {
            group,
            image,
            width,
            height,
            axis_x,
            axis_y,
            pixels,
        });
    }
    Ok(sprites)
}

pub fn encode_lz5(src: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < src.len() {
        let chunk = (src.len() - i).min(8);
        out.push(0xff);
        out.extend_from_slice(&src[i..i + chunk]);
        if chunk < 8 {
            out.resize(out.len() + (8 - chunk), 0);
        }
        i += chunk;
    }
    out
}

pub fn decode_lz5(src: &[u8], size: usize) -> Result<Vec<u8>, ExtractError> {
    let mut out = Vec::with_capacity(size);
    let mut i = 0;
    while out.len() < size {
        if i >= src.len() {
            return Err(ExtractError::new("lz5 ran out of input"));
        }
        let ctrl = src[i];
        i += 1;
        for bit in 0..8 {
            if out.len() >= size {
                break;
            }
            if ctrl & (1 << bit) != 0 {
                if i >= src.len() {
                    return Err(ExtractError::new("lz5 literal is missing"));
                }
                out.push(src[i]);
                i += 1;
            } else {
                if i + 1 >= src.len() {
                    return Err(ExtractError::new("lz5 backref is missing"));
                }
                let b0 = src[i] as usize;
                let b1 = src[i + 1] as usize;
                i += 2;
                let length = (b0 & 0x1f) + 3;
                let distance = ((b0 & 0xe0) << 3) | b1;
                if distance == 0 || distance > out.len() {
                    return Err(ExtractError::new("lz5 distance is invalid"));
                }
                for _ in 0..length {
                    if out.len() >= size {
                        break;
                    }
                    let byte = out[out.len() - distance];
                    out.push(byte);
                }
            }
        }
    }
    out.truncate(size);
    Ok(out)
}

pub fn encode_pcx(pixels: &[u8], width: u16, height: u16) -> Vec<u8> {
    let mut out = vec![0u8; 128];
    out[0] = 0x0a;
    out[2] = 1;
    out[3] = 8;
    out[8..10].copy_from_slice(&(width.saturating_sub(1)).to_le_bytes());
    out[10..12].copy_from_slice(&(height.saturating_sub(1)).to_le_bytes());
    out[65] = 1;
    let mut row = 0usize;
    while row < pixels.len() {
        let end = (row + width as usize).min(pixels.len());
        let slice = &pixels[row..end];
        let mut i = 0;
        while i < slice.len() {
            let value = slice[i];
            let mut run = 1usize;
            while i + run < slice.len() && slice[i + run] == value && run < 63 {
                run += 1;
            }
            if run > 1 || value >= 0xc0 {
                out.push(0xc0 | run as u8);
            }
            out.push(value);
            i += run;
        }
        row = end;
    }
    out
}

pub fn decode_pcx(data: &[u8], width: u16, height: u16) -> Result<Vec<u8>, ExtractError> {
    if data.len() < 128 || data[0] != 0x0a {
        return Err(ExtractError::new("not a pcx sprite"));
    }
    let mut out = Vec::with_capacity(width as usize * height as usize);
    let mut i = 128;
    let target = width as usize * height as usize;
    while out.len() < target && i < data.len() {
        let byte = data[i];
        i += 1;
        if byte >= 0xc0 {
            let run = (byte & 0x3f) as usize;
            if i >= data.len() {
                return Err(ExtractError::new("pcx run is missing its value"));
            }
            let value = data[i];
            i += 1;
            for _ in 0..run {
                if out.len() >= target {
                    break;
                }
                out.push(value);
            }
        } else {
            out.push(byte);
        }
    }
    if out.len() != target {
        return Err(ExtractError::new("pcx decoded the wrong number of pixels"));
    }
    Ok(out)
}

fn decode_png_indexed(data: &[u8]) -> Result<Vec<u8>, ExtractError> {
    if data.len() < 8 || &data[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(ExtractError::new("not a png sprite"));
    }
    let mut i = 8;
    let mut width = 0usize;
    let mut height = 0usize;
    let mut idat = Vec::new();
    while i + 8 <= data.len() {
        let len = u32::from_be_bytes(data[i..i + 4].try_into().unwrap()) as usize;
        let kind = &data[i + 4..i + 8];
        let start = i + 8;
        let end = start + len;
        if end + 4 > data.len() {
            return Err(ExtractError::new("png chunk is short"));
        }
        if kind == b"IHDR" {
            width = u32::from_be_bytes(data[start..start + 4].try_into().unwrap()) as usize;
            height = u32::from_be_bytes(data[start + 4..start + 8].try_into().unwrap()) as usize;
        } else if kind == b"IDAT" {
            idat.extend_from_slice(&data[start..end]);
        } else if kind == b"IEND" {
            break;
        }
        i = end + 4;
    }
    let raw = inflate_stored(&idat)?;
    let mut pixels = Vec::with_capacity(width * height);
    let stride = width + 1;
    if raw.len() < stride * height {
        return Err(ExtractError::new("png image is short"));
    }
    for y in 0..height {
        let row = &raw[y * stride..y * stride + stride];
        if row[0] != 0 {
            return Err(ExtractError::new("png filter is not None"));
        }
        pixels.extend_from_slice(&row[1..]);
    }
    Ok(pixels)
}

pub fn encode_png_indexed(pixels: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut raw = Vec::new();
    let w = width as usize;
    for y in 0..height as usize {
        raw.push(0);
        raw.extend_from_slice(&pixels[y * w..(y + 1) * w]);
    }
    let zlib = deflate_stored(&raw);
    let mut png = Vec::new();
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 3, 0, 0, 0]);
    write_chunk(&mut png, b"IHDR", &ihdr);
    let mut plte = Vec::new();
    for i in 0..16 {
        plte.extend_from_slice(&[i * 16, i * 8, 255 - i * 8]);
    }
    write_chunk(&mut png, b"PLTE", &plte);
    write_chunk(&mut png, b"IDAT", &zlib);
    write_chunk(&mut png, b"IEND", &[]);
    png
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_bytes = Vec::with_capacity(4 + data.len());
    crc_bytes.extend_from_slice(kind);
    crc_bytes.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_bytes).to_be_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = if crc & 1 == 1 { 0xedb8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}

fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    out.push(0x01);
    let len = data.len() as u16;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(data);
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn inflate_stored(zlib: &[u8]) -> Result<Vec<u8>, ExtractError> {
    if zlib.len() < 8 || zlib[0] != 0x78 {
        return Err(ExtractError::new("zlib header is missing"));
    }
    if zlib[2] != 0x01 {
        return Err(ExtractError::new("only stored deflate blocks are accepted"));
    }
    let len = u16::from_le_bytes(zlib[3..5].try_into().unwrap()) as usize;
    let nlen = u16::from_le_bytes(zlib[5..7].try_into().unwrap());
    if nlen != !u16::try_from(len).unwrap_or(0) && len < 65536 {
        let expect = !(len as u16);
        if nlen != expect {
            return Err(ExtractError::new("deflate len check failed"));
        }
    }
    let start = 7;
    let end = start + len;
    if end + 4 > zlib.len() {
        return Err(ExtractError::new("stored deflate block is short"));
    }
    Ok(zlib[start..end].to_vec())
}

fn adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for byte in data {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

pub fn bridge_normals(source: &[NormalDef], rom_medium: &[NormalDef]) -> Vec<NormalDef> {
    let mut out = Vec::new();
    for posture in [Posture::Stand, Posture::Crouch, Posture::Jump] {
        for limb in [Limb::Punch, Limb::Kick] {
            let light = find_normal(source, posture, limb, Strength::Light);
            let heavy = find_normal(source, posture, limb, Strength::Heavy);
            let medium_src = find_normal(source, posture, limb, Strength::Medium);
            let medium = medium_src
                .or_else(|| find_normal(rom_medium, posture, limb, Strength::Medium))
                .or_else(|| interpolate(light, heavy, posture, limb));
            if let Some(row) = light {
                out.push(row);
            }
            if let Some(row) = medium {
                out.push(row);
            }
            if let Some(row) = heavy {
                out.push(row);
            }
        }
    }
    out
}

fn find_normal(
    rows: &[NormalDef],
    posture: Posture,
    limb: Limb,
    strength: Strength,
) -> Option<NormalDef> {
    rows.iter()
        .copied()
        .find(|row| row.posture == posture && row.limb == limb && row.strength == strength)
}

fn interpolate(
    light: Option<NormalDef>,
    heavy: Option<NormalDef>,
    posture: Posture,
    limb: Limb,
) -> Option<NormalDef> {
    match (light, heavy) {
        (Some(light), Some(heavy)) => Some(NormalDef {
            posture,
            limb,
            strength: Strength::Medium,
            startup: (light.startup + heavy.startup) / 2,
            active: (light.active + heavy.active) / 2,
            recovery: (light.recovery + heavy.recovery) / 2,
            damage: (light.damage + heavy.damage) / 2,
        }),
        _ => None,
    }
}

pub fn pose_keyframes(actions: &[Action], sprites: &[Sprite]) -> Vec<PoseKeyframe> {
    let mut keys = Vec::new();
    for action in actions {
        for frame in &action.frames {
            let sprite = sprites
                .iter()
                .find(|s| i32::from(s.group) == frame.group && i32::from(s.image) == frame.image);
            let (axis_x, axis_y) = if let Some(sprite) = sprite {
                (
                    i32::from(sprite.axis_x),
                    i32::from(sprite.height) - 1 - i32::from(sprite.axis_y),
                )
            } else {
                (frame.x, frame.y)
            };
            keys.push(PoseKeyframe {
                action: action.id,
                group: frame.group,
                image: frame.image,
                axis_x,
                axis_y,
            });
        }
    }
    keys
}

pub fn missing_states(actions: &[Action]) -> Vec<&'static str> {
    let has = |id: i32| actions.iter().any(|a| a.id == id);
    let mut missing = Vec::new();
    if !has(10) && !has(11) && !has(12) {
        missing.push("crouch");
    }
    if !has(40) && !has(41) && !has(42) {
        missing.push("jump");
    }
    if !has(130) && !has(131) && !has(140) {
        missing.push("block");
    }
    if !has(800) && !has(820) {
        missing.push("throw");
    }
    missing
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareTile {
    pub pixels: [u8; 256],
    pub palette: [u16; 16],
}

pub fn validate_hardware_tile(tile: &HardwareTile) -> Result<(), ExtractError> {
    if tile.pixels.iter().any(|pen| *pen > 15) {
        return Err(ExtractError::new("pen is outside 0..=15"));
    }
    if tile.palette.iter().any(|rgb| rgb & 0xf000 != 0) {
        return Err(ExtractError::new("palette entry is not xRGB_444"));
    }
    Ok(())
}

pub fn hardware_tile_from_indexed(pixels: &[u8; 256], rgba: &[[u8; 4]; 16]) -> HardwareTile {
    let mut pens = [15u8; 256];
    let mut palette = [0u16; 16];
    palette[15] = 0;
    for (i, pen) in pixels.iter().enumerate() {
        if *pen == 0 {
            pens[i] = 15;
            continue;
        }
        let slot = (*pen).min(14);
        pens[i] = slot;
        let px = rgba[slot as usize];
        palette[slot as usize] =
            (u16::from(px[0] >> 4) << 8) | (u16::from(px[1] >> 4) << 4) | u16::from(px[2] >> 4);
    }
    HardwareTile {
        pixels: pens,
        palette,
    }
}

pub fn throw_binds(fighter: TournamentFighter, action: &Action) -> Vec<ThrowBind> {
    action
        .frames
        .iter()
        .enumerate()
        .map(|(frame, element)| ThrowBind {
            fighter,
            frame: frame as u8,
            dx: element.x as i16,
            dy: element.y as i16,
            flip_x: element.flip_x,
            flip_y: element.flip_y,
            tile: RECOIL_TILES[frame % RECOIL_TILES.len()],
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThrowFrame {
    pub fighter: TournamentFighter,
    pub tile: u16,
    pub dx: i16,
    pub dy: i16,
    pub flip_x: bool,
    pub flip_y: bool,
}

/// Step a throw playback. Every tile id comes from `RECOIL_TILES`.
pub fn step_throw(binds: &[ThrowBind]) -> Vec<ThrowFrame> {
    binds
        .iter()
        .map(|bind| ThrowFrame {
            fighter: bind.fighter,
            tile: RECOIL_TILES[RECOIL_TILES
                .iter()
                .position(|id| *id == bind.tile)
                .unwrap_or(0)],
            dx: bind.dx,
            dy: bind.dy,
            flip_x: bind.flip_x,
            flip_y: bind.flip_y,
        })
        .collect()
}

fn strip_comment(line: &str) -> &str {
    line.split(';').next().unwrap_or(line)
}

fn split_kv(line: &str) -> Option<(&str, &str)> {
    let (k, v) = line.split_once('=')?;
    Some((k.trim(), v.trim()))
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').trim().to_string()
}

fn parse_ints(text: &str) -> Vec<i32> {
    let mut nums = Vec::new();
    let mut cur = String::new();
    for ch in text.chars() {
        if ch == '-' || ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            if let Ok(n) = cur.parse() {
                nums.push(n);
            }
            cur.clear();
        }
    }
    if let Ok(n) = cur.parse() {
        nums.push(n);
    }
    nums
}

fn parse_float(text: &str) -> f64 {
    let token = text
        .split_whitespace()
        .next()
        .unwrap_or("0")
        .trim_matches(',');
    token.parse().unwrap_or(0.0)
}

pub fn write_sff_v1(sprites: &[Sprite]) -> Vec<u8> {
    let mut out = vec![0u8; 512];
    out[..11].copy_from_slice(b"ElecbyteSpr");
    out[12] = 1;
    out[20..24].copy_from_slice(&(sprites.len() as u32).to_le_bytes());
    out[24..28].copy_from_slice(&512u32.to_le_bytes());
    for (i, sprite) in sprites.iter().enumerate() {
        let pcx = encode_pcx(&sprite.pixels, sprite.width, sprite.height);
        let at = out.len();
        let next = if i + 1 == sprites.len() {
            0
        } else {
            (at + 32 + pcx.len()) as u32
        };
        out.extend_from_slice(&next.to_le_bytes());
        out.extend_from_slice(&(pcx.len() as u32).to_le_bytes());
        out.extend_from_slice(&sprite.axis_x.to_le_bytes());
        out.extend_from_slice(&sprite.axis_y.to_le_bytes());
        out.extend_from_slice(&sprite.group.to_le_bytes());
        out.extend_from_slice(&sprite.image.to_le_bytes());
        out.extend_from_slice(&sprite.width.to_le_bytes());
        out.extend_from_slice(&sprite.height.to_le_bytes());
        out.extend_from_slice(&[0u8; 12]);
        out.extend_from_slice(&pcx);
    }
    out
}

pub fn write_sff_v2(sprite: &Sprite, format: u8, payload: &[u8]) -> Vec<u8> {
    let nodes = 52usize;
    let data_off = nodes + 28;
    let mut out = vec![0u8; data_off + payload.len()];
    out[..11].copy_from_slice(b"ElecbyteSpr");
    out[12] = 2;
    out[36..40].copy_from_slice(&(nodes as u32).to_le_bytes());
    out[40..44].copy_from_slice(&1u32.to_le_bytes());
    let node = &mut out[nodes..nodes + 28];
    node[0..2].copy_from_slice(&sprite.group.to_le_bytes());
    node[2..4].copy_from_slice(&sprite.image.to_le_bytes());
    node[4..6].copy_from_slice(&sprite.width.to_le_bytes());
    node[6..8].copy_from_slice(&sprite.height.to_le_bytes());
    node[8..10].copy_from_slice(&sprite.axis_x.to_le_bytes());
    node[10..12].copy_from_slice(&sprite.axis_y.to_le_bytes());
    node[14] = format;
    node[16..20].copy_from_slice(&(data_off as u32).to_le_bytes());
    node[20..24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    out[data_off..].copy_from_slice(payload);
    out
}
