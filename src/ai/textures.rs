use egui_extras::RetainedImage;

use crate::{game::edict::Edict, helpers::try_from_iter::TryCollect};

pub struct AppTextures {
    pub edicts: [RetainedImage; 5],
}

const EDICT_TEXTURES: [&[u8]; 5] = [
    include_bytes!("../../assets/edicts/rilethepublic.jpeg"),
    include_bytes!("../../assets/edicts/divertattention.jpeg"),
    include_bytes!("../../assets/edicts/sabotage.jpeg"),
    include_bytes!("../../assets/edicts/gambit.jpeg"),
    include_bytes!("../../assets/edicts/ambush.jpeg"),
];

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

        Self { edicts }
    }
}
