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
use std::format;
use std::mem::size_of;

// {{{ Phase tags
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
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
pub trait Phase: Sync + Sized {
    type Next: Phase;

    const TAG: PhaseTag;
    const ADVANCES_TURN: bool = false;

    fn is_symmetrical(&self) -> bool;
    fn advance_phase<S: KnownStateEssentials>(
        &self,
        state: &S,
        reveal_index: RevealIndex,
    ) -> Option<Self::Next>;

    fn advance_state(
        &self,
        state: &KnownState,
        reveal_index: RevealIndex,
    ) -> TurnResult<KnownState>;

    fn battle_context(
        &self,
        _state: &KnownState,
        _reveal_index: RevealIndex,
    ) -> Option<BattleContext> {
        None
    }

    fn decision_counts(&self, state: &KnownState) -> Pair<usize>;
    fn reveal_count(&self, state: &KnownState) -> usize;
    fn hidden_counts<S: KnownStateEssentials>(&self, state: &S) -> Pair<usize> {
        Player::PLAYERS.map(|player| HiddenIndex::count(state, player, Self::TAG))
    }

    fn valid_hidden_states(
        &self,
        state: KnownStateSummary,
    ) -> impl Iterator<Item = Pair<hidden_index::EncodingInfo>>;

    fn advance_hidden_indices(
        &self,
        state: KnownStateSummary,
        hidden: Pair<hidden_index::HiddenState>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<(
        KnownStateSummary,
        Pair<hidden_index::EncodingInfo>,
        RevealIndex,
    )>;

    fn hidden_index_decoding_info(&self) -> hidden_index::DecodingInfo;

    /// Required by the per_phase! macro.
    #[inline(always)]
    fn pass_to<P>(self, f: impl FnOnce(Self) -> P) -> P {
        f(self)
    }
}
// }}}
// {{{ Phase instances
// {{{ Main phase
#[derive(Debug, Clone, Copy)]
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
    fn hidden_counts<S: KnownStateEssentials>(&self, state: &S) -> Pair<usize> {
        let count = HiddenIndex::count(state, Player::Me, Self::TAG);

        [count; 2]
    }

    fn reveal_count(&self, state: &KnownState) -> usize {
        RevealIndex::main_phase_count(state.edict_sets())
    }

    fn advance_phase<S: KnownStateEssentials>(
        &self,
        state: &S,
        reveal_index: RevealIndex,
    ) -> Option<Self::Next> {
        let edict_choices = reveal_index.decode_main_phase_reveal(state.edict_sets())?;

        Some(SabotagePhase::new(edict_choices))
    }

    fn advance_state(
        &self,
        state: &KnownState,
        _reveal_index: RevealIndex,
    ) -> TurnResult<KnownState> {
        // Sanity check
        for player in Player::PLAYERS {
            debug_assert!(state.player_edicts(player).len() >= 5 - state.battlefields.current);
        }

        TurnResult::Unfinished(*state)
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
        state: KnownStateSummary,
        hidden: Pair<hidden_index::HiddenState>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<(
        KnownStateSummary,
        Pair<hidden_index::EncodingInfo>,
        RevealIndex,
    )> {
        let (creature_choices, edicts) = Player::PLAYERS
            .try_map(|player| {
                player.select(decisions).decode_main_phase_index(
                    &state,
                    player,
                    player.select(hidden).hand,
                )
            })?
            .unzip();

        let hidden_info = Player::PLAYERS.map(|player| {
            PerPhaseInfo::Sabotage(player.select(hidden).hand, player.select(creature_choices))
        });

        let reveal_index = RevealIndex::encode_main_phase_reveal(edicts, state.edict_sets())?;

        Some((state, hidden_info, reveal_index))
    }

    fn hidden_index_decoding_info(&self) -> hidden_index::DecodingInfo {
        PerPhaseInfo::Main(())
    }
}
// }}}
// {{{ Sabotage phase
#[derive(Debug, Clone, Copy)]
pub struct SabotagePhase {
    pub edict_choices: Pair<Edict>,
}

impl SabotagePhase {
    fn new(edict_choices: Pair<Edict>) -> Self {
        Self { edict_choices }
    }

    /// Returns true if the given player has played the `Sabotage` edict.
    #[inline(always)]
    fn sabotage_status(&self, player: Player) -> bool {
        player.select(self.edict_choices) == Edict::Sabotage
    }

    /// Returns a pair where the element coresponding to some player is true
    /// if and only if `self.sabotage_status(player)`;
    #[inline(always)]
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
            state.last_creature_revealer(),
            state.graveyard,
        )
    }

    fn advance_phase<S: KnownStateEssentials>(
        &self,
        state: &S,
        reveal_index: RevealIndex,
    ) -> Option<Self::Next> {
        let (sabotage_choices, revealed_creature) = reveal_index.decode_sabotage_phase_reveal(
            self.sabotage_statuses(),
            state.last_creature_revealer(),
            state.graveyard(),
        )?;

        let next = SeerPhase::new(self.edict_choices, sabotage_choices, revealed_creature);

        Some(next)
    }

    fn advance_state(&self, state: &KnownState, _: RevealIndex) -> TurnResult<KnownState> {
        TurnResult::Unfinished(*state)
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
        state: KnownStateSummary,
        hidden: Pair<hidden_index::HiddenState>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<(
        KnownStateSummary,
        Pair<hidden_index::EncodingInfo>,
        RevealIndex,
    )> {
        let guesses = Player::PLAYERS.try_map(|player| {
            player.select(decisions).decode_sabotage_index(
                &state,
                player.select(hidden).hand,
                self.sabotage_status(player),
            )
        })?;

        let choices = hidden.try_map(|h| h.choice)?;
        let revealed = (!state.last_creature_revealer())
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
            state.last_creature_revealer(),
            revealed,
            state.graveyard(),
        )?;

        Some((state, hidden_info, reveal_index))
    }

    fn hidden_index_decoding_info(&self) -> hidden_index::DecodingInfo {
        PerPhaseInfo::Sabotage((), ())
    }
}
// }}}
// {{{ Seer phase
#[derive(Debug, Clone, Copy)]
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
}

impl Phase for SeerPhase {
    type Next = MainPhase;

    const ADVANCES_TURN: bool = true;
    const TAG: PhaseTag = PhaseTag::Seer;

    fn is_symmetrical(&self) -> bool {
        false
    }

    fn decision_counts(&self, state: &KnownState) -> Pair<usize> {
        state
            .seer_statuses()
            .map(|status| if status { 2 } else { 1 })
    }

    fn reveal_count(&self, state: &KnownState) -> usize {
        RevealIndex::seer_phase_count(state.graveyard)
    }

    fn advance_phase<S: KnownStateEssentials>(&self, _: &S, _: RevealIndex) -> Option<Self::Next> {
        Some(MainPhase::new())
    }

    fn battle_context(
        &self,
        state: &KnownState,
        reveal_index: RevealIndex,
    ) -> Option<BattleContext> {
        let seer_player_creature = reveal_index
            .decode_seer_phase_reveal(state.graveyard, self.revealed_creature)
            .unwrap();

        let main_choices = state
            .last_creature_revealer()
            .order_as([seer_player_creature, self.revealed_creature])
            .into_iter()
            .zip(self.edict_choices)
            .map(|(creatures, edict)| FinalMainPhaseChoice::new(creatures, edict))
            .attempt_collect()
            .unwrap();

        let context = BattleContext {
            main_choices,
            sabotage_choices: self.sabotage_choices,
            state: *state,
        };

        Some(context)
    }

    // TODO: this can fail!
    fn advance_state(
        &self,
        state: &KnownState,
        reveal_index: RevealIndex,
    ) -> TurnResult<KnownState> {
        match self
            .battle_context(state, reveal_index)
            .unwrap()
            .advance_known_state()
            .1
        {
            TurnResult::Finished(score) => TurnResult::Finished(score),
            TurnResult::Unfinished(state) => TurnResult::Unfinished(state),
        }
    }

    fn valid_hidden_states(
        &self,
        state: KnownStateSummary,
    ) -> impl Iterator<Item = Pair<hidden_index::EncodingInfo>> {
        let seer_player = state.last_creature_revealer();
        let revealed_creature = self.revealed_creature;
        let possibilities = !state.graveyard - revealed_creature;

        possibilities
            .subsets_of_size(state.hand_size())
            .dependent_cartesian_pair_product(move |my_hand| {
                (possibilities - my_hand).subsets_of_size(state.hand_size() - 1)
            })
            .flat_map(move |[seer_player_hand, non_seer_player_hand]| {
                let seer_player_infos = seer_player_hand
                    .subsets_of_size(state.creature_choice_size(seer_player))
                    .map(move |choice| {
                        PerPhaseInfo::Seer(seer_player_hand, choice, revealed_creature)
                    });

                let non_seer_player_info = PerPhaseInfo::Seer(
                    non_seer_player_hand + revealed_creature,
                    CreatureSet::singleton(revealed_creature),
                    revealed_creature,
                );

                seer_player_infos.map(move |o| seer_player.order_as([o, non_seer_player_info]))
            })
    }

    fn advance_hidden_indices(
        &self,
        state: KnownStateSummary,
        hidden: Pair<hidden_index::HiddenState>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<(
        KnownStateSummary,
        Pair<hidden_index::EncodingInfo>,
        RevealIndex,
    )> {
        let choices = hidden.try_map(|h| h.choice)?;
        let final_choices = Player::PLAYERS.try_map(|player| {
            player
                .select(decisions)
                .decode_seer_index(player.select(choices))
        })?;

        let hidden_info = Player::PLAYERS.try_map(|player| {
            let hand = player.select(hidden).hand;
            let final_choice = player.select(final_choices);

            Some(PerPhaseInfo::Main(hand - final_choice))
        })?;

        let reveal_index = RevealIndex::encode_seer_phase_reveal(
            state.last_creature_revealer().select(final_choices),
            state.graveyard(),
            self.revealed_creature,
        )?;

        let graveyard = {
            let mut result = state.graveyard;

            for creature in final_choices {
                result.insert(creature);
            }

            result
        };

        let edicts = Player::PLAYERS.map(|player| {
            let mut result = state.player_edicts(player);

            let [my_creature, your_creature] = player.order_as(final_choices);

            if my_creature == Creature::Steward && your_creature != Creature::Witch {
                result.fill();
            } else {
                result.remove(player.select(self.edict_choices));
            };

            result
        });

        let seer_player = Player::PLAYERS
            .into_iter()
            .filter(|player| {
                let [my_creature, your_creature] = player.order_as(final_choices);

                my_creature == Creature::Seer
                    && your_creature != Creature::Witch
                    && your_creature != Creature::Rogue
            })
            .exactly_one()
            .ok();

        let new_state = KnownStateSummary::new(edicts, graveyard, seer_player);

        Some((new_state, hidden_info, reveal_index))
    }

    fn hidden_index_decoding_info(&self) -> hidden_index::DecodingInfo {
        PerPhaseInfo::Seer((), (), self.revealed_creature)
    }
}
// }}}
// }}}
// {{{ Tests
#[cfg(test)]
mod tests {
    use super::{MainPhase, Phase, SabotagePhase, SeerPhase};
    use crate::cfr::hidden_index::{self, HiddenIndex, PerPhaseInfo};
    use crate::game::creature::CreatureSet;
    use crate::game::edict::EdictSet;
    use crate::game::known_state_summary::KnownStateSummary;
    use crate::game::types::Player;
    use crate::helpers::bitfield::Bitfield;
    use crate::helpers::itertools::Itercools;
    use crate::helpers::pair::Pair;
    use bumpalo::Bump;
    use itertools::Itertools;

    // {{{ `valid_hidden_states`
    /// Part of the next test!
    fn all_states_valid_sometimes_per_phase<P: Phase>(
        alloc: &mut Bump,
        phase: P,
        edict_sets: Pair<EdictSet>,
        seer_player: Option<Player>,
        graveyard: CreatureSet,
    ) {
        alloc.reset();
        let state = KnownStateSummary::new(edict_sets, graveyard, seer_player);

        let hidden_counts = phase.hidden_counts(&state);
        let default_value: (bool, hidden_index::DecodingInfo) = (false, PerPhaseInfo::Main(()));
        let hidden_index_trackers = [
            alloc.alloc_slice_fill_copy(hidden_counts[0], default_value),
            alloc.alloc_slice_fill_copy(hidden_counts[1], default_value),
        ];

        for infos in phase.valid_hidden_states(state) {
            let [left, right] = Player::PLAYERS
                .map(|player| HiddenIndex::encode(&state, player, player.select(infos)));

            hidden_index_trackers[0][left.0] = (true, infos[0].forget_main().forget_sabotage());
            hidden_index_trackers[1][right.0] = (true, infos[1].forget_main().forget_sabotage());
        }

        for player in Player::PLAYERS {
            let tracker = player.select_ref(&hidden_index_trackers);
            let counterexample = tracker.iter().find_position(|v| !v.0);
            let decoded = counterexample
                .map(|(index, (_, info))| HiddenIndex(index).decode(&state, player, *info));

            assert!(
                counterexample.is_none(),
                "Found a state that is not covered: {:?}",
                decoded
            );
        }
    }

    /// Checks that every possible hidden state is visited by `.valid_states` at least once.
    ///
    /// The search space is very big, so we only check a limited number of scenarios.
    /// Moreover, we use a bump allocator in order to improve test performance.
    #[test]
    fn all_states_valid_sometimes() {
        let mut alloc = Bump::with_capacity(1024);
        let seer_player_possiblities = [None, Some(Player::Me), Some(Player::You)];
        for edict_sets in EdictSet::members()
            .cartesian_pair_product(EdictSet::members())
            .take(60)
        {
            for (index, graveyard) in CreatureSet::all().subsets_of_size(4).take(60).enumerate() {
                // We cycle through these for performance reasons
                let seer_player = seer_player_possiblities[index % 3];

                let phase = MainPhase::new();
                all_states_valid_sometimes_per_phase(
                    &mut alloc,
                    phase,
                    edict_sets,
                    seer_player,
                    graveyard,
                );

                for edicts in edict_sets[0]
                    .into_iter()
                    .cartesian_pair_product(edict_sets[1])
                    .take(10)
                {
                    let phase = SabotagePhase::new(edicts);
                    all_states_valid_sometimes_per_phase(
                        &mut alloc,
                        phase,
                        edict_sets,
                        seer_player,
                        graveyard,
                    );

                    for creature in (!graveyard).into_iter().take(3) {
                        let phase = SeerPhase::new(edicts, [None; 2], creature);
                        all_states_valid_sometimes_per_phase(
                            &mut alloc,
                            phase,
                            edict_sets,
                            seer_player,
                            graveyard,
                        );
                    }
                }
            }
        }
    }
    // }}}
}
// }}}
// {{{ Some phase
#[derive(Debug, Clone, Copy)]
pub enum PerPhase<Main, Sabotage, Seer> {
    Main(Main),
    Sabotage(Sabotage),
    Seer(Seer),
}

pub type SomePhase = PerPhase<MainPhase, SabotagePhase, SeerPhase>;

impl<A, B, C> PerPhase<A, B, C> {
    #[inline(always)]
    pub fn tag(&self) -> PhaseTag {
        match self {
            Self::Main(_) => PhaseTag::Main,
            Self::Sabotage(_) => PhaseTag::Sabotage,
            Self::Seer(_) => PhaseTag::Seer,
        }
    }
}

macro_rules! per_phase {
    ($s: expr, $f:expr) => {
        match $s {
            Self::Main(inner) => inner.pass_to($f),
            Self::Sabotage(inner) => inner.pass_to($f),
            Self::Seer(inner) => inner.pass_to($f),
        }
    };
}

impl SomePhase {
    /// Calls the method with the same name on the underlying phase.
    pub fn hidden_index_decoding_info(&self) -> hidden_index::DecodingInfo {
        per_phase!(self, |inner| inner.hidden_index_decoding_info())
    }

    pub fn advance(
        &self,
        state: KnownState,
        hidden: Pair<hidden_index::HiddenState>,
        decisions: Pair<DecisionIndex>,
    ) -> Option<
        TurnResult<(
            KnownState,
            Pair<hidden_index::EncodingInfo>,
            RevealIndex,
            Self,
        )>,
    > {
        let summary = state.to_summary();
        let (next_summary, next_hidden, reveal_index) = per_phase!(self, |inner| inner
            .advance_hidden_indices(summary, hidden, decisions))?;

        let advanced_state = per_phase!(self, |inner| inner.advance_state(&state, reveal_index));
        let next_phase: Self = match self {
            Self::Main(inner) => Self::Sabotage(inner.advance_phase(&state, reveal_index)?),
            Self::Sabotage(inner) => Self::Seer(inner.advance_phase(&state, reveal_index)?),
            Self::Seer(inner) => Self::Main(inner.advance_phase(&state, reveal_index)?),
        };

        let result = advanced_state.map(|next_state| {
            assert_eq!(next_state.to_summary(), next_summary);

            (state, next_hidden, reveal_index, next_phase)
        });

        Some(result)
    }

    pub fn decision_counts(&self, state: &KnownState) -> Pair<usize> {
        per_phase!(self, |inner| inner.decision_counts(state))
    }
}
// }}}
