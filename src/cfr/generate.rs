use super::{
    decision::{DecisionMatrices, DecisionMatrix, ExploredScope, Scope, UnexploredScope},
    decision_index::DecisionIndex,
    hidden_index::HiddenIndex,
    reveal_index::RevealIndex,
};
use crate::{
    game::{
        choice::{FinalMainPhaseChoice, SabotagePhaseChoice},
        creature::Creature,
        edict::Edict,
        known_state::KnownState,
        simulate::BattleContext,
        status_effect::StatusEffect,
        types::{Player, TurnResult},
    },
    helpers::{swap::Pair, bitfield::Bitfield},
};
use bumpalo::Bump;
use std::todo;

pub struct GenerationContext<'a> {
    turns: usize,
    state: KnownState,
    allocator: &'a Bump,
}

impl<'a> GenerationContext<'a> {
    pub fn generate(&self) -> Scope<'a> {
        if self.turns == 0 {
            return Scope::Unexplored(UnexploredScope { state: self.state });
        }

        self.generate_main()
    }

    // {{{ Helpers
    /// Computes the size of the hand in the current state.
    fn hand_size(&self) -> usize {
        5 - self.state.graveyard.len() / 2
    }

    /// Returns a tuple specifying whether each player has the seer effect active.
    fn seer_statuses(&self) -> Pair<bool> {
        (
            self.state.player_states.0.effects.has(StatusEffect::Seer),
            self.state.player_states.1.effects.has(StatusEffect::Seer),
        )
    }

    /// Picks a player to reveal their creature last.
    /// If the seer effect is not active, this is arbitrary.
    fn seer_player(&self) -> Player {
        self.state.seer_player().unwrap_or(Player::Me)
    }

    // }}}
    // {{{ Main phase
    fn generate_main(&self) -> Scope<'a> {
        let edicts = (
            self.state.player_states.0.edicts,
            self.state.player_states.1.edicts,
        );

        let hand_size = self.hand_size();
        let seer_statuses = self.seer_statuses();

        let vector_sizes = (
            DecisionIndex::main_phase_index_count(edicts.0.len(), hand_size, seer_statuses.0),
            DecisionIndex::main_phase_index_count(edicts.1.len(), hand_size, seer_statuses.1),
        );

        let hidden_count = HiddenIndex::main_index_count(hand_size, self.state.graveyard);
        let matrices = DecisionMatrices::new(
            DecisionMatrix::new(hidden_count, vector_sizes.0, self.allocator),
            DecisionMatrix::new(hidden_count, vector_sizes.1, self.allocator),
        );

        let next =
            self.allocator
                .alloc_slice_fill_with(RevealIndex::main_phase_count(edicts), |index| {
                    let choice = RevealIndex(index).decode_main_phase_reveal(edicts).unwrap();
                    self.generate_sabotage(choice)
                });

        Scope::Explored(ExploredScope { matrices, next })
    }
    // }}}
    // {{{ Sabotage phase
    fn sabotage_vector_size(choice: Edict, guess_count: usize) -> usize {
        if choice == Edict::Sabotage {
            guess_count
        } else {
            1
        }
    }

    fn generate_sabotage(&self, edict_choices: Pair<Edict>) -> Scope<'a> {
        let hand_size = self.hand_size();
        let seer_statuses = self.seer_statuses();

        let guess_count =
            DecisionIndex::sabotage_phase_index_count_old_hand(hand_size, self.state.graveyard);

        let vector_sizes = (
            Self::sabotage_vector_size(edict_choices.0, guess_count),
            Self::sabotage_vector_size(edict_choices.1, guess_count),
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
            DecisionMatrix::new(hidden_counts.0, vector_sizes.0, self.allocator),
            DecisionMatrix::new(hidden_counts.1, vector_sizes.1, self.allocator),
        );

        let next = self.allocator.alloc_slice_fill_with(
            RevealIndex::sabotage_phase_count(vector_sizes, self.state.graveyard),
            |index| {
                let (decision_indices, revealed_creature) = RevealIndex(index)
                    .decode_sabotage_phase_reveal(vector_sizes, self.state.graveyard)
                    .unwrap();

                self.generate_seer(edict_choices, revealed_creature, todo!())
            },
        );

        Scope::Explored(ExploredScope { matrices, next })
    }
    // }}}
    // {{{ Seer phase
    fn generate_seer(
        &self,
        edict_choices: Pair<Edict>,
        non_seer_player_creature: Creature,
        sabotage_choices: Pair<SabotagePhaseChoice>,
    ) -> Scope<'a> {
        let hand_size = self.hand_size();
        let seer_active = self.state.seer_is_active();
        let seer_statuses = self.seer_statuses();
        let seer_player = self.seer_player();

        let mut graveyard = self.state.graveyard.clone();
        graveyard.add(non_seer_player_creature);

        let vector_sizes = seer_player.order_as((if seer_active { 2 } else { 1 }, 1));

        let hidden_counts = (
            HiddenIndex::sabotage_seer_index_count_old_hand(hand_size, graveyard, seer_statuses.0),
            HiddenIndex::sabotage_seer_index_count_old_hand(hand_size, graveyard, seer_statuses.1),
        );

        let matrices = DecisionMatrices::new(
            DecisionMatrix::new(hidden_counts.0, vector_sizes.0, self.allocator),
            DecisionMatrix::new(hidden_counts.1, vector_sizes.1, self.allocator),
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

                match context.advance_known_state().1 {
                    TurnResult::Finished(score) => Scope::Completed(score),
                    TurnResult::Unfinished(mut state) => {
                        state.graveyard.add(non_seer_player_creature);
                        state.graveyard.add(seer_player_creature);

                        let generator = Self {
                            state,
                            turns: self.turns - 1,
                            allocator: self.allocator,
                        };

                        generator.generate_main()
                    }
                }
            },
        );

        Scope::Explored(ExploredScope { matrices, next })
    }
    // }}}
}
