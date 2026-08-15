use dioxus::prelude::*;

use crate::components::Echo;

#[component]
pub fn Home() -> Element {
    rsx! {
        h1 { class: "text-3xl font-semibold tracking-tight", "rust-viewer" }
        p { class: "mt-2 text-slate-600 dark:text-slate-400",
            "Dioxus fullstack app — frontend and server live in this one crate."
        }

        Echo {}
    }
}
