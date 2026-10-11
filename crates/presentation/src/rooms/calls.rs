use crate::helpers as h;
// Voice and Stage room forms and rows; controllers supply plain render data.

#[derive(Clone, Debug, serde::Deserialize)]
pub struct CallRow {
    pub id: i64,
    pub name: String,
    pub stage: bool,
    pub icon: Option<h::AvatarIcon>,
    pub participants: Vec<crate::huddle::Participant>,
    pub live: bool,
    pub live_name: String,
    pub unread: bool,
    pub muted: bool,
    pub membership: bool,
    pub favorited: bool,
    pub favorite_position: Option<i64>,
    pub category_id: Option<i64>,
    pub can_delete: bool,
}

impl CallRow {
    pub fn param_key(&self) -> &str {
        if self.stage {
            "rooms_stage"
        } else {
            "rooms_voice"
        }
    }
}
