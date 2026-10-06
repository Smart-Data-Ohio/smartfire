//! `accounts/bots/github_connections#create`'s JSON body parser: the kit's parser for every
//! other route. It recognizes the route against the whole route table, so it sits with the
//! router rather than the controller.

/// Preserve numeric JSON lexemes solely for this controller's token `.to_s`.
/// All other routes and attributes retain the kit's ordinary parameter parser.
pub(crate) fn json_body_params(
    method: &campfire_kit::Method,
    path: &str,
    raw: &[u8],
) -> std::result::Result<campfire_kit::ParamMap, campfire_kit::params::ParamError> {
    use campfire_kit::params::{self, ParamError};
    use campfire_kit::{Param, ParamMap};
    let is_token = crate::controllers::recognize(method, &crate::controllers::normalize_path(path))
        .ok()
        .flatten()
        .is_some_and(|(route, _)| route.endpoint == "accounts/bots/github_connections#create");
    if !is_token {
        return params::from_json_body(raw);
    }
    params::validate_json_nesting(raw)?;
    let value: Box<serde_json::value::RawValue> =
        serde_json::from_slice(raw).map_err(|_| ParamError::Parse)?;
    if !value.get().starts_with('{') {
        return params::from_json_body(raw);
    }
    let fields: indexmap::IndexMap<String, Box<serde_json::value::RawValue>> =
        serde_json::from_str(value.get()).map_err(|_| ParamError::Parse)?;
    let mut result = ParamMap::new();
    for (key, value) in fields {
        let param = if key == "access_token" {
            Param::Str(
                crate::controllers::presenters::bot_input_casts::json_token_string(&value).map_err(|_| ParamError::Parse)?,
            )
        } else {
            unused_json_param(&value)?
        };
        result.insert(key, param);
    }
    Ok(result)
}

/// This action reads only access_token. Keep ordinary types for its other fields,
/// but don't reject the document for Ruby Integers/floats that Value can't hold.
/// Preserve large Integer digits and Float#as_json's null for nonfinite values
/// when sudo stores these ignored fields. Other controllers keep their parser.
fn unused_json_param(
    raw: &serde_json::value::RawValue,
) -> std::result::Result<campfire_kit::Param, campfire_kit::params::ParamError> {
    use campfire_kit::{Param, ParamMap, params::ParamError};
    use serde_json::value::RawValue;
    Ok(match raw.get().as_bytes().first() {
        Some(b'[') => {
            let values: Vec<Box<RawValue>> =
                serde_json::from_str(raw.get()).map_err(|_| ParamError::Parse)?;
            Param::Array(
                values
                    .iter()
                    .filter(|v| v.get() != "null")
                    .map(|v| unused_json_param(v))
                    .collect::<std::result::Result<_, _>>()?,
            )
        }
        Some(b'{') => {
            let values: indexmap::IndexMap<String, Box<RawValue>> =
                serde_json::from_str(raw.get()).map_err(|_| ParamError::Parse)?;
            let mut fields = ParamMap::new();
            for (key, value) in values {
                fields.insert(key, unused_json_param(&value)?);
            }
            Param::Hash(fields)
        }
        Some(b'-' | b'0'..=b'9') => {
            let text = crate::controllers::presenters::bot_input_casts::json_token_string(raw).map_err(|_| ParamError::Parse)?;
            if matches!(text.as_str(), "Infinity" | "-Infinity") {
                Param::Null
            } else if let Ok(serde_json::Value::Number(number)) = serde_json::from_str(raw.get())
                && (raw.get().contains(['.', 'e', 'E']) || number.is_i64() || number.is_u64())
            {
                Param::Number(number)
            } else {
                Param::BigInteger(text)
            }
        }
        _ => Param::from_json(serde_json::from_str(raw.get()).map_err(|_| ParamError::Parse)?),
    })
}

/// Other routes keep the already-parsed parameters without decoding them twice.
pub(crate) fn scoped_json_body_params(
    method: &campfire_kit::Method,
    path: &str,
    raw: &[u8],
) -> Option<std::result::Result<campfire_kit::ParamMap, campfire_kit::params::ParamError>> {
    if *method != campfire_kit::Method::POST {
        return None;
    }
    let is_token = crate::controllers::recognize(method, &crate::controllers::normalize_path(path))
        .ok()
        .flatten()
        .is_some_and(|(route, _)| route.endpoint == "accounts/bots/github_connections#create");
    is_token.then(|| json_body_params(method, path, raw))
}
