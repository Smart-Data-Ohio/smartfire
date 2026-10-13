//! The opt-in MCP authorization challenge. All other authentication bytes stay unchanged.
use crate::app::AppCtx;
use campfire_kit::{Ctx, Error, Result, StatusCode, halt};

pub fn challenge<T>(c: &Ctx, result: &Result<T>) -> bool {
    c.request.path() == "/agents/mcp"
        && c.app().config.oauth_issuer().is_some()
        && matches!(result,Err(Error::Halt(response)) if matches!(response.status,StatusCode::UNAUTHORIZED|StatusCode::FORBIDDEN|StatusCode::FOUND|StatusCode::SEE_OTHER))
}
pub fn unauthorized<T>(c: &mut Ctx) -> Result<T> {
    let issuer = c.app().config.oauth_issuer().expect("enabled challenge");
    c.no_store();
    c.set_header("www-authenticate",&format!("Bearer resource_metadata=\"{issuer}/.well-known/oauth-protected-resource/agents/mcp\", scope=\"agent\""));
    halt(c.head(StatusCode::UNAUTHORIZED))
}
