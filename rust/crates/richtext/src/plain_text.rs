//! `ActionText::PlainTextConversion`

use crate::dom::{Dom, NodeId};
use crate::ruby::{chomp_newlines, is_blank};

/// `PlainTextConversion.node_to_plain_text`: a bottom-up reduction keyed on each node's name.
pub fn node_to_plain_text(dom: &Dom, node: NodeId) -> String {
    chomp_newlines(&plain_text_for(dom, node)).to_string()
}

fn plain_text_for(dom: &Dom, node: NodeId) -> String {
    let child_values = || -> Vec<String> { dom.children(node).iter().map(|&c| plain_text_for(dom, c)).collect() };
    let name = dom.name(node);
    match name.as_ref() {
        "script" | "style" | "unsupported" => String::new(),
        "h1" | "p" => plain_text_for_block(&child_values()),
        "ul" | "ol" => {
            let text = plain_text_for_block(&child_values());
            if list_depth(dom, node) > 0 { format!("\n{text}") } else { text }
        }
        "br" => "\n".to_string(),
        // Text nodes, and elements that happen to be named "text" (SVG's), use `node.text`
        "text" => chomp_newlines(&dom.text_content(node)).to_string(),
        "div" => format!("{}\n", chomp_newlines(&child_values().concat())),
        "figcaption" => format!("[{}]", chomp_newlines(&child_values().concat())),
        "blockquote" => {
            let text = plain_text_for_block(&child_values());
            if is_blank(&text) {
                return "“”".to_string();
            }
            // `text.insert(text.rindex(/\S/) + 1, "”")`, then `text.index(/\S/)` for "“"
            let is_space = |c: char| matches!(c, ' ' | '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r');
            let mut text = text;
            if let Some((i, c)) = text.char_indices().rev().find(|&(_, c)| !is_space(c)) {
                text.insert(i + c.len_utf8(), '”');
            }
            if let Some((i, _)) = text.char_indices().find(|&(_, c)| !is_space(c)) {
                text.insert(i, '“');
            }
            text
        }
        "li" => {
            let bullet = bullet_for_li(dom, node);
            let text = chomp_newlines(&child_values().concat()).to_string();
            let depth = list_depth(dom, node);
            let indentation = if depth > 1 { "  ".repeat(depth - 1) } else { String::new() };
            format!("{indentation}{bullet} {text}\n")
        }
        _ => child_values().concat(),
    }
}

fn plain_text_for_block(child_values: &[String]) -> String {
    format!("{}\n\n", chomp_newlines(&child_values.concat()))
}

fn is_list(name: &str) -> bool {
    name == "ul" || name == "ol"
}

fn list_depth(dom: &Dom, node: NodeId) -> usize {
    dom.ancestors(node).into_iter().filter(|&a| is_list(&dom.name(a))).count()
}

fn bullet_for_li(dom: &Dom, node: NodeId) -> String {
    let list = dom.ancestors(node).into_iter().map(|a| dom.name(a).into_owned()).find(|n| is_list(n));
    if list.as_deref() == Some("ol") {
        let index = dom.parent(node).map(|p| dom.element_children(p).iter().position(|&c| c == node).unwrap_or(0)).unwrap_or(0);
        format!("{}.", index + 1)
    } else {
        "•".to_string()
    }
}
