#![feature(allocator_api)]
#![feature(iter_collect_into)]
#![feature(const_for)]
#![feature(const_mut_refs)]
#![feature(const_option)]
#![feature(const_fmt_arguments_new)]
#![feature(const_trait_impl)]
#![allow(dead_code)]

use bumpalo::Bump;
use cfr::generate::GenerationContext;
use game::battlefield::Battlefield;
use game::known_state::KnownState;
use std::mem::size_of;
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

    for i in 0..from {
        state.graveyard.add(Creature::CREATURES[2 * i]);
        state.graveyard.add(Creature::CREATURES[2 * i + 1]);
        state.battlefields.current += 1;
    }

    let mut generator = GenerationContext::new(turns, state, &allocator);
    let mut estimator = EstimationContext::new(turns, state);
    let state_init_duration = start.elapsed();

    println!("State init: {:?}", state_init_duration);

    let start = Instant::now();
    let stats_estimated = estimator.estimate_alloc();
    let estimation_duration = start.elapsed();

    println!("Estimation: {:?}", estimation_duration);

    let stats = if generate {
        let start = Instant::now();
        let (_, stats) = generator.generate();
        let generation_duration = start.elapsed();

        println!("Generation: {:?}", generation_duration);

        stats
    } else {
        stats_estimated
    };

    println!("\nAllocation stats:");
    println!("Allocated: {:?}MB", b_to_mb(allocator.allocated_bytes()));
    println!(
        "Remaining capacity: {:?}MB",
        b_to_mb(allocator.chunk_capacity())
    );
    println!(
        "Estimated: {:?}GB",
        b_to_gb(stats_estimated.estimate_alloc())
    );
    println!(
        "Required space for weight storage per battlefield config {}GB",
        b_to_gb(stats.estimate_weight_storage_per_battlefield())
    );
    println!(
        "Required space for weight storage of all battlefield configs {}GB",
        b_to_gb(stats.estimate_weight_storage())
    );

    println!("\nScope stats:");
    println!("Explored scopes: {}", stats.explored_scopes);
    println!("Unexplored scopes: {}", stats.unexplored_scopes);
    println!("Completed scopes: {}", stats.completed_scopes);

    println!("\nTotal stats:");
    println!("main decision count: {}", stats.main_total_decisions);
    println!("main hidden count: {}", stats.main_total_hidden);
    println!("main branching count: {}", stats.main_total_next);
    println!(
        "sabotage decision count: {}",
        stats.sabotage_total_decisions
    );
    println!("sabotage hidden count: {}", stats.sabotage_total_hidden);
    println!("sabotage branching count: {}", stats.sabotage_total_next);
    println!("seer decision count: {}", stats.seer_total_decisions);
    println!("seer hidden count: {}", stats.seer_total_hidden);
    println!("seer branching count: {}", stats.seer_total_next);
    println!("total weight count {}", stats.total_weights());

    println!("\nAverage stats:");
    println!("main decision count: {}", stats.main_average_decisions());
    println!("main hidden count: {}", stats.main_average_hidden());
    println!("main branching count: {}", stats.main_average_next());
    println!(
        "sabotage decision count: {}",
        stats.sabotage_average_decisions()
    );
    println!("sabotage hidden count: {}", stats.sabotage_average_hidden());
    println!(
        "sabotage branching count: {}",
        stats.sabotage_average_next()
    );
    println!("seer decision count: {}", stats.seer_average_decisions());
    println!("seer hidden count: {}", stats.seer_average_hidden());
    println!("seer branching count: {}", stats.seer_average_next());

    println!("\nSizes:");
    println!("KnownState: {}", size_of::<KnownState>());
}

fn main() {
    // let mut edicts = EdictSet::all();
    // edicts.0.remove(Edict::DivertAttention as u8);
    //
    // let mut graveyard = CreatureSet::all().others();
    // graveyard.0.add(Creature::Seer as u8);
    // graveyard.0.add(Creature::Steward as u8);
    //
    // for creature_one in Creature::CREATURES {
    //     for creature_two in Creature::CREATURES {
    //         if creature_one <= creature_two
    //             || graveyard.has(creature_one)
    //             || graveyard.has(creature_two)
    //         {
    //             continue;
    //         };
    //
    //         for edict in Edict::EDICTS {
    //             if !edicts.has(edict) {
    //                 continue;
    //             };
    //
    //             let encoded = DecisionIndex::encode_main_phase_index_user(
    //                 (creature_one, Some(creature_two)),
    //                 edict,
    //                 edicts,
    //                 graveyard,
    //             );
    //
    //             println!(
    //                 "Edict {:?}, creature₁ {:?}, creature₂ {:?} => {:?}",
    //                 edict,
    //                 creature_one,
    //                 creature_two,
    //                 encoded.unwrap()
    //             );
    //         }
    //     }
    // }
    //

    // let start = Instant::now();
    // let mut total = 0;
    // for c in 0..=16 {
    //     for i in 0.. {
    //         match Bitfield16::decode_ones(i, c) {
    //             Some(inner) => println!("{: >2}: {: <5} {:?}", c, i, inner),
    //             None => break,
    //         };
    //         total += 1;
    //     }
    // }
    // let duration = start.elapsed();
    // println!("Printed {} numbers in {:?}", total, duration);
    //
    // let start = Instant::now();
    // let mut total = 0;
    // for c in 0..=16 {
    //     for i in 0.. {
    //         if Bitfield16::decode_ones(i, c).is_none() {
    //             break;
    //         };
    //         total += 1;
    //     }
    // }
    // let duration = start.elapsed();
    // println!("Computed {} numbers in {:?}", total, duration);

    simple_generation(1, 3, false);
}
