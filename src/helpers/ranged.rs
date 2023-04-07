#![allow(dead_code)]

pub trait MixRanged {
   /// Embed an integer inside self given the maximum value of the integer.
   fn mix_ranged(self, value: usize, max: usize) -> Self;
}

impl MixRanged for usize {
    fn mix_ranged(self, value: usize, max: usize) -> Self {
        max * self + value
    }
}
