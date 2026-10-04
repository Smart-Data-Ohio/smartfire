//! Huddle render models, independent of the database and the request/session.
use crate::helpers as h;
use askama::Template;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Participant {
    pub id: i64,
    pub name: String,
    pub avatar_path: String,
}

#[derive(Template)]
#[template(path = "rooms/huddles/_participants.html")]
struct Participants<'a> {
    room_type: &'a str,
    room_id: i64,
    placement: &'a str,
    participants: &'a [Participant],
}
impl Participants<'_> {
    fn dom_id(&self) -> String {
        h::dom_id(
            &self.room_type.replace("::", "_").to_lowercase(),
            self.room_id,
            Some(&format!("{}_voice_participants", self.placement)),
        )
    }
    fn max_avatars(&self) -> usize {
        if self.placement == "header" { 5 } else { 3 }
    }
    fn interval(&self) -> usize {
        if self.placement == "sidebar" {
            0
        } else {
            15000
        }
    }
    fn label(&self) -> &str {
        match self.room_type {
            "Rooms::Stage" => "on stage",
            "Rooms::Voice" => "in voice",
            _ => "in huddle",
        }
    }
    fn class(&self) -> String {
        let mut class = "voice-stack".to_string();
        if !self.participants.is_empty() {
            class.push_str(" voice-stack--live");
        }
        if !matches!(self.room_type, "Rooms::Voice" | "Rooms::Stage") {
            class.push_str(" voice-stack--huddle");
        }
        class
    }
    fn accessible_label(&self) -> String {
        if self.participants.is_empty() {
            format!("Nobody {}", self.label())
        } else {
            format!(
                "{} {}: {}",
                self.participants.len(),
                self.label(),
                h::to_sentence(
                    &self
                        .participants
                        .iter()
                        .map(|user| user.name.clone())
                        .collect::<Vec<_>>(),
                    " and "
                )
            )
        }
    }
    fn avatars(&self) -> h::Html {
        let mut html = String::new();
        for user in self.participants.iter().take(self.max_avatars()) {
            let image = h::legacy_tag(
                "img",
                h::attrs()
                    .alt("")
                    .title(user.name.as_str())
                    .class("voice-stack__avatar")
                    .data("user_id", user.id)
                    .attr("src", user.avatar_path.as_str())
                    .attr("width", 20)
                    .attr("height", 20),
            );
            let trigger = h::content_tag(
                "span",
                h::attrs()
                    .class("voice-stack__trigger profile-card-trigger")
                    .attr("tabindex", 0)
                    .role("button")
                    .aria("label", format!("View profile of {}", user.name))
                    .merge(h::profile_card_trigger(user.id, true)),
                &format!("\n        {}\n", image.0),
            );
            html.push_str(&format!("      {}", trigger.0));
        }
        h::raw(html)
    }
}

pub fn participants(
    room_type: &str,
    room_id: i64,
    placement: &str,
    participants: &[Participant],
) -> String {
    Participants {
        room_type,
        room_id,
        placement,
        participants,
    }
    .render()
    .expect("huddle participants template")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huddle_presence_matches_fifty_pinned_rails_renders() {
        let vectors: serde_json::Value =
            serde_json::from_str(include_str!("huddle_presence_vectors.json")).unwrap();
        assert_eq!(vectors["reference_pin"], &include_str!("../../../parity/reference.sha").trim()[..8]);
        let cases = vectors["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 50);
        for case in cases {
            let users: Vec<Participant> =
                serde_json::from_value(case["participants"].clone()).unwrap();
            assert_eq!(
                participants(
                    case["type"].as_str().unwrap(),
                    case["room_id"].as_i64().unwrap(),
                    case["placement"].as_str().unwrap(),
                    &users
                ),
                case["html"].as_str().unwrap(),
                "{case}"
            );
        }
    }
}
