#![allow(dead_code)]

use bumpalo::Bump;
use echo::cfr::generate::EstimationContext;
use echo::cfr::generate::GenerationContext;
use echo::cfr::train::TrainingContext;
use echo::game::battlefield::Battlefield;
use echo::game::creature::Creature;
use echo::game::edict::Edict;
use echo::game::known_state::KnownState;
use echo::game::known_state_summary::KnownStateEssentials;
use echo::helpers::bitfield::Bitfield;
use std::println;
use std::time::Instant;

// {{{ Dumb size conversion functions
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
// }}}
// {{{ Simple generation/estimating routine
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
    state.battlefields.all[3] = Battlefield::LastStrand;
    state.battlefields.current = from;

    for i in 0..from {
        state.graveyard.insert(Creature::CREATURES[2 * i]);
        state.graveyard.insert(Creature::CREATURES[2 * i + 1]);
    }

    for state in state.player_states.iter_mut() {
        for edict in Edict::EDICTS.into_iter().take(from) {
            state.edicts.remove(edict);
        }
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
// }}}

fn main() {
    simple_generation(2, 2, false);

    // // {{{ State creation
    // let mut state = KnownState::new_starting([Battlefield::Plains; 4]);
    // state.battlefields.current = 2;
    // for creature in Creature::CREATURES.into_iter().take(4) {
    //     state.graveyard.insert(creature);
    // }
    //
    // for state in state.player_states.iter_mut() {
    //     for edict in Edict::EDICTS.into_iter().take(2) {
    //         state.edicts.remove(edict);
    //     }
    // }
    // // }}}
    // // {{{ Generation
    // let allocator = Bump::new();
    // let generator = GenerationContext::new(2, state, &allocator);
    // let mut scope = generator.generate();
    // // }}}
    // // {{{ Training
    // let ctx = TrainingContext::new();
    // ctx.train(&mut scope, state.to_summary(), 10000);
    // // }}}
}
