//! Classic sidebar cache adapter over shared membership facts.
use campfire_db::{Connection, User};
pub use campfire_runtime::presenters::accounts::*;
use campfire_views::users::{SidebarDirectItem, SidebarRoom, UserSummary};
use rails_compat::Secrets;

#[derive(Debug, Clone)]
pub struct Sidebar {
    pub favorite_memberships: Vec<campfire_views::users::SidebarItem>,
    pub categories: Vec<campfire_views::users::SidebarCategory>,
    pub direct_memberships: Vec<SidebarDirectItem>,
    pub other_memberships: Vec<SidebarRoom>,
    pub voice_memberships: Vec<SidebarRoom>,
    pub direct_placeholder_users: Vec<UserSummary>,
}
pub fn sidebar(conn: &Connection, secrets: &Secrets, user: &User) -> campfire_db::Result<Sidebar> {
    sidebar_in_zone(
        conn,
        secrets,
        user,
        &campfire_presentation::time::Zone::utc(),
    )
}
pub fn sidebar_in_zone(
    conn: &Connection,
    secrets: &Secrets,
    user: &User,
    zone: &campfire_presentation::time::Zone,
) -> campfire_db::Result<Sidebar> {
    let facts = campfire_runtime::presenters::accounts::sidebar_in_zone(conn, secrets, user, zone)?;
    Ok(Sidebar {
        favorite_memberships: facts.favorite_memberships,
        categories: facts.categories,
        direct_memberships: facts
            .direct_memberships
            .into_iter()
            .map(
                |row| match campfire_views::users::cached_direct_room_fragment(&row) {
                    Some(html) => SidebarDirectItem::Fragment(html),
                    None => row.into(),
                },
            )
            .collect(),
        other_memberships: facts.other_memberships,
        voice_memberships: facts.voice_memberships,
        direct_placeholder_users: facts.direct_placeholder_users,
    })
}
