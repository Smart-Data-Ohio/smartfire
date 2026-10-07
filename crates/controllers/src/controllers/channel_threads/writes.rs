//! Channel and board creation, metadata, work/result changes and lifecycle writes.
use super::*;
use crate::controllers::presenters::attachments::Assignment;
use campfire_db::models::channel_thread::{WORK_UPDATE_FORBIDDEN, WorkChanges, normalize_owner_id};
use campfire_db::{Membership, NewChannelThread, NewMessage, ThreadStatus};
use campfire_kit::{Param, ParamMap, permit_keys};

const FORBIDDEN_UPDATE: &str = "WS8bm thread update forbidden";

fn source(c: &Ctx) -> Result<&ParamMap> {
    match c.params.get("thread").filter(|value| value.is_present()) {
        Some(value) => value.as_hash().ok_or_else(|| {
            Error::internal(anyhow::anyhow!("thread parameters do not support permit"))
        }),
        None => Ok(&c.params),
    }
}

fn permitted(c: &Ctx, keys: &[&str]) -> Result<ParamMap> {
    Ok(Param::Hash(source(c)?.clone()).permit(&permit_keys(keys)))
}

fn archive_minutes(value: &Param) -> Result<i64> {
    if matches!(value, Param::Bool(_)) {
        return Err(Error::internal(anyhow::anyhow!("boolean has no to_i")));
    }
    Ok(value.to_s().as_deref().and_then(cast_integer).unwrap_or(0))
}

async fn alive_room(c: &mut Ctx) -> Result<Room> {
    let (_, room) = concerns::set_room(c).await?;
    if room.deleted_at.is_some() {
        return Err(Error::NotFound);
    }
    Ok(room)
}

pub async fn new(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = alive_room(c).await?;
    if !room.board() {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    let first_message = match c.params.get("thread").filter(|value| !value.is_null()) {
        Some(value) => value
            .as_hash()
            .ok_or_else(|| Error::internal(anyhow::anyhow!("thread does not support dig")))?
            .get("first_message")
            .map(|value| campfire_richtext::ruby::json_value_to_s(&value.to_json())),
        None => None,
    };
    let viewer = require_current_user(c)?.clone();
    let mut post = messages::present(c, move |p| {
        crate::controllers::presenters::board_posts::new_post(p, &room, &viewer)
    })
    .await?;
    post.first_message = first_message;
    page::framed_page!(c, StatusCode::OK, |ctx| {
        campfire_views::channel_threads::board::New { ctx, post: &post }
    })
    .await
}

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = alive_room(c).await?;
    if room.direct() {
        return render_error(
            c,
            StatusCode::FORBIDDEN,
            "Direct rooms cannot contain channel threads",
        );
    }
    if room.board() {
        return create_board(c, room).await;
    }
    match create_channel(c, room).await {
        Err(Error::NotFound) => Ok(c.head(StatusCode::NOT_FOUND)),
        Err(error) => write_error(c, error),
        result => result,
    }
}

async fn create_board(c: &mut Ctx, room: Room) -> Result {
    let attributes = permitted(c, &["name", "work_status", "work_owner_id", "tags"])?;
    let name = attributes.get("name").and_then(messages::string_column);
    let status = attributes
        .get("work_status")
        .filter(|value| value.is_present())
        .and_then(messages::string_column)
        .unwrap_or_else(|| "planned".into());
    let raw_owner = attributes
        .get("work_owner_id")
        .map(Param::to_json)
        .unwrap_or(Value::Null);
    let tags = attributes.get("tags").map(|value| {
        value
            .to_s()
            .unwrap_or_default()
            .split(',')
            .map(str::to_string)
            .collect::<Vec<_>>()
    });
    let first_message = permitted(c, &["first_message"])?
        .get("first_message")
        .map(|value| campfire_richtext::ruby::json_value_to_s(&value.to_json()))
        .unwrap_or_default();
    let viewer = require_current_user(c)?.clone();
    let (room_id, creator_id) = (room.id, viewer.id);
    let attempted_status = attributes
        .get("work_status")
        .and_then(messages::string_column)
        .filter(|value| !campfire_richtext::ruby::is_blank(value))
        .unwrap_or_else(|| "planned".into());
    let attempted = (
        name.clone(),
        attempted_status,
        raw_owner.clone(),
        tags.clone(),
        first_message.clone(),
    );
    let result = c
        .app()
        .db
        .write(move |tx| {
            let room = Room::find(tx.conn(), room_id)?;
            if room.deleted_at.is_some() {
                return Err(campfire_db::Error::RecordNotFound("Room"));
            }
            let owner = normalize_owner_id(&raw_owner)?;
            ChannelThread::create_board_post(
                tx,
                NewChannelThread {
                    room_id,
                    creator_id,
                    name,
                    work_status: Some(status),
                    work_owner_id: owner,
                    tag_names: tags,
                    ..Default::default()
                },
                Some(first_message),
            )
        })
        .await;
    let thread = match result {
        Ok(thread) => thread,
        Err(campfire_db::Error::RecordNotFound(_)) => return Ok(c.head(StatusCode::NOT_FOUND)),
        Err(campfire_db::Error::RecordInvalid(errors)) if validation_error_is_html(c)? => {
            let mut post = messages::present(c, move |p| {
                crate::controllers::presenters::board_posts::new_post(p, &room, &viewer)
            })
            .await?;
            post.name = attempted.0;
            post.status = attempted.1;
            post.owner_id = match attempted.2 {
                Value::Null => None,
                Value::Bool(value) => Some(i64::from(value)),
                Value::Number(value) => value.as_i64().or_else(|| value.as_f64().map(|n| n as i64)),
                Value::String(value) if campfire_richtext::ruby::is_blank(&value) => None,
                Value::String(value) => Some(cast_integer(&value).unwrap_or(0)),
                _ => None,
            };
            post.tags = campfire_db::models::channel_thread::normalize_tag_names(
                &attempted.3.unwrap_or_default(),
            );
            post.first_message = Some(attempted.4);
            post.error = Some(campfire_views::helpers::to_sentence(
                &errors.full_messages(),
                " and ",
            ));
            return page::framed_page!(c, StatusCode::UNPROCESSABLE_ENTITY, |ctx| {
                campfire_views::channel_threads::board::New { ctx, post: &post }
            })
            .await;
        }
        Err(error) => return write_error(c, Error::internal(error)),
    };
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::HTML {
        return c.redirect_to(&c.url_for(&format!("/rooms/{room_id}/threads/{}", thread.id)));
    }
    let base = c.url_for("");
    let payload=messages::present(c,move |p| {
        // Rails reloads after the committed tag-assignment callback.
        let thread = ChannelThread::find(p.conn, thread.id)?;
        Ok(json!({"thread":messages::payload::thread(p,&thread,&viewer,&base)?,"parent_message":null}))
    }).await?;
    render_json(c, StatusCode::CREATED, &payload)
}

fn validation_error_is_html(c: &mut Ctx) -> Result<bool> {
    // Rails offers HTML before its format.any error fallback. Turbo accepts
    // HTML as well as streams, so rejected posts render the actual new form.
    match c.respond_to(&[&format::HTML]) {
        Ok(chosen) => Ok(*chosen == format::HTML),
        Err(Error::UnknownFormat) => Ok(false),
        Err(error) => Err(error),
    }
}

async fn create_channel(c: &mut Ctx, room: Room) -> Result {
    let attributes = permitted(c, &["name", "auto_archive_after_minutes"])?;
    let parent = source(c)?
        .get("parent_message_id")
        .filter(|value| value.is_present())
        .cloned();
    let room_id = room.id;
    let parent_id = if let Some(value) = parent {
        Some(
            c.app()
                .db
                .read(move |conn| messages::paging_anchor(conn, Timeline::Room(room_id), &value))
                .await
                .map_err(db_error)?
                .id,
        )
    } else {
        None
    };
    let raw_initial = c
        .params
        .get("message")
        .or_else(|| source(c).ok()?.get("message"))
        .filter(|value| value.is_present());
    let initial = raw_initial
        .map(|value| {
            value.permit(&permit_keys(&[
                "body",
                "attachment",
                "markdown_source",
                "client_message_id",
                "reply_to_message_id",
                "reply_notify_author",
                "forward_note",
            ]))
        })
        .unwrap_or_default();
    let creator = require_current_user(c)?.id;
    let raw_client_id = initial.get("client_message_id");
    let client_id = raw_client_id.and_then(messages::string_column);
    let lookup_id = raw_client_id
        .filter(|value| value.is_present())
        .and(client_id.clone());
    let duplicate = if let Some(client_id) = lookup_id {
        c.app()
            .db
            .read(move |conn| {
                let duplicate = Message::find_duplicate(conn, room_id, creator, &client_id)?;
                duplicate
                    .and_then(|message| message.thread_id)
                    .map(|id| ChannelThread::find(conn, id))
                    .transpose()
            })
            .await
            .map_err(db_error)?
    } else {
        None
    };
    let thread = if let Some(thread) = duplicate {
        thread
    } else {
        let mut body = initial.get("body").and_then(messages::string_column);
        let markdown = initial
            .get("markdown_source")
            .and_then(messages::string_column);
        if markdown.is_some() {
            body = None;
        }
        let body = match body {
            Some(body) => {
                Some(messages::canonicalize_body(c.app(), body, Some(c.request.host())).await?)
            }
            None => None,
        };
        let assignment = messages::attachment_assignment(&initial)?
            .unwrap_or(Assignment::Unchanged)
            .stage(c.app())
            .await?;
        if matches!(assignment, Assignment::Invalid) {
            return Err(Error::internal(anyhow::anyhow!("invalid attachment")));
        }
        let name = attributes.get("name").and_then(messages::string_column);
        let minutes = attributes
            .get("auto_archive_after_minutes")
            .map(archive_minutes)
            .transpose()?;
        let reply = initial
            .get("reply_to_message_id")
            .filter(|value| value.is_present())
            .and_then(|value| value.to_s())
            .as_deref()
            .and_then(cast_integer);
        let notify = initial.get("reply_notify_author").map(|value| {
            if value.is_null() || value.as_str() == Some("") {
                None
            } else {
                Some(!matches!(
                    value.to_s().as_deref(),
                    Some("false" | "FALSE" | "f" | "F" | "0" | "off" | "OFF")
                ))
            }
        });
        let storage = c.app().storage.clone();
        let thread = c.app()
            .db
            .write(move |tx| {
                let room = Room::find(tx.conn(), room_id)?;
                if room.deleted_at.is_some()
                    || Membership::find_by_room_and_user(tx.conn(), room_id, creator)?.is_none()
                {
                    return Err(campfire_db::Error::RecordNotFound("Membership"));
                }
                let mut thread = ChannelThread::create(
                    tx,
                    NewChannelThread {
                        room_id,
                        creator_id: creator,
                        parent_message_id: parent_id,
                        name,
                        auto_archive_after_minutes: minutes,
                        ..Default::default()
                    },
                )?;
                ThreadMembership::join(tx, thread.id, creator)?;
                if !initial.is_empty() {
                    if notify == Some(None) {
                        return Err(campfire_db::Error::Other(
                            "reply_notify_author violates NOT NULL".into(),
                        ));
                    }
                    let blob = messages::attachment_blob(tx, assignment)?;
                    let message = thread.post_message(
                        tx,
                        creator,
                        NewMessage {
                            body,
                            markdown_source: markdown,
                            client_message_id: client_id,
                            attachment_blob_id: blob.as_ref().map(|blob| blob.id),
                            reply_to_message_id: reply,
                            reply_notify_author: notify.flatten(),
                            forward_note: initial
                                .get("forward_note")
                                .and_then(messages::string_column),
                            ..Default::default()
                        },
                    )?;
                    if let Some(blob) = &blob {
                        crate::controllers::presenters::attachments::enqueue_analysis(tx, blob);
                    }
                    crate::messaging::process_message_attachment(tx, storage, &message)?;
                }
                Ok(thread)
            })
            .await
            .map_err(db_error)?;
        c.app().broadcasts.thread_created(thread.id);
        thread
    };
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::HTML {
        return c.redirect_to(&c.url_for(&format!("/rooms/{room_id}/threads/{}", thread.id)));
    }
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    let payload = messages::present(c, move |p| {
        let parent = thread.parent_message_id.map(|id| Message::find(p.conn, id)).transpose()?;
        Ok(json!({"thread": messages::payload::thread(p, &thread, &viewer, &base)?,
            "parent_message": parent.as_ref().map(|message| messages::payload::message(p, message, &viewer, &base)).transpose()?}))
    }).await?;
    render_json(c, StatusCode::CREATED, &payload)
}

pub async fn update(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    let attributes = permitted(
        c,
        &[
            "name",
            "auto_archive_after_minutes",
            "status",
            "work_status",
            "work_owner_id",
            "tags",
            "result_markdown",
        ],
    )?;
    let work = WorkChanges {
        status: attributes.get("work_status").map(|value| {
            value
                .to_s()
                .filter(|value| !campfire_richtext::ruby::is_blank(value))
        }),
        owner_id: attributes.get("work_owner_id").map(Param::to_json),
    };
    let result_markdown = attributes.get("result_markdown").cloned();
    let actor = require_current_user(c)?.clone();
    // Keep omitted, nil, and string titles distinct for Rails' failed-form rendering.
    let name = attributes.get("name").map(messages::string_column);
    let minutes = attributes
        .get("auto_archive_after_minutes")
        .map(archive_minutes)
        .transpose()?;
    let archive_given = attributes.contains_key("auto_archive_after_minutes");
    let tags = attributes.get("tags").map(|value| {
        value
            .to_s()
            .unwrap_or_default()
            .split(',')
            .map(str::to_string)
            .collect::<Vec<_>>()
    });
    let status = attributes.get("status").and_then(messages::string_column);
    let metadata_given = attributes.contains_key("name") || minutes.is_some() || tags.is_some();
    let (thread_id, room_id) = (thread.id, room.id);
    let board = room.board();
    let attempted = std::sync::Arc::new(std::sync::Mutex::new((
        thread.clone(),
        None::<Option<String>>,
        None::<Vec<String>>,
        None::<Vec<campfire_db::WorkThreadEvent>>,
    )));
    let capture = attempted.clone();
    let result = c
        .app()
        .db
        .write(move |tx| {
            let mut thread = ChannelThread::find(tx.conn(), thread_id)?;
            let before = thread.clone();
            let mut pending_name = None;
            let mut pending_tags = None;
            let outcome = (|| {
                if Membership::find_by_room_and_user(tx.conn(), room_id, actor.id)?.is_none() {
                    return Err(campfire_db::Error::RecordNotFound("Membership"));
                }
                if board && archive_given {
                    let mut errors = campfire_db::Errors::default();
                    errors.add(
                        "auto_archive_after_minutes",
                        "is not available for board posts",
                    );
                    return Err(campfire_db::Error::RecordInvalid(errors));
                }
                let settings = thread.settings_manageable_by(tx.conn(), &actor)?;
                let moderator = thread.manageable_by(tx.conn(), &actor)?;
                let work_manager = thread.work_manageable_by(tx.conn(), &actor)?;
                let assignment_manager = thread.work_assignment_manageable_by(tx.conn(), &actor)?;
                let allowed = (!metadata_given || if board { work_manager } else { settings })
                    && (result_markdown.is_none() || work_manager)
                    && ((work.status.is_none() && work.owner_id.is_none()) || work_manager)
                    && (work.owner_id.is_none() || assignment_manager)
                    && (work
                        .status
                        .as_ref()
                        .is_none_or(|status| status.is_some() == thread.work_status.is_some())
                        || assignment_manager)
                    && match status.as_deref() {
                        None => true,
                        Some("closed") => {
                            if board {
                                moderator
                            } else {
                                settings
                            }
                        }
                        Some("locked") => moderator,
                        Some("active") if thread.locked_at.is_some() => moderator,
                        Some("active")
                            if thread.status(tx.conn(), tx.now())? == ThreadStatus::Closed =>
                        {
                            thread.membership_for(tx.conn(), actor.id)?.is_some()
                        }
                        Some("active") => true,
                        _ => false,
                    };
                if !allowed {
                    return Err(campfire_db::Error::Other(FORBIDDEN_UPDATE.into()));
                }
                pending_tags = tags
                    .as_deref()
                    .map(campfire_db::models::channel_thread::normalize_tag_names);
                pending_name = name.clone();
                // The persisted model's name is non-null; nil fails its blank validation.
                thread.update_metadata(
                    tx,
                    name.as_ref().map(|name| name.as_deref().unwrap_or_default()),
                    minutes,
                    tags.as_deref(),
                )?;
                match status.as_deref() {
                    Some("closed") => thread.close(tx)?,
                    Some("locked") => thread.lock_conversation(tx)?,
                    Some("active") if thread.locked_at.is_some() => {
                        thread.unlock_conversation(tx)?
                    }
                    Some("active") => thread.reopen(tx)?,
                    _ => {}
                }
                if let Some(markdown) = result_markdown {
                    let markdown = match markdown {
                        Param::Null | Param::Bool(false) => None,
                        Param::Str(value) => Some(value),
                        _ => {
                            return Err(campfire_db::Error::Other(
                                "result_markdown does not respond to length".into(),
                            ));
                        }
                    };
                    let reload = markdown
                        .as_ref()
                        .filter(|value| !campfire_richtext::ruby::is_blank(value))
                        != thread.result_markdown.as_ref();
                    thread.update_result(tx, &actor, markdown)?;
                    if reload {
                        pending_tags = None;
                    }
                }
                if work.status.is_some() || work.owner_id.is_some() {
                    pending_tags = None;
                    thread.update_work(tx, &actor, work)?;
                }
                Ok(())
            })();
            let history = if matches!(&outcome, Err(campfire_db::Error::RecordInvalid(_))) {
                if pending_tags.is_none() {
                    pending_tags = Some(thread.tag_names(tx.conn())?);
                }
                Some(campfire_db::WorkThreadEvent::for_thread(
                    tx.conn(),
                    thread_id,
                )?)
            } else {
                None
            };
            *capture.lock().expect("thread attempt") =
                (thread.clone(), pending_name, pending_tags, history);
            outcome.map(|()| {
                // A work change already publishes `thread.updated` from the model.
                let published = thread.work_changed_from(&before);
                (thread, published)
            })
        })
        .await;
    let (attempted, pending_name, pending_tags, history) =
        attempted.lock().expect("thread attempt").clone();
    let (thread, published) = match result {
        Ok(updated) => updated,
        Err(campfire_db::Error::RecordNotFound(_)) => return forbidden_update(c, &thread),
        Err(campfire_db::Error::Other(message))
            if message == FORBIDDEN_UPDATE || message == WORK_UPDATE_FORBIDDEN =>
        {
            return forbidden_update(c, &thread);
        }
        Err(campfire_db::Error::RecordInvalid(errors)) if validation_error_is_html(c)? => {
            let records = c
                .app()
                .db
                .read(move |conn| Message::last_page(conn, Timeline::Thread(thread_id)))
                .await
                .map_err(db_error)?;
            if room.board() {
                let viewer = require_current_user(c)?.clone();
                let picker = c.app().config.google_picker.is_some();
                let mut post = messages::present(c, move |p| {
                    let mut post = crate::controllers::presenters::board_posts::post(
                        p, &room, &attempted, &viewer, &records, picker,
                    )?;
                    if let Some(history) = history {
                        post.history =
                            crate::controllers::presenters::board_posts::history_records(
                                p, history,
                            )?;
                    }
                    Ok(post)
                })
                .await?;
                if let Some(name) = pending_name {
                    post.name = name;
                }
                if let Some(tags) = pending_tags {
                    post.tags = tags;
                }
                post.error = Some(campfire_views::helpers::to_sentence(
                    &errors.full_messages(),
                    " and ",
                ));
                return page::framed_page!(c, StatusCode::UNPROCESSABLE_ENTITY, |ctx| {
                    campfire_views::channel_threads::board::Show { ctx, post: &post }
                })
                .await;
            }
            return render_standalone(c, attempted, records, StatusCode::UNPROCESSABLE_ENTITY)
                .await;
        }
        Err(error) => return write_error(c, Error::internal(error)),
    };
    if !published {
        c.app().broadcasts.thread_updated(thread_id);
    }
    if *c.respond_to(&[&format::HTML, &format::JSON])? == format::HTML {
        return c.redirect_to(&c.url_for(&format!("/rooms/{room_id}/threads/{thread_id}")));
    }
    let viewer = require_current_user(c)?.clone();
    let base = c.url_for("");
    let payload = messages::present(c, move |p| {
        let thread = if board {
            ChannelThread::find(p.conn, thread.id)?
        } else {
            thread
        };
        Ok(json!({"thread": messages::payload::thread_details(p, &thread, &viewer, &base)?}))
    })
    .await?;
    render_json(c, StatusCode::OK, &payload)
}

fn forbidden_update(c: &mut Ctx, thread: &ChannelThread) -> Result {
    if c.format()? == Some(&format::HTML) {
        c.flash()
            .set("alert", "You are not allowed to change this post.");
        c.redirect_to(&c.url_for(&format!("/rooms/{}/threads/{}", thread.room_id, thread.id)))
    } else {
        Ok(c.head(StatusCode::FORBIDDEN))
    }
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (room, thread) = scope(c).await?;
    let actor = require_current_user(c)?.clone();
    let id = thread.id;
    let removed = c
        .app()
        .db
        .write(move |tx| {
            let thread = ChannelThread::find(tx.conn(), id)?;
            if Room::find(tx.conn(), thread.room_id)?.deleted_at.is_some()
                || Membership::find_by_room_and_user(tx.conn(), thread.room_id, actor.id)?.is_none()
            {
                return Err(campfire_db::Error::RecordNotFound("Membership"));
            }
            if !thread.manageable_by(tx.conn(), &actor)? {
                return Ok(false);
            }
            thread.destroy_by(tx, Some(actor.id))?;
            Ok(true)
        })
        .await;
    match removed {
        Ok(false) => return Ok(concerns::head(StatusCode::FORBIDDEN)),
        Err(error) => return Err(db_error(error)),
        _ => c.app().broadcasts.thread_removed(id, room.id),
    }
    if c.format()? == Some(&format::HTML) {
        c.redirect_to(&c.url_for(&format!("/rooms/{}", room.id)))
    } else {
        Ok(concerns::head(StatusCode::NO_CONTENT))
    }
}

fn write_error(c: &mut Ctx, error: Error) -> Result {
    let Error::Internal(error) = error else {
        return Err(error);
    };
    let Some(error) = error.downcast_ref::<campfire_db::Error>() else {
        return Err(Error::Internal(error));
    };
    match error {
        campfire_db::Error::RecordInvalid(errors) => render_error(
            c,
            StatusCode::UNPROCESSABLE_ENTITY,
            &campfire_views::helpers::to_sentence(&errors.full_messages(), " and "),
        ),
        error if error.is_record_not_unique() => render_error(
            c,
            StatusCode::CONFLICT,
            "A thread already exists for that message",
        ),
        _ => Err(Error::internal(anyhow::anyhow!("{error}"))),
    }
}
