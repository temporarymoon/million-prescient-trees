#![allow(dead_code)]
use flagset::{flags, FlagSet};

flags! {
    pub enum Creature:u8 {
        Wall,
        Seer,
        Rogue,
        Bard,
        Diplomat,
        Ranger,
        Steward,
        Barbarian,
        Witch,
        Mercenary,
        Monarch,
    }

    pub enum Battlefield:u8 {
        Mountain,
        Glade,
        Urban,
        LastStrand,
        Night,
        Plains,
    }

    pub enum Eddict:u8 {
        Sabotage,
        Gambit,
        Ambush,
        RileThePublic,
        DivertAttention,
    }
}

use Battlefield::*;
use Creature::*;
use Eddict::*;

impl Creature {
    // Strength of given creature (top-left of the card)
    pub fn strength(self) -> u8 {
        match self {
            Wall => 0,
            Seer => 0,
            Rogue => 1,
            Bard => 2,
            Diplomat => 2,
            Ranger => 2,
            Steward => 2,
            Barbarian => 3,
            Witch => 3,
            Mercenary => 4,
            Monarch => 6,
        }
    }
}

impl Battlefield {
    // Amount of points rewarded for winning a battle
    // in this location (top-left of card)
    pub fn reward(self) -> u8 {
        match self {
            LastStrand => 5,
            _ => 3,
        }
    }

    pub fn bonus(self, creature: Creature) -> bool {
        match (self, creature) {
            (Mountain, Ranger | Barbarian | Mercenary) => true,
            (Glade, Bard | Ranger | Witch) => true,
            (Urban, Rogue | Bard | Diplomat | Steward) => true,
            (Night, Seer | Rogue | Ranger) => true,
            _ => false,
        }
    }
}

flags! {
    // Different kind of lingering effects affecting a given player
    pub enum PlayerStatusEffect:u8 {
        // The player gains 1 strength
        Mountain,
        // The player gains +2 points if they win this battle
        Glade,
        // The player gets to play two creatures instead of one
        Seer,
        // The player gains 1 strength and gains
        // an additional point by winning this battle
        Bard
    }

    // Lingering effects affecting both players
    pub enum GlobalStatusEffect:u8 {
        Night,
    }
}

type CreatureSet = FlagSet<Creature>;
type EddictSet = FlagSet<Eddict>;
type PlayerStatusEffects = FlagSet<PlayerStatusEffect>;
type GlobalStatusEffects = FlagSet<GlobalStatusEffect>;

// State involving only one of the players
pub struct PlayerState {
    cards: CreatureSet,
    eddicts: EddictSet,
    effects: PlayerStatusEffects,
}

// What a player "knows" about it's opponent
pub struct HiddenPlayerState {
    // The eddicts a player has in hand are fully determined by:
    //    {set of eddicts} - {eddicts previously played}
    //
    // The reason we cannot do the same with creatures is
    // the existence of the overseer - a card which neither player
    // has been given. This means a player doesn't know what the
    // "full set of creatures the opponent has been given" is.
    eddicts: EddictSet,
    effects: PlayerStatusEffects,
}

// Choice made by one of the players in the main phase
pub struct MainPhaseChoice {
    eddict: Eddict,
    // The player is only allowed to play two creatures
    // if the "seer" status effect is active
    creatures: (Creature, Option<Creature>),
}

// The number of main phases is always 2
pub struct MainPhaseChoices(MainPhaseChoice, MainPhaseChoice);

// The number of sabotage phases per turn varies between 0-2
pub struct SabotagePhaseChoices(Option<Creature>, Option<Creature>);

// A decision one of the players has to take
pub enum TurnPhase {
    // Player (1/2) must pick one or two (if the seer status effect is active)
    // creatures and an eddict to play
    Main1,
    // holds the choice made in Main1
    Main2(MainPhaseChoice),
    // The sabotage eddict gets resolved
    // (the player must make a prediction for what the opponent has played)
    // Holds the choices made in Main1/2
    SabotagePhase1(MainPhaseChoices),
    // Also holds the creature chosen in SabotagePhase1
    // (We cannot get to SabotagePhase1 *unless* we
    // got past SabotagePhase1, which is why there is not
    // Option<> wrapper around the chosen creature)
    SabotagePhase2(MainPhaseChoices, Creature),
    // The seer effect is getting resolved
    // (the player must choose one of the two cards played
    // in the main phase, and return the other to the hand)
    // Holds the choices made in all previous phases
    Seer(MainPhaseChoices, SabotagePhaseChoices),
}

// Fully determined game state
pub struct GameState {
    // Player 1 score - player 2 score
    // - Negative => player 2 won
    // - Positive => player 1 won
    // - 0 => draw
    score: i8,
    // Player specific state
    player_states: (PlayerState, PlayerState),
    // All the creature played so far
    graveyard: CreatureSet,
    // The one creature which neither player has drawn
    overseer: Creature,
    // Lingering effects
    effects: GlobalStatusEffects,
    // Stack of battlefields.
    // The tail of the list represents the current battle.
    // Elements get popped as battles take place.
    battlefields: Vec<Battlefield>,
    // The next "decision" one of the players has to take
    // the player states are always arranged in such a way
    // to ensure the first player is the one taking the current decision.
    phase: TurnPhase,
}

// Game state which only contains knowedge the current player
// has of the game.
// Eg: there's no info about the cards the opponent has in hand
//
// Main differences are:
// - opponent's state is hidden
// - the identity of the overseer is hidden
pub struct InfoSet {
    // The player only has full information about themselves!
    player_states: (PlayerState, HiddenPlayerState),
    // The remaining fields have the same meaning
    // as in the fully determined game state
    score: i8,
    graveyard: CreatureSet,
    effects: GlobalStatusEffects,
    battlefields: Vec<Battlefield>,
}
