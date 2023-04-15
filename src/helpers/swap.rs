pub trait Swap {
    fn swap(self) -> Self;
}

pub type Pair<T> = (T, T);

impl<T> Swap for Pair<T> {
    fn swap(self) -> Self {
        (self.1, self.0)
    }
}

pub fn conditional_swap<T>(pair: T, should_swap: bool) -> T
where
    T: Swap,
{
    if should_swap {
        pair.swap()
    } else {
        pair
    }
}
