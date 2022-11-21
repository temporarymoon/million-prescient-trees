#![allow(dead_code)]

use super::hash;
use super::hash::{EchoHash, HashResult};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Battlefield {
    Mountain,
    Glade,
    Urban,
    LastStrand,
    Night,
    Plains,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Creature {
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

impl EchoHash for Creature {
    const MAX: u64 = 11;

    fn hash(&self) -> HashResult {
        HashResult(*self as u64)
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Edict {
    Sabotage,
    Gambit,
    Ambush,
    RileThePublic,
    DivertAttention,
}

impl EchoHash for Edict {
    const MAX: u64 = 5;
    fn hash(&self) -> HashResult {
        HashResult(*self as u64)
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

impl EchoHash for Battlefield {
    const MAX: u64 = 5;
    fn hash(&self) -> HashResult {
        HashResult(*self as u64)
    }
}

// Different kind of lingering effects affecting a given player
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum PlayerStatusEffect {
    // The player gains 1 strength
    Mountain,
    // The player gains +2 points if they win this battle
    Glade,
    // The player gets to play two creatures instead of one
    Seer,
    // The player gains 1 strength and gains
    // an additional point by winning this battle
    Bard,
}

// Lingering effects affecting both players
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum GlobalStatusEffect {
    Night,
}

#[derive(PartialEq, Eq, Clone, Copy)]
struct Bitfield(u16);

#[derive(PartialEq, Eq, Clone, Copy)]
struct CreatureSet(Bitfield);
#[derive(PartialEq, Eq, Clone, Copy)]
struct EdictSet(Bitfield);
#[derive(PartialEq, Eq, Clone, Copy)]
struct PlayerStatusEffects(Bitfield);
#[derive(PartialEq, Eq, Clone, Copy)]
struct GlobalStatusEffects(Bitfield);

impl Bitfield {
    fn has<I>(self, index: I) -> bool
    where
        I: Into<u16>,
    {
        ((self.0 >> index.into()) & 1) != 0
    }
}

impl Into<u64> for Bitfield {
    fn into(self) -> u64 {
        return self.0.into();
    }
}

impl EchoHash for CreatureSet {
    const MAX: u64 = 2 ^ 11;
    fn hash(&self) -> HashResult {
        return HashResult(self.0 .0 as u64);
    }
}

impl EchoHash for EdictSet {
    const MAX: u64 = 2 ^ Edict::MAX;
    fn hash(&self) -> HashResult {
        return HashResult(self.0.into());
    }
}

impl EchoHash for PlayerStatusEffects {
    const MAX: u64 = 2 ^ 4;
    fn hash(&self) -> HashResult {
        return HashResult(self.0.into());
    }
}

impl EchoHash for GlobalStatusEffects {
    const MAX: u64 = 2 ^ 1;
    fn hash(&self) -> HashResult {
        return HashResult(self.0.into());
    }
}

// State involving only one of the players
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct PlayerState {
    creatures: CreatureSet,
    edicts: EdictSet,
    effects: PlayerStatusEffects,
}

impl PlayerState {
    // Return only the infromation the current player should have acceess to
    pub fn conceal(&self) -> HiddenPlayerState {
        return HiddenPlayerState {
            edicts: self.edicts,
            effects: self.effects,
        };
    }
}

impl EchoHash for PlayerState {
    const MAX: u64 = EdictSet::MAX * PlayerStatusEffects::MAX * CreatureSet::MAX;
    fn hash(&self) -> HashResult {
        self.edicts
            .hash()
            .extend(&self.effects)
            .extend(&self.creatures)
    }
}

// What a player "knows" about it's opponent
#[derive(PartialEq, Eq, Clone, Copy)]
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

impl EchoHash for HiddenPlayerState {
    const MAX: u64 = EdictSet::MAX * PlayerStatusEffects::MAX;
    fn hash(&self) -> HashResult {
        self.edicts.hash().extend(&self.effects)
    }
}

// Choice made by one of the players in the main phase
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct MainPhaseChoice {
    edict: Edict,
    // The player is only allowed to play two creatures
    // if the "seer" status effect is active
    creatures: (Creature, Option<Creature>),
}

impl EchoHash for MainPhaseChoice {
    const MAX: u64 = Edict::MAX * Creature::MAX * (1 + Creature::MAX);
    fn hash(&self) -> HashResult {
        self.edict
            .hash()
            .extend(&self.creatures.0)
            .extend(&self.creatures.1.as_ref())
    }
}

// The number of main phases is always 2
type MainPhaseChoices = (MainPhaseChoice, MainPhaseChoice);

// The number of sabotage phases per turn varies between 0-2
type SabotagePhaseChoices = (Option<Creature>, Option<Creature>);

// A decision one of the players has to take
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum Phase {
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

impl Phase {
    // Return only the infromation the current player should have acceess to
    pub fn conceal(&self) -> HiddenPhase {
        match self {
            Phase::Main1 | Phase::Main2(_) => HiddenPhase::Main,
            Phase::SabotagePhase1(choices) | Phase::SabotagePhase2(choices, _) => {
                HiddenPhase::SabotagePhase(choices.0, choices.1.edict)
            }
            Phase::Seer(main_choices, sabotage_choices) => {
                HiddenPhase::Seer(*main_choices, *sabotage_choices)
            }
        }
    }
}

// Same as Phase but only containing the data the current player has access to
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum HiddenPhase {
    // Both main phase 1 and 2 happen simultaneously
    // in real life, therefor a player can't differentiate between them.
    // (The fact they are separate is an implemenetation detail)
    Main,
    // Similar deal to the explanation above
    // (both sabotage phases happen simultaneously!).
    // By this point, only the edicts have been revealed!
    // (which is why we cannot read the full MainPhaseChoice of the opponent)
    SabotagePhase(MainPhaseChoice, Edict),
    // The seer effect is getting resolved
    // (the player must choose one of the two cards played
    // in the main phase, and return the other to the hand)
    // Holds the choices made in all previous phases
    Seer(MainPhaseChoices, SabotagePhaseChoices),
}

impl HiddenPhase {
    const MAIN_PHASE_MAX: u64 = 1;
    const SABOTAGE_PHASE_MAX: u64 = MainPhaseChoice::MAX * Edict::MAX;
    const SEER_PHASE_MAX: u64 = MainPhaseChoices::MAX * (1 + Creature::MAX) ^ 2;
}

impl EchoHash for HiddenPhase {
    const MAX: u64 =
        HiddenPhase::MAIN_PHASE_MAX + HiddenPhase::SABOTAGE_PHASE_MAX + HiddenPhase::SEER_PHASE_MAX;

    fn hash(&self) -> HashResult {
        // Each branch has a different tag it adds in front of the hash
        match self {
            HiddenPhase::Main => HashResult(0),
            HiddenPhase::SabotagePhase(main_choice, edict) => {
                HiddenPhase::MAIN_PHASE_MAX + main_choice.hash().extend(edict)
            }
            HiddenPhase::Seer(main_choices, sabotage_choices) => {
                HiddenPhase::MAIN_PHASE_MAX
                    + HiddenPhase::SABOTAGE_PHASE_MAX
                    + main_choices
                        .hash()
                        .extend(&sabotage_choices.0.as_ref())
                        .extend(&sabotage_choices.1.as_ref())
            }
        }
    }
}

// Transitions from a phase to another.
// Not all (Phase, PhaseTransition) pairs are valid (obviously)
// pub enum PhaseTransition {
//     Main(MainPhaseChoice, Edict),
//     Sabotage(Creature),
//     Seer(Creature),
// }

// Player 1 score - player 2 score
// - Negative => player 2 won
// - Positive => player 1 won
// - 0 => draw
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Score(i8);

impl EchoHash for Score {
    // Scores can range from -25 to +25 (arbitrary bounds I chose)
    // 51 possibles scores:
    //    [-25, 25] => [0, 50] => [0, 51)
    const MAX: u64 = 51;
    fn hash(&self) -> HashResult {
        HashResult((self.0 + 25) as u64)
    }
}

// Stack of battlefields for a match.
// The tail of the list represents the current battle.
// Elements get popped as battles take place.
#[derive(PartialEq, Eq, Clone)]
pub struct Battlefields(Vec<Battlefield>);

impl EchoHash for Battlefields {
    // the +1 is there because a vec with <= 4 elements
    // is essentially equivalent to:
    //     (Option<T>) ^ 4
    const MAX: u64 = (Battlefield::MAX + 1) ^ 4;
    fn hash(&self) -> HashResult {
        hash::from_vec(&self.0)
    }
}

// Fully determined game state
pub struct GameState {
    score: Score,
    // Player specific state
    player_states: (PlayerState, PlayerState),
    // All the creature played so far
    graveyard: CreatureSet,
    // The one creature which neither player has drawn
    overseer: Creature,
    // Lingering effects
    effects: GlobalStatusEffects,
    // Stack of battlefields.
    battlefields: Battlefields,
    // The next "decision" one of the players has to take
    // the player states are always arranged in such a way
    // to ensure the first player is the one taking the current decision.
    phase: Phase,
}

impl GameState {
    // Return only the infromation the current player should have acceess to
    pub fn conceal(&self) -> InfoSet {
        return InfoSet {
            player_states: (self.player_states.0, self.player_states.1.conceal()),
            score: self.score,
            graveyard: self.graveyard,
            effects: self.effects,
            battlefields: self.battlefields.clone(),
            phase: self.phase.conceal(),
        };
    }
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

    // The next "decision" one of the players has to take
    // the player states are always arranged in such a way
    // to ensure the first player is the one taking the current decision.
    phase: HiddenPhase,

    // The remaining fields have the same meaning
    // as in the fully determined game state
    score: Score,
    graveyard: CreatureSet,
    effects: GlobalStatusEffects,
    battlefields: Battlefields,
}

impl EchoHash for InfoSet {
    const MAX: u64 = PlayerState::MAX
        * HiddenPlayerState::MAX
        * CreatureSet::MAX
        * Score::MAX
        * GlobalStatusEffects::MAX
        * Battlefields::MAX
        * HiddenPhase::MAX;
    fn hash(&self) -> HashResult {
        let base = self.player_states;

        base.hash()
            .extend(&self.graveyard)
            .extend(&self.effects)
            .extend(&self.battlefields)
            .extend(&self.score)
            .extend(&self.phase)
    }
}
