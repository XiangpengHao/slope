//! The data model: the graph structures slope charts, and nothing whatever
//! about how they are drawn.
//!
//! One module per graph. Each holds the minimal shape the analysis produces
//! and a chart consumes — no coordinates, no colours, no fold or selection
//! state, no chart vocabulary at all. A view reads one of these and decides
//! on its own how to plot it.
//!
//! Two constraints hold across all of it, and neither is visible from any one
//! type:
//!
//! * **It crosses the wire.** The server produces these values and the wasm
//!   client deserializes them, so everything here derives `serde` and stays
//!   free of any server-only dependency. [`crate::load`] holds the server
//!   functions that carry them across.
//! * **It cannot know about drawing.** Nothing here depends on `dioxus`, and
//!   nothing here should start to. That independence is the whole reason the
//!   model is a module of its own rather than a header on the views.

pub(crate) mod data;
pub(crate) mod dep;
pub(crate) mod quote;
