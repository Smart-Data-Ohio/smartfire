//! Stage fragments from app/views/rooms/stage and rooms/events/venue_live_dot.
use crate::helpers as h;
use askama::Template;
use rails_compat::unicode;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Member {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub avatar_path: String,
    pub administrator: bool,
    pub role: String,
    pub hand: Option<i64>,
    pub muted: bool,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Live {
    pub id: i64,
    pub membership_id: i64,
    pub name: String,
    pub identity: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Stage {
    pub room_id: i64,
    pub viewer_id: i64,
    pub members: Vec<Member>,
    pub live: Option<Live>,
}
impl Stage {
    pub fn viewer(&self) -> &Member {
        self.members
            .iter()
            .find(|m| m.id == self.viewer_id)
            .expect("stage viewer is a member")
    }
    fn can_manage(&self) -> bool {
        self.viewer().role == "host" || self.viewer().administrator
    }
    fn can_moderate(&self, target: &Member) -> bool {
        target.id != self.viewer_id && (self.viewer().administrator || !target.administrator)
    }
    fn admin_self_unmute(&self, target: &Member) -> bool {
        target.id == self.viewer_id && self.viewer().administrator && target.muted
    }
    fn group(&self, role: &str) -> Vec<&Member> {
        let mut members = self
            .members
            .iter()
            .filter(|m| m.role == role)
            .collect::<Vec<_>>();
        members.sort_by_key(|m| {
            (
                role == "listener" && m.hand.is_none(),
                if role == "listener" {
                    m.hand.unwrap_or(0)
                } else {
                    0
                },
                unicode::downcase(&m.name),
            )
        });
        members
    }
    pub fn dom_id(&self, prefix: &str) -> String {
        h::dom_id("rooms_stage", self.room_id, Some(prefix))
    }
    pub fn render(&self, partial: &str) -> String {
        match partial {
            "live_badge" => LiveBadge { stage: self }.render(),
            "live_dot" => LiveDot {
                stage: self,
                target: self.dom_id("sidebar_stage_live"),
            }
            .render(),
            "venue_live_dot" => {
                return LiveDot {
                    stage: self,
                    target: self.dom_id("event_stage_live"),
                }
                .render()
                .unwrap()
                    + "\n";
            }
            "controls" => Controls { stage: self }.render(),
            "roster" => Roster {
                stage: self,
                hosts: self.group("host"),
                speakers: self.group("speaker"),
                listeners: self.group("listener"),
            }
            .render(),
            "panel_body" => Panel {
                stage: self,
                rejoin: false,
            }
            .render(),
            "role_event" => RoleEvent {
                room_id: self.room_id,
                role: &self.viewer().role,
                muted: self.viewer().muted,
            }
            .render(),
            "stream_event" => StreamEvent {
                room_id: self.room_id,
            }
            .render(),
            _ => panic!("unknown stage partial {partial}"),
        }
        .expect("stage template")
    }
    pub fn panel(&self, rejoin: bool) -> String {
        Panel {
            stage: self,
            rejoin,
        }
        .render()
        .expect("stage panel")
    }
}
#[derive(Template)]
#[template(path = "rooms/stage/_live_badge.html")]
struct LiveBadge<'a> {
    stage: &'a Stage,
}
#[derive(Template)]
#[template(path = "rooms/stage/_live_dot.html")]
struct LiveDot<'a> {
    stage: &'a Stage,
    target: String,
}
#[derive(Template)]
#[template(path = "rooms/stage/_controls.html")]
struct Controls<'a> {
    stage: &'a Stage,
}
impl Controls<'_> {
    fn stop(&self, id: &i64) -> h::Html {
        button(
            &format!("/rooms/{}/stage/stream", self.stage.room_id),
            "Stop stream",
            "delete",
            "btn stage-panel__stop",
            Some(("stream_id", id.to_string())),
            h::attrs(),
            h::attrs()
                .data(
                    "action",
                    "turbo:submit-end->stage-panel#streamStopSubmitted",
                )
                .data("room_id", self.stage.room_id),
        )
    }
    fn hand(&self, raised: bool) -> h::Html {
        button(
            &format!("/rooms/{}/stage/hand", self.stage.room_id),
            if raised { "Lower hand" } else { "Raise hand" },
            if raised { "delete" } else { "post" },
            "btn stage-panel__hand",
            None,
            h::attrs(),
            h::attrs(),
        )
    }
    fn stream_form(&self) -> h::Html {
        let form = h::form_with(format!("/rooms/{}/stage/stream", self.stage.room_id))
            .class("stage-panel__stream-form")
            .data("room_id", self.stage.room_id);
        let label = form.label("quality", "Stream quality", h::attrs());
        let select = h::content_tag(
            "select",
            h::attrs().class("input").name("quality").id("quality"),
            "<option value=\"720p15\">720p15</option>\n<option selected=\"selected\" value=\"1080p15\">1080p15</option>\n<option value=\"1080p30\">1080p30</option>",
        );
        let submit = h::legacy_tag(
            "input",
            h::attrs()
                .type_("submit")
                .name("commit")
                .value("Go live")
                .class("btn btn--reversed stage-panel__go-live")
                .data("action", "click->stage-panel#goLive")
                .data("disable_with", "Go live"),
        );
        form.wrap(&format!("\n      <div class=\"stage-panel__stream-quality\">\n        {label}\n        {select}\n      </div>\n      {submit}\n"))
    }
}
#[derive(Template)]
#[template(path = "rooms/stage/_roster.html")]
struct Roster<'a> {
    stage: &'a Stage,
    hosts: Vec<&'a Member>,
    speakers: Vec<&'a Member>,
    listeners: Vec<&'a Member>,
}
impl Roster<'_> {
    fn avatar(&self, member: &Member, indent: usize) -> h::Html {
        let img = h::legacy_tag(
            "img",
            h::attrs()
                .aria("hidden", "true")
                .attr("src", member.avatar_path.as_str())
                .attr("width", 32)
                .attr("height", 32),
        );
        h::button_tag(
            h::attrs()
                .type_("button")
                .class("avatar stage-panel__avatar profile-card-avatar")
                .aria("label", format!("View profile of {}", member.name))
                .merge(h::profile_card_trigger(member.user_id, false)),
            &format!("\n{}{img}\n", " ".repeat(indent)),
        )
    }
    fn name_button(&self, member: &Member) -> h::Html {
        h::button_tag(
            h::attrs()
                .type_("button")
                .class("profile-card-name overflow-ellipsis")
                .merge(h::profile_card_trigger(member.user_id, false)),
            &format!("<strong>{}</strong>", h::escape(&member.name)),
        )
    }
    fn role_button(
        &self,
        member: &Member,
        label: &str,
        role: &str,
        sole: bool,
        reversed: bool,
    ) -> h::Html {
        let attrs = if sole {
            h::attrs()
                .attr("disabled", true)
                .title("The stage needs at least one host")
        } else {
            h::attrs()
        };
        button(
            &format!("/rooms/{}/stage/roles/{}", self.stage.room_id, member.id),
            label,
            "patch",
            if reversed {
                "btn btn--reversed stage-panel__action"
            } else {
                "btn btn--plain stage-panel__action"
            },
            Some(("stage_role", role.into())),
            attrs,
            h::attrs(),
        )
    }
    fn moderation(&self, member: &Member, action: &str) -> h::Html {
        let (label, method) = match action {
            "mute" => ("Mute", "post"),
            "unmute" => ("Unmute", "delete"),
            _ => ("Disconnect", "post"),
        };
        button(
            &format!(
                "/rooms/{}/call_moderation/{}/{}",
                self.stage.room_id,
                member.id,
                if action == "unmute" { "mute" } else { action }
            ),
            label,
            method,
            "btn btn--plain stage-panel__action",
            None,
            h::attrs(),
            h::attrs(),
        )
    }
    fn lower(&self, member: &Member) -> h::Html {
        button(
            &format!("/rooms/{}/stage/hand", self.stage.room_id),
            "Lower hand",
            "delete",
            "btn btn--plain stage-panel__action",
            Some(("membership_id", member.id.to_string())),
            h::attrs(),
            h::attrs(),
        )
    }
}
// button_to's params follow the button and token, in sorted hidden-field order.
fn button(
    url: &str,
    label: &str,
    method: &str,
    class: &str,
    param: Option<(&str, String)>,
    extra: h::Attrs,
    form: h::Attrs,
) -> h::Html {
    let mut html = h::button_to_form(
        url,
        h::attrs().method(method).class(class).merge(extra),
        form,
        &h::escape(label),
    )
    .0;
    if let Some((key, value)) = param {
        html.truncate(html.len() - 7);
        html.push_str(&h::legacy_tag("input", h::attrs().type_("hidden").name(key).value(value)).0);
        html.push_str("</form>");
    }
    h::raw(html)
}
#[derive(Template)]
#[template(path = "rooms/stage/_panel_body.html")]
struct Panel<'a> {
    stage: &'a Stage,
    rejoin: bool,
}
#[derive(Template)]
#[template(path = "rooms/stage/_role_event.html")]
struct RoleEvent<'a> {
    room_id: i64,
    role: &'a str,
    muted: bool,
}
#[derive(Template)]
#[template(path = "rooms/stage/_stream_event.html")]
struct StreamEvent {
    room_id: i64,
}
pub fn stream_event(room_id: i64) -> String {
    StreamEvent { room_id }.render().unwrap()
}
pub fn role_event(room_id: i64, role: &str, muted: bool) -> String {
    RoleEvent {
        room_id,
        role,
        muted,
    }
    .render()
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_parity_stage_roster_uses_ruby_sort_order_with_hand_priority() {
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("../../../vectors/unicode_casing_parity.json"))
                .unwrap();
        for role in ["host", "speaker", "listener"] {
            let mut stage = Stage {
                room_id: 73,
                viewer_id: 1,
                live: None,
                members: ["ΟΣ", "οςa"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, name)| Member {
                        id: i as i64 + 1,
                        user_id: i as i64 + 1,
                        name: name.into(),
                        avatar_path: "/avatar".into(),
                        administrator: false,
                        role: role.into(),
                        hand: None,
                        muted: false,
                    })
                    .collect(),
            };
            assert_eq!(
                serde_json::json!(
                    stage
                        .group(role)
                        .iter()
                        .map(|m| &m.name)
                        .collect::<Vec<_>>()
                ),
                oracle["sigma_names"]
            );
            if role == "listener" {
                stage.members[0].hand = Some(10);
                assert_eq!(stage.group(role)[0].name, "ΟΣ");
                stage.members[1].hand = Some(5);
                assert_eq!(stage.group(role)[0].name, "οςa");
            }
        }
    }
    #[test]
    fn stage_fragments_match_four_hundred_rails_renders() {
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("huddle_stage_vectors.json")).unwrap();
        assert_eq!(vectors["cases"].as_array().unwrap().len(), 50);
        for case in vectors["cases"].as_array().unwrap() {
            let stage: Stage = serde_json::from_value(case["input"].clone()).unwrap();
            for (partial, expected) in case["html"].as_object().unwrap() {
                let actual = stage.render(partial);
                let expected = expected.as_str().unwrap();
                if actual != expected {
                    let offset = actual
                        .bytes()
                        .zip(expected.bytes())
                        .position(|(a, b)| a != b)
                        .unwrap_or(actual.len().min(expected.len()));
                    panic!(
                        "{} {partial} first difference {offset}: actual {:?}, expected {:?}",
                        case["name"],
                        actual.get(offset.saturating_sub(30)..(offset + 80).min(actual.len())),
                        expected.get(offset.saturating_sub(30)..(offset + 80).min(expected.len()))
                    );
                }
            }
        }
    }
}
