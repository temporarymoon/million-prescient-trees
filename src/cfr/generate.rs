use super::decision::{DecisionMatrices, ExploredScope, Scope, UnexploredScope};
use super::decision_index::DecisionIndex;
use super::hidden_index::HiddenIndex;
use super::reveal_index::RevealIndex;
use crate::game::choice::{FinalMainPhaseChoice, SabotagePhaseChoice};
use crate::game::creature::{Creature, CreatureSet};
use crate::game::edict::Edict;
use crate::game::known_state::KnownState;
use crate::game::simulate::BattleContext;
use crate::game::types::TurnResult;
use crate::helpers::bitfield::Bitfield;
use crate::helpers::pair::{are_equal, Pair};
use bumpalo::Bump;
use derive_more::{Add, AddAssign, Sum};
use indicatif::HumanBytes;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::fmt::Debug;
use std::format;
use std::iter::Sum;
use std::mem::size_of;
use std::ops::{AddAssign, Index, IndexMut};

// {{{ Phase tags
#[derive(Copy, Clone)]
enum PhaseTag {
    Main,
    Sabotage,
    Seer,
}
// }}}
// {{{ PhaseStats
#[derive(Default, Copy, Clone, Add, AddAssign, Sum)]
pub struct PhaseStats {
    pub count: usize,
    pub total_decisions: usize,
    pub total_hidden: usize,
    pub total_next: usize,
    pub total_weights: usize,
    pub memory_estimate: usize,
}

impl PhaseStats {
    pub fn average_decisions(&self) -> usize {
        self.total_decisions / self.count
    }

    pub fn average_hidden(&self) -> usize {
        self.total_hidden / self.count
    }

    pub fn average_next(&self) -> usize {
        self.total_next / self.count
    }

    pub fn estimate_weight_storage_per_battlefield(&self) -> usize {
        self.total_weights * size_of::<f32>()
    }

    pub fn estimate_weight_storage(&self) -> usize {
        self.estimate_weight_storage_per_battlefield() * 24
    }
}

impl Debug for PhaseStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhaseStats")
            .field("count", &self.count)
            .field(
                "memory",
                &format!("{}", &HumanBytes(self.memory_estimate as u64)),
            )
            .field("average hidden", &self.average_hidden())
            .field("average decision", &self.average_decisions())
            .field("average next", &self.average_next())
            .finish()
    }
}
// }}}
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
// {{{ The Phase trait
trait Phase: Sync {
    type Next: Phase;

    const TAG: PhaseTag;
    const ADVANCES_TURN: bool = false;

    fn is_symmetrical(&self) -> bool;
    fn advance_state(
        &self,
        state: &KnownState,
        reveal_index: RevealIndex,
    ) -> TurnResult<(Self::Next, KnownState)>;

    fn decision_counts(&self, state: &KnownState) -> Pair<usize>;
    fn hidden_counts(&self, state: &KnownState) -> Pair<usize>;
    fn reveal_count(&self, state: &KnownState) -> usize;
}
// }}}
// {{{ Phase instances
// {{{ Main phase
struct MainPhase;

impl MainPhase {
    fn new() -> Self {
        Self {}
    }
}

impl Phase for MainPhase {
    type Next = SabotagePhase;

    const TAG: PhaseTag = PhaseTag::Main;

    fn is_symmetrical(&self) -> bool {
        true
    }

    fn decision_counts(&self, state: &KnownState) -> Pair<usize> {
        let hand_size = state.hand_size();
        let seer_statuses = state.seer_statuses();

        state
            .edict_sets()
            .zip(seer_statuses)
            .map(|(edicts, seer_status)| {
                DecisionIndex::main_phase_index_count(edicts.len(), hand_size, seer_status)
            })
    }

    fn hidden_counts(&self, state: &KnownState) -> Pair<usize> {
        let hand_size = state.hand_size();
        let count = HiddenIndex::main_index_count(hand_size, state.graveyard);

        [count; 2]
    }

    fn reveal_count(&self, state: &KnownState) -> usize {
        RevealIndex::main_phase_count(state.edict_sets())
    }

    fn advance_state(
        &self,
        state: &KnownState,
        reveal_index: RevealIndex,
    ) -> TurnResult<(Self::Next, KnownState)> {
        let edict_choices = reveal_index
            .decode_main_phase_reveal(state.edict_sets())
            .unwrap();

        TurnResult::Unfinished((SabotagePhase::new(edict_choices), *state))
    }
}
// }}}
// {{{ Sabotage phase
struct SabotagePhase {
    pub edict_choices: Pair<Edict>,
}

impl SabotagePhase {
    fn new(edict_choices: Pair<Edict>) -> Self {
        Self { edict_choices }
    }

    fn sabotage_vector_size(did_sabotage: bool, guess_count: usize) -> usize {
        if did_sabotage {
            guess_count
        } else {
            1
        }
    }

    fn sabotage_statuses(&self) -> Pair<bool> {
        self.edict_choices.map(|edict| edict == Edict::Sabotage)
    }
}

impl Phase for SabotagePhase {
    type Next = SeerPhase;

    const TAG: PhaseTag = PhaseTag::Sabotage;

    fn is_symmetrical(&self) -> bool {
        are_equal(self.edict_choices)
    }

    fn decision_counts(&self, state: &KnownState) -> Pair<usize> {
        let guess_count =
            DecisionIndex::sabotage_phase_index_count_old_hand(state.hand_size(), state.graveyard);

        self.sabotage_statuses()
            .map(|status| Self::sabotage_vector_size(status, guess_count))
    }

    fn hidden_counts(&self, state: &KnownState) -> Pair<usize> {
        state.seer_statuses().map(|status| {
            HiddenIndex::sabotage_seer_index_count_old_hand(
                state.hand_size(),
                state.graveyard,
                status,
            )
        })
    }

    fn reveal_count(&self, state: &KnownState) -> usize {
        RevealIndex::sabotage_phase_count(
            self.sabotage_statuses(),
            state.forced_seer_player(),
            state.graveyard,
        )
    }

    fn advance_state(
        &self,
        state: &KnownState,
        reveal_index: RevealIndex,
    ) -> TurnResult<(Self::Next, KnownState)> {
        let (sabotage_choices, revealed_creature) = reveal_index
            .decode_sabotage_phase_reveal(
                self.sabotage_statuses(),
                state.forced_seer_player(),
                state.graveyard,
            )
            .unwrap();

        TurnResult::Unfinished((
            SeerPhase::new(self.edict_choices, sabotage_choices, revealed_creature),
            *state,
        ))
    }
}
// }}}
// {{{ Seer phase
struct SeerPhase {
    pub edict_choices: Pair<Edict>,
    pub sabotage_choices: Pair<SabotagePhaseChoice>,
    pub revealed_creature: Creature,
}

impl SeerPhase {
    fn new(
        edict_choices: Pair<Edict>,
        sabotage_choices: Pair<SabotagePhaseChoice>,
        revealed_creature: Creature,
    ) -> Self {
        Self {
            edict_choices,
            sabotage_choices,
            revealed_creature,
        }
    }

    fn graveyard(&self, state: &KnownState) -> CreatureSet {
        let mut graveyard = state.graveyard;
        graveyard.add(self.revealed_creature);
        graveyard
    }
}

impl Phase for SeerPhase {
    type Next = MainPhase;

    const ADVANCES_TURN: bool = true;
    const TAG: PhaseTag = PhaseTag::Seer;

    fn is_symmetrical(&self) -> bool {
        false
    }

    fn decision_counts(&self, state: &KnownState) -> Pair<usize> {
        let seer_player_decisions = if state.seer_is_active() { 2 } else { 1 };
        state
            .forced_seer_player()
            .order_as([seer_player_decisions, 1])
    }

    fn hidden_counts(&self, state: &KnownState) -> Pair<usize> {
        state.seer_statuses().map(|status| {
            HiddenIndex::sabotage_seer_index_count_old_hand(
                state.hand_size(),
                self.graveyard(state),
                status,
            )
        })
    }

    fn reveal_count(&self, state: &KnownState) -> usize {
        RevealIndex::seer_phase_count(self.graveyard(state))
    }

    fn advance_state(
        &self,
        state: &KnownState,
        reveal_index: RevealIndex,
    ) -> TurnResult<(Self::Next, KnownState)> {
        let seer_player_creature = reveal_index
            .decode_seer_phase_reveal(self.graveyard(state))
            .unwrap();

        let main_choices = state
            .forced_seer_player()
            .order_as([seer_player_creature, self.revealed_creature])
            .zip(self.edict_choices)
            .map(|(creatures, edict)| FinalMainPhaseChoice::new(creatures, edict));

        let context = BattleContext {
            main_choices,
            sabotage_choices: self.sabotage_choices,
            state: *state,
        };

        match context.advance_known_state().1 {
            TurnResult::Finished(score) => TurnResult::Finished(score),
            TurnResult::Unfinished(state) => TurnResult::Unfinished((MainPhase::new(), state)),
        }
    }
}
// }}}
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
