#![allow(dead_code)]

use bumpalo::Bump;
use echo::cfr::generate::EstimationContext;
use echo::cfr::generate::GenerationContext;
use echo::game::battlefield::Battlefield;
use echo::game::creature::Creature;
use echo::game::known_state::KnownState;
use echo::helpers::bitfield::Bitfield;
use std::println;
use std::time::Instant;

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
        state.graveyard.insert(Creature::CREATURES[2 * i]);
        state.graveyard.insert(Creature::CREATURES[2 * i + 1]);
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
    


    // let mut x = 0b1001111;
    // for _ in 0..10 {
    //     println!("{:b}", x);
    //     x = snoob(x);
    // }
    //
    // let mut creatures = CreatureSet::default();
    //
    // creatures.insert(Creature::Seer);
    // creatures.insert(Creature::Wall);
    // creatures.insert(Creature::Bard);
    // creatures.insert(Creature::Mercenary);
    // creatures.insert(Creature::Steward);
    //
    // println!("{creatures:?}");
    //
    // for s in creatures.subsets_of_size(3) {
    //     println!("{s:?}");
    // }

    // let mut state = KnownState::new_starting([Battlefield::Plains; 4]);
    // state.graveyard.add(Creature::Seer);
    // state.graveyard.add(Creature::Mercenary);
    //
    // let phase = MainPhase::new();
    // println!("{:?}", phase.valid_hidden_states(&state).count());
    //
    // for indices in phase.valid_hidden_states(&state) {
    //     let my_state = indices[0]
    //         .decode_main_index(state.graveyard, state.hand_size())
    //         .unwrap();
    //
    //     let your_state = indices[1]
    //         .decode_main_index(state.graveyard, state.hand_size())
    //         .unwrap();
    //
    //     println!("{my_state:?} --- {your_state:?}");
    // }

    // let mut state = KnownState::new_starting([Battlefield::Plains; 4]);
    //
    // for i in 0..2 {
    //     state.graveyard.add(Creature::CREATURES[2 * i]);
    //     state.graveyard.add(Creature::CREATURES[2 * i + 1]);
    // }
    //
    // let revealed = Creature::Monarch;
    //
    // let phase = SeerPhase::new(
    //     [Edict::DivertAttention, Edict::RileThePublic],
    //     [None, None],
    //     revealed,
    // );
    //
    // state.player_states[0].effects.add(StatusEffect::Seer);
    //
    // println!("Seer player {:?}", state.forced_seer_player());
    //
    // let graveyard = state.graveyard | CreatureSet::singleton(revealed);
    //
    // println!("Graveyard: {:?}", graveyard);
    //
    // for indices in phase.valid_hidden_states(&state) {
    //     let my_state = indices[0]
    //         .decode_sabotage_seer_index(1, graveyard, true)
    //         .unwrap();
    //     println!("My state {my_state:?}");
    //
    //     let your_state = indices[1].decode_main_index(graveyard, 2).unwrap();
    //     println!("Your state {your_state:?}");
    // }
}
