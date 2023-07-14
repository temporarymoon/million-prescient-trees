use super::{creature::Creature, edict::Edict};

// {{{ Main phase choice
// Choice made by one of the players in the main phase
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct MainPhaseChoice {
    pub edict: Edict,

    // The player is only allowed to play two creatures
    // if the "seer" status effect is active
    pub creatures: (Creature, Option<Creature>),
}

impl MainPhaseChoice {
    pub fn to_final(self) -> Option<FinalMainPhaseChoice> {
        if self.creatures.1.is_some() {
            None
        } else {
            Some(FinalMainPhaseChoice::new(self.creatures.0, self.edict))
        }
    }
}
// }}}
// {{{ Final main phase choice
// Similar to MainPhaseChoice but used after the seer phase gets resolved
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct FinalMainPhaseChoice {
    pub creature: Creature,
    pub edict: Edict,
}

impl FinalMainPhaseChoice {
    #[inline(always)]
    pub fn new(creature: Creature, edict: Edict) -> Self {
        Self { creature, edict }
    }
}
// }}}

pub type SabotagePhaseChoice = Option<Creature>;
pub type SeerPhaseChoice = Option<Creature>;
