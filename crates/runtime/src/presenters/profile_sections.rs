//! Flagged read-only profile projections. Owners replace these with their domain APIs.
//! WS11: inbox/agent keys; WS13: call values; WS14g: Google configuration and connection;
//! GitHub and Fizzy connection fragments and WS17 effective status use merged owner APIs.
use campfire_db::Result;
use campfire_richtext::ruby::is_blank;
use campfire_presentation::users::*;
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
    fields.notifications.allowed_people=c.prepare("SELECT users.id,users.name FROM users JOIN dnd_allowed_users ON users.id=dnd_allowed_users.allowed_user_id WHERE dnd_allowed_users.user_id=? ORDER BY LOWER(users.name)")?.query_map([id],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
    fields.github=c.query_row("SELECT github_login,disconnected_reason,token_source FROM github_connected_accounts WHERE user_id=?",[id],|r| {
        let reason:Option<String>=r.get(1)?;Ok(if reason.as_deref().is_some_and(|s|!is_blank(s)){ConnectionPanel::Rejected{reason}}else{ConnectionPanel::Connected{name:r.get(0)?,workspace:None,app_token:r.get::<_,String>(2)?=="app"}})
    }).optional()?.unwrap_or_default();
    fields.github_verified = fields.github.connected();
    // Rails profile rendering only reads metadata; it never decrypts or refreshes tokens.
    if let Some(account) = campfire_db::models::google_account::GoogleAccount::for_user(c, id)? {
        fields.google.account_exists = true;
        fields.google.connected = account.connected();
        fields.google.calendar = account.calendar();
        fields.google.drive = account.drive();
        fields.google.email = account.email;
    }
    fields.google.identity_email = campfire_db::models::google_identity::GoogleIdentity::for_user(c, id)?
        .map(|identity| identity.email);
    let user = campfire_db::UserStatusSettings::find(c, id)?;
    fields.status = status_fields(&user, &campfire_db::Errors::default(), campfire_db::Timestamp::from_jiff(now));
    fields.status.fetch_error = c.query_row("SELECT fetch_error FROM calendar_meeting_caches WHERE user_id=?", [id], |r| r.get::<_, Option<String>>(0)).optional()?.flatten().filter(|s| !is_blank(s));
    Ok(fields)
}
pub fn status_fields(user: &campfire_db::UserStatusSettings, errors: &campfire_db::Errors, now: campfire_db::Timestamp) -> StatusFields {
    let mut fields = StatusFields {
        presence: user.presence_setting.clone(), emoji: user.custom_status_emoji.clone(), text: user.custom_status_text.clone(),
        ooo_note: user.ooo_note.clone(), manual_ooo: user.manual_ooo_active(now),
        meeting_enabled: user.meeting_status_enabled, ooo_calendar_enabled: user.ooo_calendar_enabled,
        ooo_return: user.ooo_until_effective(now).map(|t| t.jiff().to_zoned(user.zone()).strftime("%B %d, %Y").to_string()),

        ..Default::default()
    };
    for (key, message) in &errors.0 { fields.errors.entry(key.to_string()).or_default().push(message.clone()); }
    fields
}

#[cfg(test)]
mod unicode_tests {
    use super::*;

    #[test]
    fn unicode_parity_dnd_exceptions_use_sql_lower_order_past_nul() {
        let t = crate::integrations::test_support::TestDb::new();
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../vectors/unicode_casing_parity.json"
        ))
        .unwrap();
        let names = oracle["sql_names"]["input"].as_array().unwrap().clone();
        let owner = crate::integrations::test_support::TestDb::id("david");
        t.db.write_blocking(move |tx| {
            tx.conn().execute("DELETE FROM dnd_allowed_users WHERE user_id=?", [owner])?;
            for name in names {
                let user = campfire_db::User::create_integration_bot(tx, name.as_str().unwrap())?;
                tx.conn().execute("INSERT INTO dnd_allowed_users(user_id,allowed_user_id,created_at,updated_at) VALUES(?,?,?,?)", rusqlite::params![owner, user.id, tx.now(), tx.now()])?;
            }
            Ok(())
        }).unwrap();
        let actual =
            t.db.read_blocking(|c| load(c, owner, jiff::Timestamp::now()))
                .unwrap();
        assert_eq!(
            serde_json::json!(
                actual
                    .notifications
                    .allowed_people
                    .iter()
                    .map(|(_, name)| name)
                    .collect::<Vec<_>>()
            ),
            oracle["sql_names"]["sorted"]
        );
    }

}
