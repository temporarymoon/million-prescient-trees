#![allow(dead_code)]

use bumpalo::Bump;
use rand::Rng;

use crate::helpers::{normalize_vec, roulette};

/// Utility is the quantity players attempt to maximize.
pub type Utility = f32;

/// Node representing all the data about a decision
/// required for training.
#[derive(Debug)]
pub struct DecisionNode<'a> {
    /// The total amount of regret we've accumulated so far.
    ///
    /// Essentially, each time we chose one of the decisions
    /// this node has to offer, we check how much better our
    /// position would have been if we had made a different decision.
    /// That value quantifies the amount of "regret" we have.
    pub regret_sum: Vec<f32, &'a Bump>,

    /// Probability distribution for the strategy the current
    /// agent has for this choice.
    pub strategy: Vec<f32, &'a Bump>,

    /// Sum of all strategies so far.
    ///
    /// It's important to note this is the value which converges towards
    /// optimal play, not the current strategy in the property above!
    pub strategy_sum: Vec<f32, &'a Bump>,

    /// True only if the last decision was taken by a different player.
    pub players_swapped: bool,

    /// Link to the next state in the game.
    pub next: Vec<Node<'a>, &'a Bump>,

    /// If true, the info won't be revealed to the player taking the next decision.
    ///
    /// For example, if you are the first player to put down a creature and an edict,
    /// you are not going to tell your opponent what cards you've played just yet.
    ///
    /// That's not the case for all decisions though! For example, choosing one of
    /// the two creatures played using the seer effect will immediately reveal your
    /// choice to the opponent.
    pub hidden_info: bool,

    /// Partial injective mapping from overseer
    /// candidates to their index in the next vector.
    ///
    /// Essentially, the overseer is unknown to the players,
    /// but they can signal out certain cards (eg: things already
    /// played, cards they have in hand, etc).
    ///
    /// Such options should not take up space in the next vector,
    /// therefore this mapping will mark them as "None".
    ///
    /// The mapping is injective because two candiates cannot lead
    /// to the same infoset (eg: if you told your opponent about
    /// two candiates you consider might be the overseer, the opponent
    /// will always be able to signal out one of them (assuming you
    /// are being honest and don't include options you could signal
    /// out yourself))
    pub overseer_indices: [Option<u8>; 11],

    /// The number of Some values inside the above property.
    pub overseer_candidate_count: u8,
}

/// A node represents a game state. A game state either requires
/// someone to make a decision, or is complete (the game is over).
#[derive(Debug)]
pub enum Node<'a> {
    Decision(DecisionNode<'a>),
    Complete(Utility),
    Empty,
}

impl<'a> Default for Node<'a> {
    fn default() -> Self {
        Node::Empty
    }
}

impl<'a> Node<'a> {
    /// Returns true if the last actions performed to achive this state
    /// changed focus from a player to another.
    pub fn players_swapped(&self) -> bool {
        match self {
            Node::Decision(decision) => decision.players_swapped,
            _ => false,
        }
    }
}

impl<'a> DecisionNode<'a> {
    pub fn new(
        size: usize,
        allocator: &'a Bump,
        next: Vec<Node<'a>, &'a Bump>,
        hidden_info: bool,
        players_swapped: bool,
        overseer_indices: [Option<u8>; 11],
    ) -> Self {
        let mut regret_sum = Vec::with_capacity_in(size, allocator);
        let mut strategy = Vec::with_capacity_in(size, allocator);
        let mut strategy_sum = Vec::with_capacity_in(size, allocator);

        regret_sum.resize(size, 0.0);
        strategy.resize(size, 0.0);
        strategy_sum.resize(size, 0.0);

        Self {
            regret_sum,
            strategy,
            strategy_sum,
            next,
            hidden_info,
            overseer_indices,
            players_swapped,
            overseer_candidate_count: overseer_indices.iter().filter(|o| o.is_some()).count() as u8,
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
    /// * `realization_weight` - The probability of this state to be reached in a game.
    pub fn update_strategy(&mut self, reallization_weight: f32) {
        for i in 0..self.len() {
            // We cannot have negative probabilities.
            self.strategy[i] = f32::max(self.regret_sum[i], 0.0);

            // TODO: I don't remember why this was here.
            if self.strategy[i] < 0.001 {
                self.strategy[i] = 0.0;
            }
        }

        normalize_vec(&mut self.strategy);

        for i in 0..self.len() {
            self.strategy_sum[i] += reallization_weight * self.strategy[i];
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
    pub fn random_action<R>(&self, rng: &mut R) -> usize
    where
        R: Rng,
    {
        let average = self.get_average_strategy();

        roulette(&average, rng)
    }
}
