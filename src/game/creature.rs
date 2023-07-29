use crate::{
    helpers::{
        bitfield::{Bitfield, Bitfield16},
        choose::choose,
    },
    make_bitfield,
};
use std::{
    convert::TryFrom,
    debug_assert,
    fmt::{self, Display},
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

impl TryFrom<usize> for Creature {
    type Error = ();
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Creature::CREATURES.get(value).copied().ok_or(())
    }
}
// }}}
// {{{ CreatureSet
make_bitfield!(CreatureSet, Creature, u16, 11, Bitfield16, true);

impl CreatureSet {
    /// Computes the number of hands of a given size with cards from the current set.
    #[inline(always)]
    pub fn hands_of_size(self, size: usize) -> usize {
        choose(self.len() as usize, size)
    }
}
// }}}
