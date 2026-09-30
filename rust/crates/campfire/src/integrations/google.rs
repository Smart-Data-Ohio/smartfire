//! Google identity-only OAuth: `app/models/google/sign_in.rb` and
//! `app/controllers/concerns/google_sign_in_flow.rb`. Calendar grants are separate.
#[allow(dead_code)] // Staged domain slice: WS14g still needs the controller/session adapters.
pub mod sign_in;
