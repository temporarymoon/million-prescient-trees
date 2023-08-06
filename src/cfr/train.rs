use rand::distributions::Uniform;
use rand::prelude::Distribution;
use rand::Rng;

use super::decision::{DecisionVector, Probability, Scope, Utility};
use super::hidden_index::{self, HiddenIndex, HiddenState};
use super::phase::{MainPhase, Phase};
use crate::cfr::decision_index::DecisionIndex;
use crate::game::known_state_summary::KnownStateSummary;
use crate::game::types::Player;
use crate::helpers::pair::Pair;
use std::{debug_assert_eq, println, unreachable};

// TODO: implement resetting of weights halfway through training.
pub struct TrainingContext {
    enable_pruning: bool,
}

impl TrainingContext {
    pub fn new(enable_pruning: bool) -> Self {
        Self { enable_pruning }
    }

    pub fn cfr(&self, scope: &mut Scope, state: KnownStateSummary, iterations: usize) {
        let probabilities: Pair<Probability> = [1.0; 2];
        let phase = MainPhase::new();
        for i in 0..iterations {
            println!("Iteration {i}");

            for hidden in phase.valid_hidden_states(state) {
                self.train_phase(scope, phase, state, hidden, probabilities);
            }
        }
    }

    /// Chance-sampling counterfactual regret minimization.
    ///
    /// Similar to `cfr`, but focuses on a single (random) initial set of hidden indices.
    pub fn cs_cfr<R: Rng>(
        &self,
        rng: &mut R,
        scope: &mut Scope,
        state: KnownStateSummary,
        iterations: usize,
    ) {
        let probabilities: Pair<Probability> = [1.0; 2];
        let phase = MainPhase::new();

        // TODO: consider not allocating?
        let hidden_vec: Vec<_> = phase.valid_hidden_states(state).collect();
        let distribution = Uniform::new(0, hidden_vec.len());

        for i in 0..iterations {
            if i % 10 == 0 {
                println!("Iteration {i}");
            }

            let index = distribution.sample(rng);
            self.train_phase(scope, phase, state, hidden_vec[index], probabilities);
        }
    }

    fn train_phase<P: Phase>(
        &self,
        scope: &mut Scope,
        phase: P,
        state: KnownStateSummary,
        hidden: Pair<hidden_index::EncodingInfo>,
        probabilities: Pair<Probability>,
    ) -> Option<Utility> {
        match scope {
            Scope::Completed(score) => Some(score.to_utility()),
            Scope::Unexplored(_) => unreachable!("Oops, cannot handle unexplored scopes"),
            Scope::Explored(scope) => {
                #[cfg(debug_assertions)]
                debug_assert_eq!(
                    scope.summary, state,
                    "Something went wrong with simulating {:?}",
                    scope.context
                );

                // {{{ Prepare data
                let counts = scope.matrices.decision_counts();
                let hidden_states = hidden.map(HiddenState::from_encoding_info);
                let indices = Player::PLAYERS
                    .map(|player| HiddenIndex::encode(&state, player, player.select(hidden)));

                let mut nodes = scope.matrices.get_nodes_mut(indices);
                let mut total_utility: Utility = 0.0;
                // }}}
                // {{{ Compute strategies
                for (i, node) in nodes.iter_mut().enumerate() {
                    if let Some(node) = node {
                        node.recompute_regret_magnitude();
                        node.update_strategy_sum(probabilities[i]);
                    }
                }
                // }}}

                if self.enable_pruning && Self::is_almost_zero(probabilities[1]) {
                    return Some(0.0);
                };

                // {{{ First player
                for index in 0..(counts[0]) {
                    let my_decision = DecisionIndex(index);
                    let my_probability = DecisionVector::try_strategy(nodes[0].as_deref(), index);

                    // {{{ Second player
                    let future_utility = {
                        if self.enable_pruning && Self::is_almost_zero(probabilities[0]) {
                            0.0
                        } else {
                            let mut total_utility: Utility = 0.0;

                            for index in 0..(counts[1]) {
                                let your_decision = DecisionIndex(index);
                                let your_probability =
                                    DecisionVector::try_strategy(nodes[1].as_deref(), index);

                                // {{{ Recursive call
                                let new_probabilities = [
                                    probabilities[0] * my_probability,
                                    probabilities[1] * your_probability,
                                ];

                                let decisions = [my_decision, your_decision];

                                let (new_state, new_hidden, reveal_index) = phase
                                    .advance_hidden_indices(state, hidden_states, decisions)
                                    .unwrap();

                                let new_scope = &mut scope.next[reveal_index.0];
                                let next_phase = phase.advance_phase(&state, reveal_index)?;

                                let future_utility = -self.train_phase::<P::Next>(
                                    new_scope,
                                    next_phase,
                                    new_state,
                                    new_hidden,
                                    new_probabilities,
                                )?;
                                // }}}

                                total_utility += your_probability * future_utility;

                                // {{{ Add utility to your regret
                                if let Some(node) = &mut nodes[1] {
                                    node.accumulate_regret(
                                        index,
                                        my_probability * probabilities[0] * future_utility,
                                    );
                                }
                                // }}}
                            }

                            -total_utility
                        }
                    };
                    // }}}

                    total_utility += my_probability * future_utility;

                    // {{{ Add utility to my regret
                    if let Some(node) = &mut nodes[0] {
                        node.accumulate_regret(index, probabilities[1] * future_utility);
                    }
                    // }}}
                }
                // }}}
                // {{{ Subtract total utility from regrets
                if let Some(node) = &mut nodes[0] {
                    for index in 0..counts[0] {
                        node.accumulate_regret(index, -probabilities[1] * total_utility);
                    }
                }

                if let Some(node) = &mut nodes[1] {
                    for index in 0..counts[1] {
                        node.accumulate_regret(index, -probabilities[0] * total_utility);
                    }
                }
                // }}}

                Some(total_utility)
            }
        }
    }

    /// With the goal of trying to avoid floating point arithmetic weirdness,
    /// we declare things to be equal to 0 if they are "close enough"
    #[inline(always)]
    fn is_almost_zero(num: Probability) -> bool {
        num.abs() < 0.00000001
    }
}
