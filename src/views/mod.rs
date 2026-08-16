mod board;
// The scene and its drawing only have a job where there is a canvas; off the
// wasm target the module compiles to a stub and its data reads as unused.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod canvas;
pub mod record;
mod shell;

pub use board::Board;
pub use shell::{BoardResource, BoardState, Shell};
