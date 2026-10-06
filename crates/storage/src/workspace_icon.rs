//! Staged icon media checks from `app/models/workspace_icon.rb`.
//! Expat provides strict XML parsing; no entities or DTDs reach the parser.
use std::{
    ffi::{CStr, c_char, c_int, c_void},
    path::Path,
};

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
