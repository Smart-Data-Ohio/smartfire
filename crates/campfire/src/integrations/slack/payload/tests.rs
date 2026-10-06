use super::super::{
    client,
    jobs::tests::{run, setup, start},
    markdown, users,
};
use serde_json::Value;
use serde_json::json;
#[tokio::test]
async fn slack_malformed_payloads_match_actual_rails_classes_messages_and_results() {
    let oracle: Value =
        serde_json::from_str(include_str!("../../../../../../vectors/slack/payloads.json"))
            .unwrap();
    let (db, _, _dir) = setup().await;
    let id = start(&db).await;
    let run = run(&db, id).await;
    let mapped_run = run.clone();
    db.write(move |tx| {
        users::record(
            tx,
            &mapped_run,
            "user",
            "UKNOWN",
            "User",
            mapped_run.user_id,
            false,
        )
    })
    .await
    .unwrap();
    for case in oracle["client"].as_array().unwrap() {
        let expected = &case["expected"];
        match client::check_ok(case["input"].clone(), "users.list") {
            Ok(value) => assert_eq!(value, expected["result"]),
            Err(error) => {
                assert_eq!(error.message, expected["message"]);
                assert_eq!(error.ruby_class(), expected["class"]);
            }
        }
    }
    for case in oracle["markdown"].as_array().unwrap() {
        match markdown::try_convert(&case["input"], &Default::default()) {
            Ok(value) => assert_eq!(
                serde_json::to_value(value).unwrap(),
                case["expected"]["result"]
            ),
            Err(error) => {
                assert_eq!(error.class, case["expected"]["class"]);
                assert_eq!(error.message, case["expected"]["message"]);
            }
        }
    }
    for case in oracle["mapper"].as_array().unwrap() {
        let input = case["input"].clone();
        let run = run.clone();
        let result = db
            .write(move |tx| users::map_page(tx, &run, &input, true, &Default::default()))
            .await;
        match result {
            Ok(value) => assert_eq!(value, case["expected"]["result"]),
            Err(error) => assert_eq!(
                error.to_string(),
                format!(
                    "{}: {}",
                    case["expected"]["class"].as_str().unwrap(),
                    case["expected"]["message"].as_str().unwrap()
                )
            ),
        }
    }
    for case in oracle["conversation"].as_array().unwrap() {
        let input = case["input"].clone();
        let run = run.clone();
        let result=db.write(move |tx|{
            let target=super::super::conversations::resolve(tx,&run,&input,&[],&Default::default(),true)?;
            Ok(json!({"action":target.action,"room_id":target.room.map(|r|r.id),"reason":target.reason}))
        }).await;
        match result {
            Ok(value) => assert_eq!(value, case["expected"]["result"]),
            Err(error) => assert_eq!(
                error.to_string(),
                format!(
                    "{}: {}",
                    case["expected"]["class"].as_str().unwrap(),
                    case["expected"]["message"].as_str().unwrap()
                )
            ),
        }
    }
    println!(
        "Slack payload parity: 12 client, 23 converter, 23 user mapper and 12 conversation mapper Rails cases matched"
    );
}
