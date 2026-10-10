//! Staged icon media checks from `app/models/workspace_icon.rb`.
//! Expat provides strict XML parsing; no entities or DTDs reach the parser.
use std::{
    ffi::{CStr, c_char, c_int, c_void},
    path::Path,
};

use crate::{Blob, Filename, Json, Storage, branding::PreparedEmoji, vips::Cancellation};

pub const ANIMATED_KEY: &str = "emoji_animated";
pub const MAX_BYTES: i64 = 256 * 1024;

pub fn animated(blob: &Blob) -> bool {
    matches!(blob.metadata.get(ANIMATED_KEY), Some(Json::Bool(true)))
}

/// Detect raster formats from bytes, and parse SVG as XML regardless of the upload's label.
pub fn prepare(
    storage: &Storage,
    key: &str,
    filename: &Filename,
    byte_size: i64,
    cancel: &Cancellation,
) -> Result<PreparedEmoji, String> {
    cancel.check().map_err(|_| "could not be read")?;
    let path = storage.service.path_for(key);
    let size = std::fs::metadata(&path)
        .map_err(|_| "could not be read")?
        .len();
    if byte_size > MAX_BYTES || size > MAX_BYTES as u64 {
        return Err("must be smaller than 256 KB".into());
    }
    if byte_size < 0 || size != byte_size as u64 {
        return Err("could not be read".into());
    }
    let bytes = std::fs::read(&path).map_err(|_| "could not be read")?;
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        // libvips ignores canvases above 2048 pixels and reports the frame size, so check the raw canvas.
        let canvas = bytes.get(6..10).ok_or("could not be read")?;
        let (max_width, max_height) = crate::branding::Kind::Emoji.maximum_dimensions();
        if i32::from(u16::from_le_bytes([canvas[0], canvas[1]])) > max_width
            || i32::from(u16::from_le_bytes([canvas[2], canvas[3]])) > max_height
        {
            return Err(crate::branding::Kind::Emoji.dimension_message().into());
        }
        let frames = gif_frames(&bytes).ok_or("could not be read")?;
        if frames > 100 {
            return Err("must contain at most 100 frames".into());
        }
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        let size = u32::from_le_bytes(bytes[4..8].try_into().expect("RIFF header"));
        if u64::from(size) + 8 != bytes.len() as u64 {
            return Err("could not be read".into());
        }
    }
    if bytes.starts_with(b"GIF87a")
        || bytes.starts_with(b"GIF89a")
        || (bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"))
    {
        return crate::branding::prepare_emoji(storage, key, filename, cancel);
    }
    let (content_type, mut metadata) = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        // Static PNGs retain the original header-only checks and have no canvas maximum.
        if let Some(error) = content_error(&path, "image/png") {
            return Err(error);
        }
        (
            "image/png",
            crate::analyze::Analyzer::Image
                .metadata(&path)
                .map_err(|_| "could not be read")?,
        )
    } else {
        if let Some(error) = svg_error(&bytes) {
            return Err(error.into());
        }
        ("image/svg+xml", Json::object())
    };
    metadata.set("identified", Json::Bool(true));
    metadata.set("analyzed", Json::Bool(true));
    metadata.set(ANIMATED_KEY, Json::Bool(false));
    Ok(PreparedEmoji {
        content_type,
        metadata,
        still: None,
    })
}

// libnsgif tolerates incomplete final frames, even with libvips' strict loader options.
// Require complete blocks and a trailer before asking the decoder to evaluate pixels.
fn gif_frames(bytes: &[u8]) -> Option<usize> {
    fn take<'a>(data: &mut &'a [u8], size: usize) -> Option<&'a [u8]> {
        let (head, tail) = data.split_at_checked(size)?;
        *data = tail;
        Some(head)
    }
    fn palette(data: &mut &[u8], packed: u8) -> Option<()> {
        if packed & 0x80 != 0 {
            take(data, 3 << ((packed & 7) + 1))?;
        }
        Some(())
    }
    fn blocks(data: &mut &[u8]) -> Option<()> {
        loop {
            let size = take(data, 1)?[0] as usize;
            if size == 0 {
                return Some(());
            }
            take(data, size)?;
        }
    }
    let header = bytes.get(..13)?;
    let width = u16::from_le_bytes(header[6..8].try_into().ok()?) as u32;
    let height = u16::from_le_bytes(header[8..10].try_into().ok()?) as u32;
    let mut data = &bytes[13..];
    palette(&mut data, header[10])?;
    let mut frames = 0;
    loop {
        match take(&mut data, 1)?[0] {
            b';' => return (data.is_empty() && frames > 0).then_some(frames),
            b'!' => {
                take(&mut data, 1)?;
                blocks(&mut data)?;
            }
            b',' => {
                let frame = take(&mut data, 9)?;
                let coordinate = |i| u16::from_le_bytes([frame[i], frame[i + 1]]) as u32;
                let (left, top, frame_width, frame_height) =
                    (coordinate(0), coordinate(2), coordinate(4), coordinate(6));
                if frame_width == 0
                    || frame_height == 0
                    || left + frame_width > width
                    || top + frame_height > height
                {
                    return None;
                }
                palette(&mut data, frame[8])?;
                if !(2..=8).contains(&take(&mut data, 1)?[0]) {
                    return None;
                }
                blocks(&mut data)?;
                frames += 1;
            }
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn gif_container_requires_every_frame_and_trailer() {
        let bytes = include_bytes!("../../../fixtures/files/workspace_icons/animated.gif");
        assert_eq!(super::gif_frames(bytes), Some(2));
        for end in 0..bytes.len() {
            assert_eq!(super::gif_frames(&bytes[..end]), None, "{end}");
        }
    }
}

pub fn content_error(path: &Path, content_type: &str) -> Option<String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return Some("could not be read".into()),
    };
    if bytes.is_empty() {
        return Some("could not be read".into());
    }
    match content_type {
        "image/svg+xml" => svg_error(&bytes).map(str::to_owned),
        "image/png" => match crate::vips::Image::open_sequential(path) {
            Ok(image) if image.width() != image.height() => Some("must be square".into()),
            Ok(image) if image.width() < 64 => {
                Some("must be at least 64 pixels wide and tall".into())
            }
            Ok(_) => None,
            Err(_) => Some("could not be read".into()),
        },
        _ => None,
    }
}
#[link(name = "expat")]
unsafe extern "C" {
    fn XML_ParserCreateNS(encoding: *const c_char, separator: c_char) -> *mut c_void;
    fn XML_ParserFree(parser: *mut c_void);
    fn XML_StopParser(parser: *mut c_void, resumable: c_int) -> c_int;
    fn XML_SetUserData(parser: *mut c_void, data: *mut c_void);
    fn XML_SetElementHandler(
        parser: *mut c_void,
        start: Option<unsafe extern "C" fn(*mut c_void, *const c_char, *const *const c_char)>,
        end: Option<unsafe extern "C" fn(*mut c_void, *const c_char)>,
    );
    fn XML_SetCharacterDataHandler(
        parser: *mut c_void,
        handler: Option<unsafe extern "C" fn(*mut c_void, *const c_char, c_int)>,
    );
    fn XML_Parse(
        parser: *mut c_void,
        bytes: *const c_char,
        len: c_int,
        final_chunk: c_int,
    ) -> c_int;
}
#[derive(Default)]
struct Node {
    name: String,
    namespace: Option<String>,
    attributes: Vec<(String, String)>,
    text: String,
    children: Vec<usize>,
}
#[derive(Default)]
struct Document {
    nodes: Vec<Node>,
    stack: Vec<usize>,
    parser: *mut c_void,
}
unsafe fn string(p: *const c_char) -> String {
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}
fn expanded(s: String) -> (Option<String>, String) {
    s.rsplit_once('\x1f')
        .map_or((None, s.clone()), |(ns, name)| {
            (Some(ns.into()), name.into())
        })
}
unsafe extern "C" fn start(
    data: *mut c_void,
    name: *const c_char,
    attributes: *const *const c_char,
) {
    // Expat owns all pointers for this synchronous callback; copied values outlive it.
    let doc = unsafe { &mut *data.cast::<Document>() };
    // libxml2/Nokogiri defaults to a maximum XML nesting depth of 256.
    if doc.stack.len() >= 256 {
        unsafe { XML_StopParser(doc.parser, 0) };
        return;
    }
    let (namespace, name) = expanded(unsafe { string(name) });
    let mut node = Node {
        name,
        namespace,
        ..Default::default()
    };
    let mut index = 0;
    while !unsafe { *attributes.add(index) }.is_null() {
        let (_, name) = expanded(unsafe { string(*attributes.add(index)) });
        let value = unsafe { string(*attributes.add(index + 1)) };
        node.attributes.push((name, value));
        index += 2;
    }
    let index = doc.nodes.len();
    if let Some(parent) = doc.stack.last() {
        doc.nodes[*parent].children.push(index);
    }
    doc.nodes.push(node);
    doc.stack.push(index);
}
unsafe extern "C" fn end(data: *mut c_void, _name: *const c_char) {
    unsafe { &mut *data.cast::<Document>() }.stack.pop();
}
unsafe extern "C" fn text(data: *mut c_void, bytes: *const c_char, len: c_int) {
    let doc = unsafe { &mut *data.cast::<Document>() };
    let text = String::from_utf8_lossy(unsafe {
        std::slice::from_raw_parts(bytes.cast::<u8>(), len as usize)
    });
    // Node#text includes descendant text.
    for index in &doc.stack {
        doc.nodes[*index].text.push_str(&text);
    }
}
fn svg_error(bytes: &[u8]) -> Option<&'static str> {
    let lowered = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    if ["<!doctype", "<!entity"].iter().any(|needle| {
        lowered.match_indices(needle).any(|(i, s)| {
            lowered
                .as_bytes()
                .get(i + s.len())
                .is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_')
        })
    }) {
        return Some("must not contain a DOCTYPE or entities");
    }
    let mut doc = Document::default();
    // Strict namespace parsing; external entities are impossible after the rejection above.
    let parser = unsafe { XML_ParserCreateNS(std::ptr::null(), 0x1f) };
    if parser.is_null() {
        return Some("is not a valid SVG");
    }
    doc.parser = parser;
    let valid = unsafe {
        XML_SetUserData(parser, (&mut doc as *mut Document).cast());
        XML_SetElementHandler(parser, Some(start), Some(end));
        XML_SetCharacterDataHandler(parser, Some(text));
        let valid = XML_Parse(parser, bytes.as_ptr().cast(), bytes.len() as c_int, 1) != 0;
        XML_ParserFree(parser);
        valid
    };
    if !valid
        || doc
            .nodes
            .first()
            .is_none_or(|n| !n.name.eq_ignore_ascii_case("svg"))
    {
        return Some("is not a valid SVG");
    }
    fn visit(doc: &Document, index: usize) -> Option<&'static str> {
        let node = &doc.nodes[index];
        // Nokogiri Document#traverse visits children before their parent.
        for child in &node.children {
            if let Some(error) = visit(doc, *child) {
                return Some(error);
            }
        }
        let name = node.name.to_lowercase();
        match name.as_str() {
            "script" => return Some("must not contain script elements"),
            "foreignobject" => return Some("must not contain foreignobject elements"),
            "image" => return Some("must not contain image elements"),
            "style" if node.text.to_ascii_lowercase().contains("url(") => {
                return Some("must not contain url() styles");
            }
            "svg"
                if index != 0
                    && node
                        .namespace
                        .as_deref()
                        .is_some_and(|ns| ns != "http://www.w3.org/2000/svg") =>
            {
                return Some("must not nest svg elements from another namespace");
            }
            _ => {}
        }
        for (name, value) in &node.attributes {
            let name = name.to_lowercase();
            if name.starts_with("on") {
                return Some("must not contain event handler attributes");
            }
            if name == "href"
                && !value
                    .trim_matches([' ', '\t', '\r', '\n', '\x0b', '\x0c', '\0'])
                    .starts_with('#')
            {
                return Some("must not contain external references");
            }
            if name == "style" && value.to_ascii_lowercase().contains("url(") {
                return Some("must not contain url() styles");
            }
        }
        None
    }
    visit(&doc, 0)
}
