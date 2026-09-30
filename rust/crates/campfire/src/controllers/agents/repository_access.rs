//! Flagged WS15g seam: resolve live, owner-specific private-PR access before DB readers.
use campfire_db::models::agent_payloads::RepositoryAccess;
use campfire_kit::{Ctx, Result};

/// WS15g supplies the live resolver. An empty set keeps private repository fields
/// unavailable while retaining public event payloads; it grants no private access.
pub async fn resolve(_c: &Ctx, _agent_id: i64) -> Result<RepositoryAccess> {
    Ok(RepositoryAccess::default())
}
