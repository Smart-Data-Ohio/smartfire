//! View models for message parts that message features and the cable renderers share.
use campfire_db::Message;

pub fn poll_view(
    conn: &campfire_db::Connection,
    app: &crate::app::App,
    id: i64,
    error: Option<String>,
) -> campfire_db::Result<campfire_views::messages::parts::Poll> {
    let poll = campfire_db::Poll::find(conn, id)?;
    let message = Message::find(conn, poll.message_id)?;
    let options = poll
        .options(conn)?
        .into_iter()
        .map(|option| campfire_views::messages::parts::PollOption {
            id: option.id,
            label: option.label,
        })
        .collect();
    let votes = poll
        .votes_with_names(conn)?
        .into_iter()
        .map(|(vote,name)| {
            campfire_views::messages::parts::PollVote {
                option_id: vote.poll_option_id,
                user_id: vote.user_id,
                user_name: name,
            }
        })
        .collect();
    Ok(campfire_views::messages::parts::Poll {
        id,
        room_id: message.room_id,
        anonymous: poll.anonymous,
        multiple: poll.multiple,
        closed: poll.closed(app.db.env().now()),
        closes_at: poll.closes_at.map(|time| time.jiff()),
        options,
        votes,
        vote_error: error,
    })
}
