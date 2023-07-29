use std::debug_assert_eq;

use crate::game::creature::CreatureSet;
use crate::game::creature_choice::{CreatureChoice, UserCreatureChoice};
use crate::helpers::bitfield::const_size_codec::ConstSizeCodec;
use crate::helpers::bitfield::Bitfield;
use crate::helpers::choose::{self, choose};
use crate::helpers::ranged::MixRanged;

/// Encodes all hidden information known by a player.
///
/// *Important semantics*:
/// - the creature choice must not be in the hand during the sabotage/seer phase
/// - revealing a creature instantly adds it to the graveyard as well.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct HiddenIndex(pub usize);

type HandContentIndex = usize;

impl HiddenIndex {
    // {{{ Hand contents
    /// Encode the contents of the hand in a single integer.
    /// Removes any information regarding hand size and
    /// graveyard content from the resulting integer.
    fn encode_hand_contents(hand: CreatureSet, possibilities: CreatureSet) -> HandContentIndex {
        hand.encode_relative_to(possibilities).encode_ones()
    }

    /// Inverse of `encode_hand_contents`.
    pub fn decode_hand_contents(
        index: HandContentIndex,
        possibilities: CreatureSet,
        hand_size: usize,
    ) -> Option<CreatureSet> {
        CreatureSet::decode_relative_to(
            ConstSizeCodec::decode_ones(index, hand_size)?,
            possibilities,
        )
    }
    // }}}
    // {{{ Main phase
    /// Encodes all hidden informations known by a player during the main phase.
    #[inline(always)]
    pub fn encode_main_index(hand: CreatureSet, graveyard: CreatureSet) -> Self {
        Self(Self::encode_hand_contents(hand, !graveyard))
    }

    /// Inverse of `encode_main_index`.
    #[inline(always)]
    pub fn decode_main_index(
        self,
        graveyard: CreatureSet,
        hand_size: usize,
    ) -> Option<CreatureSet> {
        Self::decode_hand_contents(self.0, !graveyard, hand_size)
    }

    /// One more than the maximum value of `encode_main_index`
    #[inline(always)]
    pub fn main_index_count(hand_size: usize, graveyard: CreatureSet) -> usize {
        (!graveyard).hands_of_size(hand_size)
    }
    // }}}
    // {{{ Sabotage & seer phases
    /// Makes sure the given/decoded data regarding the sabotage/seer phase is valid.
    #[inline(always)]
    fn assure_valid_sabotage_seer_data(
        user_creature_choice: UserCreatureChoice,
        hand: CreatureSet,
        graveyard: CreatureSet,
    ) {
        let choice = user_creature_choice.as_creature_set();

        debug_assert_eq!(
            hand & graveyard,
            Default::default(),
            "The hand cannot conain cards from the graveyard"
        );

        debug_assert_eq!(
            hand & choice,
            Default::default(),
            "The chosen creatures must no longer be in the hand"
        );

        debug_assert_eq!(
            graveyard & choice,
            Default::default(),
            "The chosen creatures cannot yet be in the graveyard"
        );
    }

    /// Encodes all hidden informations known by a player during the sabotage or seer phases.
    /// The only information a player learns between the two is what creature the opponent has
    /// played, but this can be encoded by simply adding said creature to the graveyard.
    pub fn encode_sabotage_seer_index(
        user_creature_choice: UserCreatureChoice,
        hand: CreatureSet,
        graveyard: CreatureSet,
    ) -> Self {
        let hand_possibilites = !(graveyard);
        let choice_possibilites = !(graveyard | hand);

        Self::assure_valid_sabotage_seer_data(user_creature_choice, hand, graveyard);

        let hand_contents = Self::encode_hand_contents(hand, hand_possibilites);
        let encoded_choice =
            CreatureChoice::encode_user_choice(user_creature_choice, choice_possibilites);
        let max_choice_value = choice_possibilites.hands_of_size(user_creature_choice.len());
        let encoded = hand_contents.mix_ranged(encoded_choice.0, max_choice_value);

        Self(encoded)
    }

    /// Inverse of `encode_sabotage_index`
    pub fn decode_sabotage_seer_index(
        self,
        hand_size: usize,
        graveyard: CreatureSet,
        seer_active: bool,
    ) -> Option<(UserCreatureChoice, CreatureSet)> {
        let hand_possibilites = !(graveyard);

        let choice_size = UserCreatureChoice::len_from_status(seer_active);
        let max_choice_value = choose(
            11 - graveyard.len() - hand_size, // length of `choice_possibilities`
            choice_size,
        );

        let (encoded_hand, encoded_choice) = self.0.unmix_ranged(max_choice_value)?;
        let hand = Self::decode_hand_contents(encoded_hand, hand_possibilites, hand_size)?;

        let choice_possibilites = !(graveyard | hand);
        let user_creature_choice =
            CreatureChoice(encoded_choice).decode_user_choice(choice_possibilites, seer_active)?;

        Self::assure_valid_sabotage_seer_data(user_creature_choice, hand, graveyard);

        Some((user_creature_choice, hand))
    }

    /// One more than the maximum value of `encode_sabotage_seer_index`
    #[inline(always)]
    pub fn sabotage_seer_index_count(
        hand_size: usize,
        graveyard: CreatureSet,
        seer_active: bool,
    ) -> usize {
        // Intuitively speaking:
        // - We first pick the hand, giving us HP choose HS possibilities
        // - We then pick the choice from the remaining HP - HS cards,
        //   giving us (HP - HS) choose CL possibilities
        let hand_possibilites = !(graveyard);
        let hand_count = hand_possibilites.hands_of_size(hand_size);
        let choice_len = UserCreatureChoice::len_from_status(seer_active);
        let choice_count = choose(hand_possibilites.len() - hand_size, choice_len);

        choice_count * hand_count
    }

    /// Similar to `sabotage_seer_index_count` but accepts the size of the hand pre-main phase.
    #[inline(always)]
    pub fn sabotage_seer_index_count_old_hand(
        old_hand_size: usize,
        graveyard: CreatureSet,
        seer_active: bool,
    ) -> usize {
        Self::sabotage_seer_index_count(
            old_hand_size - UserCreatureChoice::len_from_status(seer_active),
            graveyard,
            seer_active,
        )
    }
    // }}}
}

// {{{ Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::creature::Creature;
    use std::assert_eq;

    // {{{ Main phase
    // We test for only the first 100 hand/graveyard configurations
    // (otherwise this would run too slow).
    #[test]
    fn hidden_encode_decode_main_inverses() {
        // hand
        for i in 0..=100 {
            // graveyard
            for j in 0..=100 {
                // Make sure no cards from therhand are in the graveyard.
                let i = i & !j;
                // Construct bitfields
                let graveyard = CreatureSet::new(j);
                let hand = CreatureSet::new(i);
                let encoded = HiddenIndex::encode_main_index(hand, graveyard);

                assert_eq!(encoded.decode_main_index(graveyard, hand.len()), Some(hand));

                assert!(encoded.0 < HiddenIndex::main_index_count(hand.len(), graveyard));
            }
        }
    }
    // }}}
    // {{{ Sabotage & seer phases
    #[test]
    fn hidden_encode_decode_sabotage_seer_inverses_seer() {
        // hand
        for i in 0..=100 {
            // graveyard
            for j in 0..=100 {
                // Make sure no cards from therhand are in the graveyard.
                let i = i & !j;

                // Construct bitfields
                let graveyard = CreatureSet::new(j);
                let hand = CreatureSet::new(i);

                // Generate creature choice
                for creature_one in Creature::CREATURES {
                    for creature_two in Creature::CREATURES {
                        if creature_one >= creature_two
                            || graveyard.has(creature_one)
                            || graveyard.has(creature_two)
                            || hand.has(creature_one)
                            || hand.has(creature_two)
                        {
                            continue;
                        };

                        let creature_choice = UserCreatureChoice(creature_one, Some(creature_two));
                        let encoded = HiddenIndex::encode_sabotage_seer_index(
                            creature_choice,
                            hand,
                            graveyard,
                        );

                        assert_eq!(
                            encoded.decode_sabotage_seer_index(hand.len(), graveyard, true),
                            Some((creature_choice, hand))
                        );

                        assert!(
                            encoded.0
                                < HiddenIndex::sabotage_seer_index_count(
                                    hand.len(),
                                    graveyard,
                                    true
                                )
                        );
                    }
                }
            }
        }
    }
    // }}}
}
// }}}
