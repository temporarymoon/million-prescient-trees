use crate::game::known_state::KnownState;
use crate::game::known_state_summary::KnownStateSummary;
use crate::game::simulate::BattleContext;
use crate::game::types::Score;
use crate::helpers::pair::{are_equal, Pair};
use crate::helpers::{normalize_vec, roulette};
use bumpalo::Bump;
use rand::Rng;
use std::mem::size_of;

use super::hidden_index::HiddenIndex;

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
#[derive(Debug)]
pub struct DecisionVector<'a> {
    /// Sum of every strategy devised so far during training.
    /// Unintuitively, the current strategy doesn't approach
    /// optimal play, but the sum of devised strategies does!
    pub strategy_sum: &'a mut [f32],

    /// Regret accumulated during training (so far).
    pub regret_sum: &'a mut [f32],

    /// Cached value of the positive elements in the regret_sum vector.
    regret_positive_magnitude: f32,
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

    /// Attempt to compute the current strategy on a node which might not be there.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the strategy to compute
    #[inline(always)]
    pub fn try_strategy(node: Option<&Self>, index: usize) -> f32 {
        match node {
            None => {
                debug_assert_eq!(index, 0);
                1.0
            }
            Some(node) => node.strategy(index),
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
#[derive(Debug)]
pub enum DecisionMatrix<'a> {
    Trivial,
    Expanded(&'a mut [DecisionVector<'a>]),
}

impl<'a> DecisionMatrix<'a> {
    /// Indexes the matrix, returning `None` if it is trivial. That is, it returns
    /// `None` when the player should be treated as having a single decision they can
    /// (and will) take with probability `1`.
    pub fn get_node_mut(&mut self, index: HiddenIndex) -> Option<&mut DecisionVector<'a>> {
        match self {
            Self::Trivial => None,
            Self::Expanded(vec) => Some(&mut vec[index.0]),
        }
    }

    pub fn estimate_alloc(matrix_size: usize, vector_size: usize) -> usize {
        size_of::<Self>()
            + if vector_size == 1 {
                1
            } else {
                matrix_size * DecisionVector::estimate_alloc(vector_size)
            }
    }

    pub fn estimate_weight_storage(matrix_size: usize, vector_size: usize) -> usize {
        if vector_size == 1 {
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
pub enum DecisionMatrices<'a> {
    Symmetrical(DecisionMatrix<'a>),
    Asymmetrical(Pair<DecisionMatrix<'a>>),
}

impl<'a> DecisionMatrices<'a> {
    pub fn new(
        is_symmetrical: bool,
        hidden_counts: Pair<usize>,
        decision_counts: Pair<usize>,
        allocator: &'a Bump,
    ) -> Self {
        if is_symmetrical {
            assert!(are_equal(decision_counts));
            assert!(are_equal(hidden_counts));

            Self::Symmetrical(DecisionMatrix::new(
                hidden_counts[0],
                decision_counts[0],
                allocator,
            ))
        } else {
            let matrices = hidden_counts
                .into_iter()
                .zip(decision_counts)
                .map(|(hidden, decision)| DecisionMatrix::new(hidden, decision, allocator))
                .next_chunk()
                .unwrap();

            Self::Asymmetrical(matrices)
        }
    }

    pub fn estimate_alloc(
        is_symmetrical: bool,
        hidden_counts: Pair<usize>,
        decision_counts: Pair<usize>,
    ) -> usize {
        if is_symmetrical {
            assert!(are_equal(decision_counts));
            assert!(are_equal(hidden_counts));

            DecisionMatrix::estimate_alloc(hidden_counts[0], decision_counts[0])
        } else {
            hidden_counts
                .into_iter()
                .zip(decision_counts)
                .map(|(hidden, decision)| DecisionMatrix::estimate_alloc(hidden, decision))
                .sum()
        }
    }

    pub fn estimate_weight_storage(
        is_symmetrical: bool,
        hidden_counts: Pair<usize>,
        decision_counts: Pair<usize>,
    ) -> usize {
        if is_symmetrical {
            assert!(are_equal(decision_counts));
            assert!(are_equal(hidden_counts));

            DecisionMatrix::estimate_weight_storage(hidden_counts[0], decision_counts[0])
        } else {
            hidden_counts
                .into_iter()
                .zip(decision_counts)
                .map(|(hidden, decision)| DecisionMatrix::estimate_weight_storage(hidden, decision))
                .sum()
        }
    }

    /// Compute the number of choices each player has.
    pub fn decision_counts(&self) -> Pair<usize> {
        match self {
            Self::Symmetrical(matrix) => [matrix.len(); 2],
            Self::Asymmetrical(matrices) => matrices.each_ref().map(|m| m.len()),
        }
    }

    /// Gets the node of a certain player at a certain index.
    ///
    /// Conceptually, this is like calling `.get_node_mut` on the individual
    /// matrices (although the matrices might not be "individual" if the game
    /// state is symmetric).
    pub fn get_nodes_mut(
        &mut self,
        [li, ri]: Pair<HiddenIndex>,
    ) -> Pair<Option<&mut DecisionVector<'a>>> {
        match self {
            Self::Asymmetrical([left, right]) => [left.get_node_mut(li), right.get_node_mut(ri)],
            Self::Symmetrical(matrix) => match matrix {
                DecisionMatrix::Trivial => [None, None],
                DecisionMatrix::Expanded(vec) => vec.get_many_mut([li.0, ri.0]).unwrap().map(Some),
            },
        }
    }
}
// }}}
// {{{ Explored scope
/// An explored scope is a scope where all the game rules have
/// been unrolled and all the game states have been created.
pub struct ExploredScope<'a> {
    /// Describes what kind of scopes we are in.
    #[cfg(debug_assertions)]
    pub summary: KnownStateSummary,

    /// Describes what kind of scopes we are in.
    #[cfg(debug_assertions)]
    pub context: Option<BattleContext>,

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
