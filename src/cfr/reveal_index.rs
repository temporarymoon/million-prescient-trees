use std::unreachable;

use crate::game::choice::SabotagePhaseChoice;
use crate::game::creature::{Creature, CreatureSet};
use crate::game::edict::{Edict, EdictSet};
use crate::game::types::Player;
use crate::helpers::bitfield::Bitfield;
use crate::helpers::pair::Pair;
use crate::helpers::ranged::MixRanged;

use super::decision_index::DecisionIndex;
use super::hidden_index::HiddenIndex;

/// Encodes all the information revealed at the end of a phase.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct RevealIndex(pub usize);

impl RevealIndex {
    // {{{ Main phase
    #[inline(always)]
    pub fn encode_main_phase_reveal(choices: Pair<Edict>, edicts: Pair<EdictSet>) -> Self {
        let index = edicts[1]
            .indexof(choices[1])
            .mix_indexof(choices[0], edicts[0]);

        Self(index)
    }

    #[inline(always)]
    pub fn decode_main_phase_reveal(self, edict_sets: Pair<EdictSet>) -> Option<Pair<Edict>> {
        let (p2_index, p1_choice) = self.0.unmix_indexof(edict_sets[0])?;

        Some([p1_choice, edict_sets[1].index(p2_index)?])
    }

    #[inline(always)]
    pub fn main_phase_count(player_edicts: Pair<EdictSet>) -> usize {
        player_edicts[0].len() * player_edicts[1].len()
    }

    pub fn from_decisions(
        hidden: Pair<HiddenIndex>,
        decisions: Pair<DecisionIndex>,
        graveyard: CreatureSet,
        hand_size: usize,
        edict_sets: Pair<EdictSet>,
        seer_active: bool,
    ) -> Option<(Self, Pair<HiddenIndex>)> {
        let mut hands = hidden.try_map(|h| h.decode_main_index(graveyard, hand_size))?;
        let decisions =
            decisions
                .zip(edict_sets)
                .zip(hands)
                .try_map(|((decision, edicts), hand)| {
                    decision.decode_main_phase_index(edicts, hand, seer_active)
                })?;

        let creature_choices = decisions.map(|i| i.0);
        let edicts = decisions.map(|i| i.1);
        let reveal_index = Self::encode_main_phase_reveal(edicts, edict_sets);

        let mut graveyard = graveyard; // Is this necessary?

        for (i, creature_choice) in creature_choices.iter().enumerate() {
            let played = creature_choice.as_creature_set();
            graveyard |= played;
            hands[i] -= played;
        }

        let hidden_indices = creature_choices.zip(hands).map(|(creature_choice, hand)| {
            HiddenIndex::encode_sabotage_seer_index(creature_choice, hand, graveyard)
        });

        Some((reveal_index, hidden_indices))
    }
    // }}}
    // {{{ Sabotage phase
    /// Encodes data revealed after a sabotage phase.
    /// This includes:
    /// - The creature the non seer player revealed
    /// - All the sabotage choices that took place this turn
    pub fn encode_sabotage_phase_reveal(
        sabotage_choices: Pair<SabotagePhaseChoice>,
        seer_player: Player,
        revealed_creature: Creature,
        graveyard: CreatureSet,
    ) -> Self {
        let possibilities = !graveyard; // Pool of choices for sabotage guesses
        let mut revealed_creature_possibilities = possibilities;

        assert!(
            !graveyard.has(revealed_creature),
            "Revealed creature cannot be in the graveyard"
        );

        // If we are the non seer player, then we revealed
        // `revealed_creature` this turn, which means we would've
        // had no reason to try and sabotage it.
        if let Some(sabotaged_by_non_seer) = (!seer_player).select(sabotage_choices) {
            revealed_creature_possibilities.remove(sabotaged_by_non_seer);
        };

        let mut result = revealed_creature_possibilities.indexof(revealed_creature);

        for player in Player::PLAYERS {
            if let Some(sabotaged) = player.select(sabotage_choices) {
                assert!(!graveyard.has(sabotaged), "Cannot sabotage a dead creature");
                result = result.mix_indexof(sabotaged, possibilities);
            }
        }

        Self(result)
    }

    /// Inverse of `encode_sabotage_phase_reveal`.
    pub fn decode_sabotage_phase_reveal(
        self,
        sabotage_statuses: Pair<bool>,
        seer_player: Player,
        graveyard: CreatureSet,
    ) -> Option<(Pair<SabotagePhaseChoice>, Creature)> {
        let possibilities = !graveyard; // Pool of choices for sabotage guesses
        let mut encoded = self.0;
        let mut sabotage_choices = [None; 2];

        for player in Player::PLAYERS.iter().rev() {
            if player.select(sabotage_statuses) {
                let (remaining, sabotaged) = encoded.unmix_indexof(possibilities)?;
                encoded = remaining;
                player.set_selection(&mut sabotage_choices, Some(sabotaged));
            }
        }

        let mut revealed_creature_possibilities = possibilities;

        // If we are the non seer player, then we revealed
        // `revealed_creature` this turn, which means we would've
        // had no reason to try and sabotage it.
        if let Some(sabotaged_by_non_seer) = (!seer_player).select(sabotage_choices) {
            revealed_creature_possibilities.remove(sabotaged_by_non_seer);
        };

        let revealed_creature = revealed_creature_possibilities.index(encoded)?;

        Some((sabotage_choices, revealed_creature))
    }

    pub fn sabotage_phase_count(
        sabotage_statuses: Pair<bool>,
        seer_player: Player,
        graveyard: CreatureSet,
    ) -> usize {
        // How many times the sabotage card was played this turn
        let mut sabotage_play_count = 0;

        for status in sabotage_statuses {
            if status {
                sabotage_play_count += 1;
            }
        }

        let mut reveal_possibilities = (!graveyard).len();

        if (!seer_player).select(sabotage_statuses) {
            reveal_possibilities -= 1;
        };

        let sabotage_possibilities = (!graveyard).len();

        let sabotage_count = match sabotage_play_count {
            0 => 1,
            1 => sabotage_possibilities,
            2 => sabotage_possibilities * sabotage_possibilities,
            _ => unreachable!(),
        };

        reveal_possibilities * sabotage_count
    }
    // }}}
    // {{{ Seer phase
    #[inline(always)]
    pub fn encode_seer_phase_reveal(creature: Creature, graveyard: CreatureSet) -> Self {
        Self((!graveyard).indexof(creature))
    }

    #[inline(always)]
    pub fn decode_seer_phase_reveal(self, graveyard: CreatureSet) -> Option<Creature> {
        (!graveyard).index(self.0)
    }

    #[inline(always)]
    pub fn seer_phase_count(graveyard: CreatureSet) -> usize {
        (!graveyard).len()
    }
    // }}}
}

// {{{ Tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_eq;

    // {{{ Sabotage
    #[test]
    fn sabotage_decode_encode_inverses() {
        // Test with an arbitrary amount of graveyard configurations
        // (checking all of them would take too long).
        for graveyard in 0..1000 {
            let graveyard = CreatureSet::new(graveyard);
            for seer_player in Player::PLAYERS {
                for first_sabotage_creature in Creature::CREATURES {
                    for first_sabotage_status in [false, true] {
                        let first_sabotage =
                            Some(first_sabotage_creature).filter(|_| first_sabotage_status);

                        if first_sabotage_status && graveyard.has(first_sabotage_creature) {
                            continue;
                        }

                        for second_sabotage_creature in Creature::CREATURES {
                            for second_sabotage_status in [false, true] {
                                let second_sabotage = Some(second_sabotage_creature)
                                    .filter(|_| second_sabotage_status);

                                if second_sabotage_status && graveyard.has(second_sabotage_creature)
                                {
                                    continue;
                                }

                                for revealed_creature in Creature::CREATURES {
                                    if graveyard.has(revealed_creature) {
                                        continue;
                                    }

                                    let non_seer_player_sabotage =
                                        (!seer_player).select([first_sabotage, second_sabotage]);

                                    // The non seer player revealed `reveal_creature`, and would
                                    // have no reason to sabotage their own creature.
                                    if non_seer_player_sabotage == Some(revealed_creature) {
                                        continue;
                                    }

                                    let sabotage_choices = [first_sabotage, second_sabotage];

                                    let encoded = RevealIndex::encode_sabotage_phase_reveal(
                                        sabotage_choices,
                                        seer_player,
                                        revealed_creature,
                                        graveyard,
                                    );

                                    let count = RevealIndex::sabotage_phase_count(
                                        [first_sabotage_status, second_sabotage_status],
                                        seer_player,
                                        graveyard,
                                    );

                                    assert!(encoded.0 < count, "Encoded value was {}, even though the total count was supposed to be {}", encoded.0, count);

                                    let decoded = encoded.decode_sabotage_phase_reveal(
                                        [first_sabotage_status, second_sabotage_status],
                                        seer_player,
                                        graveyard,
                                    );

                                    assert_eq!(
                                        decoded,
                                        Some((sabotage_choices, revealed_creature))
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // }}}
}
// }}}
