//! `Agents::FizzyReads`; WS11's REST/MCP adapters translate this domain result.
use super::{
    accounts::{Account, REJECTED_TOKEN_REASON},
    client::{Client, ErrorKind},
};
use crate::{app::App, integrations::net::Network};
use campfire_richtext::ruby::json_value_to_s;
use rails_compat::ar_encryption::ArEncryption;
use rusqlite::OptionalExtension;
use serde_json::{Value, json};

pub enum Read {
    Boards { account: Value },
    Board { account: Value, board: Value },
    Search { account: Value, query: Value },
    Card { account: Value, number: Value },
}
/// Mirrors Agents::ServiceResult without importing its HTTP or MCP adapters.
pub struct ReadResult {
    pub status: u16,
    pub payload: Option<Value>,
    pub error: Option<String>,
}
impl ReadResult {
    pub(super) fn ok(payload: Value) -> Self {
        Self {
            status: 200,
            payload: Some(payload),
            error: None,
        }
    }
    pub(super) fn fail(status: u16, message: &str) -> Self {
        Self {
            status,
            payload: None,
            error: Some(message.into()),
        }
    }
    pub fn body(&self) -> Value {
        self.payload
            .clone()
            .unwrap_or_else(|| json!({"error":self.error}))
    }
}
struct Access {
    account: Account,
    token: String,
}
async fn access(
    app: &App,
    agent: i64,
) -> campfire_db::Result<std::result::Result<Access, ReadResult>> {
    let crypto = ArEncryption::new(&app.secrets);
    app.db.write(move|tx| {
        let state=tx.conn().query_row("SELECT g.owner_id,(g.suspended_at IS NULL AND u.status=0 AND (g.owner_id IS NULL OR EXISTS(SELECT 1 FROM users owner WHERE owner.id=g.owner_id AND owner.status=0)) AND EXISTS(SELECT 1 FROM agent_grants WHERE agent_id=g.id AND capability='fizzy' AND room_id IS NULL AND revoked_at IS NULL)) FROM agents g JOIN users u ON u.id=g.user_id WHERE g.id=?",[agent],|r|Ok((r.get::<_,Option<i64>>(0)?,r.get::<_,bool>(1)?))).optional()?;
        let Some((owner,true))=state else {return Ok(Err(ReadResult::fail(403,"Forbidden: agent lacks fizzy capability")));};
        let Some(owner)=owner else {return Ok(Err(ReadResult::fail(422,"Agent has no owner recorded")));};
        let Some(account)=Account::for_user(tx.conn(),owner)? else {return Ok(Err(ReadResult::fail(422,"Agent owner has no usable Fizzy account")));};
        let Some(token)=account.usable_token(tx,&crypto)? else {return Ok(Err(ReadResult::fail(422,"Agent owner has no usable Fizzy account")));};
        Ok(Ok(Access{account,token}))
    }).await
}
fn id(value: &Value) -> std::result::Result<String, ReadResult> {
    let text = json_value_to_s(value);
    if !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        Ok(text)
    } else {
        Err(ReadResult::fail(404, "Not found in Fizzy"))
    }
}
fn account_id(input: &Value, account: &Account) -> std::result::Result<String, ReadResult> {
    if super::blank(input) {
        id(&json!(account.account_id))
    } else {
        id(input)
    }
}
pub async fn read(
    app: &App,
    network: &Network,
    base: &str,
    agent: i64,
    operation: Read,
) -> campfire_db::Result<ReadResult> {
    let input = match access(app, agent).await? {
        Ok(input) => input,
        Err(failure) => return Ok(failure),
    };
    let client = Client::new(network.clone(), input.token, base);
    let response = match operation {
        Read::Boards { account } => {
            let account = match account_id(&account, &input.account) {
                Ok(id) => id,
                Err(f) => return Ok(f),
            };
            client.boards(&account).await
        }
        Read::Board { account, board } => {
            let account = match account_id(&account, &input.account) {
                Ok(id) => id,
                Err(f) => return Ok(f),
            };
            let board = match id(&board) {
                Ok(id) => id,
                Err(f) => return Ok(f),
            };
            match client.board(&account, &board).await {
                Ok(board_payload) => client
                    .columns(&account, &board)
                    .await
                    .map(|columns| json!({"board":board_payload,"columns":columns})),
                Err(error) => Err(error),
            }
        }
        Read::Search { account, query } => {
            if super::blank(&query) {
                return Ok(ReadResult::fail(422, "Missing query"));
            }
            let account = match account_id(&account, &input.account) {
                Ok(id) => id,
                Err(f) => return Ok(f),
            };
            client.search(&account, &json_value_to_s(&query)).await
        }
        Read::Card { account, number } => {
            let number = json_value_to_s(&number);
            if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) {
                return Ok(ReadResult::fail(404, "Invalid card number"));
            }
            let account = match id(&account) {
                Ok(id) => id,
                Err(f) => return Ok(f),
            };
            let digits = number.trim_start_matches('0');
            client
                .card(&account, if digits.is_empty() { "0" } else { digits })
                .await
        }
    };
    match response {
        Ok(payload) => Ok(ReadResult::ok(payload)),
        Err(error) => Ok(match error.kind {
            ErrorKind::Unauthorized => {
                app.db
                    .write(move |tx| input.account.mark_disconnected(tx, REJECTED_TOKEN_REASON))
                    .await?;
                ReadResult::fail(422, "Agent owner's Fizzy token was rejected")
            }
            ErrorKind::NotFound | ErrorKind::Forbidden => {
                ReadResult::fail(404, "Not found in Fizzy")
            }
            ErrorKind::Refused => ReadResult::fail(422, &error.message),
            ErrorKind::Other => ReadResult::fail(502, &error.message),
        }),
    }
}
#[cfg(test)]
mod tests;
