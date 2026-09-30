//! Flagged read-only profile projections. Owners replace these with their domain APIs.
//! WS11: inbox/agent keys; WS13: call values; WS14g: Google configuration and connection;
//! WS15g/WS15e: GitHub/Fizzy usable-token checks; WS17: status/notification effective facts.
use campfire_db::Result;
use campfire_richtext::ruby::is_blank;
use campfire_views::users::*;
use rusqlite::{Connection, OptionalExtension};
pub fn load(c: &Connection, id: i64, now: jiff::Timestamp) -> Result<ProfileSections> {
    let mut fields=c.query_row("SELECT github_login,inbox_preferences,voice_mode,push_to_talk_key,presence_setting,custom_status_emoji,custom_status_text,ooo_note,dnd_enabled,dnd_until,quiet_hours_enabled,quiet_hours_start_minute,quiet_hours_end_minute,meeting_dnd_enabled,ooo_notify_enabled FROM users WHERE id=?",[id],|r| {
        let raw:Option<String>=r.get(1)?;let inbox:serde_json::Value=raw.and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default();
        let keys=[("github_review_requests","GitHub review requests","Inbox items when a pull request asks for your review."),("agent_approvals","Agent approval requests","Inbox items when an agent needs your approval. Requests stay on the approvals page."),("agent_work","Agent work assignments","Inbox items for work assigned by an agent."),("event_reminders","Event reminders","Inbox reminders before events you are attending. Push reminders still go out."),("huddle_invitations","Huddle invitations","Inbox items for incoming huddles. The incoming-call banner still shows.")];
        let clock=|v:Option<i64>|v.map(|v|format!("{:02}:{:02}",v/60,v%60));
        let enabled:bool=r.get(8)?;let until:Option<campfire_db::Timestamp>=r.get(9)?;
        let voice:Option<String>=r.get(2)?;let key:Option<String>=r.get(3)?;
        Ok(ProfileSections{github_login:r.get(0)?,inbox:keys.into_iter().map(|(key,label,description)|InboxSwitch{key:key.into(),label,description,enabled:!matches!(&inbox[key],serde_json::Value::Bool(false)) && !matches!(&inbox[key],serde_json::Value::Number(v) if v.as_i64()==Some(0)) && !matches!(&inbox[key],serde_json::Value::String(v) if v=="0"||v=="false")}).collect(),voice_mode:voice.filter(|s|s=="voice_activity"||s=="push_to_talk").unwrap_or("voice_activity".into()),push_to_talk_key:Some(key.filter(|s|!is_blank(s)).unwrap_or("`".into())),status:StatusFields{presence:r.get(4)?,emoji:r.get(5)?,text:r.get(6)?,ooo_note:r.get(7)?,..Default::default()},notifications:NotificationFields{manual_dnd:enabled&&until.is_none_or(|t|t.jiff()>now),quiet_hours:r.get(10)?,quiet_start:clock(r.get(11)?),quiet_end:clock(r.get(12)?),meeting_dnd:r.get(13)?,ooo_notify:r.get(14)?,..Default::default()},..Default::default()})
    })?;
    fields.notifications.keywords = c
        .prepare("SELECT phrase FROM keyword_alerts WHERE user_id=? ORDER BY phrase")?
        .query_map([id], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");
    fields.notifications.allowed_people=c.prepare("SELECT users.id,users.name FROM users JOIN dnd_allowed_users ON users.id=dnd_allowed_users.allowed_user_id WHERE dnd_allowed_users.user_id=? ORDER BY users.name COLLATE NOCASE")?.query_map([id],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
    fields.github=c.query_row("SELECT github_login,disconnected_reason,token_source FROM github_connected_accounts WHERE user_id=?",[id],|r| {
        let reason:Option<String>=r.get(1)?;Ok(if reason.as_deref().is_some_and(|s|!is_blank(s)){ConnectionPanel::Rejected{reason}}else{ConnectionPanel::Connected{name:r.get(0)?,workspace:None,app_token:r.get::<_,String>(2)?=="app"}})
    }).optional()?.unwrap_or_default();
    fields.github_verified = fields.github.connected();
    fields.fizzy=c.query_row("SELECT fizzy_user_name,fizzy_account_name,disconnected_reason FROM fizzy_connected_accounts WHERE user_id=?",[id],|r| {
        let reason:Option<String>=r.get(2)?;Ok(if reason.as_deref().is_some_and(|s|!is_blank(s)){ConnectionPanel::Rejected{reason}}else{ConnectionPanel::Connected{name:r.get::<_,Option<String>>(0)?.unwrap_or_default(),workspace:r.get(1)?,app_token:false}})
    }).optional()?.unwrap_or_default();
    // WS14g replaces this public metadata adapter with GoogleAccount display facts.
    // The Rails template uses connected?/scopes, never usable?/decrypted credentials.
    if let Some((email, scopes, reason)) = c
        .query_row(
            "SELECT email,scopes,disconnected_reason FROM google_accounts WHERE user_id=?",
            [id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?
    {
        let scopes = scopes.as_deref().unwrap_or("");
        fields.google.account_exists = true;
        fields.google.email = email;
        fields.google.connected = reason.as_deref().is_none_or(is_blank);
        fields.google.calendar = is_blank(scopes)
            || scopes
                .split(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{000b}' | '\u{000c}'))
                .any(|s| s == "https://www.googleapis.com/auth/calendar.events");
        fields.google.drive = scopes
            .split(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{000b}' | '\u{000c}'))
            .any(|s| s == "https://www.googleapis.com/auth/drive.file");
    }
    // WS17 replaces manual-only return dates with the shared effective Calendar/OOO reader.
    let (meeting,calendar,until,zone)=c.query_row("SELECT meeting_status_enabled,ooo_calendar_enabled,ooo_until,time_zone FROM users WHERE id=?",[id],|r|Ok((r.get::<_,bool>(0)?,r.get::<_,bool>(1)?,r.get::<_,Option<campfire_db::Timestamp>>(2)?,r.get::<_,Option<String>>(3)?)))?;
    fields.status.meeting_enabled = meeting;
    fields.status.ooo_calendar_enabled = calendar;
    if let Some(until) = until.filter(|t| t.jiff() > now) {
        fields.status.manual_ooo = true;
        fields.status.ooo_return = Some(
            campfire_views::time::Zone::for_user(zone.as_deref()).format(until.jiff(), "%B %d, %Y"),
        );
    }
    fields.status.fetch_error = c
        .query_row(
            "SELECT fetch_error FROM calendar_meeting_caches WHERE user_id=?",
            [id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
        .filter(|s| !is_blank(s));
    Ok(fields)
}
/// Submitted non-secret fields stay visible on Rails' failed-save page.
pub fn preview(
    fields: &mut ProfileSections,
    changes: &campfire_db::models::user::profile_settings::Changes,
    errors: &campfire_db::Errors,
) {
    if !fields.github_verified
        && let Some(login) = &changes.github_login
    {
        let login = campfire_richtext::ruby::strip(login).to_lowercase();
        fields.github_login = (!is_blank(&login)).then_some(login);
    }
    if let Some(mode) = &changes.voice_mode {
        fields.voice_mode = if matches!(mode.as_str(), "voice_activity" | "push_to_talk") {
            mode.clone()
        } else {
            "voice_activity".into()
        };
    }
    if let Some(key) = &changes.push_to_talk_key {
        let key = campfire_richtext::ruby::strip(key);
        fields.push_to_talk_key = Some(if is_blank(key) { "`" } else { key }.into());
    }
    if let Some(inbox) = &changes.inbox_preferences {
        for switch in &mut fields.inbox {
            if let Some(value) = inbox.get(&switch.key) {
                switch.enabled = !matches!(value, serde_json::Value::Bool(false))
                    && !matches!(value,serde_json::Value::Number(v) if v.as_i64()==Some(0))
                    && !matches!(value,serde_json::Value::String(v) if v=="0"||v=="false");
            }
        }
    }
    fields.github_errors = errors
        .on("github_login")
        .into_iter()
        .map(str::to_owned)
        .collect();
    for ((attr, _), message) in errors.0.iter().zip(errors.full_messages()) {
        if attr.starts_with("inbox_preferences")
            || (*attr == "base" && message.starts_with("Inbox preferences "))
        {
            fields.inbox_errors.push(message);
        } else if matches!(*attr, "voice_mode" | "push_to_talk_key") {
            fields.call_errors.push(message);
        }
    }
}
