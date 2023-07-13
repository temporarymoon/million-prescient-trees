use std::{
    debug_assert,
    fmt::{self, Display},
    ops::{BitOr, Not, BitAnd},
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
    // The player gains +2 vp if they win this battle
    Glade,
    // The player gains +1 vp if they win this batttle
    Night,

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
// }}}
// {{{ Bitfields
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Default)]
pub struct CreatureSet(pub Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct EdictSet(pub Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Default)]
pub struct PlayerStatusEffects(pub Bitfield);

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
    pub fn add(&mut self, creature: Creature) {
        self.0.add(creature as usize)
    }

    #[inline]
    pub fn remove(&mut self, creature: Creature) {
        self.0.remove(creature as usize)
    }

    #[inline]
    pub fn has(self, creature: Creature) -> bool {
        self.0.has(creature as usize)
    }

    #[inline]
    pub fn len(self) -> usize {
        let result = self.0.len();
        debug_assert!(result <= 11); // Sanity checks
        result
    }

    #[inline]
    pub fn indexof(self, target: Creature) -> CreatureIndex {
        self.0.count_from_end(target as usize)
    }

    #[inline]
    pub fn index(self, index: CreatureIndex) -> Option<Creature> {
        self.0
            .lookup_from_end(index)
            .map(|x| Creature::CREATURES[x])
    }

    #[inline]
    pub fn encode_relative_to(self, other: Self) -> Bitfield {
        self.0.encode_relative_to(other.0)
    }

    #[inline]
    pub fn decode_relative_to(bitfield: Bitfield, other: CreatureSet) -> Option<Self> {
        Some(Self(bitfield.decode_relative_to(other.0)?))
    }

    /// Computes the number of hands of a given size with cards from the current set.
    #[inline]
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

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for CreatureSet {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl Not for CreatureSet {
    type Output = Self;

    #[inline]
    fn not(self) -> Self {
        CreatureSet(self.0.invert_last_n(11))
    }
}
// }}}
// }}}
// {{{ EdictSet
impl EdictSet {
    #[inline]
    pub fn empty() -> Self {
        EdictSet(Bitfield::default())
    }

    #[inline]
    pub fn all() -> Self {
        EdictSet(Bitfield::n_ones(5))
    }

    #[inline]
    pub fn remove(&mut self, edict: Edict) {
        self.0.remove(edict as usize)
    }

    #[inline]
    pub fn has(self, edict: Edict) -> bool {
        self.0.has(edict as usize)
    }

    #[inline]
    pub fn len(self) -> usize {
        let result = self.0.len();
        debug_assert!(result <= 5); // Sanity checks
        result
    }

    #[inline]
    pub fn indexof(self, target: Edict) -> EdictIndex {
        self.0.count_from_end(target as usize)
    }

    #[inline]
    pub fn index(self, index: EdictIndex) -> Option<Edict> {
        self.0.lookup_from_end(index).map(|x| Edict::EDICTS[x])
    }
}

impl Default for EdictSet {
   fn default() -> Self {
       Self::all()
   } 
}
// }}}
// {{{ PlayerStatusEffects
impl PlayerStatusEffects {
    #[inline]
    pub fn all() -> Self {
        PlayerStatusEffects(Bitfield::n_ones(
            PlayerStatusEffect::PLAYER_STATUS_EFFECTS.len(),
        ))
    }

    #[inline]
    pub fn has(self, effect: PlayerStatusEffect) -> bool {
        self.0.has(effect as usize)
    }

    /// Sets all bits to zero.
    #[inline]
    pub fn clear(&mut self) {
        self.0.clear()
    }

    #[inline]
    pub fn add(&mut self, effect: PlayerStatusEffect) {
        self.0.add(effect as usize)
    }
}
// }}}
// {{{ Players
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Player {
    Me,  // Current player
    You, // Opponent
}

impl Not for Player {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Player::Me => Player::You,
            Player::You => Player::Me,
        }
    }
}

impl Player {
    /// List of all players.
    pub const PLAYERS: [Self; 2] = [Player::Me, Player::You];

    /// Index a pair by a player,
    /// where the first and second elements represents the data
    /// for the current and other players respectively.
    #[inline]
    pub fn select<T>(self, pair: (T, T)) -> T {
        match self {
            Player::Me => pair.0,
            Player::You => pair.1,
        }
    }

    #[inline]
    pub fn select_mut<T>(self, pair: &mut (T, T)) -> &mut T {
        match self {
            Player::Me => &mut pair.0,
            Player::You => &mut pair.1,
        }
    }
}

// }}}
// }}}
// {{{ Bitfield indices
/// Represents an index of a bit in an edict set.
pub type EdictIndex = usize;

/// Represents an index of a bit in a creature set.
pub type CreatureIndex = usize;

// {{{ UserCreatureChoice
/// User facing version of `CreatureChoice`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UserCreatureChoice(pub Creature, pub Option<Creature>);

impl UserCreatureChoice {
    /// The number of cards chosen by the user (either `1` or `2`).
    #[inline]
    pub fn len(self) -> usize {
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
/// The result fits inside an `u8`, but we are
/// using an `usize` for convenience.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CreatureChoice(pub usize);

impl CreatureChoice {
    /// Encode a one/two creature choice into a single integer, removing any info
    /// about the number of chosen creatures and the contents of the graveyard from the
    /// resulting integer.
    pub fn encode_user_choice(user_choice: UserCreatureChoice, possibilities: CreatureSet) -> Self {
        Self(
            user_choice
                .as_creature_set()
                .encode_relative_to(possibilities)
                .encode_ones(),
        )
    }

    /// Inverse of `encode_user_choice`.
    pub fn decode_user_choice(
        self,
        possibilities: CreatureSet,
        seer_active: bool,
    ) -> Option<UserCreatureChoice> {
        let length = UserCreatureChoice::len_from_status(seer_active);
        let decoded =
            CreatureSet::decode_relative_to(Bitfield::decode_ones(self.0, length)?, possibilities)?;

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
