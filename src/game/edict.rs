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

    pub const DESCRIPTIONS: [&str; 5] = [
        "- The current battlefield is worth +1 victory points.\
        \n- Negates \"divert attention\"",
        "The current battlefield iw worth -1 victory points.",
        "Write down a guess for what the opponent's creature could be. \
         When said creature is revealed, gaint +2 strength if your guess was correct.",
        "Gain +1 strength. You lose on ties.",
        "Gain an additional +1 strength if your creature has a battlefield bonus.",
    ];
}

impl From<usize> for Edict {
    fn from(value: usize) -> Self {
        Edict::EDICTS[value]
    }
}
// }}}

make_bitfield!(EdictSet, Edict, u8, 5, Bitfield16, false);
