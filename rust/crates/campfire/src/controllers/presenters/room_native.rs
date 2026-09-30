//! Mount owner components in the request rendering scope. Never cache composer/template HTML.
use askama::Template;
use campfire_views::{ViewContext, helpers as h, rooms::{Show,ShowView}};
use campfire_views::messages::composer::{Composer,Facts};
pub(crate) fn components(ctx:&ViewContext<'_>,user:&campfire_views::messages::UserView,composer:&Facts)->askama::Result<(String,String)> {
    let schedule=h::raw(campfire_views::scheduled_messages::ComposerButton{ctx,room_id:composer.room_id,thread_id:None}.render()?);
    // Rails captures the inline composer into content_for(:footer): the caller contributes
    // two spaces before its first line. The owner partial starts with one source newline.
    let inline=Composer{ctx,facts:composer,scheduled_control:&schedule}.render()?;
    let footer=format!("  {}", inline.strip_prefix('\n').unwrap_or(&inline));
    // Rails preserves the source newline after the pending-message script.
    let template=format!("{}\n",campfire_views::channel_threads::PendingTemplate{ctx,user}.render()?);
    Ok((footer,template))
}
pub fn render(ctx:&ViewContext<'_>,show:&ShowView,composer:&Facts,frame:bool)->askama::Result<String> {
    let mut show=show.clone();
    let (composer,template)=components(ctx,&show.user,composer)?;
    show.shell.composer=Some(composer);
    show.shell.message_template=Some(template);
    let page=Show{ctx,show:&show};
    if frame {campfire_views::layouts::frame(ctx,page.as_head(),page.as_content())} else {page.render()}
}
