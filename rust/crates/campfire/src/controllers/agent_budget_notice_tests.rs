use super::presenters::test_support::TestApp;

#[tokio::test]
async fn ws11_next3_budget_notice_presenter_batches_are_flat() {
    use super::presenters::activity::Sources;
    for html in [true, false] {
        let mut counts = Vec::new();
        for size in [5_i64, 50] {
            let app = TestApp::boot_frozen()
                .await
                .unwrap()
                .without_job_runner()
                .await;
            let rows=app.db().write(move|tx|{
                let mut rows=Vec::new();
                for i in 0..size {
                    let id=2106910000+i;
                    tx.conn().execute("INSERT INTO agent_budget_notices(id,agent_id,cap,day,created_at,updated_at) VALUES(?,773018776,'messages',date('2026-01-01',? || ' days'),?,?)",rusqlite::params![id,i,tx.now(),tx.now()])?;
                    rows.push(campfire_db::ActivityItem {id,user_id:127326141,source_type:"AgentBudgetNotice".into(),source_id:id,event_type:"agent_budget_exceeded".into(),read_at:None,handled_at:None,created_at:tx.now(),updated_at:tx.now()});
                }
                Ok(rows)
            }).await.unwrap();
            let log = app.db().capture_read_queries();
            app.db()
                .read(move |conn| {
                    if html {
                        Sources::load(conn, &rows)?;
                    } else {
                        Sources::load_json(conn, &rows)?;
                    }
                    Ok(())
                })
                .await
                .unwrap();
            app.db().stop_capturing_read_queries();
            counts.push(
                log.lock()
                    .unwrap()
                    .iter()
                    .filter(|sql| sql.trim_start().to_ascii_uppercase().starts_with("SELECT"))
                    .count(),
            );
        }
        println!(
            "BUDGET_NOTICE_PRELOAD html={html} rows=5/50 SELECTs={}/{}",
            counts[0], counts[1]
        );
        assert_eq!(counts[0], counts[1]);
        assert!(
            counts[0] <= 3,
            "budget notices, agents and users each load once"
        );
    }
}
