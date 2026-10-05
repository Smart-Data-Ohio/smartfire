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
        file.rewind().await?;
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
        file.seek(std::io::SeekFrom::Start(position)).await?;
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

/// Rack reads 1 MiB at a time and bounds unfinished preambles/headers. Multer
/// otherwise buffers an unterminated header indefinitely, so check before it parses.
fn validate_multipart_buffers(file: &mut std::fs::File, boundary: &str) -> Result<u64, BodyError> {
    enum State {
        Preamble,
        Headers,
        Body,
    }
    let first = format!("--{boundary}");
    let subsequent = format!("\r\n--{boundary}");
    let mut state = State::Preamble;
    let mut chunk = vec![0; 1024 * 1024];
    let mut buffered = Vec::new();
    loop {
        let count = file.read(&mut chunk)?;
        if count == 0 {
            return Ok(file.stream_position()?);
        } // EOF syntax errors belong to the multipart parser.
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
                State::Preamble | State::Body => {
                    let marker = if matches!(state, State::Preamble) {
                        first.as_bytes()
                    } else {
                        subsequent.as_bytes()
                    };
                    if let Some((end, closing)) = find_boundary(&buffered, marker) {
                        if closing {
                            return Ok(file.stream_position()?);
                        }
                        buffered.drain(..end);
                        state = State::Headers;
                    } else {
                        if matches!(state, State::Preamble) {
                            if buffered.len() > 16 * 1024 {
                                return Err(multipart_error());
                            }
                        } else {
                            let retained = buffered.len().min(marker.len() + 1);
                            buffered.drain(..buffered.len() - retained);
                        }
                        break;
                    }
                }
            }
        }
    }
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
