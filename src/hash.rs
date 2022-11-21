#![allow(dead_code)]

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct HashResult(pub u64);

impl HashResult {
    pub fn extend<T>(self, other: &T) -> HashResult
    where
        T: EchoHash,
    {
        return HashResult(self.0 * T::MAX + other.echo_hash().0);
    }
}

impl std::ops::Mul for HashResult {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        HashResult(self.0 * rhs.0)
    }
}

impl std::ops::Add for HashResult {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        HashResult(self.0 + rhs.0)
    }
}

impl std::ops::Mul<HashResult> for u64 {
    type Output = HashResult;
    fn mul(self, rhs: HashResult) -> Self::Output {
        HashResult(self * rhs.0)
    }
}

impl std::ops::Add<HashResult> for u64 {
    type Output = HashResult;
    fn add(self, rhs: HashResult) -> Self::Output {
        HashResult(self + rhs.0)
    }
}

pub trait EchoHash {
    const MAX: u64;
    fn echo_hash(&self) -> HashResult;
}

impl<T> EchoHash for Option<&T>
where
    T: EchoHash,
{
    const MAX: u64 = 1 + T::MAX;
    fn echo_hash(&self) -> HashResult {
        match self {
            Option::None => HashResult(0),
            Option::Some(x) => HashResult(1 + x.echo_hash().0),
        }
    }
}

impl<A, B> EchoHash for (A, B)
where
    A: EchoHash,
    B: EchoHash,
{
    const MAX: u64 = A::MAX * B::MAX;
    fn echo_hash(&self) -> HashResult {
        self.0.echo_hash().extend(&self.1)
    }
}

pub fn from_vec<T>(vec: &Vec<T>) -> HashResult
where
    T: EchoHash,
{
    let mut result = HashResult(0);

    for value in vec {
        result = result.extend(&Some(value))
    }

    result
}
