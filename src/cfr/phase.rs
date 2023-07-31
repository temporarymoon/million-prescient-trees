use super::decision_index::DecisionIndex;
use super::hidden_index::{self, HiddenIndex, PerPhaseInfo};
use super::reveal_index::RevealIndex;
use crate::game::choice::{FinalMainPhaseChoice, SabotagePhaseChoice};
use crate::game::creature::{Creature, CreatureSet};
use crate::game::edict::Edict;
use crate::game::known_state::KnownState;
use crate::game::known_state_summary::{KnownStateEssentials, KnownStateSummary};
use crate::game::simulate::BattleContext;
use crate::game::types::{Player, TurnResult};
use crate::helpers::bitfield::Bitfield;
use crate::helpers::itertools::{ArrayUnzip, Itercools};
use crate::helpers::pair::{are_equal, Pair};
use crate::helpers::try_from_iter::TryCollect;
use derive_more::{Add, AddAssign, Sum};
use indicatif::HumanBytes;
use itertools::Itertools;
use std::fmt::Debug;
use std::mem::size_of;
use std::{format, todo};

// {{{ Phase tags
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PhaseTag {
    Main,
    Sabotage,
    Seer,
}

impl PhaseTag {
    pub const PHASES: [PhaseTag; 3] = [PhaseTag::Main, PhaseTag::Sabotage, PhaseTag::Seer];
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
    fn reveal_count(&self, state: &KnownState) -> usize;
    fn hidden_counts(&self, state: &KnownState) -> Pair<usize> {
        Player::PLAYERS.map(|player| HiddenIndex::count(state, player, Self::TAG))
    }

    fn valid_hidden_states(
        &self,
        state: KnownStateSummary,
    ) -> impl Iterator<Item = Pair<hidden_index::EncodingInfo>>;

    fn advance_hidden_indices(
        &self,
        state: &KnownStateSummary,
        hidden: Pair<hidden_index::HiddenState>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<(Pair<hidden_index::EncodingInfo>, RevealIndex)>;
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
        Player::PLAYERS.map(|player| DecisionIndex::main_phase_index_count(state, player))
    }

    // We offer a more performant implementation than the default one,
    // which makes use of the fact that during the main phase,
    // both players have the same number of possible hidden states!
    fn hidden_counts(&self, state: &KnownState) -> Pair<usize> {
        let count = HiddenIndex::count(state, Player::Me, Self::TAG);

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
    ) -> impl Iterator<Item = Pair<hidden_index::EncodingInfo>> {
        let possibilities = !state.graveyard;
        let hand_size = state.hand_size();

        possibilities
            .subsets_of_size(hand_size)
            .dependent_cartesian_pair_product(move |my_hand| {
                (possibilities - my_hand).subsets_of_size(hand_size)
            })
            .map(move |hands| hands.map(|hand| hidden_index::PerPhaseInfo::Main(hand)))
    }

    fn advance_hidden_indices(
        &self,
        state: &KnownStateSummary,
        hidden: Pair<hidden_index::HiddenState>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<(Pair<hidden_index::EncodingInfo>, RevealIndex)> {
        let (creature_choices, edicts) = Player::PLAYERS
            .try_map(|player| {
                player.select(decisions).decode_main_phase_index(
                    state,
                    player,
                    player.select(hidden).hand,
                )
            })?
            .unzip();

        let hidden_info = Player::PLAYERS.map(|player| {
            PerPhaseInfo::Sabotage(player.select(hidden).hand, player.select(creature_choices))
        });

        let reveal_index = RevealIndex::encode_main_phase_reveal(edicts, state.edict_sets());

        Some((hidden_info, reveal_index))
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

    fn sabotage_status(&self, player: Player) -> bool {
        player.select(self.edict_choices) == Edict::Sabotage
    }

    fn sabotage_statuses(&self) -> Pair<bool> {
        self.edict_choices.map(|e| e == Edict::Sabotage)
    }
}

impl Phase for SabotagePhase {
    type Next = SeerPhase;

    const TAG: PhaseTag = PhaseTag::Sabotage;

    fn is_symmetrical(&self) -> bool {
        are_equal(self.edict_choices)
    }

    fn decision_counts(&self, state: &KnownState) -> Pair<usize> {
        Player::PLAYERS.map(|player| {
            let status = self.sabotage_status(player);
            DecisionIndex::sabotage_phase_index_count(state, status)
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

    fn valid_hidden_states(
        &self,
        state: KnownStateSummary,
    ) -> impl Iterator<Item = Pair<hidden_index::EncodingInfo>> {
        MainPhase::new()
            .valid_hidden_states(state)
            .flat_map(move |info_pairs| {
                let [a, b] = Player::PLAYERS.map(|player| {
                    let hand = player.select(info_pairs).get_main();

                    hand.subsets_of_size(state.creature_choice_size(player))
                        .map(move |choice| PerPhaseInfo::Sabotage(hand, choice))
                });

                a.cartesian_pair_product(b)
            })
    }

    fn advance_hidden_indices(
        &self,
        state: &KnownStateSummary,
        hidden: Pair<hidden_index::HiddenState>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<(Pair<hidden_index::EncodingInfo>, RevealIndex)> {
        let guesses = Player::PLAYERS.try_map(|player| {
            player.select(decisions).decode_sabotage_index(
                state,
                player.select(hidden).hand,
                self.sabotage_status(player),
            )
        })?;

        let choices = hidden.try_map(|h| h.choice)?;
        let revealed = state
            .forced_seer_player()
            .select(choices)
            .into_iter()
            .exactly_one()
            .ok()?;

        let hidden_info = Player::PLAYERS.try_map(|player| {
            let hand = player.select(hidden).hand;
            let choice = player.select(choices);

            Some(PerPhaseInfo::Seer(hand, choice, revealed))
        })?;

        let reveal_index = RevealIndex::encode_sabotage_phase_reveal(
            guesses,
            state.forced_seer_player(),
            revealed,
            state.graveyard(),
        );

        Some((hidden_info, reveal_index))
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
    ) -> impl Iterator<Item = Pair<hidden_index::EncodingInfo>> {
        let seer_player = state.forced_seer_player();
        let revealed_creature = self.revealed_creature;

        SabotagePhase::new(self.edict_choices)
            .valid_hidden_states(state)
            .flat_map(move |info_pairs| {
                let seer_player_hand = seer_player.select(info_pairs).get_main();

                let seer_player_infos = seer_player_hand
                    .subsets_of_size(state.creature_choice_size(seer_player))
                    .map(move |choice| {
                        PerPhaseInfo::Seer(seer_player_hand, choice, revealed_creature)
                    });

                let non_seer_player_hand = (!seer_player).select(info_pairs).get_main();
                let non_seer_player_info = PerPhaseInfo::Seer(
                    non_seer_player_hand,
                    CreatureSet::singleton(revealed_creature),
                    revealed_creature,
                );

                seer_player_infos.map(move |o| [o, non_seer_player_info])
            })
            .map(move |indices| seer_player.order_as(indices))
    }

    fn advance_hidden_indices(
        &self,
        _state: &KnownStateSummary,
        _hidden: Pair<hidden_index::HiddenState>,
        _decisions: Pair<DecisionIndex>,
    ) -> Option<(Pair<hidden_index::EncodingInfo>, RevealIndex)> {
        todo!()
    }
}
// }}}
// }}}
