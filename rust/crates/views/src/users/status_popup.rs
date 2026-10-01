//! #163 status fields shared by the profile and its popup. WS17 supplies effective facts.
use super::*;
#[derive(Template)]
#[template(path = "users/statuses/edit.html")]
pub struct StatusEdit {
    pub user_id: i64,
    pub fields: StatusFields,
}
impl StatusEdit {
    fn status_fields(&self) -> h::Html {
        StatusFieldsView {
            fields: self.fields.clone(),
            id_prefix: "status_popup".into(),
        }
        .html()
    }
}
#[derive(Template)]
#[template(path = "users/statuses/_fields.html")]
pub struct StatusFieldsView {
    pub fields: StatusFields,
    pub id_prefix: String,
}
impl StatusFieldsView {
    pub fn html(&self) -> h::Html {
        h::raw(self.render().unwrap())
    }
    fn error(&self, key: &str) -> Option<String> {
        self.fields
            .errors
            .get(key)
            .filter(|v| !v.is_empty())
            .map(|v| h::to_sentence(v, " and "))
    }
    fn field(&self, key: &str, html: h::Html) -> h::Html {
        if self.error(key).is_some() {
            h::content_tag("div", h::attrs().class("field_with_errors"), &html.0)
        } else {
            html
        }
    }
    fn label(&self, key: &str, text: &str, class: &str) -> h::Html {
        let form = h::form_with("/users/me/status").model("user");
        self.field(
            key,
            form.label(
                key,
                text,
                h::attrs()
                    .class(class)
                    .attr("for", format!("{}_{key}", self.id_prefix)),
            ),
        )
    }
    fn input(&self, key: &str, value: Option<&str>, options: h::Attrs) -> h::Html {
        let form = h::form_with("/users/me/status").model("user");
        self.field(
            key,
            form.text_field(key, value, options.id(format!("{}_{key}", self.id_prefix))),
        )
    }
    fn select(&self, key: &str, selected: &str, choices: &[(&str, &str)]) -> h::Html {
        let choices = choices
            .iter()
            .map(|(label, value)| {
                let options = if *value == selected {
                    h::attrs().attr("selected", "selected")
                } else {
                    h::attrs()
                };
                h::content_tag_text("option", options.value(*value), label).0
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.field(
            key,
            h::content_tag(
                "select",
                h::attrs()
                    .class("input")
                    .id(format!("{}_{key}", self.id_prefix))
                    .name(format!("user[{key}]")),
                &choices,
            ),
        )
    }
}
impl ProfileShow<'_> {
    pub(super) fn status_fields(&self) -> h::Html {
        StatusFieldsView {
            fields: self.sections.status.clone(),
            id_prefix: "user".into(),
        }
        .html()
    }
}
