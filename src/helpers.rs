pub fn zeroes(size: usize) -> Vec<f32> {
    vec![0.0; size]
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

pub fn normalize_vec(vec: &mut Vec<f32>) {
    let mut sum = 0.0;
    let size = vec.len();

    for value in &mut *vec {
        sum += *value;
    }

    for value in vec {
        if sum > 0.0 {
            *value /= sum;
        } else {
            *value = 1.0 / (size as f32);
        }
    }
}
