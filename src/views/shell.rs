use dioxus::prelude::*;

use crate::Route;
use crate::api::{BoardLoad, load_board};
use crate::components::TitleBlock;
use crate::graph::focus;

/// Shared board state. Lives on the shell so the finder, the record panel, and
/// the canvas are the same state rather than three copies of it.
#[derive(Clone, Copy)]
pub struct BoardState {
    /// The crate being held.
    pub held: Signal<Option<usize>>,
    /// Crate finder text.
    pub query: Signal<String>,
    /// Reachable dependents and dependencies of whatever is held. Computed once
    /// where the record is built and shared, so the title block and the record
    /// cannot end up quoting two different numbers for the same crate.
    pub counts: Signal<Option<(usize, usize)>>,
    /// Crates held this session, oldest first. For "why is this here" the walk
    /// *is* the answer, so throwing it away on every click threw away the thing
    /// the reader came for.
    pub history: Signal<Vec<usize>>,
}

/// The loaded board, shared by everything under the shell.
pub type BoardResource = Resource<Result<BoardLoad>>;

/// The lens frame. The dependency board is the first lens; later lenses mount
/// here as siblings, which is why the workspace identity and the finder live on
/// the shell rather than inside the board.
#[component]
pub fn Shell() -> Element {
    let resource: BoardResource = use_resource(load_board);
    use_context_provider(|| resource);

    let state = BoardState {
        held: use_signal(|| None),
        query: use_signal(String::new),
        counts: use_signal(|| None),
        history: use_signal(Vec::new),
    };
    use_context_provider(|| state);

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

    use_global_keys(state, resource);

    rsx! {
        div { class: "flex h-screen flex-col overflow-hidden bg-mask text-legend",
            TitleBlock {}
            main { class: "relative min-h-0 flex-1", Outlet::<Route> {} }
        }
    }
}

/// Which way a step travels along the board's law: left is what depends on the
/// crate you hold, right is what it depends on.
#[derive(Clone, Copy, PartialEq)]
enum Step {
    Left,
    Right,
}

/// Keys move you through the board the way the board is arranged.
///
/// The listener has to sit on the document: the canvas is not focusable, so a
/// handler on the app root never fires when focus is on `body`. It reports back
/// through the eval channel rather than writing signals from the raw callback,
/// because a write from outside the runtime is not a write at all — it fails
/// silently, which is exactly how the first version of this looked correct and
/// did nothing.
fn use_global_keys(state: BoardState, resource: BoardResource) {
    let mut held = state.held;
    let mut query = state.query;
    let mut history = state.history;

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
                if (typing) return;
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
            match message.as_str() {
                "clear" => {
                    held.set(None);
                    query.set(String::new());
                }
                "back" => {
                    // Retrace the walk. The step you leave is dropped, so
                    // pressing back twice goes back two crates rather than
                    // oscillating between the last two.
                    let mut trail = history.write();
                    trail.pop();
                    let previous = trail.last().copied();
                    drop(trail);
                    held.set(previous);
                }
                "left" | "right" => {
                    let step = if message == "left" {
                        Step::Left
                    } else {
                        Step::Right
                    };
                    let next = {
                        let loaded = resource.read();
                        let Some(Ok(BoardLoad::Ready(board))) = loaded.as_ref() else {
                            continue;
                        };
                        match held.peek().as_ref() {
                            // Nothing held yet: an arrow key starts you on the
                            // crate this workspace actually builds, which is the
                            // only place a walk through it can start.
                            None => board.pads.iter().find(|p| p.is_root).map(|p| p.id),
                            Some(&id) => {
                                let (dependents, dependencies) = focus::immediate(board, id);
                                // Busiest first, so a step lands somewhere worth
                                // landing rather than on whichever crate happened
                                // to be resolved first.
                                match step {
                                    Step::Left => dependents.first().copied(),
                                    Step::Right => dependencies.first().copied(),
                                }
                            }
                        }
                    };
                    if let Some(next) = next {
                        held.set(Some(next));
                    }
                }
                _ => {}
            }
        }
    });
}
