//! Whether Ruby's `URI.parse` (the RFC 3986 parser, `uri` 1.0 in Ruby 3.4) yields a `URI::HTTP`
//! (or `URI::HTTPS`), as `Embeds::ImageProxy.verified_url` asks.
use std::sync::LazyLock;

use regex::Regex;

// `URI::RFC3986_Parser::RFC3986_URI`, translated: `\h` spelled out, possessive quantifiers made
// greedy (no character class here overlaps what follows it, so they match the same strings),
// `\g<ls32>` and friends inlined, and `(?!/)seg++` written as a first segment character without
// `/`. Group names are dropped except the ones read below.
const HEX: &str = "[0-9A-Fa-f]";

fn rfc3986_uri() -> String {
    let h16 = format!("{HEX}{{1,4}}");
    let dec_octet = "(?:[1-9][0-9]|1[0-9]{2}|2[0-4][0-9]|25[0-5]|[0-9])";
    let ipv4 = format!(r"{dec_octet}\.{dec_octet}\.{dec_octet}\.{dec_octet}");
    let ls32 = format!("(?:{h16}:{h16}|{ipv4})");
    let ipv6 = [
        format!("(?:{h16}:){{6}}{ls32}"),
        format!("::(?:{h16}:){{5}}{ls32}"),
        format!("(?:{h16})?::(?:{h16}:){{4}}{ls32}"),
        format!("(?:(?:{h16}:)?{h16})?::(?:{h16}:){{3}}{ls32}"),
        format!("(?:(?:{h16}:){{0,2}}{h16})?::(?:{h16}:){{2}}{ls32}"),
        format!("(?:(?:{h16}:){{0,3}}{h16})?::{h16}:{ls32}"),
        format!("(?:(?:{h16}:){{0,4}}{h16})?::{ls32}"),
        format!("(?:(?:{h16}:){{0,5}}{h16})?::{h16}"),
        format!("(?:(?:{h16}:){{0,6}}{h16})?::"),
    ]
    .join("|");
    let ipvfuture = format!(r"v{HEX}+\.[!$&-.0-9:;=A-Z_a-z~]+");
    let host = format!(r"\[(?:{ipv6}|{ipvfuture})\]|{ipv4}|(?:%{HEX}{HEX}|[!$&-.0-9;=A-Z_a-z~])*");
    let userinfo = format!("(?:%{HEX}{HEX}|[!$&-.0-9:;=A-Z_a-z~])*");
    let seg = format!("(?:%{HEX}{HEX}|[!$&-.0-9:;=@A-Z_a-z~/])");
    let seg_first = format!("(?:%{HEX}{HEX}|[!$&-.0-9:;=@A-Z_a-z~])");
    let fragment = format!("(?:%{HEX}{HEX}|[!$&-.0-9:;=@A-Z_a-z~/?])*");
    format!(
        r"\A(?<scheme>[A-Za-z][+\-.0-9A-Za-z]*):(?://(?:{userinfo}@)?(?:{host})(?::[0-9]*)?(?:/{seg}*)?|/(?:{seg_first}{seg}*)?|{seg_first}{seg}*|)(?:\?(?<query>[^#]*))?(?:#{fragment})?\z"
    )
}

static RFC3986_URI: LazyLock<Regex> = LazyLock::new(|| Regex::new(&rfc3986_uri()).expect("valid regex"));

/// `URI.parse(url).is_a?(URI::HTTP)`, without raising: `URI::InvalidURIError` is `false`.
pub(crate) fn is_http(url: &str) -> bool {
    if !url.is_ascii() {
        return false;
    }
    let Some(captures) = RFC3986_URI.captures(url) else { return false };
    let scheme = &captures["scheme"];
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")) {
        return false;
    }
    // `URI::Generic#query=` refuses a `%` followed by two non-hex characters.
    !captures.name("query").is_some_and(|query| {
        query.as_str().as_bytes().windows(3).any(|w| w[0] == b'%' && !w[1].is_ascii_hexdigit() && !w[2].is_ascii_hexdigit())
    })
}

/// Rooms::EventsHelper#safe_meet_link: URI::HTTPS with a nonblank host,
/// preserving Ruby's spelling while normalizing the scheme and default port.
pub fn safe_https(value: &str) -> Option<String> {
    if !is_http(value) { return None; }
    let (scheme,rest)=value.split_once(':')?;
    if !scheme.eq_ignore_ascii_case("https") {return None;}
    let rest=rest.strip_prefix("//")?;
    let end=rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority=&rest[..end];
    let (userinfo,host_port)=authority.rsplit_once('@').map_or(("",authority),|(user,host)|(user,host));
    let mut host=host_port;
    let mut port=String::new();
    if let Some((candidate,digits))=host_port.rsplit_once(':') && digits.bytes().all(|c|c.is_ascii_digit()) {
        host=candidate;
        if !digits.is_empty() {
            let number=digits.trim_start_matches('0');
            let number=if number.is_empty() {"0"} else {number};
            if number!="443" {port=format!(":{number}");}
        }
    }
    if host.is_empty() {return None;}
    let userinfo=if authority.contains('@') {format!("{userinfo}@")} else {String::new()};
    Some(format!("https://{userinfo}{host}{port}{}",&rest[end..]))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_and_https_only() {
        assert!(is_http("https://example.com/a.png"));
        assert!(is_http("HTTP://EXAMPLE.COM/"));
        assert!(!is_http("ftp://example.com/a.png"));
        assert!(!is_http("javascript:alert(1)"));
        assert!(!is_http("/relative.png"));
        assert!(!is_http("http://exa mple.com/"));
        assert!(!is_http("https://example.com/ü.png"));
    }

    /// Probed in the reference image (`uri` 1.0.4): non-ASCII anywhere is `InvalidURIError`, even
    /// in the query, which the regex alone would take.
    #[test]
    fn non_ascii_and_bad_escapes_in_the_query_are_refused() {
        assert!(!is_http("https://h/?ü"));
        assert!(!is_http("https://h/#ü"));
        assert!(!is_http("https://h/?a%zzb"));
        assert!(is_http("https://h/?%z"));
    }
}
