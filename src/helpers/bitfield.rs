// {{{ Trait definition
pub trait Bitfield: Sized + Copy + Into<Self::Representation> {
    type Element: From<usize>;
    type Representation;

    /// The maximum valid value this bitfield can take.
    const MAX: Self::Representation;

    /// The number of bits in the bitfield
    const BITS: usize;

    /// Construct a new bitfield.
    fn new(x: Self::Representation) -> Self;

    /// Construct a bitfield containing a single bit.
    fn singleton(x: Self::Element) -> Self;

    /// Returns a bitfield containing only zeros.
    fn empty() -> Self;

    /// Returns a bitfield containing only ones.
    #[inline(always)]
    fn all() -> Self {
        Self::new(Self::MAX)
    }

    /// Checks if the bitfield contains an one at some index.
    ///
    /// # Examples
    ///
    /// ```
    /// has(0b0100, 1) // false
    /// has(0b0100, 2) // true
    /// ```
    fn has(self, index: Self::Element) -> bool;

    /// Similar to `has`, but accepts integers instead of `$element`.
    fn has_raw(self, index: usize) -> bool;

    /// Adds a bit to a bitfield.
    /// Errors out if the bit is already there.
    ///
    /// # Examples
    ///
    /// ```
    /// add(0b0100, 1) // 0b0110
    /// ```
    fn add(&mut self, index: Self::Element);

    /// Similar to `add`, but accepts integers instead of `$element`.
    fn add_raw(&mut self, index: usize);

    /// Removes a bit from a bitfield.
    /// Errors out if the bit is already there.
    /// # Examples
    ///
    /// ```
    /// add(0b0110, 1) // 0b0100
    /// ```
    fn remove(&mut self, index: Self::Element);

    /// Sets all bits to one.
    #[inline(always)]
    fn fill(&mut self) {
        *self = Self::all();
    }

    /// Sets all bits to zero.
    #[inline(always)]
    fn clear(&mut self) {
        *self = Self::empty();
    }

    /// Returns the number of ones inside self.
    ///
    /// # Examples
    ///
    /// ```
    /// len(0b101011) // 4
    /// ```
    fn len(self) -> usize;

    /// Return the number of ones between a given index and the end.
    ///
    /// # Arguments
    ///
    /// * `target` - The index to count ones after.
    ///
    /// # Examples
    ///
    /// ```
    /// indexof(0b0100, 2) // 0
    /// indexof(0b0101, 2) // 1
    /// indexof(0b0111, 2) // 2
    /// ```
    fn indexof(self, target: Self::Element) -> usize;

    /// Similar to `indexof`, but accepts integers instead of `$element`.
    fn indexof_raw(self, target: usize) -> usize;

    /// Returns the position (starting from the end) of the nth bit.
    ///
    /// # Examples
    ///
    /// ```
    /// index(0b010101, 2) // Some(4)
    /// index(0b010101, 3) // Some(4)
    /// ```
    fn index(self, index: usize) -> Option<Self::Element> {
        (0..Self::BITS)
            .filter(|x| self.has_raw(*x))
            .nth(index)
            .map(|i| Self::Element::from(i))
    }
}
// }}}
// {{{ Main macro definition
#[macro_export]
macro_rules! make_bitfield {
    (
        $name: ident, 
        $element: ty, 
        $repr: ty, 
        $bits: expr, 
        $iterator: ident, 
        $index_bitfield: ty, 
        $default_is_empty: literal
    ) => {
        #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
        pub struct $name(pub $repr);

        impl Into<$repr> for $name {
            fn into(self) -> $repr {
                self.0
            }
        }

        // {{{ Main impl block
        impl crate::helpers::bitfield::Bitfield for $name {
            type Element = $element;
            type Representation = $repr;

            const MAX: $repr = if $bits == <$repr>::BITS {
                <$repr>::MAX
            } else {
                (1 << ($bits)) - 1
            };

            const BITS: usize = $bits;

            #[inline(always)]
            fn new(x: $repr) -> Self {
                debug_assert!(x <= Self::MAX);
                Self(x)
            }

            #[inline(always)]
            fn singleton(x: $element) -> Self {
                Self::new(1 << (x as $repr))
            }

            #[inline(always)]
            fn empty() -> Self {
                Self::new(0)
            }

            #[inline(always)]
            fn has(self, index: $element) -> bool {
                self.has_raw(index as usize)
            }

            #[inline(always)]
            fn has_raw(self, index: usize) -> bool {
                ((self.0 >> (index as $repr)) & 1) != 0
            }

            fn add(&mut self, index: $element) {
                if self.has(index) {
                    panic!(
                        "Trying to add index {} that is already present in {:b}",
                        index, self.0
                    )
                }

                self.0 |= 1 << (index as $repr);
            }

            fn add_raw(&mut self, index: usize) {
                if self.has_raw(index) {
                    panic!(
                        "Trying to add index {} that is already present in {:b}",
                        index, self.0
                    )
                }

                self.0 |= 1 << (index as $repr);
            }

            fn remove(&mut self, index: $element) {
                if !self.has(index) {
                    panic!(
                        "Trying to remove index {} that is not present {:b}",
                        index, self.0
                    )
                }

                self.0 ^= 1 << (index as $repr)
            }

            fn len(self) -> usize {
                self.0.count_ones() as usize
            }

            fn indexof(self, target: $element) -> usize {
                self.indexof_raw(target as usize)
            }

            fn indexof_raw(self, target: usize) -> usize {
                (self.0 & ((1 << (target as $repr)) - 1)).count_ones() as usize
            }
        }
        // }}}
        // {{{ Relative encoding
        impl $name {
            /// Encode a bitfield as a subset of another bitfield.
            /// Bits are shifted to the left such that all zero bits in the super-bitfield
            /// are not present in the sub-bitfield.
            ///
            /// Properties:
            /// - the empty bitfield acts as a left zero elements
            /// - the full bitfield acts as a right identity
            pub fn encode_relative_to(self, other: Self) -> $index_bitfield {
                let mut result = <$index_bitfield>::empty();

                for i in 0..$bits {
                    if self.has_raw(i) {
                        assert!(
                            other.has_raw(i),
                            "{:?} contains bits not contained in {:?}",
                            self,
                            other
                        );

                        result.add(other.indexof_raw(i))
                    }
                }

                result
            }

            /// Inverse of `encode_relative_to`.
            pub fn decode_relative_to(encoded: $index_bitfield, other: Self) -> Option<Self> {
                let mut result = Self::empty();

                for i in 0..$bits {
                    if encoded.has_raw(i) {
                        result.add_raw(other.index(i as usize)? as usize);
                    }
                }

                Some(result)
            }
        }
        // }}}
        // {{{ Trait implementations
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:b}", self.0)
            }
        }

        impl std::ops::Not for $name {
            type Output = Self;

            /// Flips all the bits inside bitfield.
            ///
            /// # Examples
            ///
            /// ```
            /// invert(0b010110) // 101001
            /// ```
            #[inline(always)]
            fn not(self) -> Self::Output {
                Self(!self.0 & Self::MAX)
            }
        }

        impl std::ops::BitOr for $name {
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
                Self(self.0 | rhs.0)
            }
        }

        impl std::ops::BitAnd for $name {
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
                Self(self.0 & rhs.0)
            }
        }

        impl std::ops::BitOrAssign for $name {
            #[inline(always)]
            fn bitor_assign(&mut self, rhs: Self) {
                self.0 |= rhs.0; 
            }
        }

        impl std::ops::BitAndAssign for $name {
            #[inline(always)]
            fn bitand_assign(&mut self, rhs: Self) {
                self.0 &= rhs.0; 
            }
        }

        impl std::ops::Sub<$name> for $name {
            type Output = Self;

            /// Returns the difference between two bitfields.
            ///
            /// # Examples
            ///
            /// ```
            /// 0b0111 - 0b1010 // 0x0101
            /// ```
            #[inline(always)]
            fn sub(self, rhs: Self) -> Self::Output {
                Self(self.0 & !rhs.0)
            }
        }

        impl std::ops::SubAssign<$name> for $name {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: Self) {
                self.0 &= !rhs.0;
            }
        }
        // }}}
        // {{{ Iterator
        pub struct $iterator {
            index: usize,
            index_end: usize,
            bitfield: $name,
        }

        impl Iterator for $iterator {
            type Item = $element;

            fn next(&mut self) -> Option<Self::Item> {
                while self.index <= self.index_end {
                    if self.bitfield.has_raw(self.index) {
                        let result = <$element>::from(self.index);
                        self.index += 1;
                        return Some(result);
                    } else {
                        self.index += 1;
                    }
                }

                None
            }
        }

        impl DoubleEndedIterator for $iterator {
            fn next_back(&mut self) -> Option<Self::Item> {
                while self.index <= self.index_end {
                    if self.bitfield.has_raw(self.index_end) {
                        let result = <$element>::from(self.index_end);
                        self.index_end -= 1;
                        return Some(result);
                    } else {
                        self.index_end -= 1;
                    }
                }

                None
            }
        }

        impl IntoIterator for $name {
            type Item = $element;
            type IntoIter = $iterator;

            fn into_iter(self) -> Self::IntoIter {
                $iterator {
                    index: 0,
                    index_end: 15,
                    bitfield: self,
                }
            }
        }
        // }}}
        // {{{ Default implementation
        impl Default for $name {
            fn default() -> Self {
                if $default_is_empty {
                    Self::empty()
                } else {
                    Self::all()
                }
            }
        }
        // }}}
    };
}
// }}}
// {{{ Main definition
make_bitfield!(
    Bitfield16,
    usize,
    u16,
    16,
    Bitfield16Iterator,
    Bitfield16,
    true
);

impl Bitfield16 {
    /// A nicer form of `decode_relative_to`.
    pub fn decode_self_relative_to(self: Self, other: Self) -> Option<Self> {
        Self::decode_relative_to(self, other)
    }
}
// }}}
// {{{ Ones encoding
pub mod one_encoding {
    use once_cell::sync::Lazy;

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
    static MAGIC_INDICES: Lazy<[usize; BIT_CASES]> = Lazy::new(|| {
        let mut results = [0; BIT_CASES];

        for i in 1..BIT_CASES {
            results[i] = results[i - 1] + choose(16, i - 1);
        };

        results
    });

    /// The lookup tables required for encoding!
    /// - the first table maps raw values to encoded values.
    /// - the second array contains 17 distinct decoding tables
    ///   concatenated together. The nth table maps encoded values
    ///   to respective raw values with n-ones inside.
    /// - the third table contains the number of entries in each table
    ///   contained in the second array (used for testing / asserts).
    static LOOKUP_TABLES: Lazy<(
        [u16; DECODED_COUNT],
        [u16; DECODED_COUNT],
        [usize; BIT_CASES],
    )> = Lazy::new(|| {
        let mut encode = [0 as u16; DECODED_COUNT];
        let mut decode = [0 as u16; DECODED_COUNT];
        let mut lengths = [0 as usize; BIT_CASES];

        for decoded in 0..DECODED_COUNT {
            let count = Bitfield16::new(decoded as u16).len() as usize;
            let encoded = lengths[count];
            decode[MAGIC_INDICES[count] + encoded] = decoded as u16;
            encode[decoded] = encoded as u16;
            lengths[count] += 1;
        };

        (encode, decode, lengths)
    });

    impl Bitfield16 {
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
                return Some(Bitfield16::new(
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
                let bitfield = Bitfield16::new(i);

                assert_eq!(
                    Some(bitfield),
                    Bitfield16::decode_ones(bitfield.encode_ones(), bitfield.len())
                );
            }
        }

        #[test]
        fn decode_encode_identity() {
            for ones in 0..=16 {
                for i in 0..=(u16::MAX as usize) {
                    let decode_encode_i =
                        Bitfield16::decode_ones(i, ones).map(|bitfield| bitfield.encode_ones());

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
// {{{ Tests
#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    #[test]
    fn all_examples() {
        assert_eq!(Bitfield16::all(), Bitfield16::new(0xFFFF));
    }

    #[test]
    fn add_remove_inverses() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield16::new(i);

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
            let bitfield = Bitfield16::new(i);

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
            let bitfield = Bitfield16::new(i);

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
    fn indexof_examples() {
        assert_eq!(Bitfield16::new(0b0100).indexof(2), 0);
        assert_eq!(Bitfield16::new(0b0101).indexof(2), 1);
        assert_eq!(Bitfield16::new(0b0110).indexof(2), 1);
        assert_eq!(Bitfield16::new(0b0111).indexof(2), 2);
    }

    #[test]
    fn len_examples() {
        assert_eq!(0, Bitfield16::default().len());
        assert_eq!(5, Bitfield16::new(0b01011011).len());
        assert_eq!(16, Bitfield16::all().len());
    }

    #[test]
    fn index_examples() {
        assert_eq!(Some(4), Bitfield16::new(0b01011011).index(3));
        assert_eq!(None, Bitfield16::new(0b0101).index(2));
    }

    #[test]
    fn index_smaller_than_count_always_just() {
        for i in 0..=u16::MAX {
            for j in 0..16 {
                let bitfield = Bitfield16::new(i);

                if bitfield.has(j) {
                    for index in 0..bitfield.indexof(j) {
                        assert!(bitfield.index(index).is_some())
                    }
                }
            }
        }
    }

    #[test]
    fn index_indexof_inverses() {
        for i in 0..=u16::MAX {
            for j in 0..16 {
                let bitfield = Bitfield16::new(i);

                if bitfield.has(j) {
                    assert_eq!(Some(j), bitfield.index(bitfield.indexof(j)))
                }
            }
        }
    }

    #[test]
    fn union_with_inverse_is_all() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield16::new(i);
            assert_eq!(bitfield | !bitfield, Bitfield16::all())
        }
    }

    #[test]
    fn union_with_inverse_is_zero() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield16::new(i);
            assert_eq!(bitfield & !bitfield, Bitfield16::default())
        }
    }

    #[test]
    fn encode_relative_to_empty_left_zero_elements() {
        let empty = Bitfield16::default();
        for i in 0..=u16::MAX {
            let bitfield = Bitfield16::new(i);
            assert_eq!(empty.encode_relative_to(bitfield), empty)
        }
    }

    #[test]
    fn encode_relative_to_full_right_identity() {
        let full = Bitfield16::all();
        for i in 0..=u16::MAX {
            let bitfield = Bitfield16::new(i);
            assert_eq!(bitfield.encode_relative_to(full), bitfield)
        }
    }

    #[test]
    fn encode_relative_to_examples() {
        assert_eq!(
            Bitfield16::new(0b100010).encode_relative_to(Bitfield16::new(0b101011)),
            Bitfield16::new(0b1010)
        );
    }

    #[test]
    fn decode_relative_to_examples() {
        assert_eq!(
            Bitfield16::new(0b1010).decode_self_relative_to(Bitfield16::new(0b101011)),
            Some(Bitfield16::new(0b100010))
        );
    }

    #[test]
    fn decode_encode_relative_to_identity() {
        for i in 0..1000 {
            for j in 0..1000 {
                let other = Bitfield16::new(i);
                let j = j & i; // ensure j is a sub-bitfield of i
                let bitfield = Bitfield16::new(j);
                assert_eq!(
                    bitfield.encode_relative_to(other).decode_self_relative_to(other),
                    Some(bitfield)
                );
            }
        }
    }

    #[test]
    fn encode_decode_relative_to_identity() {
        for i in 0..1000 {
            for j in 0..1000 {
                let other = Bitfield16::new(i);
                // ensure only the last n bits of j are populated,
                // where n is the length of i.
                let j = j & ((1 << other.len()) - 1);
                let bitfield = Bitfield16::new(j);
                assert_eq!(
                    bitfield
                        .decode_self_relative_to(other)
                        .map(|d| d.encode_relative_to(other)),
                    Some(bitfield)
                );
            }
        }
    }
}
// }}}
