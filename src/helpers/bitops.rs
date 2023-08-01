// Stuff in this file is taken from [here](https://web.archive.org/web/20130731200134/http://hackersdelight.org/hdcodetxt/snoob.c.txt)

/// Computes the next integer with the same number of bits set.
pub fn snoob(x: usize) -> usize {
    // x = xxx0 1111 0000

    //     0000 0001 0000
    let smallest: usize = x & (-(x as isize) as usize);

    //     xxx1 0000 0000
    let ripple = x + smallest;

    //     0001 1111 0000
    let ones = x ^ ripple;

    //     0000 0000 0111
    unsafe {
        let shifted_ones = ones.unchecked_shr(2 + smallest.trailing_zeros());

        //     xxx1 0000 0111
        ripple | shifted_ones
    }
}
