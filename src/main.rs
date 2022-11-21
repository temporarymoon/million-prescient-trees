use crate::hash::EchoHash;

mod echo;
mod hash;

fn main() {
    let result = echo::InfoSet::MAX;
    println!("{}", result);
}
