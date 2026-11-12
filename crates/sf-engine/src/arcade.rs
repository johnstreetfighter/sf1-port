//! Arcade route, win quotes and ending content.

use crate::roster::TournamentFighter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Country {
    Japan,
    Usa,
    China,
    England,
    Thailand,
}

pub const ROUTE: [Country; 5] = [
    Country::Japan,
    Country::Usa,
    Country::China,
    Country::England,
    Country::Thailand,
];

impl Country {
    pub const fn name(self) -> &'static str {
        match self {
            Country::Japan => "Japan",
            Country::Usa => "USA",
            Country::China => "China",
            Country::England => "England",
            Country::Thailand => "Thailand",
        }
    }
}

pub fn route_from(start: Country) -> &'static [Country] {
    let at = ROUTE
        .iter()
        .position(|country| *country == start)
        .unwrap_or(0);
    &ROUTE[at..]
}

pub fn rival(country: Country) -> TournamentFighter {
    match country {
        Country::Japan => TournamentFighter::Retsu,
        Country::Usa => TournamentFighter::Joe,
        Country::China => TournamentFighter::Lee,
        Country::England => TournamentFighter::Birdie,
        Country::Thailand => TournamentFighter::Sagat,
    }
}

pub fn player_intro(fighter: TournamentFighter) -> &'static str {
    match fighter {
        TournamentFighter::Ryu => "I came for the fight. Nothing else.",
        TournamentFighter::Ken => "Long flight. Make it worth the ticket.",
        TournamentFighter::Retsu => "Your hands are high. Mine are already in.",
        TournamentFighter::Geki => "You blinked. I was already gone.",
        TournamentFighter::Joe => "Keep the guard up. The kick goes through it.",
        TournamentFighter::Mike => "No dancing. I walk until you don't.",
        TournamentFighter::Lee => "One step. That is the whole punch.",
        TournamentFighter::Gen => "Still standing. That should worry you.",
        TournamentFighter::Birdie => "Head down. You can guess the rest.",
        TournamentFighter::Eagle => "Sticks are up. Watch the buttons.",
        TournamentFighter::Adon => "You wanted the knee. Here it is.",
        TournamentFighter::Sagat => "High shot or low. Both leave a mark.",
    }
}

pub fn rival_taunt(fighter: TournamentFighter) -> &'static str {
    match fighter {
        TournamentFighter::Ryu => "The dojo is not a tour.",
        TournamentFighter::Ken => "Loud entrance. Show the punch.",
        TournamentFighter::Retsu => "Japan. Front foot. Now.",
        TournamentFighter::Geki => "The shadow moves before you do.",
        TournamentFighter::Joe => "USA. Boots stay on.",
        TournamentFighter::Mike => "Small ring. You will find the rope.",
        TournamentFighter::Lee => "China. Eyes up.",
        TournamentFighter::Gen => "You are early.",
        TournamentFighter::Birdie => "England. I charge.",
        TournamentFighter::Eagle => "Two sticks. Count them.",
        TournamentFighter::Adon => "Thailand opens with a kick.",
        TournamentFighter::Sagat => "Last wall. The tiger is awake.",
    }
}

pub fn win_quote(fighter: TournamentFighter) -> &'static str {
    match fighter {
        TournamentFighter::Ryu => "Still walking.",
        TournamentFighter::Ken => "Round's mine. Hold the plane.",
        TournamentFighter::Retsu => "Quick hands. Slow feet.",
        TournamentFighter::Geki => "No echo.",
        TournamentFighter::Joe => "The legs got there first.",
        TournamentFighter::Mike => "Stay down. The bell rang.",
        TournamentFighter::Lee => "Told you. One step.",
        TournamentFighter::Gen => "Age hits harder than pride.",
        TournamentFighter::Birdie => "Horns down. Next town.",
        TournamentFighter::Eagle => "Sticks back in the case.",
        TournamentFighter::Adon => "The knee found you.",
        TournamentFighter::Sagat => "The shot missed. The knee did not.",
    }
}

/// Two lines for the native post-match card, with a separate Thailand finale.
pub(crate) fn win_quote_lines(fighter: TournamentFighter, finale: bool) -> [&'static str; 2] {
    use TournamentFighter::*;
    if finale {
        return match fighter {
            Ryu => ["THE TIGER HAS FALLEN.", "I STILL HAVE FAR TO WALK."],
            Ken => ["THAT WAS YOUR BEST, SAGAT?", "NEXT STOP, HOME."],
            Retsu => ["YOUR KNEE WAS SWIFT, TIGER.", "YOUR PRIDE WAS SLOWER."],
            Geki => ["THE TIGER DID NOT HEAR ME.", "THAILAND HAS GONE QUIET."],
            Joe => ["YOUR ARENA, MY RULES.", "THE BELT IS COMING HOME."],
            Mike => ["THE TIGER IS ON THE CANVAS.", "COUNT HIM OUT."],
            Lee => ["ONE STEP INSIDE YOUR KNEE.", "THAT WAS ALL I NEEDED."],
            Gen => ["THE TIGER HAS HIS ANSWER.", "I SEEK ANOTHER."],
            Birdie => ["TIGER DOWN. HORNS UP.", "WHERE'S THE FEAST?"],
            Eagle => ["A WORTHY DUEL, SAGAT.", "LONDON WILL HEAR OF IT."],
            Adon => ["YOU TAUGHT ME WELL, MASTER.", "THE JAGUAR REIGNS NOW."],
            Sagat => ["YOU HAVE COURAGE, RYU.", "THE TIGER STILL REIGNS."],
        };
    }
    match fighter {
        Ryu => ["STILL WALKING.", "THERE IS MORE TO LEARN."],
        Ken => ["ROUND'S MINE.", "HOLD THE PLANE."],
        Retsu => ["QUICK HANDS. SLOW FEET.", "PRACTISE YOUR FOOTWORK."],
        Geki => ["NO ECHO.", "NOT EVEN A FOOTSTEP."],
        Joe => ["THE LEGS GOT THERE FIRST.", "KEEP YOUR GUARD UP."],
        Mike => ["STAY DOWN. THE BELL RANG.", "SAVE IT FOR THE REMATCH."],
        Lee => ["TOLD YOU. ONE STEP.", "YOU NEVER SAW IT COMING."],
        Gen => ["AGE HITS HARDER THAN PRIDE.", "REMEMBER THE DIFFERENCE."],
        Birdie => ["HORNS DOWN. NEXT TOWN.", "I COULD USE A BITE."],
        Eagle => ["STICKS BACK IN THE CASE.", "GOOD DAY TO YOU."],
        Adon => ["THE KNEE FOUND YOU.", "THE JAGUAR STRIKES FIRST."],
        Sagat => ["THE SHOT MISSED.", "THE KNEE DID NOT."],
    }
}

/// The ten bosses' native ending messages. Ryu and Ken use the ROM message.
pub(crate) fn ending_epilogue(fighter: TournamentFighter) -> Option<&'static str> {
    use TournamentFighter::*;
    Some(match fighter {
        Ryu | Ken => return None,
        Retsu => "HAVING TESTED EVERY MASTER OF THE WORLD TOURNAMENT, RETSU RETURNS TO THE SHORINJI TEMPLE. HIS KIAI STRIKES ECHO THROUGH THE COURTYARD AS NEW DISCIPLES GATHER AT THE GATE.",
        Geki => "THE TOURNAMENT ENDS IN SILENCE. BEFORE THE CROWD CAN SPEAK HIS NAME, GEKI VANISHES INTO THE NIGHT SHADOWS, HIS CONTRACT FULFILLED, HIS CLAW UNBLEMISHED.",
        Joe => "BELT SLUNG OVER HIS SHOULDER, JOE WALKS OUT OF THE THAILAND ARENA UNDEFEATED. FROM THE BACK ALLEYS TO THE WORLD STAGE, THE AMERICAN CHAMP STANDS ALONE.",
        Mike => "NONE COULD WITHSTAND THE IRON FISTS OF MIKE. HAVING CRUSHED EVERY CHALLENGER ON EARTH, THE HEAVYWEIGHT POWERHOUSE SETS HIS SIGHTS ON A NEW RING.",
        Lee => "WITH BAJIQUAN PERFECTED AGAINST THE WORLD'S GREATEST FIGHTERS, LEE BOWS TO THE FOUR WINDS AND RETURNS TO THE GREAT WALL TO GUARD CHINA'S MARTIAL LEGACY.",
        Gen => "NO ASSASSIN NOR EMPEROR COULD FELL THE MASTER OF MANTIS AND CRANE. GEN COUGHS SOFTLY AND WALKS AWAY INTO THE MIST, STILL SEEKING ONE FINAL WORTHY DEATH MATCH.",
        Birdie => "THE CROWD ROARS AS BIRDIE RAISES A MASSIVE FIST IN TRIUMPH. ENGLAND'S BRAWLER OWNS THE STREETS AND THE WORLD TITLE, AND RECKONS A PROPER FEAST IS NEXT.",
        Eagle => "TWO KALI STICKS RETURN CLEANLY TO THEIR CASE. HAVING PROVED THE ART OF GENTLEMAN'S COMBAT AGAINST EVERY STYLE ON EARTH, EAGLE TIPS HIS HAT AND DEPARTS LONDON.",
        Adon => "HAVING SURPASSED EVEN HIS MASTER'S LEGEND IN THE TEMPLE COURTYARD, ADON STANDS ATOP MUAY THAI AS ITS NEW EMPEROR. THE JAGUAR REIGNS SUPREME.",
        Sagat => "THE YOUNG CHALLENGERS FELL ONE BY ONE BEFORE THE TIGER'S WRATH. UNSCARRED AND UNBROKEN, SAGAT STANDS ATOP THE WORLD AS THE UNDISPUTED EMPEROR OF MUAY THAI.",
    })
}
