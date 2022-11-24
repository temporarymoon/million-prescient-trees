use crate::{
    echo::{CompleteGameState, GameState},
    train::Context,
};
use rand::Rng;

fn randomly_fight_ai<R>(context: &Context, initial_state: GameState, rng: &mut R) -> i8
where
    R: Rng,
{
    let mut game_state = initial_state;
    let mut score_multiplier = 1;
    let mut smart_turn = true;
    loop {
        let info_set = game_state.conceal();

        let action = if smart_turn {
            match context.nodes.get(&info_set) {
                None => {
                    let actions = info_set.available_actions();
                    actions[rng.gen_range(0..actions.len())]
                }
                Some(node) => {
                    node.random_action(rng)
                }
            }
        } else {
            let actions = info_set.available_actions();
            actions[rng.gen_range(0..actions.len())]
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

fn simulate_random_game<R>(initial_state: GameState, rng: &mut R) -> f32
where
    R: Rng,
{
    let mut game_state = initial_state;
    let mut score_multiplier = 1;
    loop {
        let info_set = game_state.conceal();
        let actions = info_set.available_actions();

        let index = rng.gen_range(0..actions.len());
        let action = actions[index];

        let (new_game_state, flipped) = game_state.apply_transition(action).unwrap();

        if flipped {
            score_multiplier = -score_multiplier;
        }

        match new_game_state {
            CompleteGameState::Finished(score) => {
                let result = score.0 * score_multiplier;
                // if result < 0 {
                //     return -1;
                // } else if result > 0 {
                //     return 1;
                // } else {
                //     return 0;
                // }
                return result as f32 / 6.0;
            }
            CompleteGameState::Unfinished(state) => {
                game_state = state;
            }
        }
    }
}

pub fn estimate_utility<R>(initial_state: &GameState, rng: &mut R, iterations: usize) -> f32
where
    R: Rng,
{
    let mut total: f32 = 0.0;
    for _ in 0..iterations {
        total += simulate_random_game(initial_state.clone(), rng);
    }

    total / iterations as f32
}

// Fights a trained AI against random play
pub fn check_against_randomness<R>(context: &Context,  rng: &mut R, iterations: usize) -> f32
where
    R: Rng,
{
    let mut total: i32 = 0;
    for _ in 0..iterations {
        total += randomly_fight_ai(context, GameState::new(), rng) as i32;
    }

    total as f32 / iterations as f32
}
