//! Google callbacks through the real router, cookie session, JWT verifier and SQLite writer.
use crate::{controllers::presenters::test_support::{TestApp, Browser, Req, DAVID, JASON, KEVIN}, integrations::{google::{sign_in::{Config, SignIn},client::{Client,Unavailable}},net::BoxFuture}};
use axum::http::{Method,StatusCode};
use base64::{Engine,engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value,json};
use std::sync::{Arc,Mutex};

struct Recorded { response:Mutex<Result<(u16,Vec<u8>),()>>, calls:Mutex<Vec<(String,String,Vec<u8>)>> }
impl Client for Recorded {
    fn request<'a>(&'a self,host:&'a str,method:Method,target:&'a str,_headers:Vec<(String,String)>,body:Vec<u8>)->BoxFuture<'a,Result<(u16,Vec<u8>),Unavailable>> {
        Box::pin(async move {
            self.calls.lock().unwrap().push((method.to_string(),format!("{host}{target}"),body));
            if host=="www.googleapis.com" && target=="/oauth2/v3/certs" {return Ok((200,include_bytes!("../integrations/google/test-jwks.json").to_vec()));}
            assert_eq!((host,method,target),("oauth2.googleapis.com",Method::POST,"/token"));
            self.response.lock().unwrap().clone().map_err(|_|Unavailable)
        })
    }
}
async fn app()->(TestApp,Arc<Recorded>) {
    let app=TestApp::boot().await.expect("pinned default seed required");
    let recorded=Arc::new(Recorded {response:Mutex::new(Err(())),calls:Mutex::new(vec![])});
    app.booted.app.google.install(SignIn::with_client(Config {client_id:"test-client-id".into(),client_secret:"FAKE-google-client-secret".into(),domains:vec!["smartdata.net".into()]},recorded.clone()));
    app.db().write(|tx| {tx.conn().execute("DELETE FROM audit_logs",[])?; tx.conn().execute("DELETE FROM google_identities",[])?; Ok(())}).await.unwrap();
    (app,recorded)
}
fn token(claims:Value)->String {
    use ring::signature::{RsaKeyPair,RSA_PKCS1_SHA256};
    let key=RsaKeyPair::from_pkcs8(include_bytes!("../integrations/google/signing.der")).unwrap();
    let input=format!("{}.{}",URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","kid":"fixture"}"#),URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap()));
    let mut signature=vec![0;key.public().modulus_len()];
    key.sign(&RSA_PKCS1_SHA256,&ring::rand::SystemRandom::new(),input.as_bytes(),&mut signature).unwrap();
    format!("{input}.{}",URL_SAFE_NO_PAD.encode(signature))
}
async fn start(b:&mut Browser<'_>,path:&str)->std::collections::BTreeMap<String,String> {
    let r=b.write(Req::new(Method::POST,path)).await;
    assert_eq!(r.status,StatusCode::FOUND,"{}",r.text());
    let u=url::Url::parse(r.location().unwrap()).unwrap();
    assert_eq!(u.host_str(),Some("accounts.google.com"));
    u.query_pairs().map(|(k,v)|(k.into_owned(),v.into_owned())).collect()
}
fn claims(a:&TestApp,q:&std::collections::BTreeMap<String,String>,subject:&str,email:&str)->Value {
    let now=a.booted.app.clock.now().as_second();
    json!({"iss":"https://accounts.google.com","aud":"test-client-id","sub":subject,"email":email,"email_verified":true,"hd":"smartdata.net","name":"Alice","nonce":q["nonce"],"exp":now+3600,"auth_time":now})
}
fn answer(r:&Recorded,v:Value) { *r.response.lock().unwrap()=Ok((200,serde_json::to_vec(&json!({"id_token":token(v)})).unwrap())); }
async fn callback(b:&mut Browser<'_>,state:&str)->crate::controllers::presenters::test_support::Reply {
    b.get(&format!("/session/google/callback?state={}&code=fixture-code",crate::controllers::presenters::test_support::encode(state))).await
}
async fn actions(a:&TestApp)->Vec<String> {
    a.db().read(|c|Ok(c.prepare("SELECT action FROM audit_logs ORDER BY id")?.query_map([],|r|r.get(0))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap()
}
async fn sessions(a:&TestApp)->i64 {a.db().read(|c|Ok(c.query_row("SELECT COUNT(*) FROM sessions",[],|r|r.get(0))?)).await.unwrap()}
#[tokio::test]
async fn provisions_then_subject_signs_in_without_repeating_link_audit() {
    let (a,r)=app().await; let mut b=a.anonymous(); b.get("/session/new").await;
    let before=sessions(&a).await; let q=start(&mut b,"/session/google").await;
    assert_eq!(q["scope"],"openid email profile"); assert!(!q.contains_key("prompt"));
    answer(&r,claims(&a,&q,"alice","alice@smartdata.net"));
    assert_eq!(callback(&mut b,&q["state"]).await.location(),Some("http://campfire.test/"));
    assert_eq!(sessions(&a).await,before+1); assert_eq!(actions(&a).await,vec!["user.create","session.sign_in.success"]);
    let mut b=a.anonymous();b.get("/session/new").await;let q=start(&mut b,"/session/google").await;
    answer(&r,claims(&a,&q,"alice","new@smartdata.net")); callback(&mut b,&q["state"]).await;
    assert_eq!(actions(&a).await,vec!["user.create","session.sign_in.success","session.sign_in.success"]);
    let body=r.calls.lock().unwrap()[0].2.clone(); let form=url::form_urlencoded::parse(&body).collect::<std::collections::BTreeMap<_,_>>();
    assert_eq!(form["redirect_uri"],"http://campfire.test/session/google/callback"); assert_eq!(form["grant_type"],"authorization_code"); assert!(form.contains_key("code_verifier"));
}
#[tokio::test]
async fn security_wrong_state_consumes_flow_without_google_or_failure_audit() {
    let (a,r)=app().await; let mut b=a.anonymous();b.get("/session/new").await; let q=start(&mut b,"/session/google").await;
    assert_eq!(callback(&mut b,"forged").await.location(),Some("http://campfire.test/session/new"));
    answer(&r,claims(&a,&q,"alice","alice@smartdata.net"));callback(&mut b,&q["state"]).await;
    assert!(r.calls.lock().unwrap().is_empty());assert!(actions(&a).await.is_empty());
}
#[tokio::test]
async fn security_signed_tokens_with_bad_nonce_audience_domain_or_expiry_never_create_session() {
    let (a,r)=app().await;
    for (key,value) in [("nonce",json!("wrong")),("aud",json!("other")),("hd",json!("evil.test")),("exp",json!(1))] {
        let mut b=a.anonymous();b.get("/session/new").await;let q=start(&mut b,"/session/google").await;
        let before=sessions(&a).await; let mut v=claims(&a,&q,"alice","alice@smartdata.net");v[key]=value;answer(&r,v);
        assert_eq!(callback(&mut b,&q["state"]).await.location(),Some("http://campfire.test/session/new"));assert_eq!(sessions(&a).await,before);
    }
    assert_eq!(actions(&a).await,vec!["session.sign_in.failure";4]);
}
#[tokio::test]
async fn link_owns_flow_and_sudo_and_reauth_require_fresh_matching_subject() {
    let (a,r)=app().await; let mut b=a.sign_in(DAVID).await;b.get("/users/me/profile").await;
    let q=start(&mut b,"/user/profile/google_sign_in_link").await;answer(&r,claims(&a,&q,"david","different@smartdata.net"));
    assert_eq!(callback(&mut b,&q["state"]).await.location(),Some("http://campfire.test/users/me/profile"));
    assert_eq!(actions(&a).await,vec!["google.sign_in.link"]);
    for (path,action) in [("/two_factor_reauthentication","two_factor.reauthenticate"),("/sudo/google","sudo.confirm.success")] {
        let q=start(&mut b,path).await;assert_eq!(q["prompt"],"login");assert_eq!(q["max_age"],"0");
        let mut v=claims(&a,&q,"david","different@smartdata.net");v["auth_time"]=json!(a.booted.app.clock.now().as_second()-3600);answer(&r,v);
        callback(&mut b,&q["state"]).await;assert!(!actions(&a).await.iter().any(|s|s==action));
        let q=start(&mut b,path).await;answer(&r,claims(&a,&q,"david","different@smartdata.net"));callback(&mut b,&q["state"]).await;
        assert!(actions(&a).await.iter().any(|s|s==action));
    }
    let q=start(&mut b,"/sudo/google").await;answer(&r,claims(&a,&q,"other","other@smartdata.net"));
    assert_eq!(callback(&mut b,&q["state"]).await.location(),Some("http://campfire.test/sudo/new"));assert_eq!(actions(&a).await.last().unwrap(),"sudo.confirm.failure");
    let mut b=a.sign_in(JASON).await;b.get("/users/me/profile").await;
    assert_eq!(callback(&mut b,&q["state"]).await.location(),Some("http://campfire.test/"));
}
#[tokio::test]
async fn session_insert_failure_rolls_back_provision_identity_and_audits() {
    let (a,r)=app().await; let mut b=a.anonymous();b.get("/session/new").await;let q=start(&mut b,"/session/google").await;
    a.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_google_session BEFORE INSERT ON sessions BEGIN SELECT RAISE(ABORT,'fixture rollback'); END;")?;Ok(())}).await.unwrap();
    answer(&r,claims(&a,&q,"rollback","rollback@smartdata.net"));assert_eq!(callback(&mut b,&q["state"]).await.status,StatusCode::INTERNAL_SERVER_ERROR);
    let count=a.db().read(|c|Ok(c.query_row("SELECT COUNT(*) FROM users WHERE email_address='rollback@smartdata.net'",[],|r|r.get::<_,i64>(0))?)).await.unwrap();assert_eq!(count,0);assert!(actions(&a).await.is_empty());
}
#[tokio::test]
async fn admin_trust_and_unlink_audit_only_actual_changes_and_deny_members() {
    let (a,_)=app().await; a.db().write(|tx| {tx.conn().execute("UPDATE users SET email_self_changed_at=?,google_email_link_allowed=0 WHERE id=?",rusqlite::params![tx.now(),JASON])?;Ok(())}).await.unwrap();
    let path=format!("/account/users/{JASON}/google_link");let mut member=a.sign_in(KEVIN).await;member.get("/users/me/profile").await;
    assert_eq!(member.write(Req::new(Method::POST,&path)).await.status,StatusCode::FORBIDDEN);
    let mut admin=a.sign_in(DAVID).await;admin.get("/users/me/profile").await;
    for _ in 0..2 {assert_eq!(admin.write(Req::new(Method::POST,&path)).await.location(),Some("http://campfire.test/account/edit"));}
    assert_eq!(actions(&a).await,vec!["google.sign_in.link_allow"]);
    for _ in 0..2 {admin.write(Req::new(Method::DELETE,&path)).await;}
    assert_eq!(actions(&a).await,vec!["google.sign_in.link_allow"]);
}
#[tokio::test]
async fn enrolled_google_sign_in_is_pending_until_second_factor_and_remembered_device_skips_challenge() {
    let (a,r)=app().await;
    a.db().write(|tx| {
        campfire_db::models::google_identity::GoogleIdentity::link_to_user(tx,json!({"sub":"david","email":"david@smartdata.net","hd":"smartdata.net"}).as_object().unwrap(),DAVID)?;Ok(())
    }).await.unwrap();
    let mut b=a.anonymous();b.get("/session/new").await;let q=start(&mut b,"/session/google").await;let before=sessions(&a).await;
    answer(&r,claims(&a,&q,"david","david@smartdata.net"));assert_eq!(callback(&mut b,&q["state"]).await.location(),Some("http://campfire.test/two_factor_challenge"));assert_eq!(sessions(&a).await,before);assert!(actions(&a).await.is_empty());
    assert_eq!(b.get("/two_factor_challenge").await.status,StatusCode::OK);
    let token=a.db().write(|tx|Ok(campfire_db::TwoFactorRememberedDevice::create_for(tx,DAVID,None,None)?.1)).await.unwrap();
    use campfire_kit::Crypto;
    let signed=campfire_kit::RailsCrypto::new(a.booted.app.secrets.clone()).sign_cookie("two_factor_remember",&token,None);
    let mut b=a.anonymous();b.absorb_cookie_header(&format!("two_factor_remember={}",campfire_kit::cookies::escape(&signed)));b.get("/session/new").await;let q=start(&mut b,"/session/google").await;
    answer(&r,claims(&a,&q,"david","david@smartdata.net"));assert_eq!(callback(&mut b,&q["state"]).await.location(),Some("http://campfire.test/"));assert_eq!(sessions(&a).await,before+1);assert_eq!(actions(&a).await,vec!["session.sign_in.success"]);
}
#[tokio::test]
async fn security_link_and_step_up_flow_cannot_move_to_another_signed_in_member() {
    let (a,r)=app().await;let mut owner=a.sign_in(DAVID).await;owner.get("/users/me/profile").await;
    let q=start(&mut owner,"/user/profile/google_sign_in_link").await;
    let flow_cookie=owner.cookie_header().split(';').find(|pair|pair.trim().starts_with("_campfire_session=")).unwrap().trim().to_owned();
    let mut other=a.sign_in(KEVIN).await;other.absorb_cookie_header(&flow_cookie);
    assert_eq!(callback(&mut other,&q["state"]).await.location(),Some("http://campfire.test/users/me/profile"));
    assert!(r.calls.lock().unwrap().is_empty());assert!(actions(&a).await.is_empty());
}
