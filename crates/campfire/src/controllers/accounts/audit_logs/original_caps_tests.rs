//! Original per-test CSV-limit replacements, through the unchanged action body.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_kit::{ActionFn, Ctx};
use serde_json::{Value, json};
#[derive(Clone)]
struct LimitAction(i64);
impl<'a> ActionFn<'a> for LimitAction {
    type Fut = futures_util::future::BoxFuture<'a, campfire_kit::Result>;
    fn call(&self, c: &'a mut Ctx) -> Self::Fut {
        c.set_current(super::TestExportLimit(self.0));
        Box::pin(crate::controllers::dispatch(c))
    }
}
async fn run(name: &str, limit: i64) {
    let mut app = TestApp::boot_frozen()
        .await
        .expect("CI seed required")
        .without_job_runner()
        .await;
    app.db().write(move|tx|{
  tx.conn().execute("DELETE FROM audit_logs",[])?;
  for action in ["user.ban","user.role.change"].into_iter().chain((limit==2).then_some("user.ban")) {
   tx.conn().execute("INSERT INTO audit_logs(action,details,created_at,updated_at) VALUES(?,'{}',?,?)",rusqlite::params![action,tx.now(),tx.now()])?;
  }Ok(())
 }).await.unwrap();
    let kit = campfire_kit::Kit::new(
        campfire_kit::KitConfig::production(true),
        std::sync::Arc::new(campfire_kit::RailsCrypto::new(
            app.booted.app.secrets.clone(),
        )),
        seed_clock(),
        app.booted.app.clone(),
    );
    let action = LimitAction(limit);
    app.booted.router = campfire_kit::app(
        axum::Router::new()
            .route(
                "/{*path}",
                campfire_kit::get(action.clone()).merge(campfire_kit::post(action.clone())),
            )
            .route("/", campfire_kit::get(action))
            .merge(crate::controllers::spa::routes("public, max-age=31536000")),
        kit,
    );
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../../vectors/audit_caps_originals.json"
    ))
    .unwrap();
    let case = oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    let mut browser = app.david();
    assert_eq!(
        browser
            .write(Req::new(Method::POST, "/sudo").form(&[("password", "secret123456")]))
            .await
            .status,
        StatusCode::FOUND
    );
    let csv = browser.get("/account/audit_log.csv").await;
    assert_eq!(csv.status, StatusCode::OK, "original cap CSV status");
    let count = app
        .db()
        .read(|c| {
            Ok(c.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| {
                r.get::<_, i64>(0)
            })?)
        })
        .await
        .unwrap();
    let rows = csv.text().lines().count() - 1;
    let actual = json!({"truncated":csv.header("content-disposition").unwrap().contains("truncated"),"filename_cap":csv.header("content-disposition").unwrap().contains(&format!("truncated-to-{limit}")),"rows":rows,"matches_table":rows as i64==count});
    assert_eq!(
        actual, { let mut expected = case["response"].clone(); expected.as_object_mut().unwrap().remove("notice"); expected },
        "{name}: exact original reduced-cap clauses"
    );
}
#[tokio::test]
async fn original_two_row_cap_warns_names_and_limits_csv() {
    run("past", 2).await;
}
#[tokio::test]
async fn original_five_row_cap_keeps_all_rows_without_warning() {
    run("within", 5).await;
}
