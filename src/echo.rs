#![allow(dead_code)]
use flagset::{flags, FlagSet};

#[derive(Clone, Copy)]
pub enum Battlefield {
    Mountain,
    Glade,
    Urban,
    LastStrand,
    Night,
    Plains,
}

flags! {
    pub enum Creature:u16 {
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

    pub enum Edict:u8 {
        Sabotage,
        Gambit,
        Ambush,
        RileThePublic,
        DivertAttention,
    }
}

use Battlefield::*;
use Creature::*;
// use Edict::*;

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
type EdictSet = FlagSet<Edict>;
type PlayerStatusEffects = FlagSet<PlayerStatusEffect>;
type GlobalStatusEffects = FlagSet<GlobalStatusEffect>;

// State involving only one of the players
pub struct PlayerState {
    creatures: CreatureSet,
    edicts: EdictSet,
    effects: PlayerStatusEffects,
}

// What a player "knows" about it's opponent
pub struct HiddenPlayerState {
    // The edicts a player has in hand are fully determined by:
    //    {set of edicts} - {edicts previously played}
    //
    // The reason we cannot do the same with creatures is
    // the existence of the overseer - a card which neither player
    // has been given. This means a player doesn't know what the
    // "full set of creatures the opponent has been given" is.
    edicts: EdictSet,
    effects: PlayerStatusEffects,
}

// Choice made by one of the players in the main phase
pub struct MainPhaseChoice {
    edict: Edict,
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
    // creatures and an edict to play
    Main1,
    // holds the choice made in Main1
    Main2(MainPhaseChoice),
    // The sabotage edict gets resolved
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
pub struct InfoSet<'a> {
    // The player only has full information about themselves!
    player_states: (PlayerState, HiddenPlayerState),
    // The remaining fields have the same meaning
    // as in the fully determined game state
    score: i8,
    graveyard: CreatureSet,
    effects: GlobalStatusEffects,
    battlefields: &'a Vec<Battlefield>,
}

impl PlayerState {
    // Layout (20 bits):
    // - [0-11] - creatures
    // - [11-16] - edicts
    // - [16-20] - effects
    pub fn hash(&self) -> u64 {
        let creatures = self.creatures.bits() as u64;
        let edicts = self.edicts.bits() as u64;
        let effects = self.effects.bits() as u64;
        (effects << 16) | (edicts << 11) | creatures
    }
}

impl HiddenPlayerState {
    // Layout (9 bits):
    // - [0-5] - edicts
    // - [5-9] - effects
    pub fn hash(&self) -> u64 {
        let edicts = self.edicts.bits() as u64;
        let effects = self.effects.bits() as u64;
        (effects << 5) | edicts
    }
}

impl<'a> InfoSet<'a> {
    // 4 battlefields or less
    // 5 values each
    // => 5^4 = 625
    // => < 2^10
    // => 10 bits
    fn hash_battlefields(&self) -> u64 {
        let mut result = 0;
        for battlefield in self.battlefields {
            result = result * 5 + *battlefield as u64
        }
        return result;
    }

    // Layout:
    // - [0-20] - Player 1 state
    // - [20-29] - Player 2 state
    // - [29-37] - Score
    // - [37-48] - Graveyard
    // - [48-49] - Effects
    // - [49-59] - Battlefields
    pub fn hash(&self) -> u64 {
        let player1 = self.player_states.0.hash();
        let player2 = self.player_states.1.hash();
        let score = ((self.score as i64) + 128) as u64;
        let graveryard = self.graveyard.bits() as u64;
        let effects = self.effects.bits() as u64;
        let battlefields = self.hash_battlefields();

        (battlefields << 49)
            | (effects << 48)
            | (graveryard << 37)
            | (score << 29)
            | (player2 << 20)
            | player1
    }
}
