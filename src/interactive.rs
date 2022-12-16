#![allow(dead_code)]
use crate::{
    echo::{
        Battlefield, Battlefields, Bitfield, Creature, CreatureSet, Edict, EdictSet,
        GlobalStatusEffect, GlobalStatusEffects, HiddenPhase, HiddenPlayerState, InfoSet,
        MainPhaseChoice, PhaseTransition, PlayerState, PlayerStatusEffect, PlayerStatusEffects,
        Score,
    },
    train::{train, utility_to_percentage, Context, TrainingOptions},
};
use dialoguer::{Confirm, Input, MultiSelect, Select};
use rand::{Rng, RngCore};
use smallvec::SmallVec;

pub fn get_edict() -> Edict {
    let items: SmallVec<[Edict; 5]> = SmallVec::from(Edict::EDICTS);

    loop {
        match Select::new().items(&items).interact() {
            Ok(index) => {
                return items[index];
            }
            _ => {
                println!("Something went wrong, try again")
            }
        }
    }
}

pub fn get_battlefields() -> Battlefields {
    let mut all: [Battlefield; 4] = [Battlefield::Plains; 4];
    let mut items = Vec::from(Battlefield::BATTLEFIELDS);

    println!("What turn is this?");
    let current = get_int();

    for i in 0..4 {
        loop {
            match Select::new().items(&items).interact() {
                Ok(index) => {
                    all[i] = items[index];
                    items.remove(index);
                    break;
                }
                _ => {
                    println!("Something went wrong, try again")
                }
            }
        }
    }

    Battlefields {
        all,
        current: current as u8,
    }
}

pub fn get_creature(choose_from: CreatureSet) -> Creature {
    let mut items: SmallVec<[Creature; 11]> = SmallVec::new();

    for creature in Creature::CREATURES {
        if choose_from.0.has(creature as u8) {
            items.push(creature);
        }
    }

    loop {
        match Select::new().items(&items).interact() {
            Ok(index) => {
                return items[index];
            }
            _ => {
                println!("Something went wrong, try again")
            }
        }
    }
}

pub fn get_creature_set(choose_from: CreatureSet, by_default: CreatureSet) -> CreatureSet {
    let mut items: SmallVec<[Creature; 11]> = SmallVec::new();
    let mut defaults: SmallVec<[bool; 11]> = SmallVec::new();

    for creature in Creature::CREATURES {
        if choose_from.0.has(creature as u8) {
            items.push(creature);
            defaults.push(by_default.0.has(creature as u8));
        }
    }

    loop {
        match MultiSelect::new()
            .items(&items)
            .defaults(&defaults)
            .interact()
        {
            Ok(indices) => {
                let mut result = Bitfield::new();

                for index in indices {
                    result.add(items[index] as u8);
                }

                return CreatureSet(result);
            }
            _ => {
                println!("Something went wrong, try again")
            }
        }
    }
}

pub fn get_edict_set(choose_from: EdictSet, by_default: EdictSet) -> EdictSet {
    let mut items: SmallVec<[Edict; 5]> = SmallVec::new();
    let mut defaults: SmallVec<[bool; 5]> = SmallVec::new();

    for edict in Edict::EDICTS {
        if choose_from.0.has(edict as u8) {
            items.push(edict);
            defaults.push(by_default.0.has(edict as u8));
        }
    }

    loop {
        match MultiSelect::new()
            .items(&items)
            .defaults(&defaults)
            .interact()
        {
            Ok(indices) => {
                let mut result = Bitfield::new();

                for index in indices {
                    result.add(items[index] as u8);
                }

                return EdictSet(result);
            }
            _ => {
                println!("Something went wrong, try again")
            }
        }
    }
}

pub fn get_effect_set(
    choose_from: PlayerStatusEffects,
    by_default: PlayerStatusEffects,
) -> PlayerStatusEffects {
    let mut items: SmallVec<[PlayerStatusEffect; 6]> = SmallVec::new();
    let mut defaults: SmallVec<[bool; 6]> = SmallVec::new();

    for effect in PlayerStatusEffect::PLAYER_STATUS_EFFECTS {
        if choose_from.0.has(effect as u8) {
            items.push(effect);
            defaults.push(by_default.0.has(effect as u8));
        }
    }

    loop {
        match MultiSelect::new()
            .items(&items)
            .defaults(&defaults)
            .interact()
        {
            Ok(indices) => {
                let mut result = Bitfield::new();

                for index in indices {
                    result.add(items[index] as u8);
                }

                return PlayerStatusEffects(result);
            }
            _ => {
                println!("Something went wrong, try again")
            }
        }
    }
}

pub fn get_initial_infoset() -> InfoSet {
    println!("Select the cards you have in hand:");
    let creatures = get_creature_set(CreatureSet::all(), CreatureSet(Bitfield::new()));

    println!("Select the battlefields the match will take place in:");
    let battlefields = get_battlefields();

    InfoSet {
        player_states: (PlayerState::new(creatures), HiddenPlayerState::new()),
        phase: crate::echo::HiddenPhase::Main,
        score: Score(0),
        graveyard: CreatureSet(Bitfield::new()),
        effects: GlobalStatusEffects(Bitfield::new()),
        battlefields,
    }
}

fn get_int() -> i8 {
    loop {
        match Input::new().default(0).interact_text() {
            Ok(num) => {
                return num;
            }
            _ => {
                println!("Try again")
            }
        }
    }
}

pub fn get_infoset(initial_infoset: &InfoSet) -> InfoSet {
    let phase = HiddenPhase::Main;

    println!("Input the graveyard:");
    let graveyard = get_creature_set(CreatureSet::all(), initial_infoset.graveyard);

    println!("Input the score:");
    let score = Score(get_int());

    println!("Enter the battlefields");
    let battlefields = get_battlefields();

    let mut effects = GlobalStatusEffects(Bitfield::new());
    if Confirm::new()
        .with_prompt("Is the night status effect active?")
        .interact()
        .unwrap()
    {
        effects.0.add(GlobalStatusEffect::Night as u8);
    }

    println!("Select the cards you have in hand:");
    let creatures_in_hand = get_creature_set(
        CreatureSet::all(),
        initial_infoset.player_states.0.creatures,
    );

    println!("Select your edicts:");
    let edicts_in_hand = get_edict_set(EdictSet::all(), initial_infoset.player_states.0.edicts);

    println!("Select your status effects:");
    let your_effects = get_effect_set(
        PlayerStatusEffects::all(),
        PlayerStatusEffects(Bitfield::new()),
    );

    println!("Select opponent's edicts:");
    let opponent_edicts = get_edict_set(EdictSet::all(), initial_infoset.player_states.1.edicts);

    println!("Select your opponent's effects:");
    let opponent_effects = get_effect_set(
        PlayerStatusEffects::all(),
        PlayerStatusEffects(Bitfield::new()),
    );

    InfoSet {
        player_states: (
            PlayerState {
                edicts: edicts_in_hand,
                creatures: creatures_in_hand,
                effects: your_effects,
            },
            HiddenPlayerState {
                edicts: opponent_edicts,
                effects: opponent_effects,
            },
        ),
        phase,
        score,
        graveyard,
        effects,
        battlefields,
    }
}

fn play_sabotage<R: RngCore>(
    context: &mut Context<R>,
    info_set: &InfoSet,
    choice: MainPhaseChoice,
) -> InfoSet {
    println!("Select the edict your opponent played");
    let opponent_edict = get_edict();
    let mut new_info_set = info_set.clone();
    new_info_set.phase = HiddenPhase::SabotagePhase(choice, opponent_edict);

    let my_sabotage = context
        .make_choice(&new_info_set)
        .unwrap()
        .to_sabotage()
        .unwrap();

    println!("Your action: {}", PhaseTransition::Sabotage(my_sabotage));

    if choice.creatures.1.is_some() {
        play_seer(
            context,
            &new_info_set,
            choice,
            opponent_edict,
            (Some(my_sabotage), get_oppponent_sabotage(&new_info_set)),
        )
    } else {
        new_info_set
    }
}

fn play_seer<R: RngCore>(
    context: &mut Context<R>,
    info_set: &InfoSet,
    choice: MainPhaseChoice,
    opponent_edict: Edict,
    sabotage_choices: (Option<Creature>, Option<Creature>),
) -> InfoSet {
    println!("Select the creature your opponent played");

    let choose_from = info_set
        .graveyard
        .0
        .union(&info_set.player_states.0.creatures.0)
        .invert();
    let opponent_creature = get_creature(CreatureSet(choose_from));

    let mut new_info_set = info_set.clone();
    new_info_set.phase = HiddenPhase::Seer(
        (
            choice,
            MainPhaseChoice {
                edict: opponent_edict,
                creatures: (opponent_creature, None),
            },
        ),
        sabotage_choices,
    );

    let action = context
        .make_choice(&new_info_set)
        .unwrap()
        .to_seer()
        .unwrap();
    println!("Your action: {}", PhaseTransition::Seer(action));

    new_info_set
}

fn get_oppponent_sabotage(info_set: &InfoSet) -> Option<Creature> {
    println!("Did your opponent sabotage?");
    let result = Confirm::new().interact().unwrap();
    let opponent_sabotage = if result {
        println!("What did the opponent sabotage?");
        let choose_from = info_set.graveyard.others();
        Some(get_creature(choose_from))
    } else {
        None
    };

    opponent_sabotage
}

fn play_turn<R: RngCore>(context: &mut Context<R>, info_set: &InfoSet) -> InfoSet {
    let action = context.make_choice(&info_set).unwrap().to_main().unwrap();

    println!("Your action: {}", PhaseTransition::Main(action));

    match action.edict {
        Edict::Sabotage => play_sabotage(context, info_set, action),
        _ => {
            if action.creatures.1.is_some() {
                println!("Choose your opponent's edict");
                let opponent_edict = get_edict();
                play_seer(
                    context,
                    info_set,
                    action,
                    opponent_edict,
                    (None, get_oppponent_sabotage(info_set)),
                )
            } else {
                info_set.clone()
            }
        }
    }

    //
    // let opponent_choices = info_set.graveyard.0.union(&info_set.player_states.0.creatures.0).invert();
    // let opponent_action = get_creature_set(CreatureSet(opponent_choices), CreatureSet(Bitfield::new()));

    // let opponent_edict =
}

pub fn play_game<R: RngCore + Rng>(initial_state: &InfoSet, might_be_start: bool, rng: &mut R) {
    let is_start = might_be_start
        && Confirm::new()
            .with_prompt("Is this the start of a game?")
            .interact()
            .unwrap();

    let mut info_set = if is_start {
        initial_state.clone()
    } else {
        get_infoset(initial_state)
    };

    info_set.phase = HiddenPhase::Main;

    let (utility, mut context) = train(
        TrainingOptions {
            pruning_threshold: Some(0.01),
            board_evaluation: crate::train::BoardEvaluation::MonteCarlo {
                iterations: 50,
                max_depth: 1,
            },
            starting_infoset: info_set.clone(),
        },
        100,
        rng,
    );

    println!(
        "You have a {:?} chance of winning against an optimal player",
        utility_to_percentage(utility)
    );

    let next_info_set = play_turn(&mut context, &info_set);

    if Confirm::new().with_prompt("Continue?").interact().unwrap() {
        play_game(&next_info_set, false, context.rng);
    }
}
