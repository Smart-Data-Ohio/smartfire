pub mod calls;

use crate::helpers as h;

/// `Rooms::Direct#direct_display_name`: named rooms keep their name; unnamed
/// groups preview three first names. The caller supplies ordered other members.
pub fn room_display_name(name: Option<&str>, direct: bool, other_member_names: &[String], for_user_name: Option<&str>) -> String {
    if direct {
        if let Some(name) = h::presence(name) { return name.to_owned(); }
        match other_member_names {
            [] => for_user_name.unwrap_or_default().to_owned(),
            [name] => name.clone(),
            names => {
                let firsts = names.iter().take(3).map(|name| {
                    name.split([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
                        .find(|s| !s.is_empty()).unwrap_or_default()
                }).collect::<Vec<_>>().join(", ");
                let remaining = names.len().saturating_sub(3);
                if remaining > 0 { format!("{firsts} +{remaining}") } else { firsts }
            }
        }
    } else {
        name.unwrap_or_default().to_string()
    }
}
