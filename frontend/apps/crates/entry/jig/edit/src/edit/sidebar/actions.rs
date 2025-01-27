use super::state::*;
use dominator::clone;
use futures_signals::{
    map_ref,
    signal::{Signal, SignalExt},
};
use shared::{
    api::endpoints::{self, ApiEndpoint},
    domain::{
        jig::{
            module::{ModuleCreateRequest, ModuleId, ModuleResponse},
            JigId, JigPlayerSettings, JigResponse, JigUpdateDraftDataRequest, LiteModule,
            ModuleKind,
        },
        CreateResponse,
    },
    error::EmptyError,
};
use std::cell::RefCell;
use std::rc::Rc;
use utils::{iframe::ModuleToJigEditorMessage, jig::JigPlayerOptions, prelude::*};

pub async fn load_jig(jig_id: JigId, jig_cell: Rc<RefCell<Option<JigResponse>>>) {
    let path = endpoints::jig::GetDraft::PATH.replace("{id}", &jig_id.0.to_string());

    match api_with_auth::<JigResponse, EmptyError, ()>(
        &path,
        endpoints::jig::GetDraft::METHOD,
        None,
    )
    .await
    {
        Ok(resp) => {
            assert!(
                resp.jig_focus.is_modules(),
                "only module focused jigs should be here"
            );
            *jig_cell.borrow_mut() = Some(resp);
        }
        Err(_) => {}
    }
}

pub fn navigate_to_publish(state: Rc<State>) {
    state.jig_edit_state.route.set_neq(JigEditRoute::Publish);
    state.collapsed.set(true);

    let jig_id = state.jig.id;
    Route::push_state(Route::Jig(JigRoute::Edit(
        jig_id,
        state.jig.jig_focus,
        JigEditRoute::Publish,
    )));
}

pub fn set_highlight_modules(state: &Rc<State>, highlight: bool) {
    if highlight {
        let modules = state.modules.lock_ref();

        if modules.iter().filter(|module| module.is_some()).count() == 0 {
            state
                .highlight_modules
                .set_neq(Some(ModuleHighlight::Publish))
        } else {
            let idx = modules.iter().position(|module| match &**module {
                Some(module) => !module.is_complete.get_cloned(),
                None => false,
            });
            match idx {
                Some(idx) => state
                    .highlight_modules
                    .set_neq(Some(ModuleHighlight::Module(idx))),
                None => state.highlight_modules.set_neq(None),
            }
        }
    } else {
        state.highlight_modules.set_neq(None);
    }
}

pub async fn update_jig(jig_id: &JigId, req: JigUpdateDraftDataRequest) -> Result<(), EmptyError> {
    let path = endpoints::jig::UpdateDraftData::PATH.replace("{id}", &jig_id.0.to_string());
    api_with_auth_empty::<EmptyError, _>(&path, endpoints::jig::UpdateDraftData::METHOD, Some(req))
        .await
}

pub fn update_display_name(state: Rc<State>, value: String) {
    state.loader.load(clone!(state => async move {
        state.name.set(value.clone());

        let req = JigUpdateDraftDataRequest {
            display_name: Some(value),
            ..Default::default()
        };

        match update_jig(&state.jig.id, req).await {
            Ok(_) => {},
            Err(_) => {},
        }
    }));
}

pub fn duplicate_module(state: Rc<State>, module_id: &ModuleId) {
    state.loader.load(clone!(state, module_id => async move {
        let module = super::module_cloner::clone_module(&state.jig.id, &module_id, &state.jig.id).await.unwrap_ji();
        populate_added_module(state.clone(), module);
    }));
}

pub fn _player_settings_change_signal(state: Rc<State>) -> impl Signal<Item = JigPlayerSettings> {
    let sig = map_ref! {
        let direction = state.settings.direction.signal_cloned(),
        let display_score = state.settings.display_score.signal(),
        let track_assessments = state.settings.track_assessments.signal(),
        let drag_assist = state.settings.drag_assist.signal()
        => ( direction.clone(), display_score.clone(), track_assessments.clone(), drag_assist.clone())
    };

    sig.map(
        |(direction, display_score, track_assessments, drag_assist)| JigPlayerSettings {
            direction: direction.clone(),
            display_score: display_score.clone(),
            track_assessments: track_assessments.clone(),
            drag_assist: drag_assist.clone(),
        },
    )
}

pub fn get_player_settings(state: Rc<State>) -> JigPlayerOptions {
    let direction = state.settings.direction.get_cloned();
    let display_score = state.settings.display_score.get();
    let track_assessments = state.settings.track_assessments.get();
    let drag_assist = state.settings.drag_assist.get();

    JigPlayerOptions {
        direction,
        display_score,
        track_assessments,
        drag_assist,
        is_student: false,
        draft: true,
    }
}

pub fn on_iframe_message(state: Rc<State>, message: ModuleToJigEditorMessage) {
    match message {
        ModuleToJigEditorMessage::AppendModule(module) => {
            populate_added_module(Rc::clone(&state), module);
        }
        ModuleToJigEditorMessage::Complete(module_id, is_complete) => {
            let modules = state.modules.lock_ref();
            let module = modules.into_iter().find(|module| {
                // Oh my.
                match &***module {
                    Some(module) => *module.id() == module_id,
                    None => false,
                }
            });

            if let Some(module) = module {
                if let Some(module) = &**module {
                    module.is_complete.set_neq(is_complete);
                }
            }
        }
        ModuleToJigEditorMessage::Next => {
            state.jig_edit_state.route.set_neq(JigEditRoute::Landing);
            let jig_id = state.jig.id;
            Route::push_state(Route::Jig(JigRoute::Edit(
                jig_id,
                state.jig.jig_focus,
                JigEditRoute::Landing,
            )));
        }
        ModuleToJigEditorMessage::Publish => {
            navigate_to_publish(state);
        }
    }
}

fn populate_added_module(state: Rc<State>, module: LiteModule) {
    // Assumes that the final module in the list is always the placeholder module.
    let insert_at_idx = state.modules.lock_ref().len() - 1;

    let module_id = module.id;

    state
        .modules
        .lock_mut()
        .insert_cloned(insert_at_idx, Rc::new(Some(module.into())));

    state
        .jig_edit_state
        .route
        .set_neq(JigEditRoute::Module(module_id));
}

pub fn use_module_as(state: Rc<State>, target_kind: ModuleKind, source_module_id: ModuleId) {
    state.loader.load(clone!(state => async move {
        let target_module_id: Result<(ModuleId, bool), EmptyError> = async {
            let path = endpoints::jig::module::GetDraft::PATH
                .replace("{id}", &state.jig.id.0.to_string())
                .replace("{module_id}", &source_module_id.0.to_string());

            let source_module = api_with_auth::<ModuleResponse, EmptyError, ()>(
                &path,
                endpoints::jig::module::GetDraft::METHOD,
                None
            ).await?.module;

            let target_body = source_module.body.convert_to_body(target_kind).unwrap_ji();

            let path = endpoints::jig::module::Create::PATH
                .replace("{id}", &state.jig.id.0.to_string());

            let req = ModuleCreateRequest { body: target_body };

            let res = api_with_auth::<CreateResponse<ModuleId>, EmptyError, ModuleCreateRequest>(
                &path,
                endpoints::jig::module::Create::METHOD,
                Some(req),
            )
            .await?;

            Ok((res.id, source_module.is_complete))
        }.await;

        match target_module_id {
            Err(_) => {
                log::error!("request to create module failed!");
            },
            Ok((target_module_id, is_complete)) => {
                let lite_module = LiteModule {
                    id: target_module_id,
                    kind: target_kind,
                    is_complete,
                };
                populate_added_module(Rc::clone(&state), lite_module);
            },
        };
    }));
}
