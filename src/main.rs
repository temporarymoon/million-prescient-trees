use crate::hash::EchoHash;

mod echo;
mod hash;
mod train;

fn main() {
    let result = echo::InfoSet::MAX;
    println!("{}", result);
}
