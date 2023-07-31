use super::{
    bitfield::{const_size_codec::ConstSizeCodec, Bitfield},
    choose::choose,
};

pub trait MixRanged: Sized {
    /// Embed an integer inside self given the maximum value of the integer.
    fn mix_ranged(self, value: usize, max: usize) -> Self;

    /// The inverse of mix_ranged.
    fn unmix_ranged(self, max: usize) -> Option<(Self, usize)>;

    /// Mix in data about the index of some bit in a bitfield.
    fn mix_indexof<T: Bitfield>(self, index: T::Element, possibilities: T) -> Option<Self> {
        Some(self.mix_ranged(possibilities.indexof(index)?, possibilities.len()))
    }

    /// Inverse of `mix_indeox`
    fn unmix_indexof<T: Bitfield>(self, possibilities: T) -> Option<(Self, T::Element)> {
        let (remaining, index) = self.unmix_ranged(possibilities.len())?;
        Some((remaining, possibilities.index(index)?))
    }

    /// Generalized version of `mix_indexof` which works with arbitrary sized subsets.
    fn mix_subset<T: Bitfield>(self, subset: T, of: T) -> Self {
        assert!(subset.is_subset_of(of));
        let values = choose(of.len(), subset.len());
        self.mix_ranged(subset.encode_ones_relative_to(of), values)
    }

    /// Inverse of `mix_subset`
    fn unmix_subset<T: Bitfield>(self, length: usize, of: T) -> Option<(Self, T)> {
        let values = choose(of.len(), length);
        let (remaining, encoded) = self.unmix_ranged(values)?;
        let subset = T::decode_ones_relative_to(encoded, length, of)?;

        Some((remaining, subset))
    }
}

impl MixRanged for usize {
    #[inline(always)]
    fn mix_ranged(self, value: usize, max: usize) -> Self {
        max * self + value
    }

    #[inline(always)]
    fn unmix_ranged(self, max: usize) -> Option<(Self, usize)> {
        Some((self / max, self % max))
    }
}

#[cfg(test)]
mod tests {
    use super::MixRanged;

    #[test]
    fn usize_mix_unmix_inverses() {
        for i in 0..500 {
            for max in 1..100 {
                for j in 0..max {
                    assert_eq!(Some((i, j)), i.mix_ranged(j, max).unmix_ranged(max))
                }
            }
        }
    }

    #[test]
    fn usize_mix_examples() {
        assert_eq!(53, 10.mix_ranged(3, 5));
        assert_eq!(90, 9.mix_ranged(0, 10));
    }

    #[test]
    fn usize_unmix_examples() {
        assert_eq!(Some((4, 9)), 53.unmix_ranged(11));
        assert_eq!(Some((8, 22)), 222.unmix_ranged(25));
    }
}
