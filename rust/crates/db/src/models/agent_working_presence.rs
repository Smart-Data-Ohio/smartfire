//! `app/services/agents/working_presence.rb`: shared by me and set_presence.
use super::agent_service::ServiceResult;
use crate::{Agent, Result, Tx};
use serde_json::json;

pub fn set(tx: &mut Tx<'_>, agent_id: i64, text: Option<&str>) -> Result<ServiceResult> {
    let mut agent =
        Agent::find(tx.conn(), agent_id)?.ok_or(crate::Error::RecordNotFound("Agent"))?;
    match agent.set_working_presence(tx, text) {
        Ok(()) => Ok(ServiceResult::ok(
            json!({"working_presence":agent.working_presence_text(tx.now())}),
            200,
        )),
        Err(crate::Error::RecordInvalid(errors)) => {
            let message = crate::slash_commands::sentence(errors.full_messages());
            let mut fields = serde_json::Map::new();
            for (field, error) in errors.0 {
                fields
                    .entry(field.to_owned())
                    .or_insert_with(|| json!([]))
                    .as_array_mut()
                    .unwrap()
                    .push(json!(error));
            }
            Ok(ServiceResult {
                payload: Some(json!({"errors":fields})),
                error: Some(message),
                status: 422,
            })
        }
        Err(error) => Err(error),
    }
}
