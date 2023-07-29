use itertools::Itertools;

use super::pair::Pair;

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
