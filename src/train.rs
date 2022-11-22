#![allow(dead_code)]
#![allow(unreachable_code)]

use rustc_hash::FxHashMap;

use crate::{
    echo::{CompleteGameState, GameState, InfoSet, PhaseTransition, Score},
    helpers::{conditional_swap, normalize_vec, zeroes},
};

#[derive(Debug)]
pub struct Node {
    regret_sum: Vec<f32>,
    strategy: Vec<f32>,
    strategy_sum: Vec<f32>,
    actions: Vec<PhaseTransition>,
    pruned: Option<f32>,
}

impl Node {
    pub fn new(actions: Vec<PhaseTransition>) -> Self {
        let size = actions.len();
        // println!("{}", size);
        Self {
            actions,
            regret_sum: zeroes(size),
            strategy: zeroes(size),
            strategy_sum: zeroes(size),
            pruned: None,
        }
    }

    fn size(&self) -> usize {
        self.actions.len()
    }

    pub fn update_strategy(&mut self, reallization_weight: f32) {
        for i in 0..self.size() {
            self.strategy[i] = f32::max(self.regret_sum[i], 0.0);
        }

        normalize_vec(&mut self.strategy);

        for i in 0..self.size() {
            self.strategy_sum[i] += reallization_weight * self.strategy[i];
        }
    }

    pub fn get_average_strategy(&self) -> Vec<f32> {
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
}

#[derive(Debug)]
pub struct Context {
    nodes: FxHashMap<InfoSet, Node>,
    terminal_nodes: usize,
    pruned_nodes: usize,
    winnable_states: usize,
    losable_states: usize,
}

impl Context {
    pub fn new() -> Self {
        Context {
            nodes: FxHashMap::default(),
            terminal_nodes: 0,
            pruned_nodes: 0,
            winnable_states: 0,
            losable_states: 0,
        }
    }

    pub fn take_node(&mut self, info_set: &InfoSet) -> Node {
        self.nodes
            .remove(info_set)
            .unwrap_or_else(|| Node::new(info_set.available_actions()))
    }
}

fn cfr(context: &mut Context, state: &CompleteGameState, realization_weights: (f32, f32)) -> f32 {
    let enable_pruning = true;
    match state {
        CompleteGameState::Finished(Score(score)) => {
            context.terminal_nodes += 1;
            // println!("{}", context.terminal_nodes);
            // println!("Finished a game!");
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

            match node.pruned {
                Some(utility) => {
                    context.nodes.insert(info_set, node);
                    utility
                }
                None => {
                    node.update_strategy(realization_weights.0);

                    let strategy = &node.strategy;

                    let mut total_utility = 0.0;
                    let mut individual_utility = zeroes(node.size());

                    for (index, action) in node.actions.iter().enumerate() {
                        // println!("State: {:?}", unfinished_state);
                        // println!("Action: {:?}", action);
                        let (new_state, flipped): (CompleteGameState, bool) =
                            unfinished_state.apply_transition(*action).unwrap();

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
                            node.pruned = Some(total_utility);
                            context.pruned_nodes += 1;
                            if (1.0 - total_utility).abs() < pruning_threshold {
                                context.winnable_states += 1;
                            } else {
                                context.losable_states += 1;
                            }
                        }
                    }

                    context.nodes.insert(info_set, node);

                    total_utility
                }
            }
        }
    }
}

pub fn train(iterations: usize) -> f32 {
    let new_game = GameState::new().flip_to(crate::echo::Phase::Main1);
    let new_game2 = new_game.clone();
    let new_game3 = new_game.clone();
    // let initial_state = CompleteGameState::Unfinished(new_game);
    let initial_state = new_game;
    let mut context = Context::new();
    let mut total = 0.0;

    for i in 0..iterations {
        println!("Iteration {}", i);
        let utility = cfr(&mut context, &initial_state, (1.0, 1.0));
        total += utility;
        println!("Utility {}", utility);
    }

    println!("Initial state {:?}", GameState::new());
    println!("Node count: {}", context.nodes.len());
    println!("Terminal nodes: {}", context.terminal_nodes);
    println!("Pruned nodes: {}", context.pruned_nodes);
    println!("Winnable nodes: {}", context.winnable_states);
    println!("Losable nodes: {}", context.losable_states);

    println!("First node:");
    let first_node = &context.nodes[&new_game2.to_game_state().unwrap().conceal()];
    let best = first_node.best_action();
    first_node.print_actions();
    println!("Second node:");
    let second_node = &context.nodes[&new_game3
        .to_game_state()
        .unwrap()
        .apply_transition(best)
        .unwrap()
        .0
        .to_game_state()
        .unwrap()
        .conceal()];
    second_node.print_actions();
    // println!("Best action {:?}", first_node.best_action());

    (total / iterations as f32) + 1.0 * 50.0
}
