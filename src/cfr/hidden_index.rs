use std::assert_eq;

use super::phase::PhaseTag;
use crate::game::creature::{Creature, CreatureSet};
use crate::game::known_state_summary::KnownStateEssentials;
use crate::game::types::Player;
use crate::helpers::bitfield::const_size_codec::ConstSizeCodec;
use crate::helpers::bitfield::Bitfield;
use crate::helpers::choose::choose;
use crate::helpers::ranged::MixRanged;

// {{{ PerPhaseInfo
/// Generic struct which holds a phase tag, and optionally:
/// - a `A` if `phase >= main`
/// - a `B` if `phase >= sabotage`
/// - a `C` if `phase >= seer`
#[derive(Debug, Clone, Copy)]
pub enum PerPhaseInfo<A, B, C> {
    Main(A),
    Sabotage(A, B),
    Seer(A, B, C),
}

impl<A, B, C> PerPhaseInfo<A, B, C> {
    #[inline(always)]
    pub fn tag(self) -> PhaseTag {
        match self {
            Self::Main(_) => PhaseTag::Main,
            Self::Sabotage(_, _) => PhaseTag::Sabotage,
            Self::Seer(_, _, _) => PhaseTag::Seer,
        }
    }

    #[inline(always)]
    pub fn forget_main(self) -> PerPhaseInfo<(), B, C> {
        match self {
            Self::Main(_) => PerPhaseInfo::Main(()),
            Self::Sabotage(_, b) => PerPhaseInfo::Sabotage((), b),
            Self::Seer(_, b, c) => PerPhaseInfo::Seer((), b, c),
        }
    }

    #[inline(always)]
    pub fn forget_sabotage(self) -> PerPhaseInfo<A, (), C> {
        match self {
            Self::Main(a) => PerPhaseInfo::Main(a),
            Self::Sabotage(a, _) => PerPhaseInfo::Sabotage(a, ()),
            Self::Seer(a, _, c) => PerPhaseInfo::Seer(a, (), c),
        }
    }

    #[inline(always)]
    pub fn is_post_main(self) -> bool {
        self.tag() != PhaseTag::Main
    }

    #[inline(always)]
    pub fn is_post_sabotage(self) -> bool {
        self.tag() == PhaseTag::Seer
    }

    #[inline(always)]
    pub fn get_main(self) -> A {
        match self {
            Self::Main(a) => a,
            Self::Sabotage(a, _) => a,
            Self::Seer(a, _, _) => a,
        }
    }

    #[inline(always)]
    pub fn get_sabotage(self) -> Option<B> {
        match self {
            Self::Main(_) => None,
            Self::Sabotage(_, b) => Some(b),
            Self::Seer(_, b, _) => Some(b),
        }
    }

    #[inline(always)]
    pub fn get_seer(self) -> Option<C> {
        match self {
            Self::Seer(_, _, c) => Some(c),
            _ => None,
        }
    }

    #[inline(always)]
    pub fn get_post_main(self) -> Option<(B, Option<C>)> {
        match self {
            Self::Main(_) => None,
            Self::Sabotage(_, b) => Some((b, None)),
            Self::Seer(_, b, c) => Some((b, Some(c))),
        }
    }

    #[inline(always)]
    pub fn get_pre_seer(self) -> (A, Option<B>) {
        match self {
            Self::Main(a) => (a, None),
            Self::Sabotage(a, b) => (a, Some(b)),
            Self::Seer(a, b, _) => (a, Some(b)),
        }
    }
}
// }}}
// {{{ HiddenIndex
/// Encodes all hidden information known by a player.
///
/// *Important semantics*:
/// - the creature choice must not be in the hand during the sabotage/seer phase
/// - revealing a creature instantly adds it to the graveyard as well.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct HiddenIndex(pub usize);

pub type EncodingInfo = PerPhaseInfo<CreatureSet, CreatureSet, Creature>;

impl From<usize> for HiddenIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl HiddenIndex {
    // {{{ Codec
    /// Returns true if a hidden index encoding under the given conditions
    /// would contain info about the current player's creature choices.
    fn index_contains_choice<S: KnownStateEssentials>(
        state: &S,
        player: Player,
        phase: PhaseTag,
    ) -> bool {
        match phase {
            PhaseTag::Main => false,
            PhaseTag::Sabotage => true,
            PhaseTag::Seer => player == state.forced_seer_player(),
        }
    }

    pub fn encode<S: KnownStateEssentials>(state: &S, player: Player, info: EncodingInfo) -> Self {
        let hand = info.get_main();
        let hand_possibilites = !state.graveyard() - CreatureSet::opt_singleton(info.get_seer());
        let irl_hand = hand - info.get_sabotage().unwrap_or_default();
        let encoded_hand = irl_hand.encode_ones_relative_to(hand_possibilites);

        if let Some((choice, revealed)) = info.get_post_main() {
            assert!(choice.is_subset_of(hand));

            match revealed {
                Some(revealed) if player != state.forced_seer_player() => {
                    assert_eq!(choice, CreatureSet::singleton(revealed));

                    encoded_hand.into()
                }
                _ => {
                    assert_eq!(choice.len(), state.creature_choice_size(player));

                    encoded_hand
                        .mix_subset(choice, hand_possibilites - irl_hand)
                        .into()
                }
            }
        } else {
            encoded_hand.into()
        }
    }

    pub fn decode<S: KnownStateEssentials>(
        self,
        state: &S,
        player: Player,
        info: PerPhaseInfo<(), (), Creature>,
    ) -> Option<(CreatureSet, Option<CreatureSet>)> {
        let hand_possibilites = !state.graveyard() - CreatureSet::opt_singleton(info.get_seer());
        let self_contains_choice = Self::index_contains_choice(state, player, info.tag());

        let irl_hand_size = state.hand_size_during(player, info.tag());
        let choice_size = state.creature_choice_size(player);

        let (encoded_hand, remaining) = if self_contains_choice {
            let max_choice_value = choose(
                hand_possibilites.len() - irl_hand_size, // length of `choice_possibilities`
                choice_size,
            );

            let (encoded_hand, remaining) = self.0.unmix_ranged(max_choice_value)?;

            (encoded_hand, Some(remaining))
        } else {
            (self.0, None)
        };

        let irl_hand =
            CreatureSet::decode_ones_relative_to(encoded_hand, irl_hand_size, hand_possibilites)?;

        let choice = if let Some(remaining) = remaining {
            let choice_possibilities = hand_possibilites - irl_hand;
            let decoded =
                CreatureSet::decode_ones_relative_to(remaining, choice_size, choice_possibilities)?;

            Some(decoded)
        } else {
            info.get_seer().map(|c| CreatureSet::singleton(c))
        };

        Some((irl_hand | choice.unwrap_or_default(), choice))
    }

    pub fn count<S: KnownStateEssentials>(state: &S, player: Player, phase: PhaseTag) -> usize {
        let mut hand_possibility_count = (!state.graveyard()).len();

        if phase == PhaseTag::Seer {
            hand_possibility_count -= 1;
        }

        let hand_size = state.hand_size_during(player, phase);
        let hand_count = choose(hand_possibility_count, hand_size);

        let choice_count = if Self::index_contains_choice(state, player, phase) {
            let choice_len = state.creature_choice_size(player);
            let choice_count = choose(hand_possibility_count - hand_size, choice_len);

            choice_count
        } else {
            1
        };

        hand_count * choice_count
    }
    // }}}
}
// }}}
// {{{ Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::known_state_summary::KnownStateSummary;
    use std::assert_eq;

    // {{{ Main phase
    #[test]
    fn hidden_encode_decode_main_inverses() {
        for graveyard in Bitfield::members() {
            let player = Player::Me;
            let state = KnownStateSummary::new_all_edicts(graveyard, Some(player));

            for hand in (!graveyard).subsets_of_size(state.hand_size()) {
                let info = PerPhaseInfo::Main(hand);
                let decoding_info = info.forget_main().forget_sabotage();
                let encoded = HiddenIndex::encode(&state, player, info);

                assert_eq!(
                    encoded.decode(&state, player, decoding_info),
                    Some(info.get_pre_seer())
                );

                let count = HiddenIndex::count(&state, player, info.tag());

                assert!(
                    encoded.0 < count,
                    "{} is bigger than the supposed count ({})",
                    encoded.0,
                    count
                );
            }
        }
    }
    // }}}
    // {{{ Sabotage phase
    #[test]
    fn hidden_encode_decode_sabotage_inverses() {
        for graveyard in Bitfield::members() {
            for seer_player in [None, Some(Player::Me), Some(Player::You)] {
                let player = Player::Me;
                let state = KnownStateSummary::new_all_edicts(graveyard, seer_player);

                for hand in (!graveyard).subsets_of_size(state.hand_size()) {
                    let choice_size = state.creature_choice_size(player);

                    if hand.len() < choice_size {
                        continue;
                    };

                    for choice in hand.subsets_of_size(choice_size) {
                        let info = PerPhaseInfo::Sabotage(hand, choice);
                        let decoding_info = info.forget_main().forget_sabotage();
                        let encoded = HiddenIndex::encode(&state, player, info);

                        assert_eq!(
                            encoded.decode(&state, player, decoding_info),
                            Some(info.get_pre_seer())
                        );

                        assert!(encoded.0 < HiddenIndex::count(&state, player, info.tag()));
                    }
                }
            }
        }
    }
    // }}}
    // {{{ Seer phase
    #[test]
    fn hidden_encode_decode_seer_inverses() {
        for graveyard in Bitfield::members() {
            for seer_player in [None, Some(Player::Me), Some(Player::You)] {
                let player = Player::Me;
                let state = KnownStateSummary::new_all_edicts(graveyard, seer_player);

                for hand in (!graveyard).subsets_of_size(state.hand_size()) {
                    let choice_size = state.creature_choice_size(player);

                    if hand.len() < choice_size {
                        continue;
                    };

                    for choice in hand.subsets_of_size(choice_size) {
                        let revealed_iter = if player == state.forced_seer_player() {
                            (!(graveyard | hand)).into_iter()
                        } else {
                            choice.into_iter()
                        };

                        for revealed in revealed_iter {
                            let info = PerPhaseInfo::Seer(hand, choice, revealed);
                            let decoding_info = info.forget_main().forget_sabotage();
                            let encoded = HiddenIndex::encode(&state, player, info);

                            assert_eq!(
                                encoded.decode(&state, player, decoding_info),
                                Some(info.get_pre_seer())
                            );

                            assert!(encoded.0 < HiddenIndex::count(&state, player, info.tag()));
                        }
                    }
                }
            }
        }
    }
    // }}}
}
// }}}
