//! All five SwitchersControllerTest mappings, including full-request query measurements.
use crate::controllers::presenters::test_support::*;
use axum::http::StatusCode;
use campfire_db::{ChannelThread, Involvement, Membership, NewChannelThread, Room, RoomType};
use serde_json::Value;
const DESIGNERS: i64 = 654632876;
async fn setup() -> TestApp { TestApp::boot_frozen().await.expect("seed required") }
async fn payload(app: &TestApp) -> Value {
    let reply = app.david().get("/switcher.json").await;
    assert_eq!(reply.status, StatusCode::OK);
    reply.json()
}
#[tokio::test]
async fn show_returns_the_user_s_rooms_people_and_recent_threads() {
    let app = setup().await;
    app.db().write(|tx| ChannelThread::create(tx, NewChannelThread { room_id: DESIGNERS, creator_id: DAVID, name: Some("Launch plan".into()), ..Default::default() }).map(|_| ())).await.unwrap();
    let payload = payload(&app).await;
    let rooms = payload["rooms"].as_array().unwrap();
    assert!(rooms.iter().any(|r| r["name"] == "HQ"));
    let designers = rooms.iter().find(|r| r["name"] == "Designers").unwrap();
    assert_eq!(designers["id"], DESIGNERS);
    assert_eq!(designers["kind"], "channel");
    assert_eq!(designers["url"], campfire_routes::room(DESIGNERS));
    let people = payload["people"].as_array().unwrap();
    assert!(people.iter().any(|p| p["name"] == "Jason"));
    assert!(!people.iter().any(|p| p["name"] == "David"));
    assert!(payload["threads"].as_array().unwrap().iter().any(|t| t["name"] == "Launch plan"));
}
#[tokio::test]
async fn show_never_includes_rooms_the_user_cannot_access() {
    let app = setup().await;
    let (secret, direct) = app.db().write(|tx| {
        let secret = Room::create_for(tx, RoomType::Closed, Some("Secret"), JASON, &[JASON])?;
        let direct = Room::create_for(tx, RoomType::Direct, None, JASON, &[JASON, KEVIN])?;
        Membership::find_by_room_and_user(tx.conn(), DESIGNERS, DAVID)?.unwrap().update_involvement(tx, Involvement::Invisible)?;
        Ok((secret.id, direct.id))
    }).await.unwrap();
    let payload = payload(&app).await;
    let ids = payload["rooms"].as_array().unwrap().iter().map(|r| r["id"].as_i64().unwrap()).collect::<Vec<_>>();
    for hidden in [secret, direct, DESIGNERS] { assert!(!ids.contains(&hidden)); }
}
#[tokio::test]
async fn show_links_people_to_their_existing_dm() {
    let app = setup().await;
    let payload = payload(&app).await;
    let people = payload["people"].as_array().unwrap();
    assert_eq!(people.iter().find(|p| p["name"] == "Jason").unwrap()["dm_url"], campfire_routes::room(DIRECT_DAVID_JASON));
    assert!(people.iter().find(|p| p["name"] == "JZ").unwrap()["dm_url"].is_null());
}
#[tokio::test]
async fn show_requires_sign_in() {
    let app = setup().await;
    let reply = app.anonymous().get("/switcher.json").await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some("http://campfire.test/session/new"));
}

async fn seed_switcher_data(db: &campfire_db::Database, offset: usize) {
    db.write(move |tx| {
        for index in 0..3 {
            let room=Room::create_for(tx,RoomType::Closed,Some(&format!("Extra {}",offset+index)),DAVID,&[DAVID,JASON])?;
            ChannelThread::create(tx,NewChannelThread{room_id:room.id,creator_id:DAVID,name:Some(format!("Thread {}",offset+index)),..Default::default()})?;
        }
        Ok(())
    }).await.unwrap();
}

async fn measured_payload(db:&campfire_db::Database,n:usize,router:&axum::Router) -> usize {
    let probe=super::query_probe::SqlProbe::start(db,n).await;
    let reply=super::query_probe::request(router,"/switcher.json").await;
    let statements=probe.finish().await;
    assert_eq!(reply.status,StatusCode::OK);
    assert!(reply.json()["rooms"].as_array().unwrap().len()>=3);
    let count=statements.len();assert!(count>0,"the actual HTTP request must execute queries");count
}
#[tokio::test]
async fn show_costs_a_constant_number_of_queries_as_rooms_people_and_threads_grow() {
    let app=setup().await;
    let db=app.db().clone();let n=app.booted.app.config.db_readers;let router=app.booted.router.clone();
    app.booted.jobs.shutdown(std::time::Duration::from_secs(5)).await;
    seed_switcher_data(&db,0).await;
    super::query_probe::request(&router,"/switcher.json").await;
    let small=measured_payload(&db,n,&router).await;
    seed_switcher_data(&db,10).await;
    let large=measured_payload(&db,n,&router).await;
    assert_eq!(small,large,"full switcher HTTP query count: {small} then {large}");
}
