//! Generic recorder contracts. Implementations belong to the source's owning domain.
use crate::{Connection, Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAuthorization {
    SourceRecipients,
    /// The caller has authorized its recipients under this transaction's writer lock.
    CallerAuthorized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityEventType(&'static str);
impl ActivityEventType {
    pub fn parse(value: &str) -> Result<Self> {
        super::EVENT_TYPES
            .iter()
            .find(|&&event| event == value)
            .copied()
            .map(Self)
            .ok_or_else(|| Error::Other(format!("Unknown activity event type: {value}")))
    }
    pub fn as_str(self) -> &'static str {
        self.0
    }
}

/// A persisted base-class identity and current recipient policy, loaded under the
/// writer lock. A source without an activity recipient hook supplies an empty roster.
/// These are owning-domain facts, never request-provided authorization assertions.
#[derive(Debug, Clone)]
pub struct ActivityRecordingFacts {
    pub source_type: &'static str,
    pub source_id: i64,
    pub creator_id: Option<i64>,
    pub thread_id: Option<i64>,
    pub recipient_ids: Vec<i64>,
}

pub trait ActivityRecordingSource {
    /// Return None for an unsaved/deleted source. Verify persistence and read current
    /// policy on this connection; cached request objects cannot preserve authority.
    fn recording_facts(&self, conn: &Connection) -> Result<Option<ActivityRecordingFacts>>;
}

/// WS11's budget-notice reader seam. Some means this notice is currently persisted;
/// the roster is the notice's current owner, or its active human administrator fallback.
pub trait AgentBudgetNoticeActivityReader {
    fn recording_recipient_ids(
        &self,
        conn: &Connection,
        notice_id: i64,
    ) -> Result<Option<Vec<i64>>>;
}
