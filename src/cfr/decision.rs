#![allow(dead_code)]

use crate::{
    game::types::{CreatureSet, Edict, EdictSet, Player},
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
    strategy_sum: Vec<f32, &'a Bump>,
    regret_sum: Vec<f32, &'a Bump>,
    realization_weights: (f32, f32),
}

impl<'a> DecisionVector<'a> {
    pub fn new(size: usize, allocator: &'a Bump) -> Self {
        let mut regret_sum = Vec::with_capacity_in(size, allocator);
        let mut strategy_sum = Vec::with_capacity_in(size, allocator);

        regret_sum.resize(size, 0.0);
        strategy_sum.resize(size, 0.0);

        Self {
            regret_sum,
            strategy_sum,
            realization_weights: (0.0, 0.0),
        }
    }

    /// Returns the number of actions we can take at this node.
    pub fn len(&self) -> usize {
        self.regret_sum.len()
    }

    /// Update the strategy once some new regret has been accumulated.
    ///
    /// # Arguments
    ///
    /// * `out` - The vector to put the strategy in.
    pub fn strategy<A: Allocator>(&mut self, out: &mut Vec<f32, A>) {
        out.resize(self.len(), 0.0);

        for i in 0..self.len() {
            // We cannot have negative probabilities.
            out[i] = f32::max(self.regret_sum[i], 0.0);

            // TODO: I don't remember why this was here.
            if out[i] < 0.001 {
                out[i] = 0.0;
            }
        }

        normalize_vec(out);

        for i in 0..self.len() {
            self.strategy_sum[i] += self.realization_weights.0 * out[i];
        }
    }

    /// Returns the strategy one should take in an actual game.
    /// Do not use this during training! (Performs a clone)
    pub fn get_average_strategy(&self) -> Vec<f32, &'a Bump> {
        let mut average_strategy = self.strategy_sum.clone();

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

    pub fn encode_main_phase_index(
        main: Creature,
        edict: Edict,
        extra_creature: Option<Creature>,
        graveyard: CreatureSet,
        edicts: EdictSet,
    ) -> usize {
        let mut creature_index = graveyard.others().count_from_end(main);
        if let Some(extra_creature) = extra_creature {
            let second_creature_index = graveyard.count_from_end(extra_creature);
            creature_index = encode_subpair((creature_index, second_creature_index));
        }
        let edict_index = edicts.count_from_end(edict);

        (creature_index as usize).mix_ranged(edict_index as usize, edicts.len() as usize)
    }

    #[allow(unused_variables)]
    pub fn decode_main_phase_index(
        edicts: EdictSet,
        graveyard: CreatureSet,
        index: usize,
    ) -> (usize, Edict) {
        let (creature_index, edict_index) = index.unmix_ranged(edicts.len() as usize);
        todo!()
    }
}
// }}}
// {{{ Decision matrix
pub type ScopeDecisionColumn<'a> = Vec<DecisionVector<'a>, &'a Bump>;
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
