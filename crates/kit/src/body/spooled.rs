//! DiskController still reads params before uploading request.body. Validate its spool
//! without allocating a JSON document or reading file parts into memory.
use std::io::{BufReader, Read, Seek};
use std::sync::LazyLock;

use axum::body::Body;
use axum::http::{HeaderMap, header};
use regex::Regex;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use super::{BodyError, MultipartPolicy, ParsedBody, parse_multipart};
use crate::{format, params, request::media_type};

pub(crate) async fn parse_spooled(
    headers: &HeaderMap,
    file: Option<&mut tokio::fs::File>,
) -> Result<ParsedBody, BodyError> {
    let Some(file) = file else {
        return Ok(ParsedBody::empty());
    };
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());
    let media = media_type(content_type);
    // Http::Parameters#parse_formatted_parameters skips JSON when content_length is zero.
    let length = if headers.contains_key(header::TRANSFER_ENCODING) {
        file.metadata().await?.len()
    } else {
        headers
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    };
    if length != 0 && format::content_mime_type(content_type).ok().flatten() == Some(&format::JSON)
    {
        let mut input = file.try_clone().await?.into_std().await;
        tokio::task::spawn_blocking(move || validate_json(&mut input))
            .await
            .map_err(std::io::Error::other)??;
        file.rewind().await?;
        return Ok(ParsedBody::empty());
    }
    let multipart = matches!(
        media.as_deref(),
        Some("multipart/form-data" | "multipart/related" | "multipart/mixed")
    );
    let boundary = if multipart && length != 0 {
        boundary(content_type.unwrap_or(""))?
    } else {
        None
    };
    let parsing_multipart = boundary.is_some();
    let parsed = if let Some(boundary) = boundary {
        // Rack rejects the declared total size before reading, even if it would
        // stop at a closing boundary before a large epilogue.
        if length > super::MULTIPART_BYTESIZE_LIMIT {
            return Err(multipart_error());
        }
        let mut input = file.try_clone().await?.into_std().await;
        let checked_boundary = boundary.clone();
        let position = tokio::task::spawn_blocking(move || {
            validate_multipart_buffers(&mut input, &checked_boundary)
        })
        .await
        .map_err(std::io::Error::other)??;
        // Multer accepts an initial closing marker even when Rack skips it while
        // looking for a later opening. Start at the boundary Rack actually accepted.
        file.seek(std::io::SeekFrom::Start(position.start)).await?;
        let input = file.try_clone().await?;
        let stream = futures_util::stream::try_unfold(input, |mut input| async move {
            let mut chunk = vec![0; 64 * 1024];
            let size = input.read(&mut chunk).await?;
            if size == 0 {
                return Ok::<_, std::io::Error>(None);
            }
            chunk.truncate(size);
            Ok(Some((chunk, input)))
        });
        let parsed = match parse_multipart(
            Body::from_stream(stream),
            boundary,
            None,
            MultipartPolicy::Disk,
        )
        .await
        {
            // Rack::Multipart::BoundaryTooLongError (also its byte-size error) maps to 500.
            Err(BodyError::TooLarge) => return Err(multipart_error()),
            other => other?,
        };
        // Rack reads 1 MiB at a time and stops at the closing boundary. Multer has
        // different read-ahead; restore Rack's position for DiskService's subsequent copy.
        file.seek(std::io::SeekFrom::Start(position.consumed))
            .await?;
        parsed
    } else if multipart || media.as_deref() == Some("application/x-www-form-urlencoded") {
        // Rack bounds urlencoded bodies at 4 MiB, even when the uploaded file is larger.
        let mut bytes = Vec::new();
        (&mut *file)
            .take((params::FORM_BYTESIZE_LIMIT + 2) as u64)
            .read_to_end(&mut bytes)
            .await?;
        ParsedBody {
            raw: Default::default(),
            params: params::disk_form_pairs(&bytes)
                .and_then(|pairs| params::from_pairs_with_depth_limit(pairs, 32)),
        }
    } else {
        return Ok(ParsedBody::empty());
    };
    if parsing_multipart && matches!(&parsed.params, Err(params::ParamError::Limit(_))) {
        // Multipart's part/file-count exceptions are not Rails ParamError rescues.
        return Err(multipart_error());
    }
    // Rack's form parsers consume rack.input; unlike JSON, they do not cache raw_post.
    // The HTTP oracle's Thruster has already decoded chunked framing before Puma.
    Ok(parsed)
}

fn multipart_error() -> BodyError {
    BodyError::Storage(std::io::Error::other("invalid multipart boundary or size"))
}

/// Rack::Multipart::Parser.parse_boundary, including its three header errors.
fn boundary(content_type: &str) -> Result<Option<String>, BodyError> {
    static BOUNDARY: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?i)^multipart/.*?boundary(\s*)="?([^";,]+)"?"#).unwrap());
    static DUPLICATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)boundary\s*=").unwrap());
    let Some(captures) = BOUNDARY.captures(content_type) else {
        return Ok(None);
    };
    let boundary = &captures[2];
    if !captures[1].is_empty()
        || boundary.len() > 70
        || DUPLICATE.is_match(&content_type[captures[0].len()..])
    {
        return Err(multipart_error());
    }
    Ok(Some(boundary.to_owned()))
}

/// Rack's accepted initial boundary and its final read-ahead position.
struct MultipartPosition {
    start: u64,
    consumed: u64,
}

/// Rack reads 1 MiB at a time and bounds unfinished preambles/headers. Multer
/// otherwise buffers an unterminated header indefinitely, so check before it parses.
fn validate_multipart_buffers<R: Read + Seek>(
    file: &mut R,
    boundary: &str,
) -> Result<MultipartPosition, BodyError> {
    enum State {
        Preamble,
        Headers,
        Body,
    }
    let first = format!("--{boundary}");
    let subsequent = format!("\r\n--{boundary}");
    let mut state = State::Preamble;
    let mut start = 0;
    let mut preamble_cursor = 0;
    let mut chunk = vec![0; 1024 * 1024];
    let mut buffered = Vec::new();
    loop {
        let count = read_full(file, &mut chunk)?;
        if count == 0 {
            // Rack's FAST_FORWARD keeps reading if it has not accepted an opening;
            // its EmptyContentError maps to 400, even if Multer accepts the close.
            if matches!(state, State::Preamble) {
                return Err(BodyError::Read("empty multipart content".into()));
            }
            return Ok(MultipartPosition {
                start,
                consumed: file.stream_position()?,
            });
        } // Other EOF syntax errors belong to the multipart parser.
        buffered.extend_from_slice(&chunk[..count]);
        loop {
            match state {
                State::Headers => {
                    if let Some(end) = memchr::memmem::find(&buffered, b"\r\n\r\n") {
                        buffered.drain(..end + 4);
                        state = State::Body;
                    } else {
                        if buffered.len() > 64 * 1024 {
                            return Err(multipart_error());
                        }
                        break;
                    }
                }
                State::Preamble => {
                    if let Some((end, closing)) =
                        find_initial_boundary(&buffered, first.as_bytes(), preamble_cursor)
                    {
                        preamble_cursor = end;
                        if closing {
                            // Rack::Multipart::Parser#handle_fast_forward: the sole
                            // closing-only exception is an exact first buffer plus CRLF.
                            if end == first.len() + 2 && buffered[end..] == *b"\r\n" {
                                return Ok(MultipartPosition {
                                    start: 0,
                                    consumed: file.stream_position()?,
                                });
                            }
                            continue;
                        }
                        start = (end - first.len() - 2) as u64;
                        buffered.drain(..end);
                        state = State::Headers;
                    } else {
                        if buffered.len() > 16 * 1024 {
                            return Err(multipart_error());
                        }
                        preamble_cursor = buffered.len();
                        break;
                    }
                }
                State::Body => {
                    let marker = subsequent.as_bytes();
                    if let Some((end, closing)) = find_boundary(&buffered, marker) {
                        if closing {
                            return Ok(MultipartPosition {
                                start,
                                consumed: file.stream_position()?,
                            });
                        }
                        buffered.drain(..end);
                        state = State::Headers;
                    } else {
                        let retained = buffered.len().min(marker.len() + 1);
                        buffered.drain(..buffered.len() - retained);
                        break;
                    }
                }
            }
        }
    }
}

/// Ruby's `IO#read(length)`: fills `buffer` unless the input ends first. A bare `read` may
/// return a short count, which would move the closing boundary into a later Rack buffer.
fn read_full<R: Read>(input: &mut R, buffer: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match input.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(count) => filled += count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(filled)
}

// Rack's strscan uses fixed_anchor: false: \A matches its current cursor, including
// immediately after a skipped close. Rejected candidates do not advance that anchor.
fn find_initial_boundary(bytes: &[u8], marker: &[u8], cursor: usize) -> Option<(usize, bool)> {
    let mut offset = cursor;
    while let Some((end, closing)) = find_boundary(&bytes[offset..], marker) {
        let end = offset + end;
        let start = end - marker.len() - 2;
        if start == cursor || (start >= 2 && bytes[start - 2..start] == *b"\r\n") {
            return Some((end, closing));
        }
        offset = start + marker.len();
    }
    None
}

fn find_boundary(bytes: &[u8], marker: &[u8]) -> Option<(usize, bool)> {
    let mut offset = 0;
    while let Some(index) = memchr::memmem::find(&bytes[offset..], marker) {
        let end = offset + index + marker.len();
        match bytes.get(end..end + 2) {
            Some(b"--") => return Some((end + 2, true)),
            Some(b"\r\n") => return Some((end + 2, false)),
            None => return None,
            _ => offset = end,
        }
    }
    None
}

fn validate_json(file: &mut std::fs::File) -> Result<(), BodyError> {
    // IgnoredAny skips strings/numbers without allocating them. It does not check UTF-8,
    // surrogate pairs or Ruby JSON's max_nesting=100, so check those in a bounded pass.
    let mut state = JsonState::default();
    let mut buffer = [0; 64 * 1024 + 3];
    let mut retained = 0;
    let mut input = JsonComments::new(&mut *file);
    loop {
        let count = input
            .read(&mut buffer[retained..])
            .map_err(json_read_error)?;
        if count == 0 {
            if retained != 0 {
                return Err(BodyError::Parse);
            }
            break;
        }
        let length = retained + count;
        let valid = match std::str::from_utf8(&buffer[..length]) {
            Ok(_) => length,
            Err(error) if error.error_len().is_none() => error.valid_up_to(),
            Err(_) => return Err(BodyError::Parse),
        };
        for &byte in &buffer[..valid] {
            state.byte(byte)?;
        }
        retained = length - valid;
        buffer.copy_within(valid..length, 0);
    }
    drop(input);
    file.rewind()?;
    let mut parser = serde_json::Deserializer::from_reader(BufReader::new(JsonComments::new(file)));
    serde::de::IgnoredAny::deserialize(&mut parser)
        .and_then(|_| parser.end())
        .map_err(|error| {
            if error.io_error_kind() == Some(std::io::ErrorKind::InvalidData) {
                BodyError::Parse
            } else if error.is_io() {
                BodyError::Storage(std::io::Error::other(error))
            } else {
                BodyError::Parse
            }
        })
}

#[derive(Default)]
struct JsonState {
    depth: u16,
    string: StringState,
}

#[derive(Default, Clone, Copy)]
enum StringState {
    #[default]
    Outside,
    Inside,
    Escape,
    Unicode {
        code: u16,
        digits: u8,
        low: bool,
    },
    LowSlash,
    LowU,
}

impl JsonState {
    fn byte(&mut self, byte: u8) -> Result<(), BodyError> {
        self.string = match self.string {
            StringState::Outside => match byte {
                b'"' => StringState::Inside,
                b'[' | b'{' => {
                    self.depth += 1;
                    if self.depth > 100 {
                        return Err(BodyError::Parse);
                    }
                    StringState::Outside
                }
                b']' | b'}' => {
                    self.depth = self.depth.saturating_sub(1);
                    StringState::Outside
                }
                _ => StringState::Outside,
            },
            StringState::Inside => match byte {
                b'"' => StringState::Outside,
                b'\\' => StringState::Escape,
                _ => StringState::Inside,
            },
            StringState::Escape => {
                if byte == b'u' {
                    StringState::Unicode {
                        code: 0,
                        digits: 0,
                        low: false,
                    }
                } else {
                    StringState::Inside
                }
            }
            StringState::Unicode { code, digits, low } => {
                let hex = (byte as char).to_digit(16).ok_or(BodyError::Parse)? as u16;
                let code = (code << 4) | hex;
                if digits < 3 {
                    StringState::Unicode {
                        code,
                        digits: digits + 1,
                        low,
                    }
                } else if low {
                    if !(0xdc00..=0xdfff).contains(&code) {
                        return Err(BodyError::Parse);
                    }
                    StringState::Inside
                } else if (0xd800..=0xdbff).contains(&code) {
                    StringState::LowSlash
                } else {
                    if (0xdc00..=0xdfff).contains(&code) {
                        return Err(BodyError::Parse);
                    }
                    StringState::Inside
                }
            }
            StringState::LowSlash => {
                if byte != b'\\' {
                    return Err(BodyError::Parse);
                }
                StringState::LowU
            }
            StringState::LowU => {
                if byte != b'u' {
                    return Err(BodyError::Parse);
                }
                StringState::Unicode {
                    code: 0,
                    digits: 0,
                    low: true,
                }
            }
        };
        Ok(())
    }
}

// Pinned json-2.21.2 accepts C/C++ comments by default. Replace them with whitespace
// for validation only; DiskController still uploads the original spool's bytes.
struct JsonComments<R> {
    input: std::io::Bytes<BufReader<R>>,
    quoted: bool,
    escaped: bool,
}

impl<R: Read> JsonComments<R> {
    fn new(input: R) -> Self {
        Self {
            input: BufReader::new(input).bytes(),
            quoted: false,
            escaped: false,
        }
    }

    fn byte(&mut self) -> std::io::Result<Option<u8>> {
        let Some(byte) = self.input.next().transpose()? else {
            return Ok(None);
        };
        if self.quoted {
            if self.escaped {
                self.escaped = false;
            } else if byte == b'\\' {
                self.escaped = true;
            } else if byte == b'"' {
                self.quoted = false;
            }
            return Ok(Some(byte));
        }
        if byte == b'"' {
            self.quoted = true;
        }
        if byte != b'/' {
            return Ok(Some(byte));
        }
        match self.input.next().transpose()? {
            Some(b'/') => {
                for byte in self.input.by_ref() {
                    if byte? == b'\n' {
                        break;
                    }
                }
            }
            Some(b'*') => {
                let mut star = false;
                loop {
                    let byte = self.input.next().transpose()?.ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "unterminated JSON comment",
                        )
                    })?;
                    if star && byte == b'/' {
                        break;
                    }
                    star = byte == b'*';
                }
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid JSON comment",
                ));
            }
        }
        Ok(Some(b' '))
    }
}

impl<R: Read> Read for JsonComments<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let mut count = 0;
        for byte in buffer {
            let Some(next) = self.byte()? else { break };
            *byte = next;
            count += 1;
        }
        Ok(count)
    }
}

fn json_read_error(error: std::io::Error) -> BodyError {
    if error.kind() == std::io::ErrorKind::InvalidData {
        BodyError::Parse
    } else {
        BodyError::Storage(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use std::io::Write;

    const PART: &str = "--B\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\nx\r\n--B--\r\n";

    async fn multipart(body: &str) -> Result<ParsedBody, BodyError> {
        let mut input = tempfile::tempfile().unwrap();
        input.write_all(body.as_bytes()).unwrap();
        input.rewind().unwrap();
        let mut file = tokio::fs::File::from_std(input);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            "multipart/form-data; boundary=B".parse().unwrap(),
        );
        headers.insert(
            header::CONTENT_LENGTH,
            body.len().to_string().parse().unwrap(),
        );
        let parsed = parse_spooled(&headers, Some(&mut file)).await?;
        assert_eq!(file.stream_position().await.unwrap(), body.len() as u64);
        Ok(parsed)
    }

    /// Returns at most `limit` bytes per read, like a file read that comes back short.
    struct ShortReads<R> {
        inner: R,
        limit: usize,
    }

    impl<R: Read> Read for ShortReads<R> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let limit = buffer.len().min(self.limit);
            self.inner.read(&mut buffer[..limit])
        }
    }

    impl<R: Seek> Seek for ShortReads<R> {
        fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(position)
        }
    }

    #[test]
    fn multipart_positions_follow_rack_buffers_despite_short_reads() {
        // The closing boundary ends inside Rack's first 1 MiB buffer, one byte before the
        // epilogue that follows it; Rack's position is the end of that first buffer.
        let head = "--B\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n";
        let tail = "\r\n--B--\r\n";
        let mut body = head.as_bytes().to_vec();
        body.resize(1024 * 1024 - tail.len(), b'x');
        body.extend_from_slice(tail.as_bytes());
        body.push(b'z');
        for limit in [usize::MAX, 64 * 1024, 4096, 7] {
            let mut input = ShortReads {
                inner: std::io::Cursor::new(&body),
                limit,
            };
            let position = validate_multipart_buffers(&mut input, "B").unwrap();
            assert_eq!(position.start, 0, "{limit}");
            assert_eq!(position.consumed, 1024 * 1024, "{limit}");
        }
    }

    #[tokio::test]
    async fn multipart_adjacent_closing_markers_parse_the_part() {
        // Rack's strscan \A is relative to the cursor after each skipped close.
        for prefix in ["--B--", "--B----B--", "preamble\r\n--B--"] {
            let parsed = multipart(&format!("{prefix}{PART}")).await.unwrap();
            assert_eq!(parsed.params.unwrap().str("a"), Some("x"), "{prefix:?}");
        }
    }

    #[tokio::test]
    async fn multipart_initial_close_separators_match_rack() {
        for separator in ["\n", " ", "\t", "\r\n "] {
            let error = multipart(&format!("--B--{separator}{PART}"))
                .await
                .unwrap_err();
            assert_eq!(error.status(), StatusCode::BAD_REQUEST, "{separator:?}");
        }
    }

    #[tokio::test]
    async fn multipart_rejected_candidates_do_not_move_the_anchor() {
        let error = multipart(&format!("--B-- --B{PART}")).await.unwrap_err();
        assert_eq!(error.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn multipart_adjacent_markers_still_validate_parts_and_limits() {
        let truncated = PART.strip_suffix("--B--\r\n").unwrap();
        assert_eq!(
            multipart(&format!("--B--{truncated}"))
                .await
                .unwrap()
                .params
                .unwrap_err(),
            params::ParamError::Parse,
        );
        let part =
            "--B\r\nContent-Disposition: form-data; name=\"a[]\"; filename=\"x.bin\"\r\n\r\nx\r\n";
        assert_eq!(
            multipart(&format!("--B--{}--B--\r\n", part.repeat(128)))
                .await
                .unwrap_err()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR,
        );
    }
}
