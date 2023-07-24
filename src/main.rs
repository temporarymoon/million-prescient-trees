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
#![feature(array_zip)]
#![feature(array_methods)]
#![allow(dead_code)]

use bumpalo::Bump;
use cfr::generate::GenerationContext;
use game::battlefield::Battlefield;
use game::known_state::KnownState;
use std::println;
use std::time::Instant;

use crate::cfr::generate::EstimationContext;
use crate::game::creature::Creature;
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
    simple_generation(0, 2, false);
}
