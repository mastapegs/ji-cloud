use crate::hebrew_buttons::HebrewButtons;
use crate::text_editor::font_css_converter::font_to_css;
use crate::text_editor::wysiwyg_types::ControlsChange;
use dominator::{clone, html, Dom};
use futures_signals::{signal::SignalExt, signal_vec::SignalVecExt};
use shared::domain::jig::module::body::_groups::design::{Text as RawText, DEFAULT_TEXT_VALUE};
use std::rc::Rc;
use strum::IntoEnumIterator;
use utils::prelude::*;

use super::super::super::state::State;
use super::super::super::wysiwyg_types::{Align, ElementType, Font, Weight, BOLD_WEIGHT};
use super::color_controls;

const STR_WEIGHT_LABEL: &str = "Weight";
const STR_FONT_LABEL: &str = "Font";

const STR_WEIGHT_200: &str = "Light";
const STR_WEIGHT_400: &str = "Regular";
const STR_WEIGHT_700: &str = "Bold";
const STR_WEIGHT_900: &str = "Bolder";
const STR_WEIGHT_CUSTOM: &str = "Custom";

const WEIGHT_OPTIONS: &[u16] = &[200, 400, 700, 900];

fn readable_weight(weight: Weight) -> &'static str {
    match weight {
        200 => STR_WEIGHT_200,
        400 => STR_WEIGHT_400,
        700 => STR_WEIGHT_700,
        900 => STR_WEIGHT_900,
        _ => STR_WEIGHT_CUSTOM,
    }
}

pub fn render(state: Rc<State>) -> Dom {
    html!("text-editor-controls", {
        .property_signal("controlsDisabled", state.wysiwyg_ref.signal_ref(|x| x.is_none()))
        .children(&mut [
            HebrewButtons::full().render(Some("hebrew-buttons")),
            html!("text-editor-controls-insert-button", {
                .property("slot", "insert-button")
                .property_signal("disabled", state.wysiwyg_ref.signal_ref(|x| x.is_some()))
                .event(clone!(state => move |_: events::Click| {
                    if let Some(on_new_text) = state.callbacks.on_new_text.as_ref() {
                        //TODO - this should create a slate value
                        //with the current settings and only replace the text
                        (on_new_text) (&RawText::value_from_str(DEFAULT_TEXT_VALUE));
                    }
                }))
            }),
            html!("input-select", {
                .property("slot", "font")
                .property("label", STR_FONT_LABEL)
                .property_signal("value", state.controls.signal_cloned().map(|controls| controls.font))
                // .style_signal("font-family", state.controls.signal_cloned().map(|controls| format!("'{}'", controls.font.to_string())))
                .children_signal_vec(
                    state
                        .fonts
                        .signal_cloned()
                        .to_signal_vec()
                        .map(clone!(state => move |font| render_font_option(state.clone(), &font)))
                )
            }),
            html!("input-select", {
                .property("slot", "weight")
                .property("label", STR_WEIGHT_LABEL)
                .property_signal("value", state.controls.signal_cloned().map(|controls| readable_weight(controls.weight)))
                .children(WEIGHT_OPTIONS.iter().map(|weight| render_weight_option(state.clone(), *weight)))
            }),
            html!("text-editor-controls-input-number", {
                .property("slot", "font-size")
                .property_signal("value", state.controls.signal_cloned().map(|controls| {
                    controls.font_size
                }))
                .event(clone!(state => move |e: events::CustomChange| {
                    let value = e.value();
                    let value = u8::from_str_radix(&value, 10).unwrap_or(24);
                    state.set_control_value(ControlsChange::FontSize(value))
                }))
            }),
            html!("text-editor-controls-button", {
                .property("kind", "bold")
                .property("slot", "bold")
                .property_signal("active", state.controls.signal_cloned().map(|controls| {
                    controls.weight == BOLD_WEIGHT
                }))
                .event(clone!(state => move |_: events::Click| {
                    state.toggle_bold();
                }))
            }),
            html!("text-editor-controls-button", {
                .property("kind", "italic")
                .property("slot", "italic")
                .property_signal("active", state.controls.signal_cloned().map(|controls| {
                    controls.italic
                }))
                .event(clone!(state => move |_: events::Click| {
                    state.toggle_italic();
                }))
            }),
            html!("text-editor-controls-button", {
                .property("kind", "underline")
                .property("slot", "underline")
                .property_signal("active", state.controls.signal_cloned().map(|controls| {
                    controls.underline
                }))
                .event(clone!(state => move |_: events::Click| {
                    state.toggle_underline();
                }))
            }),
            html!("text-editor-controls-button", {
                .property("kind", "indent")
                .property("slot", "indent")
                .property_signal("active", state.controls.signal_cloned().map(|controls| {
                    controls.indent_count > 0
                }))
                .event(clone!(state => move |_: events::Click| {
                    let count: u8 = state.controls.lock_ref().indent_count + 1;
                    state.set_control_value(ControlsChange::IndentCount(count))
                }))
            }),
            html!("text-editor-controls-button", {
                .property("kind", "outdent")
                .property("slot", "outdent")
                .event(clone!(state => move |_: events::Click| {
                    let mut count: u8 = state.controls.lock_ref().indent_count;
                    if count > 0 {
                        count -= 1;
                    }
                    state.set_control_value(ControlsChange::IndentCount(count))
                }))
                .property_signal("active", state.controls.signal_cloned().map(|controls| {
                    controls.indent_count == 0
                }))
            }),
            color_controls::render(state.clone()),
        ])
        .children(ElementType::iter()
            .map(|element| render_element_option(state.clone(), element))
        )
        .children(Align::iter()
            .map(|align| render_align_option(state.clone(), align))
        )
    })
}

fn render_element_option(state: Rc<State>, element: ElementType) -> Dom {
    html!("text-editor-controls-button", {
        .property("kind", element.to_string().to_lowercase())
        .property("slot", element.to_string().to_lowercase())
        .property_signal("active", state.controls.signal_cloned().map(clone!(element => move |controls| {
            controls.element == element
        })))
        .event(clone!(state, element => move |_: events::Click| {
            state.set_control_value(ControlsChange::Element(element.clone()));
        }))
    })
}

fn render_align_option(state: Rc<State>, align: Align) -> Dom {
    html!("text-editor-controls-button", {
        .property("kind", match align {
            Align::Left => "align-left",
            Align::Center => "align-center",
            Align::Right => "align-right",
        })
        .property("slot", match align {
            Align::Left => "align-left",
            Align::Center => "align-center",
            Align::Right => "align-right",
        })
        .property_signal("active", state.controls.signal_cloned().map(clone!(align => move |controls| {
            controls.align == align
        })))
        .event(clone!(state, align => move |_: events::Click| {
            state.set_control_value(ControlsChange::Align(align.clone()))
        }))
    })
}

fn render_weight_option(state: Rc<State>, weight: Weight) -> Dom {
    html!("input-select-option", {
        .style("font-weight", weight.to_string())
        .property_signal("selected", state.controls.signal_cloned().map(clone!(weight => move |controls| {
            controls.weight == weight
        })))
        .text(readable_weight(weight))
        .event(clone!(state, weight => move |evt: events::CustomSelectedChange| {
            if evt.selected() {
                state.set_control_value(ControlsChange::Weight(weight))
            }
        }))
    })
}

fn render_font_option(state: Rc<State>, font: &Font) -> Dom {
    html!("input-select-option", {
        .style("font-family", font_to_css(font))
        .property_signal("selected", state.controls.signal_cloned().map(clone!(font => move |controls| {
            controls.font == font
        })))
        .text(font)
        .event(clone!(state, font => move |evt: events::CustomSelectedChange| {
            if evt.selected() {
                state.set_control_value(ControlsChange::Font(font.clone()))
            }
        }))
    })
}
