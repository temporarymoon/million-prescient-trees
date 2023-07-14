use crate::helpers::{bitfield::Bitfield, choose::choose};
use std::{
    debug_assert,
    fmt::{self, Display},
    ops::{BitAnd, BitOr, Not},
};

// {{{ Creature
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord)]
pub enum Creature {
    Wall,
    Seer,
    Rogue,
    Bard,
    Diplomat,
    Ranger,
    Steward,
    Barbarian,
    Witch,
    Mercenary,
    Monarch,
}

use Creature::*;

impl Creature {
    pub const CREATURES: [Creature; 11] = [
        Wall, Seer, Rogue, Bard, Diplomat, Ranger, Steward, Barbarian, Witch, Mercenary, Monarch,
    ];

    /// Strength of given creature (top-left of the card)
    pub fn strength(self) -> u8 {
        match self {
            Wall => 0,
            Seer => 0,
            Rogue => 1,
            Bard => 2,
            Diplomat => 2,
            Ranger => 2,
            Steward => 2,
            Barbarian => 3,
            Witch => 3,
            Mercenary => 4,
            Monarch => 6,
        }
    }
}

impl Display for Creature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
// }}}
// {{{ CreatureSet
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Default)]
pub struct CreatureSet(pub Bitfield);

/// Represents an index of a bit in a creature set.
pub type CreatureIndex = usize;

impl CreatureSet {
    #[inline(always)]
    pub fn singleton(creature: Creature) -> Self {
        CreatureSet(Bitfield::singleton(creature as u8))
    }

    #[inline(always)]
    pub fn all() -> Self {
        CreatureSet(Bitfield::n_ones(11))
    }

    #[inline(always)]
    pub fn add(&mut self, creature: Creature) {
        self.0.add(creature as usize)
    }

    #[inline(always)]
    pub fn remove(&mut self, creature: Creature) {
        self.0.remove(creature as usize)
    }

    #[inline(always)]
    pub fn has(self, creature: Creature) -> bool {
        self.0.has(creature as usize)
    }

    #[inline(always)]
    pub fn len(self) -> usize {
        let result = self.0.len();
        debug_assert!(result <= 11); // Sanity checks
        result
    }

    #[inline(always)]
    pub fn indexof(self, target: Creature) -> CreatureIndex {
        self.0.count_from_end(target as usize)
    }

    #[inline(always)]
    pub fn index(self, index: CreatureIndex) -> Option<Creature> {
        self.0
            .lookup_from_end(index)
            .map(|x| Creature::CREATURES[x])
    }

    #[inline(always)]
    pub fn encode_relative_to(self, other: Self) -> Bitfield {
        self.0.encode_relative_to(other.0)
    }

    #[inline(always)]
    pub fn decode_relative_to(bitfield: Bitfield, other: CreatureSet) -> Option<Self> {
        Some(Self(bitfield.decode_relative_to(other.0)?))
    }

    /// Computes the number of hands of a given size with cards from the current set.
    #[inline(always)]
    pub fn hands_of_size(self, size: usize) -> usize {
        choose(self.len() as usize, size)
    }
}

// {{{ IntoIter
pub struct CreatureSetIterator {
    index: usize,
    bitfield: CreatureSet,
}

impl Iterator for CreatureSetIterator {
    type Item = Creature;
    fn next(&mut self) -> Option<Self::Item> {
        while self.index <= 11 {
            if self.bitfield.0.has(self.index) {
                let result = self.index;
                self.index += 1;
                return Some(Creature::CREATURES[result as usize]);
            } else {
                self.index += 1;
            }
        }

        None
    }
}

impl IntoIterator for CreatureSet {
    type Item = Creature;
    type IntoIter = CreatureSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        CreatureSetIterator {
            index: 0,
            bitfield: self,
        }
    }
}
// }}}
// {{{ Bit operations
impl BitOr for CreatureSet {
    type Output = Self;

    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for CreatureSet {
    type Output = Self;

    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Not for CreatureSet {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self {
        CreatureSet(self.0.invert_last_n(11))
    }
}
// }}}
// }}}
