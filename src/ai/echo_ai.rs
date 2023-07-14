use crate::game::{
    battlefield::Battlefield,
    choice::{MainPhaseChoice, SabotagePhaseChoice, SeerPhaseChoice},
    creature::CreatureSet,
};

/// Generic trait that can be implemented by any echo ai.
/// Right now, it requires the ai to keep track of the game state.
/// In the future, the game state *will* be provided for free to the ai.
pub trait EchoAi {
    type MainState;
    type SabotageState;
    type SeerState;

    fn begin(&self, hand: CreatureSet, battlefields: [Battlefield; 4]) -> Self::MainState;

    // {{{ Choices
    fn choose_main(
        &self,
        state: &Self::MainState,
        choices: &[MainPhaseChoice],
    ) -> Option<MainPhaseChoice>;
    fn choose_sabotage(
        &self,
        state: &Self::SabotageState,
        choices: &[SabotagePhaseChoice],
    ) -> Option<SabotagePhaseChoice>;
    fn choose_seer(
        &self,
        state: &Self::SeerState,
        choices: &[SeerPhaseChoice],
    ) -> Option<SeerPhaseChoice>;
    // }}}
    // {{{ Advance state
    fn advance_main(
        &self,
        state: &Self::MainState,
        choices: (MainPhaseChoice, MainPhaseChoice),
    ) -> Option<Self::SabotageState>;
    fn advance_sabotage(
        &self,
        state: &Self::SabotageState,
        choices: (SabotagePhaseChoice, SabotagePhaseChoice),
    ) -> Option<Self::SeerState>;
    fn advance_seer(
        &self,
        state: &Self::SeerState,
        choices: (SeerPhaseChoice, SeerPhaseChoice),
    ) -> Option<Self::MainState>;
    // }}}
}
