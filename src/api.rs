//! The client/server boundary. Resolution and layout both run server-side; the
//! client receives finished board geometry and never shells out or computes
//! coordinates itself.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::graph::Board;

/// Domain failures ride back as data rather than as transport errors, so the
/// board can render cargo's own diagnostic instead of a generic request failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BoardLoad {
    Ready(Box<Board>),
    Failed(String),
}

#[get("/api/board")]
pub async fn load_board() -> Result<BoardLoad> {
    match crate::graph::metadata::resolve() {
        Ok(resolved) => Ok(BoardLoad::Ready(Box::new(crate::graph::layout::build(
            resolved.packages,
            resolved.workspace,
            resolved.manifest_dir,
        )))),
        Err(message) => Ok(BoardLoad::Failed(message)),
    }
}
