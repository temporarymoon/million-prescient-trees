use egui_extras::RetainedImage;

use crate::{game::edict::Edict, helpers::try_from_iter::TryCollect};

pub struct AppTextures {
    pub edicts: [RetainedImage; 5],
    pub card_back: RetainedImage,
}

const EDICT_TEXTURES: [&[u8]; 5] = [
    include_bytes!("../../assets/edicts/rilethepublic.jpeg"),
    include_bytes!("../../assets/edicts/divertattention.jpeg"),
    include_bytes!("../../assets/edicts/sabotage.jpeg"),
    include_bytes!("../../assets/edicts/gambit.jpeg"),
    include_bytes!("../../assets/edicts/ambush.jpeg"),
];

const CARD_BACK: &[u8] = include_bytes!("../../assets/cardback.png");

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

        let card_back = RetainedImage::from_image_bytes("card_back", CARD_BACK).unwrap();

        Self { edicts, card_back }
    }
}
