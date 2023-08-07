use egui_extras::RetainedImage;

use crate::{
    game::{creature::Creature, edict::Edict},
    helpers::try_from_iter::TryCollect,
};

pub struct AppTextures {
    pub edicts: [RetainedImage; 5],
    pub creatures: [RetainedImage; 11],
    pub card_back: RetainedImage,
}

// {{{ Included bytes
const EDICT_TEXTURES: [&[u8]; 5] = [
    include_bytes!("../../assets/edicts/rilethepublic.jpeg"),
    include_bytes!("../../assets/edicts/divertattention.jpeg"),
    include_bytes!("../../assets/edicts/sabotage.jpeg"),
    include_bytes!("../../assets/edicts/gambit.jpeg"),
    include_bytes!("../../assets/edicts/ambush.jpeg"),
];

const CREATURE_TEXTURES: [&[u8]; 11] = [
    include_bytes!("../../assets/creatures/wall.jpeg"),
    include_bytes!("../../assets/creatures/seer.jpeg"),
    include_bytes!("../../assets/creatures/rogue.jpeg"),
    include_bytes!("../../assets/creatures/bard.jpeg"),
    include_bytes!("../../assets/creatures/diplomat.jpeg"),
    include_bytes!("../../assets/creatures/ranger.jpeg"),
    include_bytes!("../../assets/creatures/steward.jpeg"),
    include_bytes!("../../assets/creatures/barbarian.jpeg"),
    include_bytes!("../../assets/creatures/witch.jpeg"),
    include_bytes!("../../assets/creatures/mercenary.jpeg"),
    include_bytes!("../../assets/creatures/monarch.jpeg"),
];

const CARD_BACK: &[u8] = include_bytes!("../../assets/cardback.png");
// }}}
// {{{ Texture loading code
impl AppTextures {
    pub fn new() -> Self {
        let edicts = EDICT_TEXTURES
            .iter()
            .enumerate()
            .map(|(i, bytes)| {
                let name = format!("{:?}", Edict::EDICTS[i]);
                RetainedImage::from_image_bytes(name, bytes).unwrap()
            })
            .attempt_collect()
            .unwrap();

        let creatures = CREATURE_TEXTURES
            .iter()
            .enumerate()
            .map(|(i, bytes)| {
                let name = format!("{:?}", Creature::CREATURES[i]);
                RetainedImage::from_image_bytes(name, bytes).unwrap()
            })
            .attempt_collect()
            .unwrap();

        let card_back = RetainedImage::from_image_bytes("card_back", CARD_BACK).unwrap();

        Self {
            edicts,
            card_back,
            creatures,
        }
    }
}
// }}}
