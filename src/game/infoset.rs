#![allow(dead_code)]

use super::types::{Creature, Edict};

/// Keeps track of all the choices made so far
/// which the current player *knows* about.
///
/// For example, the current player knows what
/// cards they begun the game with, but does not
/// know what the seer is.
pub struct KnownInformation {
    /// Each element represents the index of the choice made.
    pub choices: Vec<u8>,

    /// Number between 0-252 representing the creatures the
    /// player has in hand.
    pub current_player_hand: u8,
}

/// All the possible things a player might have chosen
/// without the opponent knowing just yet.
pub enum HiddenChoice {
    /// The main phase is the phase when both players
    /// put down their creatures and edicts.
    MainPhase(Creature, Edict),

    /// The sabotage phase is the phase when all players
    /// who've played the sabotage edict write down their
    /// guess for what the opponent' creature is.
    SabotagePhase(Creature),
}

/// All the information the current player does not have about the game.
pub struct HiddenInformation {
    /// Sometimes a player is taking a decision
    /// after the opponent has already taken theirs,
    /// without knowing what choice the opponent made.
    ///
    /// For example, the opponent might put down their
    /// creature and eddict, while the current player is
    /// still thinking. In that case, this property would
    /// be equal to Some(MainPhase(...))
    pub hidden_choice: Option<HiddenChoice>,

    /// The seer is the creature taken out of the game
    /// at the start of a round. Represented as a number
    /// between 0 and 11.
    pub seer: u8,
}

pub struct GameState {
    pub public: KnownInformation,
    pub hidden: HiddenInformation,
}
