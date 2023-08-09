use crate::helpers::bitfield::{Bitfield, Bitfield16};
use crate::make_bitfield;
use std::convert::TryFrom;
use std::debug_assert;
use std::fmt::{self, Display};

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

    pub const DESCRIPTIONS: [&str; 11] = ["The battle this card is involved in ends in a tie.", 
        "Next battle, play two creatures instead of one. After the opponent reveals their creature, choose one creature to reveal, and return the other to your hand.",
        "Negates the seer character. Wins against the monarch and the wall.",
        "Next battle, gain +1 strength. Furthermore, winning the next battle awards you +1 victory points.",
        "Wins the battle if both players played the same edict.",
        "Gains +2 strength if you receive a battlefield bonus and the opponent does not.",
        "Edicts are twice as effective. At the end of the turn, return all your edicts back to the hand.",
        "Gains +2 strength if you lost last battle",
        "Negates the opponent's creature. Cannot gain strength from edicts.",
        "Next turn, lose 1 strength.",
        "If you do not win this battle, your opponent gains +2 additional victory points.",
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

make_bitfield!(CreatureSet, Creature, u16, 11, Bitfield16, true);
