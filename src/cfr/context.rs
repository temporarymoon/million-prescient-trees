#![allow(dead_code)]

use bumpalo::Bump;
use rand::{Rng, RngCore};

use crate::{
    game::types::Creature,
    helpers::{ranged::MixRanged, swap::conditional_swap},
};

use super::decision::{Node, Utility};

/// Context training takes place in
pub struct Context<'a, R: RngCore + Rng> {
    pub root: Node<'a>,
    pub rng: R,

    allocator: &'a Bump,
}

impl<'a, R: RngCore> Context<'a, R> {
    pub fn new(rng: R, allocator: &'a Bump) -> Self {
        Context {
            rng,
            root: Node::Empty,
            allocator,
        }
    }

    /// Generates the game tree based on a starting hand.
    pub fn generate_tree(&mut self) {
        todo!();
    }

    /// Run the cfr algorithm a given number of times.
    /// Randomizes the overseer each time.
    ///
    /// TODO: consider training for more than one starting hand.
    pub fn train(&mut self, iterations: usize) {
        // TODO: progress reporting
        let mut root = std::mem::take(&mut self.root);
        for _ in 0..iterations {
            let overseer = Creature::CREATURES[self.rng.gen_range(0..11)];

            self.cfr(&mut root, (1.0, 1.0), overseer, None);
        }
        self.root = root
    }

    fn cfr(
        &mut self,
        node: &mut Node<'a>,
        realization_weights: (f32, f32),
        overseer: Creature,
        hidden_info: Option<usize>,
    ) -> Utility {
        match node {
            Node::Empty => panic!("Cannot apply cfr to empty node!"),
            Node::Complete(utility) => *utility,
            Node::Decision(decision) => {
                let mut total_utility = 0.0;
                let size = decision.len();

                // {{{ Boilerplate for using props inside the .iter
                let next = &mut decision.next;
                let decision_hidden_info = decision.hidden_info;
                let decision_overseer_candidate_count = decision.overseer_candidate_count as usize;
                let decision_regret_sum = &mut decision.regret_sum;
                // }}}
                // {{{ Compute overseer index
                let overseer_index =
                    decision.overseer_indices[overseer as usize].unwrap_or_else(|| {
                        panic!(
                            "Degenerate game tree — overseer identity {} thought to be impossible.",
                            overseer
                        )
                    }) as usize;
                // }}}

                decision
                    .strategy
                    // The idea was that I could later change this to .par_iter,
                    // but with how bad that works together with bumpalo, I doubt that's
                    // ever going to happen...
                    .iter()
                    .enumerate()
                    .for_each(|(index, probability)| {
                        // {{{ Prepare next hidden info
                        let next_hidden_info = if decision_hidden_info {
                            Some(index)
                        } else {
                            None
                        };
                        // }}}
                        // {{{ Prepare branch
                        let branch_index = hidden_info
                            .unwrap_or(0)
                            .mix_ranged(overseer_index, decision_overseer_candidate_count);

                        let branch_index = if decision_hidden_info {
                            branch_index
                        } else {
                            branch_index.mix_ranged(index, size)
                        };

                        let branch = &mut next[branch_index];
                        // }}}
                        // {{{ Compute utility
                        let mut child = std::mem::take(branch);
                        let updated_weights = conditional_swap(
                            (realization_weights.0 * probability, realization_weights.1),
                            child.players_swapped(),
                        );

                        let mut utility =
                            self.cfr(&mut child, updated_weights, overseer, next_hidden_info);

                        if child.players_swapped() {
                            utility = -utility
                        }

                        *branch = child;
                        // }}}

                        total_utility += utility;
                        decision_regret_sum[index] += realization_weights.1 * utility;
                    });

                // Regret is conceptually defined as:
                //   regret := individual_utility[index] - total_utilit;
                //
                // The regret is added to the regret sum in two steps.
                // The first step is performed inside the loop, and the second one is
                // performed at the end.
                for index in 0..decision.len() {
                    decision.regret_sum[index] -= realization_weights.1 * total_utility;
                }

                // The strategy is a function of the regret.
                // This means we have to update the strategy when the regret changes.
                decision.update_strategy(realization_weights.0);

                total_utility
            }
        }
    }
}
