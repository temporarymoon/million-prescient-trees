use crate::helpers::bitfield::Bitfield;
use std::fmt::{self, Display};

// {{{ StatusEffect
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
    pub const PLAYER_STATUS_EFFECTS: [StatusEffect; 6] = [
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
// }}}
// {{{ StatusEffectSet
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Default)]
pub struct StatusEffectSet(pub Bitfield);

impl StatusEffectSet {
    #[inline(always)]
    pub fn all() -> Self {
        StatusEffectSet(Bitfield::n_ones(StatusEffect::PLAYER_STATUS_EFFECTS.len()))
    }

    #[inline(always)]
    pub fn has(self, effect: StatusEffect) -> bool {
        self.0.has(effect as usize)
    }

    /// Sets all bits to zero.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.0.clear()
    }

    #[inline(always)]
    pub fn add(&mut self, effect: StatusEffect) {
        self.0.add(effect as usize)
    }
}
// }}}
