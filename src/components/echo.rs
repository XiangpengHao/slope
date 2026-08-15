use dioxus::prelude::*;

/// Round-trips text through a server function, to prove the fullstack wiring works.
#[component]
pub fn Echo() -> Element {
    let mut response = use_signal(String::new);

    rsx! {
        section { class: "mt-8 rounded-lg border border-slate-200 p-6 dark:border-slate-800",
            h2 { class: "text-sm font-medium uppercase tracking-wide text-slate-500", "Server function" }

            input {
                class: "mt-3 w-full rounded-md border border-slate-300 bg-white px-3 py-2 outline-none
                        focus:border-sky-500 focus:ring-1 focus:ring-sky-500
                        dark:border-slate-700 dark:bg-slate-900",
                placeholder: "Type here to echo…",
                oninput: move |event| async move {
                    // Calling this from the client serializes the args, hits the
                    // endpoint, and deserializes the response.
                    if let Ok(data) = echo_server(event.value()).await {
                        response.set(data);
                    }
                },
            }

            if !response().is_empty() {
                p { class: "mt-3 text-slate-600 dark:text-slate-400",
                    "Server echoed: "
                    i { class: "text-slate-900 dark:text-slate-100", "{response}" }
                }
            }
        }
    }
}

// Server function bodies compile only into the server binary, so database
// queries and other server-only logic belong here. Imports they need go inside
// the function or behind `#[cfg(feature = "server")]`.
#[post("/api/echo")]
async fn echo_server(input: String) -> Result<String> {
    Ok(input)
}
