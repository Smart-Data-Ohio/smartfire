//! `QrCodeController` (reference/app/controllers/qr_code_controller.rb): a QR code SVG for a
//! Base64url-encoded URL.

mod rqrcode;

use campfire_kit::{Ctx, Error, ExpiresIn, Result, StatusCode};

use crate::concerns::{self, Before};

/// `allow_unauthenticated_access`
pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    // `Base64.urlsafe_decode64(params[:id])` raises ArgumentError (a 500) on malformed input.
    let id = c.param_str("id").unwrap_or_default().to_string();
    let url = urlsafe_decode64(&id).ok_or_else(|| Error::internal(anyhow::anyhow!("invalid base64")))?;
    // Too much to encode is the client's doing (rqrcode raises, a 500 in Rails).
    let qr_code = rqrcode::svg_bytes(&url).ok_or(Error::Status(StatusCode::UNPROCESSABLE_ENTITY))?;

    // `expires_in 1.year, public: true`
    c.expires_in(31_556_952, ExpiresIn { public: true, ..ExpiresIn::default() });
    Ok(c.render_as(StatusCode::OK, "image/svg+xml; charset=utf-8", qr_code))
}

/// Ruby's `Base64.urlsafe_decode64`: pad unpadded input, map `-_` to `+/`, then
/// `strict_decode64`, which rejects bad characters, bad lengths, misplaced padding and non-zero
/// leftover bits (`None` where Ruby raises ArgumentError).
fn urlsafe_decode64(input: &str) -> Option<Vec<u8>> {
    let mut string = input.to_string();
    if !string.ends_with('=') && !string.len().is_multiple_of(4) {
        while !string.len().is_multiple_of(4) {
            string.push('=');
        }
    }
    let string = string.replace('-', "+").replace('_', "/");
    let bytes = string.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let value = |byte: u8| -> Option<u32> {
        match byte {
            b'A'..=b'Z' => Some((byte - b'A') as u32),
            b'a'..=b'z' => Some((byte - b'a' + 26) as u32),
            b'0'..=b'9' => Some((byte - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for (index, chunk) in bytes.chunks(4).enumerate() {
        let last = index == bytes.len() / 4 - 1;
        let (a, b) = (value(chunk[0])?, value(chunk[1])?);
        match (chunk[2], chunk[3]) {
            (b'=', b'=') if last => {
                if b & 0xf != 0 {
                    return None;
                }
                out.push((a << 2 | b >> 4) as u8);
            }
            (c, b'=') if last => {
                let c = value(c)?;
                if c & 0x3 != 0 {
                    return None;
                }
                out.push((a << 2 | b >> 4) as u8);
                out.push(((b & 0xf) << 4 | c >> 2) as u8);
            }
            (c, d) => {
                let (c, d) = (value(c)?, value(d)?);
                out.push((a << 2 | b >> 4) as u8);
                out.push(((b & 0xf) << 4 | c >> 2) as u8);
                out.push(((c & 0x3) << 6 | d) as u8);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_like_ruby_urlsafe_decode64() {
        assert_eq!(urlsafe_decode64("aHR0cDovL2NhbXBmaXJlLnRlc3Q").unwrap(), b"http://campfire.test");
        assert_eq!(urlsafe_decode64("aHR0cDovL2NhbXBmaXJlLnRlc3Q=").unwrap(), b"http://campfire.test");
        assert_eq!(urlsafe_decode64("-_8").unwrap(), vec![0xfb, 0xff]);
        assert_eq!(urlsafe_decode64(""), Some(vec![]));
        assert_eq!(urlsafe_decode64("a"), None);
        assert_eq!(urlsafe_decode64("ab=c"), None);
        assert_eq!(urlsafe_decode64("aB=="), None);
        assert_eq!(urlsafe_decode64("a*bc"), None);
    }
}
