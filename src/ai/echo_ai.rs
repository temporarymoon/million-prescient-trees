use crate::cfr::decision_index::DecisionIndex;
use crate::cfr::hidden_index::{self, HiddenIndex, HiddenState};
use crate::cfr::phase::SomePhase;
use crate::game::known_state::KnownState;
use crate::game::types::{BattleResult, Player, TurnResult};
use crate::helpers::pair::Pair;

// {{{ Agent input
#[derive(Debug, Clone, Copy)]
pub struct AgentInput {
    pub phase: SomePhase,
    pub state: KnownState,
    pub hidden: HiddenIndex,
}

impl AgentInput {
    pub fn new(phase: SomePhase, state: KnownState, hidden: HiddenIndex) -> Self {
        Self {
            phase,
            state,
            hidden,
        }
    }
}
// }}}
// {{{ Main trait
/// Generic trait that can be implemented by any echo ai.
/// Right now, it requires the ai to keep track of the game state.
/// In the future, the game state *will* be provided for free to the ai.
pub trait EchoAgent {
    fn choose(&self, agent_input: AgentInput) -> Option<DecisionIndex>;
}

pub struct EchoRunner<A, B> {
    state: KnownState,
    phase: SomePhase,
    agents: (A, B),
    hidden_indices: Pair<hidden_index::EncodingInfo>,
}
// }}}
// {{{ Game runner
#[allow(unreachable_code)]
impl<A, B> EchoRunner<A, B> {
    fn input_for(&self, player: Player) -> Option<AgentInput> {
        let info = player.select(self.hidden_indices);
        let hidden = HiddenIndex::encode(&self.state, player, info);
        let input = AgentInput::new(self.phase, self.state, hidden);

        Some(input)
    }

    fn simulation_step(mut self) -> Option<BattleResult>
    where
        A: EchoAgent,
        B: EchoAgent,
    {
        loop {
            let my = self.agents.0.choose(self.input_for(Player::Me)?)?;
            let yours = self.agents.1.choose(self.input_for(Player::You)?)?;
            let decisions = [my, yours];

            let res = self.phase.advance(
                self.state,
                self.hidden_indices.map(HiddenState::from_encoding_info),
                decisions,
            )?;

            match res {
                TurnResult::Finished(score) => return Some(score.to_battle_result()),
                TurnResult::Unfinished((state, hidden, _, phase)) => {
                    self.state = state;
                    self.hidden_indices = hidden;
                    self.phase = phase;
                }
            }
        }
    }
}
// }}}
