//! Agent poll transport over the same Message, Poll and delivery APIs as human polls.
use super::{mcp::blank, ruby_i64, text};
use crate::{
    app::AppCtx,
    concerns,
    controllers::{message_features as features, messages, presenters::page::db_error},
};
use campfire_db::models::agent_service::ServiceResult;
use campfire_db::{Agent, Message, NewPoll, Poll};
use campfire_kit::{Ctx, Error, Param, Result};
use serde_json::Value;

pub(super) async fn operation(
    c: &Ctx,
    agent_id: i64,
    op: &str,
    args: Value,
    rest: bool,
) -> Result<ServiceResult> {
    let poll = if op == "get_poll" {
        let room_id = args.get("room_id").map_or(0, ruby_i64);
        let poll_id = args.get("poll_id").map_or(0, ruby_i64);
        c.app()
            .db
            .read_snapshot(move |conn| {
                let poll = Poll::find_in_room(conn, room_id, poll_id)?;
                let agent = Agent::find(conn, agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
                let message = Message::find_reachable(conn, agent.user_id, poll.message_id)?;
                if message.room_id != room_id { return Err(campfire_db::Error::RecordNotFound("Poll")); }
                Ok(poll)
            })
            .await
            .map_err(db_error)?
    } else {
        let raw = args.get("closes_at").filter(|value| !blank(value));
        let closes_at = if let Some(raw) = raw {
            let zone = features::user_zone(c).await?;
            // Polls uses Time.zone.parse (Date._parse), shared with Calendar forms.
            match campfire_db::slash_commands::time_parser::parse_calendar_time(
                &text(Some(raw)).unwrap_or_default(), zone.name(), zone.name(),
                campfire_db::Timestamp::from_jiff(c.now()),
            ) {
                Some(time) => Some(time),
                _ => return Ok(ServiceResult::fail("closes_at is invalid", 422)),
            }
        } else {
            None
        };
        if closes_at.is_some_and(|time| time.jiff() <= c.now()) {
            return Ok(ServiceResult::fail("Closes at must be in the future", 422));
        }
        // Ruby Array(options), then each label.to_s.strip and reject(&:blank?).
        let labels = match args.get("options") {
            Some(Value::Array(values)) => values
                .iter()
                .map(|value| text(Some(value)).unwrap_or_default())
                .collect(),
            // REST supplies ActionController::Parameters, whose Array conversion is a
            // single element. MCP supplies a Ruby Hash, whose Array conversion is pairs.
            Some(value @ Value::Object(_)) if rest => vec![text(Some(value)).unwrap()],
            Some(Value::Object(values)) => values
                .iter()
                .map(|(key, value)| text(Some(&serde_json::json!([key, value]))).unwrap())
                .collect(),
            None | Some(Value::Null) => vec![],
            Some(value) => vec![text(Some(value)).unwrap_or_default()],
        };
        let labels = Poll::normalize_labels(&labels);
        if !(campfire_db::models::poll::MIN_OPTIONS..=campfire_db::models::poll::MAX_OPTIONS)
            .contains(&labels.len())
        {
            return Ok(ServiceResult::fail(
                "Poll needs between 2 and 10 options",
                422,
            ));
        }
        if labels
            .iter()
            .any(|label| label.chars().count() > campfire_db::models::poll::LABEL_LIMIT)
        {
            return Ok(ServiceResult::fail(
                "Options are limited to 200 characters",
                422,
            ));
        }
        let question = text(args.get("question")).unwrap_or_default();
        let question = campfire_richtext::ruby::strip(&question);
        if campfire_richtext::ruby::is_blank(question) {
            return Ok(ServiceResult::fail("Question can't be blank", 422));
        }
        // Ruby boolean/numeric to_s is US-ASCII; pinned GFM rejects it at render time.
        if args
            .get("question")
            .is_some_and(|value| value.is_boolean() || value.is_number())
        {
            return Err(Error::internal(anyhow::anyhow!(
                "Markdown text is US-ASCII"
            )));
        }
        let question = question.to_owned();
        let multiple =
            features::boolean(args.get("multiple").cloned().map(Param::from_json).as_ref());
        let anonymous = features::boolean(
            args.get("anonymous")
                .cloned()
                .map(Param::from_json)
                .as_ref(),
        );
        let room_id = args.get("room_id").map_or(0, ruby_i64);
        let result = c
            .app()
            .db
            .write(move |tx| {
                let result=campfire_db::models::agent_polls::create(tx,agent_id,room_id,question,NewPoll{labels,multiple,anonymous,closes_at})?;
                let (poll,room,message)=match result {Ok(created)=>created,Err(denial)=>return Ok(Err(denial))};
                messages::deliver_webhooks_to_bots(tx, &room, &message)?;
                Ok(Ok((poll, room, message)))
            })
            .await;
        let (poll, room, message) = match result {
            Ok(Ok(result)) => result,
            Ok(Err(policy)) => return Ok(policy),
            Err(campfire_db::Error::RecordInvalid(errors)) => {
                return Ok(ServiceResult::fail(features::sentence(&errors), 422));
            }
            Err(error) => return Err(db_error(error)),
        };
        messages::broadcast_create(c, &room, &message).await?;
        messages::release_webhooks(c, &message).await;
        poll
    };
    let viewer = concerns::require_current_user(c)?.id;
    let rich_text = c.app().db.env().rich_text.clone();
    let now = campfire_db::Timestamp::from_jiff(c.now());
    let verifier = c.app().storage.verifier.clone();
    let op_is_get = op == "get_poll";
    let payload = c
        .app()
        .db
        .read_snapshot(move |conn| {
            let agent =
                Agent::find(conn, agent_id)?.ok_or(campfire_db::Error::RecordNotFound("Agent"))?;
            let message = Message::find_reachable(conn, agent.user_id, poll.message_id)?;
            if !agent.can(conn, "post_messages", Some(message.room_id))? {
                return Ok(ServiceResult::fail(
                    "Forbidden: agent lacks post_messages capability",
                    403,
                ));
            }
            Ok(ServiceResult::ok(
                poll.results_payload(conn, &*rich_text, now, Some(viewer), &*verifier)?,
                if op_is_get { 200 } else { 201 },
            ))
        })
        .await
        .map_err(db_error)?;
    Ok(payload)
}
