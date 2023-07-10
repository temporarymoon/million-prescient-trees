use crate::{
    game::types::{Creature, CreatureChoice, CreatureSet, Edict, EdictSet, UserCreatureChoice},
    helpers::ranged::MixRanged,
};

/// Used to index decision vectors.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct DecisionIndex(pub usize);

impl DecisionIndex {
    // {{{ Main phase
    /// Encodes a main phase user choice into a decision index.
    pub fn encode_main_phase_index(
        creatures: UserCreatureChoice,
        edict: Edict,
        edicts: EdictSet,
        hand: CreatureSet,
    ) -> DecisionIndex {
        let creature_choice = CreatureChoice::encode_user_choice(creatures, hand);
        let edict_index = edicts.indexof(edict);

        DecisionIndex((creature_choice.0).mix_ranged(edict_index, edicts.len()))
    }

    /// Decodes a main phase user choice into a decision index.
    pub fn decode_main_phase_index(
        self,
        edicts: EdictSet,
        hand: CreatureSet,
        seer_active: bool,
    ) -> Option<(UserCreatureChoice, Edict)> {
        let (creatures, edict_index) = self.0.unmix_ranged(edicts.len());
        let edict = edicts.index(edict_index)?;
        let user_creature_choice =
            CreatureChoice(creatures).decode_user_choice(hand, seer_active)?;

        Some((user_creature_choice, edict))
    }
    // }}}
    // {{{ Sabotage phase
    /// Computes a bitfield of all the allowed choices for a sabotage guess.
    /// We only guess things which are:
    /// - not in our hand
    /// - not in the graveyard
    /// - not cards we've just played
    fn sabotage_decision_possibilities(
        hand: CreatureSet,
        choice: UserCreatureChoice,
        graveyard: CreatureSet,
    ) -> CreatureSet {
        !(hand | graveyard | choice.as_creature_set())
    }

    /// Encodes a decision we can take during the sabotage phase.
    /// Assumes we know the hidden information of the current player.
    pub fn encode_sabotage_index(
        guess: Creature,
        hand: CreatureSet,
        choice: UserCreatureChoice,
        graveyard: CreatureSet,
    ) -> Self {
        let possibilities = Self::sabotage_decision_possibilities(hand, choice, graveyard);
        Self(
            CreatureSet::singleton(guess)
                .encode_relative_to(possibilities)
                .encode_ones(),
        )
    }

    /// Inverse of `encode_sabotage_index`.
    pub fn decode_sabotage_index(
        self,
        hand: CreatureSet,
        choice: UserCreatureChoice,
        graveyard: CreatureSet,
    ) -> Option<Creature> {
        let possibilities = Self::sabotage_decision_possibilities(hand, choice, graveyard);

        CreatureSet::decode_ones(self.0, 1)?
            .decode_relative_to(possibilities)?
            .into_iter()
            .next()
    }
    // }}}
    // {{{ Seer phase
    /// Encodes a decision we can take during the seer phase.
    /// Assumes we know the hidden information of the current player.
    pub fn encode_seer_index(played_cards: (Creature, Creature), choice: Creature) -> Option<Self> {
        if choice == played_cards.0 {
            Some(Self(0))
        } else if choice == played_cards.1 {
            Some(Self(1))
        } else {
            None
        }
    }

    /// Inverse of `encode_seer_index`.
    pub fn decode_seer_index(self, played_cards: (Creature, Creature)) -> Option<Creature> {
        if self.0 == 0 {
            Some(played_cards.0)
        } else if self.0 == 1 {
            Some(played_cards.1)
        } else {
            None
        }
    }
    // }}}
}

// {{{ Tests
#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;
    use crate::game::types::Creature;

    // {{{ Main phase
    #[test]
    fn encode_decode_main_inverses_seer() {
        let mut edicts = EdictSet::all();
        edicts.remove(Edict::DivertAttention);

        let mut hand = CreatureSet::default();
        hand.add(Creature::Rogue);
        hand.add(Creature::Steward);
        hand.add(Creature::Wall);
        hand.add(Creature::Witch);

        for creature_one in Creature::CREATURES {
            for creature_two in Creature::CREATURES {
                if creature_one >= creature_two
                    || !hand.has(creature_one)
                    || !hand.has(creature_two)
                {
                    continue;
                };

                for edict in Edict::EDICTS {
                    if !edicts.has(edict) {
                        continue;
                    };

                    let encoded = DecisionIndex::encode_main_phase_index(
                        UserCreatureChoice(creature_one, Some(creature_two)),
                        edict,
                        edicts,
                        hand,
                    );

                    let decoded = encoded.decode_main_phase_index(edicts, hand, true);

                    assert_eq!(
                        decoded,
                        Some((UserCreatureChoice(creature_one, Some(creature_two)), edict)),
                        "The edicts are {:?}, and the current one is {:?} (represented as {}).
                        ",
                        edicts,
                        edict,
                        edict as u8
                    );
                }
            }
        }
    }
    // }}}
    // {{{ Sabotage phase
    #[test]
    fn encode_decode_sabotage_inverses() {
        let mut hand = CreatureSet::default();
        hand.add(Creature::Rogue);
        hand.add(Creature::Wall);

        let mut graveyard = CreatureSet::default();
        graveyard.add(Creature::Seer);
        graveyard.add(Creature::Steward);

        let choice = UserCreatureChoice(Creature::Witch, Some(Creature::Monarch));

        for creature in Creature::CREATURES {
            if hand.has(creature)
                || graveyard.has(creature)
                || choice.as_creature_set().has(creature)
            {
                continue;
            };

            assert_eq!(
                DecisionIndex::encode_sabotage_index(creature, hand, choice, graveyard)
                    .decode_sabotage_index(hand, choice, graveyard),
                Some(creature)
            );
        }
    }
    // }}}
    // {{{ Seer phase
    #[test]
    fn encode_decode_seer_inverses() {
        for first in Creature::CREATURES {
            for second in Creature::CREATURES {
                if first == second {
                    continue;
                }

                let played = (first, second);

                for result in Creature::CREATURES {
                    let expected = if result == first || result == second {
                        Some(result)
                    } else {
                        None
                    };

                    assert_eq!(
                        DecisionIndex::encode_seer_index(played, result)
                            .and_then(|e| e.decode_seer_index(played)),
                        expected
                    );
                }
            }
        }
    }
    // }}}
}
// }}}
