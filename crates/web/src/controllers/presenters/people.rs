//! Map the read-only user presentation domain into plain view inputs.
use campfire_db::models::user::presentation;
use campfire_views::users;
pub fn person(secrets: &rails_compat::Secrets, facts: presentation::Person) -> users::Person {
    let user = super::user_summary(secrets, &facts.user);
    let mention = campfire_richtext::markdown::mention_token(&user.name);
    users::Person {
        user,
        online: facts.online,
        starred: facts.starred,
        agent: facts.agent.is_some(),
        agent_owner: facts.agent.and_then(|agent| agent.owner_name),
        presence: facts.presence,
        custom_status: facts.custom_status,
        mention,
    }
}
