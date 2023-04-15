use super::swap::Pair;

pub type Encoded = u8;
pub type Decoded = Pair<u8>;

/// Swaps a pair if necessary so the first element is bigger.
const fn reverse_sort_pair(p: Decoded) -> Decoded {
    if p.0 < p.1 {
        (p.1, p.0)
    } else {
        p
    }
}

/// Encodes two ints into a single int, with the assumption that
/// encode(a, b) should be equal to encode(b, a)
pub const fn encode_subpair(p: Decoded) -> Encoded {
    let (a, b) = reverse_sort_pair(p);
    let a = a as Encoded;

    return a * (a - 1) / 2 + b as Encoded;
}

const MAX_N: usize = 9;
const MAX_PAIR: usize = MAX_N * (MAX_N - 1) / 2;
const DECODE_LOOKUP_TABLE: [Decoded; MAX_PAIR] = {
    let mut result = [(0, 0); MAX_PAIR];

    const_for!(i in 0..MAX_N => {
        const_for!(i in 0..i => {
            result[encode_subpair((i as u8, j as u8)) as usize] = (i as u8, j as u8);
        });
    });

    result
};

/// Decodes two ints encoded with the above function,
/// where the two ints are smaller than some n.
pub fn decode_subpair(x: Encoded) -> Decoded {
    assert!(
        (x as usize) < MAX_PAIR,
        "Cannot decode numbers larger than {}! Received {}.",
        MAX_PAIR,
        x
    );

    DECODE_LOOKUP_TABLE[x as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_encode_inverses() {
        for i in 0..MAX_N {
            for j in 0..i {
                assert_eq!(
                    (i as u8, j as u8),
                    decode_subpair(encode_subpair((i as u8, j as u8)))
                );
            }
        }
    }

    #[test]
    fn decode_injective() {
        for ij in 0..MAX_PAIR {
            for kl in 0..MAX_PAIR {
                if ij != kl {
                    assert_ne!(decode_subpair(ij as u8), decode_subpair(kl as u8));
                }
            }
        }
    }

    #[test]
    fn ecode_injective() {
        for i in 0..MAX_N {
            for j in 0..i {
                for k in 0..MAX_N {
                    for l in 0..k {
                        if reverse_sort_pair((i as u8, j as u8))
                            != reverse_sort_pair((k as u8, l as u8))
                        {
                            assert_ne!(
                                encode_subpair((i as u8, j as u8)),
                                encode_subpair((k as u8, l as u8))
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn encode_commutative() {
        for i in 0..MAX_N {
            for j in 0..i {
                assert_eq!(
                    encode_subpair((i as u8, j as u8)),
                    encode_subpair((j as u8, i as u8))
                );
            }
        }
    }
}
