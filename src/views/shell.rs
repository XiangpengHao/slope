//! The frame both lenses hang in.
//!
//! The shell owns the workspace identity, the lens tabs, the finder and the
//! keyboard — everything that is true whichever graph you are reading. A lens
//! owns its pane and its record and nothing else, which is what lets a third
//! lens mount here as a sibling rather than as a tab bolted onto one of these.


use dioxus::prelude::*;

use crate::Route;
use crate::api::{GraphLoad, SheetLoad, load_graph, load_sheet};
use crate::components::TopBar;
use crate::graph::focus;
use crate::views::{DepsState, SheetState, calls};
use dioxus_flow::Way;

/// The resolved workspace, shared by everything under the shell.
pub type GraphResource = Resource<Result<GraphLoad>>;

/// The extracted call sheet, or `None` until the reader opens the lens that
/// needs it.
pub type SheetResource = Resource<Option<Result<SheetLoad>>>;

#[component]
pub fn Shell() -> Element {
    let resource: GraphResource = use_resource(load_graph);
    use_context_provider(|| resource);

    let state = DepsState {
        held: use_signal(|| None),
        query: use_signal(String::new),
        root: use_signal(|| None),
        opened: use_signal(Default::default),
        history: use_signal(Vec::new),
        aim: use_signal(|| None),
    };
    use_context_provider(|| state);

    // The call sheet costs an analyser process and ten seconds of indexing, so
    // it is not fetched until the reader opens the lens that needs it. Loading
    // the dependency view used to start that job and pull 581kB for it, for a
    // question nobody had asked.
    //
    // Latched rather than tied to the route: once the reader has been to the
    // call lens, leaving it does not throw the sheet away.
    let mut asked = use_signal(|| false);
    let sheet: SheetResource = use_resource(move || async move {
        if !asked() {
            return None;
        }
        Some(load_sheet().await)
    });
    use_context_provider(|| sheet);

    let sheet_state = SheetState {
        held: use_signal(|| None),
        query: use_signal(String::new),
        nest: use_signal(Default::default),
        history: use_signal(Vec::new),
        aim: use_signal(|| None),
    };
    use_context_provider(|| sheet_state);

    // Keep the trail. Re-holding something already at the end is not a step.
    use_effect(move || {
        let Some(id) = (state.held)() else {
            return;
        };
        let mut history = state.history;
        if history.peek().last() != Some(&id) {
            history.write().push(id);
        }
    });
    use_effect(move || {
        let Some(id) = (sheet_state.held)() else {
            return;
        };
        let mut history = sheet_state.history;
        if history.peek().last() != Some(&id) {
            history.write().push(id);
        }
    });

    // Which lens the keys are steering. Both lenses answer the same gestures —
    // step left into what points at this, step right into what this points at —
    // so there is one listener and it aims at whichever lens is mounted.
    let on_calls = matches!(use_route::<Route>(), Route::Calls { .. });
    let mut lens = use_signal(|| false);
    use_effect(move || {
        lens.set(on_calls);
        if on_calls {
            asked.set(true);
        }
    });

    use_global_keys(state, resource, sheet_state, sheet, lens);

    rsx! {
        div { class: "flex h-screen flex-col overflow-hidden bg-canvas text-ink",
            TopBar {}
            main { class: "relative flex min-h-0 flex-1", Outlet::<Route> {} }
        }
    }
}

/// Which way a step travels along the law of the graph: left is what points at
/// what you hold, right is what it points at.
#[derive(Clone, Copy, PartialEq)]
enum Step {
    Left,
    Right,
}

/// Keys move you through the graph the way the graph is arranged.
///
/// The listener sits on the document: the pane is not focusable, so a handler on
/// the app root never fires when focus is on `body`. It reports back through the
/// eval channel rather than writing signals from the raw callback, because a
/// write from outside the runtime is not a write at all — it fails silently,
/// which is exactly how the first version of this looked correct and did
/// nothing.
fn use_global_keys(
    mut state: DepsState,
    resource: GraphResource,
    mut sheet_state: SheetState,
    sheet: SheetResource,
    lens: Signal<bool>,
) {
    use_future(move || async move {
        let mut channel = document::eval(
            r#"
            document.addEventListener('keydown', (event) => {
                const tag = (document.activeElement?.tagName || '').toLowerCase();
                const typing = tag === 'input' || tag === 'textarea';
                if (event.key === 'Escape') {
                    dioxus.send('clear');
                    return;
                }
                if (typing || event.metaKey || event.ctrlKey) return;
                switch (event.key) {
                    case '/':
                        event.preventDefault();
                        document.querySelector('input[type=search]')?.focus();
                        break;
                    case 'ArrowLeft':
                    case 'ArrowUp':
                        event.preventDefault();
                        dioxus.send('left');
                        break;
                    case 'ArrowRight':
                    case 'ArrowDown':
                        event.preventDefault();
                        dioxus.send('right');
                        break;
                    case 'Backspace':
                        event.preventDefault();
                        dioxus.send('back');
                        break;
                }
            });
            "#,
        );
        while let Ok(message) = channel.recv::<String>().await {
            let on_calls = lens();

            match message.as_str() {
                "clear" => {
                    if on_calls {
                        sheet_state.held.set(None);
                        sheet_state.query.set(String::new());
                    } else {
                        state.held.set(None);
                        state.query.set(String::new());
                    }
                }
                "back" => {
                    // Retrace the walk. The step you leave is dropped, so
                    // pressing back twice goes back two steps rather than
                    // oscillating between the last two.
                    let mut history = if on_calls {
                        sheet_state.history
                    } else {
                        state.history
                    };
                    let previous = {
                        let mut trail = history.write();
                        trail.pop();
                        trail.last().copied()
                    };
                    match (on_calls, previous) {
                        (true, previous) => sheet_state.held.set(previous),
                        (false, Some(id)) => {
                            let loaded = resource.read();
                            if let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() {
                                state.select(workspace.as_ref(), id);
                            }
                        }
                        (false, None) => state.held.set(None),
                    }
                }
                "left" | "right" => {
                    let step = if message == "left" {
                        Step::Left
                    } else {
                        Step::Right
                    };
                    if on_calls {
                        let next = {
                            let loaded = sheet.read();
                            let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() else {
                                continue;
                            };
                            // Along the calls, at whatever level of detail the
                            // reader has opened to — the same meaning on a crate
                            // as on a function.
                            calls::step(
                                sheet.as_ref(),
                                &(sheet_state.nest)(),
                                sheet_state.held.peek().as_ref().copied(),
                                match step {
                                    Step::Left => Way::In,
                                    Step::Right => Way::Out,
                                },
                            )
                        };
                        if let Some(next) = next {
                            let loaded = sheet.read();
                            if let Some(Some(Ok(SheetLoad::Ready(sheet)))) = loaded.as_ref() {
                                sheet_state.reveal(sheet.as_ref(), next);
                            }
                        }
                    } else {
                        let loaded = resource.read();
                        let Some(Ok(GraphLoad::Ready(workspace))) = loaded.as_ref() else {
                            continue;
                        };
                        let next = match state.held.peek().as_ref() {
                            // Nothing held yet: an arrow key starts you on a
                            // crate this workspace actually builds, which is the
                            // only place a walk through it can start.
                            None => workspace.members().next().map(|member| member.id),
                            Some(&id) => {
                                // Busiest first, so a step lands somewhere worth
                                // landing rather than on whichever crate happened
                                // to be resolved first.
                                let (dependents, dependencies) =
                                    focus::immediate(workspace.as_ref(), id);
                                match step {
                                    Step::Left => dependents.first().copied(),
                                    Step::Right => dependencies.first().copied(),
                                }
                            }
                        };
                        if let Some(next) = next {
                            state.select(workspace.as_ref(), next);
                        }
                    }
                }
                _ => {}
            }
        }
    });
}
