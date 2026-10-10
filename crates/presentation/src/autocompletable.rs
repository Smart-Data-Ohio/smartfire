
use serde::Serialize;
use crate::helpers as h;
use crate::users::MentionUser;

/// `autocompletable/users/_user.json.jbuilder`; `index.json.jbuilder` is a JSON array of these.
#[derive(Clone, Debug, Serialize)]
pub struct UserJson {
    /// `h(user.name)`: HTML-escaped.
    pub name: String,
    pub value: i64,
    /// `fresh_user_avatar_url(user)`: absolute.
    pub avatar_url: String,
    pub sgid: String,
}

/// `autocompletable/users/index.json.jbuilder`.
pub fn users_index_json(users: &[MentionUser], base_url: &str) -> String {
    let users: Vec<UserJson> = users
        .iter()
        .map(|user| UserJson::new(user, base_url))
        .collect();
    h::to_rails_json(&users)
}

/// Our Markdown picker JSON; the inherited Lexxy fixtures are not this endpoint's oracle.
pub fn markdown_users_index_json(
    users: &[MentionUser],
    unique: &std::collections::HashSet<String>,
    base_url: &str,
) -> String {
    #[derive(Serialize)]
    struct MarkdownUser {
        name: String,
        markdown_display_name: String,
        value: i64,
        avatar_url: String,
        sgid: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mention_token: Option<String>,
    }
    let rows = users
        .iter()
        .map(|u| MarkdownUser {
            name: h::escape(&u.name),
            markdown_display_name: u.name.clone(),
            value: u.id,
            avatar_url: format!("{base_url}{}", u.avatar_path),
            sgid: u.attachable_sgid.clone(),
            mention_token: unique
                .contains(&u.name)
                .then(|| {
                    if !u.name.contains(['[', ']', '\r', '\n'])
                        && !u.name.chars().all(char::is_whitespace)
                    {
                        Some(format!("@[{}]", u.name))
                    } else {
                        None
                    }
                })
                .flatten(),
        })
        .collect::<Vec<_>>();
    h::to_rails_json(&rows)
}
impl UserJson {
    pub fn new(user: &MentionUser, base_url: &str) -> Self {
        UserJson {
            name: h::escape(&user.name),
            value: user.id,
            avatar_url: format!("{base_url}{}", user.avatar_path),
            sgid: user.attachable_sgid.clone(),
        }
    }
}
