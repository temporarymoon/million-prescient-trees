use core::fmt;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Bitfield(u16);

// {{{ Bitfield
impl Bitfield {
    pub fn new(x: u16) -> Self {
        Bitfield(x)
    }

    pub fn all() -> Self {
        let mut result = Bitfield::default();
        result.fill();
        result
    }

    pub fn has(self, index: u8) -> bool {
        ((self.0 >> (index as u16)) & 1) != 0
    }

    pub fn add(&mut self, index: u8) {
        if self.has(index) {
            panic!(
                "Trying to remove index {} that's not here {:b}",
                index, self.0
            )
        }

        self.0 = self.0 | (1 << (index as u16))
    }

    pub fn remove(&mut self, index: u8) {
        if !self.has(index) {
            panic!(
                "Trying to remove index {} that's not here {:b}",
                index, self.0
            )
        }

        self.0 = self.0 ^ (1 << (index as u16))
    }

    pub fn safe_remove(&mut self, index: u8) {
        if !self.has(index) {
            panic!(
                "Trying to remove index {} that's not here {:b}",
                index, self.0
            )
        }
        self.0 = self.0 ^ (1 << (index as u16))
    }

    pub fn fill(&mut self) {
        self.0 = u16::MAX;
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn union(&self, other: &Self) -> Self {
        Bitfield(self.0 | other.0)
    }

    pub fn invert(&self) -> Self {
        Bitfield(!self.0)
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
    fn invert_self_inverse() {
        for i in 0..=u16::MAX {
            let bitfield = Bitfield::new(i);

            assert_eq!(bitfield.invert().invert(), bitfield);
        }
    }

    #[test]
    fn count_from_end_examples() {
        assert_eq!(Bitfield::new(0b0100).count_from_end(2), 0);
        assert_eq!(Bitfield::new(0b0101).count_from_end(2), 1);
        assert_eq!(Bitfield::new(0b0110).count_from_end(2), 1);
        assert_eq!(Bitfield::new(0b0111).count_from_end(2), 2);
    }
}
// }}}
