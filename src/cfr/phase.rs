use super::decision_index::DecisionIndex;
use super::hidden_index::HiddenIndex;
use super::reveal_index::RevealIndex;
use crate::game::choice::{FinalMainPhaseChoice, SabotagePhaseChoice};
use crate::game::creature::{Creature, CreatureSet};
use crate::game::edict::Edict;
use crate::game::known_state::KnownState;
use crate::game::known_state_summary::{KnownStateEssentials, KnownStateSummary};
use crate::game::simulate::BattleContext;
use crate::game::types::{Player, TurnResult};
use crate::helpers::bitfield::Bitfield;
use crate::helpers::itertools::Itercools;
use crate::helpers::pair::{are_equal, Pair};
use crate::helpers::try_from_iter::TryCollect;
use derive_more::{Add, AddAssign, Sum};
use indicatif::HumanBytes;
use std::fmt::Debug;
use std::mem::size_of;
use std::{format, todo};

// {{{ Phase tags
#[derive(Copy, Clone)]
pub enum PhaseTag {
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
// {{{ The Phase trait
pub trait Phase: Sync {
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

    fn valid_hidden_states(
        &self,
        state: KnownStateSummary,
    ) -> impl Iterator<Item = Pair<HiddenIndex>>;

    fn advance_hidden_indices(
        &self,
        state: &KnownStateSummary,
        hidden: Pair<HiddenIndex>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<(Pair<HiddenIndex>, RevealIndex)>;
}
// }}}
// {{{ Phase instances
// {{{ Main phase
pub struct MainPhase;

impl MainPhase {
    pub fn new() -> Self {
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
            .iter()
            .zip(seer_statuses)
            .map(|(edicts, seer_status)| {
                DecisionIndex::main_phase_index_count(edicts.len(), hand_size, seer_status)
            })
            .attempt_collect()
            .unwrap()
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
        // Sanity check
        for player in Player::PLAYERS {
            debug_assert!(state.player_edicts(player).len() >= 5 - state.battlefields.current);
        }

        let edict_choices = reveal_index
            .decode_main_phase_reveal(state.edict_sets())
            .unwrap();

        TurnResult::Unfinished((SabotagePhase::new(edict_choices), *state))
    }

    fn valid_hidden_states(
        &self,
        state: KnownStateSummary,
    ) -> impl Iterator<Item = Pair<HiddenIndex>> {
        let possibilities = !state.graveyard;
        let graveyard = state.graveyard;
        let hand_size = state.hand_size();

        possibilities
            .subsets_of_size(hand_size)
            .dependent_cartesian_pair_product(move |my_hand| {
                (possibilities - my_hand).subsets_of_size(hand_size)
            })
            .map(move |hands| hands.map(|hand| HiddenIndex::encode_main_index(hand, graveyard)))
    }

    fn advance_hidden_indices(
        &self,
        state: &KnownStateSummary,
        hidden: Pair<HiddenIndex>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<(Pair<HiddenIndex>, RevealIndex)> {
        let mut graveyard = state.graveyard;
        let mut hands = hidden.try_map(|h| h.decode_main_index(graveyard, state.hand_size()))?;
        let edict_sets = state.edict_sets();

        let decisions = Player::PLAYERS.try_map(|player| {
            player.select(decisions).decode_main_phase_index(
                player.select(edict_sets),
                player.select(hands),
                state.seer_is_active(),
            )
        })?;

        let creature_choices = decisions.map(|i| i.0.as_creature_set());
        let edicts = decisions.map(|i| i.1);
        let reveal_index = RevealIndex::encode_main_phase_reveal(edicts, edict_sets);

        for (i, played) in creature_choices.iter().enumerate() {
            graveyard |= *played;
            hands[i] -= *played;
        }

        let hidden_indices = Player::PLAYERS.map(|player| {
            HiddenIndex::encode_sabotage_seer_index(
                state,
                player,
                player.select(hands),
                player.select(creature_choices),
            )
        });

        Some((hidden_indices, reveal_index))
    }
}
// }}}
// {{{ Sabotage phase
pub struct SabotagePhase {
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
        Player::PLAYERS.map(|player| HiddenIndex::sabotage_seer_index_count(state, player))
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

    fn valid_hidden_states(
        &self,
        state: KnownStateSummary,
    ) -> impl Iterator<Item = Pair<HiddenIndex>> {
        let possibilities = !state.graveyard;
        let hand_size = state.hand_size();
        let seer_statuses = state.seer_statuses();

        possibilities
            .subsets_of_size(hand_size)
            .dependent_cartesian_pair_product(move |my_hand| {
                (possibilities - my_hand).subsets_of_size(hand_size)
            })
            .flat_map(move |hands| {
                let [mine, yours]: Pair<_> = hands
                    .iter()
                    .copied()
                    .zip(seer_statuses)
                    .zip(Player::PLAYERS)
                    .map(move |((hand, status), player)| {
                        hand.subsets_of_size(1 + <usize>::from(status))
                            .map(move |choice| {
                                HiddenIndex::encode_sabotage_seer_index(
                                    &state, player, hand, choice,
                                )
                            })
                    })
                    .attempt_collect()
                    .unwrap();

                mine.cartesian_pair_product(yours)
            })
    }

    fn advance_hidden_indices(
        &self,
        state: &KnownStateSummary,
        hidden: Pair<HiddenIndex>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<(Pair<HiddenIndex>, RevealIndex)> {
        let edict_sets = state.edict_sets();
        let mut graveyard = state.graveyard;
        let mut hands = Player::PLAYERS.try_map(|player| {
            player
                .select(hidden)
                .decode_sabotage_seer_index(state, player)
        })?;

        // let decisions: [_; 2] = decisions
        //     .iter()
        //     .zip(edict_sets)
        //     .zip(hands)
        //     .map(|((decision, edicts), hand)| {
        //         decision.decode_main_phase_index(edicts, hand, state.seer_is_active())
        //     })
        //     .attempt_opt_collect()?;
        //
        // let creature_choices = decisions.map(|i| i.0);
        // let edicts = decisions.map(|i| i.1);
        // let reveal_index = RevealIndex::encode_main_phase_reveal(edicts, edict_sets);
        //
        // for (i, creature_choice) in creature_choices.iter().enumerate() {
        //     let played = creature_choice.as_creature_set();
        //     graveyard |= played;
        //     hands[i] -= played;
        // }
        //
        // let hidden_indices = creature_choices
        //     .iter()
        //     .zip(hands)
        //     .map(|(creature_choice, hand)| {
        //         HiddenIndex::encode_sabotage_seer_index(*creature_choice, hand, graveyard)
        //     })
        //     .attempt_collect()
        //     .unwrap();
        //
        // Some((hidden_indices, reveal_index))
        todo!()
    }
}
// }}}
// {{{ Seer phase
pub struct SeerPhase {
    pub edict_choices: Pair<Edict>,
    pub sabotage_choices: Pair<SabotagePhaseChoice>,
    pub revealed_creature: Creature,
}

impl SeerPhase {
    pub fn new(
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

    fn graveyard(&self, mut graveyard: CreatureSet) -> CreatureSet {
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
        Player::PLAYERS
            .map(|player| {
                if player == state.forced_seer_player() {
                    HiddenIndex::sabotage_seer_index_count(state, player)
                } else {
                    HiddenIndex::main_index_count(
                        state.hand_size() - 1,
                        self.graveyard(state.graveyard),
                    )
                }
            })
            .attempt_collect()
            .unwrap()
    }

    fn reveal_count(&self, state: &KnownState) -> usize {
        RevealIndex::seer_phase_count(self.graveyard(state.graveyard))
    }

    fn advance_state(
        &self,
        state: &KnownState,
        reveal_index: RevealIndex,
    ) -> TurnResult<(Self::Next, KnownState)> {
        let seer_player_creature = reveal_index
            .decode_seer_phase_reveal(self.graveyard(state.graveyard))
            .unwrap();

        let main_choices = state
            .forced_seer_player()
            .order_as([seer_player_creature, self.revealed_creature])
            .iter()
            .zip(self.edict_choices)
            .map(|(creatures, edict)| FinalMainPhaseChoice::new(*creatures, edict))
            .attempt_collect()
            .unwrap();

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

    fn valid_hidden_states(
        &self,
        state: KnownStateSummary,
    ) -> impl Iterator<Item = Pair<HiddenIndex>> {
        let graveyard = self.graveyard(state.graveyard);
        let possibilities = !graveyard;
        let seer_player = state.forced_seer_player();
        let hand_size = state.hand_size();
        let seer_is_active = state.seer_is_active();

        // We start by arranging the players such that the seer player is first.
        possibilities
            .subsets_of_size(hand_size)
            .dependent_cartesian_product(move |my_hand| {
                (possibilities - my_hand).subsets_of_size(hand_size - 1)
            })
            .flat_map(move |(seer_player_hand, non_seer_player_hand)| {
                let seer_player_indices = seer_player_hand
                    .subsets_of_size(1 + <usize>::from(seer_is_active))
                    .map(move |choice| {
                        HiddenIndex::encode_sabotage_seer_index(
                            &state,
                            seer_player,
                            seer_player_hand,
                            choice,
                        )
                    });

                let non_seer_player_indices = std::iter::once(HiddenIndex::encode_main_index(
                    non_seer_player_hand,
                    graveyard,
                ));

                seer_player_indices.cartesian_pair_product(non_seer_player_indices)
            })
            .map(move |indices| seer_player.order_as(indices))
    }

    fn advance_hidden_indices(
        &self,
        _state: &KnownStateSummary,
        _hidden: Pair<HiddenIndex>,
        _decisions: Pair<DecisionIndex>,
    ) -> Option<(Pair<HiddenIndex>, RevealIndex)> {
        todo!()
    }
}
// }}}
// }}}
