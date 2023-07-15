use super::bitfield::Bitfield;

pub trait MixRanged: Sized {
    /// Embed an integer inside self given the maximum value of the integer.
    fn mix_ranged(self, value: usize, max: usize) -> Self;

    /// The inverse of mix_ranged.
    fn unmix_ranged(self, max: usize) -> Option<(Self, usize)>;

    fn mix_indexof<T: Bitfield>(self, index: T::Element, possibilities: T) -> Self {
        self.mix_ranged(possibilities.indexof(index), possibilities.len())
    }

    fn unmix_indexof<T: Bitfield>(self,  possibilities: T) -> Option<(Self, T::Element)> {
        let (remaining, index) = self.unmix_ranged(possibilities.len())?;
        Some((remaining, possibilities.index(index)?))
    }
}

impl MixRanged for usize {
    fn mix_ranged(self, value: usize, max: usize) -> Self {
        max * self + value
    }

    // TODO: return Option
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
