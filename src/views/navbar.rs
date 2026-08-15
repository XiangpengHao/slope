use dioxus::prelude::*;

use crate::Route;

/// Layout wrapping every route: a header plus an [`Outlet`] for the active page.
#[component]
pub fn Navbar() -> Element {
    rsx! {
        div { class: "min-h-screen bg-slate-50 text-slate-900 dark:bg-slate-950 dark:text-slate-100",
            header { class: "border-b border-slate-200 dark:border-slate-800",
                nav { class: "mx-auto flex max-w-5xl items-center gap-6 px-6 py-4",
                    Link {
                        to: Route::Home {},
                        class: "font-semibold tracking-tight hover:text-sky-600 dark:hover:text-sky-400",
                        "rust-viewer"
                    }
                }
            }

            main { class: "mx-auto max-w-5xl px-6 py-10",
                // Renders the component for the active child route.
                Outlet::<Route> {}
            }
        }
    }
}
