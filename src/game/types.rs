use crate::cfr::decision::Utility;
use crate::helpers::pair::{conditional_swap, Pair};
use std::ops::Add;
use std::ops::Neg;
use std::ops::Not;
use std::ops::Sub;

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
    pub fn select<T>(self, pair: Pair<T>) -> T {
        let [a, b] = pair;
        match self {
            Player::Me => a,
            Player::You => b,
        }
    }

    /// Mutates the value selected by `select_mut`.
    #[inline(always)]
    pub fn set_selection<T>(self, pair: &mut Pair<T>, value: T) {
        let selection = self.select_mut(pair);
        *selection = value;
    }

    /// Similar to `select` but for references.
    #[inline(always)]
    pub fn select_ref<T>(self, pair: &Pair<T>) -> &T {
        let [a, b] = pair;
        match self {
            Player::Me => &a,
            Player::You => &b,
        }
    }

    /// Similar to `select` but for mut references.
    #[inline(always)]
    pub fn select_mut<T>(self, pair: &mut Pair<T>) -> &mut T {
        match self {
            Player::Me => &mut pair[0],
            Player::You => &mut pair[1],
        }
    }

    /// Swaps a pair such that:
    ///
    /// ```ignore
    /// pair.0 ==    player.select(player.order_as(pair))
    /// pair.1 == (!player).select(player.order_as(pair))
    /// player.select(pair)    == player.order_as(pair).0
    /// (!player).select(pair) == player.order_as(pair).1
    /// ```
    #[inline(always)]
    pub fn order_as<T: Copy>(self, pair: Pair<T>) -> Pair<T> {
        conditional_swap(pair, self == Player::You)
    }
}

#[cfg(test)]
mod player_tests {
    use crate::game::types::Player;

    #[test]
    fn select_examples() {
        assert_eq!(Player::Me.select([1, 2]), 1);
        assert_eq!(Player::You.select([1, 2]), 2);
    }

    #[test]
    fn order_as_properties() {
        let pair = [1, 2];

        for player in Player::PLAYERS {
            let ordered = player.order_as(pair);
            assert_eq!(pair[0], player.select(ordered));
            assert_eq!(pair[1], (!player).select(ordered));
            assert_eq!(player.select(pair), ordered[0]);
            assert_eq!((!player).select(pair), ordered[1]);
        }
    }
}
// }}}
// {{{ Score
// Player 1 score - player 2 score
// - Negative => player 2 won
// - Positive => player 1 won
// - 0 => draw
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Default)]
pub struct Score(pub i8);

impl Score {
    /// Convert score to utlity — the value training attempts to maximize.
    #[inline(always)]
    pub fn to_utility(self) -> Utility {
        match self.to_battle_result() {
            BattleResult::Won => 1.0,
            BattleResult::Lost => -1.0,
            BattleResult::Tied => 0.0,
        }
    }

    /// Returns the result of a game ending with this score.
    #[inline(always)]
    pub fn to_battle_result(self) -> BattleResult {
        if self.0 > 0 {
            BattleResult::Won
        } else if self.0 < 0 {
            BattleResult::Lost
        } else {
            BattleResult::Tied
        }
    }

    /// Returns the score from a given player's perspective.
    #[inline(always)]
    pub fn from_perspective(self, player: Player) -> Score {
        match player {
            Player::Me => self,
            Player::You => -self,
        }
    }
}

impl Add<i8> for Score {
    type Output = Self;
    fn add(self, rhs: i8) -> Self::Output {
        Score(self.0 + rhs)
    }
}

impl Sub<i8> for Score {
    type Output = Self;
    fn sub(self, rhs: i8) -> Self::Output {
        Score(self.0 - rhs)
    }
}

impl Neg for Score {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
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

    /// Maps the inner value kept by this result.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> TurnResult<U> {
        match self {
            TurnResult::Finished(s) => TurnResult::Finished(s),
            TurnResult::Unfinished(u) => TurnResult::Unfinished(f(u)),
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
