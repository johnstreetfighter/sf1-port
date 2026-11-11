//! Room browsing and configuration identity (`#112`, `O21`). Native rollback
//! runs in np2_rollback and netplay_torture instead of the retired prototype.

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::GaugeMode;
use sf_engine::netplay::{config_hash, detect_link, Browser, Link, Room, RoomKind};

fn room(region: &str, link: Link, kind: RoomKind) -> Room {
    Room {
        code: format!("{region}-1"),
        host: "203.0.113.8".into(),
        region: region.into(),
        ping_ms: 40,
        rollback_frames: 3,
        jitter_ms: 2,
        link,
        kind,
        spectators: if kind == RoomKind::Spectate { 2 } else { 0 },
        preset: RulesetPreset::DefinitiveOverhaul,
        in_match: false,
        what_if: false,
        bonus: false,
    }
}

#[test]
fn configuration_hash_covers_synchronized_rules() {
    let host = MatchConfig::faithful();
    let seed = 99;
    let hash = config_hash(&host, seed);
    assert_eq!(config_hash(&host, seed), hash);
    let mut other = host;
    other.gauge_mode = GaugeMode::NoGauges;
    assert_ne!(config_hash(&other, seed), hash);
    assert_ne!(config_hash(&MatchConfig::definitive(), seed), hash);
}

#[test]
fn browser_has_no_region_lock_and_can_hide_wifi() {
    let rooms = vec![
        room("JP", Link::Ethernet, RoomKind::Versus),
        room("BR", Link::Wifi, RoomKind::Dramatic),
        room("DE", Link::Ethernet, RoomKind::Spectate),
    ];
    let open = Browser { wired_only: false, what_if: None, bonus: None };
    let shown = open.visible(&rooms);
    assert_eq!(shown.len(), 3);
    assert!(shown.iter().any(|room| room.region == "BR" && room.kind == RoomKind::Dramatic));
    assert!(shown.iter().any(|room| room.spectators == 2));
    let mut wired = Browser { wired_only: true, what_if: None, bonus: None };
    assert_eq!(wired.visible(&rooms).len(), 2);
    assert!(!wired.accept(&rooms[1]));
    assert!(wired.accept(&rooms[0]));
    let saved = wired.save();
    wired = Browser::load(&saved);
    assert!(wired.wired_only);
    assert_eq!(detect_link("Intel Wi-Fi 6"), Link::Wifi);
    assert_eq!(detect_link("Realtek Ethernet"), Link::Ethernet);
}
