//! WS11 service seam used by both REST and MCP after transport policy checks.
//! No mutation or successful result is fabricated for an unported service.
use super::agent_service::ServiceResult;
use crate::{Result, Tx};
use serde_json::Value;

/// Fields preserve omission/null distinctions. WS11 fills the operation's
/// domain validation, callbacks and atomic write; WS12/WS15 supply their domains.
pub fn execute(
    _tx: &mut Tx<'_>,
    _agent_id: i64,
    operation: &str,
    _fields: Value,
) -> Result<ServiceResult> {
    Err(crate::Error::Other(format!(
        "WS11 service seam: {operation}"
    )))
}
