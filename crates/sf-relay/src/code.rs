//! Internet room codes: six symbols shown as `K7Q-MPX`.

use std::fmt;

/// Digits and capitals without the lookalikes 0, O, 1, I and L. The missing
/// `1` also keeps every code distinct from the `SF1-` address codes.
pub const ALPHABET: &[u8; 31] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
pub const SYMBOLS: usize = 6;

/// A relay room's code, unique among that relay's open rooms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoomCode([u8; SYMBOLS]);

impl RoomCode {
    /// How many distinct codes exist (31^6).
    pub const COUNT: u64 = 887_503_681;

    /// Read a typed or pasted code in any case, with or without the dash
    /// after the third symbol. Surrounding whitespace is ignored.
    pub fn parse(input: &str) -> Option<Self> {
        let text = input.trim().as_bytes();
        let symbols: Vec<u8> = match text.len() {
            SYMBOLS => text.to_vec(),
            7 if text[3] == b'-' => [&text[..3], &text[4..]].concat(),
            _ => return None,
        };
        Self::from_bytes(symbols.to_ascii_uppercase().try_into().ok()?)
    }

    /// The code at `index` modulo [`Self::COUNT`], most significant first.
    pub fn from_index(index: u64) -> Self {
        let mut rest = index % Self::COUNT;
        let mut symbols = [0; SYMBOLS];
        for symbol in symbols.iter_mut().rev() {
            *symbol = ALPHABET[(rest % ALPHABET.len() as u64) as usize];
            rest /= ALPHABET.len() as u64;
        }
        Self(symbols)
    }

    pub fn index(self) -> u64 {
        self.0.iter().fold(0, |index, symbol| {
            index * ALPHABET.len() as u64 + position(*symbol).unwrap_or(0) as u64
        })
    }

    /// The six uppercase symbols as they travel on the wire.
    pub fn bytes(self) -> [u8; SYMBOLS] {
        self.0
    }

    pub fn from_bytes(bytes: [u8; SYMBOLS]) -> Option<Self> {
        bytes.iter().all(|symbol| position(*symbol).is_some()).then_some(Self(bytes))
    }
}

fn position(symbol: u8) -> Option<usize> {
    ALPHABET.iter().position(|&known| known == symbol)
}

impl fmt::Display for RoomCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = |part: &[u8]| part.iter().map(|&symbol| char::from(symbol)).collect::<String>();
        write!(f, "{}-{}", text(&self.0[..3]), text(&self.0[3..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_read_in_any_case_with_or_without_the_dash() {
        let code = RoomCode::parse("K7Q-MPX").expect("a valid code");
        assert_eq!(code.to_string(), "K7Q-MPX");
        for typed in ["k7q-mpx", "K7QMPX", " k7qmpx \r\n", "k7Q-mPx"] {
            assert_eq!(RoomCode::parse(typed), Some(code), "{typed:?}");
        }
    }

    #[test]
    fn codes_never_use_lookalike_symbols_or_address_forms() {
        for typed in [
            "K7Q-MP0", "K7Q-MPO", "K7Q-MP1", "K7Q-MPI", "K7Q-MPL", // lookalikes
            "K7Q-MP", "K7Q-MPXX", "K7-QMPX", "K7Q--MPX", "", // shape
            "SF1-7F00-0001", "127.0.0.1:7000", // address codes stay addresses
        ] {
            assert_eq!(RoomCode::parse(typed), None, "{typed:?}");
        }
    }

    #[test]
    fn every_index_gives_a_distinct_code_that_reads_back() {
        let last = RoomCode::COUNT - 1;
        for index in [0, 1, 30, 31, 12_345_678, last] {
            let code = RoomCode::from_index(index);
            assert_eq!(code.index(), index);
            assert_eq!(RoomCode::parse(&code.to_string()), Some(code));
        }
        assert_eq!(RoomCode::from_index(0).to_string(), "222-222");
        assert_eq!(RoomCode::from_index(last).to_string(), "ZZZ-ZZZ");
        assert_eq!(RoomCode::from_index(RoomCode::COUNT), RoomCode::from_index(0));
    }

    #[test]
    fn wire_bytes_round_trip_and_reject_foreign_symbols() {
        let code = RoomCode::parse("K7Q-MPX").unwrap();
        assert_eq!(RoomCode::from_bytes(code.bytes()), Some(code));
        assert_eq!(RoomCode::from_bytes(*b"K7QMP0"), None);
    }
}
