//! The user view-model shared by every users/accounts/autocompletable template.

use crate::helpers as h;
pub trait UserSummaryRendering {
    fn sidebar_profile_button_attrs(&self) -> h::Attrs;
    fn title(&self) -> String;
    fn initials(&self) -> String;
    fn avatar(&self) -> h::AvatarUser;
}
impl UserSummaryRendering for UserSummary {
    fn sidebar_profile_button_attrs(&self) -> h::Attrs {
        h::attrs()
            .type_("button")
            .class("avatar profile-card-avatar")
            .aria("label", format!("View profile of {}", self.name))
            .merge(h::profile_card_trigger(self.id, false))
    }

    /// `User#title`.
    fn title(&self) -> String {
        h::user_title(&self.name, self.bio.as_deref())
    }

    /// `User#initials`.
    fn initials(&self) -> String {
        h::initials(&self.name)
    }

    fn avatar(&self) -> h::AvatarUser {
        h::AvatarUser {
            id: self.id,
            title: self.title(),
            avatar_path: self.avatar_path.clone(),
        }
    }
}

pub use campfire_presentation::users::summary::*;
