#![allow(dead_code)]

use std::{
    debug_assert,
    fmt::{self, Display}
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

use crate::helpers::{bitfield::Bitfield, choose::choose};

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

// {{{ CreatureSet
impl CreatureSet {
    #[inline]
    pub fn singleton(creature: Creature) -> Self {
        CreatureSet(Bitfield::singleton(creature as u8))
    }

    #[inline]
    pub fn all() -> Self {
        CreatureSet(Bitfield::n_ones(11))
    }

    #[inline]
    pub fn others(&self) -> Self {
        CreatureSet(self.0.invert_last_n(11))
    }

    #[inline]
    pub fn add(&mut self, creature: Creature) {
        self.0.add(creature as u8)
    }

    #[inline]
    pub fn remove(&mut self, creature: Creature) {
        self.0.remove(creature as u8)
    }

    #[inline]
    pub fn has(&self, creature: Creature) -> bool {
        self.0.has(creature as u8)
    }

    #[inline]
    pub fn len(&self) -> u8 {
        let result = self.0.len();
        debug_assert!(result <= 11); // Sanity checks
        result
    }

    #[inline]
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

    #[inline]
    pub fn lookup_from_end(&self, index: CreatureIndex) -> Option<Creature> {
        self.0
            .lookup_from_end(index.0)
            .map(|x| Creature::CREATURES[x])
    }

    #[inline]
    pub fn encode_relative_to(&self, other: Self) -> Self {
        Self(self.0.encode_relative_to(other.0))
    }

    #[inline]
    pub fn decode_relative_to(&self, other: CreatureSet) -> Option<Self> {
        Some(Self(self.0.decode_relative_to(other.0)?))
    }

    #[inline]
    pub fn encode_ones(&self) -> u16 {
        self.0.encode_ones()
    }

    #[inline]
    pub fn decode_ones(encoded: u16, ones: usize) -> Option<Self> {
        Some(Self(Bitfield::decode_ones(encoded, ones)?))
    }

    /// Computes the number of hands of a given size with cards from the current set.
    #[inline]
    pub fn hands_of_size(&self, size: usize) -> usize {
        choose(self.len() as usize, size)
    }

    #[inline]
    pub fn union(&self, other: &Self) -> Self {
        Self(self.0.union(&other.0))
    }

}

// {{{ IntoIter
pub struct CreatureSetIterator {
    index: u8,
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

impl Default for CreatureSet {
    fn default() -> Self {
        Self(Bitfield::default())
    }
}
// }}}
// {{{ EdictSet
impl EdictSet {
    #[inline]
    pub fn all() -> Self {
        EdictSet(Bitfield::n_ones(5))
    }

    #[inline]
    pub fn remove(&mut self, edict: Edict) {
        self.0.remove(edict as u8)
    }

    #[inline]
    pub fn has(&self, edict: Edict) -> bool {
        self.0.has(edict as u8)
    }

    #[inline]
    pub fn len(&self) -> u8 {
        let result = self.0.len();
        debug_assert!(result <= 5); // Sanity checks
        result
    }

    #[inline]
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

    #[inline]
    pub fn lookup_from_end(&self, index: EdictIndex) -> Option<Edict> {
        self.0.lookup_from_end(index.0).map(|x| Edict::EDICTS[x])
    }
}
// }}}
// {{{ PlayerstatusEffects
impl PlayerStatusEffects {
    #[inline]
    pub fn new() -> Self {
        PlayerStatusEffects(Bitfield::default())
    }

    #[inline]
    pub fn all() -> Self {
        PlayerStatusEffects(Bitfield::n_ones(
            PlayerStatusEffect::PLAYER_STATUS_EFFECTS.len() as u8,
        ))
    }

    #[inline]
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

// {{{ UserCreatureChoice
/// User facing version of `CreatureChoice`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UserCreatureChoice(pub Creature, pub Option<Creature>);

impl UserCreatureChoice {
    /// The number of cards chosen by the user (either `1` or `2`).
    #[inline]
    pub fn len(&self) -> usize {
        if self.1.is_some() {
            2
        } else {
            1
        }
    }

    /// Returns the length of some user creature choice based
    /// on whether the seer status effect is active or not.
    #[inline]
    pub fn len_from_status(seer_active: bool) -> usize {
        if seer_active {
            2
        } else {
            1
        }
    }

    pub fn as_creature_set(self) -> CreatureSet {
        let mut bitfield = CreatureSet::default();
        bitfield.add(self.0);

        if let Some(second) = self.1 {
            bitfield.add(second);
        }

        bitfield
    }
}
// }}}
// {{{ CreatureChoice
/// Encoded version of `UserCreatureChoice`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CreatureChoice(pub u8);

impl CreatureChoice {
    /// Encode a one/two creature choice into a single integer, removing any info
    /// about the number of chosen creatures and the contents of the graveyard from the
    /// resulting integer.
    pub fn encode_user_choice(user_choice: UserCreatureChoice, possibilities: CreatureSet) -> Self {
        Self(
            user_choice
                .as_creature_set()
                .encode_relative_to(possibilities)
                .encode_ones() as u8,
        )
    }

    /// Inverse of `encode_user_choice`.
    pub fn decode_user_choice(
        self,
        possibilities: CreatureSet,
        seer_active: bool,
    ) -> Option<UserCreatureChoice> {
        let length = UserCreatureChoice::len_from_status(seer_active);
        let encoded = self.0 as u16;
        let decoded =
            CreatureSet::decode_ones(encoded, length)?.decode_relative_to(possibilities)?;

        let mut creatures = decoded.into_iter();

        let first = creatures.next()?;
        if seer_active {
            let second = creatures.next()?;
            Some(UserCreatureChoice(first, Some(second)))
        } else {
            Some(UserCreatureChoice(first, None))
        }
    }
}
// }}}
// }}}
