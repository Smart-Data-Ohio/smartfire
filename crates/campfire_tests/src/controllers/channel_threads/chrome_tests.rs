//! Live request Layout::load inputs, compared with complete pinned Rails chrome components.
use std::sync::Arc;
use campfire_kit::clock::FrozenClock;
use crate::controllers::presenters::test_support::*;

#[tokio::test]
async fn live_thread_chrome_matches_rails_icons_and_viewer_scoped_latest_ten_searches() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!("../../../../../vectors/messaging/live-chrome.json")).unwrap();
    let app = TestApp::boot_with_test_clock(Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()))).await.unwrap();
    let inputs = oracle["inputs"].as_array().unwrap().clone();
    app.db().write(move |tx| {
        tx.conn().execute("DELETE FROM searches WHERE user_id IN (?,?)", (DAVID,JASON))?;
        for input in inputs {
            let time = |key: &str| campfire_db::Timestamp::from_jiff(input[key].as_str().unwrap().parse().unwrap());
            tx.conn().execute("INSERT INTO searches (id,user_id,query,created_at,updated_at) VALUES (?,?,?,?,?)",
                (input["id"].as_i64().unwrap(),input["user_id"].as_i64().unwrap(),input["query"].as_str().unwrap(),time("created_at"),time("updated_at")))?;
        }
        Ok(())
    }).await.unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let names = row["custom"].as_array().unwrap().clone();
        app.db().write(move |tx| {
            for name in names {
                let name = name.as_str().unwrap();
                tx.conn().execute("INSERT OR IGNORE INTO workspace_icons (name,title,creator_id,created_at,updated_at) VALUES (?,?,?,?,?)",
                    (name,format!("Chrome {name}"),DAVID,tx.now(),tx.now()))?;
            }
            Ok(())
        }).await.unwrap();
        let response = app.sign_in(row["user_id"].as_i64().unwrap()).await.get("/rooms/654632876/threads/8").await;
        assert_eq!(response.status, 200);
        let html = response.text();
        assert!(html.contains(row["brand_meta"].as_str().unwrap()), "live icon meta differs: {}",row["brand_meta"]);
        // The complete options/empty-state child is token-free. Its sibling Clear form
        // keeps a real request-local token; no tokens are normalized into the golden.
        let recents = row["recent_html"].as_str().unwrap();
        let options = &recents[recents.find("<ul id=\"global-search-listbox\"").unwrap()..];
        assert!(html.contains(options), "live viewer-scoped recents differ");
        assert_eq!(html.matches("class=\"global-search__option\"").count(), 10);
        let other = if row["user_id"] == DAVID {1} else {0};
        assert!(!html.contains(&format!("data-query=\"Chrome {other}/")), "another viewer's search leaked");
    }
}
