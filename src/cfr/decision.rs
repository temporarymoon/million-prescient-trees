#![allow(dead_code)]

use super::decision_index::DecisionIndex;
use crate::game::types::Player;
use crate::helpers::{normalize_vec, roulette, swap::Pair};
use bumpalo::Bump;
use rand::Rng;

// {{{ Helper types
/// Utility is the quantity players attempt to maximize.
pub type Utility = f32;

/// Float between 0 and 1.
pub type Probability = f32;
// }}}
// {{{ Decision vector
// {{{ Types
/// A decision a player takes in the game.
///
/// For efficiency, all the values are tightly packed into vectors indexed
/// by so called "decision indices", which are encoded/decoded differently
/// depending on the phase of the game we are currently in.
pub struct DecisionVector<'a> {
    /// Sum of every strategy devised so far during training.
    /// Unintuitively, the current strategy doesn't approach
    /// optimal play, but the sum of devised strategies does!
    strategy_sum: &'a mut [f32],

    /// Regret accumulated during training (so far).
    regret_sum: &'a mut [f32],

    /// Cached value of the positive elements in the regret_sum vector.
    regret_positive_magnitude: f32,

    /// The probabilities of each player taking the actions required to reach this state.
    realization_weights: (Probability, Probability),
}
// }}}

impl<'a> DecisionVector<'a> {
    // {{{ Helpers
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
    // }}}
    // {{{ Training-related methods
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
    // }}}
}
// }}}
// {{{ Decision matrix
pub type DecisionRows<'a> = &'a mut [DecisionVector<'a>];

/// A decision matrix contains weights for the decisions both players can take.
/// Conceptually, this actually represents a pair of matrices (one for each player).
/// Each matrix can be indexed by the information a particular player holds to yield
/// the *DecisionVector*.
pub struct DecisionMatrix<'a> {
    pub vectors: Pair<DecisionRows<'a>>,
}

impl<'a> DecisionMatrix<'a> {
    pub fn new(me: DecisionRows<'a>, you: DecisionRows<'a>) -> Self {
        Self { vectors: (me, you) }
    }

    pub fn decision_count(&self) -> (usize, usize) {
        (self.vectors.0[0].len(), self.vectors.0[1].len())
    }
}
// }}}
// {{{ Explored scope
// {{{ Extra info
/// Information we need to keep track of for main phases.
#[derive(Debug)]
pub struct MainExtraInfo {
    pub edict_counts: (u8, u8),
}

/// Information we need to keep track of for sabotage phases.
#[derive(Debug)]
pub struct SabotageExtraInfo {
    /// The player about to enter a seer phase.
    /// If neither players is entering one,
    /// the value of this does not matter.
    pub seer_player: Player,
}
// }}}

/// An index into a player's hand.
/// More efficiently packed than keeping the absolute id of the card.
pub type CreatureIndex = usize;

/// Holds additional information about the current scope we are in.
/// This information depends on the type of phase the scope represents.
#[derive(Debug)]
pub enum ExploredScopeExtraInfo {
    Main(MainExtraInfo),
    Sabotage(SabotageExtraInfo),
    Seer,
}

/// Hidden information which needs to be carried out for the current scope.
/// The overseer / hand-content is implicit here.
#[derive(Debug)]
pub enum ExploredScopeHiddenInfo {
    PreMain,
    PreSabotage(CreatureIndex, CreatureIndex),
    PreSeer(CreatureIndex),
}

/// An explored scope is a scope where all the game rules have
/// been unrolled and all the game states have been created.
pub struct ExploredScope<'a> {
    /// Describes what kind of scopes we are in.
    pub kind: ExploredScopeExtraInfo,

    /// The decision matrix holds all the weights generated by training.
    pub matrix: DecisionMatrix<'a>,

    /// Vector of possible future states.
    pub next: Vec<Scope<'a>, &'a Bump>,
}

impl<'a> ExploredScope<'a> {
    pub fn get_next(
        &self,
        _decisions: (DecisionIndex, DecisionIndex),
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
