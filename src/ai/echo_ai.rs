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
    pub last_reveal: Option<RevealIndex>,
}

impl AgentInput {
    pub fn new(
        phase: SomePhase,
        state: KnownState,
        hidden: hidden_index::EncodingInfo,
        player: Player,
        last_reveal: Option<RevealIndex>,
    ) -> Self {
        Self {
            phase,
            state,
            hidden,
            player,
            last_reveal,
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
    fn game_finished(&mut self, _score: Score) {}
}
// }}}
// {{{ Game runner
/// Struct containing the data required to make two agents fight eachother.
pub struct EchoRunner<A, B> {
    state: KnownState,
    phase: SomePhase,
    agents: (A, B),
    hidden_state: Pair<hidden_index::EncodingInfo>,
    last_reveal: Option<RevealIndex>,
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
            last_reveal: None,
        }
    }

    fn input_for(&self, player: Player) -> Option<AgentInput> {
        let hidden = player.select(self.hidden_state);
        let input = AgentInput::new(self.phase, self.state, hidden, player, self.last_reveal);

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

            let res = self.phase.advance(
                self.state,
                self.hidden_state.map(HiddenState::from_encoding_info),
                decisions,
                false,
            )?;

            tracing::event!(Level::DEBUG, "Advanced state");

            match res {
                TurnResult::Finished(score) => {
                    self.agents.0.game_finished(score);
                    self.agents.1.game_finished(score);
                    return Some(score.to_battle_result());
                }
                TurnResult::Unfinished((state, hidden, reveal_index, phase)) => {
                    self.state = state;
                    self.hidden_state = hidden;
                    self.phase = phase;
                    self.last_reveal = Some(reveal_index);
                }
            }
        }
    }
}
// }}}
