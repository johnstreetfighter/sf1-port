//! Stable gameplay-state encoding for rollback checksums (NP1).
//!
//! Scalars use little-endian bytes; collections carry a u64 length and
//! options carry a one-byte presence tag. Named records and enums have
//! explicit schema tags. This is a checksum encoding, not a save format.

use std::collections::VecDeque;

pub(crate) struct CanonicalHasher(u64);

impl CanonicalHasher {
    pub(crate) fn new() -> Self { Self(0xcbf2_9ce4_8422_2325) }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
        }
    }

    pub(crate) fn tag(&mut self, schema: &str) { schema.encode(self); }
    pub(crate) fn finish(self) -> u64 { self.0 }
}

pub(crate) trait Canonical {
    fn encode(&self, out: &mut CanonicalHasher);
}

macro_rules! scalar {
    ($($ty:ty),* $(,)?) => { $(
        impl Canonical for $ty {
            fn encode(&self, out: &mut CanonicalHasher) { out.write(&self.to_le_bytes()); }
        }
    )* };
}
scalar!(u8, u16, u32, u64, i8, i16, i32, i64);

impl Canonical for bool {
    fn encode(&self, out: &mut CanonicalHasher) { u8::from(*self).encode(out); }
}
impl Canonical for usize {
    fn encode(&self, out: &mut CanonicalHasher) { u64::try_from(*self).expect("length fits u64").encode(out); }
}
impl<T: Canonical + ?Sized> Canonical for &T {
    fn encode(&self, out: &mut CanonicalHasher) { T::encode(*self, out); }
}
impl<T: Canonical> Canonical for Option<T> {
    fn encode(&self, out: &mut CanonicalHasher) {
        match self {
            None => 0u8.encode(out),
            Some(value) => { 1u8.encode(out); value.encode(out); }
        }
    }
}
impl<T: Canonical> Canonical for [T] {
    fn encode(&self, out: &mut CanonicalHasher) {
        self.len().encode(out);
        for value in self { value.encode(out); }
    }
}
impl<T: Canonical, const N: usize> Canonical for [T; N] {
    fn encode(&self, out: &mut CanonicalHasher) { self.as_slice().encode(out); }
}
impl<T: Canonical> Canonical for Vec<T> {
    fn encode(&self, out: &mut CanonicalHasher) { self.as_slice().encode(out); }
}
impl<T: Canonical> Canonical for VecDeque<T> {
    fn encode(&self, out: &mut CanonicalHasher) {
        self.len().encode(out);
        for value in self { value.encode(out); }
    }
}
impl Canonical for str {
    fn encode(&self, out: &mut CanonicalHasher) { self.as_bytes().encode(out); }
}
impl Canonical for String {
    fn encode(&self, out: &mut CanonicalHasher) { self.as_str().encode(out); }
}
impl<A: Canonical, B: Canonical> Canonical for (A, B) {
    fn encode(&self, out: &mut CanonicalHasher) { self.0.encode(out); self.1.encode(out); }
}
impl<A: Canonical, B: Canonical, C: Canonical> Canonical for (A, B, C) {
    fn encode(&self, out: &mut CanonicalHasher) { self.0.encode(out); self.1.encode(out); self.2.encode(out); }
}

// Exhaustive destructuring makes adding a field require an encoding audit.
macro_rules! canonical_struct {
    ($ty:ident, $schema:literal, { $($field:ident),* $(,)? }) => {
        impl $crate::canonical::Canonical for $ty {
            fn encode(&self, out: &mut $crate::canonical::CanonicalHasher) {
                let Self { $($field),* } = self;
                out.tag($schema);
                $($field.encode(out);)*
            }
        }
    };
}
pub(crate) use canonical_struct;

macro_rules! canonical_enum {
    ($ty:ident, $schema:literal; $( $variant:ident $( ( $($arg:ident),* ) )? $( { $($field:ident),* } )? => $tag:literal ),* $(,)?) => {
        impl $crate::canonical::Canonical for $ty {
            fn encode(&self, out: &mut $crate::canonical::CanonicalHasher) {
                out.tag($schema);
                match self {
                    $(Self::$variant $( ( $($arg),* ) )? $( { $($field),* } )? => {
                        let variant_tag: u16 = $tag;
                        variant_tag.encode(out);
                        $($($arg.encode(out);)*)?
                        $($($field.encode(out);)*)?
                    }),*
                }
            }
        }
    };
}
pub(crate) use canonical_enum;

#[cfg(test)]
mod tests {
    use super::{Canonical, CanonicalHasher};

    #[test]
    fn fnv1a_matches_published_64_bit_vectors() {
        for (input, expected) in [
            (b"".as_slice(), 0xcbf2_9ce4_8422_2325),
            (b"a".as_slice(), 0xaf63_dc4c_8601_ec8c),
            (b"hello".as_slice(), 0xa430_d846_80aa_bd0b),
        ] {
            let mut out = CanonicalHasher::new();
            out.write(input);
            assert_eq!(out.finish(), expected);
        }
    }

    #[test]
    fn canonical_lengths_and_option_tags_distinguish_different_state() {
        fn checksum(value: &impl Canonical) -> u64 {
            let mut out = CanonicalHasher::new();
            value.encode(&mut out);
            out.finish()
        }
        assert_ne!(checksum(&None::<u8>), checksum(&Some(0u8)));
        assert_ne!(checksum(&vec!["ab", "c"]), checksum(&vec!["a", "bc"]));
        assert_ne!(checksum(&vec![vec![1u8], vec![2]]), checksum(&vec![vec![1u8, 2]]));
    }
}
