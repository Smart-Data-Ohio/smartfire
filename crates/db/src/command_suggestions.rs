//! Registry and registered-agent picker metadata, without invoking agents (WS11).
use crate::Result;
use rusqlite::Connection;
use serde::Serialize;
#[derive(Serialize)]
pub struct Suggestion {
    pub name: String,
    pub value: String,
    pub description: String,
    pub arg_hint: String,
    pub takes_arguments: bool,
    pub agent: Option<String>,
}
pub fn for_room(
    conn: &Connection,
    room_id: i64,
    thread: bool,
    query: Option<&str>,
) -> Result<Vec<Suggestion>> {
    let mut commands = crate::slash_commands::available(thread)
        .into_iter()
        .map(|c| Suggestion {
            name: c.name.clone(),
            value: c.name,
            description: c.description,
            arg_hint: c.arg_hint,
            takes_arguments: c.takes_arguments,
            agent: None,
        })
        .collect::<Vec<_>>();
    let mut stmt=conn.prepare("SELECT c.name,c.description,c.takes_arguments,u.name FROM agent_slash_commands c JOIN agents a ON a.id=c.agent_id JOIN users u ON u.id=a.user_id WHERE c.room_id=? ORDER BY c.name")?;
    commands.extend(
        stmt.query_map([room_id], |r| {
            let name: String = r.get(0)?;
            let description: Option<String> = r.get(1)?;
            Ok(Suggestion {
                name: name.clone(),
                value: name,
                description: description
                    .filter(|s| crate::slash_commands::time_parser::present(s).is_some())
                    .unwrap_or_else(|| "Custom command".into()),
                arg_hint: String::new(),
                takes_arguments: r.get(2)?,
                agent: r.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    if let Some(query) = query {
        let query = query.to_lowercase();
        commands
            .retain(|c| c.name.contains(&query) || c.description.to_lowercase().contains(&query));
    }
    Ok(commands)
}
