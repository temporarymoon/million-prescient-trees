#![allow(dead_code)]
#![allow(unreachable_code)]

use std::collections::HashMap;

use crate::{echo::{CompleteGameState, InfoSet, PhaseTransition, Score}, helpers::{conditional_swap, zeroes}};

pub struct Node {
    regret_sum: Vec<f32>,
    strategy: Vec<f32>,
    strategy_sum: Vec<f32>,
    actions: Vec<PhaseTransition>,
}

fn normalize_vec(vec: &mut Vec<f32>) {
    let mut sum = 0.0;

    for value in &mut *vec {
        sum += *value;
    }

    for value in vec {
        if sum > 0.0 {
            *value /= sum;
        } else {
            *value = 1.0 / sum;
        }
    }
}

impl Node {
    pub fn new(actions: Vec<PhaseTransition>) -> Self {
        let size = actions.len();
        Self {
            actions,
            regret_sum: zeroes(size),
            strategy: zeroes(size),
            strategy_sum: zeroes(size),
        }
    }

    fn size(&self) -> usize {
        self.actions.len()
    }

    pub fn get_strategy(&mut self, reallization_weight: f32) -> &Vec<f32> {
        for i in 0..self.size() {
            self.strategy[i] = f32::max(self.regret_sum[i], 0.0);
        }

        normalize_vec(&mut self.strategy);

        for i in 0..self.size() {
            self.strategy_sum[i] += reallization_weight * self.strategy[i];
        }

        &self.strategy
    }

    pub fn get_average_strategy(&self) -> Vec<f32> {
        let mut average_stragegy = self.strategy_sum.clone();

        normalize_vec(&mut average_stragegy);

        average_stragegy
    }
}

pub struct Context {
    nodes: HashMap<InfoSet, Node>,
}

impl Context {
    pub fn get_node(&mut self, info_set: InfoSet) -> &mut Node {
        let node = Node::new(info_set.available_actions());
        self.nodes.entry(info_set).or_insert(node)
        // let info_set_ref = &info_set;
        // self.nodes
        //     .entry(info_set)
        //     .or_insert_with(|| Node::new(info_set_ref.available_actions()))
    }
}

fn cfr(context: &mut Context, state: CompleteGameState, realization_weights: (f32, f32)) -> f32 {
    match state {
        CompleteGameState::Finished(Score(score)) => {
            if score > 0 {
                1.0
            } else if score < 0 {
                -1.0
            } else {
                0.0
            }
        }
        CompleteGameState::Unfinished(unfinished_state) => {
            let info_set = unfinished_state.conceal();
            let node = context.get_node(info_set);
            let total_utility = 0.0;
            let individual_utility = zeroes(node.size());

            for (index, action) in node.actions.iter().enumerate() {
                let (new_state, flipped): (CompleteGameState, bool) = (unimplemented!(), true);

                let updated_weights = conditional_swap(
                    (
                        realization_weights.0 * node.strategy[index],
                        realization_weights.1,
                    ),
                    flipped,
                );

                let utility = cfr(context, new_state, updated_weights);
                let utility = if flipped { -utility } else { utility };

                individual_utility[index] = utility;
                total_utility += node.strategy[index] * utility;
            }

            for index in 0..node.size() {
                let regret = individual_utility[index] - total_utility;
                node.regret_sum[index] += realization_weights.0 * regret;
            }

            return 0.0;
        }
    }
}
