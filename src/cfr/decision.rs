#![allow(dead_code)]

use crate::{
    game::types::{
        CreatureChoice, CreatureSet, Edict, EdictIndex, EdictSet, Player, UserCreatureChoice,
    },
    helpers::ranged::MixRanged,
};

use bumpalo::Bump;
use rand::Rng;

use crate::helpers::{normalize_vec, roulette, swap::Pair};

// {{{ Helper types
/// Utility is the quantity players attempt to maximize.
pub type Utility = f32;

/// Float between 0 and 1.
pub type Probability = f32;
// }}}
// {{{ Decision indices
/// Used to index decision vectors.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct DecisionIndex(pub usize);

impl DecisionIndex {
    // {{{ Main phase
    /// Encodes a main phase user choice into a decision index.
    pub fn encode_main_phase_index_user(
        creatures: UserCreatureChoice,
        edict: Edict,
        edicts: EdictSet,
        hand: CreatureSet,
    ) -> Option<DecisionIndex> {
        let creature_choice = CreatureChoice::encode_user_choice(creatures, hand);
        Some(Self::encode_main_phase_index(
            creature_choice,
            edicts.count_from_end(edict),
            edicts.len(),
        ))
    }

    /// Encodes a main phase "internal" choice into a decision index.
    pub fn encode_main_phase_index(
        creatures: CreatureChoice,
        edict: EdictIndex,
        edict_count: u8,
    ) -> DecisionIndex {
        DecisionIndex((creatures.0 as usize).mix_ranged(edict.0 as usize, edict_count as usize))
    }

    /// Decodes a main phase "internal" choice into a decision index.
    pub fn decode_main_phase_index(self, edict_count: u8) -> (CreatureChoice, EdictIndex) {
        let (creatures, edict) = self.0.unmix_ranged(edict_count as usize);
        (CreatureChoice(creatures as u8), EdictIndex(edict as u8))
    }

    /// Decodes a main phase user choice into a decision index.
    pub fn decode_main_phase_index_user(
        self,
        edicts: EdictSet,
        hand: CreatureSet,
        seer_active: bool,
    ) -> Option<(UserCreatureChoice, Edict)> {
        let (creature_choice, edict_index) = self.decode_main_phase_index(edicts.len());
        let edict = edicts.lookup_from_end(edict_index)?;
        let user_creature_choice = creature_choice.decode_user_choice(hand, seer_active)?;

        Some((user_creature_choice, edict))
    }
    // }}}
    // {{{ Sabotage phase
    // }}}
}

// {{{ Tests
#[cfg(test)]
mod decision_index_tests {
    use super::*;
    use crate::game::types::Creature;

    // {{{ Main phase
    #[test]
    fn decision_encode_decode_main_inverses_seer() {
        for creature_choice in 0..100 {
            for edicts_len in 1..5 {
                for edict in 0..edicts_len {
                    let encoded = DecisionIndex::encode_main_phase_index(
                        CreatureChoice(creature_choice),
                        EdictIndex(edict),
                        edicts_len,
                    );

                    let decoded = encoded.decode_main_phase_index(edicts_len);

                    assert_eq!(
                        decoded,
                        (CreatureChoice(creature_choice), EdictIndex(edict))
                    );
                }
            }
        }
    }

    #[test]
    fn encode_decode_main_user_inverses_seer() {
        let mut edicts = EdictSet::all();
        edicts.0.remove(Edict::DivertAttention as u8);

        let mut hand = CreatureSet::default();
        hand.0.add(Creature::Rogue as u8);
        hand.0.add(Creature::Steward as u8);
        hand.0.add(Creature::Wall as u8);
        hand.0.add(Creature::Witch as u8);

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

                    let encoded = DecisionIndex::encode_main_phase_index_user(
                        UserCreatureChoice(creature_one, Some(creature_two)),
                        edict,
                        edicts,
                        hand,
                    );

                    let decoded = encoded.and_then(|encoded| {
                        encoded.decode_main_phase_index_user(edicts, hand, true)
                    });

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
}
// }}}
// }}}
// {{{ Decision vector
// {{{ Types
/// A decision a player takes in the game.
///
/// For efficiency, all the values are tightly packed into vectors indexed
/// by so called "decision indices", which are encoded/decoded differently
/// depending on the phase of the game we are currently in.
pub struct DecisionVector<'a> {
    /// Sum of every strategy devised so far during training.
    /// Unintuitively, the current strategy doesn't approach
    /// optimal play, but the sum of devised strategies does!
    strategy_sum: &'a mut [f32],

    /// Regret accumulated during training (so far).
    regret_sum: &'a mut [f32],

    /// Cached value of the positive elements in the regret_sum vector.
    regret_positive_magnitude: f32,

    /// The probabilities of each player taking the actions required to reach this state.
    realization_weights: (Probability, Probability),
}
// }}}

impl<'a> DecisionVector<'a> {
    // {{{ Helpers
    pub fn new(size: usize, allocator: &'a Bump) -> Self {
        let regret_sum = allocator.alloc_slice_fill_copy(size, 0.0);
        let strategy_sum = allocator.alloc_slice_fill_copy(size, 0.0);

        Self {
            regret_sum,
            regret_positive_magnitude: 0.0,
            strategy_sum,
            realization_weights: (0.0, 0.0),
        }
    }

    /// Returns the number of actions we can take at this node.
    #[inline]
    pub fn len(&self) -> usize {
        self.regret_sum.len()
    }
    // }}}
    // {{{ Training-related methods
    /// Compute the ith value of the strategy.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the strategy to compute
    #[inline]
    pub fn strategy(&self, index: usize) -> f32 {
        if self.regret_positive_magnitude > 0.0 {
            f32::max(self.regret_sum[index], 0.0) / self.regret_positive_magnitude
        } else {
            1.0 / (self.len() as f32)
        }
    }

    /// Update the strategy sum with the current strategy.
    pub fn update_strategy_sum(&mut self) {
        for i in 0..self.len() {
            self.strategy_sum[i] += self.strategy(i);
        }
    }

    /// Updates the cached regret magnitude once the regret sum has been changed.
    pub fn recompute_regret_magnitude(&mut self) {
        let mut sum = 0.0;
        for i in 0..self.len() {
            sum += f32::max(self.regret_sum[i], 0.0);
        }
        self.regret_positive_magnitude = sum;
    }

    /// Returns the strategy one should take in an actual game.
    /// Do not use this during training! (Performs a clone)
    pub fn get_average_strategy(&self) -> Vec<f32> {
        let mut average_strategy = self.strategy_sum.to_vec();

        normalize_vec(&mut average_strategy);

        average_strategy
    }

    /// Returns a random action based on the probability distribution
    /// in self.strategy_sum.
    ///
    /// TODO: perform normalization on-the-fly to avoid a .clone
    ///       (not very urgent, as this is never called during training)
    pub fn random_action<R: Rng>(&self, rng: &mut R) -> usize {
        let average = self.get_average_strategy();

        roulette(&average, rng)
    }
    // }}}
}
// }}}
// {{{ HiddenIndex
/// Used to index decision matrices.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct HiddenIndex(pub usize);

pub type HandContentIndex = u16;

impl HiddenIndex {
    // {{{ Hand contents
    /// Encode the contents of the hand in a single integer.
    /// Removes any information regarding hand size and
    /// graveyard content from the resulting integer.
    pub fn encode_hand_contents(hand: CreatureSet, possibilities: CreatureSet) -> HandContentIndex {
        hand.encode_relative_to(possibilities).0.encode_ones()
    }

    /// Inverse of `encode_hand_contents`.
    pub fn decode_hand_contents(
        index: HandContentIndex,
        possibilities: CreatureSet,
        hand_size: usize,
    ) -> Option<CreatureSet> {
        CreatureSet::decode_ones(index, hand_size)?.decode_relative_to(possibilities)
    }
    // }}}
    // {{{ Main phase
    /// Encodes all hidden informations known by a player during the main phase.
    #[inline]
    pub fn encode_main_index(hand: CreatureSet, graveyard: CreatureSet) -> Self {
        Self(Self::encode_hand_contents(hand, graveyard.others()) as usize)
    }

    /// Inverse of `encode_main_index`.
    #[inline]
    pub fn decode_main_index(
        self,
        graveyard: CreatureSet,
        hand_size: usize,
    ) -> Option<CreatureSet> {
        Self::decode_hand_contents(self.0 as u16, graveyard.others(), hand_size)
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
        let possibilites = graveyard.others();
        let hand_contents = Self::encode_hand_contents(hand, possibilites) as usize;
        let encoded_choice = CreatureChoice::encode_user_choice(user_creature_choice, possibilites);
        let max = possibilites.hands_of_size(user_creature_choice.len());
        let encoded = hand_contents.mix_ranged(encoded_choice.0 as usize, max);

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
        let possibilites = graveyard.others();
        let max = possibilites.hands_of_size(UserCreatureChoice::len_from_status(seer_active));
        let (hand_contents, encoded_choice) = self.0.unmix_ranged(max);
        let user_creature_choice =
            CreatureChoice(encoded_choice as u8).decode_user_choice(possibilites, seer_active)?;
        let hand_contents = Self::decode_hand_contents(hand_contents as u16, possibilites, hand_size)?;
        Some((user_creature_choice, hand_contents))
    }
    // }}}
}

// {{{ Tests
#[cfg(test)]
mod hidden_index_tests {
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
                        .decode_main_index(graveyard, hand.len() as usize),
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
                                hand.len() as usize,
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
// }}}
// {{{ Decision matrix
pub type DecisionRows<'a> = &'a mut [DecisionVector<'a>];

/// A decision matrix contains weights for the decisions both players can take.
/// Conceptually, this actually represents a pair of matrices (one for each player).
/// Each matrix can be indexed by the information a particular player holds to yield
/// the *DecisionVector*.
pub struct DecisionMatrix<'a> {
    pub vectors: Pair<DecisionRows<'a>>,
}

impl<'a> DecisionMatrix<'a> {
    pub fn new(me: DecisionRows<'a>, you: DecisionRows<'a>) -> Self {
        Self { vectors: (me, you) }
    }

    pub fn decision_count(&self) -> (usize, usize) {
        (self.vectors.0[0].len(), self.vectors.0[1].len())
    }
}
// }}}
// {{{ Explored scope
// {{{ Extra info
/// Information we need to keep track of for main phases.
#[derive(Debug)]
pub struct MainExtraInfo {
    pub edict_counts: (u8, u8),
}

/// Information we need to keep track of for sabotage phases.
#[derive(Debug)]
pub struct SabotageExtraInfo {
    /// The player about to enter a seer phase.
    /// If neither players is entering one,
    /// the value of this does not matter.
    pub seer_player: Player,
}
// }}}

/// An index into a player's hand.
/// More efficiently packed than keeping the absolute id of the card.
pub type CreatureIndex = usize;

/// Holds additional information about the current scope we are in.
/// This information depends on the type of phase the scope represents.
#[derive(Debug)]
pub enum ExploredScopeExtraInfo {
    Main(MainExtraInfo),
    Sabotage(SabotageExtraInfo),
    Seer,
}

/// Hidden information which needs to be carried out for the current scope.
/// The overseer / hand-content is implicit here.
#[derive(Debug)]
pub enum ExploredScopeHiddenInfo {
    PreMain,
    PreSabotage(CreatureIndex, CreatureIndex),
    PreSeer(CreatureIndex),
}

/// An explored scope is a scope where all the game rules have
/// been unrolled and all the game states have been created.
pub struct ExploredScope<'a> {
    /// Describes what kind of scopes we are in.
    pub kind: ExploredScopeExtraInfo,

    /// The decision matrix holds all the weights generated by training.
    pub matrix: DecisionMatrix<'a>,

    /// Vector of possible future states.
    pub next: Vec<Scope<'a>, &'a Bump>,
}

impl<'a> ExploredScope<'a> {
    pub fn get_next(
        &self,
        _decisions: (DecisionIndex, DecisionIndex),
        hidden: ExploredScopeHiddenInfo,
    ) -> (usize, ExploredScopeHiddenInfo) {
        match (&self.kind, hidden) {
            (kind, hidden) => {
                panic!(
                    "Cannot advance from state {:?} given hidden info {:?}",
                    kind, hidden
                )
            }
        }
    }
}

// }}}
// {{{ Scope
pub enum Scope<'a> {
    Unexplored,
    Explored(ExploredScope<'a>),
}

impl<'a> Default for Scope<'a> {
    fn default() -> Self {
        Scope::Unexplored
    }
}
// }}}
