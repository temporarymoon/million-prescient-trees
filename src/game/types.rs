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
