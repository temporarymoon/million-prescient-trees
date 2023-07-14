use const_for::const_for;
use core::fmt;
use std::ops::{BitOr, Not, BitAnd};

/// Bitfield containing up to 16 bits.
/// Internally used to implement stuff like creature sets,
/// edict sets, effect sets, and more.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Bitfield(u16);

// {{{ Main definitions
impl Bitfield {
    #[inline(always)]
    pub const fn new(x: u16) -> Self {
        Bitfield(x)
    }

    /// Construct a bitfield containing a single bit.
    #[inline(always)]
    pub fn singleton<T: Into<u8>>(x: T) -> Self {
        let u = x.into();
        assert!(u <= 16);
        Bitfield(1 << u)
    }

    /// Returns a bitfield with a given amount of ones at the end.
    ///
    /// # Examples
    ///
    /// ```
    /// n_ones(4) // 0x000F
    /// ```
    pub fn n_ones(n: usize) -> Self {
        if n == 16 {
            Bitfield::all()
        } else {
            Bitfield::new((1 << n) - 1)
        }
    }

    /// Returns a bitfield containing only ones.
    pub fn all() -> Self {
        Bitfield::new(u16::MAX)
    }

    /// Checks if the bitfield contains an one at some index.
    ///
    /// # Examples
    ///
    /// ```
    /// has(0b0100, 1) // false
    /// has(0b0100, 2) // true
    /// ```
    pub const fn has(self, index: usize) -> bool {
        ((self.0 >> (index as u16)) & 1) != 0
    }

    /// Adds a bit to a bitfield.
    /// Errors out if the bit is already there.
    ///
    /// # Examples
    ///
    /// ```
    /// add(0b0100, 1) // 0b0110
    /// ```
    pub fn add(&mut self, index: usize) {
        if self.has(index) {
            panic!(
                "Trying to remove index {} that's not here {:b}",
                index, self.0
            )
        }

        self.0 = self.0 | (1 << (index as u16))
    }

    /// Removes a bit from a bitfield.
    /// Errors out if the bit is already there.
    /// # Examples
    ///
    /// ```
    /// add(0b0110, 1) // 0b0100
    /// ```
    pub fn remove(&mut self, index: usize) {
        if !self.has(index) {
            panic!(
                "Trying to remove index {} that's not here {:b}",
                index, self.0
            )
        }
        self.0 = self.0 ^ (1 << (index as u16))
    }

    /// Sets all bits to one.
    #[inline(always)]
    pub fn fill(&mut self) {
        self.0 = u16::MAX;
    }

    /// Sets all bits to zero.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Flips the last n bits.
    ///
    /// # Examples
    ///
    /// ```
    /// invert_last_n(0b0110, 2) // 0b0101
    /// ```
    pub fn invert_last_n(self, n: usize) -> Self {
        if n == 16 {
            !self
        } else {
            let mask = (1 << n) - 1;
            Bitfield(self.0 ^ mask)
        }
    }

    /// Returns the number of ones inside self.
    ///
    /// # Examples
    ///
    /// ```
    /// len(0b101011) // 4
    /// ```
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Return the number of ones between a given index and the end.
    ///  
    /// # Arguments
    ///
    /// * `target` - The index to count ones after.
    ///
    /// # Examples
    ///
    /// ```
    /// count_from_end(0b0100, 2) // 0
    /// count_from_end(0b0101, 2) // 1
    /// count_from_end(0b0111, 2) // 2
    /// ```
    pub fn count_from_end(self, target: usize) -> usize {
        (self.0 & ((1 << target) - 1)).count_ones() as usize
    }

    /// Returns the position (starting from the end) of the nth bit.
    ///
    /// # Examples
    ///
    /// ```
    /// lookup_from_end(0b010101, 2) // Some(4)
    /// lookup_from_end(0b010101, 3) // Some(4)
    /// ```
    pub fn lookup_from_end(self, index: usize) -> Option<usize> {
        (0..16)
            .enumerate()
            .filter(|(_, x)| self.has(*x))
            .nth(index)
            .map(|(i, _)| i)
    }

    /// Encode a bitfield as a subset of another bitfield.
    /// Bits are shifted to the left such that all zero bits in the super-bitfield
    /// are not present in the sub-bitfield.
    ///
    /// Properties:
    /// - the empty bitfield acts as a left zero elements
    /// - the full bitfield acts as a right identity
    pub fn encode_relative_to(self, other: Self) -> Self {
        let mut result = Bitfield::default();

        for i in 0..16 {
            if self.has(i) {
                assert!(
                    other.has(i),
                    "{:?} contains bits not contained in {:?}",
                    self,
                    other
                );

                result.add(other.count_from_end(i))
            }
        }

        result
    }

    /// Inverse of `encode_relative_to`.
    pub fn decode_relative_to(self, other: Bitfield) -> Option<Self> {
        let mut result = Bitfield::default();

        for i in 0..16 {
            if self.has(i) {
                result.add(other.lookup_from_end(i)?);
            }
        }

        Some(result)
    }
}
// }}}
// {{{ Ones encoding
pub mod one_encoding {
    // {{{ Implementation
    use crate::helpers::choose::choose;

    use super::*;

    /// The maximum number we want to encode (in this case, we want to encode all u16s)
    const DECODED_COUNT: usize = (u16::MAX as usize) + 1;

    /// The number of possible number of bits in a int smaller than `MAX_DECODED`.
    const BIT_CASES: usize = 17;

    /// For efficiency, we store all the decode tables in the same array,
    /// starting at differet indices.
    ///
    /// An index is simply equal to the previous one, plus the number
    /// of spots required by the previous table (in this case, `16 choose i - 1`).
    const MAGIC_INDICES: [usize; BIT_CASES] = {
        let mut results = [0; BIT_CASES];

        const_for!(i in 1..BIT_CASES => {
            results[i] = results[i - 1] + choose(16, i - 1);
        });

        results
    };

    /// The lookup tables required for encoding!
    /// - the first table maps raw values to encoded values.
    /// - the second array contains 17 distinct decoding tables
    ///   concatenated together. The nth table maps encoded values
    ///   to respective raw values with n-ones inside.
    /// - the third table contains the number of entries in each table
    ///   contained in the second array (used for testing / asserts).
    const LOOKUP_TABLES: (
        [u16; DECODED_COUNT],
        [u16; DECODED_COUNT],
        [usize; BIT_CASES],
    ) = {
        let mut encode = [0 as u16; DECODED_COUNT];
        let mut decode = [0 as u16; DECODED_COUNT];
        let mut lengths = [0 as usize; BIT_CASES];

        const_for!(decoded in 0..DECODED_COUNT => {
            let count = Bitfield::new(decoded as u16).len() as usize;
            let encoded = lengths[count];
            decode[MAGIC_INDICES[count] + encoded] = decoded as u16;
            encode[decoded] = encoded as u16;
            lengths[count] += 1;
        });

        (encode, decode, lengths)
    };

    impl Bitfield {
        /// Efficiently assume the number of ones in the bit
        /// representation of a number is known, removing such
        /// useless information.
        ///
        /// The result fits inside an u16,
        /// but we pass around an `usize` for convenience.
        #[inline(always)]
        pub fn encode_ones(self) -> usize {
            LOOKUP_TABLES.0[self.0 as usize] as usize
        }

        /// Readd the information removed by `encode_ones`.
        #[inline(always)]
        pub fn decode_ones(encoded: usize, ones: usize) -> Option<Self> {
            if encoded >= *LOOKUP_TABLES.2.get(ones)? {
                return None;
            } else {
                return Some(Bitfield::new(
                    *LOOKUP_TABLES.1.get(MAGIC_INDICES.get(ones)? + encoded)?,
                ));
            }
        }
    }
    // }}}
    // {{{ Tests
    #[cfg(test)]
    mod tests {
        use std::assert_eq;

        use super::*;

        /// Checks that all the spots in the decoding lookup table have been used up.
        #[test]
        fn lengths_equal_to_magic_index_diffs() {
            for i in 0..(BIT_CASES - 1) {
                assert_eq!(
                    LOOKUP_TABLES.2[i],
                    MAGIC_INDICES[i + 1] - MAGIC_INDICES[i],
                    "Failed diff for index {}",
                    i
                );
            }
        }

        #[test]
        fn encode_decode_identity() {
            for i in 0..=u16::MAX {
                let bitfield = Bitfield::new(i);

                assert_eq!(
                    Some(bitfield),
                    Bitfield::decode_ones(bitfield.encode_ones(), bitfield.len())
                );
            }
        }

        #[test]
        fn decode_encode_identity() {
            for ones in 0..=16 {
                for i in 0..=(u16::MAX as usize) {
                    let decode_encode_i =
                        Bitfield::decode_ones(i, ones).map(|bitfield| bitfield.encode_ones());

                    match decode_encode_i {
                        None => break,
                        Some(decode_encode_i) => assert_eq!(decode_encode_i, i),
                    }
                }
            }
        }
    }
    // }}}
}
// }}}
// {{{ Trait implementations
impl Default for Bitfield {
    fn default() -> Self {
        Bitfield::new(0)
    }
}

impl fmt::Debug for Bitfield {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

impl Into<u64> for Bitfield {
    fn into(self) -> u64 {
        return self.0.into();
    }
}

impl Into<u32> for Bitfield {
    fn into(self) -> u32 {
        return self.0.into();
    }
}

impl Into<usize> for Bitfield {
    fn into(self) -> usize {
        return self.0.into();
    }
}

impl Into<u16> for Bitfield {
    fn into(self) -> u16 {
        return self.0.into();
    }
}

impl Not for Bitfield {
    type Output = Self;

    /// Flips all the bits inside bitfield.
    /// Equivalent to invert_last_n(16).
    ///
    /// # Examples
    ///
    /// ```
    /// invert(0b010110) // 101001
    /// ```
    #[inline(always)]
    fn not(self) -> Self::Output {
        Bitfield(!self.0)
    }
}

impl BitOr for Bitfield {
    type Output = Self;

    /// Merges the bits from two bitfields
    ///
    /// # Examples
    ///
    /// ```
    /// 0b0101 | 0b1010 // 0xF
    /// ```
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Bitfield(self.0 | rhs.0)
    }
}

impl BitAnd for Bitfield {
    type Output = Self;

    /// Returns the common bits between two bitfields
    ///
    /// # Examples
    ///
    /// ```
    /// 0b0111 & 0b1010 // 0x0010
    /// ```
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Bitfield(self.0 & rhs.0)
    }
}
// }}}
// {{{ BitfieldIterator
pub struct BitfieldIterator {
    index: usize,
    index_end: usize,
    bitfield: Bitfield,
}

impl Iterator for BitfieldIterator {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        while self.index <= self.index_end {
            if self.bitfield.has(self.index) {
                let result = self.index;
                self.index += 1;
                return Some(result);
            } else {
                self.index += 1;
            }
        }

        None
    }
}

impl DoubleEndedIterator for BitfieldIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        while self.index <= self.index_end {
            if self.bitfield.has(self.index_end) {
                let result = self.index_end;
                self.index_end -= 1;
                return Some(result);
            } else {
                self.index_end -= 1;
            }
        }

        None
    }
}

impl IntoIterator for Bitfield {
    type Item = usize;
    type IntoIter = BitfieldIterator;

    fn into_iter(self) -> Self::IntoIter {
        BitfieldIterator {
            index: 0,
            index_end: 15,
            bitfield: self,
        }
    }
}
// }}}
// {{{ Tests
#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    #[test]
    fn all_examples() {
        assert_eq!(Bitfield::all(), Bitfield::new(0xFFFF));
    }

    #[test]
    fn add_remove_inverses() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);

            for j in 0..16 {
                let mut clone = bitfield.clone();

                if bitfield.has(j) {
                    clone.remove(j);
                    clone.add(j);
                } else {
                    clone.add(j);
                    clone.remove(j);
                }

                assert_eq!(clone, bitfield)
            }
        }
    }

    #[test]
    fn add_implies_has() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);

            for j in 0..16 {
                let mut clone = bitfield.clone();
                if !clone.has(j) {
                    clone.add(j);
                    assert!(clone.has(j));
                }
            }
        }
    }

    #[test]
    fn remove_implies_not_has() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);

            for j in 0..16 {
                let mut clone = bitfield.clone();
                if clone.has(j) {
                    clone.remove(j);
                    assert!(!clone.has(j));
                }
            }
        }
    }

    #[test]
    fn invert_last_n_self_inverse() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);

            for i in 0..16 {
                assert_eq!(bitfield.invert_last_n(i).invert_last_n(i), bitfield);
            }
        }
    }

    #[test]
    fn count_from_end_examples() {
        assert_eq!(Bitfield::new(0b0100).count_from_end(2), 0);
        assert_eq!(Bitfield::new(0b0101).count_from_end(2), 1);
        assert_eq!(Bitfield::new(0b0110).count_from_end(2), 1);
        assert_eq!(Bitfield::new(0b0111).count_from_end(2), 2);
    }

    #[test]
    fn n_ones_examples() {
        assert_eq!(Bitfield::n_ones(16), Bitfield::all());
        assert_eq!(Bitfield::n_ones(5), Bitfield::new(0x1F));
    }

    #[test]
    fn invert_last_n_examples() {
        assert_eq!(
            Bitfield::new(0b0101).invert_last_n(3),
            Bitfield::new(0b0010)
        );
    }

    #[test]
    fn len_examples() {
        assert_eq!(0, Bitfield::default().len());
        assert_eq!(5, Bitfield::new(0b01011011).len());
        assert_eq!(16, Bitfield::all().len());
    }

    #[test]
    fn lookup_from_end_examples() {
        assert_eq!(Some(4), Bitfield::new(0b01011011).lookup_from_end(3));
        assert_eq!(None, Bitfield::new(0b0101).lookup_from_end(2));
    }

    #[test]
    fn lookup_from_end_smaller_than_count_always_just() {
        for i in 0..=u16::MAX {
            for j in 0..16 {
                let bitfield = Bitfield::new(i);

                if bitfield.has(j) {
                    for index in 0..bitfield.count_from_end(j) {
                        assert!(bitfield.lookup_from_end(index).is_some())
                    }
                }
            }
        }
    }

    #[test]
    fn lookup_from_end_count_from_end_inverses() {
        for i in 0..=u16::MAX {
            for j in 0..16 {
                let bitfield = Bitfield::new(i);

                if bitfield.has(j) {
                    assert_eq!(
                        Some(j),
                        bitfield.lookup_from_end(bitfield.count_from_end(j))
                    )
                }
            }
        }
    }

    #[test]
    fn invert_is_invert_last_16() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);
            assert_eq!(!bitfield, bitfield.invert_last_n(16))
        }
    }

    #[test]
    fn invert_last_0_is_identity() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);
            assert_eq!(bitfield.invert_last_n(0), bitfield)
        }
    }

    #[test]
    fn union_with_inverse_is_all() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);
            assert_eq!(bitfield | !bitfield, Bitfield::all())
        }
    }

    #[test]
    fn union_with_inverse_is_zero() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);
            assert_eq!(bitfield & !bitfield, Bitfield::default())
        }
    }

    #[test]
    fn encode_relative_to_empty_left_zero_elements() {
        let empty = Bitfield::default();
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);
            assert_eq!(empty.encode_relative_to(bitfield), empty)
        }
    }

    #[test]
    fn encode_relative_to_full_right_identity() {
        let full = Bitfield::all();
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);
            assert_eq!(bitfield.encode_relative_to(full), bitfield)
        }
    }

    #[test]
    fn encode_relative_to_examples() {
        assert_eq!(
            Bitfield::new(0b100010).encode_relative_to(Bitfield::new(0b101011)),
            Bitfield::new(0b1010)
        );
    }

    #[test]
    fn decode_relative_to_examples() {
        assert_eq!(
            Bitfield::new(0b1010).decode_relative_to(Bitfield::new(0b101011)),
            Some(Bitfield::new(0b100010))
        );
    }

    #[test]
    fn decode_encode_relative_to_identity() {
        for i in 0..1000 {
            for j in 0..1000 {
                let other = Bitfield::new(i);
                let j = j & i; // ensure j is a sub-bitfield of i
                let bitfield = Bitfield::new(j);
                assert_eq!(
                    bitfield.encode_relative_to(other).decode_relative_to(other),
                    Some(bitfield)
                );
            }
        }
    }

    #[test]
    fn encode_decode_relative_to_identity() {
        for i in 0..1000 {
            for j in 0..1000 {
                let other = Bitfield::new(i);
                // ensure only the last n bits of j are populated,
                // where n is the length of i.
                let j = j & ((1 << other.len()) - 1);
                let bitfield = Bitfield::new(j);
                assert_eq!(
                    bitfield
                        .decode_relative_to(other)
                        .map(|d| d.encode_relative_to(other)),
                    Some(bitfield)
                );
            }
        }
    }
}
// }}}
