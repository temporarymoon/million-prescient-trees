#![allow(dead_code)]
#![allow(unreachable_code)]

use indicatif::{ProgressBar, ProgressStyle};
use rand::{Rng, RngCore};
use rustc_hash::FxHashMap;
use smallvec::{smallvec, SmallVec};

use crate::{
    echo::{CompleteGameState, GameState, InfoSet, Phase, PhaseTransition, Score},
    helpers::{conditional_swap, normalize_vec, roulette, VEC_SIZE},
    montecarlo::estimate_utility,
};

#[derive(Debug)]
pub struct Node {
    regret_sum: SmallVec<[f32; VEC_SIZE]>,
    strategy: SmallVec<[f32; VEC_SIZE]>,
    strategy_sum: SmallVec<[f32; VEC_SIZE]>,
    actions: SmallVec<[PhaseTransition; VEC_SIZE]>,
    pruned: Option<f32>,
    estimated_utilities: [Option<f32>; 11],
    cummulative_realization_weights: ([f32; 11], [f32; 11]),
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
            estimated_utilities: [Option::None; 11],
            cummulative_realization_weights: ([0.0; 11], [0.0; 11]),
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

    pub fn print_average_strategy(&self) {
        let average = self.get_average_strategy();

        for i in 0..self.size() {
            if average[i].abs() < 0.01 {
                continue;
            }

            println!("Move: {}, probabilty: {}%", self.actions[i], (100.0 *average[i]).round());
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
pub enum BoardEvaluation {
    // Just traverse the full tree bro
    FullTreeTraversal,
    // After a certain depth, simulate random games
    MonteCarlo { iterations: usize, max_depth: usize },
}

#[derive(Debug)]
pub struct TrainingOptions {
    // None means pruning is disabled
    pub pruning_threshold: Option<f32>,
    pub board_evaluation: BoardEvaluation,
    pub starting_infoset: InfoSet,
    // pub overseer_weights: [f32; 11]
}

#[derive(Debug)]
pub struct Context<R: RngCore + Rng> {
    pub training_options: TrainingOptions,
    pub nodes: FxHashMap<InfoSet, Node>,
    progress_bar: ProgressBar,
    pruned_nodes: FxHashMap<GameState, f32>,
    terminal_histories: usize,
    pruned_count: usize,
    pub rng: R,
}

impl<R: RngCore> Context<R> {
    pub fn new(training_options: TrainingOptions, rng: R, progress_bar: ProgressBar) -> Self {
        Context {
            training_options,
            rng,
            progress_bar,
            nodes: FxHashMap::default(),
            pruned_nodes: FxHashMap::default(),
            terminal_histories: 0,
            pruned_count: 0,
        }
    }

    pub fn take_node(&mut self, info_set: &InfoSet) -> Node {
        self.nodes
            .remove(info_set)
            .unwrap_or_else(|| Node::new(info_set.available_actions()))
    }

    pub fn make_choice(&mut self, info_set: &InfoSet) -> Option<PhaseTransition> {
        match self.nodes.get(info_set) {
            None => None,
            Some(node) => {
                let average = node.get_average_strategy();
                node.print_average_strategy();

                let index = roulette(&average, &mut self.rng);
                Some(node.actions[index])
            }
        }
    }
}

fn is_essentially_zero(f: f32) -> bool {
    f.abs() < 0.000000003
}

fn cfr<R: RngCore>(
    context: &mut Context<R>,
    state: &CompleteGameState,
    realization_weights: (f32, f32),
    depth: usize,
) -> f32 {
    if is_essentially_zero(realization_weights.0) && is_essentially_zero(realization_weights.1) {
        return 0.0;
    }

    match state {
        CompleteGameState::Finished(Score(score)) => {
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
            let info_set = unfinished_state.conceal();
            let mut node = context.take_node(&info_set);

            let should_estimate = match context.training_options.board_evaluation {
                BoardEvaluation::MonteCarlo {
                    iterations,
                    max_depth,
                } if depth > max_depth => Some(iterations),
                _ => None,
            };

            let overseer = unfinished_state.overseer as usize;
            node.cummulative_realization_weights.0[overseer] += realization_weights.0;
            node.cummulative_realization_weights.1[overseer] += realization_weights.1;

            match should_estimate {
                Some(iterations) => {
                    let utility = if let Some(estimated) = node.estimated_utilities[overseer] {
                        estimated
                    } else {
                        context
                            .progress_bar
                            .set_message(format!("{:?}", context.terminal_histories));

                        let utility =
                            estimate_utility(unfinished_state, &mut context.rng, iterations);

                        node.estimated_utilities[overseer] = Some(utility);

                        utility
                    };

                    context.terminal_histories += 1;
                    context.nodes.insert(info_set, node);

                    return utility;
                }
                _ => (),
            }

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
                        let (new_state, flipped): (CompleteGameState, bool) =
                            unfinished_state.apply_transition(*action).unwrap();

                        let updated_weights = conditional_swap(
                            (
                                realization_weights.0 * strategy[index],
                                realization_weights.1,
                            ),
                            flipped,
                        );

                        let depth = match &new_state {
                            CompleteGameState::Unfinished(state) if state.phase == Phase::Main1 => {
                                depth + 1
                            }
                            _ => depth,
                        };

                        let utility = cfr(context, &new_state, updated_weights, depth);
                        let utility = if flipped { -utility } else { utility };

                        individual_utility[index] = utility;
                        total_utility += strategy[index] * utility;
                    }

                    for index in 0..individual_utility.len() {
                        let regret = individual_utility[index] - total_utility;
                        node.regret_sum[index] += realization_weights.1 * regret;
                    }

                    if let Some(pruning_threshold) = context.training_options.pruning_threshold {
                        let mut should_prune = true;
                        let size = individual_utility.len();
                        for i in 0..size {
                            if i == size - 1 {
                                if 1.0 - individual_utility[i].abs() > pruning_threshold {
                                    should_prune = false;
                                    break;
                                }
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

pub fn train<R: RngCore>(
    training_options: TrainingOptions,
    iterations: usize,
    rng: R,
) -> (f32, Context<R>) {
    let progress_bar = ProgressBar::new(iterations as u64);
    let mut context = Context::new(training_options, rng, progress_bar);
    let mut total = 0.0;

    context.progress_bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap(),
    );
    context.progress_bar.tick();

    for _ in 0..iterations {
        let initial_state = GameState::from_info_set(&context.training_options.starting_infoset, &mut context.rng);
        let utility = cfr(&mut context, &CompleteGameState::Unfinished(initial_state), (1.0, 1.0), 0);
        total += utility;
        context.progress_bar.inc(1);
        // println!("Utility {}", utility);
    }

    context.progress_bar.finish();

    println!("Initial state {:?}", GameState::new());
    println!("Node count: {}", context.nodes.len());
    println!("Terminal histories: {}", context.terminal_histories);
    println!("Pruned hashmap size: {}", context.pruned_nodes.len());
    println!("Pruned count: {}", context.pruned_count);

    (total / iterations as f32, context)
}

pub fn utility_to_percentage(utility: f32) -> f32 {
    (utility + 1.0) * 50.0
}
