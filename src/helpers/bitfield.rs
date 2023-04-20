use core::fmt;

/// Bitfield containing up to 16 bits.
/// Internally used to implement stuff like creature sets,
/// edict sets, effect sets, and more.
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Bitfield(u16);

// {{{ Bitfield
impl Bitfield {
    pub fn new(x: u16) -> Self {
        Bitfield(x)
    }

    /// Returns a bitfield with a given amount of ones at the end.
    ///
    /// # Examples
    ///
    /// ```
    /// n_ones(4) // 0x000F
    /// ```
    pub fn n_ones(n: u8) -> Self {
        if n == 16 {
            Bitfield::all()
        } else {
            Bitfield::new((1 << n) - 1)
        }
    }

    /// Returns a bitfield containing only ones.
    pub fn all() -> Self {
        let mut result = Bitfield::default();
        result.fill();
        result
    }

    /// Checks if the bitfield contains an one at some index.
    ///
    /// # Examples
    ///
    /// ```
    /// has(0b0100, 1) // false
    /// has(0b0100, 2) // true
    /// ```
    pub fn has(self, index: u8) -> bool {
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
    pub fn add(&mut self, index: u8) {
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
    pub fn remove(&mut self, index: u8) {
        if !self.has(index) {
            panic!(
                "Trying to remove index {} that's not here {:b}",
                index, self.0
            )
        }
        self.0 = self.0 ^ (1 << (index as u16))
    }

    /// Sets all bits to one.
    pub fn fill(&mut self) {
        self.0 = u16::MAX;
    }

    /// Sets all bits to zero.
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Merges the bits from two bitfields
    ///
    /// # Examples
    ///
    /// ```
    /// union(0b0101, 0b1010) // 0xF
    /// ```
    pub fn union(&self, other: &Self) -> Self {
        Bitfield(self.0 | other.0)
    }

    /// Flips the last n bits.
    ///
    /// # Examples
    ///
    /// ```
    /// invert_last_n(0b0110, 2) // 0b0101
    /// ```
    pub fn invert_last_n(&self, n: u8) -> Self {
        if n == 16 {
            self.invert()
        } else {
            let mask = (1 << n) - 1;
            Bitfield(self.0 ^ mask)
        }
    }

    /// Flips all the bits inside bitfield.
    /// Equivalent to invert_last_n(16).
    ///
    /// # Examples
    ///
    /// ```
    /// invert(0b010110) // 101001
    /// ```
    pub fn invert(&self) -> Self {
        Bitfield(!self.0)
    }

    /// Returns the number of ones inside self.
    ///
    /// # Examples
    ///
    /// ```
    /// len(0b101011) // 4
    /// ```
    pub fn len(&self) -> u8 {
        let mut result = 0;

        for i in 0..16 {
            if self.has(i) {
                result += 1;
            }
        }

        result
    }

    /// Return the number of ones between a given index and the end.
    ///  
    /// # Arguments
    ///
    /// * `target` - The creature to look for the index of.
    ///
    /// # Examples
    ///
    /// ```
    /// count_from_end(0b0100, 2) // 0
    /// count_from_end(0b0101, 2) // 1
    /// count_from_end(0b0111, 2) // 2
    /// ```
    pub fn count_from_end(&self, target: u8) -> u8 {
        (0..target).filter(|x| self.has(*x)).count() as u8
    }

    /// Returns the position (starting from the end) of the nth bit.
    ///
    /// # Examples
    ///
    /// ```
    /// lookup_from_end(0b010101, 2) // Some(4)
    /// lookup_from_end(0b010101, 3) // Some(4)
    /// ```
    pub fn lookup_from_end(&self, index: u8) -> Option<usize> {
        (0..16)
            .enumerate()
            .filter(|(_, x)| self.has(*x))
            .nth(index as usize)
            .map(|(i, _)| i)
    }
}

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
// }}}
// {{{ Tests
#[cfg(test)]
mod tests {
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
        for i in 0..u16::MAX {
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
        for i in 0..u16::MAX {
            for j in 0..16 {
                let bitfield = Bitfield::new(i);

                if bitfield.has(j) {
                    assert_eq!(
                        Some(j as usize),
                        bitfield.lookup_from_end(bitfield.count_from_end(j))
                    )
                }
            }
        }
    }

    #[test]
    fn invert_is_invert_last_16() {
        for i in 0..u16::MAX {
            let bitfield = Bitfield::new(i);
            assert_eq!(bitfield.invert(), bitfield.invert_last_n(16))
        }
    }

    #[test]
    fn invert_last_0_is_identity() {
        for i in 0..u16::MAX {
            let bitfield = Bitfield::new(i);
            assert_eq!(bitfield.invert_last_n(0), bitfield)
        }
    }

    #[test]
    fn union_with_inverse_is_all() {
        for i in 0..u16::MAX {
            let bitfield = Bitfield::new(i);
            assert_eq!(bitfield.union(&bitfield.invert()), Bitfield::all())
        }
    }
}
// }}}
