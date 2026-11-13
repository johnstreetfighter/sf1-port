//! The campaign route, fighter select and RPG controls over the 1987 travel map.

use std::sync::Arc;

use sf_video::busts::Bust;
use sf_video::compositor::{ink, outline, RomFont};
use sf_video::render::{render_frame, Frame, WIDTH};
use sf_video::romset::Assets;
use sf_video::scene::SceneState;

use super::draw::{boxed, dimmed, fit};
use super::select::SelectScreen;
use super::Transition;
use crate::config::MatchConfig;
use crate::era::{EraFamily, Loadout};
use crate::roster::{MenuInput, TournamentFighter};
use crate::world_tour::native::{
    map, MissionKind, Perk, RuleModifier, Stat, WorldTourProfile, STAT_COST,
};

// The four-node clusters follow the country dots on the ROM map. The last two
// markers follow the Thailand dot south of the China and Japan routes.
const MARKERS: [(i32, i32); 18] = [
    (181, 93),
    (196, 112),
    (205, 73),
    (223, 93),
    (276, 87),
    (290, 67),
    (290, 109),
    (310, 88),
    (106, 80),
    (124, 61),
    (124, 99),
    (141, 83),
    (43, 67),
    (27, 84),
    (54, 83),
    (41, 105),
    (150, 115),
    (175, 128),
];

const GOLD: u32 = 0xffff_d873;
const WHITE: u32 = 0xffff_ffff;
const DIM: u32 = 0xff92_9292;
const SHADOW: u32 = 0xff29_2929;

#[derive(Clone)]
enum Overlay {
    Map,
    Select(Box<SelectScreen>),
    Rpg(usize),
}

#[derive(Clone)]
pub struct WorldTourScreen {
    backdrop: Arc<Frame>,
    overlay: Overlay,
}

impl WorldTourScreen {
    pub fn open(assets: &Assets) -> Self {
        Self {
            backdrop: Arc::new(travel_map(assets)),
            overlay: Overlay::Map,
        }
    }

    pub(super) fn name(&self) -> &'static str {
        match self.overlay {
            Overlay::Map => "world tour",
            Overlay::Select(_) => "world tour fighter select",
            Overlay::Rpg(_) => "world tour rpg",
        }
    }

    pub(super) fn selecting_fighter(&self) -> bool {
        matches!(self.overlay, Overlay::Select(_))
    }

    pub(super) fn open_select(
        &mut self,
        config: &MatchConfig,
        player_tables: &[TournamentFighter],
        profile: &WorldTourProfile,
    ) {
        let mut select = SelectScreen::arcade(config, player_tables)
            .with_preferred_eras([profile.style.family, EraFamily::Sf1Plus]);
        select.focus(profile.fighter);
        self.overlay = Overlay::Select(Box::new(select));
    }

    /// The second return value indicates a profile change that needs saving.
    pub(super) fn apply(
        &mut self,
        input: MenuInput,
        profile: &mut WorldTourProfile,
    ) -> (Option<Transition>, bool) {
        match &mut self.overlay {
            Overlay::Map => match input {
                MenuInput::Left | MenuInput::Up => move_node(profile, -1),
                MenuInput::Right | MenuInput::Down => move_node(profile, 1),
                MenuInput::Confirm | MenuInput::Start => (Some(Transition::StartWorldTour), false),
                MenuInput::Back => (Some(Transition::Back), false),
                MenuInput::TabLeft => (Some(Transition::WorldTourSelect), false),
                MenuInput::TabRight => {
                    self.overlay = Overlay::Rpg(0);
                    (Some(Transition::CursorMoved), false)
                }
                MenuInput::Filter => (None, false),
            },
            Overlay::Select(select) => match select.apply(input) {
                Some(Transition::StyleOpened) => {
                    if select.fighter(0) == profile.fighter {
                        select.restore_open_style(profile.style);
                    }
                    (Some(Transition::StyleOpened), false)
                }
                Some(Transition::Fighter(fighter)) => {
                    let style = select.locked_style(0).unwrap_or_default();
                    let changed = (profile.fighter, profile.style) != (fighter, style);
                    (profile.fighter, profile.style) = (fighter, style);
                    self.overlay = Overlay::Map;
                    (Some(Transition::ValueChanged), changed)
                }
                Some(Transition::Back) => {
                    self.overlay = Overlay::Map;
                    (Some(Transition::CursorMoved), false)
                }
                transition => (transition, false),
            },
            Overlay::Rpg(row) => match input {
                MenuInput::Back | MenuInput::TabRight => {
                    self.overlay = Overlay::Map;
                    (Some(Transition::CursorMoved), false)
                }
                MenuInput::Up | MenuInput::Down => {
                    let delta = if input == MenuInput::Up { -1 } else { 1 };
                    *row = (*row as i32 + delta).rem_euclid(5) as usize;
                    (Some(Transition::CursorMoved), false)
                }
                MenuInput::Left | MenuInput::Right | MenuInput::Confirm | MenuInput::Start => {
                    let next = input != MenuInput::Left;
                    let changed = match *row {
                        0 => {
                            profile.rpg = !profile.rpg;
                            true
                        }
                        1 => profile.adjust(Stat::Attack, next),
                        2 => profile.adjust(Stat::Defense, next),
                        3 => profile.adjust(Stat::Speed, next),
                        _ => cycle_perk(profile, next),
                    };
                    (changed.then_some(Transition::ValueChanged), changed)
                }
                MenuInput::TabLeft => (Some(Transition::WorldTourSelect), false),
                MenuInput::Filter => (None, false),
            },
        }
    }

    pub(super) fn render(
        &self,
        font: &RomFont<'_>,
        busts: &[Option<Bust>; 12],
        profile: &WorldTourProfile,
        frame_no: u32,
        notice: Option<&str>,
    ) -> Frame {
        if let Overlay::Select(select) = &self.overlay {
            return select.render_named(font, busts, "TOUR FIGHTER", frame_no, None);
        }
        let mut frame = (*self.backdrop).clone();
        draw_routes(&mut frame, profile, frame_no);
        draw_footer(&mut frame, font, profile);
        if let Overlay::Rpg(row) = self.overlay {
            frame = dimmed(&frame);
            draw_rpg(&mut frame, font, profile, row);
        }
        if let Some(notice) = notice {
            boxed(&mut frame, font, 1, 0, 46, 3);
            font.text_centred(&mut frame.argb, 1, &fit(notice, 44), ink::YELLOW);
        }
        frame
    }
}

fn travel_map(assets: &Assets) -> Frame {
    // travel_map_start (0x3f812) enables FG at 0x1980 and uploads this palette.
    let mut scene = SceneState {
        videoram: Vec::new(),
        objectram: Vec::new(),
        paletteram: vec![0; 0x800],
        bg_scroll: 0,
        fg_scroll: 0x1980,
        active: 0x40,
        flip_screen: false,
        bg_page: 0,
        fg_page: 0,
    };
    if let Some(palette) = assets.maincpu.get(0x1a268..0x1a468) {
        scene.paletteram[0x200..0x400].copy_from_slice(palette);
    }
    render_frame(assets, &scene)
}

fn move_node(profile: &mut WorldTourProfile, direction: isize) -> (Option<Transition>, bool) {
    let nodes = map();
    let current = nodes
        .iter()
        .position(|node| node.id == profile.current_node().id)
        .unwrap();
    let next = (1..nodes.len())
        .map(|step| {
            (current as isize + direction * step as isize).rem_euclid(nodes.len() as isize) as usize
        })
        .find(|&index| profile.is_unlocked(&nodes[index]));
    let changed = next.is_some_and(|index| profile.select_node(nodes[index].id));
    (changed.then_some(Transition::CursorMoved), changed)
}

fn cycle_perk(profile: &mut WorldTourProfile, next: bool) -> bool {
    let perks: Vec<_> = std::iter::once(None)
        .chain(profile.unlocked_perks().iter().copied().map(Some))
        .collect();
    let index = perks
        .iter()
        .position(|&perk| perk == profile.perk)
        .unwrap_or(0);
    let offset = if next { 1 } else { perks.len() - 1 };
    profile.equip(perks[(index + offset) % perks.len()])
}

fn draw_routes(frame: &mut Frame, profile: &WorldTourProfile, frame_no: u32) {
    let nodes = map();
    for (from, marker) in nodes.iter().zip(MARKERS) {
        for to in from.next {
            let index = nodes.iter().position(|node| node.id == *to).unwrap();
            let color = if profile.is_cleared(from) {
                GOLD
            } else {
                0xff6d_6d6d
            };
            route(frame, marker, MARKERS[index], color);
        }
    }
    for (node, (x, y)) in nodes.iter().zip(MARKERS) {
        if profile.is_cleared(node) {
            star(frame, x, y);
        } else {
            let color = if profile.is_unlocked(node) {
                WHITE
            } else {
                DIM
            };
            for dy in -3i32..=3 {
                let radius = 3 - dy.abs();
                for dx in -radius..=radius {
                    let edge = dx.abs() + dy.abs() == 3;
                    pixel(frame, x + dx, y + dy, if edge { SHADOW } else { color });
                }
            }
        }
        if node.id == profile.current_node().id {
            let color = if (frame_no / 10) % 2 == 0 {
                GOLD
            } else {
                0xffac_7a35
            };
            outline(
                &mut frame.argb,
                (x - 6) as usize,
                (y - 6) as usize,
                13,
                13,
                color,
            );
        }
    }
}

fn route(frame: &mut Frame, (x1, y1): (i32, i32), (x2, y2): (i32, i32), color: u32) {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let steps = dx.abs().max(dy.abs());
    for step in 0..=steps {
        let (x, y) = (x1 + dx * step / steps, y1 + dy * step / steps);
        pixel(frame, x, y + 1, SHADOW);
        pixel(frame, x, y, color);
    }
}

fn star(frame: &mut Frame, x: i32, y: i32) {
    let shape = [
        0b000010000,
        0b000111000,
        0b000111000,
        0b111111111,
        0b011111110,
        0b001111100,
        0b001111100,
        0b011101110,
        0b010000010,
    ];
    for (row, bits) in shape.into_iter().enumerate() {
        for col in 0..9 {
            if bits & (1 << col) != 0 {
                pixel(frame, x + col - 4, y + row as i32 - 3, SHADOW);
                pixel(frame, x + col - 4, y + row as i32 - 4, GOLD);
            }
        }
    }
}

fn pixel(frame: &mut Frame, x: i32, y: i32, color: u32) {
    frame.argb[y as usize * WIDTH + x as usize] = color;
}

fn draw_footer(frame: &mut Frame, font: &RomFont<'_>, profile: &WorldTourProfile) {
    let node = profile.current_node();
    let node_title = format!(
        "{} {} - {}",
        node.country.name().to_ascii_uppercase(),
        node.id.as_bytes()[3] as char,
        node.title
    );
    let reward = if profile.is_cleared(node) {
        "CLEAR".to_owned()
    } else {
        format!("XP REWARD +{}", node.xp_reward)
    };
    let opponents = node
        .opponents
        .iter()
        .map(|fighter| fighter.name().to_ascii_uppercase())
        .collect::<Vec<_>>()
        .join(" > ");
    let rule = match node.mission {
        MissionKind::RivalBout => "RIVAL BOUT  BEST OF THREE".to_owned(),
        MissionKind::SurvivalGauntlet { recovery_percent } => format!(
            "SURVIVAL  {} WAVES  +{recovery_percent}% HP",
            node.opponents.len()
        ),
        MissionKind::TimeAttack { max_ticks } => format!("TIME ATTACK  {max_ticks} TICKS"),
        MissionKind::RuleModifierTrial(RuleModifier::NoBlocking) => "NO BLOCKING FOR P1".to_owned(),
        MissionKind::RuleModifierTrial(RuleModifier::ProjectileDefense) => {
            "PROJECTILE DEFENSE  SURVIVE 60 TICKS".to_owned()
        }
        MissionKind::RuleModifierTrial(RuleModifier::ComboOnly) => {
            "COMBO ONLY  2ND HIT DEALS DAMAGE".to_owned()
        }
        MissionKind::RuleModifierTrial(RuleModifier::PureFootsies) => {
            "PURE FOOTSIES  NORMALS AND THROWS".to_owned()
        }
        MissionKind::EraStyleMastery { revision } => {
            format!("ERA MASTERY  {} LENT", revision.name())
        }
        MissionKind::DramaticBoss { .. } => {
            let ally = node.ally_for(profile.fighter).unwrap();
            format!("2V1 BOSS  ALLY {}", ally.name().to_ascii_uppercase())
        }
    };
    let style = node.player_style(profile.style);
    let loadout = if style.loadout == Loadout::Standard {
        "".to_owned()
    } else {
        format!(" {}", style.loadout.name().to_ascii_uppercase())
    };
    let lent = if style != profile.style { " LENT" } else { "" };
    let fighter = format!(
        "FIGHTER {} [{}{}]{}",
        profile.fighter.name().to_ascii_uppercase(),
        style.revision.name().to_ascii_uppercase(),
        loadout,
        lent
    );
    let perk = profile.perk.map_or("", Perk::name);
    let rpg = if profile.rpg {
        format!(
            "RPG ON  ATK {} DEF {} SPD {}  {perk}",
            profile.attack, profile.defense, profile.speed
        )
    } else {
        "RPG OFF".to_owned()
    };
    boxed(frame, font, 1, 19, 46, 9);
    let reward_col = 46 - reward.len() - usize::from(profile.is_cleared(node));
    font.text(
        &mut frame.argb,
        2,
        20,
        &fit(&node_title, reward_col - 3),
        ink::YELLOW,
    );
    font.text(&mut frame.argb, reward_col, 20, &reward, ink::YELLOW);
    if profile.is_cleared(node) {
        star(frame, 45 * 8 + 3, 20 * 8 + 4);
    }
    for (row, text) in [
        (21, format!("OPPONENT {opponents}")),
        (22, format!("RULE {rule}")),
        (23, fighter),
        (24, rpg),
    ] {
        font.text(&mut frame.argb, 2, row, &fit(&text, 44), ink::BLUE);
    }
    font.text_centred(
        &mut frame.argb,
        25,
        "← → NODE   CONFIRM FIGHT   BACK MENU",
        ink::DIM,
    );
    font.text_centred(
        &mut frame.argb,
        26,
        "TAB L FIGHTER/STYLE   TAB R STATS/PERKS",
        ink::DIM,
    );
}

fn draw_rpg(frame: &mut Frame, font: &RomFont<'_>, profile: &WorldTourProfile, row: usize) {
    boxed(frame, font, 6, 3, 36, 16);
    font.header(&mut frame.argb, 15, 4, "RPG STATS", ink::BLUE);
    font.text_centred(
        &mut frame.argb,
        7,
        &format!("XP {}   {} XP PER LEVEL", profile.xp(), STAT_COST),
        ink::YELLOW,
    );
    let entries = [
        format!("RPG BOOSTS  {}", if profile.rpg { "ON" } else { "OFF" }),
        format!("ATTACK      {} / 5", profile.attack),
        format!("DEFENSE     {} / 5", profile.defense),
        format!("SPEED       {} / 5", profile.speed),
        format!("PERK        {}", profile.perk.map_or("NONE", Perk::name)),
    ];
    for (index, text) in entries.iter().enumerate() {
        let active = row == index;
        let color = if active { ink::YELLOW } else { ink::BLUE };
        let y = 9 + index;
        if active {
            font.text(&mut frame.argb, 9, y, ">", color);
        }
        font.text(&mut frame.argb, 11, y, text, color);
    }
    let help = match row {
        0 => "CONFIRM TOGGLE",
        4 if profile.unlocked_perks().is_empty() => "NO PERKS UNLOCKED YET",
        4 => "← → CHANGE EQUIPPED PERK",
        _ => "← REFUND   → UPGRADE",
    };
    font.text_centred(&mut frame.argb, 15, help, ink::DIM);
    font.text_centred(&mut frame.argb, 17, "BACK TO WORLD TOUR", ink::DIM);
}
