use rand::Rng;

pub mod choose;
pub mod swap;
pub mod bitfield;
pub mod ranged;

/// Normalize a vector. If all the values are zero,
/// all the entries will be set to 1/size.
pub fn normalize_vec(vec: &mut [f32]) {
    let mut sum = 0.0;
    let size = vec.len();

    for value in &mut *vec {
        sum += *value;
    }

    for value in vec {
        if sum > 0.0 {
            *value /= sum;
        } else {
            // TODO: maybe extract this in the outer scope?
            // I doubt this really impact performance.
            *value = 1.0 / (size as f32);
        }
    }
}

/// Pick a random number using a probability distribution.
pub fn roulette<R>(probabilities: &[f32], rng: &mut R) -> usize
where
    R: Rng,
{
    let upper = 100000;
    let num: f32 = rng.gen_range(0..upper) as f32 / (upper as f32);
    let mut total = 0.0;

    for (index, length) in probabilities.into_iter().enumerate() {
        if num >= total && num < total + length {
            return index;
        }

        total += *length;
    }

    panic!(
        "Degenerate probability distribution {:?} — value {:?} does not fit anywhere.",
        probabilities, num
    )
}
