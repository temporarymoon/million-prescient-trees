use super::types::{Creature, Edict};
use crate::game::types::{CreatureSet, EdictSet, PlayerStatusEffects};
use std::fmt::{self, Display};
use std::hash::Hash;

// {{{ Player state
// State involving only one of the players
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PlayerState {
    pub creatures: CreatureSet,
    pub edicts: EdictSet,
    pub effects: PlayerStatusEffects,
}

impl PlayerState {
    pub fn new(creatures: CreatureSet) -> Self {
        PlayerState {
            creatures,
            edicts: EdictSet::all(),
            effects: Default::default(),
        }
    }
}
// }}}
// {{{ Main phase choice
// Choice made by one of the players in the main phase
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct MainPhaseChoice {
    pub edict: Edict,
    // The player is only allowed to play two creatures
    // if the "seer" status effect is active
    pub creatures: (Creature, Option<Creature>),
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
// }}}
// {{{ Final main phase choice
// Similar to MainPhaseChoice but used after the seer phase gets resolved
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct FinalMainPhaseChoice {
    pub creature: Creature,
    pub edict: Edict,
}

impl FinalMainPhaseChoice {
    #[inline]
    pub fn new(creature: Creature, edict: Edict) -> Self {
        Self { creature, edict }
    }
}
// }}}
// {{{ Phase
// The number of main phases is always 2
type MainPhaseChoices = (MainPhaseChoice, MainPhaseChoice);

pub type SabotagePhaseChoice = Option<Creature>;

// The number of sabotage phases per turn varies between 0-2
pub type SabotagePhaseChoices = (Option<Creature>, Option<Creature>);

// A decision one of the players has to take
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
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
// }}}
// {{{ Phase transition
// Transitions from a phase to another.
// Not all (Phase, PhaseTransition) pairs are valid (obviously)
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum PhaseTransition {
    Main(MainPhaseChoice),
    Sabotage(Creature),
    Seer(Creature),
}

impl PhaseTransition {
    pub fn to_seer(&self) -> Option<Creature> {
        match self {
            PhaseTransition::Seer(choice) => Some(*choice),
            _ => None,
        }
    }
    pub fn to_sabotage(&self) -> Option<Creature> {
        match self {
            PhaseTransition::Sabotage(choice) => Some(*choice),
            _ => None,
        }
    }
    pub fn to_main(&self) -> Option<MainPhaseChoice> {
        match self {
            PhaseTransition::Main(choice) => Some(*choice),
            _ => None,
        }
    }
}

impl Display for PhaseTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhaseTransition::Sabotage(creature) => write!(f, "Sabotage target: {}", creature),
            PhaseTransition::Seer(creature) => write!(f, "Seer choice: {}", creature),
            PhaseTransition::Main(MainPhaseChoice { edict, creatures }) => match creatures.1 {
                None => {
                    write!(f, "Creature: {}. Edict: {}", creatures.0, edict)
                }
                Some(seer_creature) => {
                    write!(
                        f,
                        "Creatures: {} & {}. Edict: {}",
                        creatures.0, seer_creature, edict
                    )
                }
            },
        }
    }
}
// }}}
// // {{{ Game state
// // Fully determined game state
// #[derive(PartialEq, Eq, Hash, Clone, Debug)]
// pub struct GameState {
//     pub score: Score,
//     // Player specific state
//     pub player_states: (PlayerState, PlayerState),
//     // All the creature played so far
//     pub graveyard: CreatureSet,
//     // The one creature which neither player has drawn
//     pub overseer: Creature,
//     // Lingering effects
//     pub night: bool,
//     // Stack of battlefields.
//     pub battlefields: Battlefields,
//     // The next "decision" one of the players has to take
//     // the player states are always arranged in such a way
//     // to ensure the first player is the one taking the current decision.
//     pub phase: Phase,
// }
//
// impl GameState {
//     // change the phase we are in
//     pub fn switch_to(&self, phase: Phase) -> CompleteGameState {
//         CompleteGameState::Unfinished(GameState {
//             battlefields: self.battlefields.clone(),
//             phase,
//             ..*self
//         })
//     }
//
//     // change the phase we are in while also flipping the player
//     pub fn flip_to(&self, phase: Phase) -> CompleteGameState {
//         CompleteGameState::Unfinished(GameState {
//             score: Score(-self.score.0),
//             player_states: self.player_states.swap(),
//             battlefields: self.battlefields.clone(),
//             phase,
//             ..*self
//         })
//     }
//
//     // fn resolve_battle(
//     //     &self,
//     //     main_choices: (FinalMainPhaseChoice, FinalMainPhaseChoice),
//     //     sabotage_choices: SabotagePhaseChoices,
//     // ) -> CompleteGameState {
//     //     let context = FullBattleContext {
//     //         main_choices,
//     //         sabotage_choices,
//     //         battlefield: self.battlefields.current(),
//     //         night: self.night
//     //         player_states: self.player_states,
//     //     };
//     //
//     //     context.advance_game_state(self)
//     // }
//
//     fn resolve_sabotage(
//         &self,
//         main_choices: MainPhaseChoices,
//         sabotage_choices: SabotagePhaseChoices,
//     ) -> (CompleteGameState, bool) {
//         if main_choices.0.creatures.1.is_some() {
//             (
//                 self.switch_to(Phase::Seer(main_choices, sabotage_choices)),
//                 false,
//             )
//         } else if main_choices.1.creatures.1.is_some() {
//             (
//                 self.flip_to(Phase::Seer(main_choices.swap(), sabotage_choices.swap())),
//                 true,
//             )
//         } else {
//             (
//                 self.resolve_battle(
//                     (main_choices.0.to_final(), main_choices.1.to_final()),
//                     sabotage_choices,
//                 ),
//                 false,
//             )
//         }
//     }
//
//     pub fn apply_transition(
//         &self,
//         transition: PhaseTransition,
//     ) -> Option<(CompleteGameState, bool)> {
//         match (self.phase, transition) {
//             (Phase::Main1, PhaseTransition::Main(choice)) => {
//                 Some((self.flip_to(Phase::Main2(choice)), true))
//             }
//             (Phase::Main2(first_choice), PhaseTransition::Main(second_choice)) => Some({
//                 let choices = (second_choice, first_choice);
//                 if second_choice.edict == Edict::Sabotage {
//                     (self.switch_to(Phase::SabotagePhase1(choices)), false)
//                 } else if first_choice.edict == Edict::Sabotage {
//                     (self.flip_to(Phase::SabotagePhase1(choices.swap())), true)
//                 } else {
//                     self.resolve_sabotage(choices, (None, None))
//                 }
//             }),
//             (Phase::SabotagePhase1(main_choices), PhaseTransition::Sabotage(creature)) => Some({
//                 if main_choices.1.edict == Edict::Sabotage {
//                     (
//                         self.flip_to(Phase::SabotagePhase2(main_choices.swap(), creature)),
//                         true,
//                     )
//                 } else {
//                     self.resolve_sabotage(main_choices, (Some(creature), None))
//                 }
//             }),
//             (
//                 Phase::SabotagePhase2(main_choices, first_sabotage),
//                 PhaseTransition::Sabotage(creature),
//             ) => Some({
//                 self.resolve_sabotage(main_choices, (Some(creature), Some(first_sabotage)))
//             }),
//             (Phase::Seer(main_choices, sabotage_choices), PhaseTransition::Seer(creature)) => {
//                 // The main choice made by the same player who is performing
//                 // the seer phase transition.
//                 let main_choice = main_choices.0;
//
//                 match main_choice.creatures.1 {
//                     Some(second_creature)
//                         if second_creature == creature || main_choice.creatures.0 == creature =>
//                     {
//                         Some((
//                             self.resolve_battle(
//                                 (
//                                     FinalMainPhaseChoice {
//                                         edict: main_choice.edict,
//                                         creature,
//                                     },
//                                     main_choices.1.to_final(),
//                                 ),
//                                 sabotage_choices,
//                             ),
//                             false,
//                         ))
//                     }
//                     _ => panic!("Invalid choice for seer phase"),
//                 }
//             }
//             _ => None,
//         }
//     }
//
//     #[inline]
//     fn get_player_state(&self, player: Player) -> PlayerState {
//         match player {
//             Player::Me => self.player_states.0,
//             Player::You => self.player_states.1,
//         }
//     }
//
//     /// Prepares the overseer candiates for a decision node.
//     /// See the docs there for more details.
//     pub fn get_overseer_candiates(&self, player: Player) -> [Option<u8>; 11] {
//         let mut out: [Option<u8>; 11] = Default::default();
//         let state = self.get_player_state(player);
//
//         let mut index = 0;
//
//         for creature in Creature::CREATURES {
//             if state.creatures.has(creature) || self.graveyard.has(creature) {
//                 continue;
//             }
//
//             out[creature as usize] = Some(index);
//             index += 1;
//         }
//
//         out
//     }
//
//     pub fn main_phase_choices<A: Allocator>(
//         &self,
//         player: Player,
//         out: &mut Vec<MainPhaseChoice, A>,
//     ) {
//         out.clear();
//
//         let state = self.get_player_state(player);
//         let seer_is_active = state.effects.has(PlayerStatusEffect::Seer);
//
//         for creature in Creature::CREATURES {
//             if !state.creatures.has(creature) {
//                 continue;
//             }
//
//             for edict in Edict::EDICTS {
//                 if !state.edicts.has(edict) {
//                     continue;
//                 }
//
//                 if seer_is_active {
//                     let creature_index = creature as usize;
//
//                     // Try to avoid duplicate pairs
//                     for extra_creature in &Creature::CREATURES[0..creature_index] {
//                         if !state.creatures.has(*extra_creature) {
//                             continue;
//                         }
//
//                         out.push(MainPhaseChoice {
//                             edict,
//                             creatures: (creature, Some(*extra_creature)),
//                         })
//                     }
//                 } else {
//                     out.push(MainPhaseChoice {
//                         edict,
//                         creatures: (creature, None),
//                     })
//                 }
//             }
//         }
//     }
//
//     // pub fn available_actions(&self, out: &mut Vec<PhaseTransition>) {
//     //     *out = vec![];
//     //
//     //     match self.phase {
//     //         HiddenPhase::SabotagePhase(_, _) => {
//     //             for creature in Creature::CREATURES {
//     //                 if self.graveyard.0.has(creature as usize) {
//     //                     continue;
//     //                 } else if self.player_states.0.creatures.0.has(creature as usize) {
//     //                     continue;
//     //                 }
//     //
//     //                 choices.push(PhaseTransition::Sabotage(creature))
//     //             }
//     //         }
//     //         HiddenPhase::Seer(main_choices, _) => {
//     //             let creatures = main_choices.0.creatures;
//     //             choices.push(PhaseTransition::Seer(creatures.0));
//     //             if let Some(secondary_pick) = creatures.1 {
//     //                 choices.push(PhaseTransition::Seer(secondary_pick))
//     //             } else {
//     //                 panic!("Invalid seer phase with single creature on the table.")
//     //             }
//     //         }
//     //     }
//     //     choices
//     // }
// }
// // }}}
// // {{{ Complete game state
// #[derive(PartialEq, Eq, Hash, Debug, Clone)]
// pub enum CompleteGameState {
//     Finished(Score),
//     Unfinished(GameState),
// }
//
// impl CompleteGameState {
//     pub fn is_finished(&self) -> bool {
//         match self {
//             CompleteGameState::Finished(_) => true,
//             CompleteGameState::Unfinished(_) => false,
//         }
//     }
//     pub fn to_game_state(self) -> Option<GameState> {
//         match self {
//             CompleteGameState::Finished(_) => None,
//             CompleteGameState::Unfinished(result) => Some(result),
//         }
//     }
// }
// // }}}
