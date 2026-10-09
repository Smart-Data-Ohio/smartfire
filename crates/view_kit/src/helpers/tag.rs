//! `ActionView::Helpers::TagHelper`: attribute rendering (`tag_options`) and element builders.
//!
//! [`Attrs`] keeps insertion order, since Rails renders attributes in hash order. Build one with
//! [`attrs()`] and chain setters, mirroring the Ruby options hash:
//! `attrs().class("btn").data("turbo_frame", "_top")` is `class: "btn", data: { turbo_frame: "_top" }`.

use std::borrow::Borrow;

use super::html::{Html, Safe, escape, push_escaped};

/// `TagHelper::BOOLEAN_ATTRIBUTES`: rendered as `name="name"` when true, omitted when false.
const BOOLEAN_ATTRIBUTES: &[&str] = &[
    "allowfullscreen",
    "allowpaymentrequest",
    "async",
    "autofocus",
    "autoplay",
    "checked",
    "compact",
    "controls",
    "declare",
    "default",
    "defaultchecked",
    "defaultmuted",
    "defaultselected",
    "defer",
    "disabled",
    "enabled",
    "formnovalidate",
    "hidden",
    "indeterminate",
    "inert",
    "ismap",
    "itemscope",
    "loop",
    "multiple",
    "muted",
    "nohref",
    "nomodule",
    "noresize",
    "noshade",
    "novalidate",
    "nowrap",
    "open",
    "pauseonexit",
    "playsinline",
    "readonly",
    "required",
    "reversed",
    "scoped",
    "seamless",
    "selected",
    "sortable",
    "truespeed",
    "typemustmatch",
    "visible",
];

/// HTML void elements, which the `tag.*` builder renders without a closing tag.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "keygen", "link", "meta", "source",
    "track", "wbr",
];

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A plain string, escaped on output.
    Text(String),
    /// An html_safe string: only `"` is replaced on output.
    Safe(String),
    Bool(bool),
}

/// Any borrowed displayable value (`&str`, `&String`, `&i64`, askama's `a ~ b`) is plain text.
impl<T: std::fmt::Display + ?Sized> From<&T> for Value {
    fn from(value: &T) -> Self {
        Value::Text(value.to_string())
    }
}
impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::Text(value)
    }
}
impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Bool(value)
    }
}
impl From<Html> for Value {
    fn from(value: Html) -> Self {
        Value::Safe(value.0)
    }
}
macro_rules! value_from_number {
    ($($t:ty),*) => { $(
        impl From<$t> for Value {
            fn from(value: $t) -> Self { Value::Text(value.to_string()) }
        }
    )* };
}
value_from_number!(i32, i64, u32, u64, usize, f64);

/// An ordered options hash. A `None` value is kept so that later assignments stay in place
/// (Ruby's `options["value"] = nil` keeps the key's position) but is never rendered.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attrs(Vec<(String, Option<Value>)>);

pub fn attrs() -> Attrs {
    Attrs::default()
}

impl Attrs {
    /// Sets `name`, replacing an earlier value in place (Ruby hash assignment semantics).
    pub fn attr(mut self, name: &str, value: impl Into<Value>) -> Self {
        self.set(name, Some(value.into()));
        self
    }

    /// Sets `name` only when `value` is `Some`, like passing a possibly-nil option.
    pub fn attr_opt<V: Into<Value>>(mut self, name: &str, value: Option<V>) -> Self {
        self.set(name, value.map(Into::into));
        self
    }

    pub fn data(self, key: &str, value: impl Into<Value>) -> Self {
        let name = format!("data-{}", dasherize(key));
        self.attr(&name, value)
    }

    pub fn aria(self, key: &str, value: impl Into<Value>) -> Self {
        let name = format!("aria-{}", dasherize(key));
        self.attr(&name, value)
    }

    /// `aria: { hidden: "true" }`, the most common option in Campfire's views.
    pub fn aria_hidden(self) -> Self {
        self.aria("hidden", "true")
    }

    pub fn class(self, value: impl Into<Value>) -> Self {
        self.attr("class", value)
    }
    pub fn id(self, value: impl Into<Value>) -> Self {
        self.attr("id", value)
    }
    pub fn style(self, value: impl Into<Value>) -> Self {
        self.attr("style", value)
    }
    pub fn title(self, value: impl Into<Value>) -> Self {
        self.attr("title", value)
    }
    pub fn alt(self, value: impl Into<Value>) -> Self {
        self.attr("alt", value)
    }
    pub fn role(self, value: impl Into<Value>) -> Self {
        self.attr("role", value)
    }
    pub fn name(self, value: impl Into<Value>) -> Self {
        self.attr("name", value)
    }
    pub fn type_(self, value: impl Into<Value>) -> Self {
        self.attr("type", value)
    }
    pub fn value(self, value: impl Into<Value>) -> Self {
        self.attr("value", value)
    }
    pub fn target(self, value: impl Into<Value>) -> Self {
        self.attr("target", value)
    }
    pub fn placeholder(self, value: impl Into<Value>) -> Self {
        self.attr("placeholder", value)
    }
    pub fn autocomplete(self, value: impl Into<Value>) -> Self {
        self.attr("autocomplete", value)
    }
    pub fn accept(self, value: impl Into<Value>) -> Self {
        self.attr("accept", value)
    }
    pub fn loading(self, value: impl Into<Value>) -> Self {
        self.attr("loading", value)
    }
    pub fn tabindex(self, value: impl Into<Value>) -> Self {
        self.attr("tabindex", value)
    }
    pub fn maxlength(self, value: impl Into<Value>) -> Self {
        self.attr("maxlength", value)
    }
    pub fn rows(self, value: impl Into<Value>) -> Self {
        self.attr("rows", value)
    }
    /// `image_tag`'s `size:` option, expanded into width and height by [`super::image_tag`].
    pub fn size(self, value: impl Into<Value>) -> Self {
        self.attr("size", value)
    }
    pub fn method(self, value: impl Into<Value>) -> Self {
        self.attr("method", value)
    }

    pub fn hidden(self) -> Self {
        self.attr("hidden", true)
    }
    pub fn required(self, value: bool) -> Self {
        self.attr("required", value)
    }
    pub fn autofocus(self) -> Self {
        self.attr("autofocus", true)
    }
    pub fn readonly(self) -> Self {
        self.attr("readonly", true)
    }
    pub fn disabled(self, value: bool) -> Self {
        self.attr("disabled", value)
    }
    pub fn checked(self, value: bool) -> Self {
        self.attr("checked", value)
    }

    pub fn set(&mut self, name: &str, value: Option<Value>) {
        match self.0.iter_mut().find(|(key, _)| key == name) {
            Some((_, slot)) => *slot = value,
            None => self.0.push((name.to_string(), value)),
        }
    }

    /// `options[name] ||= value`: sets only when absent or nil.
    pub fn set_default(&mut self, name: &str, value: Option<Value>) {
        if self.get(name).is_none() {
            self.set(name, value);
        }
    }

    /// `options.fetch(name) { value }`: sets only when the key is absent (a nil stays nil).
    pub fn fetch_or_set(&mut self, name: &str, value: Option<Value>) {
        if !self.has(name) {
            self.set(name, value);
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(key, _)| key.as_str())
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .and_then(|(_, value)| value.as_ref())
    }

    pub fn get_str(&self, name: &str) -> Option<String> {
        self.get(name).map(value_to_string)
    }

    pub fn has(&self, name: &str) -> bool {
        self.0.iter().any(|(key, _)| key == name)
    }

    pub fn remove(&mut self, name: &str) -> Option<Value> {
        let index = self.0.iter().position(|(key, _)| key == name)?;
        self.0.remove(index).1
    }

    /// Appends `other`'s entries, overriding in place like `Hash#merge!`.
    pub fn merge(mut self, other: Attrs) -> Self {
        for (name, value) in other.0 {
            self.set(&name, value);
        }
        self
    }

    /// `link_to(url, **attributes, data: defaults.merge(attributes.delete(:data)))`: the default
    /// data attributes go where the caller's first `data-*` attribute is (or at the end), ahead of
    /// the caller's own data attributes, which override them.
    pub fn with_default_data(self, defaults: Attrs) -> Self {
        let position = self
            .0
            .iter()
            .position(|(key, _)| key.starts_with("data-"))
            .unwrap_or(self.0.len());
        let mut entries = self.0;
        let tail = entries.split_off(position);
        let mut before = Attrs(entries);
        let mut data = defaults;
        let mut after = attrs();
        for (key, value) in tail {
            if key.starts_with("data-") {
                data.set(&key, value)
            } else {
                after.set(&key, value)
            }
        }
        for (key, value) in data.0.into_iter().chain(after.0) {
            before.set(&key, value);
        }
        before
    }

    /// `tag_options`: the rendered attributes, each with a leading space.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (name, value) in &self.0 {
            let Some(value) = value else { continue };
            match value {
                Value::Bool(flag) if BOOLEAN_ATTRIBUTES.contains(&name.as_str()) => {
                    if *flag {
                        out.push_str(&format!(" {name}=\"{name}\""));
                    }
                }
                Value::Bool(flag) => out.push_str(&format!(" {name}=\"{flag}\"")),
                Value::Text(text) => {
                    out.push_str(&format!(" {name}=\""));
                    push_escaped(&mut out, text);
                    out.push('"');
                }
                Value::Safe(html) => {
                    out.push_str(&format!(" {name}=\"{}\"", html.replace('"', "&quot;")));
                }
            }
        }
        out
    }
}

pub fn value_to_string(value: &Value) -> String {
    match value {
        Value::Text(text) | Value::Safe(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
    }
}

/// `String#dasherize`.
pub fn dasherize(key: &str) -> String {
    key.replace('_', "-")
}

/// `content_tag(name, content, options)` with already-safe content.
pub fn content_tag(name: &str, attrs: impl Borrow<Attrs>, content: &str) -> Html {
    let pre = if name == "textarea" { "\n" } else { "" };
    Safe(format!(
        "<{name}{}>{pre}{content}</{name}>",
        attrs.borrow().render()
    ))
}

/// `content_tag` with plain-text content, escaped.
pub fn content_tag_text(name: &str, attrs: impl Borrow<Attrs>, content: &str) -> Html {
    content_tag(name, attrs, &escape(content))
}

/// `tag.name(**options)` from the tag builder: void elements have no closing tag and no slash,
/// others render empty. Underscores in the name become dashes (`tag.turbo_frame`).
pub fn builder_tag(name: &str, attrs: impl Borrow<Attrs>) -> Html {
    let name = dasherize(name);
    if VOID_ELEMENTS.contains(&name.as_str()) {
        Safe(format!("<{name}{}>", attrs.borrow().render()))
    } else {
        Safe(format!("<{name}{}></{name}>", attrs.borrow().render()))
    }
}

/// Legacy `tag(:name, options)`, used by `image_tag` and form fields:
/// always self-closing with `" />"`.
pub fn legacy_tag(name: &str, attrs: impl Borrow<Attrs>) -> Html {
    Safe(format!("<{name}{} />", attrs.borrow().render()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_attributes_in_order_with_rails_value_rules() {
        let attrs = attrs()
            .class("a&b")
            .hidden()
            .required(false)
            .data("turbo_permanent", true)
            .aria("hidden", "true")
            .tabindex(-1)
            .attr("data-action", Html::from(Safe("a->b".to_string())));
        assert_eq!(
            attrs.borrow().render(),
            r#" class="a&amp;b" hidden="hidden" data-turbo-permanent="true" aria-hidden="true" tabindex="-1" data-action="a->b""#
        );
    }

    #[test]
    fn assignment_keeps_position() {
        let mut attrs = attrs().value("x").class("c");
        attrs.set("value", Some("y".into()));
        assert_eq!(attrs.borrow().render(), r#" value="y" class="c""#);
    }
}
