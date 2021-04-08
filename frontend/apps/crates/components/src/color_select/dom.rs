use dominator::{Dom, html, clone};
use futures_signals::signal_vec::SignalVecExt;
use rgb::RGBA8;
use std::rc::Rc;
use std::cell::RefCell;
use utils::prelude::*;
use futures_signals::signal::SignalExt;
use super::{
    actions::{add_user_color, delete_user_color, rgba8_to_hex, hex_to_rgba8},
    state::{ColorSelectConfig}
};
use super::state::State;
use dominator_helpers::futures::AsyncLoader;
use wasm_bindgen_futures::spawn_local;

const STR_SYSTEM_COLORS_LABEL: &'static str = "General colors";
const STR_THEME_COLORS_LABEL: &'static str = "Theme colors";
const STR_USER_COLORS_LABEL: &'static str = "My colors";


pub fn render(config: ColorSelectConfig) -> Dom {
    let state: Rc<RefCell<Option<State>>> = Rc::new(RefCell::new(None));

    let init_loader = AsyncLoader::new();
    init_loader.load(clone!(state => async move {
        state.replace(Some(State::new(config).await));
    }));

    Dom::with_state(init_loader, move |init_loader| {
        html!("empty-fragment", {
            .child_signal(init_loader.is_loading().map(move |loading| {
                if loading {
                    Some(html!("window-loader-block", {
                        .property("visible", true)
                    }))
                } else {
                    let state: State = state.borrow_mut().take().unwrap_ji();
                    Some(render_loaded(Rc::new(state)))
                }
            }))
        })
    })
}


pub fn render_loaded(state: Rc<State>) -> Dom {
    html!("color-select", {
        .child(render_static_section(state.clone(), state.system_colors.as_ref(), STR_SYSTEM_COLORS_LABEL))
        .apply(|dom| {
            match state.theme_colors.as_ref() {
                Some(colors) => dom.child(
                    render_static_section(state.clone(), colors, STR_THEME_COLORS_LABEL)
                ),
                None => dom,
            }
        })
        .child(render_add_color(state.clone()))
        .child_signal(state.user_colors.signal_vec_cloned().to_signal_cloned().map(clone!(state => move |user_colors| {
            if user_colors.len() > 0 {
                // this re-renders every time the anything in the vec changes, there might be better ways of doing the same thing
                Some(render_user_section(state.clone()))
            } else {
                None
            }
        })))
    })
}


fn render_static_section(state: Rc<State>, color_options: &Vec<RGBA8>, label: &str) -> Dom {
    html!("color-select-section", {
        .property("slot", "sections")
        .property("label", label)
        .children(color_options.iter().map(|color| {
            html!("color-select-item", {
                .property("color", rgba8_to_hex(color))
                .property("slot", "items")
                .property_signal("selected", state.value.signal_cloned().map(clone!(color => move |selected_color| {
                    match selected_color {
                        Some(selected_color) => color == selected_color,
                        None => false
                    }
                })))
                .event(clone!(color, state => move |_:events::Click| {
                    state.value.set(Some(color.clone()));
                }))
            })
        }))
    })
}

fn render_user_section(state: Rc<State>) -> Dom {
    html!("color-select-section", {
        .property("slot", "sections")
        .property("label", STR_USER_COLORS_LABEL)
        .children_signal_vec(state.user_colors.signal_vec_cloned().enumerate().map(clone!(state => move |(index, color)| {
            html!("color-select-item", {
                .property("slot", "items")
                .property("color", rgba8_to_hex(&color))
                .property_signal("selected", state.value.signal_cloned().map(clone!(color => move |selected_color| {
                    if selected_color.is_some() {
                        let selected_color = selected_color.unwrap();
                        return selected_color == color;
                    };
                    false
                })))
                .event(clone!(color, state => move |_:events::Click| {
                    state.value.set(Some(color.clone()));
                }))
                .attribute("deletable", "")
                .child(html!("button-icon", {
                    .property("slot", "delete-button")
                    .property("icon", "circle-x-blue")
                    .event(clone!(state => move |_:events::Click| {
                        let index: usize = index.lock_ref().unwrap_or_default();
                        spawn_local(clone!(state => async move {
                            delete_user_color(state.clone(), index).await;
                        }));
                    }))
                }))
            })
        })))
    })
}


fn render_add_color(state: Rc<State>) -> Dom {
    html!("input-color", {
        .property("slot", "add-color")
        .child(html!("button-text", {
            .text("+ Add color")
        }))
        .event(clone!(state => move |e: events::CustomChange| {
            let color = hex_to_rgba8(&e.value());
            spawn_local(clone!(state => async move {
                let _ = add_user_color(state.clone(), color).await;
            }));
            
        }))
    })
}