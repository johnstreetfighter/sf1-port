//! Route books (SC3, `#195`; spec `#192`, ADR-0026): the combos the Fair
//! CPU may take, found by searching the real game and checked in as data.
//!
//! A **route** is a combo as a pad script: steps (a normal, a motion and a
//! button) with the frames to wait before each. A **route book** holds the
//! routes of one tournament fighter in one era style. It is keyed by the
//! attacker's pair, not by opponent: every route is searched and proven
//! against one reference defender (Ryu in the attacker's style, or his own
//! default), and carries a list of opponents it is known to fail on, which
//! a developer command fills (SC13, `#205`;
//! `crates/sf-host/examples/route_check.rs`): it plays each route from the
//! same kind of position against each opponent fighter in each style, in
//! each stance the route works on, and records those it lands fewer hits
//! on. A punish is checked against each opponent's own sweep, as the
//! search made it against the reference defender's. The Fair CPU takes no
//! route against an opponent it is recorded failing on.
//!
//! **Which books there are:** one for every fighter and style the search
//! found a route of two hits or more for, listed in
//! `route-books/books.rs`; every other pair is in `route-books/unbooked.txt`
//! with the reason (the 1987 rules allow no combo, or the search found
//! none). Both are written by `route_search all`.
//!
//! **Where a route starts** ([`Reach`], [`position`]): on the ground, the
//! attacker walked in until the two fighters touch; from a jump in, from a
//! short step back; or as a punish, straight after the attacker has
//! guarded the defender's sweep. Both fighters' meters are full once and
//! are not refilled, so a route cannot spend more than a fighter holds.
//!
//! **What is recorded** ([`Route`]) comes from playing the route in the
//! engine ([`replay`]): hits on the engine's own combo count, damage,
//! meter spent and gained, whether the defender ends knocked down, the
//! stances it works on, the rules it needs, how long after the first hit
//! the second step can still begin (the time there is to see the hit),
//! how far apart the fighters can start, and where it may be cut on
//! purpose for a combo reset ([`Cut`], SC10 `#202`: the defender comes out
//! of the hit reaction on his feet with the attacker free first; format 2
//! of the book added the line, `route_search cuts` marks the checked-in
//! books). A step is named by its input, not
//! by the move the input gave: the same motion gives a super with meter
//! and a special without. Meter gained is measured from full meters, so it
//! undercounts; and a punish's Drive gain includes what guarding the sweep
//! gave back. Knockdown is read from how the defender is drawn after the
//! last hit and has not been checked by eye. Every route found here was found
//! away from the corner, so none needs it; corner-only routes are not
//! searched yet.
//!
//! **The search** ([`search`]) is the combo clip recorder's
//! (`crates/sf-host/examples/combo_clips.rs`), widened from one long route
//! to the useful ones: routes of two hits and up, several per pair. It is
//! a developer tool (`crates/sf-host/examples/route_search.rs`); nothing in
//! a fight runs it. A fight only reads the checked-in books ([`book`]). The
//! bosses' positions need the extended data bank, built from the art pack.

use std::sync::{Arc, OnceLock};

use sf_video::callouts::Side;

use crate::config::MatchConfig;
use crate::data_bank::DataBank;
use crate::era::{listed_families, listed_revisions, loadouts, Loadout, Revision, StylePick};
use crate::fair_cpu::{perceive, Body};
use crate::front::cabinet::Players;
use crate::front::fight::{Fight, FightDips, FightKind, FightSetup};
use crate::roster::{PlayMode, TournamentFighter};
use crate::run::PowerOnMachine;

/// Stick bits of a [`Pad`], for an attacker on the left facing right.
pub const RIGHT: u8 = 1;
pub const LEFT: u8 = 2;
pub const DOWN: u8 = 4;
pub const UP: u8 = 8;

/// One pad frame: stick bits and button bits (LP to HK from bit 0).
pub type Pad = (u8, u8);

/// The checked-in route books, one file per fighter and era style, as
/// `route_search all` lists them.
const BOOKS: &[&str] = include!("../route-books/books.rs");

/// The first line of a route book file.
const HEADING: &str = "route-book 2";
/// The reference defender.
const DEFENDER: TournamentFighter = TournamentFighter::Ryu;
/// An era style's health units in one health bar point.
const UNITS_PER_POINT: u16 = 20;
/// Frames with no new hit after which a combo is over.
const QUIET: usize = 50;
/// The most steps a searched route has.
const MAX_STEPS: usize = 5;

/// How a route's opener is reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reach {
    /// A hit on the ground, in neutral or in pressure.
    Ground,
    /// A jump-in attack.
    JumpIn,
    /// A hit on an opponent recovering from a move that was guarded or missed.
    Punish,
    /// A hit on a jumping opponent. No search fills this yet.
    AntiAir,
    /// A hit out of a Drive Rush. No search fills this yet.
    DriveRush,
}

impl Reach {
    const ALL: [(Reach, &'static str); 5] = [
        (Reach::Ground, "ground"),
        (Reach::JumpIn, "jump-in"),
        (Reach::Punish, "punish"),
        (Reach::AntiAir, "anti-air"),
        (Reach::DriveRush, "drive-rush"),
    ];

    fn name(self) -> &'static str {
        Self::ALL.iter().find(|(r, _)| *r == self).unwrap().1
    }
}

/// The defender's stance when the opener lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stance {
    Standing,
    Crouching,
}

impl Stance {
    const ALL: [(Stance, &'static str); 2] = [(Stance::Standing, "standing"), (Stance::Crouching, "crouching")];

    /// How a route book writes it.
    pub fn name(self) -> &'static str {
        Self::ALL.iter().find(|(s, _)| *s == self).unwrap().1
    }

    /// The stick the defender holds.
    fn stick(self) -> u8 {
        match self {
            Stance::Standing => 0,
            Stance::Crouching => DOWN,
        }
    }
}

/// One step of a route: `wait` frames with `hold` on the stick and no
/// button, then `pads`. The wait can be anything in `window`, the other
/// steps keeping their own waits, and the route still lands all its hits.
/// Windows are measured one step at a time, not together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub name: String,
    pub wait: u8,
    pub window: (u8, u8),
    pub hold: u8,
    pub pads: Vec<Pad>,
}

/// One proven combo. See the module docs for how each field is measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub reach: Reach,
    pub steps: Vec<Step>,
    pub hits: u8,
    /// In twentieths of a health bar point.
    pub damage: u16,
    /// The rules it needs: without the rule the route does less.
    pub true_combos: bool,
    pub cancels: bool,
    /// Super meter and Drive gauge spent and gained, in the engine's units.
    pub super_spent: u16,
    pub super_gained: u16,
    pub drive_spent: u16,
    pub drive_gained: u16,
    pub knockdown: bool,
    pub corner: bool,
    pub stances: Vec<Stance>,
    /// Frames from the first hit to the latest the second step can begin:
    /// the time there is to see the hit before going on. A route whose
    /// second step must begin sooner than a level's reaction delay has to
    /// be committed before the hit is seen.
    pub confirm: u8,
    /// The widest the fighters can start apart: from the attacker's centre
    /// to the near edge of the defender's hurtbox.
    pub gap: i16,
    /// Opponents it is known to fail on: a fighter in every style he has
    /// or in one (as [`opponent_key`] names it), in every stance the route
    /// works on or in one. See [`Route::fails_against`].
    pub fails_on: Vec<String>,
    /// Where the route may be cut on purpose (SC10, `#202`): see [`Cut`].
    pub cuts: Vec<Cut>,
}

/// A marked point where a route may be cut on purpose, to attack again at
/// once (a combo reset; SC10, `#202`). Cut before step `step`: the steps
/// before it have hit, the defender is in a hit reaction he comes out of
/// on his feet, and the attacker is free to act before he is. Measured by
/// [`cut_at`] at the route's recorded waits from its reference position;
/// the numbers are against a standing defender, and the cut holds in every
/// stance the route lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cut {
    pub step: u8,
    /// Frames from the cut (the script frame step `step` would begin on)
    /// to the defender's first free frame.
    pub free: u8,
    /// Frames the attacker is free before he is (at least 1).
    pub ahead: u8,
    /// From the attacker's centre to the near edge of the defender's
    /// hurtbox, on the defender's first free frame.
    pub gap: i16,
}

impl Route {
    /// The route's name: its steps' names in order.
    pub fn name(&self) -> String {
        self.steps.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")
    }

    /// Every step is a plain normal (standing, crouching or jumping, one
    /// button: `MP`, `cr.LK`, `j.HK`) and no meter is spent: what a World
    /// Tour mission that allows only normals and throws leaves (SC23,
    /// `#215`). A special, a super, a command normal (`f+LK`) and a
    /// two-button era command (`MP+MK`, `HP+HK`) are not.
    pub fn normals_only(&self) -> bool {
        const BUTTONS: [&str; 6] = ["LP", "MP", "HP", "LK", "MK", "HK"];
        let normal = |name: &str| {
            let button = name.strip_prefix("cr.").or_else(|| name.strip_prefix("j.")).unwrap_or(name);
            BUTTONS.contains(&button)
        };
        self.super_spent == 0 && self.drive_spent == 0 && self.steps.iter().all(|step| normal(&step.name))
    }

    /// The pad on frame `frame` of the route's script with its recorded
    /// waits, if the script lasts that long.
    pub fn pad_at(&self, frame: usize) -> Option<Pad> {
        let mut at = frame;
        for step in &self.steps {
            let wait = usize::from(step.wait);
            if at < wait {
                return Some((step.hold, 0));
            }
            at -= wait;
            if at < step.pads.len() {
                return Some(step.pads[at]);
            }
            at -= step.pads.len();
        }
        None
    }

    /// The script frame step `index` begins on (its wait's first frame).
    pub fn step_start(&self, index: usize) -> usize {
        self.steps[..index.min(self.steps.len())].iter().map(|s| usize::from(s.wait) + s.pads.len()).sum()
    }

    /// Whether the book records the route failing on the opponent `key`
    /// ([`opponent_key`]) when he is in `stance`: an entry names him alone
    /// (every stance the route works on) or him and one stance.
    pub fn fails_against(&self, key: &str, stance: Stance) -> bool {
        self.fails_on.iter().any(|entry| entry_stance(entry, key).is_some_and(|only| only.is_none_or(|s| s == stance)))
    }

    /// The script's length in frames, with the recorded waits.
    pub fn len(&self) -> usize {
        self.step_start(self.steps.len())
    }

    /// The route as pad frames, with each step's wait as given by `waits`
    /// (the recorded ones when `None`).
    pub fn script(&self, waits: Option<&[u8]>) -> Vec<Pad> {
        let mut out = Vec::new();
        for (i, step) in self.steps.iter().enumerate() {
            let wait = waits.map_or(step.wait, |w| w[i]);
            out.extend(std::iter::repeat_n((step.hold, 0), usize::from(wait)));
            out.extend(&step.pads);
        }
        out
    }
}

/// The routes of one tournament fighter in one era style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteBook {
    pub fighter: TournamentFighter,
    pub revision: Revision,
    pub loadout: Loadout,
    pub routes: Vec<Route>,
}

fn pads_text(pads: &[Pad]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < pads.len() {
        let run = pads[i..].iter().take_while(|&&p| p == pads[i]).count();
        let (stick, buttons) = pads[i];
        out.push(if run > 1 { format!("{stick:x}:{buttons:02x}x{run}") } else { format!("{stick:x}:{buttons:02x}") });
        i += run;
    }
    out.join(" ")
}

fn parse_pads(text: &str) -> Result<Vec<Pad>, String> {
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        let bad = || format!("bad pad {word}");
        let (pad, run) = match word.split_once('x') {
            Some((pad, run)) => (pad, run.parse::<usize>().map_err(|_| bad())?),
            None => (word, 1),
        };
        let (stick, buttons) = pad.split_once(':').ok_or_else(bad)?;
        let pad = (u8::from_str_radix(stick, 16).map_err(|_| bad())?, u8::from_str_radix(buttons, 16).map_err(|_| bad())?);
        if pad.0 > 0xf || pad.1 > 0x3f || !(1..=600).contains(&run) {
            return Err(bad());
        }
        out.extend(std::iter::repeat_n(pad, run));
    }
    Ok(out)
}

fn yes(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}

fn parse_yes(text: &str) -> Result<bool, String> {
    match text {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(format!("{text} is not yes or no")),
    }
}

fn number<T: std::str::FromStr>(text: &str) -> Result<T, String> {
    text.trim().parse().map_err(|_| format!("{text} is not a number"))
}

fn parse_style(fighter: TournamentFighter, text: &str) -> Result<(Revision, Loadout), String> {
    for family in listed_families(fighter, true) {
        for revision in listed_revisions(fighter, family, true) {
            for loadout in loadouts(revision) {
                if format!("{revision:?}/{loadout:?}") == text {
                    return Ok((revision, loadout));
                }
            }
        }
    }
    Err(format!("{fighter:?} has no style {text}"))
}

/// The next line's text after `key`, which it must begin with.
fn field<'a>(lines: &mut impl Iterator<Item = &'a str>, key: &str) -> Result<String, String> {
    let line = lines.next().ok_or_else(|| format!("the book ends before {key}"))?;
    if line == key {
        return Ok(String::new());
    }
    line.strip_prefix(key).and_then(|rest| rest.strip_prefix(' ')).map(str::to_owned).ok_or_else(|| format!("expected {key}, found {line}"))
}

impl RouteBook {
    /// The book as its checked-in text.
    pub fn to_text(&self) -> String {
        let mut out = format!("{HEADING}\nfighter {:?}\nstyle {:?}/{:?}\n", self.fighter, self.revision, self.loadout);
        for route in &self.routes {
            let rules: Vec<&str> = [("true_combos", route.true_combos), ("cancels", route.cancels)].into_iter().filter(|r| r.1).map(|r| r.0).collect();
            let stances: Vec<&str> = route.stances.iter().map(|s| s.name()).collect();
            out.push_str(&format!(
                "\nroute {}\nname {}\nhits {}\ndamage {}\nrules {}\nmeter super -{} +{} drive -{} +{}\nknockdown {}\ncorner {}\nstances {}\nconfirm {}\ngap {}\nfails-on {}\ncuts {}\n",
                route.reach.name(),
                route.name(),
                route.hits,
                route.damage,
                rules.join(" "),
                route.super_spent,
                route.super_gained,
                route.drive_spent,
                route.drive_gained,
                yes(route.knockdown),
                yes(route.corner),
                stances.join(" "),
                route.confirm,
                route.gap,
                route.fails_on.join("; "),
                route.cuts.iter().map(|c| format!("{} free {} ahead {} gap {}", c.step, c.free, c.ahead, c.gap)).collect::<Vec<_>>().join("; "),
            ));
            for step in &route.steps {
                out.push_str(&format!(
                    "step {} | wait {} {}..{} | hold {:x} | {}\n",
                    step.name,
                    step.wait,
                    step.window.0,
                    step.window.1,
                    step.hold,
                    pads_text(&step.pads)
                ));
            }
            out.push_str("end\n");
        }
        // A line with nothing after its key ends at the key.
        out.replace(" \n", "\n")
    }

    /// Read a book from its checked-in text. Every line of a route is
    /// required, in [`Self::to_text`]'s order.
    pub fn parse(text: &str) -> Result<RouteBook, String> {
        let mut lines = text.lines().map(str::trim_end).filter(|l| !l.is_empty()).peekable();
        if field(&mut lines, HEADING)? != "" {
            return Err(format!("the first line is not {HEADING}"));
        }
        let fighter = field(&mut lines, "fighter")?;
        let fighter = TournamentFighter::ALL.into_iter().find(|f| format!("{f:?}") == fighter).ok_or_else(|| format!("no fighter {fighter}"))?;
        let (revision, loadout) = parse_style(fighter, &field(&mut lines, "style")?)?;
        let mut routes = Vec::new();
        let keys: std::collections::HashSet<String> = opponents().into_iter().map(|(who, style)| opponent_key(who, style)).collect();
        let fighters: Vec<String> = TournamentFighter::ALL.iter().map(|f| format!("{f:?}")).collect();
        let known = |entry: &str| {
            entry_parts(entry).is_some_and(|(fighter, style, _)| match style {
                Some(style) => keys.contains(&format!("{fighter} {style}")),
                None => fighters.iter().any(|f| f == fighter),
            })
        };
        // Routes to the end of the text: anything else there is a fault.
        while lines.peek().is_some() {
            let reach = field(&mut lines, "route")?;
            let reach = Reach::ALL.iter().find(|(_, name)| *name == reach).ok_or_else(|| format!("no reach {reach}"))?.0;
            let name = field(&mut lines, "name")?;
            let hits = number(&field(&mut lines, "hits")?)?;
            let damage = number(&field(&mut lines, "damage")?)?;
            let rules = field(&mut lines, "rules")?;
            let meter = field(&mut lines, "meter")?;
            let parts: Vec<&str> = meter.split_whitespace().collect();
            let ["super", super_spent, super_gained, "drive", drive_spent, drive_gained] = parts[..] else { return Err(format!("bad meter line {meter}")) };
            if rules.split_whitespace().any(|rule| !["true_combos", "cancels"].contains(&rule)) {
                return Err(format!("unknown rule in {rules}"));
            }
            let signed = |text: &str, sign: char| text.strip_prefix(sign).ok_or_else(|| format!("bad meter line {meter}")).and_then(number::<u16>);
            let knockdown = parse_yes(&field(&mut lines, "knockdown")?)?;
            let corner = parse_yes(&field(&mut lines, "corner")?)?;
            let stances = field(&mut lines, "stances")?
                .split_whitespace()
                .map(|word| Stance::ALL.iter().find(|(_, name)| *name == word).map(|s| s.0).ok_or_else(|| format!("no stance {word}")))
                .collect::<Result<Vec<_>, _>>()?;
            let confirm = number(&field(&mut lines, "confirm")?)?;
            let gap = number(&field(&mut lines, "gap")?)?;
            let fails_on: Vec<String> = field(&mut lines, "fails-on")?.split("; ").filter(|s| !s.is_empty()).map(str::to_owned).collect();
            if let Some(entry) = fails_on.iter().find(|entry| !known(entry)) {
                return Err(format!("no opponent {entry}"));
            }
            let cuts = field(&mut lines, "cuts")?
                .split("; ")
                .filter(|s| !s.is_empty())
                .map(|text| {
                    let parts: Vec<&str> = text.split_whitespace().collect();
                    let [step, "free", free, "ahead", ahead, "gap", gap] = parts[..] else { return Err(format!("bad cut {text}")) };
                    Ok(Cut { step: number(step)?, free: number(free)?, ahead: number(ahead)?, gap: number(gap)? })
                })
                .collect::<Result<Vec<Cut>, String>>()?;
            let mut steps = Vec::new();
            loop {
                let line = lines.next().ok_or("a route does not end")?;
                if line == "end" {
                    break;
                }
                let rest = line.strip_prefix("step ").ok_or_else(|| format!("expected a step, found {line}"))?;
                let parts: Vec<&str> = rest.split(" | ").collect();
                let [step_name, wait, hold, pads] = parts[..] else { return Err(format!("bad step {line}")) };
                let wait: Vec<&str> = wait.split_whitespace().collect();
                let ["wait", at, window] = wait[..] else { return Err(format!("bad step {line}")) };
                let (low, high) = window.split_once("..").ok_or_else(|| format!("bad step {line}"))?;
                let hold = hold.strip_prefix("hold ").ok_or_else(|| format!("bad step {line}"))?;
                steps.push(Step {
                    name: step_name.to_owned(),
                    wait: number(at)?,
                    window: (number(low)?, number(high)?),
                    hold: u8::from_str_radix(hold, 16).map_err(|_| format!("bad step {line}"))?,
                    pads: parse_pads(pads)?,
                });
            }
            let route = Route {
                reach,
                steps,
                hits,
                damage,
                true_combos: rules.split_whitespace().any(|r| r == "true_combos"),
                cancels: rules.split_whitespace().any(|r| r == "cancels"),
                super_spent: signed(super_spent, '-')?,
                super_gained: signed(super_gained, '+')?,
                drive_spent: signed(drive_spent, '-')?,
                drive_gained: signed(drive_gained, '+')?,
                knockdown,
                corner,
                stances,
                confirm,
                gap,
                fails_on,
                cuts,
            };
            if route.name() != name || route.steps.is_empty() || route.stances.is_empty() {
                return Err(format!("route {name} does not match its steps"));
            }
            if route.steps.iter().any(|s| s.pads.is_empty() || !(s.window.0..=s.window.1).contains(&s.wait)) {
                return Err(format!("route {name} has an empty step or a wait outside its window"));
            }
            if route.cuts.iter().any(|c| c.step == 0 || usize::from(c.step) >= route.steps.len() || c.ahead == 0 || c.free == 0 || c.ahead > c.free)
                || route.cuts.windows(2).any(|w| w[0].step >= w[1].step)
            {
                return Err(format!("route {name} has a cut that is not between two of its steps, in order, with the attacker free first and not before the cut"));
            }
            routes.push(route);
        }
        Ok(RouteBook { fighter, revision, loadout, routes })
    }
}

/// Every checked-in route book. A book that does not parse is a build
/// fault, caught by the route proof test.
pub fn books() -> &'static [RouteBook] {
    static PARSED: OnceLock<Vec<RouteBook>> = OnceLock::new();
    PARSED.get_or_init(|| BOOKS.iter().map(|text| RouteBook::parse(text).expect("a checked-in route book parses")).collect())
}

/// The checked-in text of every route book, as the build holds it.
pub fn book_texts() -> &'static [&'static str] {
    BOOKS
}

/// The identity of a set of route book texts, in the order given: each
/// text whole, with its length, so a byte changed, moved between books, or
/// a book added, dropped or reordered gives another value. What a netplay
/// session agrees on for the books (SC14, `#206`; see
/// [`crate::netplay::protocol_identity`]). Line endings are read as `\n`:
/// a Windows checkout holds the books with `\r\n` (`core.autocrlf`), a
/// Linux one with `\n`, and the two builds play from the same routes.
pub fn books_hash_of(texts: &[&str]) -> u64 {
    use crate::canonical::{Canonical, CanonicalHasher};
    let mut out = CanonicalHasher::new();
    out.tag("sf1-port/route-books/v1");
    texts.len().encode(&mut out);
    for text in texts {
        text.replace("\r\n", "\n").encode(&mut out);
    }
    out.finish()
}

/// [`books_hash_of`] the books this build holds, worked out once.
pub fn books_hash() -> u64 {
    static HASH: OnceLock<u64> = OnceLock::new();
    *HASH.get_or_init(|| books_hash_of(BOOKS))
}

/// The file a fighter's route book in a style is checked in as, in
/// `crates/sf-engine/route-books/`.
pub fn file_name(fighter: TournamentFighter, revision: Revision, loadout: Loadout) -> String {
    let plain = |text: &str| text.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_ascii_lowercase();
    format!("{}_{}_{}.routes", plain(fighter.name()), plain(&format!("{revision:?}")), plain(&format!("{loadout:?}")))
}

/// The route book of a fighter in an era style, if one is checked in.
pub fn book(fighter: TournamentFighter, revision: Revision, loadout: Loadout) -> Option<&'static RouteBook> {
    books().iter().find(|b| (b.fighter, b.revision, b.loadout) == (fighter, revision, loadout))
}

/// One frame with the attacker's pad and the defender's.
fn tick(m: &mut PowerOnMachine, (stick, buttons): Pad, (foe_stick, foe_buttons): Pad) {
    m.set_p1_direction(stick & RIGHT != 0, stick & LEFT != 0, stick & DOWN != 0, stick & UP != 0);
    m.set_p1_buttons(std::array::from_fn(|i| buttons & (1 << i) != 0));
    m.set_p2_direction(foe_stick & RIGHT != 0, foe_stick & LEFT != 0, foe_stick & DOWN != 0, foe_stick & UP != 0);
    m.set_p2_buttons(std::array::from_fn(|i| foe_buttons & (1 << i) != 0));
    m.set_cabinet_inputs(false, false, false, false, false);
    m.step();
}

/// The two fighters as the frame shows them: the attacker, then the defender.
fn bodies(m: &PowerOnMachine) -> [Body; 2] {
    let view = perceive(m.state.frame, true, m.work_ram(), &m.state.objectram, Side::P1);
    [view.me, view.foe]
}

/// From the attacker's centre to the near edge of the defender's hurtbox.
fn gap(m: &PowerOnMachine) -> i16 {
    let [me, foe] = bodies(m);
    foe.hurt.0 - me.x
}

/// The engine's count of hits in the combo on the defender.
fn hits(m: &PowerOnMachine) -> u8 {
    m.toggle_state().combo.hits[1]
}

/// The defender's health in an era style's units.
fn health(m: &PowerOnMachine) -> u16 {
    let era = &m.era_fighters()[1];
    if era.max_sub_hp != 0 { era.sub_hp } else { bodies(m)[1].health * UNITS_PER_POINT }
}

/// The attacker's Super meter and Drive gauge.
fn meters(m: &PowerOnMachine) -> [u16; 2] {
    let era = &m.era_fighters()[0];
    [u16::from(era.super_meter), era.sf6_drive_points().unwrap_or(0)]
}

/// Follows the engine's combo count from one frame to the next. The engine
/// writes the count only on a hit: one more if the defender was still in
/// hitstun, otherwise back to 1, and it is never cleared when a combo ends.
/// So a hit (the defender's health falls) that does not raise the count by
/// one is a new combo: the old one dropped.
#[derive(Clone, Copy)]
struct Combo {
    count: u8,
    health: u16,
    dropped: bool,
}

impl Combo {
    fn at(m: &PowerOnMachine) -> Combo {
        Combo { count: hits(m), health: health(m), dropped: false }
    }

    /// Take in the frame just stepped. True when a hit landed that carries
    /// the combo on.
    fn follow(&mut self, m: &PowerOnMachine) -> bool {
        let (count, now) = (hits(m), health(m));
        let hit = now < self.health || count != self.count;
        let carried = hit && !self.dropped && count == self.count.wrapping_add(1);
        self.dropped |= hit && !carried;
        self.health = now;
        if carried {
            self.count = count;
        }
        carried
    }
}

/// The reference position a route of `reach` starts from, against the
/// reference defender in `stance`: see the module docs. `data` is the
/// extended data bank with the bosses' tables; without one only Ryu and Ken
/// can attack.
pub fn position(
    program_rom: &[u8],
    data: Option<Arc<DataBank>>,
    fighter: TournamentFighter,
    style: (Revision, Loadout),
    reach: Reach,
    stance: Stance,
) -> Result<PowerOnMachine, String> {
    from_opening(&opening(program_rom, data, fighter, style, None, stance)?, reach, stance)
}

/// Every opponent a route can meet: each fighter in each style he can be
/// picked in, what-if styles included.
pub fn opponents() -> Vec<(TournamentFighter, (Revision, Loadout))> {
    let mut out = Vec::new();
    for fighter in TournamentFighter::ALL {
        for family in listed_families(fighter, true) {
            for revision in listed_revisions(fighter, family, true) {
                out.extend(loadouts(revision).into_iter().map(|loadout| (fighter, (revision, loadout))));
            }
        }
    }
    out
}

/// A `fails-on` entry: a fighter, then his style unless it is every style
/// he has, then the one stance unless it is every stance the route works
/// on (`Lee`, `Mike crouching`, `Sagat Sf6/Standard`, `Gen Sfz3/XIsm
/// standing`). `None` when it is not of that shape.
fn entry_parts(entry: &str) -> Option<(&str, Option<&str>, Option<Stance>)> {
    let mut words = entry.split(' ');
    let fighter = words.next()?;
    let mut style = None;
    let mut stance = None;
    for word in words {
        if let Some(s) = Stance::ALL.iter().find(|(_, name)| *name == word) {
            stance = Some(s.0);
        } else if word.contains('/') && style.is_none() && stance.is_none() {
            style = Some(word);
        } else {
            return None;
        }
    }
    Some((fighter, style, stance))
}

/// Whether a `fails-on` entry names the opponent `key` ([`opponent_key`]):
/// `Some(None)` in every stance, `Some(Some(stance))` in one.
fn entry_stance(entry: &str, key: &str) -> Option<Option<Stance>> {
    let (fighter, style, stance) = entry_parts(entry)?;
    let (key_fighter, key_style) = key.split_once(' ')?;
    (fighter == key_fighter && style.is_none_or(|s| s == key_style)).then_some(stance)
}

/// How a route book names an opponent in a route's `fails-on` line.
pub fn opponent_key(fighter: TournamentFighter, (revision, loadout): (Revision, Loadout)) -> String {
    format!("{fighter:?} {revision:?}/{loadout:?}")
}

/// The two fighters walked in until they touch, the defender holding
/// `stance` throughout, both meters filled once: where every reach's
/// position begins ([`from_opening`]). The defender is `defender` in his
/// style, or the reference defender when `None`: Ryu in the attacker's
/// style where he has it, otherwise in SF6.
pub fn opening(
    program_rom: &[u8],
    data: Option<Arc<DataBank>>,
    fighter: TournamentFighter,
    (revision, loadout): (Revision, Loadout),
    defender: Option<(TournamentFighter, (Revision, Loadout))>,
    stance: Stance,
) -> Result<PowerOnMachine, String> {
    let pick = StylePick::for_revision(revision, loadout);
    if !pick.available_for(fighter, true) {
        return Err(format!("{fighter:?} has no {revision:?}/{loadout:?} style"));
    }
    let (defender, defender_pick) = match defender {
        Some((who, (revision, loadout))) => {
            let theirs = StylePick::for_revision(revision, loadout);
            if !theirs.available_for(who, true) {
                return Err(format!("{who:?} has no {revision:?}/{loadout:?} style"));
            }
            (who, theirs)
        }
        // The reference defender takes the attacker's style where he has it.
        None => (DEFENDER, if pick.available_for(DEFENDER, true) { pick } else { StylePick::for_revision(Revision::Sf6, Loadout::Standard) }),
    };
    let mut config = MatchConfig::definitive();
    config.roster.mode = PlayMode::Training;
    config.roster.p1 = fighter;
    config.roster.p2 = defender;
    config.roster.what_if_styles = true;
    config.fill_meters = true;
    let setup = FightSetup {
        kind: FightKind::TrainingDummy,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    };
    let data = data.unwrap_or_else(|| Arc::new(DataBank::build(program_rom, &[])));
    let mut fight = Fight::start_banked(program_rom, setup, Some(0), None, Some(data)).map_err(|event| format!("the fight did not start: {event:?}"))?;
    fight.set_selected_styles([Some(pick), Some(defender_pick)]);
    let mut m = fight.machine().clone();
    let foe = (stance.stick(), 0);
    let mut frames = 0;
    while !m.in_fight() {
        tick(&mut m, (0, 0), foe);
        frames += 1;
        if frames > 3_000 || !m.unrecovered().is_empty() {
            return Err("the fight loop was not reached".into());
        }
    }
    for _ in 0..60 {
        tick(&mut m, (0, 0), foe);
    }
    // Walk in until the gap stops closing. Some walks pause between steps:
    // a short still spell is not the end.
    let (mut last, mut still) = (gap(&m), 0);
    for _ in 0..400 {
        tick(&mut m, (RIGHT, 0), foe);
        let now = gap(&m);
        still = if now == last { still + 1 } else { 0 };
        last = now;
        if still >= 40 {
            break;
        }
    }
    for _ in 0..20 {
        tick(&mut m, (0, 0), foe);
    }
    // The meters are full now, once: from here nothing refills them.
    m.match_config_mut().fill_meters = false;
    tick(&mut m, (0, 0), foe);
    Ok(m)
}

/// The position a route of `reach` starts from, from an [`opening`] made
/// with the defender in `stance`.
pub fn from_opening(opening: &PowerOnMachine, reach: Reach, stance: Stance) -> Result<PowerOnMachine, String> {
    let mut m = opening.clone();
    let foe = (stance.stick(), 0);
    match reach {
        Reach::Ground => {}
        Reach::AntiAir | Reach::DriveRush => return Err(format!("no reference position for a {} route yet", reach.name())),
        Reach::JumpIn => {
            let near = gap(&m);
            for _ in 0..60 {
                if gap(&m) >= near + 70 {
                    break;
                }
                tick(&mut m, (LEFT, 0), foe);
            }
            if gap(&m) < near + 40 {
                return Err("the attacker could not step back for a jump-in".into());
            }
            for _ in 0..12 {
                tick(&mut m, (0, 0), foe);
            }
        }
        Reach::Punish => {
            // The defender sweeps and the attacker guards it low. The
            // position is the first frame the attacker is out of blockstun.
            let mut guarded = false;
            for frame in 0..90 {
                let sweep = if frame < 3 { (DOWN, 0x20) } else { foe };
                tick(&mut m, (DOWN | LEFT, 0), sweep);
                let stun = m.toggle_state().combo.blockstun[0] != 0;
                if guarded && !stun {
                    break;
                }
                guarded |= stun;
            }
            if !guarded {
                return Err("the attacker did not guard the defender's sweep".into());
            }
        }
    }
    if hits(&m) != 0 {
        return Err("the position begins inside a combo".into());
    }
    Ok(m)
}

/// What playing a route gave.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Played {
    /// The hits of the one combo the script made: counting stops where it
    /// dropped.
    pub hits: u8,
    pub damage: u16,
    pub super_spent: u16,
    pub super_gained: u16,
    pub drive_spent: u16,
    pub drive_gained: u16,
    pub knockdown: bool,
    /// The script frame the first hit landed on, and whether the defender
    /// was drawn in an attack on the frame before it: a punish.
    pub first_hit: Option<usize>,
    pub punished: bool,
    /// After the first hit, a button went in while the attacker was still
    /// drawn in an attack: a cancel, not a link.
    pub cancelled: bool,
}

/// Enter `script` on the attacker's pad from `base`, the defender holding
/// `stance`, then let go until the combo is over, and count what it did.
pub fn replay(base: &PowerOnMachine, script: &[Pad], stance: Stance) -> Played {
    let mut m = base.clone();
    let foe = (stance.stick(), 0);
    let mut played = Played::default();
    let mut combo = Combo { count: 0, ..Combo::at(&m) };
    let mut last_meters = meters(&m);
    let mut quiet = 0;
    let mut frame = 0;
    while frame < script.len() || (quiet <= QUIET && frame < script.len() + 400) {
        let [attacker, defender] = bodies(&m);
        let (defender_attacking, before) = (defender.attacking(), combo.health);
        let pad = script.get(frame).copied().unwrap_or((0, 0));
        played.cancelled |= played.hits > 0 && pad.1 != 0 && attacker.attacking() && !attacker.jumping();
        tick(&mut m, pad, foe);
        let carried = combo.follow(&m);
        if combo.dropped {
            break;
        }
        if carried {
            if played.first_hit.is_none() {
                played.first_hit = Some(frame);
                played.punished = defender_attacking;
            }
            played.hits = combo.count;
            played.damage += before.saturating_sub(combo.health);
            played.knockdown = false;
            quiet = 0;
        } else if frame >= script.len() && (played.hits > 0 || frame >= script.len() + 60) {
            // A slow move's first hit can come well after its input.
            quiet += 1;
        }
        if played.hits > 0 {
            let defender = bodies(&m)[1];
            played.knockdown |= defender.move_id == crate::combo::throw::GET_UP_MOVE || (defender.move_id >= 0x30 && defender.state & 1 != 0);
        }
        let now = meters(&m);
        played.super_spent += last_meters[0].saturating_sub(now[0]);
        played.super_gained += now[0].saturating_sub(last_meters[0]);
        played.drive_spent += last_meters[1].saturating_sub(now[1]);
        played.drive_gained += now[1].saturating_sub(last_meters[1]);
        last_meters = now;
        frame += 1;
    }
    played
}

/// Something the search can press: a normal, a motion and a button, a
/// button pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub name: String,
    pub pads: Vec<Pad>,
    /// The direction held while waiting to press: down-back for a charge.
    pub hold: u8,
}

const BUTTONS: [&str; 6] = ["LP", "MP", "HP", "LK", "MK", "HK"];

/// The presses the search tries on the ground: every normal standing,
/// crouching and with the stick forward or back, the usual motions with
/// each button and button set, charge releases, and the button pairs.
pub fn ground_actions() -> Vec<Action> {
    let (r, l, d, u) = (RIGHT, LEFT, DOWN, UP);
    let mut out = Vec::new();
    for (stance, dir) in [("", 0), ("cr.", d), ("f+", r), ("b+", l)] {
        for (i, button) in BUTTONS.iter().enumerate() {
            out.push(Action { name: format!("{stance}{button}"), pads: vec![(dir, 1 << i), (dir, 1 << i), (dir, 0)], hold: 0 });
        }
    }
    let mut sets: Vec<(String, u8)> = (0..6).map(|i| (BUTTONS[i].to_owned(), 1 << i)).collect();
    sets.extend([("PP".to_owned(), 0b000_011), ("KK".to_owned(), 0b011_000), ("PPP".to_owned(), 0b000_111), ("KKK".to_owned(), 0b111_000)]);
    let motions: [(&str, &[u8]); 9] = [
        ("236", &[d, d | r, r]),
        ("214", &[d, d | l, l]),
        ("623", &[r, d, d | r]),
        ("421", &[l, d, d | l]),
        ("41236", &[l, d | l, d, d | r, r]),
        ("63214", &[r, d | r, d, d | l, l]),
        ("236236", &[d, d | r, r, d, d | r, r]),
        ("214214", &[d, d | l, l, d, d | l, l]),
        ("d,d", &[d, 0, d]),
    ];
    let releases: [(&str, &[u8]); 3] = [("[4]6", &[r]), ("[2]8", &[u]), ("[4]646", &[r, l, r])];
    for (charge, list) in [(false, &motions[..]), (true, &releases[..])] {
        for &(name, steps) in list {
            for (set, bits) in &sets {
                let lead = if charge { &steps[..steps.len() - 1] } else { steps };
                let mut pads: Vec<Pad> = lead.iter().flat_map(|&s| [(s, 0), (s, 0)]).collect();
                let last = *steps.last().unwrap();
                pads.extend([(last, *bits), (last, *bits), (last, *bits), (last, 0)]);
                out.push(Action { name: format!("{name}+{set}"), pads, hold: if charge { d | l } else { 0 } });
            }
        }
    }
    for (i, button) in BUTTONS.iter().enumerate() {
        out.push(Action { name: format!("db+{button}"), pads: vec![(d | l, 1 << i), (d | l, 1 << i), (d | l, 0)], hold: d | l });
    }
    for (name, bits) in [("LP+LK", 0b001_001), ("MP+MK", 0b010_010), ("HP+HK", 0b100_100)] {
        out.push(Action { name: name.into(), pads: vec![(0, bits), (0, bits), (0, bits), (0, 0)], hold: 0 });
    }
    out
}

/// Jump-in openers: up-forward, then a button on the way down.
pub fn jump_in_actions() -> Vec<Action> {
    let mut out = Vec::new();
    for button in [2usize, 5, 4] {
        for at in (18..=44).step_by(2) {
            let mut pads = vec![(UP | RIGHT, 0); 3];
            pads.extend(std::iter::repeat_n((0, 0), at - 3));
            pads.extend([(0, 1 << button), (0, 1 << button), (0, 0)]);
            out.push(Action { name: format!("j.{}", BUTTONS[button]), pads, hold: 0 });
        }
    }
    out
}

/// Whether pressing `action` from `base` does anything the frames show: the
/// attacker is drawn in another move, or a meter changes. A press that does
/// nothing is not a step of any route.
pub fn does_something(base: &PowerOnMachine, action: &Action, stance: Stance) -> bool {
    what_it_does(base, action, stance).is_some()
}

/// What pressing `action` from `base` does, as far as the frames show in
/// the dozen after it: the moves the attacker is drawn in, in order, and
/// which meters it spent. `None` when he is drawn in nothing new and no
/// meter is spent. Two presses that give the same answer are the same
/// press to the search.
fn what_it_does(base: &PowerOnMachine, action: &Action, stance: Stance) -> Option<(Vec<u8>, [bool; 2])> {
    let mut m = base.clone();
    let before = meters(&m);
    let lead = std::iter::repeat_n((action.hold, 0), if action.hold != 0 { 60 } else { 0 });
    let neutral = std::iter::repeat_n((0, 0), 12);
    let mut moves: Vec<u8> = Vec::new();
    for pad in lead.chain(action.pads.iter().copied()).chain(neutral) {
        tick(&mut m, pad, (stance.stick(), 0));
        let now = bodies(&m)[0].move_id;
        // Walking and crouching on the way to a press are not the press.
        if now >= 0x0c && moves.last() != Some(&now) {
            moves.push(now);
        }
    }
    let after = meters(&m);
    // Which meters it spent, not by how much: a meter also fills by the frame.
    let spent = [after[0] < before[0], after[1] < before[1]];
    (!moves.is_empty() || spent != [false; 2]).then_some((moves, spent))
}

/// A state of the search: the fight after some steps, at a hit.
#[derive(Clone)]
struct Node {
    /// The fight as the last step's pads ended, its hit perhaps still to
    /// land: the next step may begin from here, as a cancel's motion does.
    after_pads: PowerOnMachine,
    /// Frames from there to the hit.
    since: usize,
    steps: Vec<Step>,
    hits: u8,
    /// The count's highest reading if the pad is left alone from the hit.
    ride: u8,
    /// The defender's health when the route began, and what it has lost.
    health: u16,
    damage: u16,
    /// The attacker was in the air when the last hit landed.
    airborne: bool,
}

impl Node {
    fn names(&self) -> Vec<&str> {
        self.steps.iter().map(|s| s.name.as_str()).collect()
    }
}

/// Leave the pad alone until the combo is over: the count it reaches.
fn ride_out(machine: &PowerOnMachine, foe: Pad) -> u8 {
    let mut m = machine.clone();
    let mut combo = Combo::at(&m);
    let mut quiet = 0;
    for _ in 0..300 {
        tick(&mut m, (0, 0), foe);
        if combo.follow(&m) {
            quiet = 0;
        } else {
            quiet += 1;
            if quiet > QUIET || combo.dropped {
                break;
            }
        }
    }
    combo.count
}

/// Press `action` `wait` frames after `node`'s last pads ended. The node it
/// makes, if the press landed a further hit of the same combo.
fn expand(node: &Node, action: &Action, wait: usize, patience: usize, foe: Pad) -> Option<Node> {
    let mut m = node.after_pads.clone();
    // The last step's own hit may still be on its way.
    let mut combo = Combo::at(&m);
    let mut pads = Vec::new();
    let script = std::iter::repeat_n((action.hold, 0), wait).map(|pad| (pad, false)).chain(action.pads.iter().map(|&pad| (pad, true)));
    for (pad, pressed) in script {
        tick(&mut m, pad, foe);
        if pressed {
            pads.push(pad);
        }
        combo.follow(&m);
        if combo.dropped {
            return None;
        }
        if combo.count > node.hits {
            // A hit before the press began is not the press's.
            if !pressed {
                return None;
            }
            break;
        }
    }
    if pads.is_empty() {
        return None;
    }
    let after_pads = m.clone();
    let mut since = 0;
    while combo.count <= node.hits {
        if since >= patience {
            return None;
        }
        tick(&mut m, (0, 0), foe);
        since += 1;
        combo.follow(&m);
        if combo.dropped {
            return None;
        }
    }
    // The count must have come by way of the last step's hit.
    if combo.count != node.hits + 1 {
        return None;
    }
    let wait = u8::try_from(wait).ok()?;
    let mut steps = node.steps.clone();
    steps.push(Step { name: action.name.clone(), wait, window: (wait, wait), hold: action.hold, pads });
    let damage = node.health.saturating_sub(health(&m));
    let airborne = bodies(&m)[0].jumping();
    let ride = ride_out(&m, foe);
    Some(Node { after_pads, since, steps, hits: combo.count, ride, health: node.health, damage, airborne })
}

/// How much searching to do, and what to keep.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    /// States the search goes on from after each step past the first,
    /// for each way a route can start. Every opener is gone on from.
    pub nodes: usize,
    /// Routes kept for each way a route can start.
    pub keep: usize,
}

/// What the search needs to make its positions.
pub struct Pair<'a> {
    pub program_rom: &'a [u8],
    pub data: Option<Arc<DataBank>>,
    pub fighter: TournamentFighter,
    pub style: (Revision, Loadout),
}

impl Pair<'_> {
    fn position(&self, reach: Reach, stance: Stance) -> Result<PowerOnMachine, String> {
        position(self.program_rom, self.data.clone(), self.fighter, self.style, reach, stance)
    }
}

/// What a search found, and the ways to start a route it could not search
/// from, with why.
pub struct Searched {
    pub book: RouteBook,
    pub skipped: Vec<(Reach, String)>,
}

/// Search for the useful routes of a pair from `actions` (see
/// [`ground_actions`]): for each of a ground hit, a jump-in and a punish,
/// up to `budget.keep` routes of two hits or more, short ones included,
/// each measured against the reference defender. A jump-in or punish whose
/// reference position cannot be made is skipped and noted; a ground hit's
/// is the pair's fault.
pub fn search(pair: &Pair, actions: &[Action], budget: Budget) -> Result<Searched, String> {
    let mut routes = Vec::new();
    let mut skipped = Vec::new();
    for reach in [Reach::Ground, Reach::JumpIn, Reach::Punish] {
        let stance = Stance::Standing;
        let base = match pair.position(reach, stance) {
            Ok(base) => base,
            Err(why) if reach != Reach::Ground => {
                skipped.push((reach, why));
                continue;
            }
            Err(why) => return Err(why),
        };
        let foe = (stance.stick(), 0);
        let root = Node { after_pads: base.clone(), since: 0, steps: Vec::new(), hits: 0, ride: 0, health: health(&base), damage: 0, airborne: false };
        // A press that does nothing is no part of a route, and of the
        // presses that do the same thing one is enough.
        let mut kinds = Vec::new();
        let live: Vec<&Action> = actions
            .iter()
            .filter(|action| match what_it_does(&base, action, stance) {
                Some(kind) if !kinds.contains(&kind) => {
                    kinds.push(kind);
                    true
                }
                _ => false,
            })
            .collect();
        let mut frontier: Vec<Node> = Vec::new();
        match reach {
            Reach::JumpIn => frontier.extend(jump_in_actions().iter().filter_map(|a| expand(&root, a, 0, 60, foe))),
            _ => {
                // A punish must land while the defender is still recovering.
                let delays: &[usize] = if reach == Reach::Punish { &[0, 2, 4, 6, 8, 10, 12] } else { &[0] };
                for action in &live {
                    for &delay in delays {
                        let lead = if action.hold != 0 { 60 } else { 0 };
                        frontier.extend(expand(&root, action, delay + lead, 60, foe));
                    }
                }
            }
        }
        // One opener of each kind: those drawn in the same move that do the
        // same damage and ride on to the same count do the same. Not the
        // jump-ins: how deep one lands decides what links after it.
        // And a press that leaves the ground is a jump-in, not one of these.
        let mut seen = Vec::new();
        frontier.retain(|n| {
            let sig = (bodies(&n.after_pads)[0].move_id, n.since, n.damage, n.ride);
            let new = reach == Reach::JumpIn || (!n.airborne && !seen.contains(&sig));
            seen.push(sig);
            new
        });
        // Breadth first, a step deeper each pass: every opener is tried,
        // and after it the states that have done most damage.
        let mut found: Vec<Node> = Vec::new();
        for depth in 1..=MAX_STEPS {
            frontier.sort_by_key(|n| (std::cmp::Reverse(n.damage), n.since));
            if depth > 1 {
                frontier.truncate(budget.nodes);
            }
            let mut next: Vec<Node> = Vec::new();
            for node in &frontier {
                if node.ride >= 2 {
                    found.push(node.clone());
                }
                // More hits are already on their way from this state: a
                // press here would be credited with them.
                if node.ride > node.hits || depth == MAX_STEPS {
                    continue;
                }
                for action in &live {
                    // From the end of the last pads, before its hit lands
                    // (a cancel's motion begins there), to well after it.
                    for wait in 0..=node.since + if reach == Reach::JumpIn { 40 } else { 22 } {
                        let Some(child) = expand(node, action, wait, 45, foe) else { continue };
                        // One state per sequence of moves and count: the
                        // one that has done most.
                        match next.iter_mut().find(|n| n.names() == child.names() && n.hits == child.hits) {
                            Some(same) if same.damage >= child.damage => {}
                            Some(same) => *same = child,
                            None => next.push(child),
                        }
                    }
                }
            }
            frontier = next;
        }
        // One route per sequence of moves: the one that does most.
        let mut measured: Vec<Route> = Vec::new();
        for node in found {
            let Some(route) = measure(pair, reach, &base, node.steps) else { continue };
            match measured.iter_mut().find(|r| r.name() == route.name()) {
                Some(same) if same.damage >= route.damage => {}
                Some(same) => *same = route,
                None => measured.push(route),
            }
        }
        routes.extend(choose(measured, budget.keep));
    }
    Ok(Searched { book: RouteBook { fighter: pair.fighter, revision: pair.style.0, loadout: pair.style.1, routes }, skipped })
}

/// Keep `keep` routes, for variety before damage: the best of each length
/// that spends no Super meter, so short and cheap routes stay; then the
/// best of each length whatever it spends; then the best from each opener
/// not yet kept; then the rest by damage.
fn choose(mut routes: Vec<Route>, keep: usize) -> Vec<Route> {
    routes.sort_by_key(|r| (std::cmp::Reverse(r.damage), r.steps.len(), r.name()));
    let mut lengths: Vec<u8> = routes.iter().map(|r| r.hits).collect();
    lengths.sort_unstable();
    lengths.dedup();
    let mut order: Vec<&Route> = Vec::new();
    for free in [true, false] {
        for &hits in &lengths {
            order.extend(routes.iter().find(|r| r.hits == hits && (!free || r.super_spent == 0)));
        }
    }
    let mut openers: Vec<&str> = Vec::new();
    for route in &routes {
        if !openers.contains(&route.steps[0].name.as_str()) {
            openers.push(&route.steps[0].name);
            order.push(route);
        }
    }
    order.extend(&routes);
    let mut chosen: Vec<Route> = Vec::new();
    for route in order {
        if chosen.len() < keep && !chosen.contains(route) {
            chosen.push(route.clone());
        }
    }
    chosen.sort_by_key(|r| (std::cmp::Reverse(r.damage), r.name()));
    chosen
}

/// Measure each step's window and the time to confirm from `base`, the
/// route's reference position against a standing defender: the waits
/// around a step's own (the others kept) that change nothing the book
/// records of it: its hits, its damage, the meter it spends, and for a
/// punish that its first hit meets him recovering. Two motions close
/// together can read as a Super Art's and spend meter: a wait that does
/// that is outside the window. `first_hit` is the script frame the route's
/// first hit landed on at its recorded waits.
pub fn set_windows(base: &PowerOnMachine, route: &mut Route, first_hit: Option<usize>) {
    let reach = route.reach;
    let recorded = (route.hits, route.damage, route.super_spent, route.drive_spent);
    let same = |played: Played| {
        (played.hits, played.damage, played.super_spent, played.drive_spent) == recorded && (reach != Reach::Punish || played.punished)
    };
    let waits: Vec<u8> = route.steps.iter().map(|s| s.wait).collect();
    let mut windows = Vec::new();
    for (i, &wait) in waits.iter().enumerate() {
        let holds = |at: u8| {
            let mut tried = waits.clone();
            tried[i] = at;
            same(replay(base, &route.script(Some(&tried)), Stance::Standing))
        };
        let (mut low, mut high) = (wait, wait);
        // The opener's wait is where the route starts, except in a punish.
        if i > 0 || reach == Reach::Punish {
            while low > 0 && wait - low < 30 && holds(low - 1) {
                low -= 1;
            }
            while high - wait < 30 && high < u8::MAX && holds(high + 1) {
                high += 1;
            }
        }
        windows.push((low, high));
    }
    for (step, window) in route.steps.iter_mut().zip(windows) {
        step.window = window;
    }
    // The time to see the first hit before the second step must begin.
    route.confirm = 0;
    if let (Some(first_hit), Some(second)) = (first_hit, route.steps.get(1)) {
        let first = &route.steps[0];
        let latest = usize::from(first.wait) + first.pads.len() + usize::from(second.window.1);
        route.confirm = latest.saturating_sub(first_hit + 1).min(255) as u8;
    }
}

/// Frames after a cut that the defender is watched for coming out of his
/// hit reaction.
const CUT_WATCH: usize = 120;

/// The cut before step `step` of `route`, played from `base` (its reference
/// position against a defender in `stance`) at its recorded waits: see
/// [`Cut`]. `None` where the route may not be cut there: the steps before
/// it dropped or did not hit, the defender is not in a hit reaction at the
/// cut, he is knocked down, or the attacker is not free before he is.
pub fn cut_at(base: &PowerOnMachine, route: &Route, step: usize, stance: Stance) -> Option<Cut> {
    if step == 0 || step >= route.steps.len() {
        return None;
    }
    let script = route.script(None);
    let at = route.step_start(step);
    let mut m = base.clone();
    let foe = (stance.stick(), 0);
    let mut combo = Combo { count: 0, ..Combo::at(&m) };
    for &pad in &script[..at] {
        tick(&mut m, pad, foe);
        combo.follow(&m);
        if combo.dropped {
            return None;
        }
    }
    let knocked = |body: &Body| body.move_id == crate::combo::throw::GET_UP_MOVE || (body.move_id >= 0x30 && body.state & 1 != 0);
    // The last step before the cut can still be on its way in (its hit
    // lands some frames after its input): the cut needs that hit, then
    // the defender out of his reaction on his feet.
    let [me, _] = bodies(&m);
    let mut me_free = me.free().then_some(0);
    for frame in 1..=CUT_WATCH {
        tick(&mut m, (0, 0), foe);
        combo.follow(&m);
        let [me, defender] = bodies(&m);
        if combo.dropped || knocked(&defender) {
            return None;
        }
        if me_free.is_none() && me.free() {
            me_free = Some(frame);
        }
        if combo.count > 0 && !defender.reacting() {
            if !defender.free() && !defender.guarding() {
                return None;
            }
            let ahead = frame.checked_sub(me_free?).filter(|&a| a >= 1)?;
            return Some(Cut { step: step as u8, free: frame.min(255) as u8, ahead: ahead.min(255) as u8, gap: gap(&m) });
        }
    }
    None
}

/// Every cut `route` may take: each between two of its steps that holds
/// against a standing defender from `standing` and, if the route works on
/// a crouching one, from `crouching` too (with no crouching position, no
/// cut is marked on such a route).
pub fn measure_cuts(standing: &PowerOnMachine, crouching: Option<&PowerOnMachine>, route: &Route) -> Vec<Cut> {
    (1..route.steps.len())
        .filter_map(|step| {
            let cut = cut_at(standing, route, step, Stance::Standing)?;
            let crouch_ok = !route.stances.contains(&Stance::Crouching)
                || crouching.is_some_and(|base| cut_at(base, route, step, Stance::Crouching).is_some());
            crouch_ok.then_some(cut)
        })
        .collect()
}

/// Play the steps a search state holds and record the route they make, or
/// nothing if they do not replay as a combo of two hits or more.
fn measure(pair: &Pair, reach: Reach, base: &PowerOnMachine, steps: Vec<Step>) -> Option<Route> {
    let mut route = Route {
        reach,
        steps,
        hits: 0,
        damage: 0,
        true_combos: false,
        cancels: false,
        super_spent: 0,
        super_gained: 0,
        drive_spent: 0,
        drive_gained: 0,
        knockdown: false,
        corner: false,
        stances: vec![Stance::Standing],
        confirm: 0,
        gap: gap(base),
        fails_on: Vec::new(),
        cuts: Vec::new(),
    };
    let played = replay(base, &route.script(None), Stance::Standing);
    if played.hits < 2 || (reach == Reach::Punish && !played.punished) {
        return None;
    }
    route.hits = played.hits;
    route.damage = played.damage;
    route.super_spent = played.super_spent;
    route.super_gained = played.super_gained;
    route.drive_spent = played.drive_spent;
    route.drive_gained = played.drive_gained;
    route.knockdown = played.knockdown;
    set_windows(base, &mut route, played.first_hit);
    // A punish stays one only while its first hit meets a recovering defender.
    let same = |played: Played| played.hits == route.hits && played.damage == route.damage && (reach != Reach::Punish || played.punished);
    let script = route.script(None);
    // The rules it needs, the stances it works on, and how far apart it
    // can start: each is the same script played somewhere else.
    let needs = |switch_off: fn(&mut crate::config::CoreToggles)| {
        let mut without = base.clone();
        switch_off(&mut without.match_config_mut().toggles);
        !same(replay(&without, &script, Stance::Standing))
    };
    // An era style brings both rules with it whatever the switches say, so
    // the switches alone do not show what its routes need. What a route is
    // made of does: any second hit needs true combos, and a button pressed
    // while the last attack is still drawn needs cancels.
    route.true_combos = route.hits >= 2 || needs(|toggles| toggles.true_combos = false);
    route.cancels = played.cancelled || needs(|toggles| toggles.cancels = false);
    let crouching = pair.position(reach, Stance::Crouching).ok();
    if let Some(crouching) = &crouching {
        let played = replay(crouching, &script, Stance::Crouching);
        if played.hits == route.hits && (reach != Reach::Punish || played.punished) {
            route.stances.push(Stance::Crouching);
        }
    }
    if reach == Reach::Ground {
        let mut back = base.clone();
        for _ in 0..40 {
            tick(&mut back, (LEFT, 0), (0, 0));
            let mut settled = back.clone();
            for _ in 0..10 {
                tick(&mut settled, (0, 0), (0, 0));
            }
            if !same(replay(&settled, &script, Stance::Standing)) {
                break;
            }
            route.gap = route.gap.max(gap(&settled));
        }
    }
    route.cuts = measure_cuts(base, crouching.as_ref(), &route);
    Some(route)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_round_trip_through_their_text() {
        let pads = vec![(4, 0x10), (4, 0x10), (4, 0), (5, 0), (1, 0x04), (0, 0), (0, 0), (0, 0)];
        assert_eq!(pads_text(&pads), "4:10x2 4:00 5:00 1:04 0:00x3");
        assert_eq!(parse_pads(&pads_text(&pads)).unwrap(), pads);
        assert!(parse_pads("g:00").is_err() && parse_pads("1:40").is_err() && parse_pads("1:00x0").is_err());
    }

    #[test]
    fn a_book_with_a_missing_line_or_a_wait_outside_its_window_is_refused() {
        let book = "route-book 2\nfighter Ryu\nstyle Sf6/Standard\n\nroute ground\nname cr.MK\nhits 2\ndamage 100\nrules cancels\nmeter super -0 +0 drive -0 +0\nknockdown no\ncorner no\nstances standing\nconfirm 0\ngap 40\nfails-on \ncuts \nstep cr.MK | wait 0 0..0 | hold 0 | 4:10x2 4:00\nend\n";
        let parsed = RouteBook::parse(book).unwrap();
        assert_eq!(parsed.routes[0].steps[0].pads, [(4, 0x10), (4, 0x10), (4, 0)]);
        assert!(parsed.routes[0].cancels && !parsed.routes[0].true_combos);
        assert!(RouteBook::parse(&book.replace("corner no\n", "")).is_err());
        assert!(RouteBook::parse(&book.replace("wait 0 0..0", "wait 3 0..2")).is_err());
        assert!(RouteBook::parse(&book.replace("name cr.MK", "name cr.HK")).is_err());
        assert!(RouteBook::parse(&book.replace("style Sf6/Standard", "style Sf9/Standard")).is_err());
        assert!(RouteBook::parse(&format!("{book}stray line\n")).is_err(), "nothing may follow the last route");
        assert!(RouteBook::parse(&book.replace("rules cancels", "rules cancles")).is_err());
        let failing = RouteBook::parse(&book.replace("fails-on \n", "fails-on Sagat Sf6/Standard; Gen Sfz3/XIsm\n")).unwrap();
        assert_eq!(failing.routes[0].fails_on, ["Sagat Sf6/Standard", "Gen Sfz3/XIsm"]);
        assert_eq!(RouteBook::parse(&failing.to_text()).unwrap(), failing);
        assert!(RouteBook::parse(&book.replace("fails-on 
", "fails-on Sagat Sf9/Standard
")).is_err(), "an opponent that is not one");
        let one = RouteBook::parse(&book.replace("fails-on 
", "fails-on Sagat Sf6/Standard crouching; Gen Sfz3/XIsm
")).unwrap();
        let route = &one.routes[0];
        assert!(route.fails_against("Sagat Sf6/Standard", Stance::Crouching) && !route.fails_against("Sagat Sf6/Standard", Stance::Standing));
        assert!(route.fails_against("Gen Sfz3/XIsm", Stance::Standing) && route.fails_against("Gen Sfz3/XIsm", Stance::Crouching));
        assert!(!route.fails_against("Ryu Sf6/Standard", Stance::Standing));
        assert_eq!(opponent_key(TournamentFighter::Gen, (Revision::Sfz3, Loadout::XIsm)), "Gen Sfz3/XIsm");
        // A fighter alone is every style he has.
        let every = RouteBook::parse(&book.replace("fails-on \n", "fails-on Lee; Mike crouching\n")).unwrap();
        let route = &every.routes[0];
        assert!(route.fails_against("Lee Sf6/Standard", Stance::Standing) && route.fails_against("Lee Sfz3/XIsm", Stance::Crouching));
        assert!(route.fails_against("Mike St/Old", Stance::Crouching) && !route.fails_against("Mike St/Old", Stance::Standing));
        assert!(!route.fails_against("Ryu Sf6/Standard", Stance::Standing) && !route.fails_against("Leeds Sf6/Standard", Stance::Standing));
        assert_eq!(RouteBook::parse(&every.to_text()).unwrap(), every);
        assert!(RouteBook::parse(&book.replace("fails-on \n", "fails-on Leo\n")).is_err(), "no such fighter");
        assert!(RouteBook::parse(&book.replace("fails-on \n", "fails-on Lee kneeling\n")).is_err(), "no such stance");
    }

    #[test]
    fn a_routes_cut_points_round_trip_and_one_not_between_its_steps_is_refused() {
        let book = "route-book 2
fighter Ryu
style Sf6/Standard

route ground
name MP, 236+LP
hits 2
damage 300
rules true_combos cancels
meter super -0 +0 drive -0 +0
knockdown yes
corner no
stances standing
confirm 0
gap 52
fails-on 
cuts 1 free 24 ahead 3 gap 64
step MP | wait 0 0..0 | hold 0 | 0:02x2 0:00
step 236+LP | wait 0 0..5 | hold 0 | 4:00x2 5:00x2 1:00x2 1:01x3 1:00
end
";
        let parsed = RouteBook::parse(book).unwrap();
        assert_eq!(parsed.routes[0].cuts, [Cut { step: 1, free: 24, ahead: 3, gap: 64 }]);
        assert_eq!(RouteBook::parse(&parsed.to_text()).unwrap(), parsed);
        assert_eq!(parsed.to_text(), book.replace("fails-on 
", "fails-on
"));
        // No cuts: the line stays, empty.
        assert!(RouteBook::parse(&book.replace("cuts 1 free 24 ahead 3 gap 64", "cuts")).unwrap().routes[0].cuts.is_empty());
        assert!(RouteBook::parse(&book.replace("cuts 1 free 24 ahead 3 gap 64
", "")).is_err(), "the line is required");
        for bad in ["cuts 0 free 24 ahead 3 gap 64", "cuts 2 free 24 ahead 3 gap 64", "cuts 1 free 24 ahead 0 gap 64", "cuts 1 free 24 gap 64", "cuts 1 free 24 ahead 3 gap 64; 1 free 20 ahead 2 gap 60", "cuts 1 free 0 ahead 0 gap 64", "cuts 1 free 3 ahead 4 gap 64"] {
            assert!(RouteBook::parse(&book.replace("cuts 1 free 24 ahead 3 gap 64", bad)).is_err(), "{bad}");
        }
        assert!(RouteBook::parse(&book.replace("route-book 2", "route-book 1")).is_err(), "the old format");
    }
}
