#![allow(dead_code)]

use std::hash::Hash;
use super::helpers::Swap;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Battlefield {
    Mountain,
    Glade,
    Urban,
    LastStrand,
    Night,
    Plains,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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

use Creature::*;

impl Creature {
    const CREATURES: [Creature; 11] = [
        Wall, Seer, Rogue, Bard, Diplomat, Ranger, Steward, Barbarian, Witch, Mercenary, Monarch,
    ];
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

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum Edict {
    Sabotage,
    Gambit,
    Ambush,
    RileThePublic,
    DivertAttention,
}

impl Edict {
    const EDICTS: [Edict; 5] = [
        Edict::Sabotage,
        Edict::Gambit,
        Edict::Ambush,
        Edict::RileThePublic,
        Edict::DivertAttention,
    ];
}


use Battlefield::*;

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


// Different kind of lingering effects affecting a given player
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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
    // This battle, lose 1 strength
    Mercenary,
    // The barbarian gains 2 strength if
    // it gets played
    Barbarian,
}

// Lingering effects affecting both players
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum GlobalStatusEffect {
    Night,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct Bitfield(u16);

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct CreatureSet(Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct EdictSet(Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct PlayerStatusEffects(Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct GlobalStatusEffects(Bitfield);

impl Bitfield {
    pub fn has(self, index: u8) -> bool {
        ((self.0 >> index as u16) & 1) != 0
    }
}

impl Into<u64> for Bitfield {
    fn into(self) -> u64 {
        return self.0.into();
    }
}

// State involving only one of the players
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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

// What a player "knows" about it's opponent
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct MainPhaseChoice {
    edict: Edict,
    // The player is only allowed to play two creatures
    // if the "seer" status effect is active
    creatures: (Creature, Option<Creature>),
}

impl MainPhaseChoice {
    pub fn to_final(self) -> FinalMainPhaseChoice {
        if self.creatures.1.is_some() {
            panic!("Tried making main phase choice final before resolving seer phase");
        }

        FinalMainPhaseChoice {
            edict: self.edict,
            creature: self.creatures.0,
        }
    }
}

// Similar to MainPhaseChoice but used after the seer phase gets resolved
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct FinalMainPhaseChoice {
    edict: Edict,
    creature: Creature,
}

// The number of main phases is always 2
type MainPhaseChoices = (MainPhaseChoice, MainPhaseChoice);

// The number of sabotage phases per turn varies between 0-2
type SabotagePhaseChoices = (Option<Creature>, Option<Creature>);

// A decision one of the players has to take
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
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


// Transitions from a phase to another.
// Not all (Phase, PhaseTransition) pairs are valid (obviously)
pub enum PhaseTransition {
    Main(MainPhaseChoice),
    Sabotage(Creature),
    Seer(Creature),
}

// Player 1 score - player 2 score
// - Negative => player 2 won
// - Positive => player 1 won
// - 0 => draw
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct Score(pub i8);


// Stack of battlefields for a match.
// The tail of the list represents the current battle.
// Elements get popped as battles take place.
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Battlefields(Vec<Battlefield>);


// Fully determined game state
#[derive(PartialEq, Eq, Hash, Clone)]
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

    // change the phase we are in
    pub fn switch_to(&self, phase: Phase) -> Self {
        GameState {
            battlefields: self.battlefields.clone(),
            phase,
            ..*self
        }
    }

    // change the phase we are in while also flipping the player
    pub fn flip_to(&self, phase: Phase) -> Self {
        GameState {
            score: Score(-self.score.0),
            player_states: self.player_states.swap(),
            battlefields: self.battlefields.clone(),
            phase,
            ..*self
        }
    }

    fn resolve_battle(
        &self,
        main_choices: (FinalMainPhaseChoice, FinalMainPhaseChoice),
        sabotage_choices: (Option<Creature>, Option<Creature>),
    ) -> Self {
        unimplemented!()
    }

    fn resolve_sabotage(
        &self,
        main_choices: MainPhaseChoices,
        sabotage_choices: (Option<Creature>, Option<Creature>),
    ) -> (Self, bool) {
        if main_choices.0.creatures.1.is_some() {
            (
                self.switch_to(Phase::Seer(main_choices, sabotage_choices)),
                false,
            )
        } else if main_choices.1.creatures.1.is_some() {
            (
                self.flip_to(Phase::Seer(main_choices.swap(), sabotage_choices)),
                true,
            )
        } else {
            (
                self.resolve_battle(
                    (main_choices.0.to_final(), main_choices.1.to_final()),
                    sabotage_choices,
                ),
                false,
            )
        }
    }

    pub fn apply_transition(&self, transition: PhaseTransition) -> Option<(Self, bool)> {
        match (self.phase, transition) {
            (Phase::Main1, PhaseTransition::Main(choice)) => {
                Some((self.flip_to(Phase::Main2(choice)), true))
            }
            (Phase::Main2(first_choice), PhaseTransition::Main(second_choice)) => Some({
                let choices = (first_choice, second_choice);
                if first_choice.edict == Edict::Sabotage {
                    (self.switch_to(Phase::SabotagePhase1(choices)), false)
                } else if second_choice.edict == Edict::Sabotage {
                    (self.flip_to(Phase::SabotagePhase1(choices.swap())), true)
                } else {
                    self.resolve_sabotage(choices, (None, None))
                }
            }),
            (Phase::SabotagePhase1(main_choices), PhaseTransition::Sabotage(creature)) => Some({
                if main_choices.1.edict == Edict::Sabotage {
                    (
                        self.flip_to(Phase::SabotagePhase2(main_choices.swap(), creature)),
                        true,
                    )
                } else {
                    self.resolve_sabotage(main_choices, (Some(creature), None))
                }
            }),
            (
                Phase::SabotagePhase2(main_choices, first_sabotage),
                PhaseTransition::Sabotage(creature),
            ) => Some({
                self.resolve_sabotage(main_choices, (Some(creature), Some(first_sabotage)))
            }),
            (Phase::Seer(main_choices, sabotage_choices), PhaseTransition::Seer(creature)) => {
                // The main choice made by the same player who is performing
                // the seer phase transition.
                let main_choice = main_choices.0;

                match main_choice.creatures.1 {
                    Some(second_creature)
                        if second_creature == creature || main_choice.creatures.0 == creature =>
                    {
                        Some((
                            self.resolve_battle(
                                (
                                    FinalMainPhaseChoice {
                                        edict: main_choice.edict,
                                        creature,
                                    },
                                    main_choices.1.to_final(),
                                ),
                                sabotage_choices,
                            ),
                            false,
                        ))
                    }
                    _ => panic!("Invalid choice for seer phase"),
                }
            }
            _ => None,
        }
    }
}

// Game state which only contains knowedge the current player
// has of the game.
// Eg: there's no info about the cards the opponent has in hand
//
// Main differences are:
// - opponent's state is hidden
// - the identity of the overseer is hidden
#[derive(PartialEq, Eq, Hash, Clone)]
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

impl InfoSet {
    pub fn available_actions(&self) -> Vec<PhaseTransition> {
        let mut choices = Vec::new();
        match self.phase {
            HiddenPhase::Main => {
                let effects = self.player_states.0.effects.0;
                let seer_is_active = effects.has(PlayerStatusEffect::Seer as u8);

                for creature in Creature::CREATURES {
                    for edict in Edict::EDICTS {
                        if seer_is_active {
                            for extra_creature in Creature::CREATURES {
                                if creature == extra_creature {
                                    continue;
                                }

                                choices.push(PhaseTransition::Main(MainPhaseChoice {
                                    edict,
                                    creatures: (creature, Some(extra_creature)),
                                }))
                            }
                        } else {
                            choices.push(PhaseTransition::Main(MainPhaseChoice {
                                edict,
                                creatures: (creature, None),
                            }))
                        }
                    }
                }
            }
            HiddenPhase::SabotagePhase(_, _) => {
                for creature in Creature::CREATURES {
                    if self.graveyard.0.has(creature as u8) {
                        continue;
                    } else if self.player_states.0.creatures.0.has(creature as u8) {
                        continue;
                    }

                    choices.push(PhaseTransition::Sabotage(creature))
                }
            }
            HiddenPhase::Seer(main_choices, _) => {
                let creatures = main_choices.0.creatures;
                choices.push(PhaseTransition::Seer(creatures.0));
                if let Some(secondary_pick) = creatures.1 {
                    choices.push(PhaseTransition::Seer(secondary_pick))
                } else {
                    panic!("Invalid seer phase with single creature on the table.")
                }
            }
        }
        choices
    }
}

pub enum CompleteGameState {
    Finished(Score),
    Unfinished(GameState),
}
