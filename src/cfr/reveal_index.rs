use crate::{
    game::{
        creature::{Creature, CreatureSet},
        edict::{Edict, EdictSet},
    },
    helpers::ranged::MixRanged,
};

/// Encodes all the information revealed at the end of a phase.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct RevealIndex(pub usize);

impl RevealIndex {
    // {{{ Main phase
    pub fn encode_main_phase_reveal(choices: (Edict, Edict), edicts: (EdictSet, EdictSet)) -> Self {
        let p1_index = edicts.0.indexof(choices.0);
        let p2_index = edicts.1.indexof(choices.1);

        Self(p2_index.mix_ranged(p1_index, edicts.0.len()))
    }

    pub fn main_phase_count(player_edicts: (EdictSet, EdictSet)) -> usize {
        player_edicts.0.len() * player_edicts.1.len()
    }
    // }}}
    // {{{ Sabotage & seer phases
    pub fn encode_sabotage_seer_phase_reveal(creature: Creature, graveyard: CreatureSet) -> Self {
        Self((!graveyard).indexof(creature))
    }

    pub fn sabotage_seer_phase_count(graveyard: CreatureSet) -> usize {
        (!graveyard).len()
    }
    // }}}
}
