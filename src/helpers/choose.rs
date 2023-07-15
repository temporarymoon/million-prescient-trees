/// Const `n choose k` function.
/// - tested for values smaller than 17.
/// - fails when n=0 or n>=k.
pub fn choose(n: usize, k: usize) -> usize {
    assert!(n >= 1); // Our implementation doesn't handle 0 nicely
    assert!(n >= k);

    let mut result: u64 = 1;

    for i in (n - k + 1)..(n + 1) {
        result *= i as u64;
    }

    for i in 2..(k + 1) {
        result /= i as u64;
    }

    return result as usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_eq;

    /// We want to test the function for values smaller than this.
    const UPPER_BOUND: usize = 17;

    /// `n choose k` measures the number of k-element subsets of a set with n elements.
    /// The number of subsets of a set with n elements is 2^n,
    /// so the sum of `n choose k` for all k in the range 0..n should be 2^n.
    #[test]
    fn choices_add_to_powers_of_two() {
        for i in 1..UPPER_BOUND {
            let mut result = 0;
            for j in 0..(i + 1) {
                result += choose(i, j);
            }

            assert_eq!(result, (2 as usize).pow(i as u32), "Failed for {}", i);
        }
    }

    /// Tests that `n choose k` is equal to `n choose n - k`.
    #[test]
    fn choice_complements() {
        for i in 1..UPPER_BOUND {
            for j in 0..i {
                assert_eq!(choose(i, j), choose(i, i - j));
            }
        }
    }
}
