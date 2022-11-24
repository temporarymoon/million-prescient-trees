use rand::thread_rng;

use crate::{
    echo::GameState,
    montecarlo::{check_against_randomness, estimate_utility},
    train::{train, utility_to_percentage, BoardEvaluation, TrainingOptions},
};
use std::time::Instant;

mod echo;
mod helpers;
mod montecarlo;
mod train;

fn main() {
    let rng = &mut thread_rng();
    let start = Instant::now();
    let utility = estimate_utility(&GameState::new(), rng, 100000);
    let duration = start.elapsed();

    println!("Time elapsed: {:?}", duration);
    println!(
        "You have a {}% chance of winning by playing randomly",
        utility_to_percentage(utility)
    );

    let start = Instant::now();
    let options = TrainingOptions {
        pruning_threshold: Some(0.01),
        board_evaluation: BoardEvaluation::MonteCarlo {
            iterations: 100,
            max_depth: 1,
        },
    };
    let (utility, mut context) = train(options, 100, rng);
    let duration = start.elapsed();

    println!("Time elapsed: {:?}", duration);
    println!(
        "You have a {}% chance of winning against an optimal player",
        &utility_to_percentage(utility)
    );

    let start = Instant::now();
    let utility = check_against_randomness(&mut context, 100000);
    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);
    println!(
        "You have a {}% chance of winning against a random player",
        utility_to_percentage(utility)
    );
}
