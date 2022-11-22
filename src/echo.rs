#![allow(dead_code)]

use super::helpers::Swap;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use smallvec::{smallvec, SmallVec};
use std::fmt;
use std::hash::Hash;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Battlefield {
    Mountain,
    Glade,
    Urban,
    LastStrand,
    Night,
    Plains,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
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

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
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
    const BATTLEFIELDS: [Battlefield; 6] = [Mountain, Glade, Urban, Night, LastStrand, Plains];

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
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
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
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum GlobalStatusEffect {
    Night,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct Bitfield(u16);

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct CreatureSet(Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct EdictSet(Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PlayerStatusEffects(Bitfield);
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct GlobalStatusEffects(Bitfield);

impl Bitfield {
    pub fn new() -> Self {
        Bitfield(0)
    }
    pub fn has(self, index: u8) -> bool {
        ((self.0 >> (index as u16)) & 1) != 0
    }

    pub fn add(&mut self, index: u8) {
        self.0 = self.0 | (1 << (index as u16))
    }

    pub fn remove(&mut self, index: u8) {
        if !self.has(index) {
            panic!(
                "Trying to remove index {} that's not here {:b}",
                index, self.0
            )
        }
        self.0 = self.0 ^ (1 << (index as u16))
    }

    pub fn fill(&mut self) {
        self.0 = u16::MAX;
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

impl fmt::Debug for Bitfield {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

impl Into<u64> for Bitfield {
    fn into(self) -> u64 {
        return self.0.into();
    }
}

// State involving only one of the players
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct PlayerState {
    creatures: CreatureSet,
    edicts: EdictSet,
    effects: PlayerStatusEffects,
}

impl PlayerState {
    pub fn new(creatures: CreatureSet) -> Self {
        let edicts = Bitfield(63);

        PlayerState {
            creatures,
            edicts: EdictSet(edicts),
            effects: PlayerStatusEffects(Bitfield::new()),
        }
    }

    // Return only the infromation the current player should have acceess to
    pub fn conceal(&self) -> HiddenPlayerState {
        return HiddenPlayerState {
            edicts: self.edicts,
            effects: self.effects,
        };
    }
}

// What a player "knows" about it's opponent
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
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
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
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
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct FinalMainPhaseChoice {
    edict: Edict,
    creature: Creature,
}

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
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
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
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum PhaseTransition {
    Main(MainPhaseChoice),
    Sabotage(Creature),
    Seer(Creature),
}

// Player 1 score - player 2 score
// - Negative => player 2 won
// - Positive => player 1 won
// - 0 => draw
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Score(pub i8);

// Stack of battlefields for a match.
// The tail of the list represents the current battle.
// Elements get popped as battles take place.
type Battlefields = SmallVec<[Battlefield; 4]>;

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
        assert_eq!(battle_result, self.flip().battle_result().flip());

        let score_delta = self.battle_score_delta(battle_result);
        assert_eq!(
            score_delta,
            -self.flip().battle_score_delta(battle_result.flip())
        );

        let score = Score(game_state.score.0 + score_delta);

        let mut battlefields = game_state.battlefields.clone();
        battlefields.pop();

        // Continue game
        return if battlefields.len() > 0 {
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
        else {
            CompleteGameState::Finished(score)
        };
    }
}

// Fully determined game state
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
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
    pub battlefields: Battlefields,
    // The next "decision" one of the players has to take
    // the player states are always arranged in such a way
    // to ensure the first player is the one taking the current decision.
    phase: Phase,
}

impl GameState {
    pub fn new() -> Self {
        let rng = &mut thread_rng();

        let mut cards = Vec::from(Creature::CREATURES);
        cards.shuffle(rng);
        cards = vec![
            Rogue, Witch, Diplomat, Seer, Barbarian, Monarch, Ranger, Wall, Bard, Steward,
            Mercenary,
        ];
        let overseer = cards.pop().unwrap();

        let mut battlefields = Vec::from(Battlefield::BATTLEFIELDS);
        battlefields.pop();
        battlefields.pop();
        battlefields.pop();
        battlefields.pop();
        battlefields.shuffle(rng);
        let battlefields = smallvec![
            Battlefield::Night,
            Battlefield::Urban,
            Battlefield::Mountain,
            Battlefield::LastStrand,
        ];

        let mut p1_cards = CreatureSet(Bitfield::new());
        let mut p2_cards = CreatureSet(Bitfield::new());

        for (index, creature) in cards.iter().enumerate() {
            if index < 5 {
                p1_cards.0.add(*creature as u8);
            } else {
                p2_cards.0.add(*creature as u8);
            }
        }

        GameState {
            score: Score(0),
            player_states: (PlayerState::new(p1_cards), PlayerState::new(p2_cards)),
            graveyard: CreatureSet(Bitfield::new()),
            overseer,
            effects: GlobalStatusEffects(Bitfield::new()),
            battlefields,
            phase: Phase::Main1,
        }
    }

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
            battlefield: *self.battlefields.last().unwrap(),
            effects: self.effects,
            player_states: self.player_states,
        };

        let result = context.advance_game_state(self);
        // let other = context
        //     .flip()
        //     .advance_game_state(&self.flip_to(Phase::Main1).to_game_state().unwrap());
        //
        // match (result, other) {
        //     (CompleteGameState::Unfinished(state1), CompleteGameState::Unfinished(state2)) => {
        //         if state1.switch_to(Phase::Main1) == state2.flip_to(Phase::Main1) {
        //             CompleteGameState::Unfinished(state1)
        //             // state2.flip_to(Phase::Main1)
        //         } else {
        //             panic!("Ended with different states {:?} {:?}", state1, state2)
        //         }
        //     }
        //     (CompleteGameState::Finished(score1), CompleteGameState::Finished(score2)) => {
        //         if score1.0 == -score2.0 {
        //             CompleteGameState::Finished(score1)
        //         } else {
        //             panic!("Ended with different scopres {:?} {:?}", score1, score2)
        //         }
        //     }
        //     (a, b) => panic!("Different states entirely {:?} {:?}", a, b),
        // }

        result
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
}

// Game state which only contains knowedge the current player
// has of the game.
// Eg: there's no info about the cards the opponent has in hand
//
// Main differences are:
// - opponent's state is hidden
// - the identity of the overseer is hidden
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
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
                    if !self.player_states.0.creatures.0.has(creature as u8) {
                        continue;
                    }

                    for edict in Edict::EDICTS {
                        if !self.player_states.0.edicts.0.has(edict as u8) {
                            continue;
                        }

                        if seer_is_active {
                            for extra_creature in Creature::CREATURES {
                                if !self.player_states.0.creatures.0.has(extra_creature as u8) {
                                    continue;
                                }

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

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum CompleteGameState {
    Finished(Score),
    Unfinished(GameState),
}

impl CompleteGameState {
    pub fn to_game_state(self) -> Option<GameState> {
        match self {
            CompleteGameState::Finished(_) => None,
            CompleteGameState::Unfinished(result) => Some(result),
        }
    }
}
