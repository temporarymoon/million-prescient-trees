use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use echo::{
    cfr::generate::EstimationContext,
    game::{battlefield::Battlefield, known_state::KnownState},
    helpers::bitfield::{Bitfield, Bitfield16},
};

pub fn subsets_of_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("expensive");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));
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

    group.bench_function("estimate first two turns", |b| {
        b.iter(|| {
            let state = KnownState::new_starting([Battlefield::Plains; 4]);
            let estimator = EstimationContext::new(2, state);

            estimator.estimate()
        })
    });

    group.finish();
}

criterion_group!(benches, subsets_of_size);
criterion_main!(benches);
