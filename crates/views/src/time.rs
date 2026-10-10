//! `SetTimeZone`, ActiveSupport::TimeZone and the formats used by our Rails templates.

pub mod calendar;
pub use calendar::CalendarTime;

/// `TimeHelper#local_datetime_tag`. The supplied block content is already rendered HTML.
pub fn local_datetime_tag(
    zone: &Zone,
    instant: jiff::Timestamp,
    style: &str,
    attributes: crate::helpers::Attrs,
    content: &str,
) -> crate::helpers::Html {
    local_datetime_tag_iso(&zone.iso8601(instant), style, attributes, content)
}

/// A preformatted presentation value, including years outside Jiff's envelope.
pub fn local_datetime_tag_iso(iso: &str, style: &str, attributes: crate::helpers::Attrs, content: &str) -> crate::helpers::Html {
    crate::helpers::content_tag(
        "time",
        attributes
            .attr("datetime", iso)
            .data("local_time_target", style),
        content,
    )
}
pub use campfire_presentation::time::*;
