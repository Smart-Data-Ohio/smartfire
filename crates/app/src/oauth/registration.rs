//! Public-client registration metadata accepted by the MCP authorization server.
use super::client::redirect_url;
use serde_json::{Value, json};

pub struct Registration {
    pub client_name: String,
    pub client_uri: Option<String>,
    pub redirect_uris: Vec<String>,
    pub metadata: Value,
}

impl Registration {
    pub fn parse(value: Value) -> Result<Self, &'static str> {
        let invalid = "invalid_client_metadata";
        let client_name = value["client_name"].as_str().ok_or(invalid)?;
        if client_name.trim().is_empty()
            || client_name.chars().count() > 100
            || value["token_endpoint_auth_method"].as_str() != Some("none")
        {
            return Err(invalid);
        }
        let redirect_uris: Vec<String> = serde_json::from_value(value["redirect_uris"].clone())
            .map_err(|_| "invalid_redirect_uri")?;
        if redirect_uris.is_empty() || redirect_uris.iter().any(|uri| redirect_url(uri).is_none()) {
            return Err("invalid_redirect_uri");
        }
        let grant_types: Vec<String> = match value.get("grant_types") {
            Some(grants) => serde_json::from_value(grants.clone()).map_err(|_| invalid)?,
            None => vec!["authorization_code".into(), "refresh_token".into()],
        };
        if !grant_types
            .iter()
            .any(|grant| grant == "authorization_code")
            || grant_types
                .iter()
                .any(|grant| !matches!(grant.as_str(), "authorization_code" | "refresh_token"))
        {
            return Err(invalid);
        }
        if let Some(responses) = value.get("response_types") {
            let responses: Vec<String> =
                serde_json::from_value(responses.clone()).map_err(|_| invalid)?;
            if responses != ["code"] {
                return Err(invalid);
            }
        }
        if value
            .get("scope")
            .is_some_and(|scope| scope.as_str() != Some("agent"))
        {
            return Err(invalid);
        }
        let client_uri = value
            .get("client_uri")
            .map(|uri| {
                let uri = uri.as_str().ok_or(invalid)?;
                if redirect_url(uri).is_none_or(|url| url.scheme() != "https") {
                    return Err(invalid);
                }
                Ok(uri.to_string())
            })
            .transpose()?;
        // Unknown metadata is ignored, and only the server's accepted values are stored/returned.
        let mut metadata = json!({
            "client_name": client_name,
            "redirect_uris": redirect_uris,
            "token_endpoint_auth_method": "none",
            "grant_types": grant_types,
            "response_types": ["code"],
            "scope": "agent",
        });
        if let Some(uri) = &client_uri {
            metadata["client_uri"] = json!(uri);
        }
        Ok(Self {
            client_name: client_name.to_string(),
            client_uri,
            redirect_uris,
            metadata,
        })
    }
}
