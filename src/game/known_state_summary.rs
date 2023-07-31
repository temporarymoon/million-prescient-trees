use super::{
    creature::CreatureSet, creature_choice::UserCreatureChoice, edict::EdictSet, types::Player,
};
use crate::{
    cfr::phase::PhaseTag,
    helpers::{bitfield::Bitfield, pair::Pair},
};

// {{{ Essentials trait
pub trait KnownStateEssentials {
    /// This function is assumed to be pure.
    fn graveyard(&self) -> CreatureSet;

    /// Returns the edicts each player has in hand.
    /// This function is assumed to be pure.
    fn edict_sets(&self) -> Pair<EdictSet>;

    /// Returns the player under the seer status effect.
    /// This function is assumed to be pure.
    fn seer_player(&self) -> Option<Player>;

    /// Returns a tuple specifying whether each player has the seer effect active.
    #[inline(always)]
    fn seer_statuses(&self) -> Pair<bool> {
        Player::PLAYERS.map(|p| Some(p) == self.seer_player())
    }

    /// Equivalent to `player.select(self.seer_statuses())`.
    #[inline(always)]
    fn seer_status(&self, player: Player) -> bool {
        Some(player) == self.seer_player()
    }

    /// Equivalent to `player.select(self.seer_statuses())`.
    #[inline(always)]
    fn creature_choice_size(&self, player: Player) -> usize {
        if self.seer_status(player) {
            2
        } else {
            1
        }
    }

    /// Computes the size of the hand in the current state.
    #[inline(always)]
    fn hand_size(&self) -> usize {
        5 - self.graveyard().len() / 2
    }

    /// Computes the size of the hand in a non-main phase.
    #[inline(always)]
    fn post_main_hand_size(&self, player: Player) -> usize {
        self.hand_size() - UserCreatureChoice::len_from_status(self.seer_player() == Some(player))
    }

    /// Computes the size of the hand in a non-main phase.
    #[inline(always)]
    fn hand_size_during(&self, player: Player, phase: PhaseTag) -> usize {
        if phase == PhaseTag::Main {
            self.hand_size()
        } else {
            self.post_main_hand_size(player)
        }
    }

    /// Picks a player to reveal their creature last.
    /// If the seer effect is not active, this is arbitrary.
    #[inline(always)]
    fn forced_seer_player(&self) -> Player {
        self.seer_player().unwrap_or(Player::Me)
    }

    /// Returns true if the seer status effect is active on either player.
    #[inline(always)]
    fn seer_is_active(&self) -> bool {
        self.seer_player().is_some()
    }

    /// Returns the edicts a player has in hand.
    #[inline(always)]
    fn player_edicts(&self, player: Player) -> EdictSet {
        player.select(self.edict_sets())
    }

    /// Saves the results of seer_player and graveyard to a variable.
    #[inline(always)]
    fn to_summary(&self) -> KnownStateSummary {
        KnownStateSummary::new(self.edict_sets(), self.graveyard(), self.seer_player())
    }
}
// }}}
// {{{ Minimal implementation
/// Subset of `KnownState` that is very cheap to upgrade,
/// but still enough for certain operations.
///
/// Furthermore, this struct holds the minimal information required
/// to implement `KnownStateEssentials`.
#[derive(Debug, Clone, Copy)]
pub struct KnownStateSummary {
    pub edict_sets: Pair<EdictSet>,
    pub graveyard: CreatureSet,
    pub seer_player: Option<Player>,
}

impl KnownStateSummary {
    pub fn new(
        edict_sets: Pair<EdictSet>,
        graveyard: CreatureSet,
        seer_player: Option<Player>,
    ) -> Self {
        Self {
            edict_sets,
            graveyard,
            seer_player,
        }
    }

    pub fn new_all_edicts(graveyard: CreatureSet, seer_player: Option<Player>) -> Self {
        Self {
            edict_sets: Default::default(),
            graveyard,
            seer_player,
        }
    }
}

impl KnownStateEssentials for KnownStateSummary {
    #[inline(always)]
    fn edict_sets(&self) -> Pair<EdictSet> {
        self.edict_sets
    }

    #[inline(always)]
    fn graveyard(&self) -> CreatureSet {
        self.graveyard
    }

    #[inline(always)]
    fn seer_player(&self) -> Option<Player> {
        self.seer_player
    }
}
// }}}
