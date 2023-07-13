use super::types::{Battlefield, EdictSet, PlayerStatusEffects, CreatureSet};
use std::ops::Add;

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
    pub effects: PlayerStatusEffects,
}

/// List of battlefields used in a battle.
// TODO: consider sharing battlefields.all
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Battlefields {
    pub all: [Battlefield; 4],
    pub current: usize,
}

// {{{ impl Battlefields
impl Battlefields {
    pub const fn new(all: [Battlefield; 4]) -> Self {
        Battlefields { all, current: 0 }
    }

    pub fn is_last(&self) -> bool {
        self.current == 3
    }

    pub fn next(&self) -> Option<Self> {
        if self.is_last() {
            None
        } else {
            Some(Battlefields {
                all: self.all,
                current: self.current + 1,
            })
        }
    }

    pub fn active(&self) -> &[Battlefield] {
        &self.all[self.current..]
    }

    pub fn current(&self) -> Battlefield {
        self.all[self.current]
    }
}
// }}}

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
