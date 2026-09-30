//! GitHub approval request policy over WS11's shared credential, grant, budget and approval models.
use super::{
    accounts::Account, actions::Action, client::PullRequestKey, pull_requests::PullRequest,
    threads::PullRequestThread,
};
use campfire_db::models::{
    agent_access,
    agent_posting::{self, Cap},
};
use campfire_db::{AgentApproval, Connection, NewApproval, Result, Room, Tx};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};

pub struct Scope {
    pub agent_id: i64,
    pub owner_id: Option<i64>,
    pub user_id: i64,
    pub pull_request: PullRequest,
}
pub struct Reply {
    pub status: u16,
    pub body: Option<Value>,
}
impl Reply {
    fn error(status: u16, error: &str) -> Self {
        Self {
            status,
            body: Some(json!({"error":error})),
        }
    }
    fn missing() -> Self {
        Self {
            status: 404,
            body: None,
        }
    }
}
/// Membership precedes capability checks, keeping unknown and cross-room objects indistinguishable.
pub fn scope(
    conn: &Connection,
    user_id: i64,
    room_id: i64,
    pr_id: Option<i64>,
) -> Result<std::result::Result<Scope, Reply>> {
    let identity = conn
        .query_row(
            "SELECT id,owner_id FROM agents WHERE user_id=? AND suspended_at IS NULL",
            [user_id],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?)),
        )
        .optional()?;
    let Some((agent_id, owner_id)) = identity else {
        return Ok(Err(Reply::error(
            403,
            "Forbidden: Bearer agent token required",
        )));
    };
    if Room::find_for_user(conn, user_id, room_id)?.is_none() {
        return Ok(Err(Reply::missing()));
    }
    let Some(pr_id) = pr_id else {
        return Ok(Err(Reply::missing()));
    };
    let pr = match PullRequest::find(conn, pr_id) {
        Ok(pr) => pr,
        Err(campfire_db::Error::RecordNotFound(_)) => return Ok(Err(Reply::missing())),
        Err(e) => return Err(e),
    };
    if PullRequestThread::for_room_pr(conn, room_id, pr_id)?.is_none() {
        return Ok(Err(Reply::missing()));
    }
    if !agent_access::capability_for_agent(conn, agent_id, "external_action", Some(room_id))? {
        return Ok(Err(Reply::error(
            403,
            "Forbidden: agent lacks external_action capability",
        )));
    }
    Ok(Ok(Scope {
        agent_id,
        owner_id,
        user_id,
        pull_request: pr,
    }))
}
/// All replay, budget and approval/inbox writes run in one transaction. The writer serializes
/// duplicate external IDs; replay intentionally precedes action validation and budgets.
pub struct Request {
    pub user_id: i64,
    pub room_id: i64,
    pub pr_id: Option<i64>,
    pub secret: String,
    pub account: Account,
    pub submitted: Value,
}
pub fn create(tx: &mut Tx<'_>, request: Request) -> Result<Reply> {
    let Request {
        user_id,
        room_id,
        pr_id,
        secret,
        account,
        submitted,
    } = request;
    let Some(identity) = agent_access::verify_identity(tx.conn(), &secret, tx.now())? else {
        return Ok(Reply {
            status: 401,
            body: None,
        });
    };
    if identity.user.id != user_id {
        return Ok(Reply::error(403, "Forbidden: Bearer agent token required"));
    }
    let scope = match scope(tx.conn(), user_id, room_id, pr_id)? {
        Ok(scope) => scope,
        Err(reply) => return Ok(reply),
    };
    // Don't retain an authorization snapshot across an App-token refresh or a relink.
    let current = Account::find(tx.conn(), account.id)?;
    if current.as_ref().is_none_or(|a| {
        !a.connected()
            || a.github_login != account.github_login
            || (a.user_id != user_id && Some(a.user_id) != scope.owner_id)
    }) {
        return Ok(Reply::error(422, "Agent has no usable GitHub account"));
    }
    let external = submitted
        .get("external_id")
        .filter(|v| !v.is_null() && **v != Value::Bool(false))
        .map(super::client::ruby_string)
        .filter(|s| !super::blank(s));
    if let Some(external) = &external
        && let Some(mut existing) =
            AgentApproval::find_by_external_id(tx.conn(), scope.agent_id, external)?
    {
        existing.expire_if_due(tx)?;
        return Ok(Reply {
            status: 200,
            body: Some(existing.created_payload(tx.now())),
        });
    }
    let pr = scope.pull_request;
    let mut action = Action::from_payload(
        pr.id,
        PullRequestKey {
            owner: pr.owner,
            repo: pr.repo,
            number: pr.number,
        },
        &submitted,
    );
    action.kind = Value::String(super::client::ruby_string(&action.kind));
    let errors = action.errors();
    if !errors.is_empty() {
        let mut fields = serde_json::Map::new();
        for (field, message) in errors.0 {
            fields
                .entry(field.to_owned())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .expect("array")
                .push(message.into());
        }
        return Ok(Reply {
            status: 422,
            body: Some(json!({"errors":fields})),
        });
    }
    if let Some(denial) = agent_posting::check_budget(tx, scope.agent_id, Cap::ExternalActions)? {
        return Ok(Reply {
            status: 429,
            body: Some(denial),
        });
    }
    let attributes = NewApproval {
        agent_id: scope.agent_id,
        agent_credential_id: Some(identity.credential_id),
        room_id: Some(room_id),
        action: action.action_name(),
        summary: action.summary().expect("valid action"),
        payload: Some(action.payload_json()),
        external_id: external,
        github_account_id: Some(account.id),
        github_login: Some(account.github_login.clone()),
        ..Default::default()
    };
    let errors = AgentApproval::validate(tx.conn(), &attributes, tx.now(), None)?;
    if !errors.is_empty() {
        return Ok(Reply::error(422, &sentence(errors.full_messages())));
    }
    let approval = AgentApproval::create(tx, attributes)?;
    Ok(Reply {
        status: 202,
        body: Some(approval.created_payload(tx.now())),
    })
}

fn sentence(messages: Vec<String>) -> String {
    match messages.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} and {b}"),
        many => format!(
            "{}, and {}",
            many[..many.len() - 1].join(", "),
            many.last().expect("nonempty")
        ),
    }
}
