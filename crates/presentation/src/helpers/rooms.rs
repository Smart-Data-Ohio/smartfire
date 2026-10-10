

pub const FIRST_PAINT_CONTROLLERS: &[&str] = &[
    "messages",
    "maintain_scroll",
    "reply",
    "composer",
    "markdown_editor",
    "typing_notifications",
    "local_time",
    "presence",
    "message_list",
    "header_overflow",
    "attach_menu",
];

/// A room as the involvement helpers see it.
pub struct InvolvementRoom<'r> {
    pub id: i64,
    /// `model_name.param_key` of the room's class: "rooms_open", "rooms_closed" or "rooms_direct".
    pub param_key: &'r str,
    pub direct: bool,
}

/// `HUMANIZE_INVOLVEMENT`.
pub fn humanize_involvement(involvement: &str) -> &'static str {
    match involvement {
        "mentions" => "Notifying about @ mentions",
        "everything" => "Notifying about all messages",
        "muted" => "Muted, notifying only about @ mentions",
        "nothing" => "Notifications are off",
        "invisible" => "Notifications are off and room invisible in sidebar",
        _ => "",
    }
}

/// `involvement_levels_for(room)`.
pub fn involvement_levels(direct: bool) -> &'static [&'static str] {
    if direct {
        &["everything", "muted", "nothing"]
    } else {
        &["mentions", "everything", "muted", "nothing", "invisible"]
    }
}

/// `short_involvement_label(level)` for the notification chooser.
pub fn short_involvement_label(involvement: &str) -> &'static str {
    match involvement {
        "mentions" => "Mentions",
        "everything" => "Everything",
        "muted" => "Muted",
        "nothing" => "Off",
        "invisible" => "Invisible",
        _ => panic!("unknown involvement {involvement}"),
    }
}

/// `next_involvement_for(room, involvement:)`.
pub fn next_involvement(direct: bool, involvement: &str) -> &'static str {
    let order = involvement_levels(direct);
    let index = order.iter().position(|candidate| *candidate == involvement);
    index
        .and_then(|index| order.get(index + 1))
        .copied()
        .unwrap_or(order[0])
}
