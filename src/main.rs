#![feature(allocator_api)]
#![feature(iter_collect_into)]
#![feature(const_for)]
#![feature(const_mut_refs)]
#![feature(const_option)]
#![feature(const_fmt_arguments_new)]
#![allow(dead_code)]

use std::println;

use game::types::{Creature, CreatureSet, Edict, EdictSet};
use helpers::bitfield::Bitfield;

use crate::{
    cfr::decision::DecisionIndex,
    helpers::{choose::choose, upair::decode_upair},
};

mod cfr;
mod echo;
mod game;
mod helpers;

fn main() {
    // let mut edicts = EdictSet::all();
    // edicts.0.remove(Edict::DivertAttention as u8);
    //
    // let mut graveyard = CreatureSet::all().others();
    // graveyard.0.add(Creature::Seer as u8);
    // graveyard.0.add(Creature::Steward as u8);
    //
    // for creature_one in Creature::CREATURES {
    //     for creature_two in Creature::CREATURES {
    //         if creature_one <= creature_two
    //             || graveyard.has(creature_one)
    //             || graveyard.has(creature_two)
    //         {
    //             continue;
    //         };
    //
    //         for edict in Edict::EDICTS {
    //             if !edicts.has(edict) {
    //                 continue;
    //             };
    //
    //             let encoded = DecisionIndex::encode_main_phase_index_user(
    //                 (creature_one, Some(creature_two)),
    //                 edict,
    //                 edicts,
    //                 graveyard,
    //             );
    //
    //             println!(
    //                 "Edict {:?}, creature₁ {:?}, creature₂ {:?} => {:?}",
    //                 edict,
    //                 creature_one,
    //                 creature_two,
    //                 encoded.unwrap()
    //             );
    //         }
    //     }
    // }
    //

    let c = 2;
    for i in 0..36 {
        let (a, b) = decode_upair(i as u8).unwrap();
        let mut bitfield = Bitfield::default();
        bitfield.add(a);
        bitfield.add(b);
        println!(
            "{:?} --- {:?}",
            Bitfield::decode_ones(i as u16, c).unwrap(),
            bitfield
        );
    }
}
