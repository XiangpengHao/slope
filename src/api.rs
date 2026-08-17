//! The client/server boundary. Resolution runs server-side; the client receives
//! a graph and works out for itself which part of it to draw.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::call::Sheet;
use crate::graph::Workspace;

/// Domain failures ride back as data rather than as transport errors, so the
/// view can render cargo's own diagnostic instead of a generic request failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GraphLoad {
    Ready(Box<Workspace>),
    Failed(String),
}

#[get("/api/graph")]
pub async fn load_graph() -> Result<GraphLoad> {
    match crate::graph::metadata::resolve() {
        Ok(resolved) => Ok(GraphLoad::Ready(Box::new(crate::graph::build::build(
            resolved.packages,
            resolved.workspace,
            resolved.manifest_dir,
        )))),
        Err(message) => Ok(GraphLoad::Failed(message)),
    }
}

/// The call sheet, same contract: failures ride back as data so the lens can
/// show rust-analyzer's own diagnostic rather than a generic request failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SheetLoad {
    Ready(Box<Sheet>),
    Failed(String),
}

#[get("/api/calls")]
pub async fn load_sheet() -> Result<SheetLoad> {
    // Extraction costs an analyser process and ten seconds of indexing, and the
    // answer does not change while the server is up. Paying that once per
    // process rather than once per navigation is the difference between a lens
    // you can leave and come back to and one you cannot.
    Ok(sheet_once())
}

#[cfg(not(target_arch = "wasm32"))]
fn sheet_once() -> SheetLoad {
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<Option<SheetLoad>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    // Held across the build on purpose: two tabs asking at once should wait for
    // one extraction rather than start a second analyser.
    let mut slot = match cache.lock() {
        Ok(slot) => slot,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(ready) = slot.as_ref() {
        return ready.clone();
    }
    let built = match crate::call::extract::build() {
        Ok(sheet) => SheetLoad::Ready(Box::new(sheet)),
        Err(message) => SheetLoad::Failed(message),
    };
    *slot = Some(built.clone());
    built
}
