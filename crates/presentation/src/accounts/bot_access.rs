

#[derive(Clone, Debug)]
pub enum CredentialExpiry {
    Time(jiff::Timestamp),
    Extended {
        datetime: String,
        microseconds: bnum::types::I512,
    },
}

#[derive(Clone, Debug)]
pub struct Credential {
    pub id: i64,
    pub name: String,
    pub last_four: String,
    pub created_by: String,
    pub created_at: jiff::Timestamp,
    pub expires_at: Option<CredentialExpiry>,
    pub last_used_at: Option<jiff::Timestamp>,
    pub revoked: bool,
}
#[derive(Clone, Debug, Default)]
pub struct CredentialForm {
    pub name: Option<String>,
    pub expires_at: Option<String>,
    pub errors: Option<String>,
    pub error_fields: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Grant {
    pub id: i64,
    pub capability: String,
    pub room_name: String,
    pub granted_by: String,
    pub created_at: jiff::Timestamp,
    pub revoked: bool,
}
/// The capabilities the grant form offers, in its order.
pub const CAPABILITIES: [&str; 7] = [
    "read_messages",
    "post_messages",
    "react",
    "manage_threads",
    "external_action",
    "fizzy",
    "dm_anyone",
];

#[derive(Clone, Debug, Default)]
pub struct GrantForm {
    pub capability: Option<String>,
    pub room_id: Option<String>,
    pub errors: Option<String>,
    pub error_fields: Vec<String>,
}
impl CredentialExpiry {
    /// Whether the credential has expired at `now`: the row shows "Expired" instead of Revoke.
    pub fn passed(&self, now: jiff::Timestamp) -> bool {
        match self {
            CredentialExpiry::Time(at) => *at <= now,
            CredentialExpiry::Extended { microseconds, .. } => {
                *microseconds <= bnum::types::I512::from(now.as_microsecond())
            }
        }
    }
}
