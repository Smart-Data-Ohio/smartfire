//! `Internal::HuddleController`: an API controller, without the session/CSRF chain.
use crate::app::AppCtx;
use crate::huddle::Config;
use campfire_db::Timestamp;
use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig};
use campfire_kit::{Ctx, Param, Result, StatusCode};

fn authenticate(c: &mut Ctx) -> Option<Config> {
    c.set_header("cache-control", "no-store");
    let config = c.app().config.huddle.clone();
    let provided = c.request.header("x-huddle-gateway-secret").unwrap_or("");
    let expected = config.gateway_secret.as_deref().unwrap_or("");
    if provided.chars().all(char::is_whitespace)
        || expected.chars().all(char::is_whitespace)
        || !campfire_db::models::user::secure_compare(provided.as_bytes(), expected.as_bytes())
    {
        return None;
    }
    Some(config)
}
fn domain_config(config: &Config) -> HuddleConfig {
    HuddleConfig {
        api_secret: config
            .signing_configured()
            .then(|| config.api_secret.clone())
            .flatten(),
        admin_configured: config.admin_configured(),
    }
}
fn before(c: &mut Ctx) -> std::result::Result<Config, StatusCode> {
    let config = authenticate(c).ok_or(StatusCode::UNAUTHORIZED)?;
    if !config.configured() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    Ok(config)
}
pub async fn authorize(c: &mut Ctx) -> Result {
    let config = match before(c) {
        Ok(config) => config,
        Err(status) => return Ok(c.head(status)),
    };
    // Ruby split(" ", 2) skips leading whitespace but preserves the second field's tail.
    let authorization = c
        .request
        .header("authorization")
        .unwrap_or("")
        .trim_start_matches(char::is_whitespace);
    let (scheme, token) = authorization
        .split_once(char::is_whitespace)
        .unwrap_or((authorization, ""));
    let token = token.trim_start_matches(char::is_whitespace);
    if !scheme.eq_ignore_ascii_case("Bearer") || token.chars().all(char::is_whitespace) {
        return Ok(c.head(StatusCode::UNAUTHORIZED));
    }
    let coordinates = match rails_compat::jwt::livekit::verify(
        token,
        config.api_key.as_deref().unwrap(),
        config.api_secret.as_deref().unwrap(),
        c.app().clock.now().as_second(),
    ) {
        Ok(coordinates) => coordinates,
        Err(_) => return Ok(c.head(StatusCode::UNAUTHORIZED)),
    };
    let grant = c
        .app()
        .db
        .read(move |conn| {
            HuddleGrant::find_by_coordinates(conn, &coordinates.identity, &coordinates.room_name)
        })
        .await
        .map_err(anyhow::Error::from)?;
    respond(c, grant, true, StatusCode::FORBIDDEN, &config).await
}
pub async fn show(c: &mut Ctx) -> Result {
    let config = match before(c) {
        Ok(config) => config,
        Err(status) => return Ok(c.head(status)),
    };
    let id = integer_id(c.param_str("id").unwrap_or(""));
    let grant = match id {
        Some(id) => c
            .app()
            .db
            .read(move |conn| HuddleGrant::find_by_id(conn, id))
            .await
            .map_err(anyhow::Error::from)?,
        None => None,
    };
    let record_seen = c.param_str("record_seen") != Some("0");
    respond(c, grant, record_seen, StatusCode::NOT_FOUND, &config).await
}
async fn respond(
    c: &mut Ctx,
    grant: Option<HuddleGrant>,
    record_seen: bool,
    denied: StatusCode,
    config: &Config,
) -> Result {
    let Some(grant) = grant else {
        return Ok(c.head(denied));
    };
    let now = Timestamp::from_jiff(c.app().clock.now());
    let id = grant.id;
    let (grant, authorized) = c
        .app()
        .db
        .read(move |conn| {
            let authorized = grant.authorized(conn)?;
            Ok((grant, authorized))
        })
        .await
        .map_err(anyhow::Error::from)?;
    // The steady-state check is a read. Write only an invalid grant or a due sighting;
    // the immediate transaction repeats authorization before either mutation.
    let seen_due = record_seen
        && grant
            .last_seen_at
            .is_none_or(|at| at <= now.ago(jiff::SignedDuration::from_secs(10)));
    let grant = if !authorized || seen_due {
        let domain = domain_config(config);
        c.app()
            .db
            .write(move |tx| {
                let Some(mut grant) = HuddleGrant::authorize_or_revoke(tx, id, &domain)? else {
                    return Ok(None);
                };
                if record_seen {
                    grant.record_seen(tx)?;
                }
                Ok(Some(grant))
            })
            .await
            .map_err(anyhow::Error::from)?
    } else {
        Some(grant)
    };
    match grant {
        Some(grant) => c.json(StatusCode::OK, &grant.authorization_payload()),
        None => Ok(c.head(denied)),
    }
}
pub async fn left(c: &mut Ctx) -> Result {
    if let Err(status) = before(c) {
        return Ok(c.head(status));
    }
    let Some(id) = integer_id(c.param_str("id").unwrap_or("")) else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    if c.app()
        .db
        .read(move |conn| HuddleGrant::find_by_id(conn, id))
        .await
        .map_err(anyhow::Error::from)?
        .is_none()
    {
        return Ok(c.head(StatusCode::NOT_FOUND));
    }
    let raw = c.param("disconnected_at");
    let floor = raw.and_then(|p| parse_time(&ruby_string(p), c.app().clock.now()));
    if raw.is_some_and(param_present) && floor.is_none() {
        return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
    }
    c.app()
        .db
        .write(move |tx| {
            if let Some(mut grant) = HuddleGrant::find_by_id(tx.conn(), id)? {
                grant.mark_out_of_call(tx, floor)?;
            }
            Ok(())
        })
        .await
        .map_err(anyhow::Error::from)?;
    Ok(c.head(StatusCode::OK))
}
/// Rails' integer attribute cast accepts a numeric prefix ("17tail" => 17).
fn integer_id(raw: &str) -> Option<i64> {
    let raw = raw.trim_start();
    let sign = usize::from(raw.starts_with(['+', '-']));
    let digits = raw[sign..].bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    raw[..sign + digits].parse().ok()
}
fn param_present(p: &Param) -> bool {
    match p {
        Param::Null | Param::Bool(false) => false,
        Param::Str(s) => !s.chars().all(char::is_whitespace),
        Param::Array(a) => !a.is_empty(),
        Param::Hash(h) => !h.is_empty(),
        _ => true,
    }
}
fn ruby_string(p: &Param) -> String {
    match p {
        Param::Null => String::new(),
        Param::Str(s) => s.clone(),
        _ => p.to_json().to_string(),
    }
}
/// UTC Time.zone.parse's ISO/date/time forms used by the gateway. Ruby normalizes dates
/// such as Feb 30 and accepts years beyond Jiff's range; clamp those only for comparison.
fn parse_time(raw: &str, now: jiff::Timestamp) -> Option<Timestamp> {
    use std::sync::LazyLock;
    static DATE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(-?\d{4,})-(\d{1,2})-(\d{1,2})").unwrap());
    static TIME: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(\d{1,2}):(\d{2})(?::(\d{2})(?:\.(\d+))?)?").unwrap());
    static OFFSET: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"^\s*([+-])(\d{2})(?::?(\d{2}))?(?::?(\d{2}))?").unwrap()
    });
    let date = DATE.captures(raw);
    let time = TIME.captures(raw);
    if date.is_none() && time.is_none() {
        return None;
    }
    let current = now.to_zoned(jiff::tz::TimeZone::UTC).datetime();
    let parse = |s: &str| s.parse::<i64>().ok();
    let (year, month, day) = match date.as_ref() {
        Some(d) => (parse(&d[1])?, parse(&d[2])?, parse(&d[3])?),
        None => (
            i64::from(current.year()),
            i64::from(current.month()),
            i64::from(current.day()),
        ),
    };
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let (hour, minute, second, fraction) = match time.as_ref() {
        Some(t) => (
            parse(&t[1])?,
            parse(&t[2])?,
            t.get(3).map_or(Some(0), |s| parse(s.as_str()))?,
            t.get(4).map(|s| s.as_str()).unwrap_or(""),
        ),
        None => (0, 0, 0, ""),
    };
    if hour > 24
        || minute > 59
        || second > 60
        || (hour == 24 && (minute != 0 || second != 0 || fraction.bytes().any(|b| b != b'0')))
    {
        return None;
    }
    // Time.zone.parse honors both compact and colon-separated numeric offsets.
    // Validate before normalization: Time.new rejects +2500 and 24:01, but
    // accepts exactly 24:00 and normalizes leap seconds rather than clamping.
    let offset = match time
        .as_ref()
        .and_then(|t| OFFSET.captures(&raw[t.get(0).unwrap().end()..]))
    {
        Some(zone) => {
            let hour = parse(&zone[2])?;
            let minute = zone.get(3).map_or(Some(0), |m| parse(m.as_str()))?;
            let second = zone.get(4).map_or(Some(0), |s| parse(s.as_str()))?;
            if hour > 23 || minute > 59 || second > 59 {
                return None;
            }
            let sign = if &zone[1] == "-" { -1 } else { 1 };
            sign * (hour * 3600 + minute * 60 + second)
        }
        None => 0,
    };
    if year > 9999 {
        return Some(Timestamp::from_second(jiff::Timestamp::MAX.as_second() - 1));
    }
    if year < -9999 {
        return Some(Timestamp::from_second(jiff::Timestamp::MIN.as_second() + 1));
    }
    let micros: i64 = format!("{fraction:0<6}")
        .chars()
        .take(6)
        .collect::<String>()
        .parse()
        .ok()?;
    let base = jiff::civil::date(year as i16, month as i8, 1)
        .at(0, 0, 0, 0)
        .to_zoned(jiff::tz::TimeZone::UTC)
        .ok()?
        .timestamp();
    let seconds = (day - 1) * 86400 + hour * 3600 + minute * 60 + second - offset;
    Some(Timestamp::from_jiff(
        base.checked_add(
            jiff::SignedDuration::from_secs(seconds)
                .checked_add(jiff::SignedDuration::from_micros(micros))?,
        )
        .ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn huddle_integer_cast_accepts_ruby_prefixes() {
        for (raw, expected) in [
            ("17tail", Some(17)),
            ("  +17", Some(17)),
            ("nope", None),
            ("", None),
            ("999999999999999999999999", None),
        ] {
            assert_eq!(integer_id(raw), expected);
        }
    }

    #[test]
    fn ws13b_review_disconnect_parser_matches_pinned_rails_offsets_and_bounds() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../db/src/tests/ws13b_review_fixes.json"
        ))
        .unwrap();
        let now: jiff::Timestamp = "2026-01-01T12:00:01Z".parse().unwrap();
        for case in oracle["times"].as_array().unwrap() {
            let expected = case["parsed"]
                .as_str()
                .map(|raw| Timestamp::from_jiff(raw.parse().unwrap()));
            assert_eq!(
                parse_time(case["input"].as_str().unwrap(), now),
                expected,
                "{}",
                case["input"]
            );
        }
    }
    #[test]
    fn ws13b_review_disconnect_rejects_invalid_24_hour_time() {
        let now: jiff::Timestamp = "2026-01-01T12:00:01Z".parse().unwrap();
        assert_eq!(parse_time("2026-01-01T24:01:00Z", now), None);
    }
}
