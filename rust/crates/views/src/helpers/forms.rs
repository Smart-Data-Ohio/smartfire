//! `form_with`, its `FormBuilder` fields, `button_to`, `hidden_field_tag` and `button_tag`,
//! reproducing ActionView's attribute order (see `form_helper.rb`, `form_tag_helper.rb`,
//! `tags/*.rb` and `url_helper.rb#button_to` in the reference image's Rails).
//!
//! In templates a form is a filter block, so the `<form>` tag is built after its fields have
//! rendered (that's how Rails learns about `multipart` from a `file_field`):
//!
//! ```text
//! {% let form = h::form_with(ctx, h::first_run_path()).model("user").class("center") %}
//! {% filter form_with(form) %}
//!   {{ form.text_field("name", user.name.as_deref(), h::attrs().class("input")) }}
//! {% endfilter %}
//! ```

use std::cell::Cell;
use std::rc::Rc;

use super::html::{Html, Safe, escape};
use super::request_forgery::token_tag;
use super::tag::{Attrs, Value, attrs, content_tag, legacy_tag, value_to_string};

/// The hidden `_method` field (`method_tag`).
pub fn method_tag(method: &str) -> Html {
    legacy_tag("input", attrs().type_("hidden").name("_method").value(method))
}

/// `form_with(url:, model:, method:, id:, class:, data:)`.
#[derive(Clone)]
pub struct FormWith {
    action: String,
    method: String,
    object_name: Option<String>,
    id: Option<String>,
    class: Option<String>,
    data: Attrs,
    /// `html:`, merged after the options Rails slices out (`id`, `class`, `data`).
    html: Attrs,
    multipart: Rc<Cell<bool>>,
}

pub fn form_with(url: impl std::fmt::Display) -> FormWith {
    FormWith {
        action: url.to_string(),
        method: "post".to_string(),
        object_name: None,
        id: None,
        class: None,
        data: attrs(),
        html: attrs(),
        multipart: Rc::new(Cell::new(false)),
    }
}

impl FormWith {
    /// `model:` — the param key fields are scoped under ("user", "account").
    pub fn model(mut self, param_key: &str) -> Self {
        self.object_name = Some(param_key.to_string());
        self
    }

    /// `method:`; a persisted `model:` implies "patch", so pass it for those too.
    pub fn method(mut self, method: &str) -> Self {
        self.method = method.to_string();
        self
    }

    pub fn id(mut self, id: impl std::fmt::Display) -> Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn class(mut self, class: impl std::fmt::Display) -> Self {
        self.class = Some(class.to_string());
        self
    }

    pub fn data(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.data = self.data.data(key, value);
        self
    }

    /// `html: { role: "search", aria: { label: "Search" } }`: extra attributes for the `<form>`,
    /// after `id`, `class` and `data` (`html_options_for_form_with`).
    pub fn html(mut self, html: Attrs) -> Self {
        self.html = self.html.merge(html);
        self
    }

    /// `auto_submit_form_with` (`FormsHelper`): prepends the auto-submit Stimulus controller.
    pub fn auto_submit(mut self) -> Self {
        let existing = self.data.get_str("data-controller").unwrap_or_default();
        let controller = format!("auto-submit {existing}").trim().to_string();
        self.data.set("data-controller", Some(controller.into()));
        self
    }

    /// `multipart: true`; also set by any `file_field`.
    pub fn multipart(self) -> Self {
        self.multipart.set(true);
        self
    }

    /// `form_with ... do |form|`'s builder methods, scoped to `object_name[...]`.
    fn builder(&self) -> FormBuilder {
        FormBuilder {
            object_name: self.object_name.clone().unwrap_or_default(),
            multipart: self.multipart.clone(),
        }
    }

    /// `<form ...>` plus the `_method` and `authenticity_token` hidden fields
    /// (`html_options_for_form_with` + `extra_tags_for_form`). The token is the per-form one for
    /// this action and method (see [`super::request_forgery`]).
    pub fn open(&self) -> Html {
        let mut html = attrs().attr_opt("id", self.id.as_deref()).attr_opt("class", self.class.as_deref());
        html = html.merge(self.data.clone()).merge(self.html.clone());
        if self.multipart.get() {
            html = html.attr("enctype", "multipart/form-data");
        }
        html = html.attr("action", self.action.as_str()).attr("accept-charset", "UTF-8");

        let method = self.method.to_lowercase();
        let extra = match method.as_str() {
            "get" => {
                html = html.method("get");
                String::new()
            }
            "post" | "" => {
                html = html.method("post");
                token_tag(&self.action, "post").0
            }
            other => {
                html = html.method("post");
                method_tag(other).0 + &token_tag(&self.action, other).0
            }
        };
        Safe(format!("<form{}>{extra}", html.render()))
    }

    /// The whole form around already-rendered `content` (a block-less `form_with` passes "").
    pub fn wrap(&self, content: &str) -> Html {
        Safe(format!("{}{content}</form>", self.open().0))
    }

    pub fn text_field(&self, method: &str, value: Option<&str>, options: Attrs) -> Html {
        self.builder().input_field("text", method, value, options)
    }

    pub fn email_field(&self, method: &str, value: Option<&str>, options: Attrs) -> Html {
        self.builder().input_field("email", method, value, options)
    }

    /// `form.search_field(method, options)`: pass the value as the `value` option, where the
    /// ERB passes it (`Tags::SearchField`).
    pub fn search_field(&self, method: &str, options: Attrs) -> Html {
        self.builder().input_field("search", method, None, options)
    }

    /// `form.label(method, text, options)` (`Tags::Label`): `for` defaults to the field's id.
    pub fn label(&self, method: &str, text: &str, options: Attrs) -> Html {
        let builder = self.builder();
        let mut options = options;
        options.fetch_or_set("for", Some(builder.tag_id(method).into()));
        super::tag::content_tag_text("label", &options, text)
    }

    pub fn url_field(&self, method: &str, value: Option<&str>, options: Attrs) -> Html {
        self.builder().input_field("url", method, value, options)
    }

    /// Password fields never render the model's value (`{ value: nil }.merge!(options)`).
    pub fn password_field(&self, method: &str, options: Attrs) -> Html {
        let mut merged = attrs();
        merged.set("value", None);
        self.builder().input_field("password", method, None, merged.merge(options))
    }

    pub fn hidden_field(&self, method: &str, value: Option<&str>, options: Attrs) -> Html {
        self.builder().input_field("hidden", method, value, options)
    }

    pub fn file_field(&self, method: &str, options: Attrs) -> Html {
        self.multipart.set(true);
        self.builder().input_field("file", method, None, options)
    }

    pub fn text_area(&self, method: &str, value: Option<&str>, options: Attrs) -> Html {
        self.builder().text_area(method, value, options)
    }

    /// `form.check_box(method, options, checked_value, unchecked_value)`; `current` is the
    /// model's value, compared with `checked_value`.
    pub fn check_box(&self, method: &str, options: Attrs, checked_value: &str, unchecked_value: &str, current: &str) -> Html {
        self.builder().check_box(method, options, checked_value, unchecked_value, current)
    }

    /// `form.fields_for(:settings)`: a builder for `object_name[settings]`.
    pub fn fields_for(&self, name: &str) -> FormWith {
        let mut nested = self.clone();
        nested.object_name = Some(format!("{}[{name}]", self.object_name.clone().unwrap_or_default()));
        nested
    }
}

struct FormBuilder {
    object_name: String,
    multipart: Rc<Cell<bool>>,
}

impl FormBuilder {
    /// `Tags::Base#tag_name`; a model-less `form_with` names fields after the method alone.
    fn tag_name(&self, method: &str) -> String {
        if self.object_name.is_empty() { method.to_string() } else { format!("{}[{method}]", self.object_name) }
    }

    /// `Tags::Base#tag_id`: the sanitized object name and method joined by "_".
    fn tag_id(&self, method: &str) -> String {
        if self.object_name.is_empty() {
            return method.to_string();
        }
        let object = sanitize_object_name(&self.object_name);
        format!("{object}_{method}")
    }

    fn add_default_name_and_id(&self, method: &str, options: &mut Attrs) {
        options.fetch_or_set("name", Some(self.tag_name(method).into()));
        options.fetch_or_set("id", Some(self.tag_id(method).into()));
    }

    /// `Tags::TextField#render`.
    fn input_field(&self, field_type: &str, method: &str, value: Option<&str>, mut options: Attrs) -> Html {
        if field_type == "file" {
            self.multipart.set(true);
        }
        if !options.has("size") {
            let size = options.get("maxlength").cloned();
            options.set("size", size);
        }
        options.set_default("type", Some(field_type.into()));
        if field_type != "file" {
            options.fetch_or_set("value", value.map(Into::into));
        }
        self.add_default_name_and_id(method, &mut options);
        legacy_tag("input", &options)
    }

    /// `Tags::TextArea#render`: the value is the element's content, after a newline.
    fn text_area(&self, method: &str, value: Option<&str>, mut options: Attrs) -> Html {
        self.add_default_name_and_id(method, &mut options);
        let content = match options.remove("value") {
            Some(value) => escape(&value_to_string(&value)),
            None => value.map(escape).unwrap_or_default(),
        };
        content_tag("textarea", &options, &content)
    }

    /// `Tags::CheckBox#render`: a hidden unchecked value, then the checkbox.
    fn check_box(&self, method: &str, mut options: Attrs, checked_value: &str, unchecked_value: &str, current: &str) -> Html {
        options.set("type", Some("checkbox".into()));
        options.set("value", Some(checked_value.into()));
        if current == checked_value {
            options.set("checked", Some("checked".into()));
        }
        self.add_default_name_and_id(method, &mut options);

        let mut hidden = attrs();
        for key in ["name", "disabled", "form"] {
            if options.has(key) {
                hidden.set(key, options.get(key).cloned());
            }
        }
        let hidden = hidden.type_("hidden").value(unchecked_value);
        Safe(format!("{}{}", legacy_tag("input", &hidden).0, legacy_tag("input", &options).0))
    }
}

/// `object_name.gsub(/\]\[|[^-a-zA-Z0-9:.]/, "_").delete_suffix("_")`.
fn sanitize_object_name(name: &str) -> String {
    let replaced = name.replace("][", "_");
    let sanitized: String = replaced
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | ':' | '.') { c } else { '_' })
        .collect();
    sanitized.strip_suffix('_').map(str::to_string).unwrap_or(sanitized)
}

/// `form.button(options) { ... }` / `button_tag`: `{ name: "button", type: "submit" }` merged
/// with the options.
pub fn button_tag(options: Attrs, content: &str) -> Html {
    let merged = attrs().name("button").type_("submit").merge(options);
    content_tag("button", &merged, content)
}

/// `hidden_field_tag(name, value, options)`.
pub fn hidden_field_tag(name: &str, value: Option<&str>, options: Attrs) -> Html {
    let base = attrs().type_("hidden").name(name).id(sanitize_to_id(name)).attr_opt("value", value);
    legacy_tag("input", base.merge(options))
}

/// `sanitize_to_id`: `]` removed, other non-id characters become "_".
fn sanitize_to_id(name: &str) -> String {
    name.replace(']', "")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.') { c } else { '_' })
        .collect()
}

/// `button_to(url, options) { content }`. `options` may carry `method` ("delete", "put",
/// "patch", "post" or "get"), `form_class`, and the button's own attributes.
pub fn button_to(url: &str, options: Attrs, content: &str) -> Html {
    button_to_form(url, options, attrs(), content)
}

/// `button_to(url, options.merge(form: form_options)) { content }`: `form_options` are the
/// `<form>`'s attributes, ahead of its `method` and `action`. Its `class` defaults to
/// `form_class`, else "button_to".
pub fn button_to_form(url: &str, mut options: Attrs, form_options: Attrs, content: &str) -> Html {
    let method = options.remove("method").map(|value| value_to_string(&value)).unwrap_or_else(|| "post".into());
    let form_class = options.remove("form_class").map(|value| value_to_string(&value)).unwrap_or_else(|| "button_to".into());
    let mut form = form_options;
    form.set_default("class", Some(form_class.into()));

    let method_field = if matches!(method.as_str(), "delete" | "patch" | "put") { method_tag(&method).0 } else { String::new() };
    let form_method = if method == "get" { "get" } else { "post" };

    // `request_token_tag`: the per-form token for the method the form really submits.
    let token = if form_method == "post" {
        token_tag(url, if method_field.is_empty() { "post" } else { &method }).0
    } else {
        String::new()
    };

    options.set("type", Some("submit".into()));
    let button = content_tag("button", &options, content).0;

    let form = form.method(form_method).attr("action", url);
    Safe(format!("<form{}>{method_field}{button}{token}</form>", form.render()))
}

/// `radio_button_tag(name, value, checked, options)`: `type`, `name`, `id` and `value` first,
/// then the options, then `checked`.
pub fn radio_button_tag(name: &str, value: &str, checked: bool, options: Attrs) -> Html {
    let id = format!("{}_{}", sanitize_to_id(name), sanitize_to_id(value));
    let base = attrs().type_("radio").name(name).id(id).value(value).merge(options);
    let base = if checked { base.attr("checked", "checked") } else { base };
    legacy_tag("input", base)
}

/// `datetime_local_field_tag(name, value, options)` (`text_field_tag` with type
/// "datetime-local").
pub fn datetime_local_field_tag(name: &str, value: Option<&str>, options: Attrs) -> Html {
    let base = attrs().type_("datetime-local").name(name).id(sanitize_to_id(name)).attr_opt("value", value);
    legacy_tag("input", base.merge(options))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_nested_object_names() {
        assert_eq!(sanitize_object_name("account[settings]"), "account_settings");
        assert_eq!(sanitize_object_name("user"), "user");
    }
}
