use crate::helpers::bitfield::Bitfield;
use crate::helpers::Pair;
use super::types::{Player, Score};
use super::battlefield::Battlefields;
use super::status_effect::StatusEffectSet;
use super::edict::EdictSet;
use super::creature::CreatureSet;

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
    pub fn seer_is_active(&self) -> bool {
        self.seer_player().is_some()
    }
}
