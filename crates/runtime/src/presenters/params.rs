//! Request parameters as the views show them.
use campfire_kit::{Ctx, Param};

/// Explicit request `to_s` sites, including Ruby Array/Parameters coercion. This does
/// not change the kit-wide parameter API or ActiveRecord lookup casting.
pub fn param_string(value:&Param)->String {
    use campfire_richtext::ruby::{json_value_to_s,json_value_inspect};
    fn inspect(value:&Param)->String {
        match value {
            Param::Hash(_)=>format!("#<ActionController::Parameters {} permitted: false>",param_string(value)),
            Param::Array(_)=>param_string(value),
            value=>json_value_inspect(&value.to_json()),
        }
    }
    match value {
        Param::Array(values)=>format!("[{}]",values.iter().map(inspect).collect::<Vec<_>>().join(", ")),
        Param::Hash(map)=>format!("{{{}}}",map.iter().map(|(k,v)|format!("{} => {}",json_value_inspect(&serde_json::Value::String(k.clone())),inspect(v))).collect::<Vec<_>>().join(", ")),
        value=>json_value_to_s(&value.to_json()),
    }
}

pub fn display_query(c: &Ctx) -> Option<String> {
    let raw = c.param("q").map(param_string).unwrap_or_default();
    let q = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    (!q.is_empty()).then_some(q)
}
