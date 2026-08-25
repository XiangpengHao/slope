//! Loading a graph: the server functions that carry the data model from the
//! analysis to the client.
//!
//! Each is a thin call into [`crate::analyze`] returning a [`crate::graph`]
//! value, and does no analysis and no interpretation of its own. They live
//! here rather than beside the types they return so that the data model never
//! has to depend on dioxus, and so the program's whole client/server surface
//! reads in one short file.

use dioxus::prelude::*;

use crate::graph::data::CodeGraph;
use crate::graph::dep::WorkspaceGraph;
use crate::graph::quote::ItemSource;

/// Analyze the target workspace: resolved dependency graph via
/// `cargo metadata`, diff via the detected VCS. The target is
/// `SLOPE_WORKSPACE` (falling back to the server's working directory);
/// `SLOPE_BASE` overrides the diff base revision.
#[server]
pub(crate) async fn workspace_graph() -> Result<WorkspaceGraph, ServerFnError> {
    tokio::task::spawn_blocking(crate::analyze::analyze)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .map_err(ServerFnError::new)
}

/// Survey the workspace's code structure with rust-analyzer: every workspace
/// source file, its items, and semantically resolved references. The first
/// call runs the survey (tens of seconds on a large workspace); later calls
/// answer from the cache.
#[server]
pub(crate) async fn code_graph() -> Result<CodeGraph, ServerFnError> {
    crate::analyze::code::survey_index()
        .await
        .map(|idx| idx.graph.clone())
        .map_err(ServerFnError::new)
}

/// One item's source, lexed — `item` is an index into [`CodeGraph::items`].
/// The sheet asks for it when a reviewer opens a row this chart draws no
/// block for: a function, a trait, a method. A ghost has no source to quote —
/// its definition left the working copy — and is not askable.
#[server]
pub(crate) async fn item_source(item: u32) -> Result<ItemSource, ServerFnError> {
    let idx = crate::analyze::code::survey_index()
        .await
        .map_err(ServerFnError::new)?;
    idx.item_source(item)
        .ok_or_else(|| ServerFnError::new(format!("item {item} is not in this survey")))
}
