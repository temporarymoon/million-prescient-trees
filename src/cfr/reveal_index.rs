use crate::game::choice::SabotagePhaseChoice;
use crate::game::types::Player;
use crate::helpers::bitfield::Bitfield;
use crate::helpers::{ranged::MixRanged, swap::Pair};
use crate::{
    game::{
        creature::{Creature, CreatureSet},
        edict::{Edict, EdictSet},
    },
    helpers::choose::choose,
};

/// Encodes all the information revealed at the end of a phase.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct RevealIndex(pub usize);

impl RevealIndex {
    // {{{ Main phase
    pub fn encode_main_phase_reveal(choices: (Edict, Edict), edicts: (EdictSet, EdictSet)) -> Self {
        let p1_index = edicts.0.indexof(choices.0);
        let p2_index = edicts.1.indexof(choices.1);

        Self(p2_index.mix_ranged(p1_index, edicts.0.len()))
    }

    pub fn decode_main_phase_reveal(self, edicts: (EdictSet, EdictSet)) -> Option<(Edict, Edict)> {
        let (p2_index, p1_index) = self.0.unmix_ranged(edicts.0.len());

        Some((edicts.0.index(p1_index)?, edicts.1.index(p2_index)?))
    }

    pub fn main_phase_count(player_edicts: (EdictSet, EdictSet)) -> usize {
        player_edicts.0.len() * player_edicts.1.len()
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

        // If we are the non seer player, then we revealed
        // `revealed_creature` this turn, which means we would've
        // had no reason to try and sabotage it.
        if let Some(sabotaged_by_non_seer) = (!seer_player).select(sabotage_choices) {
            revealed_creature_possibilities.remove(sabotaged_by_non_seer);
        };

        let mut result = revealed_creature_possibilities.indexof(revealed_creature);

        for player in Player::PLAYERS {
            if let Some(sabotaged) = player.select(sabotage_choices) {
                result = result.mix_indexof(sabotaged, possibilities)
            }
        }

        Self(result)
    }

    pub fn decode_sabotage_phase_reveal(
        self,
        sabotage_statuses: Pair<bool>,
        seer_player: Player,
        graveyard: CreatureSet,
    ) -> Option<(Pair<SabotagePhaseChoice>, Creature)> {
        // let creature = (!graveyard).index(creature_index)?;

        for player in Player::PLAYERS.iter().rev() {}

        todo!()
    }

    pub fn sabotage_phase_count(sabotage_statuses: Pair<bool>, graveyard: CreatureSet) -> usize {
        // How many times the sabotage card was played this turn
        let mut sabotage_play_count = 0;

        if sabotage_statuses.0 {
            sabotage_play_count += 1;
        };

        if sabotage_statuses.1 {
            sabotage_play_count += 1;
        };

        let sabotage_count = choose((!graveyard).len(), sabotage_play_count);

        (!graveyard).len() * sabotage_count
    }
    // }}}
    // {{{ Seer phase
    pub fn encode_seer_phase_reveal(creature: Creature, graveyard: CreatureSet) -> Self {
        Self((!graveyard).indexof(creature))
    }

    pub fn decode_seer_phase_reveal(self, graveyard: CreatureSet) -> Option<Creature> {
        (!graveyard).index(self.0)
    }

    pub fn seer_phase_count(graveyard: CreatureSet) -> usize {
        (!graveyard).len()
    }
    // }}}
}
