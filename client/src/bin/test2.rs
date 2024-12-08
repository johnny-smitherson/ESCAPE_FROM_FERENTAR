
use dioxus::prelude::*;
use dioxus_logger::tracing::info;

fn main() {
   dioxus::launch(MapsCrosshair);
}

#[component]
pub fn MapsCrosshair() -> Element {
    info!("MapsCrossHair()");
    rsx! {
        div {
            div { style: "
                z-index: 0;
                background-color: #FFF;
                mix-blend-mode: difference;
                width: 10vmin;
                height: 0.5vmin;
                margin-left: -5vmin;
                margin-top: -0.25vmin;
                position: absolute;
                left: 50vw;
                top: 50vh;
            " },
        div { style: "
                z-index: 0;
                background-color: #FFF;
                mix-blend-mode: difference;
                width: 0.5vmin;
                height: 10vmin;
                margin-left: -0.25vmin;
                margin-top: -5vmin;
                position: absolute;
                left: 50vw;
                top: 50vh;
            " }
        }
    }
}