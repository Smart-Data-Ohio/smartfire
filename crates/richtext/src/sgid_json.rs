//! The one-shot json 2.21.2 parser used by Message::MentionPreloader.gid_uri_for_sgid.
//! This ports ext/json/ext/parser/parser.c's byte cursor, error order and error fragments,
//! not serde_json's grammar/error positions. Binary strings are reinterpreted as UTF-8;
//! Ruby does not validate raw string bytes. See data/JSON-PARSER-LICENSE.

#[derive(Debug)]
pub(crate) enum Value {
    Null,
    False,
    Other,
    String(Vec<u8>),
    Object(Vec<(Vec<u8>, Value)>),
}

impl Value {
    pub(crate) fn field(&self, key: &[u8]) -> Option<&Self> {
        match self {
            Self::Object(fields) => fields.iter().rev().find(|(name, _)| name == key).map(|(_, value)| value),
            _ => None,
        }
    }

    pub(crate) fn truthy(&self) -> bool {
        !matches!(self, Self::Null | Self::False)
    }
}

#[derive(Debug)]
pub(crate) struct ParseError {
    pub(crate) class: &'static str,
    pub(crate) message: Vec<u8>,
}

impl ParseError {
    pub(crate) fn unloggable(&self) -> bool {
        std::str::from_utf8(&self.message).is_err()
    }
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Value, ParseError> {
    let mut parser = Parser { bytes, cursor: 0, nesting: 0 };
    let value = parser.value()?;
    parser.whitespace()?;
    if parser.cursor != bytes.len() {
        return Err(parser.error("unexpected token at end of stream %s", parser.cursor));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    cursor: usize,
    nesting: usize,
}

impl Parser<'_> {
    fn peek(&self) -> u8 {
        self.bytes.get(self.cursor).copied().unwrap_or(0)
    }

    fn error(&self, format: &str, cursor: usize) -> ParseError {
        let mut message = Vec::new();
        if let Some((before, after)) = format.split_once("%s") {
            message.extend_from_slice(before.as_bytes());
            // build_parse_error_message: a 32-byte word, trimmed exactly as the C buffer.
            // A leading whitespace/NUL leaves ptr unquoted (a raw NUL-terminated C string).
            let rest = &self.bytes[cursor..];
            if rest.is_empty() {
                message.extend_from_slice(b"EOF");
            } else {
                let mut len = rest.iter().take(32).position(|b| b"\0\n \t\r".contains(b)).unwrap_or(rest.len().min(32));
                if len == 0 {
                    let end = rest.iter().position(|b| *b == 0).unwrap_or(rest.len());
                    message.extend_from_slice(&rest[..end]);
                } else {
                    while len > 0 && (0x80..0xc0).contains(&rest[len - 1]) {
                        len -= 1;
                    }
                    if len > 0 && rest[len - 1] >= 0xc0 {
                        len -= 1;
                    }
                    message.push(b'\'');
                    message.extend_from_slice(&rest[..len]);
                    message.push(b'\'');
                }
            }
            message.extend_from_slice(after.as_bytes());
        } else {
            message.extend_from_slice(format.as_bytes());
        }
        // cursor_position includes a newline at the cursor, giving column zero.
        let prefix = &self.bytes[..(cursor + 1).min(self.bytes.len())];
        let line = prefix.iter().filter(|b| **b == b'\n').count() + 1;
        let column = prefix.iter().rposition(|b| *b == b'\n').map_or(cursor + 1, |last| cursor - last);
        message.extend_from_slice(format!(" at line {line} column {column}").as_bytes());
        ParseError { class: "JSON::ParserError", message }
    }

    fn whitespace(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                b' ' | b'\n' | b'\r' | b'\t' => self.cursor += 1,
                b'/' => {
                    let start = self.cursor;
                    self.cursor += 1;
                    match self.peek() {
                        b'/' => {
                            // One-shot JSON.parse accepts line comments without a final newline.
                            self.cursor = self.bytes[self.cursor..]
                                .iter()
                                .position(|b| *b == b'\n')
                                .map_or(self.bytes.len(), |offset| self.cursor + offset + 1);
                        }
                        b'*' => {
                            self.cursor += 1;
                            if let Some(offset) = self.bytes[self.cursor..].windows(2).position(|pair| pair == b"*/") {
                                self.cursor += offset + 2;
                            } else {
                                return Err(self.error("unterminated comment, expected closing '*/'", start));
                            }
                        }
                        _ => return Err(self.error("unexpected token %s", start)),
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn value(&mut self) -> Result<Value, ParseError> {
        self.whitespace()?;
        match self.peek() {
            b'n' => {
                self.keyword(b"null")?;
                Ok(Value::Null)
            }
            b't' => {
                self.keyword(b"true")?;
                Ok(Value::Other)
            }
            b'f' => {
                self.keyword(b"false")?;
                Ok(Value::False)
            }
            b'N' | b'I' => Err(self.error("unexpected token %s", self.cursor)),
            b'-' | b'0'..=b'9' => self.number(),
            b'"' => self.string().map(Value::String),
            b'[' | b'{' => self.container(),
            0 if self.cursor == self.bytes.len() => Err(self.error("unexpected end of input", self.cursor)),
            0 => Err(self.error("unexpected NULL byte: %s", self.cursor)),
            _ => Err(self.error("unexpected character: %s", self.cursor)),
        }
    }

    fn keyword(&mut self, keyword: &[u8]) -> Result<(), ParseError> {
        if self.bytes[self.cursor..].starts_with(keyword) {
            self.cursor += keyword.len();
            Ok(())
        } else {
            Err(self.error("unexpected token %s", self.cursor))
        }
    }

    fn number(&mut self) -> Result<Value, ParseError> {
        let start = self.cursor;
        let negative = self.peek() == b'-';
        if negative {
            self.cursor += 1;
        }
        let first = self.peek();
        let digits = self.digits();
        if (first == b'0' && digits > 1) || (negative && digits == 0) {
            return Err(self.error("invalid number: %s", start));
        }
        if self.peek() == b'.' {
            self.cursor += 1;
            if self.digits() == 0 {
                return Err(self.error("invalid number: %s", start));
            }
        }
        if matches!(self.peek(), b'e' | b'E') {
            self.cursor += 1;
            if matches!(self.peek(), b'-' | b'+') {
                self.cursor += 1;
            }
            if self.digits() == 0 {
                return Err(self.error("invalid number: %s", start));
            }
        }
        // SGID lookup only observes the type/truthiness of numbers, never their magnitude.
        // Ruby accepts arbitrarily large integers and floating-point overflow here.
        Ok(Value::Other)
    }

    fn digits(&mut self) -> usize {
        let start = self.cursor;
        while self.peek().is_ascii_digit() {
            self.cursor += 1;
        }
        self.cursor - start
    }

    fn container(&mut self) -> Result<Value, ParseError> {
        let object = self.peek() == b'{';
        self.cursor += 1;
        self.whitespace()?;
        let close = if object { b'}' } else { b']' };
        let mut fields = Vec::new();
        // Empty containers don't increment the gem's nesting count.
        if self.peek() == close {
            self.cursor += 1;
            return Ok(if object { Value::Object(fields) } else { Value::Other });
        }
        self.nesting += 1;
        if self.nesting > 100 {
            return Err(ParseError {
                class: "JSON::NestingError",
                message: format!("nesting of {} is too deep", self.nesting).into_bytes(),
            });
        }
        loop {
            if object {
                self.whitespace()?;
                if self.peek() != b'"' {
                    return Err(self.error(
                        if fields.is_empty() { "expected object key, got %s" } else { "expected object key, got: %s" },
                        self.cursor,
                    ));
                }
                let name = self.string()?;
                self.whitespace()?;
                if self.peek() != b':' {
                    return Err(self.error(
                        if fields.is_empty() { "expected ':' after object key" } else { "expected ':' after object key, got: %s" },
                        self.cursor,
                    ));
                }
                self.cursor += 1;
                fields.push((name, self.value()?));
            } else {
                self.value()?;
            }
            self.whitespace()?;
            if self.peek() == close {
                self.cursor += 1;
                self.nesting -= 1;
                return Ok(if object { Value::Object(fields) } else { Value::Other });
            }
            if self.peek() != b',' {
                return Err(self.error(
                    if object { "expected ',' or '}' after object value, got: %s" } else { "expected ',' or ']' after array value" },
                    self.cursor,
                ));
            }
            self.cursor += 1;
        }
    }

    fn string(&mut self) -> Result<Vec<u8>, ParseError> {
        self.cursor += 1;
        let start = self.cursor;
        let mut escapes = Vec::new();
        // Scan the whole string before unescaping. Raw control errors therefore precede
        // earlier invalid escapes; an unterminated string never reaches unescaping.
        while self.cursor < self.bytes.len() {
            match self.peek() {
                b'"' => {
                    let end = self.cursor;
                    let result = self.unescape(start, end, &escapes);
                    self.cursor += 1;
                    return result;
                }
                b'\\' => {
                    escapes.push(self.cursor);
                    self.cursor = (self.cursor + 2).min(self.bytes.len());
                }
                0..=0x1f => return Err(self.error("invalid ASCII control character in string: %s", self.cursor)),
                _ => self.cursor += 1,
            }
        }
        Err(self.error("unexpected end of input, expected closing \"", self.cursor))
    }

    fn hex4(&self, start: usize, end: usize) -> Result<u32, ParseError> {
        if start + 4 <= end {
            let mut result = 0;
            for &byte in &self.bytes[start..start + 4] {
                if let Some(digit) = (byte as char).to_digit(16) {
                    result = result * 16 + digit;
                } else {
                    return Err(self.error("incomplete unicode character escape sequence at %s", start - 2));
                }
            }
            return Ok(result);
        }
        Err(self.error("incomplete unicode character escape sequence at %s", start - 2))
    }

    fn unescape(&self, start: usize, end: usize, escapes: &[usize]) -> Result<Vec<u8>, ParseError> {
        let mut out = Vec::new();
        let mut chunk = start;
        for &slash in escapes {
            if slash < chunk {
                continue;
            } // The second half of a surrogate pair was consumed.
            out.extend_from_slice(&self.bytes[chunk..slash]);
            let escaped = slash + 1;
            match self.bytes[escaped] {
                b'"' | b'/' => chunk = escaped,
                b'\\' | b'n' | b'r' | b't' | b'b' | b'f' => {
                    out.push(match self.bytes[escaped] {
                        b'n' => b'\n',
                        b'r' => b'\r',
                        b't' => b'\t',
                        b'b' => 8,
                        b'f' => 12,
                        _ => b'\\',
                    });
                    chunk = escaped + 1;
                }
                b'u' => {
                    let mut codepoint = self.hex4(slash + 2, end)?;
                    let mut next = slash + 6;
                    if codepoint & 0xfc00 == 0xd800 {
                        if next + 6 > end || &self.bytes[next..next + 2] != b"\\u" {
                            return Err(self.error("incomplete surrogate pair at %s", chunk));
                        }
                        let low = self.hex4(next + 2, end)?;
                        if low & 0xfc00 != 0xdc00 {
                            return Err(self.error("invalid surrogate pair at %s", chunk));
                        }
                        codepoint = 0x10000 + ((codepoint & 0x3ff) << 10) + (low & 0x3ff);
                        next += 6;
                    }
                    // The gem encodes lone low surrogates as invalid UTF-8 rather than rejecting.
                    if codepoint <= 0x7f {
                        out.push(codepoint as u8);
                    } else if codepoint <= 0x7ff {
                        out.extend_from_slice(&[(0xc0 | codepoint >> 6) as u8, (0x80 | codepoint & 63) as u8]);
                    } else if codepoint <= 0xffff {
                        out.extend_from_slice(&[
                            (0xe0 | codepoint >> 12) as u8,
                            (0x80 | codepoint >> 6 & 63) as u8,
                            (0x80 | codepoint & 63) as u8,
                        ]);
                    } else {
                        out.extend_from_slice(&[
                            (0xf0 | codepoint >> 18) as u8,
                            (0x80 | codepoint >> 12 & 63) as u8,
                            (0x80 | codepoint >> 6 & 63) as u8,
                            (0x80 | codepoint & 63) as u8,
                        ]);
                    }
                    chunk = next;
                }
                0 => return Err(self.error("unexpected end of input, expected closing \"", end + 1)),
                b'\n' => return Err(self.error("Invalid unescaped newline character (\\n) in string: %s", slash)),
                1..=0x1f => return Err(self.error("invalid ASCII control character in string: %s", slash)),
                _ => return Err(self.error("invalid escape character in string: %s", slash)),
            }
        }
        out.extend_from_slice(&self.bytes[chunk..end]);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn malformed_json_error_encoding_matches_the_ruby_gem() {
        let cases: serde_json::Value = serde_json::from_str(include_str!("../tests/corpus/json-errors.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let decoded = base64::engine::general_purpose::STANDARD.decode(case["payload"].as_str().unwrap()).unwrap();
            let error = parse(&decoded).unwrap_err();
            assert_eq!(error.unloggable(), case["unloggable"].as_bool().unwrap(), "{}", case["name"]);
        }
    }

    #[test]
    fn json_error_bytes_match_the_helper_corpus() {
        let corpus: serde_json::Value = serde_json::from_str(include_str!("../tests/corpus/sgid-json.json")).unwrap();
        let mut failures = Vec::new();
        for case in corpus["cases"].as_array().unwrap() {
            let payload = base64::engine::general_purpose::STANDARD.decode(case["payload"].as_str().unwrap()).unwrap();
            let actual = parse(&payload);
            let expected = &case["parser"];
            match actual {
                Ok(_) if expected["status"] == "valid" => {}
                Err(error) if expected["status"] == "error" => {
                    let message = base64::engine::general_purpose::STANDARD.decode(expected["message"].as_str().unwrap()).unwrap();
                    if error.class != expected["class"].as_str().unwrap() || error.message != message {
                        failures.push(format!(
                            "{}: {} {:?} != {} {:?}",
                            case["name"],
                            error.class,
                            String::from_utf8_lossy(&error.message),
                            expected["class"],
                            String::from_utf8_lossy(&message)
                        ));
                    }
                }
                actual => failures.push(format!("{}: {actual:?} != {expected}", case["name"])),
            }
        }
        println!(
            "Ruby JSON parser corpus: {} cases, {} differences (error class and raw message bytes)",
            corpus["cases"].as_array().unwrap().len(),
            failures.len()
        );
        assert!(failures.is_empty(), "{}", failures.iter().take(30).cloned().collect::<Vec<_>>().join("\n"));
    }
}
