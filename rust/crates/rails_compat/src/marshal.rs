//! The sliver of Ruby's Marshal format needed to read Rails 7-era signed messages, whose payload
//! is a marshaled String (`Marshal.dump("gid://campfire/User/1")`). Anything else is rejected.

pub const SIGNATURE: &[u8] = b"\x04\x08";

/// Loads a marshaled String: `"\x04\x08" ["I"] '"' <len> <bytes> [<ivars>]`.
pub fn load_string(dumped: &[u8]) -> Option<Vec<u8>> {
    let rest = dumped.strip_prefix(SIGNATURE)?;
    let rest = rest.strip_prefix(b"I").unwrap_or(rest);
    let rest = rest.strip_prefix(b"\"")?;
    let (len, rest) = read_fixnum(rest)?;
    let len = usize::try_from(len).ok()?;
    rest.get(..len).map(<[u8]>::to_vec)
}

fn read_fixnum(bytes: &[u8]) -> Option<(i64, &[u8])> {
    let (&first, rest) = bytes.split_first()?;
    let first = first as i8;
    match first {
        0 => Some((0, rest)),
        1..=4 => {
            let n = first as usize;
            let digits = rest.get(..n)?;
            let value = digits.iter().rev().fold(0i64, |acc, &b| (acc << 8) | b as i64);
            Some((value, &rest[n..]))
        }
        -4..=-1 => {
            let n = (-first) as usize;
            let digits = rest.get(..n)?;
            let mut value = -1i64;
            for (i, &b) in digits.iter().enumerate() {
                value &= !(0xff << (8 * i));
                value |= (b as i64) << (8 * i);
            }
            Some((value, &rest[n..]))
        }
        5..=127 => Some((first as i64 - 5, rest)),
        _ => Some((first as i64 + 5, rest)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_strings() {
        // Marshal.dump("gid://campfire/User/1")
        let dumped = b"\x04\x08I\"\x1agid://campfire/User/1\x06:\x06ET";
        assert_eq!(load_string(dumped).unwrap(), b"gid://campfire/User/1");
        // A 300-byte string uses a two-byte length: "\x02\x2c\x01".
        let mut long = b"\x04\x08I\"\x02\x2c\x01".to_vec();
        long.extend(std::iter::repeat_n(b'x', 300));
        assert_eq!(load_string(&long).unwrap().len(), 300);
        assert_eq!(load_string(b"\x04\x08i\x06"), None);
    }
}
