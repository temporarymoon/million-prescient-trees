use crate::{
    echo::{CompleteGameState, GameState},
    train::Context,
};
use rand::{Rng, RngCore};

fn randomly_fight_ai<R: RngCore>(context: &mut Context<R>, initial_state: GameState) -> i8 {
    let mut game_state = initial_state;
    let mut score_multiplier = 1;
    let mut smart_turn = true;
    loop {
        let info_set = game_state.conceal();

        let action = if smart_turn {
            match context.nodes.get(&info_set) {
                None => {
                    let actions = info_set.available_actions();
                    actions[context.rng.gen_range(0..actions.len())]
                }
                Some(node) => node.random_action(&mut context.rng),
            }
        } else {
            let actions = info_set.available_actions();
            actions[context.rng.gen_range(0..actions.len())]
        };

        let (new_game_state, flipped) = game_state.apply_transition(action).unwrap();

        if flipped {
            score_multiplier = -score_multiplier;
            smart_turn = !smart_turn;
        }

        match new_game_state {
            CompleteGameState::Finished(score) => {
                let result = score.0 * score_multiplier;
                if result < 0 {
                    return -1;
                } else if result > 0 {
                    return 1;
                } else {
                    return 0;
                }
            }
            CompleteGameState::Unfinished(state) => {
                game_state = state;
            }
        }
    }
}

// I had to define this as a recursive function to
// please the borrow checker.
//
// For the sake of tail recursiveness I defined a separate function with
// extra arguments which gets called by simulate_random_game
fn simulate_random_game_impl<R>(
    game_state: &GameState,
    rng: &mut R,
    score_multiplier: i8,
    score_bounds: (f32, f32),
) -> f32
where
    R: Rng,
{
    let info_set = game_state.conceal();
    let actions = info_set.available_actions();

    let index = rng.gen_range(0..actions.len());
    let action = actions[index];

    let (new_game_state, flipped) = game_state.apply_transition(action).unwrap();

    let score_multiplier = if flipped {
        -score_multiplier
    } else {
        score_multiplier
    };

    match new_game_state {
        CompleteGameState::Finished(score) => {
            // if score_bounds.0 == score_bounds.1 {
            //     if result < 0 {
            //         return -1;
            //     } else if result > 0 {
            //         return 1;
            //     } else {
            //         return 0;
            //     }
            // }

            let mut utility =
                2.0 * (score.0 as f32 - score_bounds.0) / (score_bounds.1 - score_bounds.0) - 1.0;
            if utility < -1.0 {
                utility = -1.0;
            } else if utility > 1.0 {
                utility = 1.0;
            }

            utility * (score_multiplier as f32)
        }
        CompleteGameState::Unfinished(state) => {
            simulate_random_game_impl(&state, rng, score_multiplier, score_bounds)
        }
    }
}

fn simulate_random_game<R>(initial_state: &GameState, rng: &mut R) -> f32
where
    R: Rng,
{
    let min_score = -(initial_state.max_score() as f32) + initial_state.score.0 as f32;
    let max_score = initial_state.max_score() as f32 + initial_state.score.0 as f32;
    simulate_random_game_impl(initial_state, rng, 1, (min_score, max_score))
}

pub fn estimate_utility<R>(initial_state: &GameState, rng: &mut R, iterations: usize) -> f32
where
    R: Rng,
{
    // let progress_bar = ProgressBar::new(iterations as u64);
    let mut total: f32 = 0.0;
    for _ in 0..iterations {
        total += simulate_random_game(initial_state, rng);
        // progress_bar.inc(1);
    }
    // progress_bar.finish();

    total / iterations as f32
}

// Fights a trained AI against random play
pub fn check_against_randomness<R: RngCore>(context: &mut Context<R>, iterations: usize) -> f32 {
    let mut total: i32 = 0;
    for _ in 0..iterations {
        total += randomly_fight_ai(context, GameState::new()) as i32;
    }

    total as f32 / iterations as f32
}
