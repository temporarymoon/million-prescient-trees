#![allow(dead_code)]

use std::mem::size_of;
use std::u8;

use crate::game::known_state::KnownState;
use crate::game::types::{Player, Score};
use crate::helpers::{normalize_vec, roulette, Pair};
use bumpalo::Bump;
use rand::Rng;

// {{{ Helper types
/// Utility is the quantity players attempt to maximize.
pub type Utility = f32;

/// Float between 0 and 1.
pub type Probability = f32;
// }}}
// {{{ Decision vector
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

    /// Estimates how much memory an instance of this type will take.
    pub fn estimate_alloc(size: usize) -> usize {
        size_of::<f32>() * size * 2 + size_of::<Self>()
    }

    /// Returns the number of actions we can take at this node.
    #[inline(always)]
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
    #[inline(always)]
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
/// A decision matrix holds all the decision weights for a certain player
/// (in a certain known game state).
///
/// We don't have to expand this mapping out if the player can make a single decision.
pub enum DecisionMatrix<'a> {
    Trivial,
    Expanded(&'a mut [DecisionVector<'a>]),
}

impl<'a> DecisionMatrix<'a> {
    pub fn estimate_alloc(matrix_size: usize, vector_size: usize) -> usize {
        size_of::<Self>()
            + if vector_size == 1 {
                1
            } else {
                matrix_size * DecisionVector::estimate_alloc(vector_size)
            }
    }

    pub fn estimate_weight_storage(matrix_size: usize, vector_size: usize) -> usize {
        size_of::<Self>()
            + if vector_size == 1 {
                1
            } else {
                matrix_size * vector_size * 2
            }
    }

    pub fn new(matrix_size: usize, vector_size: usize, allocator: &'a Bump) -> DecisionMatrix<'a> {
        assert!(
            vector_size >= 1,
            "Players always have at least one valid decision"
        );
        assert!(
            matrix_size >= 1,
            "Players always have at least one valid state to be in"
        );

        if vector_size == 1 {
            Self::Trivial
        } else {
            Self::Expanded(allocator.alloc_slice_fill_with(matrix_size, |_| {
                DecisionVector::new(vector_size, allocator)
            }))
        }
    }

    /// Computes the number of decisions in the vector.
    ///
    /// This number is known by both players, so no hidden information
    /// is required for it's compuation.
    pub fn len(&self) -> usize {
        match self {
            Self::Trivial => 1,
            Self::Expanded(vectors) => vectors[0].len(),
        }
    }
}
// }}}
// {{{ Decision matrices
/// A pair of decision matrices
pub struct DecisionMatrices<'a> {
    pub matrices: Pair<DecisionMatrix<'a>>,
}

impl<'a> DecisionMatrices<'a> {
    pub fn new(me: DecisionMatrix<'a>, you: DecisionMatrix<'a>) -> Self {
        Self {
            matrices: (me, you),
        }
    }

    pub fn decision_count(&self) -> (usize, usize) {
        (self.matrices.0.len(), self.matrices.1.len())
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

/// Holds additional information about the current scope we are in.
/// This information depends on the type of phase the scope represents.
#[derive(Debug)]
pub enum ExploredScopeExtraInfo {
    Main(MainExtraInfo),
    Sabotage(SabotageExtraInfo),
    Seer,
}

/// An explored scope is a scope where all the game rules have
/// been unrolled and all the game states have been created.
pub struct ExploredScope<'a> {
    /// Describes what kind of scopes we are in.
    // pub kind: ExploredScopeExtraInfo,

    /// The decision matrix holds all the weights generated by training.
    pub matrices: DecisionMatrices<'a>,

    /// Vector of possible future states.
    pub next: &'a mut [Scope<'a>],
}
// }}}
// {{{ Unexplored scope
/// An explored scope is a scope where all the game rules have
/// been unrolled and all the game states have been created.
// TODO: add utility tables
pub struct UnexploredScope<'a> {
    pub state: Option<&'a KnownState>,
}
// }}}
// {{{ Scope
pub enum Scope<'a> {
    Completed(Score),
    Unexplored(UnexploredScope<'a>),
    Explored(ExploredScope<'a>),
}
// }}}
