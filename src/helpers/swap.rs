use super::Pair;

/// Trait which allows swapping the elements of some structure.
pub trait Swap {
    fn swap(self) -> Self;
}

impl<T> Swap for Pair<T> {
    #[inline(always)]
    fn swap(self) -> Self {
        (self.1, self.0)
    }
}

/// Swap the elements of a structure only if a condition is true.
#[inline(always)]
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
