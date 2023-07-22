use super::decision::{DecisionMatrices, DecisionMatrix, ExploredScope, Scope, UnexploredScope};
use super::decision_index::DecisionIndex;
use super::hidden_index::HiddenIndex;
use super::reveal_index::RevealIndex;
use crate::game::choice::{FinalMainPhaseChoice, SabotagePhaseChoice};
use crate::game::creature::Creature;
use crate::game::edict::Edict;
use crate::game::known_state::KnownState;
use crate::game::simulate::BattleContext;
use crate::game::types::TurnResult;
use crate::helpers::bitfield::Bitfield;
use crate::helpers::pair::{are_equal, Pair};
use bumpalo::Bump;
use derive_more::{Add, AddAssign};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use std::iter::Sum;
use std::mem::size_of;

#[derive(Default, Copy, Clone, Add, AddAssign)]
pub struct GenerationStats {
    pub explored_scopes: usize,
    pub unexplored_scopes: usize,
    pub completed_scopes: usize,

    pub main_count: usize,
    pub sabotage_count: usize,
    pub seer_count: usize,

    pub main_total_decisions: usize,
    pub main_total_hidden: usize,
    pub main_total_next: usize,
    pub main_total_weights: usize,
    pub sabotage_total_decisions: usize,
    pub sabotage_total_hidden: usize,
    pub sabotage_total_next: usize,
    pub sabotage_total_weights: usize,
    pub seer_total_decisions: usize,
    pub seer_total_hidden: usize,
    pub seer_total_next: usize,
    pub seer_total_weights: usize,
    pub memory_estimate: usize,
}

impl GenerationStats {
    // {{{ Averages
    pub fn main_average_decisions(&self) -> usize {
        self.main_total_decisions / self.main_count
    }

    pub fn main_average_hidden(&self) -> usize {
        self.main_total_hidden / self.main_count
    }

    pub fn main_average_next(&self) -> usize {
        self.main_total_next / self.main_count
    }

    pub fn sabotage_average_decisions(&self) -> usize {
        self.sabotage_total_decisions / self.sabotage_count
    }

    pub fn sabotage_average_hidden(&self) -> usize {
        self.sabotage_total_hidden / self.sabotage_count
    }

    pub fn sabotage_average_next(&self) -> usize {
        self.sabotage_total_next / self.sabotage_count
    }

    pub fn seer_average_decisions(&self) -> usize {
        self.seer_total_decisions / self.seer_count
    }

    pub fn seer_average_hidden(&self) -> usize {
        self.seer_total_hidden / self.seer_count
    }

    pub fn seer_average_next(&self) -> usize {
        self.seer_total_next / self.seer_count
    }
    // }}}
    // {{{ Other helpers
    pub fn total_count(&self) -> usize {
        self.main_count + self.sabotage_count + self.seer_count
    }

    pub fn total_weights(&self) -> usize {
        self.main_total_weights + self.sabotage_total_weights + self.seer_total_weights
    }

    pub fn estimate_weight_storage_per_battlefield(&self) -> usize {
        self.total_weights() * size_of::<f32>()
    }

    pub fn estimate_weight_storage(&self) -> usize {
        self.estimate_weight_storage_per_battlefield() * 24
    }

    pub fn estimate_alloc(&self) -> usize {
        self.memory_estimate
    }
    // }}}
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

// {{{ Generate
pub struct GenerationContext<'a> {
    turns: usize,
    state: KnownState,
    is_first_turn: bool,
    allocator: &'a Bump,
}

impl<'a> GenerationContext<'a> {
    pub fn new(turns: usize, state: KnownState, allocator: &'a Bump) -> Self {
        Self {
            turns,
            state,
            allocator,
            is_first_turn: true, // TODO: do we want to always assume this?
        }
    }

    fn next_turn(&self, state: KnownState) -> Self {
        Self {
            turns: self.turns - 1,
            state,
            is_first_turn: false,
            ..*self
        }
    }

    pub fn generate(&mut self) -> (Scope<'a>, GenerationStats) {
        let mut stats = GenerationStats::default();
        let result = self.generate_turn(&mut stats);
        (result, stats)
    }

    fn generate_turn(&mut self, stats: &mut GenerationStats) -> Scope<'a> {
        if self.turns == 0 {
            stats.unexplored_scopes += 1;
            // let state = self.allocator.alloc(self.state);
            return Scope::Unexplored(UnexploredScope { state: None });
        }

        self.generate_main(stats)
    }

    // {{{ Main phase
    fn generate_main(&mut self, stats: &mut GenerationStats) -> Scope<'a> {
        let edicts = (
            self.state.player_states.0.edicts,
            self.state.player_states.1.edicts,
        );

        let hand_size = self.state.hand_size();
        let seer_statuses = self.state.seer_statuses();

        let vector_sizes = (
            DecisionIndex::main_phase_index_count(edicts.0.len(), hand_size, seer_statuses.0),
            DecisionIndex::main_phase_index_count(edicts.1.len(), hand_size, seer_statuses.1),
        );

        let hidden_count = HiddenIndex::main_index_count(hand_size, self.state.graveyard);
        let matrices = DecisionMatrices::new(
            self.state.is_symmetrical(self.is_first_turn),
            (hidden_count, hidden_count),
            vector_sizes,
            self.allocator,
        );

        let next =
            self.allocator
                .alloc_slice_fill_with(RevealIndex::main_phase_count(edicts), |index| {
                    let choice = RevealIndex(index).decode_main_phase_reveal(edicts).unwrap();
                    stats.main_total_next += 1;
                    self.generate_sabotage(stats, choice)
                });

        stats.main_count += 1;
        stats.main_total_hidden += hidden_count * 2;
        stats.main_total_decisions += vector_sizes.0 + vector_sizes.1;
        stats.main_total_weights +=
            DecisionMatrix::estimate_weight_storage(hidden_count, vector_sizes.0);
        stats.main_total_weights +=
            DecisionMatrix::estimate_weight_storage(hidden_count, vector_sizes.1);
        stats.explored_scopes += 1;
        Scope::Explored(ExploredScope { matrices, next })
    }
    // }}}
    // {{{ Sabotage phase
    fn sabotage_vector_size(did_sabotage: bool, guess_count: usize) -> usize {
        if did_sabotage {
            guess_count
        } else {
            1
        }
    }

    fn generate_sabotage(
        &mut self,
        stats: &mut GenerationStats,
        edict_choices: Pair<Edict>,
    ) -> Scope<'a> {
        let hand_size = self.state.hand_size();
        let seer_statuses = self.state.seer_statuses();
        let seer_player = self.state.forced_seer_player();
        let sabotage_statuses = (
            edict_choices.0 == Edict::Sabotage,
            edict_choices.1 == Edict::Sabotage,
        );

        let guess_count =
            DecisionIndex::sabotage_phase_index_count_old_hand(hand_size, self.state.graveyard);

        let vector_sizes = (
            Self::sabotage_vector_size(sabotage_statuses.0, guess_count),
            Self::sabotage_vector_size(sabotage_statuses.1, guess_count),
        );

        let hidden_counts = (
            HiddenIndex::sabotage_seer_index_count_old_hand(
                hand_size,
                self.state.graveyard,
                seer_statuses.0,
            ),
            HiddenIndex::sabotage_seer_index_count_old_hand(
                hand_size,
                self.state.graveyard,
                seer_statuses.1,
            ),
        );

        let matrices = DecisionMatrices::new(
            self.state.is_symmetrical(self.is_first_turn) && are_equal(edict_choices),
            hidden_counts,
            vector_sizes,
            self.allocator,
        );

        let next = self.allocator.alloc_slice_fill_with(
            RevealIndex::sabotage_phase_count(sabotage_statuses, seer_player, self.state.graveyard),
            |index| {
                let (sabotage_choices, revealed_creature) = RevealIndex(index)
                    .decode_sabotage_phase_reveal(
                        sabotage_statuses,
                        seer_player,
                        self.state.graveyard,
                    )
                    .unwrap();
                stats.sabotage_total_next += 1;
                self.generate_seer(stats, edict_choices, revealed_creature, sabotage_choices)
            },
        );

        stats.sabotage_count += 1;
        stats.sabotage_total_hidden += hidden_counts.0 + hidden_counts.1;
        stats.sabotage_total_decisions += vector_sizes.0 + vector_sizes.1;
        stats.sabotage_total_weights +=
            DecisionMatrix::estimate_weight_storage(hidden_counts.0, vector_sizes.0);
        stats.sabotage_total_weights +=
            DecisionMatrix::estimate_weight_storage(hidden_counts.1, vector_sizes.1);
        stats.explored_scopes += 1;
        Scope::Explored(ExploredScope { matrices, next })
    }
    // }}}
    // {{{ Seer phase
    fn generate_seer(
        &mut self,
        stats: &mut GenerationStats,
        edict_choices: Pair<Edict>,
        non_seer_player_creature: Creature,
        sabotage_choices: Pair<SabotagePhaseChoice>,
    ) -> Scope<'a> {
        let hand_size = self.state.hand_size();
        let seer_active = self.state.seer_is_active();
        let seer_statuses = self.state.seer_statuses();
        let seer_player = self.state.forced_seer_player();

        let mut graveyard = self.state.graveyard.clone();
        graveyard.add(non_seer_player_creature);

        let vector_sizes = seer_player.order_as((if seer_active { 2 } else { 1 }, 1));

        let hidden_counts = (
            HiddenIndex::sabotage_seer_index_count_old_hand(hand_size, graveyard, seer_statuses.0),
            HiddenIndex::sabotage_seer_index_count_old_hand(hand_size, graveyard, seer_statuses.1),
        );

        let matrices = DecisionMatrices::new(
            self.state.is_symmetrical(self.is_first_turn)
                && are_equal(sabotage_choices)
                && are_equal(edict_choices),
            hidden_counts,
            vector_sizes,
            self.allocator,
        );

        let next = self.allocator.alloc_slice_fill_with(
            RevealIndex::seer_phase_count(graveyard),
            |index| {
                let seer_player_creature = RevealIndex(index)
                    .decode_seer_phase_reveal(graveyard)
                    .unwrap();

                let creature_choices =
                    seer_player.order_as((seer_player_creature, non_seer_player_creature));

                let context = BattleContext {
                    main_choices: (
                        FinalMainPhaseChoice::new(creature_choices.0, edict_choices.0),
                        FinalMainPhaseChoice::new(creature_choices.1, edict_choices.1),
                    ),
                    sabotage_choices,
                    state: self.state,
                };

                stats.seer_total_next += 1;
                match context.advance_known_state().1 {
                    TurnResult::Finished(score) => {
                        stats.completed_scopes += 1;
                        Scope::Completed(score)
                    }
                    TurnResult::Unfinished(mut state) => {
                        state.graveyard.add(non_seer_player_creature);
                        state.graveyard.add(seer_player_creature);

                        self.next_turn(state).generate_turn(stats)
                    }
                }
            },
        );

        stats.seer_count += 1;
        stats.seer_total_hidden += hidden_counts.0 + hidden_counts.1;
        stats.seer_total_decisions += vector_sizes.0 + vector_sizes.1;
        stats.seer_total_weights +=
            DecisionMatrix::estimate_weight_storage(hidden_counts.0, vector_sizes.0);
        stats.seer_total_weights +=
            DecisionMatrix::estimate_weight_storage(hidden_counts.1, vector_sizes.1);
        stats.explored_scopes += 1;
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
    is_first_turn: bool,
}

impl EstimationContext {
    pub fn new(turns: usize, state: KnownState) -> Self {
        Self {
            turns,
            state,
            is_first_turn: true,
        }
    }

    // {{{ Helpers
    fn next_turn(&self, state: KnownState) -> Self {
        Self {
            turns: self.turns - 1,
            state,
            is_first_turn: false,
            ..*self
        }
    }

    fn estimate_slice_alloc<T: Sum + Send, F>(len: usize, f: F) -> (usize, T)
    where
        F: Sync + Fn(usize) -> T,
    {
        let combined = (0..len).into_par_iter().map(|i| f(i)).sum();
        let size = size_of::<T>() * len;

        (size, combined)
    }

    pub fn estimate_alloc(&mut self) -> GenerationStats {
        self.estimate_turn()
    }

    fn estimate_turn(&mut self) -> GenerationStats {
        if self.turns == 0 {
            let mut stats = GenerationStats::default();
            stats.unexplored_scopes += 1;
            stats
        } else {
            self.estimate_main()
        }
    }
    // }}}
    // {{{ Main phase
    fn estimate_main(&self) -> GenerationStats {
        let edicts = (
            self.state.player_states.0.edicts,
            self.state.player_states.1.edicts,
        );

        let hand_size = self.state.hand_size();
        let seer_statuses = self.state.seer_statuses();

        let vector_sizes = (
            DecisionIndex::main_phase_index_count(edicts.0.len(), hand_size, seer_statuses.0),
            DecisionIndex::main_phase_index_count(edicts.1.len(), hand_size, seer_statuses.1),
        );

        let hidden_count = HiddenIndex::main_index_count(hand_size, self.state.graveyard);
        let hidden_counts = (hidden_count, hidden_count);
        let next_count = RevealIndex::main_phase_count(edicts);
        let (slice_estimate, mut stats) = Self::estimate_slice_alloc(next_count, move |index| {
            let choice = RevealIndex(index).decode_main_phase_reveal(edicts).unwrap();
            self.estimate_sabotage(choice)
        });

        stats.main_total_next += next_count;
        stats.memory_estimate += slice_estimate;

        let is_symmetrical = self.state.is_symmetrical(self.is_first_turn);
        stats.memory_estimate +=
            DecisionMatrices::estimate_alloc(is_symmetrical, hidden_counts, vector_sizes);
        stats.main_total_weights +=
            DecisionMatrices::estimate_weight_storage(is_symmetrical, hidden_counts, vector_sizes);

        stats.main_count += 1;
        stats.main_total_hidden += hidden_count * 2;
        stats.main_total_decisions += vector_sizes.0 + vector_sizes.1;
        stats.explored_scopes += 1;
        stats
    }
    // }}}
    // {{{ Sabotage phase
    fn estimate_sabotage(&self, edict_choices: Pair<Edict>) -> GenerationStats {
        let hand_size = self.state.hand_size();
        let seer_statuses = self.state.seer_statuses();
        let seer_player = self.state.forced_seer_player();
        let sabotage_statuses = (
            edict_choices.0 == Edict::Sabotage,
            edict_choices.1 == Edict::Sabotage,
        );

        let guess_count =
            DecisionIndex::sabotage_phase_index_count_old_hand(hand_size, self.state.graveyard);

        let vector_sizes = (
            GenerationContext::sabotage_vector_size(sabotage_statuses.0, guess_count),
            GenerationContext::sabotage_vector_size(sabotage_statuses.1, guess_count),
        );

        let hidden_counts = (
            HiddenIndex::sabotage_seer_index_count_old_hand(
                hand_size,
                self.state.graveyard,
                seer_statuses.0,
            ),
            HiddenIndex::sabotage_seer_index_count_old_hand(
                hand_size,
                self.state.graveyard,
                seer_statuses.1,
            ),
        );

        let reveal_count =
            RevealIndex::sabotage_phase_count(sabotage_statuses, seer_player, self.state.graveyard);

        let (slice_estimate, mut stats) = Self::estimate_slice_alloc(reveal_count, move |index| {
            let (sabotage_choices, revealed_creature) = RevealIndex(index)
                .decode_sabotage_phase_reveal(sabotage_statuses, seer_player, self.state.graveyard)
                .unwrap();
            self.estimate_seer(edict_choices, revealed_creature, sabotage_choices)
        });

        stats.sabotage_total_next += reveal_count;
        stats.memory_estimate += slice_estimate;

        let is_symmetrical =
            self.state.is_symmetrical(self.is_first_turn) && are_equal(edict_choices);
        stats.memory_estimate +=
            DecisionMatrices::estimate_alloc(is_symmetrical, hidden_counts, vector_sizes);
        stats.main_total_weights +=
            DecisionMatrices::estimate_weight_storage(is_symmetrical, hidden_counts, vector_sizes);

        stats.sabotage_count += 1;
        stats.sabotage_total_hidden += hidden_counts.0 + hidden_counts.1;
        stats.sabotage_total_decisions += vector_sizes.0 + vector_sizes.1;
        stats.sabotage_total_weights +=
            hidden_counts.0 * vector_sizes.0 + hidden_counts.1 * vector_sizes.1;
        stats.explored_scopes += 1;
        stats
    }
    // }}}
    // {{{ Seer phase
    fn estimate_seer(
        &self,
        edict_choices: Pair<Edict>,
        non_seer_player_creature: Creature,
        sabotage_choices: Pair<SabotagePhaseChoice>,
    ) -> GenerationStats {
        let hand_size = self.state.hand_size();
        let seer_active = self.state.seer_is_active();
        let seer_statuses = self.state.seer_statuses();
        let seer_player = self.state.forced_seer_player();

        let mut graveyard = self.state.graveyard.clone();
        graveyard.add(non_seer_player_creature);

        let vector_sizes = seer_player.order_as((if seer_active { 2 } else { 1 }, 1));

        let hidden_counts = (
            HiddenIndex::sabotage_seer_index_count_old_hand(hand_size, graveyard, seer_statuses.0),
            HiddenIndex::sabotage_seer_index_count_old_hand(hand_size, graveyard, seer_statuses.1),
        );

        let reveal_count = RevealIndex::seer_phase_count(graveyard);

        let (slice_estimate, mut stats) = Self::estimate_slice_alloc(reveal_count, |index| {
            let seer_player_creature = RevealIndex(index)
                .decode_seer_phase_reveal(graveyard)
                .unwrap();

            let creature_choices =
                seer_player.order_as((seer_player_creature, non_seer_player_creature));

            let context = BattleContext {
                main_choices: (
                    FinalMainPhaseChoice::new(creature_choices.0, edict_choices.0),
                    FinalMainPhaseChoice::new(creature_choices.1, edict_choices.1),
                ),
                sabotage_choices,
                state: self.state,
            };

            match context.advance_known_state().1 {
                TurnResult::Finished(_) => {
                    let mut stats = GenerationStats::default();
                    stats.completed_scopes += 1;
                    stats
                }
                TurnResult::Unfinished(mut state) => {
                    state.graveyard.add(non_seer_player_creature);
                    state.graveyard.add(seer_player_creature);

                    self.next_turn(state).estimate_turn()
                }
            }
        });

        stats.memory_estimate += slice_estimate;

        let is_symmetrical = self.state.is_symmetrical(self.is_first_turn)
            && are_equal(edict_choices)
            && are_equal(sabotage_choices);
        stats.memory_estimate +=
            DecisionMatrices::estimate_alloc(is_symmetrical, hidden_counts, vector_sizes);
        stats.main_total_weights +=
            DecisionMatrices::estimate_weight_storage(is_symmetrical, hidden_counts, vector_sizes);

        stats.seer_total_next += reveal_count;
        stats.seer_count += 1;
        stats.seer_total_hidden += hidden_counts.0 + hidden_counts.1;
        stats.seer_total_decisions += vector_sizes.0 + vector_sizes.1;
        stats.explored_scopes += 1;
        stats
    }
    // }}}
}
// }}}
