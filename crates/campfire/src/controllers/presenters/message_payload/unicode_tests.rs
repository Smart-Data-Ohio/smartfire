use super::*;
use campfire_db::ChannelThread;
use campfire_db::User;
use campfire_web::controllers::presenters::Presenter;
use serde_json::Value;
use crate::controllers::presenters::test_support::{DAVID, KEVIN, TestApp};
use campfire_db::{Agent, NewAgent, NewChannelThread, Room, RoomType};

#[tokio::test]
async fn unicode_parity_thread_payload_sorts_human_and_agent_owner_options() {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let thread = app
        .booted
        .app
        .db
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET name='ΟΣ' WHERE id=?", [DAVID])?;
            tx.conn()
                .execute("UPDATE users SET name='οςa' WHERE id=?", [KEVIN])?;
            let mut members = vec![DAVID, KEVIN];
            for name in ["ΟΣ", "οςa"] {
                let user = User::create_integration_bot(tx, name)?;
                Agent::create(
                    tx,
                    NewAgent {
                        user_id: user.id,
                        owner_id: Some(DAVID),
                        ..Default::default()
                    },
                )?;
                members.push(user.id);
            }
            let room = Room::create_for(tx, RoomType::Closed, Some("Casing"), DAVID, &members)?;
            ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: room.id,
                    creator_id: DAVID,
                    name: Some("Casing".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let state = app.booted.app.clone();
    let presenter_state = state.clone();
    let payload = state
        .db
        .read(move |c| {
            let p = Presenter::new(c, &presenter_state, None);
            thread_details(&p, &thread, &User::find(c, DAVID)?, "http://campfire.test")
        })
        .await
        .unwrap();
    let names = payload["work_owner_options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|owner| owner["name"].clone())
        .collect::<Vec<_>>();
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/unicode_casing_parity.json"
    ))
    .unwrap();
    let mut expected = oracle["sigma_names"].as_array().unwrap().clone();
    expected.extend(expected.clone());
    assert_eq!(names, expected);
    assert!(payload["work_owner_options"][0]["human"].as_bool().unwrap());
    assert!(payload["work_owner_options"][2]["agent"].as_bool().unwrap());
}
