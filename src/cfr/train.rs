use std::{unreachable, println};

use crate::game::types::Score;

use super::decision::{Scope, Utility};

pub struct TrainingContext {}

impl TrainingContext {
    pub fn propagate_probabilities(&self, scope: &mut Scope) -> Utility {
        match scope {
            Scope::Completed(Score(score)) => {
                if *score > 0 {
                    1.0
                } else if *score < 0 {
                    -1.0
                } else {
                    0.0
                }
            }
            Scope::Unexplored(_) => unreachable!("Oops, cannot handle unexplored scopes"),
            Scope::Explored(_) => {
                println!("Explored");
                0.0
            }
        }
    }
}
