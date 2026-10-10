use crate::helpers as h;

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct HeaderIdentity {
    pub id: i64,
    pub param_key: String,
    pub direct: bool,
    pub kind_label: String,
    pub display_name: String,
    pub icon: Option<h::AvatarIcon>,
}
