use dominator::{Dom, html};
use futures_signals::signal::Signal;
use std::rc::Rc;
use utils::routes::{HomeRoute, Route};
use super::{home, student_code};


pub struct Router {
}

impl Router {
    pub fn new() -> Self {
        Self { }
    }

    pub fn render() -> Dom {
        html!("main", {
            .child_signal(Self::dom_signal())
        })
    }

    fn dom_signal() -> impl Signal<Item = Option<Dom>> {
        dominator::routing::url()
            .signal_ref(|url| {
                let route = Route::from_url(&url);
                match route {
                    Route::Home(route) => {
                        match route {
                            HomeRoute::Home => {
                                Some(home::dom::render(Rc::new(home::state::State::new())))
                            },
                            HomeRoute::StudentCode => {
                                Some(student_code::dom::render(Rc::new(student_code::state::State::new())))
                            },
                        }
                    }
                    _ => None
                }
            })
    }
}
