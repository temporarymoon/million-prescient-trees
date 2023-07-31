use std::mem::MaybeUninit;

use itertools::Itertools;

use super::pair::Pair;

// {{{ Iterator helpers
/// More itertools!
pub trait Itercools: Iterator + Sized {
    /// Like `cartesian_product`, except the second iterator
    /// can depend on values yielded by the first.
    fn dependent_cartesian_product<J, F>(
        self,
        make_other: F,
    ) -> impl Iterator<Item = (Self::Item, J::Item)>
    where
        J: IntoIterator,
        J::Item: Clone,
        Self::Item: Clone,
        F: Fn(Self::Item) -> J,
    {
        self.flat_map(move |a| {
            make_other(a.clone())
                .into_iter()
                .map(move |b| (a.clone(), b))
        })
    }

    /// Similar to `dependent_cartesian_product`, except it returns a `Pair` instead of a tuple
    #[inline(always)]
    fn dependent_cartesian_pair_product<J, F>(
        self,
        make_other: F,
    ) -> impl Iterator<Item = Pair<Self::Item>>
    where
        J: IntoIterator<Item = Self::Item>,
        Self::Item: Clone,
        F: Fn(Self::Item) -> J,
    {
        self.dependent_cartesian_product(make_other)
            .map(|(a, b)| [a, b])
    }

    /// Similar to `cartesian_product`, except it returns a `Pair` instead of a tuple.
    #[inline(always)]
    fn cartesian_pair_product<J>(self, other: J) -> impl Iterator<Item = Pair<Self::Item>>
    where
        J: IntoIterator<Item = Self::Item>,
        J::IntoIter: Clone,
        Self::Item: Clone,
    {
        self.cartesian_product(other).map(|(a, b)| [a, b])
    }
}

impl<T: Iterator + Sized> Itercools for T {}

#[cfg(test)]
mod itercools_tests {
    use super::Itercools;

    #[test]
    fn dependent_cartesian_pair_product_examples() {
        let a = [1, 2, 3];
        let b: Vec<[i32; 2]> = a
            .into_iter()
            .dependent_cartesian_pair_product(|a| 0..a)
            .collect();

        assert_eq!(b, vec![[1, 0], [2, 0], [2, 1], [3, 0], [3, 1], [3, 2]]);
    }
}
// }}}
// {{{ Array helpers
pub trait ArrayUnzip<B, C> {
    /// The opposite of `.zip`.
    fn unzip(self) -> (B, C);
}

impl<const N: usize, B, C> ArrayUnzip<[B; N], [C; N]> for [(B, C); N] {
    #[inline(always)]
    fn unzip(self) -> ([B; N], [C; N]) {
        let (mut left, mut right): (MaybeUninit<[B; N]>, MaybeUninit<[C; N]>) =
            (MaybeUninit::uninit(), MaybeUninit::uninit());

        unsafe {
            for (i, (b, c)) in self.into_iter().enumerate() {
                left.as_mut_ptr().cast::<B>().add(i).write(b);
                right.as_mut_ptr().cast::<C>().add(i).write(c);
            }
            (left.assume_init(), right.assume_init())
        }
    }
}

#[cfg(test)]
mod array_methods_tests {
    use crate::helpers::itertools::ArrayUnzip;

    #[test]
    fn unzip_examples() {
        let (a, b) = [(1, 2), (3, 4)].unzip();
        assert_eq!(a, [1, 3]);
        assert_eq!(b, [2, 4]);
    }
}
// }}}
