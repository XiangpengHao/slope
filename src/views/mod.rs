//! Layouts and route pages.

mod chrome;
mod data;
mod dep;
mod shell;

pub(crate) use data::{DataFocus, DataModFocus, DataOverview};
pub(crate) use dep::{DepFocus, DepOverview, DepRing};
pub(crate) use shell::AppShell;
