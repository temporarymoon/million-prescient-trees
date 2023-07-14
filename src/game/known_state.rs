use super::{
    battlefield::Battlefields, creature::CreatureSet, edict::EdictSet,
    status_effect::StatusEffectSet, types::Score,
};

/// State of a player known by both players.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct KnownPlayerState {
    pub edicts: EdictSet,
    pub effects: StatusEffectSet,
}

/// State known by both players at some point in time.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct KnownState {
    pub player_states: (KnownPlayerState, KnownPlayerState),
    pub battlefields: Battlefields,
    pub graveyard: CreatureSet,
    pub score: Score,
}
