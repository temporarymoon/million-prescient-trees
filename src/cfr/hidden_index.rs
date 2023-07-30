use std::debug_assert_eq;

use crate::game::creature::{Creature, CreatureSet};
use crate::game::creature_choice::UserCreatureChoice;
use crate::game::known_state_summary::KnownStateEssentials;
use crate::game::types::Player;
use crate::helpers::bitfield::const_size_codec::ConstSizeCodec;
use crate::helpers::bitfield::Bitfield;
use crate::helpers::choose::choose;
use crate::helpers::ranged::MixRanged;

/// Encodes all hidden information known by a player.
///
/// *Important semantics*:
/// - the creature choice must not be in the hand during the sabotage/seer phase
/// - revealing a creature instantly adds it to the graveyard as well.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct HiddenIndex(pub usize);

type HandContentIndex = usize;

impl From<usize> for HiddenIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

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
        (!graveyard).count_subsets_of_size(hand_size)
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
    pub fn encode_sabotage_seer_index<S: KnownStateEssentials>(
        state: &S,
        player: Player,
        hand: CreatureSet,
        creature_choice: CreatureSet,
    ) -> Self {
        assert!(creature_choice.is_subset_of(hand));
        assert_eq!(state.creature_choice_size(player), creature_choice.len());

        let irl_hand = hand - creature_choice;
        let graveyard = state.graveyard();
        let hand_possibilites = !(graveyard);
        let choice_possibilites = !(graveyard | irl_hand);

        Self::encode_hand_contents(irl_hand, hand_possibilites)
            .mix_subset(creature_choice, choice_possibilites)
            .into()
    }

    /// Inverse of `encode_sabotage_index`
    ///
    /// The return values are:
    /// - the decoded hand
    /// - the decoded creature choice
    pub fn decode_sabotage_seer_index<S: KnownStateEssentials>(
        self,
        state: &S,
        player: Player,
    ) -> Option<(CreatureSet, CreatureSet)> {
        let graveyard = state.graveyard();
        let hand_size = state.post_main_hand_size(player);
        let choice_size = state.creature_choice_size(player);

        let max_choice_value = choose(
            Creature::CREATURES.len() - graveyard.len() - hand_size, // length of `choice_possibilities`
            choice_size,
        );

        let (remaining, encoded_choice) = self.0.unmix_ranged(max_choice_value)?;
        let irl_hand = Self::decode_hand_contents(remaining, !graveyard, hand_size)?;
        let creature_choice = CreatureSet::decode_ones_relative_to(
            encoded_choice,
            choice_size,
            !(graveyard | irl_hand),
        )?;

        Some((irl_hand | creature_choice, creature_choice))
    }

    /// One more than the maximum value of `encode_sabotage_seer_index`
    #[inline(always)]
    pub fn sabotage_seer_index_count<S: KnownStateEssentials>(state: &S, player: Player) -> usize {
        // Intuitively speaking:
        // - We first pick the hand, giving us HP choose HS possibilities
        // - We then pick the choice from the remaining HP - HS cards,
        //   giving us (HP - HS) choose CL possibilities
        let hand_possibilites = !(state.graveyard());
        let hand_size = state.post_main_hand_size(player);
        let hand_count = hand_possibilites.count_subsets_of_size(hand_size);

        let choice_len = state.creature_choice_size(player);
        let choice_count = choose(hand_possibilites.len() - hand_size, choice_len);

        choice_count * hand_count
    }
    // }}}
}

// {{{ Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::known_state_summary::KnownStateSummary;
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
        // graveyard
        for j in 0..=100 {
            let graveyard = CreatureSet::new(j);
            let player = Player::Me;
            let state = KnownStateSummary::new(Default::default(), graveyard, Some(player));

            if state.hand_size() < 2 {
                continue;
            }

            // hand
            for hand in (!graveyard).subsets_of_size(state.hand_size()) {
                // Generate creature choice
                for creature_choice in hand.subsets_of_size(2) {
                    if !(creature_choice.is_disjoint_from(graveyard))
                        || !(creature_choice.is_subset_of(hand))
                    {
                        continue;
                    };

                    let encoded = HiddenIndex::encode_sabotage_seer_index(
                        &state,
                        player,
                        hand,
                        creature_choice,
                    );

                    assert_eq!(
                        encoded.decode_sabotage_seer_index(&state, player),
                        Some((hand, creature_choice))
                    );

                    assert!(encoded.0 < HiddenIndex::sabotage_seer_index_count(&state, player));
                }
            }
        }
    }
    // }}}
}
// }}}
