use crate::train::train;
use std::time::Instant;

mod echo;
mod helpers;
mod train;

fn main() {
    let start = Instant::now();
    let utility = train(50000);
    let duration = start.elapsed();

    println!("Time elapsed: {:?}", duration);
    println!("You have a {}% chance of winning", utility);
}
