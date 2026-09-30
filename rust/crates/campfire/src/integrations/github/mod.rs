//! GitHub domain and fixed-host clients, matching our Rails `app/models/github/`.
//! Controllers and HTML belong outside this module.

pub mod accounts;
pub mod client;
pub mod oauth;

fn blank(value: &str) -> bool {
    value.chars().all(char::is_whitespace)
}

#[cfg(test)]
mod tests;
