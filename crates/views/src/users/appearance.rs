//! users/profiles/_appearance.html.erb. The choices are the pinned Rails catalogue.
use super::*;

#[derive(Clone)]
pub struct AppearanceData {
    pub theme: String,
    pub text_size: String,
    pub zone_identifier: Option<String>,
    pub theme_errors: Vec<String>,
    pub text_size_errors: Vec<String>,
    pub time_zone_errors: Vec<String>,
}
#[derive(Template)]
#[template(path = "users/profiles/_appearance.html")]
pub struct Appearance<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub data: AppearanceData,
}
impl Appearance<'_> {
    fn form(&self) -> h::FormWith {
        h::form_with(h::routes::user_profile())
            .model("user")
            .method("patch")
            .data("turbo", false)
            .class("flex flex-column gap")
    }
    fn radio(&self, field: &str, value: &str, errors: &[String]) -> h::Html {
        let checked = match field {
            "theme" => self.data.theme == value,
            "text_size" => self.data.text_size == value,
            _ => false,
        };
        let mut attrs = h::attrs()
            .id(format!("user_{field}_{value}"))
            .type_("radio")
            .value(value);
        if checked {
            attrs = attrs.attr("checked", "checked");
        }
        attrs = attrs.name(format!("user[{field}]"));
        Self::field(h::legacy_tag("input", attrs), errors)
    }
    fn field(html: h::Html, errors: &[String]) -> h::Html {
        if errors.is_empty() {
            html
        } else {
            h::content_tag("div", h::attrs().class("field_with_errors"), &html.0)
        }
    }
    fn zone_label(&self) -> h::Html {
        Self::field(
            self.form()
                .label("time_zone", "Time zone", h::attrs().class("txt-medium")),
            &self.data.time_zone_errors,
        )
    }
    fn zones(&self) -> h::Html {
        static CHOICES: std::sync::LazyLock<Vec<(String, String)>> =
            std::sync::LazyLock::new(|| {
                serde_json::from_str(include_str!("profile_time_zones.json")).unwrap()
            });
        let mut options =
            vec![h::content_tag_text("option", h::attrs().value(""), "Not set (use system)").0];
        for (label, value) in CHOICES.iter() {
            let mut attrs = h::attrs();
            if self.data.zone_identifier.as_deref() == Some(value) {
                attrs = attrs.attr("selected", "selected");
            }
            options.push(h::content_tag_text("option", attrs.value(value.as_str()), label).0);
        }
        Self::field(
            h::content_tag(
                "select",
                h::attrs()
                    .class("input flex-item-grow")
                    .id("user_time_zone")
                    .name("user[time_zone]"),
                &options.join("\n"),
            ),
            &self.data.time_zone_errors,
        )
    }
    fn sentence(&self, errors: &[String]) -> String {
        h::to_sentence(errors, " and ")
    }
}
