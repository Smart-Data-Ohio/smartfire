//! Calendar push authentication and durable producer seams (`app/models/calendar/push_channel.rb`).
use crate::{Connection, Tx, Result, User, Event, Job};
use rusqlite::OptionalExtension;
use serde::{Serialize,Deserialize};
use sha2::{Digest,Sha256};
use subtle::ConstantTimeEq;

// Positional Rails job arguments are JSON arrays; MeetingRefresh follows WS17's named payload.
#[derive(Debug,Serialize,Deserialize)] pub struct SyncEntryJob(pub (i64,i64));
impl Job for SyncEntryJob { const CLASS:&'static str="Calendar::SyncEntryJob"; }
#[derive(Debug,Serialize,Deserialize)] pub struct InboundSyncJob(pub (i64,));
impl Job for InboundSyncJob { const CLASS:&'static str="Calendar::InboundSyncJob"; }
#[derive(Debug,Serialize,Deserialize)] pub struct MeetLinkJob(pub (i64,));
impl Job for MeetLinkJob { const CLASS:&'static str="Calendar::MeetLinkJob"; }
#[derive(Debug,Serialize,Deserialize)] pub struct WatchChannelJob(pub (i64,));
impl Job for WatchChannelJob { const CLASS:&'static str="Calendar::WatchChannelJob"; }
#[derive(Debug,Serialize,Deserialize)] pub struct DisconnectCleanupJob(pub (Vec<String>,serde_json::Value,Option<i64>));
impl Job for DisconnectCleanupJob { const CLASS:&'static str="Calendar::DisconnectCleanupJob"; }
#[derive(Debug,Serialize,Deserialize)] pub struct MeetingRefreshJob {pub user_id:i64}
impl Job for MeetingRefreshJob {const CLASS:&'static str="Calendar::MeetingRefreshJob";}
// RemoteDeleteJob already lives in room_delete and serializes [user_id, google_event_id].

#[derive(Debug)]
pub struct PushChannel {pub id:i64,pub user_id:i64,pub channel_id:String,pub token_digest:String}
impl PushChannel {
    pub fn for_channel(conn:&Connection,channel:&str)->Result<Option<Self>> {
        Ok(conn.query_row("SELECT id,user_id,channel_id,token_digest FROM calendar_push_channels WHERE channel_id=? LIMIT 1",[channel],|r|Ok(Self {id:r.get(0)?,user_id:r.get(1)?,channel_id:r.get(2)?,token_digest:r.get(3)?})).optional()?)
    }
    /// Rails validates the associated user, unique user_id, and present channel/digest.
    pub fn create(tx:&Tx<'_>,user_id:i64,channel_id:&str,token_digest:&str)->Result<Self> {
        let mut errors=crate::Errors::default();
        if User::find_by_id(tx.conn(),user_id)?.is_none() {errors.add("user","must exist");}
        if campfire_richtext::ruby::is_blank(channel_id) {errors.add("channel_id","can't be blank");}
        if campfire_richtext::ruby::is_blank(token_digest) {errors.add("token_digest","can't be blank");}
        if tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM calendar_push_channels WHERE user_id=?)",[user_id],|r|r.get::<_,bool>(0))? {errors.add("user_id","has already been taken");}
        errors.into_result()?;
        let now=tx.now();
        tx.conn().execute("INSERT INTO calendar_push_channels(user_id,channel_id,token_digest,created_at,updated_at) VALUES(?,?,?,?,?)",rusqlite::params![user_id,channel_id,token_digest,now,now])?;
        Ok(Self {id:tx.conn().last_insert_rowid(),user_id,channel_id:channel_id.into(),token_digest:token_digest.into()})
    }
    pub fn digest(token:&str)->String { Sha256::digest(token.as_bytes()).iter().map(|b|format!("{b:02x}")).collect() }
    pub fn token_matches(&self,token:&str)->bool {
        !campfire_richtext::ruby::is_blank(token) && !campfire_richtext::ruby::is_blank(&self.token_digest) && bool::from(Self::digest(token).as_bytes().ct_eq(self.token_digest.as_bytes()))
    }
    pub fn claim(&self,tx:&Tx<'_>,number:&str)->Result<bool> {
        let number=crate::models::google_calendar::message_number(number)?;
        if number<=0 {return Ok(false);}
        Ok(tx.conn().execute("UPDATE calendar_push_channels SET last_message_number=?,last_notification_at=? WHERE id=? AND last_message_number<?",rusqlite::params![number,tx.now(),self.id,number])?==1)
    }
}
fn message_number(value:&str)->Result<i64> {
    // String#to_i accepts underscores between decimal digits, and stops at other suffixes.
    let value=value.trim_start_matches(|c:char|c.is_ascii_whitespace());
    let (negative,value)=if let Some(v)=value.strip_prefix('-'){(true,v)}else{(false,value.strip_prefix('+').unwrap_or(value))};
    let bytes=value.as_bytes(); let mut index=0; let mut n=Some(0i64);
    while index<bytes.len() {
        let digit=bytes[index];
        if digit.is_ascii_digit() {n=n.and_then(|n|n.checked_mul(10)?.checked_add(i64::from(digit-b'0')));}
        else if digit==b'_' && index>0 && bytes[index-1].is_ascii_digit() && bytes.get(index+1).is_some_and(u8::is_ascii_digit) {}
        else {break;}
        index+=1;
    }
    if negative {Ok(-n.unwrap_or(i64::MAX))}else{n.ok_or_else(||crate::Error::Other("Calendar message number out of SQLite range".into()))}
}

pub fn notification(tx:&mut Tx<'_>,channel_id:&str,token:&str,state:&str,number:&str)->Result<u16> {
    let Some(channel)=PushChannel::for_channel(tx.conn(),channel_id)? else {return Ok(404)};
    if !channel.token_matches(token) {return Ok(403);}
    match state {
        "exists" if channel.claim(tx,number)? => {
            tx.emit_after_commit(Event::job(&InboundSyncJob((channel.user_id,))));
            tx.emit_after_commit(Event::job(&MeetingRefreshJob {user_id:channel.user_id}));
        }
        "not_exists"=>{
            tx.conn().execute("DELETE FROM calendar_push_channels WHERE id=?",[channel.id])?;
            tx.emit_after_commit(Event::job(&WatchChannelJob((channel.user_id,))));
        }
        _=>{}
    }
    Ok(200)
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn ruby_message_number_prefixes() {
        for (v,n) in [("2abc",2),("  +3.5",3),("-9",-9),("garbage",0),("",0)] {assert_eq!(message_number(v).unwrap(),n);}
    }
    #[test] fn digest_and_tokens_are_exact_and_nonblank() {
        let c=PushChannel {id:1,user_id:1,channel_id:"fixture".into(),token_digest:PushChannel::digest("fixture-token")};
        assert!(c.token_matches("fixture-token"));assert!(!c.token_matches("wrong"));assert!(!c.token_matches(" "));
    }
}
