#![allow(dead_code)]
#![allow(unreachable_code)]

use std::collections::HashSet;

use rand::Rng;
use rustc_hash::FxHashMap;
use smallvec::{smallvec, SmallVec};

use crate::{
    echo::{CompleteGameState, GameState, InfoSet, PhaseTransition, Score},
    helpers::{conditional_swap, normalize_vec, roulette, VEC_SIZE},
};

#[derive(Debug)]
pub struct Node {
    regret_sum: SmallVec<[f32; VEC_SIZE]>,
    strategy: SmallVec<[f32; VEC_SIZE]>,
    strategy_sum: SmallVec<[f32; VEC_SIZE]>,
    actions: SmallVec<[PhaseTransition; VEC_SIZE]>,
    pruned: Option<f32>,
}

impl Node {
    pub fn new(actions: SmallVec<[PhaseTransition; VEC_SIZE]>) -> Self {
        let size = actions.len();
        // println!("{}", size);
        Self {
            actions,
            regret_sum: smallvec![0.0;size],
            strategy: smallvec![0.0;size],
            strategy_sum: smallvec![0.0;size],
            pruned: None,
        }
    }

    fn size(&self) -> usize {
        self.actions.len()
    }

    pub fn update_strategy(&mut self, reallization_weight: f32) {
        for i in 0..self.size() {
            self.strategy[i] = f32::max(self.regret_sum[i], 0.0);
            if self.strategy[i] < 0.01 {
                self.strategy[i] = 0.0;
            }
        }

        normalize_vec(&mut self.strategy);

        for i in 0..self.size() {
            self.strategy_sum[i] += reallization_weight * self.strategy[i];
        }
    }

    pub fn get_average_strategy(&self) -> SmallVec<[f32; VEC_SIZE]> {
        let mut average_strategy = self.strategy_sum.clone();

        normalize_vec(&mut average_strategy);

        average_strategy
    }

    pub fn best_action(&self) -> PhaseTransition {
        let index = self
            .get_average_strategy()
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        self.actions[index]
    }

    pub fn print_actions(&self) {
        let average = self.get_average_strategy();
        // let average = &self.strategy;
        for i in 0..self.size() {
            if average[i].abs() < 0.01 {
                continue;
            }

            println!("Move: {:?}, probabilty: {:?}", self.actions[i], average[i]);
        }
    }

    pub fn random_action<R>(&self, rng: &mut R) -> PhaseTransition
    where
        R: Rng,
    {
        let average = self.get_average_strategy();
        self.actions[roulette(&average, rng)]
    }
}

#[derive(Debug)]
pub struct Context {
    pub nodes: FxHashMap<InfoSet, Node>,
    pruned_nodes: FxHashMap<GameState, f32>,
    terminal_nodes: HashSet<InfoSet>,
    terminal_histories: usize,
    pruned_count: usize,
    cached_transitions: FxHashMap<(GameState, usize), (CompleteGameState, bool)>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            nodes: FxHashMap::default(),
            pruned_nodes: FxHashMap::default(),
            cached_transitions: FxHashMap::default(),
            terminal_nodes: HashSet::new(),
            terminal_histories: 0,
            pruned_count: 0,
        }
    }

    pub fn take_node(&mut self, info_set: &InfoSet) -> Node {
        self.nodes
            .remove(info_set)
            .unwrap_or_else(|| Node::new(info_set.available_actions()))
    }
}

fn is_essentially_zero(f: f32) -> bool {
    f.abs() < 0.000000003
}

fn cfr(context: &mut Context, state: &CompleteGameState, realization_weights: (f32, f32)) -> f32 {
    let enable_pruning = true;

    if is_essentially_zero(realization_weights.0) && is_essentially_zero(realization_weights.1) {
        return 0.0;
    }

    match state {
        CompleteGameState::Finished(Score(score)) => {
            // println!("{}", context.terminal_nodes);
            // println!("Finished a game!");
            context.terminal_histories += 1;
            if *score > 0 {
                1.0
            } else if *score < 0 {
                -1.0
            } else {
                0.0
            }
        }
        CompleteGameState::Unfinished(unfinished_state) => {
            let node_count = context.nodes.len();
            if node_count % 100000 == 0 {
                println!("{}", node_count);
            };
            let info_set = unfinished_state.conceal();

            let mut node = context.take_node(&info_set);

            match context.pruned_nodes.get(unfinished_state) {
                Some(utility) => {
                    context.nodes.insert(info_set, node);
                    *utility
                }
                None => {
                    node.update_strategy(realization_weights.0);

                    let strategy = &node.strategy;

                    let mut total_utility = 0.0;
                    let mut individual_utility: SmallVec<[f32; VEC_SIZE]> =
                        smallvec![0.0;node.size()];

                    for (index, action) in node.actions.iter().enumerate() {
                        // println!("State: {:?}", unfinished_state);
                        // println!("Action: {:?}", action);
                        // let (new_state, flipped): (CompleteGameState, bool) = context
                        //     .cached_transitions
                        //     .remove(&(unfinished_state.clone(), index))
                        //     .unwrap_or_else(|| unfinished_state.apply_transition(*action).unwrap());
                        let (new_state, flipped): (CompleteGameState, bool) =
                            unfinished_state.apply_transition(*action).unwrap();

                        // if new_state.is_finished() {
                        //     context.terminal_nodes.insert(info_set.clone());
                        // }

                        let updated_weights = conditional_swap(
                            (
                                realization_weights.0 * strategy[index],
                                realization_weights.1,
                            ),
                            flipped,
                        );

                        let utility = cfr(context, &new_state, updated_weights);
                        let utility = if flipped { -utility } else { utility };

                        individual_utility[index] = utility;
                        total_utility += strategy[index] * utility;

                        // context
                        //     .cached_transitions
                        //     .insert((unfinished_state.clone(), index), (new_state, flipped));
                    }

                    for index in 0..individual_utility.len() {
                        let regret = individual_utility[index] - total_utility;
                        node.regret_sum[index] += realization_weights.1 * regret;
                    }

                    if enable_pruning {
                        let mut should_prune = true;
                        let size = individual_utility.len();
                        let pruning_threshold = 0.0001;
                        for i in 0..size {
                            if i == size - 1 {
                                if 1.0 - individual_utility[i].abs() > pruning_threshold {
                                    should_prune = false;
                                    break;
                                }
                                // else {
                                //      println!("Prunning node with individual utilities {:?} and total utility {}", individual_utility, total_utility);
                                //      println!("{:?}", unfinished_state);
                                // }
                            } else if (individual_utility[i + 1] - individual_utility[i]).abs()
                                > pruning_threshold
                            {
                                should_prune = false;
                                break;
                            }
                        }

                        if should_prune {
                            context
                                .pruned_nodes
                                .insert(unfinished_state.clone(), total_utility);
                            context.pruned_count += 1;
                        }
                    }

                    context.nodes.insert(info_set, node);

                    total_utility
                }
            }
        }
    }
}

pub fn train(iterations: usize) -> (f32, Context) {
    // let new_game2 = new_game.clone();
    // let new_game3 = new_game.clone();
    // let initial_state = CompleteGameState::Unfinished(new_game);
    // let initial_state = new_game;
    let mut context = Context::new();
    let mut total = 0.0;

    for i in 0..iterations {
        let initial_state = GameState::new().switch_to(crate::echo::Phase::Main1);
        println!("Iteration {}", i);
        let utility = cfr(&mut context, &initial_state, (1.0, 1.0));
        total += utility;
        // println!("Utility {}", utility);
    }

    println!("Initial state {:?}", GameState::new());
    println!("Node count: {}", context.nodes.len());
    println!("Terminal nodes: {}", context.terminal_nodes.len());
    println!("Terminal histories: {}", context.terminal_histories);
    println!("Pruned hashmap size: {}", context.pruned_nodes.len());
    println!("Pruned count: {}", context.pruned_count);

    // println!("First node:");
    // let first_node = &context.nodes[&new_game2.to_game_state().unwrap().conceal()];
    // let best = first_node.best_action();
    // first_node.print_actions();
    // println!("Second node:");
    // let second_node = &context.nodes[&new_game3
    //     .to_game_state()
    //     .unwrap()
    //     .apply_transition(best)
    //     .unwrap()
    //     .0
    //     .to_game_state()
    //     .unwrap()
    //     .conceal()];
    // second_node.print_actions();
    // println!("Best action {:?}", first_node.best_action());

    (total / iterations as f32, context)
}

pub fn utility_to_percentage(utility: f32) -> f32 {
    (utility + 1.0) * 50.0
}
