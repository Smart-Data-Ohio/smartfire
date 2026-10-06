use super::room_delete_test::{assert_checks, setup};
#[test]
fn retention_survivors_and_unlinked_grants_match_rails() {
    let g: serde_json::Value =
        serde_json::from_str(include_str!("ws8_retention_vectors.json")).unwrap();
    let t = setup(&g);
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(crate::models::retention::perform(&t.db))
        .unwrap();
    assert_checks(&t, &g["checks"]);
    let jobs: Vec<_> = t
        .events()
        .iter()
        .filter_map(|e| match e {
            crate::Event::Job(j) if j.class == "Room::DestroyJob" => {
                j.arguments["room_id"].as_i64()
            }
            _ => None,
        })
        .collect();
    assert_eq!(serde_json::to_value(jobs).unwrap(), g["room_jobs"]);
}
