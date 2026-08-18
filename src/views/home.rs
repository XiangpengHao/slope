use dioxus::prelude::*;

/// The page at `/`.
#[component]
pub fn Home() -> Element {
    rsx! {
        div { class: "p-6",
            h1 { class: "text-lg font-semibold", "slopify" }
        }
    }
}
