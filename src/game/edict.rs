use crate::{
    helpers::bitfield::{Bitfield, Bitfield16},
    make_bitfield,
};
use std::{
    debug_assert,
    fmt::{self, Display},
};

// {{{ Edict
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Edict {
    // Victory point edicts
    RileThePublic,
    DivertAttention,

    // Strength edicts
    Sabotage,
    Gambit,
    Ambush,
}

impl Display for Edict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Edict {
    pub const EDICTS: [Edict; 5] = [
        Edict::RileThePublic,
        Edict::DivertAttention,
        Edict::Sabotage,
        Edict::Gambit,
        Edict::Ambush,
    ];
}

impl From<usize> for Edict {
    fn from(value: usize) -> Self {
        Edict::EDICTS[value]
    }
}
// }}}

make_bitfield!(EdictSet, Edict, u8, 5, EdictSetIterator, Bitfield16, false);
