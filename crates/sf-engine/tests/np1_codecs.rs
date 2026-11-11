use sf_engine::front::session::PlayerInputs;
use sf_engine::netplay::{decode_inputs, decode_room_code, encode_inputs, encode_room_code};

#[test]
fn player_inputs_use_the_ten_known_wire_bits() {
    for (directions, bits) in [
        ([true, false, false, false], 0x0001),
        ([false, true, false, false], 0x0002),
        ([false, false, true, false], 0x0004),
        ([false, false, false, true], 0x0008),
    ] {
        let inputs = PlayerInputs { directions, buttons: [false; 6] };
        assert_eq!(encode_inputs(inputs), bits);
        assert_eq!(decode_inputs(bits), inputs);
    }
    for (buttons, bits) in [
        ([true, false, false, false, false, false], 0x0010),
        ([false, true, false, false, false, false], 0x0020),
        ([false, false, true, false, false, false], 0x0040),
        ([false, false, false, true, false, false], 0x0080),
        ([false, false, false, false, true, false], 0x0100),
        ([false, false, false, false, false, true], 0x0200),
    ] {
        let inputs = PlayerInputs { directions: [false; 4], buttons };
        assert_eq!(encode_inputs(inputs), bits);
        assert_eq!(decode_inputs(bits), inputs);
    }
}

#[test]
fn every_combination_of_direction_and_buttons_roundtrips() {
    for bits in 0_u16..1024 {
        let inputs = PlayerInputs {
            directions: std::array::from_fn(|i| bits & (1 << i) != 0),
            buttons: std::array::from_fn(|i| bits & (1 << (i + 4)) != 0),
        };
        assert_eq!(decode_inputs(encode_inputs(inputs)), inputs, "{bits:#06x}");
        assert_eq!(encode_inputs(inputs), bits);
    }
}

#[test]
fn reserved_transport_bits_never_become_gameplay_inputs() {
    assert_eq!(decode_inputs(0xfc00), PlayerInputs::default());
    for bits in 0_u16..=u16::MAX {
        assert_eq!(encode_inputs(decode_inputs(bits)), bits & 0x03ff);
    }
}

#[test]
fn default_port_room_codes_match_the_documented_address() {
    assert_eq!(encode_room_code([127, 0, 0, 1], 7000), "SF1-7F00-0001");
    assert_eq!(decode_room_code("SF1-7F00-0001"), Some(([127, 0, 0, 1], 7000)));
}

#[test]
fn room_codes_accept_custom_ports_and_raw_ipv4_endpoints() {
    assert_eq!(encode_room_code([192, 168, 1, 2], 8080), "SF1-C0A8-0102-1F90");
    for (text, expected) in [
        ("SF1-C0A8-0102-1F90", ([192, 168, 1, 2], 8080)),
        ("SF1-c0a8-0102-1f90", ([192, 168, 1, 2], 8080)),
        ("SF1-7F00-0001-1B58", ([127, 0, 0, 1], 7000)),
        ("192.168.1.2:8080", ([192, 168, 1, 2], 8080)),
        ("127.0.0.1:7000", ([127, 0, 0, 1], 7000)),
    ] {
        assert_eq!(decode_room_code(text), Some(expected), "{text}");
    }
}

#[test]
fn room_codes_roundtrip_ipv4_and_port_boundaries() {
    assert_eq!(encode_room_code([0, 0, 0, 0], 1), "SF1-0000-0000-0001");
    assert_eq!(encode_room_code([255, 255, 255, 255], 65535), "SF1-FFFF-FFFF-FFFF");
    assert_eq!(encode_room_code([1, 2, 3, 4], 7000), "SF1-0102-0304");
    for ipv4 in [[0, 0, 0, 0], [255, 255, 255, 255], [1, 2, 3, 4], [192, 168, 1, 2]] {
        for port in [1, 1023, 1024, 6999, 7000, 7001, 32768, 65535] {
            assert_eq!(decode_room_code(&encode_room_code(ipv4, port)), Some((ipv4, port)));
        }
    }
    assert_eq!(decode_room_code("0.0.0.0:1"), Some(([0, 0, 0, 0], 1)));
    assert_eq!(decode_room_code("255.255.255.255:65535"), Some(([255, 255, 255, 255], 65535)));
}

#[test]
fn malformed_room_endpoints_are_rejected() {
    for text in [
        "SF1-7F00-0001-0000", "127.0.0.1:0", "",
        "SF1", "SF1-7F00", "SF1-7F00-000", "SF1-7F00-00001",
        "SF1-7F00-0001-001", "SF1-7F00-0001-10000", "SF1-7F00-0001-1B58-extra",
        "SF1-+001-0001", "SF1-7F00-+001", "SF1-7F00-0001-+001", "SF1-7F00-ZZZZ",
        "SF1--0001", "SF1-7F00-0001-", "SF2-7F00-0001", "sf1-7F00-0001",
        " SF1-7F00-0001", "SF1-7F00-0001 ", "SF1-7F00-0001\n",
        "127.0.0.1", "127.0.0.1:", "127.0.0.1:65536", "127.0.0.1:-1", "127.0.0.1:+1",
        "127.0.0.1:7000:7001", "256.0.0.1:7000", "127.0.0:7000", "127.00.0.1:7000",
        "127.0.0.1:seven", "localhost:7000", "[::1]:7000", "127.0.0.1:7000 ",
    ] {
        assert_eq!(decode_room_code(text), None, "{text:?}");
    }
}

#[test]
#[should_panic(expected = "room code port must be nonzero")]
fn encoding_a_room_code_rejects_the_unconnectable_zero_port() {
    encode_room_code([127, 0, 0, 1], 0);
}
