
#[derive(Clone, Debug)]
pub struct Icon {
    pub id: i64,
    pub name: String,
    pub title: String,
    pub creator_name: String,
}
#[derive(Clone, Debug, Default)]
pub struct Form {
    pub name: Option<String>,
    pub title: Option<String>,
    pub errors: Vec<String>,
    pub invalid_fields: Vec<String>,
}
