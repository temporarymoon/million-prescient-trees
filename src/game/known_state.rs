use std::ops::Add;

use super::{
    battlefield::Battlefields, creature::CreatureSet, edict::EdictSet,
    status_effect::StatusEffectSet,
};

// Player 1 score - player 2 score
// - Negative => player 2 won
// - Positive => player 1 won
// - 0 => draw
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct Score(pub i8);

// {{{ impl Score
impl Add<i8> for Score {
    type Output = Self;
    fn add(self, rhs: i8) -> Self::Output {
        Score(self.0 + rhs)
    }
}
// }}}

/// State of a player known by both players.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct KnownPlayerState {
    pub edicts: EdictSet,
    pub effects: StatusEffectSet,
}

/// State known by both players at some point in time.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct KnownState {
    pub player_states: (KnownPlayerState, KnownPlayerState),
    pub battlefields: Battlefields,
    pub graveyard: CreatureSet,
    pub score: Score,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TurnResult<T> {
    Finished(Score),
    Unfinished(T),
}

// {{{ impl TurnResult
impl<T> TurnResult<T> {
    pub fn is_finished(&self) -> bool {
        match self {
            TurnResult::Finished(_) => true,
            TurnResult::Unfinished(_) => false,
        }
    }
    pub fn get_unfinished(self) -> Option<T> {
        match self {
            TurnResult::Finished(_) => None,
            TurnResult::Unfinished(result) => Some(result),
        }
    }
}
// }}}
