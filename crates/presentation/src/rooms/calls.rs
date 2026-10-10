use crate::messages::UserView;
use crate::helpers as h;
// Voice and Stage room forms and rows; controllers supply plain render data.
use super::FormRoom;

#[derive(Clone, Debug)]
pub struct CallForm {
    pub room: FormRoom,
    pub stage: bool,
    pub can_administer: bool,
    pub current_user_id: i64,
    pub selected_users: Vec<UserView>,
    pub unselected_users: Vec<UserView>,
    pub icon_name: Option<String>,
    pub icon: Option<h::AvatarIcon>,
    pub errors: Vec<String>,
    pub settings: Option<super::edit_sections::EditSections>,
}

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
impl CallForm {
    pub fn action(&self) -> String {
        match (self.stage, self.room.id) {
            (true, Some(id)) => campfire_routes::rooms_stage(id),
            (true, None) => campfire_routes::rooms_stages(),
            (false, Some(id)) => campfire_routes::rooms_voice(id),
            (false, None) => campfire_routes::rooms_voices(),
        }
    }
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
