use super::battlefield::{Battlefield, Battlefields};
use super::creature::{Creature, CreatureSet};
use super::creature_choice::UserCreatureChoice;
use super::edict::{Edict, EdictSet};
use super::status_effect::{StatusEffect, StatusEffectSet};
use super::types::{Player, Score};
use crate::helpers::bitfield::Bitfield;
use crate::helpers::pair::{are_equal, Pair};
use crate::helpers::try_from_iter::TryCollect;

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
        if self.player_states[0]
            .effects
            .has(super::status_effect::StatusEffect::Seer)
        {
            Some(Player::Me)
        } else if self.player_states[1]
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

    /// Computes the size of the hand in a non-main phase.
    #[inline(always)]
    pub fn post_main_hand_sizes(&self) -> Pair<usize> {
        self.seer_statuses()
            .map(|status| self.hand_size() - UserCreatureChoice::len_from_status(status))
    }

    /// Returns a tuple specifying whether each player has the seer effect active.
    pub fn seer_statuses(&self) -> Pair<bool> {
        self.player_states
            .map(|s| s.effects.has(StatusEffect::Seer))
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
    pub fn is_symmetrical(&self) -> bool {
        self.battlefields.current == 0
            && are_equal(self.player_states)
            && self.score == Score::default()
    }

    /// Returns the score from a given player's perspective
    #[inline(always)]
    pub fn score(&self, player: Player) -> Score {
        match player {
            Player::Me => self.score,
            Player::You => -self.score,
        }
    }

    /// Returns the edicts a player has in hand.
    #[inline(always)]
    pub fn edicts(&self, player: Player) -> EdictSet {
        player.select(self.player_states).edicts
    }

    /// Computes whether a given player is guaranteed to win,
    /// no matter what the opponent can pull off.
    // TODO: add stalling with wall?
    pub fn guaranteed_win(&self, player: Player) -> bool {
        // {{{ Rile the public spam
        let has_rtp = self.edicts(!player).has(Edict::RileThePublic);
        let has_steward = !self.graveyard.has(Creature::Steward);
        let has_urban = self.battlefields.will_be_active(Battlefield::Urban);

        let turns_left = 4 - self.battlefields.current;
        let mut rtp_usages = 0;

        if has_rtp {
            rtp_usages += 1; // base usage
        };

        if has_urban {
            rtp_usages += 1; // edict multiplier
        };

        if has_steward {
            rtp_usages += 1; // edict multiplier

            if turns_left > 1 {
                rtp_usages += 1; // steward return edicts to hand effect
            }
        };
        // }}}

        let mut max_opponent_gain = self
            .battlefields
            .active()
            .iter()
            .map(|battlefield| battlefield.reward())
            .sum::<u8>() as i8
            + rtp_usages;

        // {{{ Battlefield vp bonuses
        let effects = (!player).select(self.player_states).effects;

        if effects.has(StatusEffect::Glade) || self.battlefields.will_be_active(Battlefield::Glade)
        {
            max_opponent_gain += 2;
        }

        if effects.has(StatusEffect::Night) || self.battlefields.will_be_active(Battlefield::Night)
        {
            max_opponent_gain += 1;
        }
        // }}}

        self.score(player) > Score(max_opponent_gain)
    }

    /// Returns the edicts owned by each player.
    pub fn edict_sets(&self) -> Pair<EdictSet> {
        self.player_states.map(|s| s.edicts)
    }
}
