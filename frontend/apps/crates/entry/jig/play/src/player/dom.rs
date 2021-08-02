use std::rc::Rc;
use dominator_helpers::{events::Message, signals::DefaultSignal};
use futures_signals::map_ref;
use js_sys::Reflect;
use utils::{iframe::{IframeAction, ModuleToJigMessage}, prelude::SETTINGS, routes::{ModuleRoute, Route}, unwrap::UnwrapJiExt};
use futures_signals::signal::{SignalExt, Signal};
use dominator::{Dom, clone, events, html, with_node};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlElement, HtmlIFrameElement};
use super::{actions, sidebar};

use super::state::State;


pub fn render(state: Rc<State>) -> Dom {

    actions::load_jig(state.clone());

    html!("jig-play-landing", {
        .property_signal("paused", state.paused.signal())
        .global_event(clone!(state => move |evt:Message| {
            match evt.try_serde_data::<IframeAction<ModuleToJigMessage>>() {
                Err(_) => {},
                Ok(m) => {
                    actions::on_iframe_message(Rc::clone(&state), m.data)
                },
            };
        }))
        .apply(|dom| {
            if state.is_teacher {
                let sidebar_state = Rc::new(sidebar::state::State::new(state.clone()));
                dom.child(sidebar::dom::render(sidebar_state))
            } else {
                dom
            }
        })
        .apply(clone!(state => move|dom| {
            if state.player_settings.display_score {
                dom.child(html!("jig-play-points-indicator", {
                    .visible(state.player_settings.display_score)
                    .property("slot", "indicators")
                    .property_signal("value", state.points.signal())
                }))
            } else {
                dom
            }
        }))
        .children(&mut [
            html!("iframe" => HtmlIFrameElement, {
                .property("allow", "autoplay; fullscreen")
                .property("slot", "iframe")
                .property_signal("src", state.active_module.signal_cloned().map(clone!(state => move|active_module_index| {
                    match &*state.jig.lock_ref() {
                        None => String::new(),
                        Some(jig) => {
                            let active_module = &jig.modules[active_module_index];

                            let route: String = Route::Module(ModuleRoute::Play(
                                active_module.kind,
                                state.jig_id,
                                active_module.id
                            )).into();
                            let url = unsafe {
                                SETTINGS.get_unchecked()
                                    .remote_target
                                    .spa_iframe(&route)
                            };
                            url
                        },
                    }
                })))
                .after_inserted(clone!(state => move|element| {
                    *state.iframe.borrow_mut() = Some(element);
                }))
            }),
            html!("jig-play-play-button", {
                .property("slot", "play-button")
            }),
            html!("jig-play-play-pause", {
                .property("slot", "play-pause-button")
                .property_signal("mode", state.paused.signal().map(|paused| {
                    match paused {
                        true =>  "pause",
                        false =>  "play",
                    }
                }))
                .event(clone!(state => move |_:events::Click| {
                    actions::toggle_paused(Rc::clone(&state));
                }))
            }),
            html!("jig-play-background-music", {
                .property("slot", "background")
            }),
            html!("jig-play-move-button", {
                .property("slot", "back")
                .property("kind", "back")
                .visible_signal(state.active_module.signal().map(|active_module| {
                    active_module != 0
                }))
                .event(clone!(state => move |_: events::Click| {
                    let mut active_module = state.active_module.lock_mut();
                    if *active_module != 0 {
                        *active_module -= 1;
                    }
                }))
            }),
            html!("jig-play-progress-bar", {
                .property("slot", "progress")
                .property_signal("percent", progress_signal(state.clone()))
            }),
            html!("jig-play-move-button", {
                .property("slot", "forward")
                .property("kind", "forward")
                .visible_signal(state.active_module.signal().map(clone!(state => move |active_module| {
                    match &*state.jig.lock_ref() {
                        None => true,
                        Some(jig) => {
                            let module_length = jig.modules.len();
                            active_module != module_length - 1
                        },
                    }
                })))
                .event(clone!(state => move |_: events::Click| {
                    let mut active_module = state.active_module.lock_mut();
                    if let Some(jig) = &*state.jig.lock_ref() {
                        if *active_module < jig.modules.len() - 1 {
                            *active_module += 1;
                        }
                    }
                }))
            }),
        ])
        .child_signal(render_time_indicator(Rc::clone(&state)))
        // .child_signal(render_done_popup(Rc::clone(&state)))
        .child_signal(render_time_up_popup(Rc::clone(&state)))
    })
}

fn ten_sec_signal(state: Rc<State>) -> impl Signal<Item = bool> {
    state.timer.signal_cloned().map(|timer| {
        DefaultSignal::new(false, timer.map(|timer| {
            timer.time.signal().map(|time| {
                time == 10
            })
        }))
    }).flatten()
}

fn progress_signal(state: Rc<State>) -> impl Signal<Item = u32> {
    (map_ref! {
        let active_module = state.active_module.signal(),
        let jig = state.jig.signal_cloned() =>
            (*active_module, jig.clone())
    }).map(move|(active_module_index, jig)| {
        match jig {
            None => 0,
            Some(jig) => {
                let len = jig.modules.len();
                let step_percent = 100f32 / len as f32;
                let current_progress = active_module_index as f32 * step_percent;
                // TODO: ask corrine if this should be here
                let current_progress = current_progress + step_percent;
                log::info!("{}", current_progress);
                current_progress.round() as u32
            },
        }
    })
}


fn render_done_popup(state: Rc<State>) -> impl Signal<Item = Option<Dom>> {
    state.active_module.signal().map(clone!(state => move |_| {
        Some(html!("dialog-overlay", {
            .property("slot", "dialog")
            .property("open", true)
            .property("autoClose", false)
            .child(html!("jig-play-done-popup", {
                .apply(|mut dom| {
                    if state.player_settings.display_score {
                        dom = dom.property_signal("score", state.points.signal());
                    };
                    if !state.player_settings.assessment_mode {
                        dom = dom.child(
                            html!("jig-play-replay", {
                                .property("slot", "actions")
                                .event(clone!(state => move |_: events::Click| {
                                    actions::reload_iframe(Rc::clone(&state));
                                }))
                            })
                        );
                    }
                    dom
                })
            }))
        }))
    }))
}


fn time_up_signal(state: Rc<State>) -> impl Signal<Item = bool> {
    state.timer.signal_cloned().map(|timer| {
        DefaultSignal::new(false, timer.map(|timer| {
            timer.time.signal().map(|time| {
                time == 0
            })
        }))
    }).flatten()
}

fn render_time_up_popup(state: Rc<State>) -> impl Signal<Item = Option<Dom>> {
    time_up_signal(Rc::clone(&state)).map(clone!(state => move |time_up| {
        match time_up {
            false => None,
            true => {
                Some(html!("dialog-overlay", {
                    .property("slot", "dialog")
                    .property("open", true)
                    .property("autoClose", false)
                    .child(html!("jig-play-time-up-popup", {
                        .apply(|mut dom| {
                            if !state.player_settings.assessment_mode {
                                dom = dom.child(
                                    html!("jig-play-replay", {
                                        .property("slot", "actions")
                                        .event(clone!(state => move |_: events::Click| {
                                            actions::reload_iframe(Rc::clone(&state));
                                        }))
                                    })
                                );
                            }
                            dom
                        })
                    }))
                }))
            }
        }
    }))
}

fn render_time_indicator(state: Rc<State>) -> impl Signal<Item = Option<Dom>> {
    state.timer.signal_cloned().map(clone!(state => move |timer| {
        match timer {
            None => None,
            Some(timer) => {
                Some(html!("jig-play-timer-indicator" => HtmlElement, {
                    .property("slot", "indicators")
                    .property_signal("value", timer.time.signal())
                    .with_node!(elem => {
                        .future(ten_sec_signal(Rc::clone(&state)).for_each(move |less_than_10_sec| {
                            if less_than_10_sec {
                                let buzz_method = Reflect::get(
                                    &elem,
                                    &JsValue::from_str("buzz")
                                )
                                    .unwrap();
                                log::info!("{:?}", buzz_method);
                                let buzz_method = buzz_method.dyn_ref::<js_sys::Function>().unwrap_ji();
                                let _ = buzz_method.call0(&elem);
                            }
                            async {}
                        }))
                    })
                }))
            }
        }
    }))
}