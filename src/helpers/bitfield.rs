use std::{fmt::{Binary, Debug}, convert::{TryFrom, TryInto}, ops::BitAnd};

use self::const_size_codec::ConstSizeCodec;

use super::choose::choose;

// {{{ Trait definition
/// A (non exhuasive) list of laws:
/// - `Self::Representation::try_from` shouldn't be able to fail
///   for values in the range `0..Self::MAX.into()`
pub trait Bitfield: Sized + Copy + Binary + Into<Self::Representation> 
  + IntoIterator<Item = Self::Element> + BitAnd<Output = Self> + Eq 
{
    type Element: TryFrom<usize> + Copy;
    type IndexBitfield: Bitfield<Element = usize>;
    type Representation: Into<usize> + TryFrom<usize>;

    /// The maximum valid value this bitfield can take.
    const MAX: Self::Representation;

    /// The number of bits in the bitfield
    const BITS: usize;

    /// Construct a new bitfield.
    fn new(x: Self::Representation) -> Self;

    /// Construct a bitfield containing a single bit.
    fn singleton(x: Self::Element) -> Self;

    /// Construct a bitfield containing at most one bit.
    #[inline(always)]
    fn opt_singleton(x: Option<Self::Element>) -> Self {
        match x {
            Some(x) => Self::singleton(x),
            None => Self::empty()
        }
    }

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
    fn insert(&mut self, index: Self::Element);

    /// Similar to `add`, but accepts integers instead of `$element`.
    fn insert_raw(&mut self, index: usize);

    /// Removes a bit from a bitfield.
    /// Errors out if the bit is already there.
    /// # Examples
    ///
    /// ```
    /// add(0b0110, 1) // 0b0100
    /// ```
    fn remove(&mut self, index: Self::Element);

    /// Moves an element from `self` to some other bitfield.
    fn move_one(&mut self, to: &mut Self, bit: Self::Element) {
        self.remove(bit);
        to.insert(bit);
    }

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
    fn indexof(self, target: Self::Element) -> Option<usize>;

    /// Similar to `indexof`, but accepts integers instead of `$element`.
    fn indexof_raw(self, target: usize) -> Option<usize>;

    /// Returns the position (starting from the end) of the nth bit.
    ///
    /// # Examples
    ///
    /// ```
    /// index(0b010101, 2) // Some(4)
    /// index(0b010101, 3) // Some(4)
    /// ```
    fn index_raw(self, index: usize) -> Option<usize> {
        (0..Self::BITS)
            .filter(|x| self.has_raw(*x))
            .nth(index)
    }

    /// Returns the position (starting from the end) of the nth bit.
    ///
    /// # Examples
    ///
    /// ```
    /// index(0b010101, 2) // Some(4)
    /// index(0b010101, 3) // Some(4)
    /// ```
    fn index(self, index: usize) -> Option<Self::Element> {
        self.index_raw(index).and_then(|i| i.try_into().ok())
    }
    
    /// Encode a bitfield as a subset of another bitfield.
    /// Bits are shifted to the left such that all zero bits in the super-bitfield
    /// are not present in the sub-bitfield.
    ///
    /// Properties:
    /// - the empty bitfield acts as a left zero elements
    /// - the full bitfield acts as a right identity
    fn encode_relative_to(self, other: Self) -> Self::IndexBitfield {
        let mut result = Self::IndexBitfield::empty();

        assert!(self.is_subset_of(other));

        for i in 0..Self::BITS {
            if self.has_raw(i) {
                result.insert(other.indexof_raw(i).unwrap());
            }
        }

        result
    }

    /// Inverse of `encode_relative_to`.
    fn decode_relative_to(encoded: Self::IndexBitfield, other: Self) -> Option<Self> {
        let mut result = Self::empty();

        for i in 0..Self::BITS {
            if encoded.has_raw(i) {
                result.insert_raw(other.index_raw(i as usize)?);
            }
        }

        debug_assert!(result.is_subset_of(other));

        Some(result)
    }

    /// Returns an iterator over the subsets of a given size.
    #[inline(always)]
    fn subsets_of_size(self, ones: usize) -> BitfieldFixedSizeSubsetIterator<Self> {
        BitfieldFixedSizeSubsetIterator::new(self, ones) 
    }

    /// Returns an iterator over all subsets of a given bitfield.
    #[inline(always)]
    fn subsets(self) -> BitfieldSubsetIterator<Self> {
        BitfieldSubsetIterator::new(self) 
    }

    /// Returns an iterator over every valid bitfield
    #[inline(always)]
    fn members() -> BitfieldSubsetIterator<Self> {
        Self::all().subsets()
    }

    /// Returns true if all bits in `self` occur in `other`.
    #[inline(always)]
    fn is_subset_of(self, other: Self) -> bool {
        self & other == self
    }

    /// Computes the number of subsets of a given size with elements from the current set.
    #[inline(always)]
    fn count_subsets_of_size(self, ones: usize) -> usize {
        choose(self.len(), ones)
    }

    /// Returns true if two bitfields are disjoint
    #[inline(always)]
    fn is_disjoint_from(self, other: Self) -> bool {
        self & other == Self::empty()
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
            type IndexBitfield = $index_bitfield;

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

            fn insert(&mut self, index: $element) {
                if self.has(index) {
                    panic!(
                        "Trying to add index {} that is already present in {:b}",
                        index, self.0
                    )
                }

                self.0 |= 1 << (index as $repr);
            }

            fn insert_raw(&mut self, index: usize) {
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

            #[inline(always)]
            fn len(self) -> usize {
                self.0.count_ones() as usize
            }

            #[inline(always)]
            fn indexof(self, target: $element) -> Option<usize> {
                self.indexof_raw(target as usize)
            }

            fn indexof_raw(self, target: usize) -> Option<usize> {
                if self.has_raw(target) {
                    Some((self.0 & ((1 << (target as $repr)) - 1)).count_ones() as usize)
                } else {
                    None
                }
            }
        }
        // }}}
        // {{{ Trait implementations
        impl std::fmt::Binary for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:b}", self.0)
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_list().entries(self.into_iter()).finish()
            }
        }

        impl std::ops::Not for $name {
            type Output = Self;

            #[inline(always)]
            fn not(self) -> Self::Output {
                Self(!self.0 & Self::MAX)
            }
        }

        impl std::ops::BitOr for $name {
            type Output = Self;

            #[inline(always)]
            fn bitor(self, rhs: Self) -> Self::Output {
                Self(self.0 | rhs.0)
            }
        }

        impl std::ops::BitAnd for $name {
            type Output = Self;

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

        impl std::ops::Add<$element> for $name {
            type Output = Self;

            #[inline(always)]
            fn add(mut self, rhs: $element) -> Self::Output {
                self.insert(rhs);
                self
            }
        }

        impl std::ops::Sub<$name> for $name {
            type Output = Self;

            #[inline(always)]
            fn sub(self, rhs: Self) -> Self::Output {
                Self(self.0 & !rhs.0)
            }
        }

        impl std::ops::Sub<$element> for $name {
            type Output = Self;

            #[inline(always)]
            fn sub(mut self, rhs: $element) -> Self::Output {
                self.remove(rhs);
                self
            }
        }

        impl std::ops::SubAssign<$name> for $name {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: Self) {
                self.0 &= !rhs.0;
            }
        }

        impl IntoIterator for $name {
            type Item = $element;
            type IntoIter = crate::helpers::bitfield::BitfieldIterator<$name>;

            fn into_iter(self) -> Self::IntoIter {
                crate::helpers::bitfield::BitfieldIterator::new(self)
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
// {{{ Iterator
#[derive(Debug, Clone, Copy)]
pub struct BitfieldIterator<B> {
    index: usize,
    index_end: usize,
    bitfield: B,
}

impl<B: Bitfield> BitfieldIterator<B> {
    #[inline(always)]
    pub fn new(bitfield: B) -> Self {
        Self { index: 0, index_end: B::BITS - 1, bitfield }
    }
}

impl<B: Bitfield> Iterator for BitfieldIterator<B> {
    type Item = B::Element;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index <= self.index_end {
            if self.bitfield.has_raw(self.index) {
                let result = B::Element::try_from(self.index).ok();
                self.index += 1;
                return result;
            } else {
                self.index += 1;
            }
        }

        None
    }
}

impl<B: Bitfield> DoubleEndedIterator for BitfieldIterator<B> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while self.index <= self.index_end {
            if self.bitfield.has_raw(self.index_end) {
                let result = B::Element::try_from(self.index_end).ok();
                self.index_end -= 1;
                return result;
            } else {
                self.index_end -= 1;
            }
        }

        None
    }
}
// }}}
// {{{ Fixed size subset iterator
#[derive(Debug, Clone, Copy)]
pub struct BitfieldFixedSizeSubsetIterator<B> {
    index: usize,
    index_end: usize,
    ones: usize,
    possibilities: B,
}

impl<B: Bitfield> BitfieldFixedSizeSubsetIterator<B> {
    #[inline(always)]
    pub fn new(possibilities: B, ones: usize) -> Self {
        Self {
            index: 0,
            index_end: choose(possibilities.len(), ones) - 1,
            ones,
            possibilities,
        }
    }
}

impl<B: Bitfield> Iterator for BitfieldFixedSizeSubsetIterator<B> {
    type Item = B;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index <= self.index_end {
            let result = ConstSizeCodec::decode_ones(self.index, self.ones)?;
            self.index += 1;
            Bitfield::decode_relative_to(result, self.possibilities)
        } else {
            None
        }
    }
}

impl<B: Bitfield> DoubleEndedIterator for BitfieldFixedSizeSubsetIterator<B> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.index <= self.index_end {
            let result = ConstSizeCodec::decode_ones(self.index_end, self.ones)?;
            self.index_end -= 1;
            Bitfield::decode_relative_to(result, self.possibilities)
        } else {
            None
        }
    }
}
// }}}
// {{{ Subset iterator
#[derive(Debug, Clone, Copy)]
pub struct BitfieldSubsetIterator<B> {
    index: usize,
    index_end: usize,
    possibilities: B,
}

impl<B: Bitfield> BitfieldSubsetIterator<B> {
    #[inline(always)]
    pub fn new(possibilities: B) -> Self {
        Self {
            index: 0,
            // NOTE: this could fail a bit if the representation is `usize`,
            // but we never use bitfields that large in practice.
            index_end: 2usize.pow(possibilities.len() as u32) - 1,
            possibilities,
        }
    }
}

impl<B: Bitfield> Iterator for BitfieldSubsetIterator<B> {
    type Item = B;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index <= self.index_end {
            let result = B::IndexBitfield::new(self.index.try_into().ok()?);
            self.index += 1;
            Bitfield::decode_relative_to(result, self.possibilities)
        } else {
            None
        }
    }
}

impl<B: Bitfield> DoubleEndedIterator for BitfieldSubsetIterator<B> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.index <= self.index_end {
            let result = B::IndexBitfield::new(self.index_end.try_into().ok()?);
            self.index_end -= 1;
            Bitfield::decode_relative_to(result, self.possibilities)
        } else {
            None
        }
    }
}
// }}}
// {{{ Main definition
make_bitfield!(
    Bitfield16,
    usize,
    u16,
    16,
    Bitfield16,
    true
);

impl Bitfield16 {
    /// A nicer form of `decode_relative_to`.
    // NOTE: only used in testing now. Is it worth keeping?
    #[inline(always)]
    pub fn decode_self_relative_to(self: Self, other: Self) -> Option<Self> {
        Self::decode_relative_to(self, other)
    }
}
// }}}
// {{{ Ones encoding
pub mod const_size_codec {
    use std::convert::TryInto;
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

    /// Returns the number of possible bitfields containing a given number of ones.
    #[inline(always)]
    pub fn count_with_n_ones(ones: usize) -> usize {
        assert!(ones <= 16);
        LOOKUP_TABLES.2[ones]
    }

    /// Represents a bitfield, after all information about the number of ones
    /// has been removed.
    pub type Encoded = usize;

    pub trait ConstSizeCodec: Bitfield  {
        /// Efficiently assume the number of ones in the bit
        /// representation of a number is known, removing such
        /// useless information.
        ///
        /// The result fits inside an u16,
        /// but we pass around an `usize` for convenience.
        #[inline(always)]
        fn encode_ones(self) -> Encoded {
            assert!(Self::BITS <= 16);

            LOOKUP_TABLES.0[self.into().into()] as usize
        }

        /// Inverse of `encode_ones`.
        fn decode_ones(encoded: Encoded, ones: usize) -> Option<Self> {
            assert!(Self::BITS <= 16);

            if encoded >= count_with_n_ones(ones) {
                None
            } else {
                let decoded = *LOOKUP_TABLES.1.get(MAGIC_INDICES[ones] + encoded)? as usize; 

                Some(Self::new(
                    decoded.try_into().ok()?
                ))
            }
        }

        /// Combination of `encode_ones` chained onto `encode_relative_to`
        #[inline(always)]
        fn encode_ones_relative_to(self, other: Self) -> Encoded {
            self.encode_relative_to(other).encode_ones()
        }

        /// Inverse of `encode_ones_relative_to`
        #[inline(always)]
        fn decode_ones_relative_to(encoded: Encoded, ones: usize, other: Self) -> Option<Self> {
            let encoded = Self::IndexBitfield::decode_ones(encoded, ones)?;
            Self::decode_relative_to(encoded, other)
        }
    } 

    impl<T: Bitfield> ConstSizeCodec for T {}
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
                    ConstSizeCodec::decode_ones(bitfield.encode_ones(), bitfield.len())
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
                    clone.insert(j);
                } else {
                    clone.insert(j);
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
                    clone.insert(j);
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
        assert_eq!(Bitfield16::new(0b0100).indexof(2), Some(0));
        assert_eq!(Bitfield16::new(0b0101).indexof(2), Some(1));
        assert_eq!(Bitfield16::new(0b0110).indexof(2), Some(1));
        assert_eq!(Bitfield16::new(0b0111).indexof(2), Some(2));
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

                if let Some(index) = bitfield.indexof(j) {
                    for index in 0..index {
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

                if let Some(index) = bitfield.indexof(j) {
                    assert_eq!(Some(j), bitfield.index(index))
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

    #[test]
    fn subset_iterator_produces_subsets() {
        for i in 0..Bitfield16::MAX {
            let b = Bitfield16::new(i);
            for s in b.subsets() {
                assert!(s.is_subset_of(b))
            }
        }
    }

    #[test]
    fn subsets_correct_count() {
        for i in 0..Bitfield16::MAX {
            let b = Bitfield16::new(i);
            assert_eq!(b.subsets().count(), 2usize.pow(b.len() as u32));
        }
    }

    #[test]
    fn subsets_of_size_iterator_produces_subsets() {
        for i in 0..Bitfield16::MAX {
            let b = Bitfield16::new(i);
            for ones in 0..=b.len()  {
                for s in b.subsets_of_size(ones) {
                    assert!(s.is_subset_of(b), "{:b} is not a subset of {:b}", s, b)
                }
            }
        }
    }

    #[test]
    fn subsets_of_size_correct_count() {
        for i in 0..Bitfield16::MAX {
            let b = Bitfield16::new(i);
            for ones in 0..=b.len()  {
                assert_eq!(b.subsets_of_size(ones).count(), b.count_subsets_of_size(ones));
            }
        }
    }
}
// }}}
