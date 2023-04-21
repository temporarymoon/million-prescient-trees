#![allow(dead_code)]

use crate::{
    game::types::{CreatureChoice, CreatureSet, Edict, EdictIndex, EdictSet, Player},
    helpers::{ranged::MixRanged, subpair::encode_subpair},
};
use std::alloc::Allocator;

use bumpalo::Bump;
use rand::Rng;

use crate::{
    game::types::Creature,
    helpers::{normalize_vec, roulette, swap::Pair},
};

/// Utility is the quantity players attempt to maximize.
pub type Utility = f32;

// {{{ Decision vector
pub struct DecisionVector<'a> {
    strategy_sum: &'a mut [f32],
    regret_sum: &'a mut [f32],
    regret_positive_magnitude: f32,
    realization_weights: (f32, f32),
}

impl<'a> DecisionVector<'a> {
    pub fn new(size: usize, allocator: &'a Bump) -> Self {
        let regret_sum = allocator.alloc_slice_fill_copy(size, 0.0);
        let strategy_sum = allocator.alloc_slice_fill_copy(size, 0.0);

        Self {
            regret_sum,
            regret_positive_magnitude: 0.0,
            strategy_sum,
            realization_weights: (0.0, 0.0),
        }
    }

    /// Returns the number of actions we can take at this node.
    #[inline]
    pub fn len(&self) -> usize {
        self.regret_sum.len()
    }

    /// Compute the ith value of the strategy.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the strategy to compute
    #[inline]
    pub fn strategy(&self, index: usize) -> f32 {
        if self.regret_positive_magnitude > 0.0 {
            f32::max(self.regret_sum[index], 0.0) / self.regret_positive_magnitude
        } else {
            1.0 / (self.len() as f32)
        }
    }

    /// Update the strategy sum with the current strategy.
    pub fn update_strategy_sum(&mut self) {
        for i in 0..self.len() {
            self.strategy_sum[i] += self.strategy(i);
        }
    }

    /// Updates the cached regret magnitude once the regret sum has been changed.
    pub fn recompute_regret_magnitude(&mut self) {
        let mut sum = 0.0;
        for i in 0..self.len() {
            sum += f32::max(self.regret_sum[i], 0.0);
        }
        self.regret_positive_magnitude = sum;
    }

    /// Returns the strategy one should take in an actual game.
    /// Do not use this during training! (Performs a clone)
    pub fn get_average_strategy(&self) -> Vec<f32> {
        let mut average_strategy = self.strategy_sum.to_vec();

        normalize_vec(&mut average_strategy);

        average_strategy
    }

    /// Returns a random action based on the probability distribution
    /// in self.strategy_sum.
    ///
    /// TODO: perform normalization on-the-fly to avoid a .clone
    ///       (not very urgent, as this is never called during training)
    pub fn random_action<R: Rng>(&self, rng: &mut R) -> usize {
        let average = self.get_average_strategy();

        roulette(&average, rng)
    }

    /// Encodes a main phase user choice into a decision index.
    pub fn encode_main_phase_index_user(
        creatures: (Creature, Option<Creature>),
        edict: Edict,
        edicts: EdictSet,
        graveyard: CreatureSet,
    ) -> Option<usize> {
        let edict = edicts.count_from_end(edict);
        let creature_set = graveyard.others();
        let first_creature_index = creature_set.count_from_end(creatures.0);
        let creatures = match creatures.1 {
            Some(second_creature) => {
                let second_creature_index = creature_set.count_from_end(second_creature);
                CreatureChoice::encode_two(first_creature_index, second_creature_index)?
            }
            None => CreatureChoice::encode_one(first_creature_index),
        };

        Some(Self::encode_main_phase_index(
            creatures,
            edict,
            edicts.len(),
        ))
    }

    /// Encodes a main phase "internal" choice into a decision index.
    pub fn encode_main_phase_index(
        creatures: CreatureChoice,
        edict: EdictIndex,
        edict_count: u8,
    ) -> usize {
        (creatures.0 as usize).mix_ranged(edict.0 as usize, edict_count as usize)
    }

    /// Decodes a main phase "internal" choice into a decision index.
    pub fn decode_main_phase_index(index: usize, edict_count: u8) -> (CreatureChoice, EdictIndex) {
        let (creatures, edict) = index.unmix_ranged(edict_count as usize);
        (CreatureChoice(creatures as u8), EdictIndex(edict as u8))
    }

    /// Decodes a main phase user choice into a decision index.
    pub fn decode_main_phase_index_user(
        index: usize,
        edicts: EdictSet,
        graveyard: CreatureSet,
        seer_active: bool,
    ) -> Option<(Creature, Option<Creature>, Edict)> {
        let (creature_choice, edict_index) = Self::decode_main_phase_index(index, edicts.len());
        let edict = edicts.lookup_from_end(edict_index)?;
        let creature_set = graveyard.others();
        if seer_active {
            let (creature_one, creature_two) = creature_choice.decode_two()?;
            Some((
                creature_set.lookup_from_end(creature_one)?,
                Some(creature_set.lookup_from_end(creature_two)?),
                edict,
            ))
        } else {
            let creature_index = creature_choice.decode_one();
            Some((creature_set.lookup_from_end(creature_index)?, None, edict))
        }
    }
}

#[cfg(test)]
mod decision_vector_tests {
    use super::*;
    #[test]
    fn encode_decode_main_inverses_seer() {
        for creature_choice in 0..100 {
            for edicts_len in 1..5 {
                for edict in 0..edicts_len {
                    let encoded = DecisionVector::encode_main_phase_index(
                        CreatureChoice(creature_choice),
                        EdictIndex(edict),
                        edicts_len,
                    );

                    let decoded = DecisionVector::decode_main_phase_index(encoded, edicts_len);

                    assert_eq!(
                        decoded,
                        (CreatureChoice(creature_choice), EdictIndex(edict))
                    );
                }
            }
        }
    }

    #[test]
    fn encode_decode_main_user_inverses_seer() {
        let mut edicts = EdictSet::all();
        edicts.0.remove(Edict::DivertAttention as u8);

        let mut graveyard = CreatureSet::all().others();
        graveyard.0.add(Creature::Seer as u8);
        graveyard.0.add(Creature::Steward as u8);

        for creature_one in Creature::CREATURES {
            for creature_two in Creature::CREATURES {
                if creature_one <= creature_two
                    || graveyard.has(creature_one)
                    || graveyard.has(creature_two)
                {
                    continue;
                };

                for edict in Edict::EDICTS {
                    if !edicts.has(edict) {
                        continue;
                    };

                    let encoded = DecisionVector::encode_main_phase_index_user(
                        (creature_one, Some(creature_two)),
                        edict,
                        edicts,
                        graveyard,
                    );

                    let decoded = encoded.and_then(|encoded| {
                        DecisionVector::decode_main_phase_index_user(
                            encoded, edicts, graveyard, true,
                        )
                    });

                    assert_eq!(
                        decoded,
                        Some((creature_one, Some(creature_two), edict)),
                        "The edicts are {:?}, and the current one is {:?} (represented as {}).
                        ",
                        edicts,
                        edict,
                        edict as u8
                    );
                }
            }
        }
    }
}
// }}}
// {{{ Decision matrix
pub type ScopeDecisionColumn<'a> = &'a mut [DecisionVector<'a>];
pub struct DecisionMatrix<'a> {
    pub vectors: Pair<ScopeDecisionColumn<'a>>,
}

impl<'a> DecisionMatrix<'a> {
    pub fn new(me: ScopeDecisionColumn<'a>, you: ScopeDecisionColumn<'a>) -> Self {
        Self { vectors: (me, you) }
    }

    pub fn decision_count(&self) -> (usize, usize) {
        (self.vectors.0[0].len(), self.vectors.0[1].len())
    }
}
// }}}
// {{{ Explored scope
#[derive(Debug)]
pub struct MainExtraInfo {
    pub edict_counts: (u8, u8),
}

#[derive(Debug)]
pub struct SabotageExtraInfo {
    /// The player about to enter a seer phase.
    /// If neither players is entering one,
    /// the value of this can be whatever.
    pub seer_player: Player,
}

#[derive(Debug)]
pub enum ExploredScopeKind {
    Main(MainExtraInfo),
    Sabotage(SabotageExtraInfo),
    Seer,
}

pub type CreatureIndex = usize;

#[derive(Debug)]
pub enum ExploredScopeHiddenInfo {
    PreSabotage(CreatureIndex, CreatureIndex),
    PreSeer(CreatureIndex),
    PreMain,
}

pub struct ExploredScope<'a> {
    pub matrix: DecisionMatrix<'a>,
    pub kind: ExploredScopeKind,
    pub next: Vec<Scope<'a>, &'a Bump>,
}

impl<'a> ExploredScope<'a> {
    pub fn get_next(
        &self,
        decisions: (usize, usize),
        hidden: ExploredScopeHiddenInfo,
    ) -> (usize, ExploredScopeHiddenInfo) {
        match (&self.kind, hidden) {
            (kind, hidden) => {
                panic!(
                    "Cannot advance from state {:?} given hidden info {:?}",
                    kind, hidden
                )
            }
        }
    }
}

// }}}
// {{{ Scope
pub enum Scope<'a> {
    Unexplored,
    Explored(ExploredScope<'a>),
}

impl<'a> Default for Scope<'a> {
    fn default() -> Self {
        Scope::Unexplored
    }
}
// }}}
