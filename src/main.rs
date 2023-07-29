#![feature(allocator_api)]
#![feature(iter_collect_into)]
#![feature(const_for)]
#![feature(const_mut_refs)]
#![feature(const_option)]
#![feature(const_fmt_arguments_new)]
#![feature(const_trait_impl)]
#![feature(iterator_try_collect)]
#![feature(iter_array_chunks)]
#![feature(iter_next_chunk)]
#![feature(array_try_map)]
#![feature(array_methods)]
#![feature(return_position_impl_trait_in_trait)]
#![allow(dead_code)]

use bumpalo::Bump;
use cfr::generate::GenerationContext;
use cfr::phase::{Phase, SeerPhase};
use game::battlefield::Battlefield;
use game::creature::CreatureSet;
use game::edict::Edict;
use game::known_state::KnownState;
use std::println;
use std::time::Instant;

use crate::cfr::generate::EstimationContext;
use crate::game::creature::Creature;
use crate::game::status_effect::StatusEffect;
use crate::helpers::bitfield::Bitfield;

mod ai;
mod cfr;
mod game;
mod helpers;

fn mb_to_b(mb: usize) -> usize {
    mb * 1024 * 1024
}

fn b_to_kb(b: usize) -> usize {
    b / 1024
}

fn b_to_mb(b: usize) -> usize {
    b_to_kb(b) / 1024
}

fn b_to_gb(b: usize) -> usize {
    b_to_mb(b) / 1024
}

fn simple_generation(from: usize, turns: usize, generate: bool) {
    let start = Instant::now();
    let capacity = mb_to_b(4096);
    let allocator = Bump::with_capacity(2500);
    allocator.set_allocation_limit(Some(capacity));
    let allocation_duration = start.elapsed();

    println!("Performance:");
    println!("Allocation: {:?}", allocation_duration);

    let start = Instant::now();
    let mut state = KnownState::new_starting([Battlefield::Plains; 4]);
    state.battlefields.all[3 - from] = Battlefield::LastStrand;

    for i in 0..from {
        state.graveyard.add(Creature::CREATURES[2 * i]);
        state.graveyard.add(Creature::CREATURES[2 * i + 1]);
    }

    let generator = GenerationContext::new(turns, state, &allocator);
    let estimator = EstimationContext::new(turns, state);
    let state_init_duration = start.elapsed();

    println!("State init: {:?}", state_init_duration);

    let start = Instant::now();
    let stats = estimator.estimate();
    let estimation_duration = start.elapsed();

    println!("Estimation: {:?}", estimation_duration);

    if generate {
        let start = Instant::now();
        generator.generate();
        let generation_duration = start.elapsed();

        println!("Generation: {:?}", generation_duration);
    };

    println!("\nAllocation stats:");
    println!("Allocated: {:?}MB", b_to_mb(allocator.allocated_bytes()));
    println!(
        "Remaining capacity: {:?}MB",
        b_to_mb(allocator.chunk_capacity())
    );
    println!("{stats:#?}");
}

fn main() {
    // simple_generation(0, 2, false);

    let mut creatures = CreatureSet::default();

    creatures.add(Creature::Seer);
    creatures.add(Creature::Wall);
    creatures.add(Creature::Bard);
    creatures.add(Creature::Mercenary);
    creatures.add(Creature::Steward);

    for s in creatures.subsets_of_size(3) {
        println!("{s:?}");
    }

    let mut state = KnownState::new_starting([Battlefield::Plains; 4]);

    for i in 0..2 {
        state.graveyard.add(Creature::CREATURES[2 * i]);
        state.graveyard.add(Creature::CREATURES[2 * i + 1]);
    }

    let revealed = Creature::Monarch;

    let phase = SeerPhase::new(
        [Edict::DivertAttention, Edict::RileThePublic],
        [None, None],
        revealed,
    );

    state.player_states[0].effects.add(StatusEffect::Seer);

    println!("Seer player {:?}", state.forced_seer_player());

    let graveyard = state.graveyard | CreatureSet::singleton(revealed);

    println!("Graveyard: {:?}", graveyard);

    for indices in phase.valid_hidden_states(&state) {
        let my_state = indices[0]
            .decode_sabotage_seer_index(1, graveyard, true)
            .unwrap();
        println!("My state {my_state:?}");

        let your_state = indices[1].decode_main_index(graveyard, 2).unwrap();
        println!("Your state {your_state:?}");
    }
}
