//! Page-bounded production fragment dependencies. Each query carries its root id,
//! so batching preserves individual keys rather than invalidating the whole page.
use super::{Presenter, Result, cache_key_with_version};
use campfire_db::{Message, Timestamp, models::message_rendering::RenderingRecords};
use std::collections::{BTreeSet, HashMap};

#[derive(Default)]
pub struct CacheFacts {
    pub versions: HashMap<i64, BTreeSet<String>>,
    pub avatars: HashMap<i64, BTreeSet<String>>,
    pub icons: HashMap<String, (i64, Timestamp)>,
    pub validators: HashMap<i64, String>,
}
impl CacheFacts {
    pub fn load(
        p: &Presenter<'_>,
        messages: &[Message],
        records: &RenderingRecords,
    ) -> Result<Self> {
        let mut facts = Self::load_rendered(p, messages, records)?;
        let pages = messages
            .iter()
            .map(|m| {
                let mut dependencies = vec![m.clone()];
                if let Some(source) = m
                    .reply_to_message_id
                    .and_then(|id| records.sources.get(&id))
                {
                    dependencies.push(source.clone());
                }
                (m.id, dependencies)
            })
            .collect::<Vec<_>>();
        facts.validators =
            crate::controllers::presenters::message_freshness::etags_for_pages(p.conn, &pages)?;
        Ok(facts)
    }

    pub fn load_rendered(
        p: &Presenter<'_>,
        messages: &[Message],
        records: &RenderingRecords,
    ) -> Result<Self> {
        let mut facts = Self::default();
        if messages.is_empty() {
            return Ok(facts);
        }
        // Integers originate in the authorized bounded result window; never SQL input.
        let ids = messages
            .iter()
            .map(|m| m.id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let prefix = format!("WITH roots AS (SELECT * FROM messages WHERE id IN ({ids})), bodies AS (
            SELECT id AS root,id FROM roots
            UNION SELECT r.id,s.id FROM roots r JOIN messages s ON s.id=r.reply_to_message_id
            UNION SELECT r.message_id,s.id FROM message_references r JOIN roots m ON m.id=r.message_id JOIN messages s ON s.id=r.referenced_message_id
        ), attachments AS (
            SELECT b.root,a.id,a.blob_id FROM bodies b JOIN active_storage_attachments a ON a.record_id=b.id AND a.record_type='Message' AND a.name='attachment'
            UNION SELECT b.root,a.id,a.blob_id FROM bodies b JOIN action_text_rich_texts t ON t.record_id=b.id AND t.record_type='Message' AND t.name='body'
            JOIN active_storage_attachments a ON a.record_id=t.id AND a.record_type='ActionText::RichText' AND a.name='embeds'
        ) ");
        let mut queries = vec![
            "SELECT root,'active_storage/attachments' AS stem,id,NULL AS stamp FROM attachments".to_owned(),
            "SELECT a.root,'active_storage/blobs',b.id,NULL FROM attachments a JOIN active_storage_blobs b ON b.id=a.blob_id".to_owned(),
            "SELECT b.root,'messages',m.id,m.updated_at FROM bodies b JOIN messages m ON m.id=b.id".to_owned(),
            "SELECT b.root,'action_text/rich_texts',t.id,t.updated_at FROM bodies b JOIN action_text_rich_texts t ON t.record_id=b.id AND t.record_type='Message' AND t.name='body'".to_owned(),
        ];
        for (table, stamp) in [
            ("boosts", "updated_at"),
            ("message_pins", "updated_at"),
            ("agent_steps", "updated_at"),
            ("drive_attachments", "NULL"),
            ("polls", "updated_at"),
            ("message_references", "updated_at"),
        ] {
            let stamp = if stamp == "NULL" {
                "NULL".to_owned()
            } else {
                format!("t.{stamp}")
            };
            queries.push(format!("SELECT m.id,'{table}',t.id,{stamp} FROM roots m JOIN {table} t ON t.message_id=m.id"));
        }
        for table in ["poll_options", "poll_votes"] {
            queries.push(format!("SELECT m.id,'{table}',t.id,t.updated_at FROM roots m JOIN polls p ON p.message_id=m.id JOIN {table} t ON t.poll_id=p.id"));
        }
        queries.push("SELECT m.id,'github/pull_request_threads',t.id,t.updated_at FROM roots m JOIN github_pull_request_references r ON r.message_id=m.id JOIN github_pull_request_threads t ON t.room_id=m.room_id AND t.github_pull_request_id=r.github_pull_request_id".into());
        for (reference, card, fk, namespace) in [
            (
                "github_pull_request_references",
                "github_pull_requests",
                "github_pull_request_id",
                true,
            ),
            (
                "fizzy_card_references",
                "fizzy_cards",
                "fizzy_card_id",
                true,
            ),
            (
                "twitter_post_references",
                "twitter_posts",
                "twitter_post_id",
                true,
            ),
            ("event_references", "events", "event_id", false),
            (
                "link_embed_references",
                "link_embeds",
                "link_embed_id",
                false,
            ),
        ] {
            let stem = |table: &str| {
                if namespace {
                    table.replacen('_', "/", 1)
                } else {
                    table.to_owned()
                }
            };
            queries.push(format!("SELECT m.id,'{}',r.id,r.updated_at FROM roots m JOIN {reference} r ON r.message_id=m.id",stem(reference)));
            queries.push(format!("SELECT m.id,'{}',c.id,c.updated_at FROM roots m JOIN {reference} r ON r.message_id=m.id JOIN {card} c ON c.id=r.{fk}",stem(card)));
        }
        for row in p
            .conn
            .prepare(&(prefix + &queries.join(" UNION ALL ")))?
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<Timestamp>>(3)?,
                ))
            })?
        {
            let (root, stem, id, stamp) = row?;
            facts
                .versions
                .entry(root)
                .or_default()
                .insert(stamp.map_or_else(
                    || format!("{stem}/{id}"),
                    |t| cache_key_with_version(&stem, id, t.jiff()),
                ));
        }
        let mentions = records
            .bodies
            .values()
            .flatten()
            .flat_map(|body| crate::controllers::presenters::search_preloads::mention_ids(body, 0))
            .collect::<Vec<_>>();
        let user_ids = messages
            .iter()
            .chain(records.sources.values())
            .map(|m| m.creator_id)
            .chain(records.boosts.values().flatten().map(|b| b.booster_id))
            .chain(records.votes.values().flatten().map(|(v, _)| v.user_id))
            .chain(mentions)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("SELECT a.record_id,'active_storage/attachments',a.id FROM active_storage_attachments a JOIN users u ON u.id=a.record_id WHERE a.record_type='User' AND a.name='avatar' AND a.record_id IN ({user_ids})
            UNION ALL SELECT a.record_id,'active_storage/blobs',b.id FROM active_storage_attachments a JOIN users u ON u.id=a.record_id JOIN active_storage_blobs b ON b.id=a.blob_id WHERE a.record_type='User' AND a.name='avatar' AND a.record_id IN ({user_ids})");
        for row in p.conn.prepare(&sql)?.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })? {
            let (user, stem, id) = row?;
            facts
                .avatars
                .entry(user)
                .or_default()
                .insert(format!("{stem}/{id}"));
        }
        facts.icons = p
            .conn
            .prepare("SELECT name,id,updated_at FROM workspace_icons")?
            .query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?
            .collect::<std::result::Result<_, _>>()?;
        Ok(facts)
    }
}
