use std::mem::MaybeUninit;

// {{{ TryFromIterator trait
/// Trait for types which can be collected into for certain iterators only.
pub trait TryFromIterator<A>: Sized {
    /// Attempt to collect an iterator into a given structure.
    /// Returns `None` when a `None` element is encountered or when
    /// the iterator cannot be converted into the given structure.
    fn try_from_opt_iter<T: IntoIterator<Item = Option<A>>>(iter: T) -> Option<Self>;

    /// Attempt to collect an iterator into a given structure.
    fn try_from_iter<T: IntoIterator<Item = A>>(iter: T) -> Option<Self> {
        TryFromIterator::try_from_opt_iter(iter.into_iter().map(Some))
    }
}

impl<A, const N: usize> TryFromIterator<A> for [A; N] {
    fn try_from_opt_iter<T: IntoIterator<Item = Option<A>>>(iter: T) -> Option<Self> {
        let mut iter = iter.into_iter();
        let mut output: MaybeUninit<[A; N]> = MaybeUninit::uninit();

        unsafe {
            for i in 0..N {
                if let Some(Some(value)) = iter.next() {
                    output.as_mut_ptr().cast::<A>().add(i).write(value);
                } else {
                    return None;
                }
            }

            if iter.next().is_some() {
                None
            } else {
                Some(output.assume_init())
            }
        }
    }
}
// }}}
// {{{ Traits for iter methods
pub trait TryOptCollect<A>: Sized + IntoIterator<Item = Option<A>> {
    /// Like `try_collect`, but can fail even when only given `Some` values.
    fn attempt_opt_collect<I: TryFromIterator<A>>(self) -> Option<I>;
}

pub trait TryCollect: Sized + IntoIterator {
    /// Like `collect`, but can fail.
    fn attempt_collect<I: TryFromIterator<Self::Item>>(self) -> Option<I>;
}

impl<A, T: IntoIterator<Item = Option<A>>> TryOptCollect<A> for T {
    fn attempt_opt_collect<I: TryFromIterator<A>>(self) -> Option<I> {
        TryFromIterator::try_from_opt_iter(self)
    }
}

impl<T: IntoIterator> TryCollect for T {
    fn attempt_collect<I: TryFromIterator<Self::Item>>(self) -> Option<I> {
        TryFromIterator::try_from_iter(self)
    }
}
// }}}
