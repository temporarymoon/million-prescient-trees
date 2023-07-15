use super::creature::{Creature, CreatureSet};
use crate::helpers::bitfield::{Bitfield16, Bitfield};

// {{{ UserCreatureChoice
/// User facing version of `CreatureChoice`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UserCreatureChoice(pub Creature, pub Option<Creature>);

impl UserCreatureChoice {
    /// The number of cards chosen by the user (either `1` or `2`).
    #[inline(always)]
    pub fn len(self) -> usize {
        if self.1.is_some() {
            2
        } else {
            1
        }
    }

    /// Returns the length of some user creature choice based
    /// on whether the seer status effect is active or not.
    #[inline(always)]
    pub fn len_from_status(seer_active: bool) -> usize {
        if seer_active {
            2
        } else {
            1
        }
    }

    pub fn as_creature_set(self) -> CreatureSet {
        let mut bitfield = CreatureSet::default();
        bitfield.add(self.0);

        if let Some(second) = self.1 {
            bitfield.add(second);
        }

        bitfield
    }
}
// }}}
// {{{ CreatureChoice
/// Encoded version of `UserCreatureChoice`.
/// The result fits inside an `u8`, but we are
/// using an `usize` for convenience.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CreatureChoice(pub usize);

impl CreatureChoice {
    /// Encode a one/two creature choice into a single integer, removing any info
    /// about the number of chosen creatures and the contents of the graveyard from the
    /// resulting integer.
    pub fn encode_user_choice(user_choice: UserCreatureChoice, possibilities: CreatureSet) -> Self {
        Self(
            user_choice
                .as_creature_set()
                .encode_relative_to(possibilities)
                .encode_ones(),
        )
    }

    /// Inverse of `encode_user_choice`.
    pub fn decode_user_choice(
        self,
        possibilities: CreatureSet,
        seer_active: bool,
    ) -> Option<UserCreatureChoice> {
        let length = UserCreatureChoice::len_from_status(seer_active);
        let decoded =
            CreatureSet::decode_relative_to(Bitfield16::decode_ones(self.0, length)?, possibilities)?;

        let mut creatures = decoded.into_iter();

        let first = creatures.next()?;
        if seer_active {
            let second = creatures.next()?;
            Some(UserCreatureChoice(first, Some(second)))
        } else {
            Some(UserCreatureChoice(first, None))
        }
    }
}
// }}}
