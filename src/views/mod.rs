//! Layouts and route pages. Each layout and route in [`crate::Route`] renders
//! one of these components.

mod home;
pub use home::Home;

mod navbar;
pub use navbar::Navbar;
