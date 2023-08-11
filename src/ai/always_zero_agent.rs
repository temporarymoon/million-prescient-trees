use super::echo_ai::EchoAgent;
use crate::cfr::decision_index::DecisionIndex;

/// An echo agent which always plays the first choice it's offered.
#[derive(Debug, Clone, Copy, Default)]
pub struct AlwaysZeroAgent {}

impl EchoAgent for AlwaysZeroAgent {
    fn choose(
        &mut self,
        _agent_input: super::echo_ai::AgentInput,
    ) -> crate::cfr::decision_index::DecisionIndex {
        DecisionIndex::default()
    }
}
