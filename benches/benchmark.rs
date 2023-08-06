use bumpalo::Bump;
use criterion::{criterion_group, criterion_main, Criterion};
use echo::cfr::generate::{EstimationContext, GenerationContext};
use echo::cfr::train::TrainingContext;
use echo::game::battlefield::Battlefield;
use echo::game::creature::Creature;
use echo::game::edict::Edict;
use echo::game::known_state::KnownState;
use echo::game::known_state_summary::KnownStateEssentials;
use echo::helpers::bitfield::{Bitfield, Bitfield16};
use std::time::Duration;

pub fn subsets_of_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("expensive");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    // {{{ Subsets of size
    group.bench_function("subsets of size", |b| {
        b.iter(|| {
            let mut res = 0;
            for i in 0..Bitfield16::MAX {
                let b = Bitfield16::new(i);
                for ones in 0..=b.len() {
                    res += b.subsets_of_size(ones).count();
                }
            }

            res
        })
    });
    // }}}
    // {{{ Estimate first two turns
    group.bench_function("estimate first two turns", |b| {
        b.iter(|| {
            let state = KnownState::new_starting([Battlefield::Plains; 4]);
            let estimator = EstimationContext::new(2, state);

            estimator.estimate()
        })
    });
    // }}}
    // {{{ Generate and train last two turns
    group.bench_function("train last two turns", |b| {
        b.iter(|| {
            // {{{ State creation
            let mut state = KnownState::new_starting([Battlefield::Plains; 4]);
            state.battlefields.current = 2;
            for creature in Creature::CREATURES.into_iter().take(4) {
                state.graveyard.insert(creature);
            }

            for state in state.player_states.iter_mut() {
                for edict in Edict::EDICTS.into_iter().take(2) {
                    state.edicts.remove(edict);
                }
            }
            // }}}
            // {{{ Generation
            let allocator = Bump::new();
            let generator = GenerationContext::new(2, state, &allocator);
            let mut scope = generator.generate();
            // }}}
            // {{{ Training
            let ctx = TrainingContext::new();
            ctx.cfr(&mut scope, state.to_summary(), 10);
            // }}}
        })
    });
    // }}}

    group.finish();
}

criterion_group!(benches, subsets_of_size);
criterion_main!(benches);
