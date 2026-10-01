//! Inbox rendering and agent expiry. Mutating item-state actions remain with WS12.
use crate::{app::AppCtx, concerns::{self,Before},controllers::presenters::{activity,page,view_context}};
use campfire_kit::{Ctx,Error,Result,StatusCode,format};
use campfire_views::{activity::Inbox,helpers as h};
use askama::Template;
fn filters(c:&Ctx)->(String,String) {
    let status=c.param_str("status").filter(|s|["unread","read","handled"].contains(s)).unwrap_or("unread").into();
    let kind=c.param_str("type").filter(|s|campfire_views::activity::TYPES.iter().any(|(key,_)|key==s)).unwrap_or("all").into();
    (status,kind)
}
pub async fn index(c:&mut Ctx)->Result {
    concerns::before_actions(c,Before::default()).await?;
    let viewer=concerns::require_current_user(c)?.clone();let viewer_id=viewer.id;
    c.app().db.write(move|tx|campfire_db::AgentApproval::resolve_overdue(tx,Some(viewer_id))).await.map_err(Error::internal)?;
    // WS13's overdue huddle invitation resolver is not yet exported here.
    let (filter,kind)=filters(c);
    let before=c.param_str("before").filter(|s|!s.is_empty()&&s.bytes().all(|c|c.is_ascii_digit())).and_then(|s|s.parse::<i64>().ok());
    let app=c.app().clone();let f=filter.clone();let k=kind.clone();
    let (items,unread,next)=c.app().db.read(move|conn|{
        let accessible=activity::accessible(conn,&viewer)?;
        let unread=accessible.iter().filter(|i|i.unread()).count();
        let cursor=before.and_then(|id|accessible.iter().find(|i|i.id==id).map(|i|(i.updated_at,i.id)));
        let rows=accessible.into_iter().filter(|i| activity::state(i)==f && activity::matches_type(i,&k) && cursor.is_none_or(|pair|(i.updated_at,i.id)<pair)).take(100).collect::<Vec<_>>();
        let next=(rows.len()==100).then(||rows.last().unwrap().id);
        Ok((rows.iter().map(|i|activity::item(conn,&app,i,&viewer)).collect::<campfire_db::Result<Vec<_>>>()?,unread,next))
    }).await.map_err(Error::internal)?;
    let now=c.now();
    let chosen=c.respond_to(&[&format::HTML,&format::TURBO_STREAM])?;
    let mut response=if chosen==&format::TURBO_STREAM {
        page::bare(c,StatusCode::OK,&format::TURBO_STREAM,|ctx|{
            let list=campfire_views::activity::List{ctx,items:&items,filter:&filter,type_filter:&kind,now}.render()?;
            let count=format!("<span id=\"activity-unread-count\" class=\"activity-inbox__count\" {}>{unread}</span>",if unread==0 {"hidden"}else{""});
            let pagination=next.map(|id|h::link_to_text("Older activity",&campfire_views::activity::path(&filter,&kind,Some(id),false),h::attrs().class("btn btn--plain activity-inbox__older")).0).unwrap_or_default();
            let chunks=[("activity-items-list",format!("  <div id=\"activity-items-list\" class=\"activity-inbox__list\" aria-live=\"polite\">\n    {list}\n  </div>\n")),("activity-unread-count",format!("  {count}\n")),("activity-items-pagination",format!("  <div id=\"activity-items-pagination\">\n    {pagination}\n  </div>\n"))];
            Ok(chunks.into_iter().map(|(target,html)|format!("<turbo-stream action=\"replace\" target=\"{target}\"><template>{html}</template></turbo-stream>\n")).collect::<Vec<_>>().join("\n"))
        }).await?
    } else {
        view_context::page_or_frame(c,StatusCode::OK,|ctx|Inbox{ctx,items:&items,filter:&filter,type_filter:&kind,before,next_cursor:next,unread_count:unread,now}.render(),|ctx|{
            let page=Inbox{ctx,items:&items,filter:&filter,type_filter:&kind,before,next_cursor:next,unread_count:unread,now};
            campfire_views::layouts::frame(ctx,page.as_head(),page.as_content())
        }).await?
    };
    response.headers.insert("cache-control", "no-store".parse().unwrap());response.headers.insert("pragma","no-cache".parse().unwrap());Ok(response)
}
pub async fn unread_count(c:&mut Ctx)->Result {
    concerns::before_actions(c,Before::default()).await?;
    let viewer=concerns::require_current_user(c)?.clone();
    let count=c.app().db.read(move|conn|Ok(activity::accessible(conn,&viewer)?.iter().filter(|i|i.unread()).count())).await.map_err(Error::internal)?;
    let mut response=c.json(StatusCode::OK,&serde_json::json!({"unread_count":count}))?;
    response.headers.insert("cache-control","no-store".parse().unwrap());response.headers.insert("pragma","no-cache".parse().unwrap());Ok(response)
}
