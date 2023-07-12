use crate::game::types::{CreatureChoice, CreatureSet, UserCreatureChoice};
use crate::helpers::bitfield::Bitfield;
use crate::helpers::ranged::MixRanged;

/// Used to index decision matrices.
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
        CreatureSet::decode_relative_to(Bitfield::decode_ones(index, hand_size)?, possibilities)
    }
    // }}}
    // {{{ Main phase
    /// Encodes all hidden informations known by a player during the main phase.
    #[inline]
    pub fn encode_main_index(hand: CreatureSet, graveyard: CreatureSet) -> Self {
        Self(Self::encode_hand_contents(hand, !graveyard))
    }

    /// Inverse of `encode_main_index`.
    #[inline]
    pub fn decode_main_index(
        self,
        graveyard: CreatureSet,
        hand_size: usize,
    ) -> Option<CreatureSet> {
        Self::decode_hand_contents(self.0, !graveyard, hand_size)
    }
    // }}}
    // {{{ Sabotage & seer phases
    /// Encodes all hidden informations known by a player during the sabotage or seer phases.
    /// The only information a player learns between the two is what creature the opponent has
    /// played, but this can be encoded by simply adding said creature to the graveyard.
    pub fn encode_sabotage_seer_index(
        user_creature_choice: UserCreatureChoice,
        hand: CreatureSet,
        graveyard: CreatureSet,
    ) -> Self {
        let possibilites = !graveyard;
        let hand_contents = Self::encode_hand_contents(hand, possibilites);
        let encoded_choice = CreatureChoice::encode_user_choice(user_creature_choice, possibilites);
        let max = possibilites.hands_of_size(user_creature_choice.len());
        let encoded = hand_contents.mix_ranged(encoded_choice.0, max);

        Self(encoded)
    }

    /// Inverse of `encode_sabotage_index`
    #[inline]
    pub fn decode_sabotage_seer_index(
        self,
        hand_size: usize,
        graveyard: CreatureSet,
        seer_active: bool,
    ) -> Option<(UserCreatureChoice, CreatureSet)> {
        let possibilites = !graveyard;
        let max = possibilites.hands_of_size(UserCreatureChoice::len_from_status(seer_active));
        let (hand_contents, encoded_choice) = self.0.unmix_ranged(max);
        let user_creature_choice =
            CreatureChoice(encoded_choice).decode_user_choice(possibilites, seer_active)?;
        let hand_contents = Self::decode_hand_contents(hand_contents, possibilites, hand_size)?;
        Some((user_creature_choice, hand_contents))
    }
    // }}}
}

// {{{ Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{game::types::Creature, helpers::bitfield::Bitfield};
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
                let graveyard = CreatureSet(Bitfield::new(j));
                let hand = CreatureSet(Bitfield::new(i));

                assert_eq!(
                    HiddenIndex::encode_main_index(hand, graveyard)
                        .decode_main_index(graveyard, hand.len()),
                    Some(hand)
                );
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
                let graveyard = CreatureSet(Bitfield::new(j));
                let hand = CreatureSet(Bitfield::new(i));

                // Generate creature choice
                for creature_one in Creature::CREATURES {
                    for creature_two in Creature::CREATURES {
                        if creature_one >= creature_two
                            || graveyard.has(creature_one)
                            || graveyard.has(creature_two)
                        {
                            continue;
                        };

                        let creature_choice = UserCreatureChoice(creature_one, Some(creature_two));

                        assert_eq!(
                            HiddenIndex::encode_sabotage_seer_index(
                                creature_choice,
                                hand,
                                graveyard
                            )
                            .decode_sabotage_seer_index(
                                hand.len(),
                                graveyard,
                                true
                            ),
                            Some((creature_choice, hand))
                        );
                    }
                }
            }
        }
    }
    // }}}
}
// }}}
