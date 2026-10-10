

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub enum AvatarIcon {
    Emoji {
        title: String,
        character: String,
    },
    Image {
        title: String,
        url: String,
        brand: bool,
    },
}
pub trait IconSource {
    fn resolve_avatar_icon(&self, name: &str) -> Option<AvatarIcon>;
}
