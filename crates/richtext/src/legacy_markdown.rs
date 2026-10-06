//! `app/models/message/legacy_markdown.rb`: legacy stored HTML becomes source for the Markdown
//! textarea on edit. This is a serializer, not the presentation sanitizer.
use crate::Error;
use crate::attachables::{self, Attachable, MENTION_CONTENT_TYPE, RenderContext};
use crate::content::ATTACHMENT_TAG;
use crate::dom::{Dom, NodeId};
use crate::ruby::{is_blank, presence, strip};

pub fn render(html: &str, ctx: &RenderContext) -> Result<String, Error> {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).map_err(Error::Parse)?;
    let markdown = Renderer { dom: &dom, ctx }.blocks(dom.children(root));
    Ok(if is_blank(&markdown) { strip(&dom.text_content(root)).to_owned() } else { markdown })
}
pub fn non_mention_attachments(html: &str) -> Result<String, Error> {
    let mut dom = Dom::new();
    let root = dom.parse_fragment(html).map_err(Error::Parse)?;
    Ok(dom
        .descendants(root)
        .into_iter()
        .filter(|&n| dom.local_name(n) == Some(ATTACHMENT_TAG) && dom.attr(n, "content-type") != Some(MENTION_CONTENT_TYPE))
        .map(|n| dom.to_html(n))
        .collect::<Vec<_>>()
        .join("\n"))
}
struct Renderer<'a, 'b> {
    dom: &'a Dom,
    ctx: &'a RenderContext<'b>,
}
impl Renderer<'_, '_> {
    fn blocks(&self, nodes: &[NodeId]) -> String {
        strip(&nodes.iter().map(|&n| self.block(n)).filter(|s| !is_blank(s)).collect::<Vec<_>>().join("\n\n")).to_owned()
    }
    fn block(&self, node: NodeId) -> String {
        if let Some(text) = self.dom.text(node) {
            return escaped_text(text);
        }
        match self.dom.name(node).as_ref() {
            "p" | "div" | "section" | "article" | "figure" => strip(&self.inline(node)).to_owned(),
            name @ ("h1" | "h2" | "h3" | "h4" | "h5" | "h6") => {
                format!("{} {}", "#".repeat(name.as_bytes()[1] as usize - b'0' as usize), strip(&self.inline(node)))
            }
            "blockquote" => self.blocks(self.dom.children(node)).split_inclusive('\n').map(|line| format!("> {line}")).collect(),
            "ul" => self.list(node, false),
            "ol" => self.list(node, true),
            "pre" => self.code_block(node),
            "hr" => "---".into(),
            "br" => "\n".into(),
            ATTACHMENT_TAG => self.attachment(node),
            _ => strip(&self.inline(node)).to_owned(),
        }
    }
    fn list(&self, node: NodeId, ordered: bool) -> String {
        self.dom
            .element_children(node)
            .into_iter()
            .filter(|&n| self.dom.local_name(n) == Some("li"))
            .enumerate()
            .map(|(i, item)| {
                let prefix = if ordered { format!("{}.", i + 1) } else { "-".into() };
                let children = self
                    .dom
                    .children(item)
                    .iter()
                    .copied()
                    .filter(|&n| !matches!(self.dom.local_name(n), Some("ul" | "ol")))
                    .collect::<Vec<_>>();
                let mut parts = vec![format!("{prefix} {}", strip(&self.inline_children(&children)))];
                for nested in self.dom.element_children(item).into_iter().filter(|&n| matches!(self.dom.local_name(n), Some("ul" | "ol"))) {
                    let text = self.list(nested, self.dom.local_name(nested) == Some("ol"));
                    parts.push(text.split_inclusive('\n').map(|line| format!("  {line}")).collect::<String>());
                }
                parts.into_iter().filter(|s| !is_blank(s)).collect::<Vec<_>>().join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn code_block(&self, node: NodeId) -> String {
        let code = self.dom.descendants(node).into_iter().find(|&n| self.dom.local_name(n) == Some("code")).unwrap_or(node);
        let language =
            self.dom.attr(code, "class").unwrap_or("").split_whitespace().find_map(|c| c.strip_prefix("language-")).unwrap_or("");
        let text = self.dom.text_content(code);
        let delimiter = code_delimiter(&text);
        format!("{delimiter}{language}\n{}\n{delimiter}", text.trim_end_matches([' ', '\t', '\n', '\r', '\u{b}', '\u{c}']))
    }
    fn inline(&self, node: NodeId) -> String {
        self.inline_children(self.dom.children(node))
    }
    fn inline_children(&self, nodes: &[NodeId]) -> String {
        nodes.iter().map(|&n| self.inline_node(n)).collect()
    }
    fn inline_node(&self, node: NodeId) -> String {
        if let Some(text) = self.dom.text(node) {
            return escaped_text(text);
        }
        let content = self.inline(node);
        match self.dom.name(node).as_ref() {
            "strong" | "b" => format!("**{content}**"),
            "em" | "i" => format!("*{content}*"),
            "del" | "s" | "strike" => format!("~~{content}~~"),
            "code" => {
                let text = self.dom.text_content(node);
                let delimiter = code_delimiter(&text);
                format!("{delimiter}{text}{delimiter}")
            }
            "a" => match presence(self.dom.attr(node, "href")) {
                Some(href) => format!("[{content}](<{}>)", href.replace('<', "\\<").replace('>', "\\>")),
                None => content,
            },
            "br" => "\n".into(),
            ATTACHMENT_TAG => self.attachment(node),
            "ul" | "ol" => format!("\n{}\n", self.list(node, self.dom.local_name(node) == Some("ol"))),
            "pre" => format!("\n{}\n", self.code_block(node)),
            _ => content,
        }
    }
    fn attachment(&self, node: NodeId) -> String {
        let result = attachables::attachment_from_node(self.dom, node, self.ctx);
        if let Ok(attachment) = &result
            && let Attachable::User(user) = &attachment.attachable
        {
            return format!("@[{}]", user.name);
        }
        if let Some(value) = presence(self.dom.attr(node, "href")).or_else(|| presence(self.dom.attr(node, "url"))) {
            return value.to_owned();
        }
        if let Ok(attachment) = result {
            let plain = match attachables::attachment_plain_text(&attachment) {
                attachables::PlainTextRepresentation::Html(text) => text,
                attachables::PlainTextRepresentation::Content(html) => {
                    let mut dom = Dom::new();
                    match dom.parse_fragment(&html) {
                        Ok(root) => return dom.text_content(root),
                        Err(_) => String::new(),
                    }
                }
            };
            if !is_blank(&plain) {
                return plain;
            }
        }
        presence(self.dom.attr(node, "filename")).unwrap_or("[attachment]").to_owned()
    }
}
fn escaped_text(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if matches!(c, '\\' | '`' | '*' | '_' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
fn code_delimiter(text: &str) -> String {
    "`".repeat(text.split(|c| c != '`').map(str::len).max().unwrap_or(0) + 1)
}
