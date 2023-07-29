use crate::{helpers::bitfield::{Bitfield16, Bitfield}, make_bitfield};
use std::fmt::{self, Display};

/// Different kind of lingering effects affecting a given player
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum StatusEffect {
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

impl StatusEffect {
    pub const STATUS_EFFECTS: [StatusEffect; 6] = [
        StatusEffect::Mountain,
        StatusEffect::Glade,
        StatusEffect::Seer,
        StatusEffect::Bard,
        StatusEffect::Mercenary,
        StatusEffect::Barbarian,
    ];
}

impl Display for StatusEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<usize> for StatusEffect {
   fn from(value: usize) -> Self {
       StatusEffect::STATUS_EFFECTS[value]
   } 
}

make_bitfield!(StatusEffectSet, StatusEffect, u8, 7, Bitfield16, true);

