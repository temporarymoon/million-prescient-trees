use super::decision::{DecisionMatrices, ExploredScope, Scope, UnexploredScope};
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
use std::ops::Index;

// {{{ Stats
#[derive(Default, Copy, Clone, Add, AddAssign)]
pub struct PhaseStats {
    pub count: usize,
    pub total_decisions: usize,
    pub total_hidden: usize,
    pub total_next: usize,
    pub total_weights: usize,
    pub memory_estimate: usize,
}

#[derive(Default, Copy, Clone, Add, AddAssign)]
pub struct GenerationStats {
    pub explored_scopes: usize,
    pub unexplored_scopes: usize,
    pub completed_scopes: usize,
    pub phase_stats: [PhaseStats; 3],
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

impl Index<PhaseTag> for GenerationStats {
    type Output = PhaseStats;
    fn index(&self, index: PhaseTag) -> &Self::Output {
        &self.phase_stats[index as usize]
    }
}
// }}}
// {{{ Phase tags
enum PhaseTag {
    Main,
    Sabotage,
    Seer,
}
// }}}
// {{{ The Phase trait
trait Phase {
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

        let mut new_state = *state;
        new_state.graveyard.add(revealed_creature);

        TurnResult::Unfinished((
            SeerPhase::new(self.edict_choices, sabotage_choices, revealed_creature),
            new_state,
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
                state.graveyard,
                status,
            )
        })
    }

    fn reveal_count(&self, state: &KnownState) -> usize {
        RevealIndex::seer_phase_count(state.graveyard)
    }

    fn advance_state(
        &self,
        state: &KnownState,
        reveal_index: RevealIndex,
    ) -> TurnResult<(Self::Next, KnownState)> {
        let seer_player_creature = reveal_index
            .decode_seer_phase_reveal(state.graveyard)
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
    pub fn new(turns: usize, state: KnownState, allocator: &'a Bump) -> Self {
        Self {
            turns,
            state,
            allocator,
        }
    }

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
                    TurnResult::Unfinished((next, new_state)) => Self::new(
                        self.turns - P::ADVANCES_TURN as usize,
                        new_state,
                        self.allocator,
                    )
                    .generate_generic::<P::Next>(next),
                }
            });

        Scope::Explored(ExploredScope { matrices, next })
    }
    // }}}
    // // {{{ Helpers
    // fn next_turn(&self, state: KnownState) -> Self {
    //     Self {
    //         turns: self.turns - 1,
    //         state,
    //         ..*self
    //     }
    // }
    //
    // pub fn generate(&mut self) -> Scope<'a> {
    //     self.generate_turn()
    // }
    //
    // fn generate_turn(&mut self) -> Scope<'a> {
    //     if self.turns == 0 {
    //         // let state = self.allocator.alloc(self.state);
    //         return Scope::Unexplored(UnexploredScope { state: None });
    //     }
    //
    //     self.generate_main()
    // }
    // // }}}
    // // {{{ Main phase
    // fn generate_main(&mut self) -> Scope<'a> {
    //     let edicts = self.state.player_states.map(|s| s.edicts);
    //
    //     let hand_size = self.state.hand_size();
    //     let seer_statuses = self.state.seer_statuses();
    //
    //     let vector_sizes = edicts.zip(seer_statuses).map(|(edicts, seer_status)| {
    //         DecisionIndex::main_phase_index_count(edicts.len(), hand_size, seer_status)
    //     });
    //
    //     let hidden_count = HiddenIndex::main_index_count(hand_size, self.state.graveyard);
    //     let matrices = DecisionMatrices::new(
    //         self.state.is_symmetrical(),
    //         [hidden_count; 2],
    //         vector_sizes,
    //         self.allocator,
    //     );
    //
    //     let next =
    //         self.allocator
    //             .alloc_slice_fill_with(RevealIndex::main_phase_count(edicts), |index| {
    //                 let choice = RevealIndex(index).decode_main_phase_reveal(edicts).unwrap();
    //                 self.generate_sabotage(choice)
    //             });
    //
    //     Scope::Explored(ExploredScope { matrices, next })
    // }
    // // }}}
    // // {{{ Sabotage phase
    // fn sabotage_vector_size(did_sabotage: bool, guess_count: usize) -> usize {
    //     if did_sabotage {
    //         guess_count
    //     } else {
    //         1
    //     }
    // }
    //
    // fn generate_sabotage(&mut self, edict_choices: Pair<Edict>) -> Scope<'a> {
    //     let hand_size = self.state.hand_size();
    //     let seer_statuses = self.state.seer_statuses();
    //     let seer_player = self.state.forced_seer_player();
    //     let sabotage_statuses = edict_choices.map(|c| c == Edict::Sabotage);
    //
    //     let guess_count =
    //         DecisionIndex::sabotage_phase_index_count_old_hand(hand_size, self.state.graveyard);
    //
    //     let vector_sizes =
    //         sabotage_statuses.map(|status| Self::sabotage_vector_size(status, guess_count));
    //
    //     let hidden_counts = seer_statuses.map(|status| {
    //         HiddenIndex::sabotage_seer_index_count_old_hand(hand_size, self.state.graveyard, status)
    //     });
    //
    //     let matrices = DecisionMatrices::new(
    //         self.state.is_symmetrical() && are_equal(edict_choices),
    //         hidden_counts,
    //         vector_sizes,
    //         self.allocator,
    //     );
    //
    //     let next = self.allocator.alloc_slice_fill_with(
    //         RevealIndex::sabotage_phase_count(sabotage_statuses, seer_player, self.state.graveyard),
    //         |index| {
    //             let (sabotage_choices, revealed_creature) = RevealIndex(index)
    //                 .decode_sabotage_phase_reveal(
    //                     sabotage_statuses,
    //                     seer_player,
    //                     self.state.graveyard,
    //                 )
    //                 .unwrap();
    //             self.generate_seer(edict_choices, revealed_creature, sabotage_choices)
    //         },
    //     );
    //
    //     Scope::Explored(ExploredScope { matrices, next })
    // }
    // // }}}
    // // {{{ Seer phase
    // fn generate_seer(
    //     &mut self,
    //     edict_choices: Pair<Edict>,
    //     non_seer_player_creature: Creature,
    //     sabotage_choices: Pair<SabotagePhaseChoice>,
    // ) -> Scope<'a> {
    //     let hand_size = self.state.hand_size();
    //     let seer_active = self.state.seer_is_active();
    //     let seer_statuses = self.state.seer_statuses();
    //     let seer_player = self.state.forced_seer_player();
    //
    //     let mut graveyard = self.state.graveyard.clone();
    //     graveyard.add(non_seer_player_creature);
    //
    //     let vector_sizes = seer_player.order_as([if seer_active { 2 } else { 1 }, 1]);
    //
    //     let hidden_counts = seer_statuses.map(|status| {
    //         HiddenIndex::sabotage_seer_index_count_old_hand(hand_size, graveyard, status)
    //     });
    //
    //     let matrices = DecisionMatrices::new(
    //         self.state.is_symmetrical() && are_equal(sabotage_choices) && are_equal(edict_choices),
    //         hidden_counts,
    //         vector_sizes,
    //         self.allocator,
    //     );
    //
    //     let next = self.allocator.alloc_slice_fill_with(
    //         RevealIndex::seer_phase_count(graveyard),
    //         |index| {
    //             let seer_player_creature = RevealIndex(index)
    //                 .decode_seer_phase_reveal(graveyard)
    //                 .unwrap();
    //
    //             let creature_choices =
    //                 seer_player.order_as([seer_player_creature, non_seer_player_creature]);
    //
    //             let context = BattleContext {
    //                 main_choices: creature_choices
    //                     .zip(edict_choices)
    //                     .map(|(creatures, edict)| FinalMainPhaseChoice::new(creatures, edict)),
    //                 sabotage_choices,
    //                 state: self.state,
    //             };
    //
    //             match context.advance_known_state().1 {
    //                 TurnResult::Finished(score) => Scope::Completed(score),
    //                 TurnResult::Unfinished(state) => self.next_turn(state).generate_turn(),
    //             }
    //         },
    //     );
    //
    //     Scope::Explored(ExploredScope { matrices, next })
    // }
    // // }}}
}
// }}}
// {{{ Estimate
#[derive(Clone, Copy)]
pub struct EstimationContext {
    turns: usize,
    state: KnownState,
}

impl EstimationContext {
    pub fn new(turns: usize, state: KnownState) -> Self {
        Self { turns, state }
    }

    // {{{ Helpers
    fn next_turn(&self, state: KnownState) -> Self {
        Self {
            turns: self.turns - 1,
            state,
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

    /// Reveal indices are not yet optimized for symmetrical
    /// game states, so we fake it by dividing the number by this factor.
    fn symmetrical_reveal_factor(is_symmetrical: bool) -> usize {
        if is_symmetrical {
            2
        } else {
            1
        }
    }
    // }}}
    // {{{ Main phase
    fn estimate_main(&self) -> GenerationStats {
        let edicts = self.state.player_states.map(|s| s.edicts);

        let hand_size = self.state.hand_size();
        let seer_statuses = self.state.seer_statuses();

        let vector_sizes = edicts.zip(seer_statuses).map(|(edicts, seer_status)| {
            DecisionIndex::main_phase_index_count(edicts.len(), hand_size, seer_status)
        });

        let is_symmetrical = self.state.is_symmetrical();
        let hidden_count = HiddenIndex::main_index_count(hand_size, self.state.graveyard);
        let hidden_counts = [hidden_count; 2];
        let next_count =
            RevealIndex::main_phase_count(edicts) / Self::symmetrical_reveal_factor(is_symmetrical);
        let (slice_estimate, mut stats) = Self::estimate_slice_alloc(next_count, move |index| {
            let choice = RevealIndex(index).decode_main_phase_reveal(edicts).unwrap();
            self.estimate_sabotage(choice)
        });

        stats.main_total_next += next_count;
        stats.memory_estimate += slice_estimate;

        stats.memory_estimate +=
            DecisionMatrices::estimate_alloc(is_symmetrical, hidden_counts, vector_sizes);
        stats.main_total_weights +=
            DecisionMatrices::estimate_weight_storage(is_symmetrical, hidden_counts, vector_sizes);

        stats.main_count += 1;
        stats.main_total_hidden += hidden_count * 2;
        stats.main_total_decisions += vector_sizes[0] + vector_sizes[1];
        stats.explored_scopes += 1;
        stats
    }
    // }}}
    // {{{ Sabotage phase
    fn estimate_sabotage(&self, edict_choices: Pair<Edict>) -> GenerationStats {
        let hand_size = self.state.hand_size();
        let seer_statuses = self.state.seer_statuses();
        let seer_player = self.state.forced_seer_player();
        let sabotage_statuses = edict_choices.map(|e| e == Edict::Sabotage);

        let guess_count =
            DecisionIndex::sabotage_phase_index_count_old_hand(hand_size, self.state.graveyard);

        let vector_sizes = sabotage_statuses
            .map(|status| GenerationContext::sabotage_vector_size(status, guess_count));

        let hidden_counts = seer_statuses.map(|status| {
            HiddenIndex::sabotage_seer_index_count_old_hand(hand_size, self.state.graveyard, status)
        });

        let is_symmetrical = self.state.is_symmetrical() && are_equal(edict_choices);
        let reveal_count =
            RevealIndex::sabotage_phase_count(sabotage_statuses, seer_player, self.state.graveyard)
                / Self::symmetrical_reveal_factor(is_symmetrical);

        let (slice_estimate, mut stats) = Self::estimate_slice_alloc(reveal_count, move |index| {
            let (sabotage_choices, revealed_creature) = RevealIndex(index)
                .decode_sabotage_phase_reveal(sabotage_statuses, seer_player, self.state.graveyard)
                .unwrap();
            self.estimate_seer(edict_choices, revealed_creature, sabotage_choices)
        });

        stats.sabotage_total_next += reveal_count;
        stats.memory_estimate += slice_estimate;

        stats.memory_estimate +=
            DecisionMatrices::estimate_alloc(is_symmetrical, hidden_counts, vector_sizes);
        stats.main_total_weights +=
            DecisionMatrices::estimate_weight_storage(is_symmetrical, hidden_counts, vector_sizes);

        stats.sabotage_count += 1;
        stats.sabotage_total_hidden += hidden_counts.iter().sum::<usize>();
        stats.sabotage_total_decisions += vector_sizes.iter().product::<usize>();
        stats.sabotage_total_weights +=
            hidden_counts[0] * vector_sizes[0] + hidden_counts[1] * vector_sizes[1];
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

        let vector_sizes = seer_player.order_as([if seer_active { 2 } else { 1 }, 1]);

        let hidden_counts = seer_statuses.map(|status| {
            HiddenIndex::sabotage_seer_index_count_old_hand(hand_size, graveyard, status)
        });

        let is_symmetrical =
            self.state.is_symmetrical() && are_equal(edict_choices) && are_equal(sabotage_choices);
        let reveal_count = RevealIndex::seer_phase_count(graveyard)
            / Self::symmetrical_reveal_factor(is_symmetrical);

        let (slice_estimate, mut stats) = Self::estimate_slice_alloc(reveal_count, |index| {
            let seer_player_creature = RevealIndex(index)
                .decode_seer_phase_reveal(graveyard)
                .unwrap();

            let creature_choices =
                seer_player.order_as([seer_player_creature, non_seer_player_creature]);

            let context = BattleContext {
                main_choices: creature_choices
                    .zip(edict_choices)
                    .map(|(creature, edict)| FinalMainPhaseChoice::new(creature, edict)),
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

        stats.memory_estimate +=
            DecisionMatrices::estimate_alloc(is_symmetrical, hidden_counts, vector_sizes);
        stats.main_total_weights +=
            DecisionMatrices::estimate_weight_storage(is_symmetrical, hidden_counts, vector_sizes);

        stats.seer_total_next += reveal_count;
        stats.seer_count += 1;
        stats.seer_total_hidden += hidden_counts.iter().sum::<usize>();
        stats.seer_total_decisions += vector_sizes.iter().product::<usize>();
        stats
    }
    // }}}
}
// }}}
