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
    show.shell.thread_panel=format!("  {}\n",campfire_views::rooms::panels::ThreadPanel {
        ctx,room:&show.room,neutral_name:show.shell.thread_panel_name.as_deref(),
    }.render()?);
    show.shell.poll_builder=campfire_views::rooms::panels::PollBuilder{ctx,room:&show.room}.render()?;
    show.shell.pins_panel=campfire_views::pins::PanelPartial {
        ctx,room_id:show.room.id,
        room_param_key:show.room.header.as_ref().map(|h|h.param_key.as_str()).unwrap_or(show.room.kind.param_key()),
        count:show.shell.pins_count,
    }.render()?;
    let page=Show{ctx,show:&show};
    if frame {campfire_views::layouts::frame(ctx,page.as_head(),page.as_content())} else {page.render()}
}

/// Only rooms/show contributes this whitespace. The message owner's standalone list seam
/// remains byte-identical to its own collection corpus, including unread/anchor windows.
pub(crate) fn message_list(presenter:&super::Presenter<'_>,records:&[campfire_db::Message],divider_id:Option<i64>,unread_count:i64)->campfire_db::Result<String> {
    presenter.room_message_list(records,divider_id,unread_count).map(|list| {
        if records.is_empty() { "\n".into() } else { format!("\n    \n{list}") }
    })
}

/// The same read-only owner inputs used by the HTTP controller and full-page oracle.
pub(crate) struct NativePage {
    pub show: ShowView,
    pub composer: Facts,
    pub link_fetches: Vec<i64>,
    pub twitter_fetches: Vec<i64>,
    pub github_refreshes: Vec<i64>,
}

pub(crate) fn load(conn: &campfire_db::Connection, app: &crate::app::AppState,
    room: &campfire_db::Room, user: &campfire_db::User, message_id: Option<i64>,
    request_host: Option<String>, cache_base_url: String) -> campfire_db::Result<NativePage> {
    use campfire_db::{Room, Account, Message, Timeline};
    let messages = super::room_shell::find_messages(conn,room.id,message_id)?;
    let membership=campfire_db::Membership::find_by_room_and_user(conn,room.id,user.id)?.ok_or(campfire_db::Error::RecordNotFound("Membership"))?;
    let divider=super::room_shell::unread_divider(conn,&membership,&messages)?;
    let mut presenter = super::Presenter::new(conn, app, request_host);
    presenter.cache_base_url = Some(cache_base_url);
    let original = Room::original(conn)?.is_some_and(|original| original.id == room.id);
    let room_gid = crate::channels::room_gid(room).to_param();
    let drive=presenter.composer_drive_flow(user,app.config.google_picker.is_some() && !user.is_bot())?;
    let composer=presenter.composer_facts(room,user,None,drive)?;
    let list=super::room_native::message_list(&presenter,&messages,divider.message_id,divider.count)?;
    let show = campfire_views::rooms::ShowView {
        shell:campfire_views::rooms::ShellComponents{message_list:Some(list),pins_count:campfire_db::MessagePin::count_for_room(conn,room.id)?,thread_panel_name:Some(presenter.room_display_name(room,None)?),..Default::default()},scroll_to_unread_divider:divider.scroll,jump_to_unread_url:divider.jump_url,unread_divider_message_id:divider.message_id,unread_count:divider.count,unread_divider_index:messages.iter().position(|message|Some(message.id)==divider.message_id),
        room: presenter.room_view(room, user)?,
        updated_at: room.updated_at.jiff(),
        user: super::user_view(&app.secrets, user),
        // The page's message fragments come from the store the render then uses.
        messages: campfire_views::fragment_cache::with(&app.fragment_cache, || presenter.messages(&messages))?,
        invitation: original && !Message::paged(conn, Timeline::Room(room.id))?,
        join_code: Account::first(conn)?.map(|account| account.join_code).unwrap_or_default(),
        messages_stream_name: rails_compat::turbo::signed_stream_name(&app.secrets, &[&room_gid, "messages"]),
        ooo_notice_members:
            crate::controllers::presenters::status_settings::ooo_notice_members(
                conn,
                &app.secrets,
                room,
                user.id,
                app.db.env().now(),
            )?,
    };
    Ok(NativePage { show, composer, link_fetches: presenter.pending_link_fetches(), twitter_fetches: presenter.pending_twitter_fetches(), github_refreshes: presenter.take_github_refreshes() })
}
