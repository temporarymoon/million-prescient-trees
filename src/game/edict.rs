use crate::helpers::bitfield::Bitfield;
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

// }}}
// {{{ EdictSet
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct EdictSet(pub Bitfield);

/// Represents an index of a bit in an edict set.
pub type EdictIndex = usize;

impl EdictSet {
    #[inline(always)]
    pub fn empty() -> Self {
        EdictSet(Bitfield::default())
    }

    #[inline(always)]
    pub fn all() -> Self {
        EdictSet(Bitfield::n_ones(5))
    }

    #[inline(always)]
    pub fn remove(&mut self, edict: Edict) {
        self.0.remove(edict as usize)
    }

    #[inline(always)]
    pub fn has(self, edict: Edict) -> bool {
        self.0.has(edict as usize)
    }

    #[inline(always)]
    pub fn len(self) -> usize {
        let result = self.0.len();
        debug_assert!(result <= 5); // Sanity checks
        result
    }

    #[inline(always)]
    pub fn indexof(self, target: Edict) -> EdictIndex {
        self.0.count_from_end(target as usize)
    }

    #[inline(always)]
    pub fn index(self, index: EdictIndex) -> Option<Edict> {
        self.0.lookup_from_end(index).map(|x| Edict::EDICTS[x])
    }
}

impl Default for EdictSet {
    fn default() -> Self {
        Self::all()
    }
}
// }}}
