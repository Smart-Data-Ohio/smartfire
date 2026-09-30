//! Root-message and user JSON from MessagePayloadHelper. Keep request-bound values out of caches.
use super::{Presenter, json_time};
use campfire_db::{ChannelThread, Message, Result};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
impl Presenter<'_> {
    pub fn user_payload(&self, id: i64) -> Result<Value> {
        let user = self.user(id)?;
        let icon: Option<String> =
            self.conn
                .query_row("SELECT icon_name FROM users WHERE id=?", [id], |r| r.get(0))?;
        let icon_url = icon.as_deref().and_then(|name| {
            let name = name
                .trim_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
                .trim_matches(':')
                .trim()
                .to_lowercase();
            match campfire_views::helpers::IconSource::resolve_avatar_icon(self, &name) {
                Some(campfire_views::helpers::AvatarIcon::Image {
                    url, brand: true, ..
                }) => campfire_assets::try_asset_path(&url).ok(),
                Some(campfire_views::helpers::AvatarIcon::Image { url, .. }) => Some(url),
                _ => None,
            }
        });
        let base = self.cache_base_url.as_deref().unwrap_or_default();
        Ok(
            json!({"id":user.id,"name":user.name,"role":user.role.name(),"avatar_url":format!("{base}{}",super::avatar_path(self.secrets,&user)),"icon_name":icon,"icon_avatar_url":icon_url}),
        )
    }
    fn payload_html(&self, message: &Message) -> Result<String> {
        if message
            .markdown_source
            .as_deref()
            .is_some_and(|s| !campfire_richtext::ruby::is_blank(s))
        {
            let body = message.body_html(self.conn)?.unwrap_or_default();
            let resolver = self.resolver();
            let ctx = resolver.render_context(self.request_host.clone());
            let icons = crate::rich_text::icons(self.conn).map_err(campfire_db::Error::Other)?;
            campfire_richtext::markdown::presentation(&body, &ctx, &icons, None)
                .map_err(|e| campfire_db::Error::Other(e.to_string()))
        } else {
            self.body_html(message)
        }
    }
    fn permalink(&self, message: &Message) -> String {
        let path = if let Some(thread) = message.thread_id {
            format!(
                "/rooms/{}?thread={thread}&message_id={}",
                message.room_id, message.id
            )
        } else {
            campfire_routes::room_at_message(message.room_id, message.id)
        };
        format!(
            "{}{path}",
            self.cache_base_url.as_deref().unwrap_or_default()
        )
    }
    pub(super) fn root_message_payload(&self, message: &Message) -> Result<Value> {
        let summary = self
            .conn
            .query_row(
                "SELECT id FROM channel_threads WHERE parent_message_id=?",
                [message.id],
                |r| r.get::<_, i64>(0),
            )
            .optional()?;
        let icon: Option<String> = self.conn.query_row(
            "SELECT icon_name FROM rooms WHERE id=?",
            [message.room_id],
            |r| r.get(0),
        )?;
        let mut body =
            json!({"plain_text":self.plain_text_body(message)?,"html":self.payload_html(message)?});
        if let Some(markdown) = &message.markdown_source {
            body["markdown_source"] = markdown.clone().into();
        }
        let drive = message
            .drive_file_ids(self.conn)?
            .into_iter()
            .map(|id| json!({"url":format!("https://drive.google.com/open?id={id}"),"file_id":id}))
            .collect::<Vec<_>>();
        let mut payload = json!({"id":message.id,"client_message_id":message.client_message_id,"created_at":json_time(message.created_at.jiff()),"updated_at":json_time(message.updated_at.jiff()),"body":body,"creator":self.user_payload(message.creator_id)?,"room":{"id":message.room_id,"icon_name":icon},"drive_attachments":drive,"url":self.permalink(message)});
        if let Some(thread) = message.thread_id {
            payload["thread_context"] =
                self.bot_thread_payload(&ChannelThread::find(self.conn, thread)?)?;
        }
        if let Some(thread) = summary {
            payload["thread_summary"] =
                self.bot_thread_payload(&ChannelThread::find(self.conn, thread)?)?;
        }
        if message.forwarded() {
            let mut forward = json!({"label":"Forwarded"});
            if let Some(note) = &message.forward_note {
                forward["note"] = note.clone().into();
            }
            payload["forwarded"] = forward;
        }
        if message.streaming {
            payload["streaming"] = true.into();
        }
        let source = message
            .reply_to_message_id
            .map(|id| Message::find_by_id(self.conn, id))
            .transpose()?
            .flatten();
        if source.is_some() || message.reply_target_deleted_at.is_some() {
            let mut reply = if let Some(source) = &source {
                json!({"id":source.id,"url":self.permalink(source),"creator":self.user_payload(source.creator_id)?,"body":{"plain_text":self.plain_text_body(source)?,"html":self.payload_html(source)?}})
            } else {
                json!({})
            };
            reply["deleted"] = source.is_none().into();
            reply["notify_author"] = message.reply_notify_author.into();
            payload["reply_to"] = reply;
        }
        Ok(payload)
    }
    fn bot_thread_payload(&self, thread: &ChannelThread) -> Result<Value> {
        let user = self.user(self.current_user_id.ok_or_else(|| {
            campfire_db::Error::Other("MessagePayloadHelper requires the requesting user".into())
        })?)?;
        if !user.is_bot() || thread.work_status.is_some() || thread.work_owner_id.is_some() {
            return Err(campfire_db::Error::Other(
                "WS8b-m/WS12 seam: MessagePayloadHelper work thread_payload".into(),
            ));
        }
        let membership = thread.membership_for(self.conn, user.id)?;
        let settings = thread.settings_manageable_by(self.conn, &user)?;
        let lifecycle = thread.manageable_by(self.conn, &user)?;
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM thread_memberships WHERE thread_id=?",
            [thread.id],
            |r| r.get(0),
        )?;
        let base = self.cache_base_url.as_deref().unwrap_or_default();
        Ok(json!({
            "id":thread.id,"name":thread.name,"status":thread.status(self.conn,campfire_db::Timestamp::from_jiff(self.now))?.name(),"room_id":thread.room_id,"parent_message_id":thread.parent_message_id,
            "last_activity_at":json_time(thread.last_activity_at.jiff()),"closed_at":thread.closed_at.map(|t|json_time(t.jiff())),"locked_at":thread.locked_at.map(|t|json_time(t.jiff())),"auto_archive_after_minutes":thread.auto_archive_after_minutes,
            "work":false,"work_status":Value::Null,"work_owner_id":Value::Null,"work_owner":Value::Null,"work_owner_active":false,"work_history":Value::Null,"work_owner_options":Value::Null,
            "joined":membership.is_some(),"unread":membership.as_ref().map(|m|m.unread()),"involvement":membership.as_ref().map(|m|m.involvement.name()),"message_count":thread.message_count(self.conn)?,"member_count":count,"creator":self.user_payload(thread.creator_id)?,
            "url":format!("{base}/rooms/{}/threads/{}",thread.room_id,thread.id),"permalink_url":format!("{base}/rooms/{}?thread={}",thread.room_id,thread.id),
            "permissions":{"can_rename":settings,"can_close":settings,"can_reopen":if thread.locked_at.is_some(){lifecycle}else{membership.is_some()},"can_lock":lifecycle,"can_unlock":lifecycle,"can_delete":lifecycle,
            // Human work controls are false for bot viewers (ChannelThread#work_viewable_by?).
            "can_convert_work":false,"can_manage_work":false,"can_update_work_status":false,"can_assign_work":false,"can_remove_work":false}
        }))
    }
}
