//! A bot's picked bot avatar through the admin bot API, the personal agent API and the
//! directory: stored with the picture that is its render (in the same write), replaced, cleared by
//! a plain picture (on any path that replaces or removes it), and refused when it names something the library doesn't have or comes without
//! its render.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::admin_tests::{app, error, get, owned_upload, parse, spa, upload, write};
use crate::controllers::presenters::test_support::{Browser, DAVID, KEVIN, Req};

/// Bender: a workspace agent David owns.
const BENDER: i64 = 394959859;

fn cat() -> Value {
    json!({"type": "cat", "color": "#FF8A5C", "face": "mouth", "accessory": "roundGlasses"})
}

fn cat_spec() -> api::BotAvatarSpec {
    api::BotAvatarSpec {
        kind: api::BotAvatarType::Cat,
        color: "#ff8a5c".into(),
        face: api::BotAvatarFace::Mouth,
        accessory: Some(api::BotAvatarAccessory::RoundGlasses),
    }
}

async fn directory_entry(b: &mut Browser<'_>, id: i64) -> api::User {
    let list: api::UserList = parse(&b.send(get(&format!("/api/v1/users?ids={id}"))).await);
    list.users.into_iter().next().expect("the user")
}

#[tokio::test]
async fn a_new_bot_keeps_its_picked_avatar_and_its_render() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    david.grant_sudo().await;
    let render = upload(&a, "bot-avatar.png").await;
    let key: api::BotKey = spa(
        &mut david,
        Method::POST,
        "/api/v1/admin/bots",
        json!({"name": "Whiskers", "iconName": null, "webhookUrl": null, "avatar": render, "botAvatar": cat()}),
    )
    .await;

    let bot: api::Bot = parse(
        &david
            .send(get(&format!("/api/v1/admin/bots/{}", key.id)))
            .await,
    );
    assert_eq!(
        bot.bot_avatar,
        Some(cat_spec()),
        "the colour is stored in lowercase"
    );
    assert!(
        bot.avatar_attached,
        "the render is the picture other surfaces show"
    );

    let list: api::BotList = parse(&david.send(get("/api/v1/admin/bots")).await);
    let listed = list.bots.iter().find(|each| each.id == key.id).unwrap();
    assert_eq!(listed.bot_avatar, Some(cat_spec()));
    assert_eq!(
        list.bots
            .iter()
            .find(|each| each.id == BENDER)
            .unwrap()
            .bot_avatar,
        None,
        "bots without one keep their icon or picture"
    );

    let user = directory_entry(&mut david, key.id).await;
    assert_eq!(user.bot_avatar, Some(cat_spec()));
    assert!(user.has_avatar);
    assert_eq!(directory_entry(&mut david, DAVID).await.bot_avatar, None);
}

#[tokio::test]
async fn an_edit_replaces_the_avatar_and_a_plain_picture_clears_it() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/admin/bots/{BENDER}");
    let before = directory_entry(&mut david, BENDER).await;

    let render = upload(&a, "bot-avatar.png").await;
    let change: api::BotChange = spa(
        &mut david,
        Method::PATCH,
        &path,
        json!({"avatar": render, "botAvatar": cat()}),
    )
    .await;
    assert_eq!(change.bot.bot_avatar, Some(cat_spec()));
    assert!(change.bot.avatar_attached);
    let after = directory_entry(&mut david, BENDER).await;
    assert_ne!(
        after.avatar_url, before.avatar_url,
        "a new avatar URL for the new render"
    );

    let renamed: api::BotChange = spa(
        &mut david,
        Method::PATCH,
        &path,
        json!({"name": "Bender B."}),
    )
    .await;
    assert_eq!(
        renamed.bot.bot_avatar,
        Some(cat_spec()),
        "other edits keep it"
    );

    let picture = upload(&a, "photo.png").await;
    let cleared: api::BotChange =
        spa(&mut david, Method::PATCH, &path, json!({"avatar": picture})).await;
    assert_eq!(
        cleared.bot.bot_avatar, None,
        "the picture is no longer the avatar's render"
    );
    assert!(cleared.bot.avatar_attached);
}

#[tokio::test]
async fn a_spec_the_library_does_not_have_is_refused() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/admin/bots/{BENDER}");
    for bad in [
        json!({"type": "dragon", "color": "#ff8a5c", "face": "eyes", "accessory": null}),
        json!({"type": "cat", "color": "#ff8a5c", "face": "nose", "accessory": null}),
        json!({"type": "cat", "color": "#ff8a5c", "face": "eyes", "accessory": "monocle"}),
    ] {
        let reply = write(&mut david, Method::PATCH, &path, json!({"botAvatar": bad})).await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
    }
    for color in ["orange", "#f80", "#ff8a5cff", "ff8a5c"] {
        let reply = write(
            &mut david,
            Method::PATCH,
            &path,
            json!({"botAvatar": {"type": "cat", "color": color, "face": "eyes", "accessory": null}}),
        )
        .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{color}: {}",
            reply.text()
        );
        assert_eq!(
            error(&reply)["fields"]["botAvatar"][0],
            "needs a colour like #7aa2ff"
        );
    }
    let bot: api::Bot = parse(&david.send(get(&path)).await);
    assert_eq!(bot.bot_avatar, None, "nothing was written");
}

#[tokio::test]
async fn a_personal_agent_takes_a_picked_avatar() {
    let Some(a) = app().await else { return };
    let mut kevin = a.sign_in(KEVIN).await;
    kevin.grant_sudo().await;
    let render = owned_upload(&a, "bot-avatar.png", KEVIN).await;
    let reply = write(
        &mut kevin,
        Method::POST,
        "/api/v1/me/agents",
        json!({"name": "Helper", "roomIds": [], "capabilities": [], "avatar": render, "botAvatar": cat()}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let created: api::PersonalAgentCreated = parse(&reply);
    assert_eq!(created.agent.bot_avatar, Some(cat_spec()));
    assert!(
        created.agent.avatar_url.is_some(),
        "the render is its picture"
    );

    let path = format!("/api/v1/me/agents/{}", created.agent.id);
    let ghost = json!({"type": "ghost", "color": "#ffffff", "face": "eyes", "accessory": null});
    let reply = write(
        &mut kevin,
        Method::PATCH,
        &path,
        json!({"botAvatar": ghost}),
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "a new spec comes with its render: {}",
        reply.text()
    );
    let render = owned_upload(&a, "bot-avatar.png", KEVIN).await;
    let reply = write(
        &mut kevin,
        Method::PATCH,
        &path,
        json!({"avatar": render, "botAvatar": ghost}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let agent: api::PersonalAgent = parse(&reply);
    assert_eq!(
        agent.bot_avatar.map(|spec| spec.kind),
        Some(api::BotAvatarType::Ghost)
    );
    assert_eq!(
        directory_entry(&mut kevin, created.agent.user_id)
            .await
            .bot_avatar
            .map(|spec| spec.kind),
        Some(api::BotAvatarType::Ghost)
    );
}

#[tokio::test]
async fn a_spec_without_its_render_is_refused() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    david.grant_sudo().await;
    let path = format!("/api/v1/admin/bots/{BENDER}");
    let before = directory_entry(&mut david, BENDER).await;

    // Notifications and link previews show the picture: a spec alone would leave them showing
    // another bot than the SPA draws.
    let reply = write(
        &mut david,
        Method::PATCH,
        &path,
        json!({"botAvatar": cat()}),
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert_eq!(
        error(&reply)["fields"]["botAvatar"][0],
        "needs the picture rendered from it"
    );
    let after = directory_entry(&mut david, BENDER).await;
    assert_eq!(after.bot_avatar, None, "nothing was written");
    assert_eq!(after.avatar_url, before.avatar_url);

    let reply = write(
        &mut david,
        Method::POST,
        "/api/v1/admin/bots",
        json!({"name": "Whiskers", "iconName": null, "webhookUrl": null, "avatar": null, "botAvatar": cat()}),
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
}

/// Faults the next write to `table` (an SQLite trigger), as a concurrent or failing save would.
async fn refuse_writes_to(a: &crate::controllers::presenters::test_support::TestApp, table: &str) {
    let sql = format!(
        "DROP TRIGGER IF EXISTS refuse_bot_avatar_test; \
         CREATE TRIGGER refuse_bot_avatar_test BEFORE INSERT ON {table} \
         BEGIN SELECT RAISE(ABORT, 'refused by the test'); END;"
    );
    a.db()
        .write(move |tx| {
            tx.conn().execute_batch(&sql)?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn a_bots_picture_and_its_spec_land_in_one_write() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/admin/bots/{BENDER}");
    let before = directory_entry(&mut david, BENDER).await;

    // The spec's write fails: the picture it was rendered from isn't attached either.
    refuse_writes_to(&a, "bot_avatars").await;
    let render = upload(&a, "bot-avatar.png").await;
    let reply = write(
        &mut david,
        Method::PATCH,
        &path,
        json!({"avatar": render, "botAvatar": cat()}),
    )
    .await;
    assert!(!reply.status.is_success(), "{}", reply.text());
    let after = directory_entry(&mut david, BENDER).await;
    assert_eq!(after.bot_avatar, None);
    assert_eq!(
        after.avatar_url, before.avatar_url,
        "no render without its spec"
    );

    // A write after them fails (the rename's audit): the picture and spec landed together, so
    // the render every other surface shows is the bot the SPA draws.
    refuse_writes_to(&a, "audit_logs").await;
    let render = upload(&a, "bot-avatar.png").await;
    let reply = write(
        &mut david,
        Method::PATCH,
        &path,
        json!({"name": "Bender B.", "avatar": render, "botAvatar": cat()}),
    )
    .await;
    assert!(!reply.status.is_success(), "{}", reply.text());
    let after = directory_entry(&mut david, BENDER).await;
    assert_eq!(after.bot_avatar, Some(cat_spec()), "{}", reply.text());
    assert_ne!(after.avatar_url, before.avatar_url, "with its render");
}

/// Bender picks the cat through the SPA, with its render.
async fn pick_cat(
    a: &crate::controllers::presenters::test_support::TestApp,
    david: &mut Browser<'_>,
) {
    let render = upload(a, "bot-avatar.png").await;
    let change: api::BotChange = spa(
        david,
        Method::PATCH,
        &format!("/api/v1/admin/bots/{BENDER}"),
        json!({"avatar": render, "botAvatar": cat()}),
    )
    .await;
    assert_eq!(change.bot.bot_avatar, Some(cat_spec()));
}

/// The file name of Bender's attached picture.
async fn attached_name(
    a: &crate::controllers::presenters::test_support::TestApp,
) -> Option<String> {
    a.db()
        .read(|conn| {
            crate::controllers::presenters::attachments::attached_blob(
                conn, "User", BENDER, "avatar",
            )
        })
        .await
        .unwrap()
        .map(|blob| blob.filename.to_string())
}

#[tokio::test]
async fn the_classic_form_replacing_or_removing_the_picture_drops_the_spec() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    let classic_path = format!("/account/bots/{BENDER}");
    // A plain picture through the classic form: the SPA must stop drawing the cat that
    // notifications no longer show.
    pick_cat(&a, &mut david).await;
    let picture = upload(&a, "photo.png").await;
    let reply = david
        .write(Req::new(Method::PATCH, &classic_path).form(&[("user[avatar]", &picture)]))
        .await;
    assert!(reply.status.is_redirection(), "{}", reply.text());
    let replaced = directory_entry(&mut david, BENDER).await;
    assert_eq!(
        replaced.bot_avatar, None,
        "the picture is no longer the cat's render"
    );
    assert_eq!(
        attached_name(&a).await.as_deref(),
        Some("photo.png"),
        "the plain picture replaced the render"
    );

    // The picture removed: the spec goes with it.
    pick_cat(&a, &mut david).await;
    let reply = david
        .write(Req::new(Method::PATCH, &classic_path).form(&[("user[avatar]", "")]))
        .await;
    assert!(reply.status.is_redirection(), "{}", reply.text());
    let removed = directory_entry(&mut david, BENDER).await;
    assert_eq!(removed.bot_avatar, None);
    assert_eq!(attached_name(&a).await, None);

    // Other classic edits leave both alone.
    pick_cat(&a, &mut david).await;
    let reply = david
        .write(Req::new(Method::PATCH, &classic_path).form(&[("user[name]", "Bender B.")]))
        .await;
    assert!(reply.status.is_redirection(), "{}", reply.text());
    assert_eq!(
        directory_entry(&mut david, BENDER).await.bot_avatar,
        Some(cat_spec())
    );
}

#[tokio::test]
async fn a_personal_agents_plain_picture_drops_its_spec() {
    let Some(a) = app().await else { return };
    let mut kevin = a.sign_in(KEVIN).await;
    kevin.grant_sudo().await;
    let render = owned_upload(&a, "bot-avatar.png", KEVIN).await;
    let created: api::PersonalAgentCreated = parse(
        &write(
            &mut kevin,
            Method::POST,
            "/api/v1/me/agents",
            json!({"name": "Helper", "roomIds": [], "capabilities": [], "avatar": render, "botAvatar": cat()}),
        )
        .await,
    );
    assert_eq!(created.agent.bot_avatar, Some(cat_spec()));

    let picture = owned_upload(&a, "photo.png", KEVIN).await;
    let reply = write(
        &mut kevin,
        Method::PATCH,
        &format!("/api/v1/me/agents/{}", created.agent.id),
        json!({"avatar": picture}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let agent: api::PersonalAgent = parse(&reply);
    assert_eq!(agent.bot_avatar, None);
    assert_eq!(
        directory_entry(&mut kevin, created.agent.user_id)
            .await
            .bot_avatar,
        None
    );
}
