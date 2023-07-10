use crate::game::types::{Battlefield, Creature, CreatureSet};

pub type MainChoice = (Creature, Option<Creature>);
pub type SabotageChoice = Option<Creature>;
pub type SeerChoice = Option<Creature>;

/// Generic trait that can be implemented by any echo ai.
/// Right now, it requires the ai to keep track of the game state.
/// In the future, the game state *will* be provided for free to the ai.
pub trait EchoAi {
    type MainState;
    type SabotageState;
    type SeerState;

    fn begin(&self, hand: CreatureSet, battlefields: [Battlefield; 4]) -> Self::MainState;

    // {{{ Choices
    fn choose_main(&self, state: &Self::MainState, choices: &[MainChoice]) -> Option<MainChoice>;
    fn choose_sabotage(
        &self,
        state: &Self::SabotageState,
        choices: &[SabotageChoice],
    ) -> Option<SabotageChoice>;
    fn choose_seer(&self, state: &Self::SeerState, choices: &[SeerChoice]) -> Option<SeerChoice>;
    // }}}
    // {{{ Advance state
    fn advance_main(
        &self,
        state: &Self::MainState,
        choices: (MainChoice, MainChoice),
    ) -> Option<Self::SabotageState>;
    fn advance_sabotage(
        &self,
        state: &Self::SabotageState,
        choices: (SabotageChoice, SabotageChoice),
    ) -> Option<Self::SeerState>;
    fn advance_seer(
        &self,
        state: &Self::SeerState,
        choices: (SeerChoice, SeerChoice),
    ) -> Option<Self::MainState>;
    // }}}
}
