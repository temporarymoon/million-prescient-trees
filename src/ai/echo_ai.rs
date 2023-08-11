use tracing::Level;

use crate::cfr::decision_index::DecisionIndex;
use crate::cfr::hidden_index::{self, HiddenState};
use crate::cfr::phase::SomePhase;
use crate::cfr::reveal_index::RevealIndex;
use crate::game::known_state::KnownState;
use crate::game::types::{BattleResult, Player, Score, TurnResult};
use crate::helpers::pair::Pair;

// {{{ Agent input
#[derive(Debug, Clone, Copy)]
pub struct AgentInput {
    pub phase: SomePhase,
    pub state: KnownState,
    pub player: Player,
    pub hidden: hidden_index::EncodingInfo,
}

impl AgentInput {
    pub fn new(
        phase: SomePhase,
        state: KnownState,
        hidden: hidden_index::EncodingInfo,
        player: Player,
    ) -> Self {
        Self {
            phase,
            state,
            hidden,
            player,
        }
    }
}
// }}}
// {{{ Main trait
/// Generic trait that can be implemented by any echo ai.
/// Right now, it requires the ai to keep track of the game state.
/// In the future, the game state *will* be provided for free to the ai.
pub trait EchoAgent {
    fn choose(&mut self, agent_input: AgentInput) -> DecisionIndex;

    #[inline(always)]
    fn reveal_info(&mut self, _reveal_index: RevealIndex, _updated_score: Score) {}

    #[inline(always)]
    fn game_finished(&mut self) {}
}
// }}}
// {{{ Game runner
/// Struct containing the data required to make two agents fight eachother.
pub struct EchoRunner<A, B> {
    state: KnownState,
    phase: SomePhase,
    agents: (A, B),
    hidden_state: Pair<hidden_index::EncodingInfo>,
}

impl<A: EchoAgent, B: EchoAgent> EchoRunner<A, B> {
    pub fn new(
        state: KnownState,
        phase: SomePhase,
        agents: (A, B),
        hidden_state: Pair<hidden_index::EncodingInfo>,
    ) -> Self {
        Self {
            state,
            phase,
            agents,
            hidden_state,
        }
    }

    fn input_for(&self, player: Player) -> Option<AgentInput> {
        let hidden = player.select(self.hidden_state);
        let input = AgentInput::new(self.phase, self.state, hidden, player);

        Some(input)
    }

    pub fn run_game(mut self) -> Option<BattleResult> {
        let _guard = tracing::span!(Level::DEBUG, "Echo fight");
        loop {
            let _guard = tracing::span!(
                Level::DEBUG,
                "Phase",
                kind = format!("{:?}", self.phase.tag())
            );

            let my = self.agents.0.choose(self.input_for(Player::Me)?);
            let yours = self.agents.1.choose(self.input_for(Player::You)?);
            let decisions = [my, yours];

            tracing::event!(Level::DEBUG, "Received both inputs");

            let (reveal_index, result) = self.phase.advance(
                self.state,
                self.hidden_state.map(HiddenState::from_encoding_info),
                decisions,
                false,
            )?;

            tracing::event!(Level::DEBUG, "Advanced state");

            let score = match result {
                TurnResult::Finished(score) => score,
                TurnResult::Unfinished((state, _, _)) => state.score,
            };

            self.agents.0.reveal_info(reveal_index, score);
            self.agents.1.reveal_info(reveal_index, score);
            tracing::event!(Level::DEBUG, "Pushed reveal indices");

            match result {
                TurnResult::Finished(_) => {
                    tracing::event!(Level::DEBUG, "Game finished");

                    self.agents.0.game_finished();
                    self.agents.1.game_finished();

                    return Some(score.to_battle_result());
                }
                TurnResult::Unfinished((state, hidden, phase)) => {
                    self.state = state;
                    self.hidden_state = hidden;
                    self.phase = phase;
                }
            }
        }
    }
}
// }}}
