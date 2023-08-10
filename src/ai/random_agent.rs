use super::echo_ai::EchoAgent;
use crate::cfr::decision_index::DecisionIndex;
use rand::Rng;

pub struct RandomAgent<R> {
    rng: R,
}

impl<R: Rng> RandomAgent<R> {
    pub fn new(rng: R) -> Self {
        Self { rng }
    }
}

impl<R: Rng> EchoAgent for RandomAgent<R> {
    fn choose(
        &mut self,
        agent_input: super::echo_ai::AgentInput,
    ) -> Option<crate::cfr::decision_index::DecisionIndex> {
        let counts = agent_input.phase.decision_counts(&agent_input.state);
        let count = agent_input.player.select(counts);
        let index = self.rng.gen_range(0..count);
        Some(DecisionIndex(index))
    }
}
