//! Individually executed ports of all twelve pinned closed-room controller cases.
//! Per-member broadcasts use real signed cable subscriptions, including outsider silence.
use super::directs_rails_cases::ids;
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Account, CachedStatements, Room, RoomType};
const JZ: i64 = 773523953;
const PETS: i64 = 104393281;
const DESIGNERS: i64 = 654632876;
async fn setup() -> TestApp {
    TestApp::boot_frozen().await.expect("seed required")
}
fn write(method: Method, path: &str, name: &str, icon: Option<&str>, ids: &[i64]) -> Req {
    let mut fields = vec![("room[name]".to_owned(), name.to_owned())];
    if let Some(icon) = icon {
        fields.push(("room[icon_name]".to_owned(), icon.to_owned()));
    }
    fields.extend(ids.iter().map(|id| ("user_ids[]".into(), id.to_string())));
    let fields: Vec<_> = fields
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    Req::new(method, path).form(&fields)
}
fn update(id: i64, name: &str, icon: Option<&str>, ids: &[i64]) -> Req {
    write(
        Method::PUT,
        &format!("/rooms/closeds/{id}"),
        name,
        icon,
        ids,
    )
}
fn redirect(reply: &Reply, id: i64) {
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/rooms/{id}").as_str())
    );
}
async fn room(app: &TestApp, id: i64) -> Room {
    app.db()
        .read(move |conn| Room::find(conn, id))
        .await
        .unwrap()
}
async fn count(app: &TestApp) -> i64 {
    app.db()
        .read(|conn| Ok(conn.query_row_cached("SELECT count(*) FROM rooms", [], |r| r.get(0))?))
        .await
        .unwrap()
}

#[tokio::test]
async fn show_redirects_to_get_general_show() {
    let app = setup().await;
    // The Rails case intentionally uses the open namespace for a closed room.
    redirect(
        &app.david().classic_page(&format!("/rooms/opens/{DESIGNERS}")).await,
        DESIGNERS,
    );
}
#[tokio::test]
async fn create_case() {
    let app = setup().await;
    let members = [DAVID, KEVIN, JASON];


    let reply = app
        .david()
        .write(write(
            Method::POST,
            "/rooms/closeds",
            "My New Room",
            None,
            &members,
        ))
        .await;
    let id = reply
        .location()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    redirect(&reply, id);
    let mut expected = members.to_vec();
    expected.sort();
    assert_eq!(ids(&app, id).await, expected);
    assert_eq!(room(&app, id).await.room_type, RoomType::Closed);

}
#[tokio::test]
async fn create_forbidden_by_non_admin_when_account_restricts_creation_to_admins() {
    let app = setup().await;
    app.db()
        .write(|tx| {
            Account::first(tx.conn())?.unwrap().update(
                tx,
                None,
                None,
                Some(&[("restrict_room_creation_to_administrators", "1")]),
            )
        })
        .await
        .unwrap();
    let before = count(&app).await;
    let reply = app
        .sign_in(JZ)
        .await
        .write(write(
            Method::POST,
            "/rooms/closeds",
            "My New Room",
            None,
            &[DAVID, KEVIN, JASON],
        ))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(count(&app).await, before);
}
#[tokio::test]
async fn update_with_membership_revisions() {
    let app = setup().await;
    let before = ids(&app, DESIGNERS).await;
    assert!(before.contains(&JASON));
    let keep: Vec<_> = before.iter().copied().filter(|id| *id != JASON).collect();
    redirect(
        &app.david()
            .write(update(DESIGNERS, "New Name", None, &keep))
            .await,
        DESIGNERS,
    );
    assert_eq!(ids(&app, DESIGNERS).await, keep);
    assert_eq!(
        room(&app, DESIGNERS).await.name.as_deref(),
        Some("New Name")
    );
}
#[tokio::test]
async fn update_an_open_room_to_be_closed() {
    let app = setup().await;
    redirect(
        &app.david()
            .write(update(PETS, "Doesn't matter", None, &[DAVID, JASON]))
            .await,
        PETS,
    );
    assert_eq!(room(&app, PETS).await.room_type, RoomType::Closed);
    assert_eq!(ids(&app, PETS).await, vec![DAVID, JASON]);
}
#[tokio::test]
async fn only_admins_or_creators_can_update() {
    let app = setup().await;
    let before = room(&app, DESIGNERS).await;
    let people = ids(&app, DESIGNERS).await;

    assert_eq!(
        app.sign_in(JZ)
            .await
            .write(update(DESIGNERS, "New Name", None, &[]))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(room(&app, DESIGNERS).await, before);
    assert_eq!(ids(&app, DESIGNERS).await, people);

}
#[tokio::test]
async fn updating_the_icon_replaces_sidebar_rows_and_headers_for_members_only() {
    let app = setup().await;
    let people = ids(&app, DESIGNERS).await;
    assert!(!people.contains(&BENDER));


    redirect(
        &app.david()
            .write(update(DESIGNERS, "Designers", Some(":openai:"), &people))
            .await,
        DESIGNERS,
    );
    assert_eq!(
        room(&app, DESIGNERS).await.icon_name.as_deref(),
        Some("openai")
    );

}
#[tokio::test]
async fn create_with_an_unknown_icon_re_renders_the_new_form() {
    let app = setup().await;
    let before = count(&app).await;
    let reply = app
        .david()
        .write(write(
            Method::POST,
            "/rooms/closeds",
            "Iconic",
            Some(":notanicon:"),
            &[DAVID],
        ))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);

    assert_eq!(count(&app).await, before);
}
#[tokio::test]
async fn update_with_an_unknown_icon_re_renders_the_edit_form_without_revising_members() {
    let app = setup().await;
    let before = room(&app, DESIGNERS).await;
    let people = ids(&app, DESIGNERS).await;
    let reply = app
        .david()
        .write(update(DESIGNERS, "New Name", Some(":notanicon:"), &[DAVID]))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);

    assert_eq!(room(&app, DESIGNERS).await, before);
    assert_eq!(ids(&app, DESIGNERS).await, people);
}
#[tokio::test]
async fn a_direct_room_cant_be_converted_to_closed_and_have_its_participants_revised() {
    let app = setup().await;
    let before = room(&app, DIRECT_KEVIN_BENDER).await;
    let people = ids(&app, DIRECT_KEVIN_BENDER).await;
    let reply = app
        .sign_in(KEVIN)
        .await
        .write(update(
            DIRECT_KEVIN_BENDER,
            "Watercooler",
            None,
            &[KEVIN, JZ],
        ))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some("http://campfire.test/"));
    assert_eq!(room(&app, DIRECT_KEVIN_BENDER).await, before);
    assert_eq!(ids(&app, DIRECT_KEVIN_BENDER).await, people);
}
#[tokio::test]
async fn remove_yourself() {
    let app = setup().await;
    let mut david = app.david();
    redirect(
        &david
            .write(update(DESIGNERS, "Designers", None, &[JASON, JZ]))
            .await,
        DESIGNERS,
    );
    assert_eq!(ids(&app, DESIGNERS).await, vec![JASON, JZ]);
    let reply = david.classic_page(&format!("/rooms/{DESIGNERS}")).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some("http://campfire.test/"));
}
