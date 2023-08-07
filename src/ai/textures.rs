use crate::game::battlefield::Battlefield;
use crate::game::creature::Creature;
use crate::game::edict::Edict;
use crate::helpers::try_from_iter::TryCollect;
use egui_extras::RetainedImage;
use std::fmt::Debug;

pub struct AppTextures {
    pub edicts: [RetainedImage; 5],
    pub battlefields: [RetainedImage; 6],
    pub creatures: [RetainedImage; 11],
    pub card_back: RetainedImage,
}

// {{{ Included bytes
const BATTLEFIELD_TEXTURES: [&[u8]; 6] = [
    include_bytes!("../../assets/battlefields/mountain.jpeg"),
    include_bytes!("../../assets/battlefields/glade.jpeg"),
    include_bytes!("../../assets/battlefields/urban.jpeg"),
    include_bytes!("../../assets/battlefields/laststrand.jpeg"),
    include_bytes!("../../assets/battlefields/night.jpeg"),
    include_bytes!("../../assets/battlefields/plains.jpeg"),
];

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
    fn load_array<const N: usize, T: Debug>(images: [&[u8]; N], all: [T; N]) -> [RetainedImage; N] {
        images
            .iter()
            .zip(all)
            .map(|(bytes, value)| {
                let name = format!("{:?}", value);
                RetainedImage::from_image_bytes(name, bytes).unwrap()
            })
            .attempt_collect()
            .unwrap()
    }

    pub fn new() -> Self {
        let edicts = Self::load_array(EDICT_TEXTURES, Edict::EDICTS);
        let creatures = Self::load_array(CREATURE_TEXTURES, Creature::CREATURES);
        let battlefields = Self::load_array(BATTLEFIELD_TEXTURES, Battlefield::BATTLEFIELDS);

        let card_back = RetainedImage::from_image_bytes("card_back", CARD_BACK).unwrap();

        Self {
            edicts,
            card_back,
            creatures,
            battlefields,
        }
    }
}
// }}}
