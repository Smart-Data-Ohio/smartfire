//! `fizzy/cards/{card,chip}` and `rooms/fizzy/cards/show`: view facts, no SQL/client state.
use crate::{helpers as h, time::Zone};
use serde_json::Value;

pub struct Frame<'a> {
    pub account: &'a str,
    pub number: i64,
    pub web_url: &'a str,
    pub id: &'a str,
    pub connect: bool,
    pub payload: Option<&'a Value>,
    pub error: Option<&'a str>,
    pub zone: &'a Zone,
}
fn blank(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => h::is_blank(s),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}
fn truthy(value: &Value) -> bool {
    !matches!(value, Value::Null | Value::Bool(false))
}
fn text(value: &Value) -> String {
    campfire_richtext::ruby::json_value_to_s(value)
}
/// Time.zone.parse accepts civil dates/times in the viewer's zone as well as offset timestamps.
fn last_active_at(zone: &Zone, value: &Value) -> Option<jiff::Timestamp> {
    let value = text(value);
    let value = value.trim();
    if let Ok(at) = value.parse::<jiff::Timestamp>() {
        return Some(at);
    }
    let local = value.parse::<jiff::civil::DateTime>().ok().or_else(|| {
        value
            .parse::<jiff::civil::Date>()
            .ok()
            .map(|date| date.at(0, 0, 0, 0))
    });
    local?
        .to_zoned(zone.tz().clone())
        .ok()
        .map(|time| time.timestamp())
}

fn escaped(value: &Value) -> String {
    h::escape(&text(value))
}
fn array(value: &Value) -> Vec<&Value> {
    match value {
        Value::Null => vec![],
        Value::Array(a) => a.iter().collect(),
        _ => vec![value],
    }
}
fn https(url: &str) -> bool {
    // URI::HTTPS normalizes its scheme; an absolute HTTPS URL may use any nonblank host.
    campfire_richtext::uri::parse(url).ok().is_some_and(|uri| {
        uri.scheme
            .as_deref()
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https"))
            && uri.host.is_some_and(|host| !h::is_blank(&host))
    })
}
impl Frame<'_> {
    pub fn render(&self) -> String {
        let body = if self.connect {
            self.chip(true)
        } else if let Some(payload) = self.payload.filter(|p| !blank(p)) {
            format!("{}\n", self.card(payload))
        } else if self.error == Some("not_found") {
            self.chip(false)
        } else if let Some(error) = self.error.filter(|s| !h::is_blank(s)) {
            format!(
                "<p class=\"fizzy-card__error\">\n      Couldn’t load this Fizzy card.\n      <span class=\"fizzy-card__error-detail\">{}</span>\n    </p>\n",
                h::escape(error)
            )
        } else {
            "<p class=\"fizzy-card__loading\">Loading Fizzy card…</p>\n".into()
        };
        format!(
            "<turbo-frame id=\"{}\">\n    {body}</turbo-frame>",
            h::escape(self.id)
        )
    }
    fn chip(&self, connect: bool) -> String {
        let hint = if connect {
            "    <span class=\"fizzy-chip__hint\"><a href=\"/users/me/profile\">Connect Fizzy to preview</a></span>\n"
        } else {
            ""
        };
        format!(
            "<p class=\"fizzy-chip\">\n  <a class=\"fizzy-chip__link\" target=\"_blank\" rel=\"noopener noreferrer\" href=\"{}\">Fizzy card #{}</a>\n{hint}</p>\n\n",
            h::escape(self.web_url),
            self.number
        )
    }
    fn card(&self, p: &Value) -> String {
        let (kind, label) = if truthy(&p["closed"]) {
            ("closed", "Closed".into())
        } else if truthy(&p["postponed"]) {
            ("postponed", "Postponed".into())
        } else if !blank(&p["column"]["name"]) {
            ("column", text(&p["column"]["name"]))
        } else {
            ("triage", "Maybe?".into())
        };
        let mut html = format!(
            "<article class=\"fizzy-card fizzy-card--{kind}\" data-fizzy-card=\"{}#{}\">\n  <header class=\"fizzy-card__header\">\n    <span class=\"fizzy-card__board\">{}</span>\n    <span class=\"fizzy-card__number\">#{}</span>\n    <span class=\"fizzy-card__state\">{}</span>\n  </header>\n\n",
            h::escape(self.account),
            self.number,
            escaped(&p["board"]["name"]),
            self.number,
            h::escape(&label)
        );
        if !blank(&p["title"]) {
            html.push_str(&format!("    <p class=\"fizzy-card__title\">{}</p>\n\n    <div class=\"fizzy-card__meta\">\n",escaped(&p["title"])));
            for assignee in array(&p["assignees"]) {
                html.push_str("        <span class=\"fizzy-card__assignee\">\n");
                let avatar = text(&assignee["avatar_url"]);
                if https(&avatar) {
                    html.push_str(&format!("            <img alt=\"\" class=\"fizzy-card__avatar\" loading=\"lazy\" src=\"{}\" width=\"16\" height=\"16\" />\n",h::escape(&avatar)));
                }
                html.push_str(&format!(
                    "          {}\n        </span>\n",
                    escaped(&assignee["name"])
                ));
            }
            if truthy(&p["has_more_assignees"]) {
                html.push_str("        <span class=\"fizzy-card__more-assignees\">+ more</span>\n");
            }
            html.push('\n');
            for tag in array(&p["tags"]) {
                html.push_str(&format!(
                    "        <span class=\"fizzy-card__tag\">#{}</span>\n",
                    escaped(tag)
                ));
            }
            html.push('\n');
            let steps = array(&p["steps"]);
            if !steps.is_empty() {
                let done = steps.iter().filter(|s| truthy(&s["completed"])).count();
                html.push_str(&format!(
                    "        <span class=\"fizzy-card__steps\">{done}/{} steps</span>\n",
                    steps.len()
                ));
            }
            html.push_str("    </div>\n");
        } else {
            html.push_str("    <p class=\"fizzy-card__loading\">Loading card…</p>\n");
        }
        html.push_str("\n  <footer class=\"fizzy-card__footer\">\n");
        if let Some(active) = last_active_at(self.zone, &p["last_active_at"]) {
            html.push_str(&format!(
                "      <span class=\"fizzy-card__updated\">Active {}</span>\n",
                crate::time::local_datetime_tag(self.zone, active, "time", h::attrs(), "").0
            ));
        }
        let url = text(&p["url"]);
        let url = if https(&url) { &url } else { self.web_url };
        html.push_str(&format!("    <a class=\"fizzy-card__link\" target=\"_blank\" rel=\"noopener noreferrer\" href=\"{}\">View in Fizzy</a>\n  </footer>\n</article>\n",h::escape(url)));
        html
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ws15e_fizzy_frames_match_pinned_rails_bytes() {
        let vectors: Value =
            serde_json::from_str(include_str!("../../../vectors/ws15e_fizzy_cards.json")).unwrap();
        for case in vectors["frames"].as_array().unwrap() {
            let frame = Frame {
                account: "897362094",
                number: 579,
                web_url: "https://app.fizzy.do/897362094/cards/579",
                id: "card_for_message_99_fizzy_card_3",
                connect: case["connect"].as_bool().unwrap(),
                payload: Some(&case["payload"]),
                error: case["error"].as_str(),
                zone: &Zone::utc(),
            };
            assert_eq!(
                frame.render(),
                case["html"].as_str().unwrap(),
                "{}",
                case["name"]
            );
        }
    }
}
