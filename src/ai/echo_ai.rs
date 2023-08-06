use crate::cfr::decision_index::DecisionIndex;
use crate::cfr::hidden_index::HiddenIndex;
use crate::cfr::phase::SomePhase;
use crate::game::known_state::KnownPlayerState;

#[derive(Debug, Clone, Copy)]
pub struct AgentInput {
    phase: SomePhase,
    state: KnownPlayerState,
    hidden: HiddenIndex,
}

/// Generic trait that can be implemented by any echo ai.
/// Right now, it requires the ai to keep track of the game state.
/// In the future, the game state *will* be provided for free to the ai.
pub trait EchoAgent {
    fn choose(&self, agent_input: AgentInput) -> Option<DecisionIndex>;
}
