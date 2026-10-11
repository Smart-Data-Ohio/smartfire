use crate::helpers as h;

pub fn search_path(query: &str) -> String {
    format!(
        "{}?q={}",
        campfire_routes::searches(),
        h::url::cgi_escape(query)
    )
}
