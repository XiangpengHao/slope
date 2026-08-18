use crate::Route;
use dioxus::prelude::*;

/// Wraps every page. Child routes render in the [`Outlet`].
#[component]
pub fn Navbar() -> Element {
    rsx! {
        div { class: "min-h-screen bg-white text-zinc-900",
            header { class: "border-b border-zinc-200 px-6 py-3",
                Link { to: Route::Home {}, class: "font-semibold", "slopify" }
            }
            Outlet::<Route> {}
        }
    }
}
