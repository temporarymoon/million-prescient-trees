pub fn zeroes(size: usize) -> Vec<f32> {
    let mut vec = Vec::with_capacity(size);
    vec.fill(0.0);
    vec
}


pub trait Swap {
    fn swap(self) -> Self;
}

impl<T> Swap for (T, T) {
    fn swap(self) -> Self {
        (self.1, self.0)
    }
}

pub fn conditional_swap<T>(pair: (T, T), should_swap: bool) -> (T, T) {
    if should_swap {
        pair.swap()
    } else {
        pair
    }
}
