use core::fmt;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Bitfield(u16);

impl Bitfield {
    pub fn new() -> Self {
        Bitfield(0)
    }

    pub fn all() -> Self {
        let mut result = Bitfield::new();
        result.fill();
        result
    }

    pub fn has(self, index: u8) -> bool {
        ((self.0 >> (index as u16)) & 1) != 0
    }

    pub fn add(&mut self, index: u8) {
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
