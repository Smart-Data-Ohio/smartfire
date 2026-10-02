//! app/models/board_automations/{sla_dispatcher,digest_dispatcher}.rb.
//! Eligibility is loaded across boards in batches. Only unclaimed due work acquires a writer.
use std::collections::{BTreeMap, HashSet};
use crate::sql::{query_all, query_one};
use crate::{BoardSlaNudge, BoardStaleDigest, ChannelThread, Connection, Database, Event, NewBoardSlaNudge, NewMessage, Message, Result, Room, Timestamp};
use crate::broadcasts::{Broadcast, Partial, room_dom_id, room_messages};
use jiff::{SignedDuration, tz::TimeZone};
use rusqlite::Row;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct DispatchStats {
    pub claims: usize,
    pub pushes: usize,
    pub notes: usize,
    pub failed_ids: Vec<i64>,
}
#[derive(Clone)]
struct SlaCandidate {
    thread_id: i64,
    room_id: i64,
    status: String,
    entered: Timestamp,
    nudge_minutes: i64,
    escalate_minutes: i64,
    nudge_recipient: Option<i64>,
    escalation_recipient: Option<i64>,
}
impl SlaCandidate {
    fn from_row(r: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self { thread_id:r.get("id")?,room_id:r.get("room_id")?,status:r.get("work_status")?,entered:r.get("work_status_changed_at")?,nudge_minutes:r.get("nudge_after_minutes")?,escalate_minutes:r.get("escalate_after_minutes")?,nudge_recipient:r.get("nudge_recipient")?,escalation_recipient:r.get("escalation_recipient")? })
    }
    fn recipient(&self, stage: &str) -> Option<i64> {
        if stage=="escalation" { self.escalation_recipient } else { self.nudge_recipient }
    }
    fn due(&self, stage: &str, now: Timestamp) -> bool {
        let minutes = if stage=="escalation" { self.escalate_minutes } else { self.nudge_minutes };
        self.entered <= now.ago(SignedDuration::from_secs(minutes.saturating_mul(60)))
    }
}
// EXISTS keeps duplicate memberships from multiplying candidates. An unavailable human/agent
// owner falls back to the creator, exactly like recipient_for; posting grants do not gate nudges.
const SLA_SELECT: &str = r#"
SELECT t.id,t.room_id,t.work_status,t.work_status_changed_at,r.nudge_after_minutes,r.escalate_after_minutes,
CASE
 WHEN owner.role != 2 AND owner.status=0 AND EXISTS(SELECT 1 FROM memberships WHERE room_id=t.room_id AND user_id=owner.id) THEN owner.id
 WHEN owner.role=2 AND human.role != 2 AND human.status=0 AND EXISTS(SELECT 1 FROM memberships WHERE room_id=t.room_id AND user_id=human.id) THEN human.id
 WHEN creator.role != 2 AND creator.status=0 AND EXISTS(SELECT 1 FROM memberships WHERE room_id=t.room_id AND user_id=creator.id) THEN creator.id
END AS nudge_recipient,
CASE WHEN creator.role != 2 AND creator.status=0 AND EXISTS(SELECT 1 FROM memberships WHERE room_id=t.room_id AND user_id=creator.id) THEN creator.id END AS escalation_recipient
FROM board_sla_rules r JOIN rooms b ON b.id=r.room_id
JOIN channel_threads t ON t.room_id=b.id AND t.work_status=r.work_status
LEFT JOIN users owner ON owner.id=t.work_owner_id
LEFT JOIN agents a ON a.id=(SELECT id FROM agents WHERE user_id=owner.id ORDER BY id LIMIT 1)
LEFT JOIN users human ON human.id=a.owner_id LEFT JOIN users creator ON creator.id=b.creator_id
WHERE b.type='Rooms::Board' AND b.deleted_at IS NULL AND r.work_status!='done' AND t.work_status_changed_at IS NOT NULL
"#;

/// Repeat sweeps read but never open a claim transaction. Each new stage commits its source,
/// inbox record and optional durable BoardNudgeJob atomically through the merged model API.
pub async fn dispatch_sla(db: &Database, now: Timestamp) -> Result<DispatchStats> {
    let pending = db.read(move |conn| {
        let candidates = query_all(conn,&format!("{SLA_SELECT} ORDER BY r.id,t.id"),[],SlaCandidate::from_row)?;
        let claims: HashSet<_> = query_all(conn,
            "SELECT n.channel_thread_id,n.work_status,n.stage,n.status_entered_at FROM board_sla_nudges n JOIN channel_threads t ON t.id=n.channel_thread_id JOIN board_sla_rules r ON r.room_id=t.room_id AND r.work_status=t.work_status",
            [],|r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Timestamp>(3)?)))?.into_iter().collect();
        Ok(candidates.into_iter().flat_map(|c| {
            ["nudge","escalation"].into_iter().filter_map(|stage| {
                (c.due(stage,now) && c.recipient(stage).is_some() && !claims.contains(&(c.thread_id,c.status.clone(),stage.into(),c.entered))).then(|| (c.clone(),stage))
            }).collect::<Vec<_>>()
        }).collect::<Vec<_>>())
    }).await?;
    let mut stats=DispatchStats::default();
    let mut pushed=HashSet::new();
    for (candidate,stage) in pending {
        let id=candidate.thread_id;
        let pushed_before=pushed.clone();
        let outcome=db.write(move |tx| {
            // Recheck the crossing and recipient in the source transaction. A concurrent status
            // change or revocation cannot notify for the stale eligibility snapshot.
            let fresh=query_one(tx.conn(),&format!("{SLA_SELECT} AND t.id=?"),[candidate.thread_id],SlaCandidate::from_row)?;
            let Some(fresh)=fresh.filter(|c| c.room_id==candidate.room_id && c.status==candidate.status && c.entered==candidate.entered) else { return Ok(None); };
            let Some(recipient)=fresh.recipient(stage) else {return Ok(None);};
            let push=!pushed_before.contains(&recipient);
            let input=NewBoardSlaNudge {room_id:candidate.room_id,channel_thread_id:candidate.thread_id,recipient_id:recipient,work_status:Some(candidate.status),stage:Some(stage.into()),status_entered_at:Some(candidate.entered)};
            match BoardSlaNudge::claim_and_notify(tx,input,push) {
                Ok(_)=>Ok(Some((recipient,push))),
                Err(crate::Error::RecordInvalid(_))=>Ok(None),
                Err(error) if error.is_record_not_unique()=>Ok(None),
                Err(error)=>Err(error),
            }
        }).await;
        match outcome {
            Ok(Some((recipient,push)))=>{stats.claims+=1;stats.pushes+=usize::from(push);pushed.insert(recipient);}
            Ok(None)=>{},
            Err(error)=>{tracing::error!(thread_id=id,%error,"Board SLA nudge failed");stats.failed_ids.push(id);}
        }
    }
    Ok(stats)
}

#[derive(Clone)]
struct StalePost {
    thread: ChannelThread,
    owner_name: Option<String>,
    creator_id: i64,
    nudge_minutes: i64,
}
fn stale_posts(conn: &Connection, now: Timestamp) -> Result<BTreeMap<i64,Vec<StalePost>>> {
    let posts = query_all(conn,r#"
SELECT t.*,u.name AS owner_name,b.creator_id AS board_creator,r.nudge_after_minutes
FROM rooms b JOIN board_sla_rules r ON r.room_id=b.id
JOIN channel_threads t ON t.room_id=b.id AND t.work_status=r.work_status
LEFT JOIN users u ON u.id=t.work_owner_id
WHERE b.type='Rooms::Board' AND b.deleted_at IS NULL AND t.work_status!='done' AND t.work_status_changed_at IS NOT NULL
AND NOT EXISTS(SELECT 1 FROM board_stale_digests d WHERE d.room_id=b.id AND d.digest_on=?)
ORDER BY b.id,t.work_status_changed_at,t.id
"#,[now.jiff().to_zoned(TimeZone::UTC).date().to_string()],|r| Ok(StalePost {thread:ChannelThread::from_row(r)?,owner_name:r.get("owner_name")?,creator_id:r.get("board_creator")?,nudge_minutes:r.get("nudge_after_minutes")?}))?;
    let mut boards=BTreeMap::<i64,Vec<StalePost>>::new();
    for post in posts {
        if post.thread.work_status_changed_at.is_some_and(|at|at<=now.ago(SignedDuration::from_secs(post.nudge_minutes.saturating_mul(60)))) {
            boards.entry(post.thread.room_id).or_default().push(post);
        }
    }
    Ok(boards)
}
/// The daily claim commits before the quiet note. Rails retains a won claim if posting fails.
/// Message creation and its broadcasts use the shared message domain, with no unread/inbox/push.
pub async fn dispatch_digests(db: &Database, now: Timestamp) -> Result<DispatchStats> {
    let boards=db.read(move |conn|stale_posts(conn,now)).await?;
    let mut stats=DispatchStats::default();
    let on=now.jiff().to_zoned(TimeZone::UTC).date();
    for (room_id,posts) in boards {
        let result: Result<_>=async {
            let Some(mut claim)=db.write(move |tx|BoardStaleDigest::claim(tx,room_id,on)).await? else { return Ok(false); };
            stats.claims+=1;
            let text=digest_text(&posts,now);
            let creator=posts[0].creator_id;
            db.write(move |tx| {
                let message=Message::create(tx,NewMessage {room_id,creator_id:creator,system_note:true,body:Some(campfire_richtext::ruby::html_escape(&text)),..Default::default()})?;
                claim.attach_message(tx,&message)?;
                let room=Room::find(tx.conn(),room_id)?;
                tx.emit_after_commit(Event::broadcast(&Broadcast::append(room_messages(&room),room_dom_id(&room,Some("messages")),Partial::Message {message_id:message.id})));
                Ok(())
            }).await?;
            Ok(true)
        }.await;
        match result {
            Ok(posted)=>stats.notes+=usize::from(posted),
            Err(error)=>{tracing::error!(room_id,%error,"Board stale digest failed");stats.failed_ids.push(room_id);}
        }
    }
    Ok(stats)
}
fn digest_text(posts: &[StalePost], now: Timestamp) -> String {
    let n=posts.len();
    let mut lines=vec![format!("Stale work digest: {n} {} past its SLA",if n==1 {"post"} else {"posts"})];
    for post in posts.iter().take(20) {
        let minutes=(now.as_microsecond()-post.thread.work_status_changed_at.expect("stale post has entry").as_microsecond()).div_euclid(60_000_000).max(0);
        let age=if minutes<60 {format!("{minutes} {}",if minutes==1 {"minute"} else {"minutes"})}
        else if minutes<1440 {let hours=(minutes as f64/6.0).round()/10.0;format!("{hours:.1} {}",if hours==1.0 {"hour"} else {"hours"})}
        else {let days=(minutes as f64/144.0).round()/10.0;format!("{days:.1} {}",if days==1.0 {"day"} else {"days"})};
        lines.push(format!("{} — {} · {} · {age}",post.thread.name,post.thread.work_status_label(),post.owner_name.as_deref().unwrap_or("Unassigned")));
    }
    if n>20 {lines.push(format!("and {} more",n-20));}
    lines.join("\n")
}
