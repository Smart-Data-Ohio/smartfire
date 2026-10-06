//! Popup's four status attributes, via WS8a's shared User save validator.
//! WS17 integration seam: replace with UserStatusSettings/save_status from its branch.
//! Meeting/OOO policy, refresh jobs, and notification settings remain WS17/WS14-owned.
use crate::slash_commands::time_parser;
use crate::{Connection, Result, Timestamp, Tx};
use serde_json::{Value, json};
#[derive(Clone)]
pub struct StatusForm {
    pub presence: Option<String>,
    pub emoji: Option<String>,
    pub text: Option<String>,
    pub expiry: Option<Timestamp>,
    pub zone: Option<String>,
}
impl StatusForm {
    pub fn load(c: &Connection, id: i64) -> Result<Self> {
        Ok(c.query_row("SELECT presence_setting,custom_status_emoji,custom_status_text,custom_status_expires_at,time_zone FROM users WHERE id=?",[id],|r|Ok(Self {presence:r.get(0)?,emoji:r.get(1)?,text:r.get(2)?,expiry:r.get(3)?,zone:r.get(4)?}))?)
    }
    pub fn set_expiry(&mut self, preset: &str, now: Timestamp) -> Result<()> {
        if campfire_richtext::ruby::is_blank(preset) {
            return Ok(());
        }
        let zone = self
            .zone
            .as_deref()
            .and_then(time_parser::known_zone)
            .unwrap_or(jiff::tz::TimeZone::UTC);
        self.expiry = match preset {
            "minutes_30" => Some(now.since(jiff::SignedDuration::from_mins(30))),
            "hour_1" => Some(now.since(jiff::SignedDuration::from_hours(1))),
            "hours_4" => Some(now.since(jiff::SignedDuration::from_hours(4))),
            "today" => time_parser::end_of_day(now, &zone),
            "week" => {
                let date = now.jiff().to_zoned(zone.clone()).date();
                let days = 6 - i64::from(date.weekday().to_monday_zero_offset());
                date.checked_add(jiff::Span::new().days(days))
                    .ok()
                    .and_then(|date| time_parser::date_end_of_day(date, &zone))
            }
            "never" => None,
            _ => {
                let mut errors = crate::Errors::default();
                errors.add("custom_status_expires_in", "is not valid");
                return Err(crate::Error::RecordInvalid(errors));
            }
        };
        Ok(())
    }
    pub fn save(&self, tx: &mut Tx<'_>, id: i64) -> Result<()> {
        let original = Self::load(tx.conn(), id)?;
        let changed = self.presence != original.presence
            || self.emoji != original.emoji
            || self.text != original.text
            || self.expiry != original.expiry;
        crate::slash_commands::user_settings::update(
            tx,
            id,
            json!({"presence_setting":self.presence,"custom_status_emoji":self.emoji,"custom_status_text":self.text,"custom_status_expires_at":self.expiry.map(|t|t.to_db())}),
        )?;
        if changed {
            use crate::broadcasts::{Broadcast, Partial, Streamable, TurboAction, TurboStream};
            tx.emit_after_commit(crate::Event::broadcast(&Broadcast::Turbo(TurboStream {
                streamables: vec![Streamable::User(id), Streamable::Name("status".into())],
                action: TurboAction::Update,
                target: format!("status_badge_user_{id}"),
                partial: Some(Partial::UserStatus { user_id: id }),
                maintain_scroll: false,
            })));
        }
        Ok(())
    }
    pub fn attributes(&self) -> Value {
        json!({"presence_setting":self.presence,"custom_status_emoji":self.emoji,"custom_status_text":self.text,"custom_status_expires_at":self.expiry.map(|t|t.to_db())})
    }
}
