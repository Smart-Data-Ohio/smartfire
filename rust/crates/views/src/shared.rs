//! Views for `reference/app/views/shared`.

use askama::Template;
use crate::helpers::{self as h, filters};

#[derive(Template)]
#[template(path = "shared/_multi_select_bar.html")]
pub struct MultiSelectBar { pub exit_button: bool }
