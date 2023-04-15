use crate::game::types::{CreatureSet, EdictSet, PlayerStatusEffects};
use crate::helpers::swap::Swap;
use std::alloc::Allocator;
use std::fmt::{self, Display};
use std::hash::Hash;

use super::types::{
    Battlefield, Creature, Edict, GlobalStatusEffect, GlobalStatusEffects, Player,
    PlayerStatusEffect,
};

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
            effects: PlayerStatusEffects::new(),
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
    edict: Edict,
    creature: Creature,
}
// }}}
// {{{ Phase
// The number of main phases is always 2
type MainPhaseChoices = (MainPhaseChoice, MainPhaseChoice);

// The number of sabotage phases per turn varies between 0-2
type SabotagePhaseChoices = (Option<Creature>, Option<Creature>);

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
// {{{ Score
// Player 1 score - player 2 score
// - Negative => player 2 won
// - Positive => player 1 won
// - 0 => draw
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Score(pub i8);
// }}}
// {{{ Battlefields
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Battlefields {
    pub all: [Battlefield; 4],
    pub current: u8,
}

impl Battlefields {
    pub fn new(all: [Battlefield; 4]) -> Self {
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
        &self.all[(self.current as usize)..]
    }

    pub fn current(&self) -> Battlefield {
        self.all[self.current as usize]
    }
}
// }}}
// {{{ Battle context & result
// Context for resolving a battle, including data already present in GameState
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct FullBattleContext {
    main_choices: (FinalMainPhaseChoice, FinalMainPhaseChoice),
    sabotage_choices: SabotagePhaseChoices,
    player_states: (PlayerState, PlayerState),
    effects: GlobalStatusEffects,
    battlefield: Battlefield,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum BattleResult {
    Lost,
    Tied,
    Won,
}

impl BattleResult {
    pub fn flip(self) -> Self {
        match self {
            BattleResult::Lost => BattleResult::Won,
            BattleResult::Tied => BattleResult::Tied,
            BattleResult::Won => BattleResult::Lost,
        }
    }
}

impl FullBattleContext {
    // Flips the context to the perspective of the opponent
    fn flip(&self) -> Self {
        FullBattleContext {
            main_choices: self.main_choices.swap(),
            sabotage_choices: self.sabotage_choices.swap(),
            player_states: self.player_states.swap(),
            ..*self
        }
    }

    // Returns the edict played by the current player
    #[inline]
    fn current_edict(&self) -> Edict {
        self.main_choices.0.edict
    }

    // Returns the edict played by the opponent
    #[inline]
    fn other_edict(&self) -> Edict {
        self.main_choices.1.edict
    }

    // Returns the creature played by the current player
    #[inline]
    fn current_creature(&self) -> Creature {
        self.main_choices.0.creature
    }

    // Returns the creature played by the other player
    #[inline]
    fn other_creature(&self) -> Creature {
        self.main_choices.1.creature
    }

    // Checks if the main creature we played is negated
    #[inline]
    fn creature_is_negated(&self) -> bool {
        self.other_creature() == Creature::Witch
            || (self.current_creature() == Creature::Seer
                && self.other_creature() == Creature::Rogue)
    }

    // Returns true if the given creature is the one we have played
    // and if it's effect has not been negated
    #[inline]
    fn active_creature(&self, creature: Creature) -> bool {
        creature == self.current_creature() && !self.creature_is_negated()
    }

    // Calculates the edict multiplier for the current player
    fn edict_multiplier(&self) -> u8 {
        let mut result = 1;

        if self.battlefield == Battlefield::Urban {
            result += 1;
        }

        if self.active_creature(Creature::Steward) {
            result += 1;
        }

        result
    }

    // Calculates the strength modifier for the creature the current player has played
    fn strength_modifier(&self) -> i8 {
        let effects = self.player_states.0.effects.0;
        let mut result: i8 = 0;

        // Battlefield bonuses:
        if self.battlefield.bonus(self.current_creature()) {
            result += 2;
        }

        // Creature strength bonuses:
        if !self.creature_is_negated() {
            // - Ranger
            if self.current_creature() == Creature::Ranger
                && self.battlefield.bonus(self.current_creature())
                && !(self.battlefield.bonus(self.other_creature()))
            {
                result += 2;
            }
            // - Barbarian
            else if effects.has(PlayerStatusEffect::Barbarian as u8)
                && self.current_creature() == Creature::Barbarian
            {
                result += 2;
            }
        }

        // Edict strength bonuses:
        // (the witch cannot get strength bonuses from edicts)
        if self.current_creature() != Creature::Witch {
            result += self.edict_multiplier() as i8
                * match self.current_edict() {
                    Edict::Sabotage if Some(self.other_creature()) == self.sabotage_choices.0 => 3,
                    Edict::Ambush if self.battlefield.bonus(self.current_creature()) => 1,
                    Edict::Gambit => 1,
                    _ => 0,
                }
        }

        // Lingering effects which modify strength:
        // Effects caused by the previously played creature
        if effects.has(PlayerStatusEffect::Bard as u8) {
            result += 1;
        } else if effects.has(PlayerStatusEffect::Mercenary as u8) {
            result -= 1;
        }
        // Effects caused by previous battlefields
        if effects.has(PlayerStatusEffect::Mountain as u8) {
            result += 1;
        }

        result
    }

    // Calculate strength modifiers for both players
    fn strength_modifiers(&self) -> (i8, i8) {
        (self.strength_modifier(), self.flip().strength_modifier())
    }

    // Check if the current player wins because of an effect
    fn wins_by_effect(&self) -> bool {
        if self.creature_is_negated() {
            return false;
        }

        // The wall gets negated by the witch and rogue characters
        if self.other_creature() == Creature::Wall
            && (self.current_creature() == Creature::Witch
                || self.current_creature() == Creature::Rogue)
        {
            return true;
        }

        // The rogue wins against the monarch
        if self.current_creature() == Creature::Rogue && self.other_creature() == Creature::Monarch
        {
            return true;
        }

        // The diplomat wins against any creature
        // if the two edicts are identical
        if self.current_creature() == Creature::Diplomat
            && self.current_edict() == self.main_choices.1.edict
        {
            return true;
        }

        return false;
    }

    // Resolves the gambit effects on a tie
    fn resolve_gambits(&self) -> BattleResult {
        // If both players played gambits, nothing happens
        if self.current_edict() == self.other_edict() {
            return BattleResult::Tied;
        }

        // if we played a gambit, we lose on ties
        if self.current_edict() == Edict::Gambit {
            return BattleResult::Lost;
        }

        // if the opponent has played a gambit, they lose on ties
        if self.other_edict() == Edict::Gambit {
            return BattleResult::Won;
        }

        // Otherwise it's a draw
        BattleResult::Tied
    }

    fn battle_result(&self) -> BattleResult {
        if self.wins_by_effect() {
            return BattleResult::Won;
        } else if self.flip().wins_by_effect() {
            return BattleResult::Lost;
        }
        // The wall can force ties
        else if self.current_creature() == Creature::Wall
            || self.other_creature() == Creature::Wall
        {
            return self.resolve_gambits();
        }

        let base_strengths = (
            self.current_creature().strength() as i8,
            self.other_creature().strength() as i8,
        );

        let strength_modifiers = self.strength_modifiers();

        let strengths = (
            base_strengths.0 + strength_modifiers.0,
            base_strengths.1 + strength_modifiers.1,
        );

        return if strengths.0 < strengths.1 {
            BattleResult::Lost
        } else if strengths.0 > strengths.1 {
            BattleResult::Won
        } else {
            self.resolve_gambits()
        };
    }

    // Calculate the amount of victory points
    // the value of the current battle changes
    // because of our own cards.
    fn edict_reward(&self) -> i8 {
        self.edict_multiplier() as i8
            * if self.current_edict() == Edict::RileThePublic {
                1
            } else if self.current_edict() == Edict::DivertAttention
                && self.other_edict() != Edict::RileThePublic
            {
                -1
            } else {
                0
            }
    }

    // Calculates the amount of victory points
    // earned by winning this partidcular battle
    fn battle_win_reward(&self) -> u8 {
        let effects = self.player_states.0.effects.0;
        let mut total = self.battlefield.reward();

        // Global lingering effects:
        if self.effects.0.has(GlobalStatusEffect::Night as u8) {
            total += 1;
        }

        // Local lingering effects:
        if effects.has(PlayerStatusEffect::Bard as u8) {
            total += 1;
        }

        if effects.has(PlayerStatusEffect::Glade as u8) {
            total += 2;
        }

        // Apply the "rile the public" and "divert attention" edict
        // This is the only place where the total can decrease,
        // which is why we must be careful for it not to become negative.
        total = i8::max(
            0,
            total as i8 + self.edict_reward() + self.flip().edict_reward(),
        ) as u8;

        total
    }

    // The reward for killing the monarch
    fn monarch_reward(&self, result: BattleResult) -> u8 {
        match result {
            BattleResult::Won | BattleResult::Tied
                if self.flip().active_creature(Creature::Monarch) =>
            {
                2
            }
            _ => 0,
        }
    }

    // Calculates the delta we need to change the score by.
    // - positive values mean we've earned points
    // - negative values mean we've lost points
    fn battle_score_delta(&self, result: BattleResult) -> i8 {
        let mut delta = match result {
            BattleResult::Tied => 0,
            BattleResult::Won => self.battle_win_reward() as i8,
            BattleResult::Lost => -(self.flip().battle_win_reward() as i8),
        };

        // Trigger monarch's effect
        delta += self.monarch_reward(result) as i8;
        delta -= self.flip().monarch_reward(result.flip()) as i8;

        delta
    }

    pub fn advance_game_state(&self, game_state: &GameState) -> CompleteGameState {
        let battle_result = self.battle_result();
        // assert_eq!(battle_result, self.flip().battle_result().flip());

        let score_delta = self.battle_score_delta(battle_result);
        // assert_eq!(
        //     score_delta,
        //     -self.flip().battle_score_delta(battle_result.flip())
        // );

        let score = Score(game_state.score.0 + score_delta);

        return match game_state.battlefields.next() {
            Some(battlefields) =>
            // Continue game
            {
                let mut new_game_state = GameState {
                    battlefields,
                    score,
                    phase: Phase::Main1,
                    ..*game_state
                };

                new_game_state
                    .graveyard
                    .0
                    .add(self.current_creature() as u8);
                new_game_state.graveyard.0.add(self.other_creature() as u8);

                let p1 = &mut new_game_state.player_states.0;
                let p2 = &mut new_game_state.player_states.1;

                // Discard used creatures
                p1.creatures.0.remove(self.current_creature() as u8);
                p2.creatures.0.remove(self.other_creature() as u8);

                // Discard used edicts
                p1.edicts.0.remove(self.current_edict() as u8);
                p2.edicts.0.remove(self.other_edict() as u8);

                // Clear status effects
                p1.effects.0.clear();
                p2.effects.0.clear();
                new_game_state.effects.0.clear();

                // Resolve the Steward effect
                if self.current_creature() == Creature::Steward && !self.creature_is_negated() {
                    p1.edicts.0.fill();
                } else if self.other_creature() == Creature::Steward
                    && !self.flip().creature_is_negated()
                {
                    p2.edicts.0.fill();
                }

                // Set up global lingering effects
                if self.battlefield == Battlefield::Night {
                    new_game_state
                        .effects
                        .0
                        .add(GlobalStatusEffect::Night as u8);
                }

                // first is winner, second is loser
                let player_by_status = match battle_result {
                    BattleResult::Won => Some((p1, p2)),
                    BattleResult::Lost => Some((p2, p1)),
                    BattleResult::Tied => None,
                };

                if let Some((winner, loser)) = player_by_status {
                    // Set up battlefield lingering effects
                    // - Glade:
                    if self.battlefield == Battlefield::Glade {
                        winner.effects.0.add(PlayerStatusEffect::Glade as u8);
                    }
                    // - Mountain
                    if self.battlefield == Battlefield::Mountain {
                        winner.effects.0.add(PlayerStatusEffect::Mountain as u8);
                    }

                    // Set up creature lingering effects
                    // - Barbarian
                    // if this card has already been played there's no point
                    // in adding the status effect anymore
                    if !new_game_state.graveyard.0.has(Creature::Barbarian as u8) {
                        loser.effects.0.add(PlayerStatusEffect::Barbarian as u8)
                    }
                }

                let p1 = &mut new_game_state.player_states.0;
                let p2 = &mut new_game_state.player_states.1;

                let creatures = [
                    (Creature::Mercenary, PlayerStatusEffect::Mercenary),
                    (Creature::Seer, PlayerStatusEffect::Seer),
                    (Creature::Bard, PlayerStatusEffect::Bard),
                ];

                // - Mercenary
                for (creature, effect) in creatures {
                    if self.active_creature(creature) {
                        p1.effects.0.add(effect as u8)
                    } else if self.flip().active_creature(creature) {
                        p2.effects.0.add(effect as u8)
                    }
                }

                CompleteGameState::Unfinished(new_game_state)
            }
            // Report final results
            None => CompleteGameState::Finished(score),
        };
    }
}
// }}}
// {{{ Game state
// Fully determined game state
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct GameState {
    pub score: Score,
    // Player specific state
    pub player_states: (PlayerState, PlayerState),
    // All the creature played so far
    pub graveyard: CreatureSet,
    // The one creature which neither player has drawn
    pub overseer: Creature,
    // Lingering effects
    pub effects: GlobalStatusEffects,
    // Stack of battlefields.
    pub battlefields: Battlefields,
    // The next "decision" one of the players has to take
    // the player states are always arranged in such a way
    // to ensure the first player is the one taking the current decision.
    pub phase: Phase,
}

impl GameState {
    // change the phase we are in
    pub fn switch_to(&self, phase: Phase) -> CompleteGameState {
        CompleteGameState::Unfinished(GameState {
            battlefields: self.battlefields.clone(),
            phase,
            ..*self
        })
    }

    // change the phase we are in while also flipping the player
    pub fn flip_to(&self, phase: Phase) -> CompleteGameState {
        CompleteGameState::Unfinished(GameState {
            score: Score(-self.score.0),
            player_states: self.player_states.swap(),
            battlefields: self.battlefields.clone(),
            phase,
            ..*self
        })
    }

    fn resolve_battle(
        &self,
        main_choices: (FinalMainPhaseChoice, FinalMainPhaseChoice),
        sabotage_choices: SabotagePhaseChoices,
    ) -> CompleteGameState {
        let context = FullBattleContext {
            main_choices,
            sabotage_choices,
            battlefield: self.battlefields.current(),
            effects: self.effects,
            player_states: self.player_states,
        };

        context.advance_game_state(self)
    }

    fn resolve_sabotage(
        &self,
        main_choices: MainPhaseChoices,
        sabotage_choices: SabotagePhaseChoices,
    ) -> (CompleteGameState, bool) {
        if main_choices.0.creatures.1.is_some() {
            (
                self.switch_to(Phase::Seer(main_choices, sabotage_choices)),
                false,
            )
        } else if main_choices.1.creatures.1.is_some() {
            (
                self.flip_to(Phase::Seer(main_choices.swap(), sabotage_choices.swap())),
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

    pub fn apply_transition(
        &self,
        transition: PhaseTransition,
    ) -> Option<(CompleteGameState, bool)> {
        match (self.phase, transition) {
            (Phase::Main1, PhaseTransition::Main(choice)) => {
                Some((self.flip_to(Phase::Main2(choice)), true))
            }
            (Phase::Main2(first_choice), PhaseTransition::Main(second_choice)) => Some({
                let choices = (second_choice, first_choice);
                if second_choice.edict == Edict::Sabotage {
                    (self.switch_to(Phase::SabotagePhase1(choices)), false)
                } else if first_choice.edict == Edict::Sabotage {
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

    #[inline]
    fn get_player_state(&self, player: Player) -> PlayerState {
        match player {
            Player::Me => self.player_states.0,
            Player::You => self.player_states.1,
        }
    }

    /// Prepares the overseer candiates for a decision node.
    /// See the docs there for more details.
    pub fn get_overseer_candiates(&self, player: Player) -> [Option<u8>; 11] {
        let mut out: [Option<u8>; 11] = Default::default();
        let state = self.get_player_state(player);

        let mut index = 0;

        for creature in Creature::CREATURES {
            if state.creatures.has(creature) || self.graveyard.has(creature) {
                continue;
            }

            out[creature as usize] = Some(index);
            index += 1;
        }

        out
    }

    pub fn main_phase_choices<A: Allocator>(
        &self,
        player: Player,
        out: &mut Vec<MainPhaseChoice, A>,
    ) {
        out.clear();

        let state = self.get_player_state(player);
        let seer_is_active = state.effects.has(PlayerStatusEffect::Seer);

        for creature in Creature::CREATURES {
            if !state.creatures.has(creature) {
                continue;
            }

            for edict in Edict::EDICTS {
                if !state.edicts.has(edict) {
                    continue;
                }

                if seer_is_active {
                    let creature_index = creature as usize;

                    // Try to avoid duplicate pairs
                    for extra_creature in &Creature::CREATURES[0..creature_index] {
                        if !state.creatures.has(*extra_creature) {
                            continue;
                        }

                        out.push(MainPhaseChoice {
                            edict,
                            creatures: (creature, Some(*extra_creature)),
                        })
                    }
                } else {
                    out.push(MainPhaseChoice {
                        edict,
                        creatures: (creature, None),
                    })
                }
            }
        }
    }

    // pub fn available_actions(&self, out: &mut Vec<PhaseTransition>) {
    //     *out = vec![];
    //
    //     match self.phase {
    //         HiddenPhase::SabotagePhase(_, _) => {
    //             for creature in Creature::CREATURES {
    //                 if self.graveyard.0.has(creature as u8) {
    //                     continue;
    //                 } else if self.player_states.0.creatures.0.has(creature as u8) {
    //                     continue;
    //                 }
    //
    //                 choices.push(PhaseTransition::Sabotage(creature))
    //             }
    //         }
    //         HiddenPhase::Seer(main_choices, _) => {
    //             let creatures = main_choices.0.creatures;
    //             choices.push(PhaseTransition::Seer(creatures.0));
    //             if let Some(secondary_pick) = creatures.1 {
    //                 choices.push(PhaseTransition::Seer(secondary_pick))
    //             } else {
    //                 panic!("Invalid seer phase with single creature on the table.")
    //             }
    //         }
    //     }
    //     choices
    // }
}
// }}}
// {{{ Complete game state
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum CompleteGameState {
    Finished(Score),
    Unfinished(GameState),
}

impl CompleteGameState {
    pub fn is_finished(&self) -> bool {
        match self {
            CompleteGameState::Finished(_) => true,
            CompleteGameState::Unfinished(_) => false,
        }
    }
    pub fn to_game_state(self) -> Option<GameState> {
        match self {
            CompleteGameState::Finished(_) => None,
            CompleteGameState::Unfinished(result) => Some(result),
        }
    }
}
// }}}
