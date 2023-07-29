use super::decision::{DecisionMatrices, ExploredScope, Scope, UnexploredScope};
use super::phase::{PhaseStats, PhaseTag, MainPhase, Phase};
use super::reveal_index::RevealIndex;
use crate::game::known_state::KnownState;
use crate::game::types::TurnResult;
use bumpalo::Bump;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::fmt::Debug;
use std::iter::Sum;
use std::mem::size_of;
use std::ops::{AddAssign, Index, IndexMut};

// {{{ Stats
#[derive(Default, Copy, Clone)]
pub struct GenerationStats {
    pub explored_scopes: usize,
    pub unexplored_scopes: usize,
    pub completed_scopes: usize,
    pub phase_stats: [PhaseStats; 3],
}

impl GenerationStats {
    pub fn total(&self) -> PhaseStats {
        self.phase_stats.iter().copied().sum()
    }
}

impl AddAssign for GenerationStats {
    fn add_assign(&mut self, rhs: Self) {
        self.explored_scopes += rhs.explored_scopes;
        self.unexplored_scopes += rhs.unexplored_scopes;
        self.completed_scopes += rhs.completed_scopes;
        self.phase_stats[0] += rhs.phase_stats[0];
        self.phase_stats[1] += rhs.phase_stats[1];
        self.phase_stats[2] += rhs.phase_stats[2];
    }
}

impl Sum for GenerationStats {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut result = Self::default();
        for i in iter {
            result += i
        }
        result
    }
}

impl Index<PhaseTag> for GenerationStats {
    type Output = PhaseStats;
    fn index(&self, index: PhaseTag) -> &Self::Output {
        &self.phase_stats[index as usize]
    }
}

impl IndexMut<PhaseTag> for GenerationStats {
    fn index_mut(&mut self, index: PhaseTag) -> &mut Self::Output {
        &mut self.phase_stats[index as usize]
    }
}

impl Debug for GenerationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerationStats")
            .field("explored scopes", &self.explored_scopes)
            .field("unexplored scopes", &self.unexplored_scopes)
            .field("completed scopes", &self.completed_scopes)
            .field("main phase", &self[PhaseTag::Main])
            .field("sabotage phase", &self[PhaseTag::Sabotage])
            .field("seer phase", &self[PhaseTag::Seer])
            .field("total", &self.total())
            .finish()
    }
}
// }}}
// {{{ Generate
#[derive(Clone, Copy)]
pub struct GenerationContext<'a> {
    turns: usize,
    state: KnownState,
    allocator: &'a Bump,
}

impl<'a> GenerationContext<'a> {
    // {{{ Helpers
    pub fn new(turns: usize, state: KnownState, allocator: &'a Bump) -> Self {
        Self {
            turns,
            state,
            allocator,
        }
    }

    pub fn generate(&self) -> Scope<'a> {
        self.generate_generic(MainPhase::new())
    }
    // }}}
    // {{{ Generic generation
    fn generate_generic<P: Phase>(&self, phase: P) -> Scope<'a> {
        if self.turns == 0 {
            return Scope::Unexplored(UnexploredScope { state: None });
        }

        let vector_sizes = phase.decision_counts(&self.state);
        let hidden_counts = phase.hidden_counts(&self.state);
        let matrices = DecisionMatrices::new(
            self.state.is_symmetrical() && phase.is_symmetrical(),
            hidden_counts,
            vector_sizes,
            self.allocator,
        );

        let next = self
            .allocator
            .alloc_slice_fill_with(phase.reveal_count(&self.state), |index| {
                let advanced = phase.advance_state(&self.state, RevealIndex(index));

                match advanced {
                    TurnResult::Finished(score) => Scope::Completed(score),
                    TurnResult::Unfinished((next, new_state)) => {
                        let new_self = Self::new(
                            self.turns - P::ADVANCES_TURN as usize,
                            new_state,
                            self.allocator,
                        );

                        new_self.generate_generic::<P::Next>(next)
                    }
                }
            });

        Scope::Explored(ExploredScope { matrices, next })
    }
    // }}}
}
// }}}
// {{{ Estimate
#[derive(Clone, Copy)]
pub struct EstimationContext {
    turns: usize,
    state: KnownState,
}

impl EstimationContext {
    // {{{ Helpers
    pub fn new(turns: usize, state: KnownState) -> Self {
        Self { turns, state }
    }

    pub fn estimate(&self) -> GenerationStats {
        self.estimate_generic(MainPhase::new())
    }

    fn estimate_slice_alloc<T: Sum + Send, F>(len: usize, f: F) -> (usize, T)
    where
        F: Sync + Fn(usize) -> T,
    {
        let combined = (0..len).into_par_iter().map(|i| f(i)).sum();
        let size = size_of::<T>() * len;

        (size, combined)
    }
    // }}}
    // {{{ Generic estimation
    fn estimate_generic<P: Phase>(&self, phase: P) -> GenerationStats {
        if self.turns == 0 {
            let mut stats = GenerationStats::default();
            stats.unexplored_scopes += 1;
            return stats;
        }

        let is_symmetrical = self.state.is_symmetrical() && phase.is_symmetrical();
        let vector_sizes = phase.decision_counts(&self.state);
        let hidden_counts = phase.hidden_counts(&self.state);
        let reveal_count = phase.reveal_count(&self.state) / if is_symmetrical { 2 } else { 1 };

        let (slice_memory_estimate, mut stats) =
            Self::estimate_slice_alloc(reveal_count, |index| {
                let advanced = phase.advance_state(&self.state, RevealIndex(index));

                match advanced {
                    TurnResult::Finished(_) => {
                        let mut stats = GenerationStats::default();
                        stats.completed_scopes += 1;
                        stats
                    }
                    TurnResult::Unfinished((next, new_state)) => {
                        let new_self = Self::new(self.turns - P::ADVANCES_TURN as usize, new_state);

                        new_self.estimate_generic::<P::Next>(next)
                    }
                }
            });

        let tag = P::TAG;
        stats[tag].count += 1;
        stats[tag].total_next += reveal_count;
        stats[tag].memory_estimate +=
            DecisionMatrices::estimate_alloc(is_symmetrical, hidden_counts, vector_sizes);
        stats[tag].total_weights +=
            DecisionMatrices::estimate_weight_storage(is_symmetrical, hidden_counts, vector_sizes);
        stats[tag].memory_estimate += slice_memory_estimate;

        // TODO: these are not quite accurate
        stats[tag].total_hidden += hidden_counts[0] + hidden_counts[1];
        stats[tag].total_decisions += vector_sizes[0] + vector_sizes[1];

        stats.explored_scopes += 1;

        stats
    }
    // }}}
}
// }}}
