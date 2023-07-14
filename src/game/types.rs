use std::ops::Add;
use std::ops::Not;

// {{{ Players
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Player {
    Me,  // Current player
    You, // Opponent
}

impl Not for Player {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Player::Me => Player::You,
            Player::You => Player::Me,
        }
    }
}

impl Player {
    /// List of all players.
    pub const PLAYERS: [Self; 2] = [Player::Me, Player::You];

    /// Index a pair by a player,
    /// where the first and second elements represents the data
    /// for the current and other players respectively.
    #[inline(always)]
    pub fn select<T>(self, pair: (T, T)) -> T {
        match self {
            Player::Me => pair.0,
            Player::You => pair.1,
        }
    }

    #[inline(always)]
    pub fn select_mut<T>(self, pair: &mut (T, T)) -> &mut T {
        match self {
            Player::Me => &mut pair.0,
            Player::You => &mut pair.1,
        }
    }
}
// }}}
// {{{ Score
// Player 1 score - player 2 score
// - Negative => player 2 won
// - Positive => player 1 won
// - 0 => draw
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct Score(pub i8);

impl Add<i8> for Score {
    type Output = Self;
    fn add(self, rhs: i8) -> Self::Output {
        Score(self.0 + rhs)
    }
}
// }}}
// {{{ TurnResult
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TurnResult<T> {
    Finished(Score),
    Unfinished(T),
}

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
// {{{ BattleResult
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum BattleResult {
    Lost,
    Tied,
    Won,
}

impl Not for BattleResult {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            BattleResult::Lost => BattleResult::Won,
            BattleResult::Tied => BattleResult::Tied,
            BattleResult::Won => BattleResult::Lost,
        }
    }
}
// }}}
