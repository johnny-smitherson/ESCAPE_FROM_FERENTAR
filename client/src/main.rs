use dioxus::prelude::*;

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Blog(id: i32) -> Element {
    rsx! {
        Link { to: Route::Home {}, "Go to counter" }
        "Blog post {id}"
    }
}

#[component]
fn Home() -> Element {
    let mut count = use_signal(|| 0);
    use std::{collections::VecDeque, fmt::Debug, rc::Rc};

        // Using a VecDeque so its cheap to pop old events off the front
        let mut events = use_signal(VecDeque::new);

        // All events and their data implement Debug, so we can re-cast them as Rc<dyn Debug> instead of their specific type
        let mut log_event = move |event: Rc<dyn Debug>| {
            // Only store the last 20 events
            if events.read().len() >= 5 {
                events.write().pop_front();
            }
            events.write().push_back(event);
        };

    rsx! {
        Link {
            to: Route::Blog {
                id: count()
            },
            "Go to blog"
        }
        div {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        }

        div { id: "container",
            // focusing is necessary to catch keyboard events
            div { id: "receiver", tabindex: 0,
                onmousemove: move |event| log_event(event.data()),
                onclick: move |event| log_event(event.data()),
                ondoubleclick: move |event| log_event(event.data()),
                onmousedown: move |event| log_event(event.data()),
                onmouseup: move |event| log_event(event.data()),

                onwheel: move |event| log_event(event.data()),

                onkeydown: move |event| log_event(event.data()),
                onkeyup: move |event| log_event(event.data()),
                onkeypress: move |event| log_event(event.data()),

                onfocusin: move |event| log_event(event.data()),
                onfocusout: move |event| log_event(event.data()),

                "Hover, click, type or scroll to see the info down below"
            }
            div { id: "log",
                for event in events.read().iter() {
                    div { "{event:?}" }
                }
            }
        }
    }
}