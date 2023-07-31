use super::creature::Creature;
use std::fmt::{self, Display};
use Battlefield::*;

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
        use Creature::*;

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
// {{{ Battlefields
/// List of battlefields used in a battle.
// TODO: consider sharing battlefields.all
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Battlefields {
    pub all: [Battlefield; 4],
    pub current: usize,
}

impl Battlefields {
    pub const fn new(all: [Battlefield; 4]) -> Self {
        Battlefields { all, current: 0 }
    }

    pub fn is_last(&self) -> bool {
        self.current == 3
    }

    pub fn next(&self) -> Option<Self> {
        if self.is_last() {
            None
        } else {
            Some(Battlefields {
                all: self.all,
                current: self.current + 1,
            })
        }
    }

    pub fn active(&self) -> &[Battlefield] {
        &self.all[self.current..]
    }

    pub fn current(&self) -> Battlefield {
        self.all[self.current]
    }

    /// Returns whether a given battlefield will ever be active
    pub fn will_be_active(&self, battlefield: Battlefield) -> bool {
        self.active().into_iter().find(|b| **b == battlefield).is_some()
    }
}
// }}}
