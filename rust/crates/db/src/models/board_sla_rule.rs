//! app/models/board_sla_rule.rb. One validated timer per unfinished board status.
use crate::sql::{query_all, query_one};
use crate::{Connection, Errors, Result, Room, Timestamp, Tx};
use rusqlite::{Row, params};

pub const MAX_MINUTES: i64 = 43_200;
pub const RULED_STATUSES: [&str; 3] = ["planned", "in_progress", "blocked"];

#[derive(Debug, Clone, PartialEq)]
pub struct BoardSlaRule {
    pub id: i64,
    pub room_id: i64,
    pub work_status: String,
    pub nudge_after_minutes: i64,
    pub escalate_after_minutes: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}
/// Preserve the submitted integer attributes for Rails' before-type-cast numericality checks.
#[derive(Debug, Clone)]
pub struct NewBoardSlaRule {
    pub room_id: i64,
    pub work_status: Option<String>,
    pub nudge_after_minutes: Option<String>,
    pub escalate_after_minutes: Option<String>,
}
impl BoardSlaRule {
    pub(crate) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            room_id: row.get("room_id")?,
            work_status: row.get("work_status")?,
            nudge_after_minutes: row.get("nudge_after_minutes")?,
            escalate_after_minutes: row.get("escalate_after_minutes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Option<Self>> {
        query_one(
            conn,
            "SELECT * FROM board_sla_rules WHERE id=?",
            [id],
            Self::from_row,
        )
    }
    pub fn for_room(conn: &Connection, room_id: i64) -> Result<Vec<Self>> {
        query_all(
            conn,
            "SELECT * FROM board_sla_rules WHERE room_id=? ORDER BY id",
            [room_id],
            Self::from_row,
        )
    }
    pub fn validate(
        conn: &Connection,
        input: &NewBoardSlaRule,
        existing_id: Option<i64>,
    ) -> Result<Errors> {
        let mut errors = Errors::default();
        let room = Room::find_by_id(conn, input.room_id)?;
        if room.is_none() {
            errors.add("room", "must exist");
        }
        let status = input.work_status.as_deref();
        if status.is_none_or(campfire_richtext::ruby::is_blank) {
            errors.add("work_status", "can't be blank");
        }
        if status.is_none_or(|s| !super::channel_thread::WORK_STATUSES.contains(&s)) {
            errors.add("work_status", "is not included in the list");
        }
        let duplicate: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM board_sla_rules WHERE room_id=? AND work_status IS ? AND (? IS NULL OR id != ?))", params![input.room_id,status,existing_id,existing_id], |r|r.get(0))?;
        if duplicate {
            errors.add("work_status", "has already been taken");
        }
        threshold_errors(
            &mut errors,
            "nudge_after_minutes",
            input.nudge_after_minutes.as_deref(),
        );
        threshold_errors(
            &mut errors,
            "escalate_after_minutes",
            input.escalate_after_minutes.as_deref(),
        );
        if let (Some(nudge), Some(escalate)) = (
            Self::cast_threshold(input.nudge_after_minutes.as_deref()),
            Self::cast_threshold(input.escalate_after_minutes.as_deref()),
        ) && compare_integers(&escalate, &nudge) != std::cmp::Ordering::Greater
        {
            errors.add(
                "escalate_after_minutes",
                "must be after the nudge threshold",
            );
        }
        if status == Some("done") {
            errors.add("work_status", "takes no SLA timer: done posts never breach");
        }
        if room.is_some_and(|room| !room.board()) {
            errors.add("room", "must be a board");
        }
        Ok(errors)
    }
    /// Rails' integer-column cast, kept as decimal text for invalid arbitrary-size forms.
    pub fn cast_threshold(value: Option<&str>) -> Option<String> {
        let value = value?.trim_matches(numeric_whitespace);
        if campfire_richtext::ruby::is_blank(value) {
            return None;
        }
        let negative = value.starts_with('-');
        let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value).as_bytes();
        let mut digits = String::new();
        for (index, byte) in unsigned.iter().copied().enumerate() {
            if byte.is_ascii_digit() {
                digits.push(char::from(byte));
            } else if byte != b'_'
                || digits.is_empty()
                || !unsigned.get(index + 1).is_some_and(u8::is_ascii_digit)
            {
                break;
            }
        }
        let digits = digits.trim_start_matches('0');
        Some(if digits.is_empty() {
            "0".into()
        } else if negative {
            format!("-{digits}")
        } else {
            digits.into()
        })
    }
    pub fn create(tx: &mut Tx<'_>, input: NewBoardSlaRule) -> Result<Self> {
        Self::validate(tx.conn(), &input, None)?.into_result()?;
        Ok(tx.conn().query_row("INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(?,?,?,?,?,?) RETURNING *", params![input.room_id,input.work_status,cast_integer(input.nudge_after_minutes.as_deref()),cast_integer(input.escalate_after_minutes.as_deref()),tx.now(),tx.now()],Self::from_row)?)
    }
    /// Only the changed columns write; a stale instance cannot overwrite another timer.
    pub fn update(
        &mut self,
        tx: &mut Tx<'_>,
        nudge: Option<String>,
        escalate: Option<String>,
    ) -> Result<()> {
        let input = NewBoardSlaRule {
            room_id: self.room_id,
            work_status: Some(self.work_status.clone()),
            nudge_after_minutes: nudge,
            escalate_after_minutes: escalate,
        };
        Self::validate(tx.conn(), &input, Some(self.id))?.into_result()?;
        let nudge = cast_integer(input.nudge_after_minutes.as_deref()).expect("validated integer");
        let escalate =
            cast_integer(input.escalate_after_minutes.as_deref()).expect("validated integer");
        let dirty_nudge = nudge != self.nudge_after_minutes;
        let dirty_escalate = escalate != self.escalate_after_minutes;
        if dirty_nudge || dirty_escalate {
            self.updated_at = tx.now();
            match (dirty_nudge,dirty_escalate) {
                (true,true) => tx.conn().execute("UPDATE board_sla_rules SET nudge_after_minutes=?,escalate_after_minutes=?,updated_at=? WHERE id=?",params![nudge,escalate,self.updated_at,self.id])?,
                (true,false) => tx.conn().execute("UPDATE board_sla_rules SET nudge_after_minutes=?,updated_at=? WHERE id=?",params![nudge,self.updated_at,self.id])?,
                (false,true) => tx.conn().execute("UPDATE board_sla_rules SET escalate_after_minutes=?,updated_at=? WHERE id=?",params![escalate,self.updated_at,self.id])?,
                (false,false) => unreachable!(),
            };
            self.nudge_after_minutes = nudge;
            self.escalate_after_minutes = escalate;
        }
        Ok(())
    }
    pub fn destroy(&self, tx: &mut Tx<'_>) -> Result<()> {
        tx.conn()
            .execute("DELETE FROM board_sla_rules WHERE id=?", [self.id])?;
        Ok(())
    }
}
fn threshold_errors(errors: &mut Errors, field: &'static str, value: Option<&str>) {
    let Some(value) = value.filter(|s| !campfire_richtext::ruby::is_blank(s)) else {
        errors.add(field, "can't be blank");
        errors.add(field, "is not a number");
        return;
    };
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        errors.add(
            field,
            if float_number(value) {
                "must be an integer"
            } else {
                "is not a number"
            },
        );
        return;
    }
    // Huge submitted integers still get Rails' bound errors rather than a parser failure.
    if value.starts_with('-') || digits.bytes().all(|c| c == b'0') {
        errors.add(field, "must be greater than 0");
    } else if value.parse::<i64>().ok().is_none_or(|n| n > MAX_MINUTES) {
        errors.add(field, "must be less than or equal to 43200");
    }
}
fn cast_integer(value: Option<&str>) -> Option<i64> {
    Some(BoardSlaRule::cast_threshold(value)?.parse().unwrap_or(0))
}
fn compare_integers(left: &str, right: &str) -> std::cmp::Ordering {
    let negative = left.starts_with('-');
    let right_negative = right.starts_with('-');
    if negative != right_negative {
        return right_negative.cmp(&negative);
    }
    let left = left.trim_start_matches('-');
    let right = right.trim_start_matches('-');
    let order = left.len().cmp(&right.len()).then_with(|| left.cmp(right));
    if negative { order.reverse() } else { order }
}
fn numeric_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r' | ' ')
}
fn float_number(value: &str) -> bool {
    // Numericality checks a strict integer first, then Kernel.Float, excluding a raw
    // hexadecimal literal. Its float fallback accepts underscores and exponent overflow.
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    if unsigned.starts_with("0x") || unsigned.starts_with("0X") {
        return false;
    }
    static NUMBER: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
        regex::Regex::new(concat!(
            r"\A[+-]?(?:",
            r"(?:[0-9](?:_?[0-9])*(?:\.(?:[0-9](?:_?[0-9])*)?)?|\.[0-9](?:_?[0-9])*)(?:[eE][+-]?[0-9](?:_?[0-9])*)?",
            r"|0[xX](?:[0-9a-fA-F](?:_?[0-9a-fA-F])*(?:\.(?:[0-9a-fA-F](?:_?[0-9a-fA-F])*)?)?|\.[0-9a-fA-F](?:_?[0-9a-fA-F])*)(?:[pP][+-]?[0-9](?:_?[0-9])*)?",
            r")\z"
        )).expect("Ruby Float grammar")
    });
    NUMBER.is_match(value.trim_matches(numeric_whitespace))
}
