//! Rails-compatible escaping and JSON wire encoding.


/// Escapes like `ERB::Util.html_escape`.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    push_escaped(&mut out, text);
    out
}

pub fn push_escaped(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
}

/// `ActiveSupport::JSON.encode` output escaping (`escape_html_entities_in_json`): `<`, `>` and
/// `&` become `\uXXXX`. The JS line separators stay raw (`load_defaults 8.2` turns
/// `escape_js_separators_in_json` off). Apply to serde_json output for Jbuilder views.
pub fn rails_json_escape(json: &str) -> String {
    json.replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

/// `render json:` / Jbuilder: serde_json with Rails' HTML-entity escaping.
pub fn to_rails_json<T: serde::Serialize>(value: &T) -> String {
    rails_json_escape(&serde_json::to_string(value).expect("serializable"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_like_erb_util() {
        assert_eq!(escape(r#"<&>"'x"#), "&lt;&amp;&gt;&quot;&#39;x");
    }

    #[test]
    fn escapes_json_like_rails() {
        assert_eq!(
            to_rails_json(&"<b>&</b>"),
            r#""\u003cb\u003e\u0026\u003c/b\u003e""#
        );
    }
}
