use components::{audio::mixer::AudioMixer, module::_common::play::prelude::*};
use shared::domain::jig::{
    module::{
        body::{
            Instructions,
            _groups::design::{Backgrounds, Sticker},
            video::{Mode, ModuleData as RawData, PlaySettings, Step},
        },
        ModuleId,
    },
    Jig, JigId,
};
use std::rc::Rc;
use utils::prelude::*;

pub struct Base {
    pub jig_id: JigId,
    pub module_id: ModuleId,
    pub jig: Jig,
    pub theme_id: ThemeId,
    pub instructions: Instructions,
    pub backgrounds: Backgrounds,
    pub stickers: Vec<Sticker>,
    pub play_settings: PlaySettings,
    pub module_phase: Mutable<ModulePlayPhase>,
}

impl Base {
    pub async fn new(init_args: InitFromRawArgs<RawData, Mode, Step>) -> Rc<Self> {
        let InitFromRawArgs {
            jig_id,
            module_id,
            jig,
            raw,
            theme_id,
            ..
        } = init_args;

        let content = raw.content.unwrap_ji();
        let base_content = content.base;

        Rc::new(Self {
            jig_id,
            module_id,
            jig,
            theme_id,
            instructions: base_content.instructions,
            backgrounds: base_content.backgrounds,
            stickers: base_content.stickers,
            play_settings: content.play_settings,
            module_phase: init_args.play_phase,
        })
    }
}

impl BaseExt for Base {
    fn get_instructions(&self) -> Option<Instructions> {
        Some(self.instructions.clone())
    }
    fn play_phase(&self) -> Mutable<ModulePlayPhase> {
        self.module_phase.clone()
    }
}
