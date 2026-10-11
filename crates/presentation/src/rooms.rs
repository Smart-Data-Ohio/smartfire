pub mod header;
pub mod board_automations;
pub mod boards;
pub mod edit_sections;
pub mod composition;
pub mod shell;
pub mod calls;

pub use header::HeaderIdentity;
use serde::Deserialize;
use crate::helpers as h;
use crate::messages::RoomKind;

/// A persisted room.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct RoomView {
    #[serde(default = "default_involvement")]
    pub involvement: String,
    pub id: i64,
    pub kind: RoomKind,
    pub name: Option<String>,
    /// `room_display_name(room)` for `Current.user`.
    pub display_name: String,
    #[serde(default)]
    pub header: Option<HeaderIdentity>,
}

/// The room being created or edited by the open and closed room forms. `id` is `None` for a
/// new record.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct FormRoom {
    pub id: Option<i64>,
    pub name: Option<String>,
    #[serde(default)]
    pub icon_name: Option<String>,
    #[serde(default)]
    pub icon: Option<h::AvatarIcon>,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub error_attributes: Vec<String>,
    #[serde(default)]
    pub inbound_email: Option<InboundEmailView>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct InboundEmailView {
    pub id: i64,
    pub emailable: bool,
    pub enabled: bool,
    pub address: Option<String>,
}

/// `Rooms::Direct#direct_display_name`: named rooms keep their name; unnamed
/// groups preview three first names. The caller supplies ordered other members.
pub fn room_display_name(name: Option<&str>, direct: bool, other_member_names: &[String], for_user_name: Option<&str>) -> String {
    if direct {
        if let Some(name) = h::presence(name) { return name.to_owned(); }
        match other_member_names {
            [] => for_user_name.unwrap_or_default().to_owned(),
            [name] => name.clone(),
            names => {
                let firsts = names.iter().take(3).map(|name| {
                    name.split([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
                        .find(|s| !s.is_empty()).unwrap_or_default()
                }).collect::<Vec<_>>().join(", ");
                let remaining = names.len().saturating_sub(3);
                if remaining > 0 { format!("{firsts} +{remaining}") } else { firsts }
            }
        }
    } else {
        name.unwrap_or_default().to_string()
    }
}

pub fn default_involvement() -> String {
    "mentions".into()
}
