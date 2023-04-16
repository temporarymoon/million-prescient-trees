#![allow(dead_code)]

use std::fmt::{self, Display};

// {{{ Creature
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
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
    pub const EDICTS: [Edict; 5] = [
        Edict::Sabotage,
        Edict::Gambit,
        Edict::Ambush,
        Edict::RileThePublic,
        Edict::DivertAttention,
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

use crate::helpers::bitfield::Bitfield;

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
        CreatureSet(Bitfield::all())
    }

    pub fn others(&self) -> Self {
        CreatureSet(self.0.invert())
    }

    pub fn has(&self, creature: Creature) -> bool {
        self.0.has(creature as u8)
    }

    pub fn count_from_end(&self, target: Creature) -> u8 {
        self.0.count_from_end(target as u8)
    }
}

impl EdictSet {
    pub fn all() -> Self {
        EdictSet(Bitfield::all())
    }

    pub fn has(&self, edict: Edict) -> bool {
        self.0.has(edict as u8)
    }
}

impl PlayerStatusEffects {
    pub fn new() -> Self {
        PlayerStatusEffects(Bitfield::default())
    }

    pub fn all() -> Self {
        PlayerStatusEffects(Bitfield::all())
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
