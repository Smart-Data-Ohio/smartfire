//! Accounts integration health page. Read-only snapshots contain no credential values.
use crate::{ViewContext, helpers as h, layouts::Page};
use askama::Template;
use serde_json::Value;
use crate::helpers::filters;
#[derive(Template)]
#[template(path="accounts/integrations_health/show.html",blocks=["head","content"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub snapshot: Value,
}
impl Page for Show<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Integration health".into())
    }
}
impl Show<'_> {
    fn body(&self) -> h::Html {
        h::raw(body(&self.snapshot, &self.ctx.time_zone))
    }
}
fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => h::escape(s),
        _ => h::escape(&value.to_string()),
    }
}
fn pairs(out: &mut String, value: &Value, title: &str, prefix: &str) {
    if let Some(rows) = value.as_array().filter(|r| !r.is_empty()) {
        out.push_str(&format!("      <h3>{title}</h3>\n      <ul>\n"));
        for r in rows {
            out.push_str(&format!(
                "          <li><strong>{prefix}{}</strong> — {}</li>\n",
                text(&r[0]),
                text(&r[1])
            ));
        }
        out.push_str("      </ul>\n");
    }
}
pub fn body(v: &Value, zone: &crate::time::Zone) -> String {
    let mut s = String::from(
        "\n\n<div class=\"panel flex flex-column gap\">\n  <h1 class=\"margin-none\">Integration health</h1>\n\n",
    );
    s.push_str(&crate::github::health(&v["github"]));
    s.push('\n');
    let g = &v["google"];
    s.push_str(&format!("  <section aria-labelledby=\"health-google\">\n    <h2 id=\"health-google\">Google Calendar</h2>\n    <dl class=\"flex flex-column gap-half\">\n      <div><dt>OAuth client</dt><dd>{}</dd></div>\n      <div><dt>Connected accounts</dt><dd>{}</dd></div>\n      <div><dt>Push channels</dt><dd>{}</dd></div>\n    </dl>\n",if g["configured"]==true{"Configured"}else{"Not configured — set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET"},text(&g["connected"]),if g["push"]["enabled"]==true{format!("{} watching",text(&g["push"]["count"]))}else{"Disabled — set GOOGLE_CALENDAR_WEBHOOK_URL for two-way RSVP sync (see docs/meet-and-rsvp.md)".into()}));
    pairs(&mut s, &g["disconnected"], "Disconnected accounts", "");
    if let Some(rows) = g["entry_errors"].as_array().filter(|r| !r.is_empty()) {
        s.push_str("      <h3>Calendar entry errors</h3>\n      <ul>\n");
        for r in rows {
            s.push_str(&format!(
                "          <li><strong>event {} / user {}</strong> — {}</li>\n",
                text(&r[0]),
                text(&r[1]),
                text(&r[2])
            ));
        }
        s.push_str("      </ul>\n");
    }
    if let Some(rows) = g["push"]["expiring"].as_array().filter(|r| !r.is_empty()) {
        s.push_str("      <h3>Push channels expiring within 24 hours</h3>\n      <ul>\n");
        for r in rows {
            let expiry = r[1]
                .as_str()
                .and_then(|s| s.parse::<jiff::Timestamp>().ok())
                .map(|t| zone.to_fs(t, "default"))
                .unwrap_or("unknown".into());
            let error = r[2]
                .as_str()
                .filter(|s| !s.chars().all(char::is_whitespace))
                .map(|s| format!(" — {}", h::escape(s)))
                .unwrap_or_default();
            s.push_str(&format!(
                "          <li><strong>user {}</strong> — expires {}{error}</li>\n",
                text(&r[0]),
                h::escape(&expiry)
            ));
        }
        s.push_str("      </ul>\n");
    }
    s.push_str("  </section>\n\n  <section aria-labelledby=\"health-fizzy\">\n    <h2 id=\"health-fizzy\">Fizzy</h2>\n");
    s.push_str(&format!(
        "    <p>{}</p>\n  </section>\n\n",
        if v["fizzy"]["configured"] == true {
            "Configured".into()
        } else {
            text(&v["fizzy"]["note"])
        }
    ));
    let a = &v["agent_delivery"];
    s.push_str(&format!("  <section aria-labelledby=\"health-delivery\">\n    <h2 id=\"health-delivery\">Webhook and agent delivery</h2>\n    <dl class=\"flex flex-column gap-half\">\n      <div><dt>Agent webhooks pending</dt><dd>{}</dd></div>\n      <div><dt>Agent webhooks failed (24h)</dt><dd>{}</dd></div>\n    </dl>\n",text(&a["pending"]),text(&a["failed"])));
    if let Some(rows) = a["recent_errors"].as_array().filter(|r| !r.is_empty()) {
        s.push_str("      <h3>Recent delivery errors</h3>\n      <ul>\n");
        for r in rows {
            s.push_str(&format!(
                "          <li><strong>agent {} / {}</strong> — {}</li>\n",
                text(&r[0]),
                text(&r[1]),
                text(&r[2])
            ));
        }
        s.push_str("      </ul>\n");
    }
    s.push_str("  </section>\n\n  <section aria-labelledby=\"health-email\">\n    <h2 id=\"health-email\">Email to room</h2>\n");
    if v["email"]["enabled"] == true {
        let n = v["email"]["rooms_with_addresses"].as_i64().unwrap_or(0);
        s.push_str(&format!(
            "      <p>{n} {} with a forward-to address.</p>\n",
            if n == 1 { "room" } else { "rooms" }
        ));
    } else {
        s.push_str("      <p>Inbound email is not configured. Set <code>INBOUND_EMAIL_DOMAIN</code> and the relay ingress password (see docs/email-to-room.md).</p>\n");
    }
    s.push_str("  </section>\n</div>\n");
    s
}
