//! X card rendering helpers; inputs are plain facts, and no SQL/HTTP runs here.
pub mod formatter;
pub mod cards;
pub use cards::{Card, cards};
