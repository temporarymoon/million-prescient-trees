use bumpalo::Bump;

use super::simulate::MainPhaseChoice;

pub struct GameTreeContext<'a> {
    allocator: &'a Bump,
    main_phase_choice_vec: Vec<MainPhaseChoice, &'a Bump>,
}

impl<'a> GameTreeContext<'a> {
    pub fn new(allocator: &'a Bump) -> Self {
        Self {
            allocator,
            main_phase_choice_vec: Vec::with_capacity_in(25, allocator),
        }
    }

    // pub fn generate(&mut self, state: CompleteGameState) -> Node<'a> {
    //     match state {
    //         CompleteGameState::Finished(score) => Node::Complete(match score {
    //             Score(0) => 0.0,
    //             Score(x) if x < 0 => -1.0,
    //             Score(x) if x > 0 => 1.0,
    //         }),
    //         CompleteGameState::Unfinished(state) => {
    //             let main_phase_choices = &mut self.main_phase_choice_vec;
    //             state.main_phase_choices(Player::You, main_phase_choices);
    //
    //             let node = Node::Decision(DecisionNode::new(
    //                 main_phase_choices.len(),
    //                 self.allocator,
    //                 main_phase_choices.len(), 
    //                 false, // hidden info
    //                 true, // yes, the players have been swapped
    //                 state.get_overseer_candiates(Player::You)
    //             ));
    //
    //             Node::Empty
    //         }
    //     }
    // }
}
