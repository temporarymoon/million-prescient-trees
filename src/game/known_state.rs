use super::battlefield::{Battlefield, Battlefields};
use super::creature::CreatureSet;
use super::edict::EdictSet;
use super::status_effect::{StatusEffect, StatusEffectSet};
use super::types::{Player, Score};
use crate::helpers::bitfield::Bitfield;
use crate::helpers::pair::{are_equal, Pair};

/// State of a player known by both players.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct KnownPlayerState {
    pub edicts: EdictSet,
    pub effects: StatusEffectSet,
}

/// State known by both players at some point in time.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct KnownState {
    pub player_states: Pair<KnownPlayerState>,
    pub battlefields: Battlefields,
    pub graveyard: CreatureSet,
    pub score: Score,
}

impl KnownState {
    pub fn new_starting(battlefields: [Battlefield; 4]) -> Self {
        Self {
            player_states: Default::default(),
            graveyard: Default::default(),
            score: Default::default(),
            battlefields: Battlefields::new(battlefields),
        }
    }

    /// Returns the player under the seer status effect.
    pub fn seer_player(&self) -> Option<Player> {
        if self
            .player_states
            .0
            .effects
            .has(super::status_effect::StatusEffect::Seer)
        {
            Some(Player::Me)
        } else if self
            .player_states
            .1
            .effects
            .has(super::status_effect::StatusEffect::Seer)
        {
            Some(Player::You)
        } else {
            None
        }
    }

    /// Returns true if the seer status effect is active on either player.
    #[inline(always)]
    pub fn seer_is_active(&self) -> bool {
        self.seer_player().is_some()
    }

    /// Computes the size of the hand in the current state.
    #[inline(always)]
    pub fn hand_size(&self) -> usize {
        5 - self.graveyard.len() / 2
    }

    /// Returns a tuple specifying whether each player has the seer effect active.
    pub fn seer_statuses(&self) -> Pair<bool> {
        (
            self.player_states.0.effects.has(StatusEffect::Seer),
            self.player_states.1.effects.has(StatusEffect::Seer),
        )
    }

    /// Picks a player to reveal their creature last.
    /// If the seer effect is not active, this is arbitrary.
    pub fn forced_seer_player(&self) -> Player {
        self.seer_player().unwrap_or(Player::Me)
    }

    /// Returns whether the current known game state is symmetrical.
    /// A game state is symmetrical if whenever (A, B) is a possible
    /// combination of hidden information the two players might know,
    /// (B, A) is also such a possibility.
    ///
    /// The first turn is usually the only symmetrical game state.
    pub fn is_symmetrical(&self, is_first_turn: bool) -> bool {
        // false
        is_first_turn && are_equal(self.player_states) && self.score == Score::default()
    }
}
