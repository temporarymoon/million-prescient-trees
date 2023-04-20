#![allow(dead_code)]

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
// {{{ Edict
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Edict {
    // Victory point edicts
    RileThePublic,
    DivertAttention,
    // Strength edicts
    Sabotage,
    Gambit,
    Ambush,
}

impl Display for Edict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Edict {
    pub const EDICTS: [Edict; 5] =  [
        Edict::RileThePublic,
        Edict::DivertAttention,
        Edict::Sabotage,
        Edict::Gambit,
        Edict::Ambush,
    ];
}

// }}}
// {{{ Battlefield
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Battlefield {
    Mountain,
    Glade,
    Urban,
    LastStrand,
    Night,
    Plains,
}

use Battlefield::*;

use crate::helpers::{
    bitfield::Bitfield,
    ranged::MixRanged,
    subpair::{decode_subpair, encode_subpair},
};

impl Battlefield {
    pub const BATTLEFIELDS: [Battlefield; 6] = [Mountain, Glade, Urban, Night, LastStrand, Plains];

    // Amount of points rewarded for winning a battle
    // in this location (top-left of card)
    pub fn reward(self) -> u8 {
        match self {
            LastStrand => 5,
            _ => 3,
        }
    }

    pub fn bonus(self, creature: Creature) -> bool {
        match (self, creature) {
            (Mountain, Ranger | Barbarian | Mercenary) => true,
            (Glade, Bard | Ranger | Witch) => true,
            (Urban, Rogue | Bard | Diplomat | Steward) => true,
            (Night, Seer | Rogue | Ranger) => true,
            _ => false,
        }
    }
}

impl Display for Battlefield {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
// }}}
// {{{ PlayerStatusEffect
/// Different kind of lingering effects affecting a given player
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum PlayerStatusEffect {
    // === Effects caused by battlefields:
    // The player gains 1 strength
    Mountain,
    // The player gains +2 points if they win this battle
    Glade,

    // === Effects caused by creatures:
    // The player gets to play two creatures instead of one
    Seer,
    // The player gains 1 strength and gains
    // an additional point by winning this battle
    Bard,
    // This battle, lose 1 strength
    Mercenary,
    // The barbarian gains 2 strength if
    // it gets played
    Barbarian,
}

impl PlayerStatusEffect {
    pub const PLAYER_STATUS_EFFECTS: [PlayerStatusEffect; 6] = [
        PlayerStatusEffect::Mountain,
        PlayerStatusEffect::Glade,
        PlayerStatusEffect::Seer,
        PlayerStatusEffect::Bard,
        PlayerStatusEffect::Mercenary,
        PlayerStatusEffect::Barbarian,
    ];
}

impl Display for PlayerStatusEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Lingering effects affecting both players
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum GlobalStatusEffect {
    Night,
}
// }}}
// {{{ Bitfields
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct CreatureSet(pub Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct EdictSet(pub Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PlayerStatusEffects(pub Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct GlobalStatusEffects(pub Bitfield);

impl CreatureSet {
    pub fn all() -> Self {
        CreatureSet(Bitfield::n_ones(11))
    }

    pub fn others(&self) -> Self {
        CreatureSet(self.0.invert_last_n(11))
    }

    pub fn has(&self, creature: Creature) -> bool {
        self.0.has(creature as u8)
    }

    pub fn count_from_end(&self, target: Creature) -> CreatureIndex {
        assert!(
            self.has(target),
            "Bitfield {:?} does not contain creature {:?} (represented as {})",
            self.0,
            target,
            target as u8
        );

        CreatureIndex(self.0.count_from_end(target as u8))
    }

    pub fn lookup_from_end(&self, index: CreatureIndex) -> Option<Creature> {
        self.0
            .lookup_from_end(index.0)
            .map(|x| Creature::CREATURES[x])
    }
}

impl EdictSet {
    pub fn all() -> Self {
        EdictSet(Bitfield::n_ones(5))
    }

    pub fn has(&self, edict: Edict) -> bool {
        self.0.has(edict as u8)
    }

    pub fn len(&self) -> u8 {
        let result = self.0.len();
        assert!(result <= 11); // Sanity checks
        result
    }

    pub fn count_from_end(&self, target: Edict) -> EdictIndex {
        assert!(
            self.has(target),
            "Bitfield {:?} does not contain edict {:?} (represented as {})",
            self.0,
            target,
            target as u8
        );

        EdictIndex(self.0.count_from_end(target as u8))
    }

    pub fn lookup_from_end(&self, index: EdictIndex) -> Option<Edict> {
        self.0.lookup_from_end(index.0).map(|x| Edict::EDICTS[x])
    }
}

impl PlayerStatusEffects {
    pub fn new() -> Self {
        PlayerStatusEffects(Bitfield::default())
    }

    pub fn all() -> Self {
        PlayerStatusEffects(Bitfield::n_ones(
            PlayerStatusEffect::PLAYER_STATUS_EFFECTS.len() as u8,
        ))
    }

    pub fn has(&self, effect: PlayerStatusEffect) -> bool {
        self.0.has(effect as u8)
    }
}
// }}}
// {{{ Players
#[derive(Debug)]
pub enum Player {
    Me,  // Current player
    You, // Opponent
}
// }}}
// {{{ Bitfield indices
/// Represents an index of a bit in an edict set.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EdictIndex(pub u8);

/// Represents an index of a bit in a creature set.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CreatureIndex(pub u8);

/// Either a single index or a pair of them into a creature set,
/// where the order does not matter.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CreatureChoice(pub u8);

impl CreatureChoice {
    pub fn encode_one(index: CreatureIndex) -> Self {
        CreatureChoice(index.0)
    }

    pub fn encode_two(first: CreatureIndex, second: CreatureIndex) -> Option<Self> {
        encode_subpair((first.0, second.0)).map(CreatureChoice)
    }

    pub fn decode_one(self) -> CreatureIndex {
        CreatureIndex(self.0)
    }

    pub fn decode_two(self) -> Option<(CreatureIndex, CreatureIndex)> {
        let (a, b) = decode_subpair(self.0)?;
        Some((CreatureIndex(a), CreatureIndex(b)))
    }
}
// }}}
