// Stage fragments from app/views/rooms/stage and rooms/events/venue_live_dot.
use rails_compat::unicode;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Member {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub avatar_path: String,
    pub administrator: bool,
    pub role: String,
    pub hand: Option<i64>,
    pub muted: bool,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Live {
    pub id: i64,
    pub membership_id: i64,
    pub name: String,
    pub identity: Option<String>,
}
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Stage {
    pub room_id: i64,
    pub viewer_id: i64,
    pub members: Vec<Member>,
    pub live: Option<Live>,
}
impl Stage {
    pub fn viewer(&self) -> &Member {
        self.members
            .iter()
            .find(|m| m.id == self.viewer_id)
            .expect("stage viewer is a member")
    }
    pub fn can_manage(&self) -> bool {
        self.viewer().role == "host" || self.viewer().administrator
    }
    pub fn can_moderate(&self, target: &Member) -> bool {
        target.id != self.viewer_id && (self.viewer().administrator || !target.administrator)
    }
    pub fn admin_self_unmute(&self, target: &Member) -> bool {
        target.id == self.viewer_id && self.viewer().administrator && target.muted
    }
    pub fn group(&self, role: &str) -> Vec<&Member> {
        let mut members = self
            .members
            .iter()
            .filter(|m| m.role == role)
            .collect::<Vec<_>>();
        members.sort_by_key(|m| {
            (
                role == "listener" && m.hand.is_none(),
                if role == "listener" {
                    m.hand.unwrap_or(0)
                } else {
                    0
                },
                unicode::downcase(&m.name),
            )
        });
        members
    }
}
