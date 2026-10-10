//! A small arena DOM that behaves like the Nokogiri HTML5 (Gumbo) trees Action Text works on.
//!
//! Parsing goes through html5ever, which implements the same WHATWG tree construction
//! algorithm as Gumbo. Serialization reproduces Nokogiri's `html_standard_serialize`
//! (`Nokogiri::HTML5::Node#write_to`), which is what `ActionText::HtmlConversion.node_to_html`
//! and Loofah's `to_html` produce for HTML5 documents.

use std::borrow::Cow;
use std::cell::RefCell;

use html5ever::interface::{ElemName, ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::{Attribute, LocalName, Namespace, ParseOpts, QualName, local_name, ns};

pub type NodeId = usize;

/// Gumbo's defaults, which Nokogiri raises `ArgumentError` for exceeding
/// (`Nokogiri::Gumbo::DEFAULT_MAX_TREE_DEPTH`, `DEFAULT_MAX_ATTRIBUTES`).
pub const MAX_TREE_DEPTH: usize = 400;
pub const MAX_ATTRIBUTES: usize = 400;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("Document tree depth limit exceeded")]
    TreeDepthExceeded,
    #[error("Attributes per element limit exceeded")]
    TooManyAttributes,
}

#[derive(Debug, Clone)]
pub struct Attr {
    pub name: QualName,
    pub value: String,
}

impl Attr {
    /// The attribute's name as Nokogiri reports and serializes it ("xlink:href", "href").
    pub fn qualified_name(&self) -> String {
        match &self.name.prefix {
            Some(prefix) => format!("{}:{}", prefix, self.name.local),
            None => self.name.local.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ElementData {
    pub name: QualName,
    pub attrs: Vec<Attr>,
}

#[derive(Debug, Clone)]
pub enum NodeData {
    Document,
    Fragment,
    Element(ElementData),
    Text(String),
    Comment(String),
    Doctype(String),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub data: NodeData,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

#[derive(Debug, Default, Clone)]
pub struct Dom {
    nodes: Vec<Node>,
}

/// The context a fragment is parsed in: Nokogiri parses `Document#fragment` in a `body`
/// context and `Node#fragment` (used by `replace`, `inner_html=`) in the node's own context.
#[derive(Debug, Clone)]
pub struct Context {
    pub name: QualName,
}

impl Context {
    pub fn body() -> Self {
        Self::html("body")
    }

    pub fn html(local: &str) -> Self {
        Context { name: QualName::new(None, ns!(html), LocalName::from(local)) }
    }
}

impl Dom {
    pub fn new() -> Self {
        Dom { nodes: Vec::new() }
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    fn push(&mut self, data: NodeData) -> NodeId {
        self.nodes.push(Node { data, parent: None, children: Vec::new() });
        self.nodes.len() - 1
    }

    pub fn new_fragment(&mut self) -> NodeId {
        self.push(NodeData::Fragment)
    }

    pub fn create_element(&mut self, local: &str, attrs: &[(&str, &str)]) -> NodeId {
        let attrs = attrs
            .iter()
            .map(|(k, v)| Attr { name: QualName::new(None, ns!(), LocalName::from(*k)), value: v.to_string() })
            .collect();
        self.push(NodeData::Element(ElementData {
            name: QualName::new(None, ns!(html), LocalName::from(local)),
            attrs,
        }))
    }

    pub fn create_text(&mut self, text: &str) -> NodeId {
        self.push(NodeData::Text(text.to_string()))
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        &self.nodes[id].children
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes[id].parent
    }

    pub fn element(&self, id: NodeId) -> Option<&ElementData> {
        match &self.nodes[id].data {
            NodeData::Element(e) => Some(e),
            _ => None,
        }
    }

    pub fn element_mut(&mut self, id: NodeId) -> Option<&mut ElementData> {
        match &mut self.nodes[id].data {
            NodeData::Element(e) => Some(e),
            _ => None,
        }
    }

    pub fn is_element(&self, id: NodeId) -> bool {
        matches!(self.nodes[id].data, NodeData::Element(_))
    }

    pub fn is_text(&self, id: NodeId) -> bool {
        matches!(self.nodes[id].data, NodeData::Text(_))
    }

    /// Nokogiri's `Node#name`: the local name for elements, "text", "comment" and so on otherwise.
    pub fn name(&self, id: NodeId) -> Cow<'_, str> {
        match &self.nodes[id].data {
            NodeData::Element(e) => Cow::Borrowed(&*e.name.local),
            NodeData::Text(_) => Cow::Borrowed("text"),
            NodeData::Comment(_) => Cow::Borrowed("comment"),
            NodeData::Fragment => Cow::Borrowed("#document-fragment"),
            NodeData::Document => Cow::Borrowed("document"),
            NodeData::Doctype(n) => Cow::Owned(n.clone()),
        }
    }

    /// The element's local name, if `id` is an element.
    pub fn local_name(&self, id: NodeId) -> Option<&str> {
        self.element(id).map(|e| &*e.name.local)
    }

    /// Keeps the element's attributes and children, and changes its tag.
    pub fn set_local_name(&mut self, id: NodeId, local: &str) {
        if let Some(element) = self.element_mut(id) {
            element.name.local = LocalName::from(local);
        }
    }

    pub fn is_html_element(&self, id: NodeId) -> bool {
        self.element(id).is_some_and(|e| e.name.ns == ns!(html))
    }

    pub fn attr(&self, id: NodeId, name: &str) -> Option<&str> {
        self.element(id)?.attrs.iter().find(|a| a.qualified_name() == name).map(|a| a.value.as_str())
    }

    pub fn has_attr(&self, id: NodeId, name: &str) -> bool {
        self.attr(id, name).is_some()
    }

    /// Nokogiri's `node[name] = value`: updates in place, or appends a new attribute.
    pub fn set_attr(&mut self, id: NodeId, name: &str, value: &str) {
        if let Some(element) = self.element_mut(id) {
            if let Some(attr) = element.attrs.iter_mut().find(|a| a.qualified_name() == name) {
                attr.value = value.to_string();
            } else {
                element.attrs.push(Attr {
                    name: QualName::new(None, ns!(), LocalName::from(name)),
                    value: value.to_string(),
                });
            }
        }
    }

    pub fn remove_attr(&mut self, id: NodeId, name: &str) -> Option<String> {
        let element = self.element_mut(id)?;
        let index = element.attrs.iter().position(|a| a.qualified_name() == name)?;
        Some(element.attrs.remove(index).value)
    }

    pub fn attrs(&self, id: NodeId) -> Vec<(String, String)> {
        self.element(id)
            .map(|e| e.attrs.iter().map(|a| (a.qualified_name(), a.value.clone())).collect())
            .unwrap_or_default()
    }

    /// Element children only (Nokogiri's `Node#elements`).
    pub fn element_children(&self, id: NodeId) -> Vec<NodeId> {
        self.children(id).iter().copied().filter(|&c| self.is_element(c)).collect()
    }

    /// All descendants in document order, excluding `id` itself.
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack: Vec<NodeId> = self.children(id).iter().rev().copied().collect();
        while let Some(n) = stack.pop() {
            out.push(n);
            stack.extend(self.children(n).iter().rev().copied());
        }
        out
    }

    pub fn ancestors(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut current = self.parent(id);
        while let Some(p) = current {
            out.push(p);
            current = self.parent(p);
        }
        out
    }

    pub fn detach(&mut self, id: NodeId) {
        if let Some(parent) = self.nodes[id].parent.take() {
            self.nodes[parent].children.retain(|&c| c != id);
        }
    }

    pub fn append(&mut self, parent: NodeId, child: NodeId) {
        self.detach(child);
        self.nodes[child].parent = Some(parent);
        self.nodes[parent].children.push(child);
    }

    pub fn insert_before(&mut self, reference: NodeId, new: NodeId) {
        self.detach(new);
        let parent = self.nodes[reference].parent.expect("reference node has a parent");
        let index = self.nodes[parent].children.iter().position(|&c| c == reference).unwrap();
        self.nodes[new].parent = Some(parent);
        self.nodes[parent].children.insert(index, new);
    }

    /// The text content of a node, as libxml2's `xmlNodeGetContent` computes it.
    pub fn text_content(&self, id: NodeId) -> String {
        match &self.nodes[id].data {
            NodeData::Text(t) | NodeData::Comment(t) => t.clone(),
            _ => {
                let mut out = String::new();
                for d in self.descendants(id) {
                    if let NodeData::Text(t) = &self.nodes[d].data {
                        out.push_str(t);
                    }
                }
                out
            }
        }
    }

    pub fn text(&self, id: NodeId) -> Option<&str> {
        match &self.nodes[id].data {
            NodeData::Text(t) => Some(t),
            _ => None,
        }
    }

    pub fn deep_clone(&mut self, id: NodeId) -> NodeId {
        let data = self.nodes[id].data.clone();
        let copy = self.push(data);
        for child in self.nodes[id].children.clone() {
            let child_copy = self.deep_clone(child);
            self.nodes[child_copy].parent = Some(copy);
            self.nodes[copy].children.push(child_copy);
        }
        copy
    }

    // --- Parsing -----------------------------------------------------------------------------

    /// `Nokogiri::HTML5::Document#fragment(html)`: a new fragment parsed in a `body` context.
    pub fn parse_fragment(&mut self, html: &str) -> Result<NodeId, ParseError> {
        let fragment = self.new_fragment();
        for node in self.parse_nodes(html, &Context::body())? {
            self.append(fragment, node);
        }
        Ok(fragment)
    }

    /// The context Nokogiri uses when parsing markup for a node (`Node#fragment`).
    pub fn context_for(&self, id: NodeId) -> Context {
        match &self.nodes[id].data {
            NodeData::Element(e) => Context { name: e.name.clone() },
            // Gumbo falls back to a body context for anything that isn't an element
            _ => Context::body(),
        }
    }

    /// Parses `html` in the given context and returns the resulting top-level nodes, detached.
    pub fn parse_nodes(&mut self, html: &str, context: &Context) -> Result<Vec<NodeId>, ParseError> {
        let parsed = parse_with_html5ever(html, context);
        check_limits(&parsed)?;
        let root = parsed.fragment_root();
        let children = parsed.nodes[root].children.clone();
        let offset = self.nodes.len();
        for node in parsed.nodes {
            self.nodes.push(Node {
                data: node.data,
                parent: node.parent.map(|p| p + offset),
                children: node.children.into_iter().map(|c| c + offset).collect(),
            });
        }
        let top: Vec<NodeId> = children.into_iter().map(|c| c + offset).collect();
        for &n in &top {
            self.nodes[n].parent = None;
        }
        Ok(top)
    }

    /// Nokogiri's `node.inner_html = html` (via `children=`), parsed in the node's context.
    pub fn set_inner_html(&mut self, id: NodeId, html: &str) -> Result<(), ParseError> {
        let context = self.context_for(id);
        let new_children = self.parse_nodes(html, &context)?;
        for child in self.nodes[id].children.clone() {
            self.nodes[child].parent = None;
        }
        self.nodes[id].children.clear();
        for child in new_children {
            self.append(id, child);
        }
        Ok(())
    }

    /// Nokogiri's `node.replace(html)`: the markup is parsed in the context of the node's parent.
    pub fn replace_with_html(&mut self, id: NodeId, html: &str) -> Result<(), ParseError> {
        let Some(parent) = self.parent(id) else { return Ok(()) };
        let context = self.context_for(parent);
        let replacements = self.parse_nodes(html, &context)?;
        for n in replacements {
            self.insert_before(id, n);
        }
        self.detach(id);
        Ok(())
    }

    /// Replaces a node with other (detached) nodes.
    pub fn replace_with_nodes(&mut self, id: NodeId, replacements: &[NodeId]) {
        if self.parent(id).is_none() {
            return;
        }
        for &n in replacements {
            self.insert_before(id, n);
        }
        self.detach(id);
    }

    // --- Serialization -----------------------------------------------------------------------

    /// `node.to_html` for an HTML5 document: the node itself (children only for a fragment).
    pub fn to_html(&self, id: NodeId) -> String {
        self.serialize(id, AttributeEscaping::Nokogiri)
    }

    /// `to_html`, but with `<` and `>` escaped in attribute values as well, as the current HTML
    /// serialization algorithm does. Nokogiri leaves them raw, which lets auto_link's regular
    /// expressions mistake an attribute value for text.
    pub fn to_html_with_escaped_attribute_brackets(&self, id: NodeId) -> String {
        self.serialize(id, AttributeEscaping::Brackets)
    }

    fn serialize(&self, id: NodeId, escaping: AttributeEscaping) -> String {
        let mut out = String::new();
        match self.nodes[id].data {
            NodeData::Fragment | NodeData::Document => self.serialize_children(id, escaping, &mut out),
            _ => self.serialize_node(id, escaping, &mut out),
        }
        out
    }

    pub fn inner_html(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.serialize_children(id, AttributeEscaping::Nokogiri, &mut out);
        out
    }

    fn serialize_children(&self, id: NodeId, escaping: AttributeEscaping, out: &mut String) {
        for &child in &self.nodes[id].children {
            self.serialize_node(child, escaping, out);
        }
    }

    fn serialize_node(&self, id: NodeId, escaping: AttributeEscaping, out: &mut String) {
        match &self.nodes[id].data {
            NodeData::Element(e) => {
                let tag = serialized_tag_name(&e.name);
                out.push('<');
                out.push_str(&tag);
                for attr in &e.attrs {
                    out.push(' ');
                    out.push_str(&attr.qualified_name());
                    out.push_str("=\"");
                    escape_attribute(&attr.value, escaping, out);
                    out.push('"');
                }
                out.push('>');
                if e.name.ns == ns!(html) && is_void_element(&e.name.local) {
                    return;
                }
                self.serialize_children(id, escaping, out);
                out.push_str("</");
                out.push_str(&tag);
                out.push('>');
            }
            NodeData::Text(text) => {
                let raw = self.parent(id).is_some_and(|p| {
                    self.element(p).is_some_and(|e| e.name.ns == ns!(html) && is_raw_text_element(&e.name.local))
                });
                if raw {
                    out.push_str(text);
                } else {
                    escape_text(text, out);
                }
            }
            NodeData::Comment(text) => {
                out.push_str("<!--");
                out.push_str(text);
                out.push_str("-->");
            }
            NodeData::Doctype(name) => {
                out.push_str("<!DOCTYPE ");
                out.push_str(name);
                out.push('>');
            }
            NodeData::Fragment | NodeData::Document => self.serialize_children(id, escaping, out),
        }
    }
}

fn serialized_tag_name(name: &QualName) -> String {
    if name.ns == ns!(html) || name.ns == ns!(svg) || name.ns == ns!(mathml) {
        name.local.to_string()
    } else {
        match &name.prefix {
            Some(p) => format!("{}:{}", p, name.local),
            None => name.local.to_string(),
        }
    }
}

pub fn is_void_element(local: &str) -> bool {
    matches!(
        local,
        "area" | "base" | "basefont" | "bgsound" | "br" | "col" | "embed" | "frame" | "hr" | "img" | "input"
            | "keygen" | "link" | "meta" | "param" | "source" | "track" | "wbr"
    )
}

fn is_raw_text_element(local: &str) -> bool {
    matches!(
        local,
        "style" | "script" | "xmp" | "iframe" | "noembed" | "noframes" | "plaintext" | "noscript"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttributeEscaping {
    Nokogiri,
    Brackets,
}

fn escape_attribute(value: &str, escaping: AttributeEscaping, out: &mut String) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            '"' => out.push_str("&quot;"),
            '<' if escaping == AttributeEscaping::Brackets => out.push_str("&lt;"),
            '>' if escaping == AttributeEscaping::Brackets => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

fn escape_text(value: &str, out: &mut String) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

// --- html5ever glue ------------------------------------------------------------------------------

fn check_limits(parsed: &ParsedTree) -> Result<(), ParseError> {
    let root = parsed.fragment_root();
    let mut stack: Vec<(NodeId, usize)> = parsed.nodes[root].children.iter().map(|&c| (c, 1)).collect();
    while let Some((n, depth)) = stack.pop() {
        if let NodeData::Element(e) = &parsed.nodes[n].data {
            if depth > MAX_TREE_DEPTH {
                return Err(ParseError::TreeDepthExceeded);
            }
            if e.attrs.len() > MAX_ATTRIBUTES {
                return Err(ParseError::TooManyAttributes);
            }
            stack.extend(parsed.nodes[n].children.iter().map(|&c| (c, depth + 1)));
        }
    }
    Ok(())
}

struct ParsedTree {
    nodes: Vec<Node>,
}

impl ParsedTree {
    /// html5ever puts the fragment's nodes under an `html` element beneath the document.
    fn fragment_root(&self) -> NodeId {
        self.nodes[0]
            .children
            .iter()
            .copied()
            .find(|&c| matches!(&self.nodes[c].data, NodeData::Element(e) if e.name.local == local_name!("html")))
            .unwrap_or(0)
    }
}

fn parse_with_html5ever(html: &str, context: &Context) -> ParsedTree {
    let mut opts = ParseOpts::default();
    // Gumbo parses without scripting, so <noscript> content is markup rather than raw text
    opts.tree_builder.scripting_enabled = false;
    opts.tree_builder.quirks_mode = QuirksMode::NoQuirks;
    let sink = Sink { nodes: RefCell::new(vec![Node { data: NodeData::Document, parent: None, children: vec![] }]) };
    let parser = html5ever::driver::parse_fragment(sink, opts, context.name.clone(), Vec::new(), false);
    parser.one(StrTendril::from(html))
}

struct Sink {
    nodes: RefCell<Vec<Node>>,
}

#[derive(Debug)]
struct OwnedName(QualName);

impl ElemName for OwnedName {
    fn ns(&self) -> &Namespace {
        &self.0.ns
    }

    fn local_name(&self) -> &LocalName {
        &self.0.local
    }
}

impl Sink {
    fn new_node(&self, data: NodeData) -> NodeId {
        let mut nodes = self.nodes.borrow_mut();
        nodes.push(Node { data, parent: None, children: vec![] });
        nodes.len() - 1
    }

    fn detach(&self, id: NodeId) {
        let mut nodes = self.nodes.borrow_mut();
        if let Some(parent) = nodes[id].parent.take() {
            nodes[parent].children.retain(|&c| c != id);
        }
    }

    fn insert_at(&self, parent: NodeId, index: usize, child: NodeOrText<NodeId>) {
        match child {
            NodeOrText::AppendNode(node) => {
                self.detach(node);
                let mut nodes = self.nodes.borrow_mut();
                nodes[node].parent = Some(parent);
                nodes[parent].children.insert(index, node);
            }
            NodeOrText::AppendText(text) => {
                // Adjacent text merges into the preceding text node, as in the DOM
                let previous = if index > 0 { Some(self.nodes.borrow()[parent].children[index - 1]) } else { None };
                if let Some(prev) = previous
                    && let NodeData::Text(existing) = &mut self.nodes.borrow_mut()[prev].data {
                        existing.push_str(&text);
                        return;
                    }
                let node = self.new_node(NodeData::Text(text.to_string()));
                let mut nodes = self.nodes.borrow_mut();
                nodes[node].parent = Some(parent);
                nodes[parent].children.insert(index, node);
            }
        }
    }
}

impl TreeSink for Sink {
    type Handle = NodeId;
    type Output = ParsedTree;
    type ElemName<'a> = OwnedName;

    fn finish(self) -> ParsedTree {
        ParsedTree { nodes: self.nodes.into_inner() }
    }

    fn parse_error(&self, _msg: Cow<'static, str>) {}

    fn get_document(&self) -> NodeId {
        0
    }

    fn elem_name<'a>(&'a self, target: &'a NodeId) -> OwnedName {
        match &self.nodes.borrow()[*target].data {
            NodeData::Element(e) => OwnedName(e.name.clone()),
            _ => panic!("not an element"),
        }
    }

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, _flags: ElementFlags) -> NodeId {
        let mut seen: Vec<Attr> = Vec::with_capacity(attrs.len());
        for a in attrs {
            if !seen.iter().any(|s| s.name == a.name) {
                seen.push(Attr { name: a.name, value: a.value.to_string() });
            }
        }
        self.new_node(NodeData::Element(ElementData { name, attrs: seen }))
    }

    fn create_comment(&self, text: StrTendril) -> NodeId {
        self.new_node(NodeData::Comment(text.to_string()))
    }

    fn create_pi(&self, _target: StrTendril, data: StrTendril) -> NodeId {
        self.new_node(NodeData::Comment(data.to_string()))
    }

    fn append(&self, parent: &NodeId, child: NodeOrText<NodeId>) {
        let index = self.nodes.borrow()[*parent].children.len();
        self.insert_at(*parent, index, child);
    }

    fn append_based_on_parent_node(&self, element: &NodeId, prev_element: &NodeId, child: NodeOrText<NodeId>) {
        if self.nodes.borrow()[*element].parent.is_some() {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(&self, name: StrTendril, _public_id: StrTendril, _system_id: StrTendril) {
        let node = self.new_node(NodeData::Doctype(name.to_string()));
        self.append(&0, NodeOrText::AppendNode(node));
    }

    fn get_template_contents(&self, target: &NodeId) -> NodeId {
        // libxml2 has no template contents: Nokogiri keeps them as ordinary children
        *target
    }

    fn same_node(&self, x: &NodeId, y: &NodeId) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn append_before_sibling(&self, sibling: &NodeId, new_node: NodeOrText<NodeId>) {
        let parent = self.nodes.borrow()[*sibling].parent.expect("sibling has a parent");
        let index = self.nodes.borrow()[parent].children.iter().position(|c| c == sibling).unwrap();
        self.insert_at(parent, index, new_node);
    }

    fn add_attrs_if_missing(&self, target: &NodeId, attrs: Vec<Attribute>) {
        if let NodeData::Element(e) = &mut self.nodes.borrow_mut()[*target].data {
            for a in attrs {
                if !e.attrs.iter().any(|existing| existing.name == a.name) {
                    e.attrs.push(Attr { name: a.name, value: a.value.to_string() });
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &NodeId) {
        self.detach(*target);
    }

    fn reparent_children(&self, node: &NodeId, new_parent: &NodeId) {
        let children = std::mem::take(&mut self.nodes.borrow_mut()[*node].children);
        for child in children {
            self.nodes.borrow_mut()[child].parent = None;
            self.append(new_parent, NodeOrText::AppendNode(child));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(html: &str) -> String {
        let mut dom = Dom::new();
        let f = dom.parse_fragment(html).unwrap();
        dom.to_html(f)
    }

    #[test]
    fn gumbo_adoption_agency_keeps_the_fourth_formatting_element_on_the_stack() {
        let cases: serde_json::Value = serde_json::from_str(include_str!("../tests/markdown/parser.json")).unwrap();
        for case in cases.as_array().unwrap() {
            assert_eq!(roundtrip(case["input"].as_str().unwrap()), case["output"].as_str().unwrap());
        }
    }

    #[test]
    fn serializes_like_nokogiri() {
        assert_eq!(roundtrip("<td>x</td>"), "x");
        assert_eq!(roundtrip("<p><table><tr><td>a</td></tr></table>"), "<p></p><table><tbody><tr><td>a</td></tr></tbody></table>");
        assert_eq!(
            roundtrip("<a title='a<b>c' href=\"x&y\u{a0}z\">t&lt;\u{a0}>\"'</a>"),
            "<a title=\"a<b>c\" href=\"x&amp;y&nbsp;z\">t&lt;&nbsp;&gt;\"'</a>"
        );
        let mut dom = Dom::new();
        let f = dom.parse_fragment("<a title='a<b>c'>d>e</a>").unwrap();
        assert_eq!(dom.to_html_with_escaped_attribute_brackets(f), "<a title=\"a&lt;b&gt;c\">d&gt;e</a>");
        assert_eq!(roundtrip("<pre>\n\nx</pre>"), "<pre>\nx</pre>");
        assert_eq!(roundtrip("<noscript><b>x</b></noscript>"), "<noscript><b>x</b></noscript>");
        assert_eq!(roundtrip("<?php x ?>"), "<!--?php x ?-->");
        assert_eq!(roundtrip("<SVG viewBox='0 0 1 1'><CLIPPATH/></SVG>"), "<svg viewBox=\"0 0 1 1\"><clipPath></clipPath></svg>");
    }

    #[test]
    fn enforces_gumbo_limits() {
        let mut dom = Dom::new();
        assert!(dom.parse_fragment(&"<b>".repeat(400)).is_ok());
        assert_eq!(dom.parse_fragment(&"<b>".repeat(401)), Err(ParseError::TreeDepthExceeded));
    }
}
