//! The agent-step and poll children of the shared message tree. Domain owners supply facts;
//! these views contain no viewer capabilities, selected ballot or session token.
use crate::{ViewContext, helpers as h};
use askama::Template;

#[derive(Template)]
#[template(path = "agent_steps/_steps.html")]
struct Steps<'a> {
    steps: &'a [AgentStep],
}
pub fn steps(steps: &[AgentStep]) -> h::Html {
    h::raw(Steps { steps }.render().expect("agent_steps/steps renders"))
}
pub trait PollRendering {
    fn form_open(&self, class: &str) -> h::Html;
    fn close_time(&self, ctx: &ViewContext) -> h::Html;
    fn input(&self, option_id: &i64) -> h::Html;
    fn submit(&self, retract: bool) -> h::Html;
}
impl PollRendering for Poll {
    fn form_open(&self, class: &str) -> h::Html {
        h::form_with(campfire_routes::vote_room_poll(self.room_id, self.id))
            .class(class)
            .authenticity_token(false)
            .open()
    }
    fn close_time(&self, ctx: &ViewContext) -> h::Html {
        self.closes_at.map_or_else(h::empty, |time| {
            crate::time::local_datetime_tag(
                &ctx.time_zone,
                time,
                "datetime",
                h::attrs().class("poll__close-time"),
                "",
            )
        })
    }
    fn input(&self, option_id: &i64) -> h::Html {
        let options = h::attrs()
            .id(format!("poll_{}_option_{option_id}", self.id))
            .class("poll__input")
            .data("poll_target", "input");
        if self.multiple {
            h::legacy_tag(
                "input",
                h::attrs()
                    .type_("checkbox")
                    .name("option_ids[]")
                    .id("option_ids_")
                    .value(option_id.to_string())
                    .merge(options),
            )
        } else {
            h::radio_button_tag("option_ids[]", &option_id.to_string(), false, options)
        }
    }
    fn submit(&self, retract: bool) -> h::Html {
        let value = if retract { "Retract vote" } else { "Vote" };
        let attrs = h::attrs()
            .type_("submit")
            .name("commit")
            .value(value)
            .class(if retract {
                "btn btn--plain btn--small"
            } else {
                "btn btn--primary btn--small"
            })
            .data("poll_target", if retract { "retract" } else { "submit" });
        h::legacy_tag(
            "input",
            if retract {
                attrs.data("disable_with", value).hidden()
            } else {
                attrs.data("disable_with", value)
            },
        )
    }
}

#[derive(Template)]
#[template(path = "polls/_poll.html")]
struct PollPartial<'a> {
    ctx: &'a ViewContext<'a>,
    poll: &'a Poll,
}
pub fn poll(ctx: &ViewContext, poll: &Poll) -> h::Html {
    h::raw(PollPartial { ctx, poll }.render().expect("polls/poll renders"))
}
pub use campfire_presentation::messages::parts::*;
