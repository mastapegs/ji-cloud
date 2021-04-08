use dominator::{html, Dom, clone};
use crate::data::{raw, state::*};
use std::rc::Rc;
use std::cell::RefCell;
use utils::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;
use js_sys::Reflect;
use futures_signals::{
    map_ref,
    signal::{ReadOnlyMutable, SignalExt},
    signal_vec::SignalVecExt,
};
use components::image_search::types::*;

pub struct PairDom {}
impl PairDom {
    pub fn render(state:Rc<State>, game_mode: GameMode, step: Step, index: ReadOnlyMutable<Option<usize>>, pair:(Card, Card)) -> Dom {

        let left = CardDom::render(state.clone(), game_mode, step, index.clone(), Side::Left, pair.0.clone(), pair.1.clone());
        let right = CardDom::render(state.clone(), game_mode, step, index.clone(), Side::Right, pair.1, pair.0);

        if step == Step::One {
            html!("main-card-pair", {
                .property("hoverable", true)
                .property_signal("index", index.signal().map(|x| {
                    JsValue::from_f64(x.unwrap_or_default() as f64)
                }))
                .child(left)
                .child(right)
                .child(html!("button-icon", {
                    .property("slot", "close")
                    .property("icon", "circle-x-blue")
                    .event(clone!(state => move |evt:events::Click| {
                        state.delete_pair(index.get().unwrap_or_default());
                    }))
                }))
            })
        } else {
            html!("main-card-pair", {
                .property("hoverable", false)
                .property_signal("index", index.signal().map(|x| {
                    JsValue::from_f64(x.unwrap_or_default() as f64)
                }))
                .child(left)
                .child(right)
            })
        }
    }
}

struct CardDom {}

impl CardDom {
    pub fn render(state:Rc<State>, game_mode: GameMode, step: Step, index: ReadOnlyMutable<Option<usize>>, side:Side, card: Card, other: Card) -> Dom {
        let input_ref:Rc<RefCell<Option<HtmlElement>>> = Rc::new(RefCell::new(None));

        html!("main-card", {
            .property("slot", side.slot_name())
            .property("flippable", step == Step::Two)
            .property("editing", step == Step::One)
            .property_signal("theme", state.theme_id_str_signal())
            .event(clone!(input_ref => move |evt:events::Click| {
                if let Some(input_ref) = input_ref.borrow().as_ref() {
                    Reflect::set(input_ref, &JsValue::from_str("editing"), &JsValue::from_bool(true));
                }
            }))
            .child({
                match card {
                    Card::Text(data) => {
                        html!("input-textarea-content", {
                            .property_signal("value", data.signal_cloned())
                            .property("clickMode", "none")
                            .event(clone!(state, index, other => move |evt:events::CustomInput| {
                                let index = index.get().unwrap_or_default();
                                let value = evt.value();

                                if game_mode == GameMode::Duplicate {
                                    other.as_text_mutable().set_neq(value);
                                }
                            }))
                            .event(clone!(state, index => move |evt:events::CustomChange| {
                                let index = index.get().unwrap_or_default();
                                let value = evt.value();
                                state.replace_card_text(index, side, value);
                            }))
                            .event(clone!(state, other => move |evt:events::Reset| {
                                //Just need to change the linked pair
                                //without affecting history
                                if game_mode == GameMode::Duplicate {
                                    //other.as_text_mutable().set_neq(original_data.clone());
                                    other.as_text_mutable().set_neq(data.get_cloned());
                                }
                            }))
                            .after_inserted(clone!(input_ref => move |dom| {
                                *input_ref.borrow_mut() = Some(dom);
                            }))
                        })
                    },
                    Card::Image(data) => {
                        html!("empty-fragment", {
                            .child_signal(data.signal_cloned().map(clone!(state => move |data| {
                                Some(match data {
                                    None => {
                                        html!("img-ui", {
                                            .property("path", "core/_common/image-empty.svg")
                                            .event_preventable(clone!(state => move |evt:events::DragOver| {
                                                if let Some(data_transfer) = evt.data_transfer() {
                                                    if data_transfer.types().index_of(&JsValue::from_str(IMAGE_SEARCH_DATA_TRANSFER), 0) != -1 {
                                                        evt.prevent_default();
                                                    }
                                                }

                                            }))
                                            .event(clone!(state, index => move |evt:events::Drop| {
                                                if let Some(data_transfer) = evt.data_transfer() {
                                                    if let Some(data) = data_transfer.get_data(IMAGE_SEARCH_DATA_TRANSFER).ok() { 
                                                        let data:ImageDataTransfer = serde_json::from_str(&data).unwrap_ji();
                                                        let index = index.get().unwrap_or_default();
                                                        state.replace_card_image(index, side, (data.id, data.lib));
                                                    }
                                                }
                                            }))
                                        })
                                    },
                                    Some(data) => {
                                        html!("img-ji", {
                                            .property("size", "full")
                                            .property("id", data.0.0.to_string())
                                            .property("lib", data.1.to_str())
                                        })
                                    }
                                })
                            })))
                        })
                    },
                    _ => unimplemented!("can't render other types yet!")
                }
            })
        })
    }
}