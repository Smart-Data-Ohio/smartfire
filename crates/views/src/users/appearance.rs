//! users/profiles/_appearance.html.erb. The choices are the pinned Rails catalogue.
use super::*;

/// The time zone select's choices (label, IANA identifier), in its order, after "Not set".
pub fn profile_time_zones() -> &'static [(String, String)] {
    static CHOICES: std::sync::LazyLock<Vec<(String, String)>> =
        std::sync::LazyLock::new(|| serde_json::from_str(include_str!("profile_time_zones.json")).unwrap());
    &CHOICES
}

#[derive(Clone)]
pub struct AppearanceData {
    pub theme: String,
    pub text_size: String,
    pub zone_identifier: Option<String>,
    pub theme_errors: Vec<String>,
    pub text_size_errors: Vec<String>,
    pub time_zone_errors: Vec<String>,
    /// The old switch between the two UIs. Always `None` since the SPA became everyone's UI;
    /// the panel goes with the classic pages.
    pub next_ui: Option<NextUi>,
}

/// "Try the new Smartfire": a form posting the person's UI to `/app/ui_preference`.
#[derive(Clone, Debug)]
pub struct NextUi {
    /// They use the new UI (`ui_preference`, else `SPA_DEFAULT`).
    pub on: bool,
    /// This page, where "Switch to classic" comes back to.
    pub return_to: String,
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
        let mut options =
            vec![h::content_tag_text("option", h::attrs().value(""), "Not set (use system)").0];
        for (label, value) in profile_time_zones() {
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
    fn next_ui_panel(&self, next_ui: &NextUi) -> h::Html {
        const URL: &str = "/app/ui_preference";
        let form = h::attrs().class("button_to").data("turbo", false);
        let (button, note) = if next_ui.on {
            let switch = h::button_to_form_params(
                URL,
                h::attrs().class("btn"),
                form,
                "Switch to classic",
                &[("ui", "classic"), ("return_to", next_ui.return_to.as_str())],
            );
            let open =
                h::content_tag_text("a", h::attrs().class("btn btn--reversed").attr("href", "/app/"), "Open the new Smartfire");
            (format!("{}{}", open.0, switch.0), "You use the new Smartfire. Pages it doesn't have yet open here.")
        } else {
            let button = h::button_to_form_params(
                URL,
                h::attrs().class("btn btn--reversed"),
                form,
                "Try the new Smartfire",
                &[("ui", "next")],
            );
            (button.0, "A faster, redesigned Smartfire. You can switch back at any time.")
        };
        h::raw(format!(
            "<div class=\"flex flex-wrap align-center gap pad-block-half\" id=\"next_ui\">{button}<p class=\"txt-small margin-none\">{}</p></div>",
            h::escape(note)
        ))
    }
    fn sentence(&self, errors: &[String]) -> String {
        h::to_sentence(errors, " and ")
    }
}
