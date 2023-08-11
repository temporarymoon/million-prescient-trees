use crate::game::creature::{Creature, CreatureSet};
use crate::game::edict::Edict;
use crate::game::known_state_summary::KnownStateEssentials;
use crate::game::types::Player;
use crate::helpers::bitfield::const_size_codec::ConstSizeCodec;
use crate::helpers::bitfield::Bitfield;
use crate::helpers::choose::choose;
use crate::helpers::ranged::MixRanged;
use itertools::Itertools;

/// Used to index decision vectors.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Default)]
pub struct DecisionIndex(pub usize);

impl DecisionIndex {
    // {{{ Main phase
    /// Encodes a main phase user choice into a decision index.
    pub fn encode_main_phase_index<S: KnownStateEssentials>(
        state: &S,
        player: Player,
        hand: CreatureSet,
        creatures: CreatureSet,
        edict: Edict,
    ) -> Option<DecisionIndex> {
        let creature_choice = creatures.encode_ones_relative_to(hand);

        Some(DecisionIndex(
            creature_choice.mix_indexof(edict, state.player_edicts(player))?,
        ))
    }

    /// Decodes a main phase user choice into a decision index.
    pub fn decode_main_phase_index<S: KnownStateEssentials>(
        self,
        state: &S,
        player: Player,
        hand: CreatureSet,
    ) -> Option<(CreatureSet, Edict)> {
        assert_eq!(hand.len(), state.hand_size());

        let (encoded_creatures, edict) = self.0.unmix_indexof(state.player_edicts(player))?;
        let creature_choice = CreatureSet::decode_ones_relative_to(
            encoded_creatures,
            state.creature_choice_size(player),
            hand,
        )?;

        Some((creature_choice, edict))
    }

    /// One more than the maximum value of `encode_main_phase_index`.
    #[inline(always)]
    pub fn main_phase_index_count<S: KnownStateEssentials>(state: &S, player: Player) -> usize {
        let choice_count = choose(state.hand_size(), state.creature_choice_size(player));
        let edict_count = state.player_edicts(player).len();

        choice_count * edict_count
    }
    // }}}
    // {{{ Sabotage phase
    /// Computes a bitfield of all the allowed choices for a sabotage guess.
    /// We only guess things which are:
    /// - not in our hand
    /// - not in the graveyard
    #[inline(always)]
    fn sabotage_decision_possibilities(hand: CreatureSet, graveyard: CreatureSet) -> CreatureSet {
        !(hand | graveyard)
    }

    /// Encodes a decision we can take during the sabotage phase.
    /// Assumes we know the hidden information of the current player.
    pub fn encode_sabotage_index<S: KnownStateEssentials>(
        state: &S,
        hand: CreatureSet,
        guess: Option<Creature>,
    ) -> Self {
        match guess {
            Some(guess) => {
                let possibilities = Self::sabotage_decision_possibilities(hand, state.graveyard());
                Self(CreatureSet::singleton(guess).encode_ones_relative_to(possibilities))
            }
            None => Self(0),
        }
    }

    /// Inverse of `encode_sabotage_index`.
    pub fn decode_sabotage_index<S: KnownStateEssentials>(
        self,
        state: &S,
        hand: CreatureSet,
        sabotage_status: bool,
    ) -> Option<Option<Creature>> {
        let result = if sabotage_status {
            let possibilities = Self::sabotage_decision_possibilities(hand, state.graveyard());

            let creature = CreatureSet::decode_ones_relative_to(self.0, 1, possibilities)?
                .into_iter()
                .exactly_one()
                .ok()?;

            Some(creature)
        } else {
            assert_eq!(self.0, 0);
            None
        };

        Some(result)
    }

    /// One more than the maximum value of `encode_sabotage_phase_index`.
    #[inline(always)]
    pub fn sabotage_phase_index_count<S: KnownStateEssentials>(
        state: &S,
        sabotage_status: bool,
    ) -> usize {
        if sabotage_status {
            (!state.graveyard()).len() - state.hand_size()
        } else {
            1
        }
    }
    // }}}
    // {{{ Seer phase
    /// Encodes a decision we can take during the seer phase.
    /// Assumes we know the hidden information of the current player.
    pub fn encode_seer_index(creatures: CreatureSet, choice: Creature) -> Option<Self> {
        creatures.indexof(choice).map(Self)
    }

    /// Inverse of `encode_seer_index`.
    pub fn decode_seer_index(self, creatures: CreatureSet) -> Option<Creature> {
        creatures.index(self.0)
    }

    /// One more than the maximum value of `encode-seer_index`
    pub fn seer_index_count(creatures: CreatureSet) -> usize {
        creatures.len()
    }
    // }}}
}

impl From<usize> for DecisionIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

// {{{ Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::edict::EdictSet;
    use crate::game::known_state_summary::KnownStateSummary;
    use std::{assert_eq, iter};

    // {{{ Main phase
    #[test]
    fn encode_decode_main_inverses_seer() {
        for edicts in EdictSet::members() {
            if edicts.len() == 0 {
                continue;
            }

            for graveyard in CreatureSet::members() {
                let player = Player::Me;

                for seer_player in [None, Some(player)] {
                    // We don't care about what edicts the opponent has,
                    // so we give them the same ones we have.
                    let state = KnownStateSummary::new([edicts; 2], graveyard, seer_player);
                    let choice_size = state.creature_choice_size(player);

                    if state.hand_size() < choice_size {
                        continue;
                    }

                    for hand in (!graveyard).subsets_of_size(state.hand_size()) {
                        let mut found_max = false;

                        for (creatures, edict) in
                            hand.subsets_of_size(choice_size).cartesian_product(edicts)
                        {
                            let encoded = DecisionIndex::encode_main_phase_index(
                                &state, player, hand, creatures, edict,
                            )
                            .unwrap();

                            let decoded = encoded.decode_main_phase_index(&state, player, hand);
                            let count = DecisionIndex::main_phase_index_count(&state, player);

                            assert_eq!(decoded, Some((creatures, edict)));
                            assert!(encoded.0 < count);

                            if encoded.0 + 1 == count {
                                found_max = true;
                            }
                        }

                        assert!(found_max);
                    }
                }
            }
        }
    }
    // }}}
    // {{{ Sabotage phase
    #[test]
    fn encode_decode_sabotage_inverses() {
        for graveyard in CreatureSet::members() {
            let player = Player::Me;
            let state = KnownStateSummary::new(Default::default(), graveyard, None);
            let choice_size = state.creature_choice_size(player);

            if state.hand_size() < choice_size {
                continue;
            }

            for hand in (!graveyard).subsets_of_size(state.hand_size()) {
                let mut found_max = false;

                for guess in DecisionIndex::sabotage_decision_possibilities(hand, graveyard)
                    .into_iter()
                    .map(Some)
                    .chain(iter::once(None))
                {
                    let encoded = DecisionIndex::encode_sabotage_index(&state, hand, guess);

                    let decoded = encoded.decode_sabotage_index(&state, hand, guess.is_some());
                    let count = DecisionIndex::sabotage_phase_index_count(&state, guess.is_some());

                    assert_eq!(decoded, Some(guess));
                    assert!(encoded.0 < count);

                    if encoded.0 + 1 == count {
                        found_max = true;
                    }
                }

                assert!(found_max);
            }
        }
    }
    // }}}
    // {{{ Seer phase
    #[test]
    fn encode_decode_seer_inverses() {
        let pairs = CreatureSet::all().subsets_of_size(2);
        let single = CreatureSet::all().subsets_of_size(1);

        for creatures in pairs.chain(single) {
            for result in Creature::CREATURES {
                let expected = if creatures.has(result) {
                    Some(result)
                } else {
                    None
                };

                let encoded = DecisionIndex::encode_seer_index(creatures, result);

                assert_eq!(
                    encoded.and_then(|e| e.decode_seer_index(creatures)),
                    expected
                );

                if let Some(encoded) = encoded {
                    assert!(encoded.0 < DecisionIndex::seer_index_count(creatures));
                }
            }
        }
    }
    // }}}
}
// }}}
