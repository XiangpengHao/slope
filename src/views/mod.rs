//! Layouts and route pages.

mod chrome;
mod data;
mod dep;
mod func;
mod shell;
mod survey;

pub(crate) use data::{DataFocus, DataModFocus, DataOverview};
pub(crate) use dep::{DepFocus, DepOverview, DepRing};
pub(crate) use func::{FnBandFocus, FnFocus, FnModFocus, FnOverview, FnOwnerFocus};
pub(crate) use shell::AppShell;
