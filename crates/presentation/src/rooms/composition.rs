// Room composer, member/thread panels and poll builder from our Rails markup.
// Ordinary view inputs bind all routes, assets, labels and request form tokens.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Drive {
    None,
    Legacy,
    Picker,
}
