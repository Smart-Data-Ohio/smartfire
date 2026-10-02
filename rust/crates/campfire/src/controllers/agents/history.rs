//! Human management histories; WS11 models provide effective status and content policy.
use crate::{
    app::AppCtx,
    concerns::{self, AuthenticatedBy, Before},
    controllers::presenters::{self, page::framed_page},
};
use campfire_db::{Agent, AgentApproval, User};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_views::agents::history::{Approvals, Ledger};

pub async fn approvals(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_bot_access().allow_agent_access()).await?;
    let viewer = concerns::require_current_user(c)?.clone();
    if viewer.is_bot()
        || matches!(
            concerns::authenticated_by(c),
            AuthenticatedBy::AgentToken | AuthenticatedBy::BotKey
        )
    {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    let Ok(id) = agent_id(c) else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    let user = viewer.clone();
    let Some((agent, bot)) = c
        .app()
        .db
        .read(move |conn| {
            let Some(agent) = Agent::find(conn, id)? else {
                return Ok(None);
            };
            let Some(bot) = User::find_by_id(conn, agent.user_id)? else {
                return Ok(None);
            };
            Ok((bot.is_active()
                && user.is_active()
                && !user.is_bot()
                && (user.is_administrator() || agent.owner_id == Some(user.id)))
            .then_some((agent, bot)))
        })
        .await
        .map_err(Error::internal)?
    else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    let filter = c
        .param_str("status")
        .filter(|s| campfire_db::models::agent_approval::STATUSES.contains(s))
        .map(str::to_owned);
    let page = page(c);
    let offset = offset(page)?;
    let status = filter.clone();
    let secrets = c.app().secrets.clone();
    let (approvals,has_next) = c.app().db.write(move |tx| {
        // The effective-status predicate is the same as WS11's API list. The
        // HTML controller adds Rails' 50+1 page window before lazy expiry.
        let ids = query_ids(tx.conn(), "SELECT id FROM agent_approvals WHERE agent_id=?1 AND (?2 IS NULL OR (?2='pending' AND status='pending' AND expires_at>?3) OR (?2='expired' AND (status='expired' OR (status='pending' AND expires_at<=?3))) OR (?2 NOT IN ('pending','expired') AND status=?2)) ORDER BY id DESC LIMIT 51 OFFSET ?4", rusqlite::params![agent.id,status,tx.now(),offset])?;
        let mut approvals=Vec::new();
        for id in ids {
            let mut approval=AgentApproval::find(tx.conn(),id)?.expect("selected approval");
            approval.expire_if_due(tx)?;
            if status.as_deref().is_some_and(|s| matches!(s,"pending"|"expired") && approval.effective_status(tx.now())!=s) { continue; }
            approvals.push(presenters::agents::history::approval(tx.conn(),&secrets,&approval,&viewer,tx.now())?);
        }
        let has_next=approvals.len()>50; if has_next {approvals.pop();}
        Ok((approvals,has_next))
    }).await.map_err(Error::internal)?;
    c.respond_to(&[&format::HTML])?;
    let now = c.now();
    let (bot_id, bot_name) = (bot.id, bot.name);
    framed_page!(c, StatusCode::OK, |ctx| Approvals {
        ctx,
        agent_id: id,
        bot_id,
        bot_name: bot_name.clone(),
        approvals: approvals.clone(),
        filter: filter.clone(),
        page,
        has_next,
        now
    })
    .await
}
pub async fn ledger(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let id = agent_id(c)?;
    let viewer = concerns::require_current_user(c)?.clone();
    let agent = c
        .app()
        .db
        .read(move |conn| Agent::find(conn, id))
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)?;
    if !viewer.is_administrator() && agent.owner_id != Some(viewer.id) {
        return Ok(c.head(StatusCode::FORBIDDEN));
    }
    let filter = c
        .param_str("outcome")
        .filter(|s| ["pending", "delivered", "acknowledged", "suppressed"].contains(s))
        .map(str::to_owned);
    let status = filter.clone();
    let page = page(c);
    let offset = offset(page)?;
    let rich_text = c.app().db.env().rich_text.clone();
    let (bot,events,has_next)=c.app().db.read(move |conn| {
        let bot=User::find(conn,agent.user_id)?;
        let ids=query_ids(conn,"SELECT id FROM agent_events WHERE agent_id=? AND (? IS NULL OR outcome=?) ORDER BY id DESC LIMIT 51 OFFSET ?",rusqlite::params![id,status,status,offset])?;
        let mut events=Vec::new(); for id in ids {events.push(presenters::agents::history::event(conn,id,&agent,&viewer,&*rich_text)?);}
        let has_next=events.len()>50;if has_next{events.pop();} Ok((bot,events,has_next))
    }).await.map_err(Error::internal)?;
    c.respond_to(&[&format::HTML])?;
    let (bot_id, bot_name) = (bot.id, bot.name);
    framed_page!(c, StatusCode::OK, |ctx| Ledger {
        ctx,
        agent_id: id,
        bot_id,
        bot_name: bot_name.clone(),
        events: events.clone(),
        filter: filter.clone(),
        page,
        has_next
    })
    .await
}
fn agent_id(c: &Ctx) -> Result<i64> {
    c.param_str("id")
        .and_then(concerns::cast_integer)
        .ok_or(Error::NotFound)
}
fn page(c: &Ctx) -> i64 {
    c.param_str("page")
        .and_then(concerns::cast_integer)
        .unwrap_or(1)
        .max(1)
}
fn offset(page: i64) -> Result<i64> {
    (page - 1)
        .checked_mul(50)
        .ok_or_else(|| Error::internal(anyhow::anyhow!("page offset out of range")))
}

fn query_ids(
    conn: &campfire_db::Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> campfire_db::Result<Vec<i64>> {
    let mut statement = conn.prepare(sql)?;
    Ok(statement
        .query_map(params, |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}
