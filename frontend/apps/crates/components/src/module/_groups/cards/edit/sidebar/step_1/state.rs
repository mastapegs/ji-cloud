use crate::{
    image::search::{
        callbacks::Callbacks as ImageSearchCallbacks,
        state::{ImageSearchKind, ImageSearchOptions, State as ImageSearchState},
    },
    lists::{
        dual::{
            callbacks::Callbacks as DualListCallbacks,
            state::{Options as DualListOptions, State as DualListState},
        },
        single::{
            callbacks::Callbacks as SingleListCallbacks,
            state::{Options as SingleListOptions, State as SingleListState},
        },
    },
    module::_groups::cards::edit::{config, state::*, strings},
    tabs::MenuTabKind,
};
use dominator::clone;
use futures_signals::signal::Mutable;
use once_cell::sync::OnceCell;
use shared::{
    domain::jig::module::body::{Image, _groups::cards::Mode},
    config as shared_config,
};
use std::rc::Rc;

pub struct Step1<RawData: RawDataExt, E: ExtraExt> {
    pub base: Rc<CardsBase<RawData, E>>,
    pub tabs: OnceCell<Vec<Tab>>,
    pub tab_index: Mutable<Option<usize>>,
}

impl<RawData: RawDataExt, E: ExtraExt> Step1<RawData, E> {
    pub fn new(base: Rc<CardsBase<RawData, E>>, tab_index: Mutable<Option<usize>>) -> Rc<Self> {
        // If the tab index isn't set yet, make it the first tab
        if tab_index.lock_ref().is_none() {
            tab_index.set(Some(0));
        }

        let state = Rc::new(Self {
            base: base.clone(),
            tabs: OnceCell::default(),
            tab_index,
        });

        // Widgets require a reference to the top-level state so that they can have access to any
        // fields they might require in callbacks.
        let tabs = match base.mode {
            Mode::WordsAndImages => {
                vec![
                    Tab::new(state.clone(), MenuTabKind::Text),
                    Tab::new(state.clone(), MenuTabKind::Image),
                ]
            }
            Mode::Duplicate | Mode::Lettering => vec![Tab::new(state.clone(), MenuTabKind::Text)],
            _ => vec![Tab::new(state.clone(), MenuTabKind::DualList)],
        };

        // TODO add audio tab to all modes

        // `set()` will return an Err if the cell already has a value. However, because we are
        // using this purely to lazily set the tabs immediately after initializing the state, the
        // error here will never occur.
        let _ = state.tabs.set(tabs);

        state
    }
}

#[derive(Clone)]
pub enum Tab {
    Single(Rc<SingleListState>),
    Dual(Rc<DualListState>),
    Image(Rc<ImageSearchState>),
}

impl Tab {
    pub fn new<RawData: RawDataExt, E: ExtraExt>(
        state: Rc<Step1<RawData, E>>,
        kind: MenuTabKind,
    ) -> Self {
        match kind {
            MenuTabKind::Image => {
                let opts = ImageSearchOptions {
                    kind: ImageSearchKind::Sticker,
                    ..ImageSearchOptions::default()
                };

                let callbacks = ImageSearchCallbacks::new(None::<fn(Image)>);
                let state = ImageSearchState::new(opts, callbacks);

                Self::Image(Rc::new(state))
            }
            MenuTabKind::Text => Self::Single(Rc::new(make_single_list(state))),
            MenuTabKind::DualList => Self::Dual(Rc::new(make_dual_list(state))),

            _ => unimplemented!("unsupported tab kind!"),
        }
    }

    pub fn kind(&self) -> MenuTabKind {
        match self {
            Self::Single(_) => MenuTabKind::Text,
            Self::Dual(_) => MenuTabKind::DualList,
            Self::Image(_) => MenuTabKind::Image,
        }
    }
}

fn make_single_list<RawData: RawDataExt, E: ExtraExt>(
    state: Rc<Step1<RawData, E>>,
) -> SingleListState {
    let _mode = state.base.mode;

    let callbacks = SingleListCallbacks::new(
        |text| super::actions::limit_text(config::SINGLE_LIST_CHAR_LIMIT, text),
        clone!(state => move |tooltip| {
            state.base.tooltips.list_error.set(tooltip);
        }),
        clone!(state => move |list| {
            state.base.replace_single_list(list);

            // If the current mode is words and images, then at this point the user can be
            // navigated directly to the Image tab.
            if matches!(state.base.mode, Mode::WordsAndImages) {
                state.tab_index.set_neq(Some(1));
            }
        }),
        config::get_single_list_init_word,
    );

    let options = SingleListOptions {
        max_rows: shared_config::MAX_LIST_WORDS,
        min_valid: shared_config::MIN_LIST_WORDS,
    };

    SingleListState::new(options, callbacks)
}
fn make_dual_list<RawData: RawDataExt, E: ExtraExt>(
    state: Rc<Step1<RawData, E>>,
) -> DualListState {
    let mode = state.base.mode;

    let callbacks = DualListCallbacks::new(
        |text| super::actions::limit_text(config::DUAL_LIST_CHAR_LIMIT, text),
        clone!(state => move |tooltip| {
            state.base.tooltips.list_error.set(tooltip);
        }),
        clone!(state => move |list| {
            state.base.replace_dual_list(list);
        }),
        config::get_dual_list_init_word,
        clone!(mode => move |side| {
            strings::STR_HEADER(side, mode).to_string()
        }),
    );

    let options = DualListOptions {
        max_rows: shared_config::MAX_LIST_WORDS,
        cell_rows: {
            match state.base.mode {
                Mode::Riddles => 2,
                _ => 1,
            }
        },
        min_valid: shared_config::MIN_LIST_WORDS,
    };

    DualListState::new(options, callbacks)
}
