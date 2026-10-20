//! The opaque keyset cursors the list endpoints hand out as `nextCursor` and take back as
//! `before`: the base64url (unpadded) `"<timestamp>|<id>"` of the previous page's last row's
//! sort key.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use campfire_db::Timestamp;

pub(crate) fn encode(at: Timestamp, id: i64) -> String {
    URL_SAFE_NO_PAD.encode(format!("{}|{id}", at.to_db()))
}

/// `None` for anything [`encode`] didn't make.
pub(crate) fn decode(raw: &str) -> Option<(Timestamp, i64)> {
    let text = String::from_utf8(URL_SAFE_NO_PAD.decode(raw).ok()?).ok()?;
    let (at, id) = text.rsplit_once('|')?;
    Some((Timestamp::parse_db(at)?, id.parse().ok()?))
}

/// An id-only cursor (`"<id>"`), for lists ordered by id alone.
pub(crate) fn encode_id(id: i64) -> String {
    URL_SAFE_NO_PAD.encode(id.to_string())
}

/// `None` for anything [`encode_id`] didn't make.
pub(crate) fn decode_id(raw: &str) -> Option<i64> {
    let text = String::from_utf8(URL_SAFE_NO_PAD.decode(raw).ok()?).ok()?;
    text.parse().ok().filter(|id: &i64| *id > 0)
}

/// A rank-prefixed cursor (`"<rank>|<timestamp>|<id>"`), for lists ordered by a match rank first.
pub(crate) fn encode_ranked(rank: i64, at: Timestamp, id: i64) -> String {
    URL_SAFE_NO_PAD.encode(format!("{rank}|{}|{id}", at.to_db()))
}

/// `None` for anything [`encode_ranked`] didn't make.
pub(crate) fn decode_ranked(raw: &str) -> Option<(i64, Timestamp, i64)> {
    let text = String::from_utf8(URL_SAFE_NO_PAD.decode(raw).ok()?).ok()?;
    let (rank, rest) = text.split_once('|')?;
    let (at, id) = rest.rsplit_once('|')?;
    Some((
        rank.parse().ok()?,
        Timestamp::parse_db(at)?,
        id.parse().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cursor_round_trips_and_garbage_does_not_decode() {
        let at = Timestamp::parse_db("2026-10-06 12:00:00.123456").unwrap();
        assert_eq!(decode(&encode(at, 42)), Some((at, 42)));
        for raw in ["", "nope", "bm9wZQ", "fDQy"] {
            assert_eq!(decode(raw), None, "{raw}");
        }
    }

    #[test]
    fn an_id_cursor_round_trips_and_garbage_does_not_decode() {
        assert_eq!(decode_id(&encode_id(42)), Some(42));
        for raw in ["", "nope", "LTE", "MA"] {
            assert_eq!(decode_id(raw), None, "{raw}");
        }
    }

    #[test]
    fn a_ranked_cursor_round_trips_and_garbage_does_not_decode() {
        let at = Timestamp::parse_db("2026-10-06 12:00:00.123456").unwrap();
        assert_eq!(decode_ranked(&encode_ranked(3, at, 42)), Some((3, at, 42)));
        for raw in ["", "nope", "fDQy", &encode(at, 42)] {
            assert_eq!(decode_ranked(raw), None, "{raw}");
        }
    }
}
