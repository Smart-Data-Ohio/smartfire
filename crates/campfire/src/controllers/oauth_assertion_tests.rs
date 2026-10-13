use super::*;
use campfire_app::oauth::assertion;
use ring::{hmac, rand::SystemRandom, signature};

const ISSUER: &str = "https://smartfire.example";
const JWKS: &str = "https://client.example/jwks.json";
const RSA: &[u8] = include_bytes!("../../../../test-support/oauth-jwt/rsa.der");
const EC: &[u8] = include_bytes!("../../../../test-support/oauth-jwt/ec.der");

fn keys(alg: &str) -> Value {
    serde_json::from_str(match alg {
        "RS256" => include_str!("../../../../test-support/oauth-jwt/rsa.jwks.json"),
        "ES256" => include_str!("../../../../test-support/oauth-jwt/ec.jwks.json"),
        _ => panic!("test algorithm"),
    })
    .unwrap()
}
fn document() -> Value {
    let mut value = metadata();
    value["token_endpoint_auth_methods_supported"] = json!(["private_key_jwt", "none"]);
    value["jwks_uri"] = json!(JWKS);
    value
}
fn key_route(value: &Value) -> Route {
    key_route_with_type(value, "application/json")
}
fn key_route_with_type(value: &Value, media_type: &str) -> Route {
    Route::new("GET", "client.example", "/jwks.json", 200)
        .header("Content-Type", media_type)
        .header("Cache-Control", "max-age=600")
        .body(serde_json::to_vec(value).unwrap())
}
async fn jwt_fixture(alg: &str) -> Fixture {
    let mut doc = document();
    // Exercise singular-method negotiation independently of the published array shape.
    if alg == "ES256" {
        doc.as_object_mut()
            .unwrap()
            .remove("token_endpoint_auth_methods_supported");
        doc["token_endpoint_auth_method"] = json!("private_key_jwt");
    }
    jwt_fixture_with_document(alg, doc).await
}
async fn jwt_fixture_with_document(alg: &str, doc: Value) -> Fixture {
    let f = fixture(doc).await;
    // Fetch CIMD first; all subsequent network requests in these cases are JWKS.
    campfire_app::oauth::client::identify(&f.app.booted.app, CLIENT, || Ok(()))
        .await
        .unwrap();
    f.server.replace_routes(vec![key_route(&keys(alg))]);
    f
}
fn claims(f: &Fixture, jti: &str) -> Value {
    let now = f.clock.now().as_second();
    json!({"iss":CLIENT,"sub":CLIENT,"aud":ISSUER,"exp":now+120,"iat":now,"jti":jti})
}
fn header(alg: &str) -> Value {
    json!({"alg":alg,"kid":if alg == "ES256" {"ec"} else {"rsa"}})
}
fn sign(alg: &str, header: &Value, claims: &Value) -> String {
    sign_parts(
        alg,
        &serde_json::to_vec(header).unwrap(),
        &serde_json::to_vec(claims).unwrap(),
    )
}
fn sign_parts(alg: &str, header: &[u8], claims: &[u8]) -> String {
    let message = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(claims)
    );
    let rng = SystemRandom::new();
    let signature = match alg {
        "RS256" | "RS512" => {
            let key = signature::RsaKeyPair::from_pkcs8(RSA).unwrap();
            let mut output = vec![0; key.public().modulus_len()];
            let padding = if alg == "RS512" {
                &signature::RSA_PKCS1_SHA512
            } else {
                &signature::RSA_PKCS1_SHA256
            };
            key.sign(padding, &rng, message.as_bytes(), &mut output)
                .unwrap();
            output
        }
        "ES256" => signature::EcdsaKeyPair::from_pkcs8(
            &signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            EC,
            &rng,
        )
        .unwrap()
        .sign(&rng, message.as_bytes())
        .unwrap()
        .as_ref()
        .to_vec(),
        "HS256" | "HS384" | "HS512" => {
            let public = include_bytes!("../../../../test-support/oauth-jwt/rsa-public.pem");
            let algorithm = match alg {
                "HS384" => hmac::HMAC_SHA384,
                "HS512" => hmac::HMAC_SHA512,
                _ => hmac::HMAC_SHA256,
            };
            hmac::sign(&hmac::Key::new(algorithm, public), message.as_bytes())
                .as_ref()
                .to_vec()
        }
        "none" => vec![],
        _ => panic!("test signer"),
    };
    format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature))
}
fn code_form(code: &str) -> Vec<(&str, &str)> {
    vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("client_id", CLIENT),
        ("redirect_uri", REDIRECT),
        ("code_verifier", VERIFIER),
        ("resource", RESOURCE),
    ]
}
fn refresh_form(token: &str) -> Vec<(&str, &str)> {
    vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", token),
        ("client_id", CLIENT),
        ("resource", RESOURCE),
    ]
}
async fn send(app: &TestApp, form: &[(&str, &str)], jwt: &str) -> Reply {
    send_to(app, "/oauth/token", form, jwt).await
}
async fn send_to(app: &TestApp, path: &str, form: &[(&str, &str)], jwt: &str) -> Reply {
    let mut form = form.to_vec();
    form.extend([
        ("client_assertion_type", assertion::TYPE),
        ("client_assertion", jwt),
    ]);
    app.anonymous()
        .send(Req::new(Method::POST, path).form(&form))
        .await
}
fn invalid_client(reply: Reply) {
    auth_error(reply, "invalid_client");
}
fn auth_error(reply: Reply, error: &str) {
    assert_eq!(reply.status.as_u16(), 400, "{}", reply.text());
    assert_eq!(reply.json()["error"], error);
    assert_eq!(reply.header("cache-control"), Some("no-store"));
}
fn key_fetches(f: &Fixture) -> usize {
    f.server
        .received
        .lock()
        .unwrap()
        .iter()
        .filter(|request| request.target == "/jwks.json")
        .count()
}

#[tokio::test]
async fn oauth_jwt_invalid_authentication_counts_toward_token_rate_limit() {
    let f = jwt_fixture("RS256").await;
    let jwt = sign("HS256", &header("HS256"), &claims(&f, "rate limit"));
    for _ in 0..60 {
        invalid_client(send(&f.app, &code_form("unknown"), &jwt).await);
    }
    let limited = send(&f.app, &code_form("unknown"), &jwt).await;
    assert_eq!(limited.status.as_u16(), 429, "{}", limited.text());
    assert_eq!(limited.json()["error"], "rate_limited");
    assert!(limited.header("retry-after").is_some());
    assert_eq!(key_fetches(&f), 0);
}
async fn code(f: &Fixture) -> String {
    approve(&mut f.app.sign_in(DAVID).await, CLIENT, REDIRECT).await
}

async fn refuses_claim(update: impl FnOnce(&mut Value, i64)) {
    let f = jwt_fixture("RS256").await;
    let code = code(&f).await;
    let mut claim = claims(&f, "negative");
    update(&mut claim, f.clock.now().as_second());
    invalid_client(
        send(
            &f.app,
            &code_form(&code),
            &sign("RS256", &header("RS256"), &claim),
        )
        .await,
    );
    let valid = send(
        &f.app,
        &code_form(&code),
        &sign("RS256", &header("RS256"), &claims(&f, "valid")),
    )
    .await;
    assert_eq!(valid.status.as_u16(), 200, "{}", valid.text());
}

#[tokio::test]
async fn oauth_jwt_expired_without_iat_is_rejected() {
    refuses_claim(|claim, now| {
        claim.as_object_mut().unwrap().remove("iat");
        claim["exp"] = json!(now - 61);
    })
    .await;
}

#[tokio::test]
async fn oauth_jwt_expiry_at_skew_boundary_without_iat_is_rejected() {
    refuses_claim(|claim, now| {
        claim.as_object_mut().unwrap().remove("iat");
        claim["exp"] = json!(now - 60);
    })
    .await;
}

#[tokio::test]
async fn oauth_jwt_expiry_too_far_without_iat_is_rejected() {
    refuses_claim(|claim, now| {
        claim.as_object_mut().unwrap().remove("iat");
        claim["exp"] = json!(now + 301);
    })
    .await;
}

#[tokio::test]
async fn oauth_jwt_future_iat_is_rejected() {
    refuses_claim(|claim, now| claim["iat"] = json!(now + 1)).await;
}

#[tokio::test]
async fn oauth_jwt_iat_lifetime_over_five_minutes_is_rejected() {
    refuses_claim(|claim, now| claim["iat"] = json!(now - 181)).await;
}

#[tokio::test]
async fn oauth_jwt_iat_after_exp_within_skew_is_rejected() {
    refuses_claim(|claim, now| claim["exp"] = json!(now - 1)).await;
}

#[tokio::test]
async fn oauth_jwt_future_nbf_is_rejected() {
    refuses_claim(|claim, now| claim["nbf"] = json!(now + 61)).await;
}

async fn refuses_jwks(route: Route) {
    let f = jwt_fixture("RS256").await;
    let code = code(&f).await;
    let jwt = sign("RS256", &header("RS256"), &claims(&f, "jwks"));
    f.server.replace_routes(vec![route]);
    invalid_client(send(&f.app, &code_form(&code), &jwt).await);
    assert_eq!(key_fetches(&f), 1);
    f.server.replace_routes(vec![key_route(&keys("RS256"))]);
    let valid = send(&f.app, &code_form(&code), &jwt).await;
    assert_eq!(valid.status.as_u16(), 200, "{}", valid.text());
}

fn oversized_jwks() -> Vec<u8> {
    let mut body = serde_json::to_vec(&keys("RS256")).unwrap();
    body.resize(16 * 1024 + 1, b' ');
    body
}

#[tokio::test]
async fn oauth_jwt_jwks_content_length_over_limit_is_rejected() {
    refuses_jwks(key_route(&keys("RS256")).body(oversized_jwks())).await;
}

#[tokio::test]
async fn oauth_jwt_jwks_chunked_body_over_limit_is_rejected() {
    let mut route = key_route(&keys("RS256")).body(oversized_jwks());
    route.chunked = true;
    refuses_jwks(route).await;
}

#[tokio::test]
async fn oauth_jwt_jwks_wrong_media_type_with_valid_keys_is_rejected() {
    refuses_jwks(key_route_with_type(&keys("RS256"), "text/plain")).await;
}

#[tokio::test]
async fn oauth_jwt_jwks_redirect_with_valid_keys_is_rejected() {
    let mut route = key_route(&keys("RS256")).header("Location", JWKS);
    route.status = 302;
    refuses_jwks(route).await;
}

#[tokio::test]
async fn oauth_jwt_correctly_signed_small_rsa_is_rejected() {
    let f = jwt_fixture("RS256").await;
    let code = code(&f).await;
    let values: Value = serde_json::from_str(include_str!(
        "../../../../test-support/oauth-jwt/rsa-1024.jwks.json"
    ))
    .unwrap();
    f.server.replace_routes(vec![key_route(&values)]);
    let jwt = include_str!("../../../../test-support/oauth-jwt/rsa-1024.jwt").trim();
    invalid_client(send(&f.app, &code_form(&code), jwt).await);
    assert_eq!(key_fetches(&f), 1);
}

#[tokio::test]
async fn oauth_jwt_rs256_and_es256_exchange_refresh_and_bind_method() {
    for alg in ["RS256", "ES256"] {
        let f = jwt_fixture(alg).await;
        let code = code(&f).await;
        let mut claim = claims(&f, "exchange");
        if alg == "ES256" {
            claim["aud"] = json!([format!("{ISSUER}/oauth/token")]);
        }
        let mut form = code_form(&code);
        // client_id is optional when the assertion identifies the client.
        form.retain(|(key, _)| *key != "client_id");
        let pair = send(&f.app, &form, &sign(alg, &header(alg), &claim)).await;
        assert_eq!(pair.status.as_u16(), 200, "{alg}: {}", pair.text());
        let pair = pair.json();
        assert_eq!(
            mcp(&f.app, pair["access_token"].as_str().unwrap())
                .await
                .status
                .as_u16(),
            200
        );
        let token = pair["refresh_token"].as_str().unwrap();
        invalid(refresh(&f.app, token, CLIENT, RESOURCE, false).await);
        let secret = token.to_string();
        f.app
            .db()
            .read(move |conn| {
                let token =
                    campfire_db::models::oauth::OAuthToken::find_by_secret(conn, &secret)?.unwrap();
                assert!(token.used_at.is_none());
                let connection = campfire_db::models::oauth::OAuthConnection::find(
                    conn,
                    token.oauth_connection_id,
                )?
                .unwrap();
                assert_eq!(connection.auth_method.as_deref(), Some("private_key_jwt"));
                Ok(())
            })
            .await
            .unwrap();
        let renewed = send(
            &f.app,
            &refresh_form(token),
            &sign(alg, &header(alg), &claims(&f, "refresh")),
        )
        .await;
        assert_eq!(renewed.status.as_u16(), 200, "{}", renewed.text());
        assert_ne!(renewed.json()["refresh_token"], pair["refresh_token"]);
        assert_eq!(key_fetches(&f), 1, "JWKS cache serves refresh");
    }
}

#[tokio::test]
async fn oauth_jwt_header_claim_and_mechanism_refusals_leave_code_unconsumed() {
    let f = jwt_fixture("RS256").await;
    let code = code(&f).await;
    let now = f.clock.now().as_second();
    let mut values = keys("RS256");
    values["keys"][0].as_object_mut().unwrap().remove("alg");
    f.server.replace_routes(vec![key_route(&values)]);
    let mut cases: Vec<(&str, Value, Value)> = vec![];
    for (name, field, value) in [
        (
            "wrong iss",
            "iss",
            json!("https://other.example/metadata.json"),
        ),
        ("wrong sub", "sub", json!("other")),
        ("wrong aud", "aud", json!("https://other.example")),
        ("aud two elements", "aud", json!([ISSUER, ISSUER])),
        ("aud empty array", "aud", json!([])),
        ("aud nonstring", "aud", json!([1])),
        ("exp string", "exp", json!("123")),
        ("iat null", "iat", Value::Null),
        ("nbf null", "nbf", Value::Null),
        ("empty jti", "jti", json!("")),
        ("long jti", "jti", json!("x".repeat(257))),
        ("jti nonstring", "jti", json!(10)),
    ] {
        let mut claim = claims(&f, name);
        claim[field] = value;
        cases.push((name, header("RS256"), claim));
    }
    for field in ["iss", "sub", "aud", "exp", "jti"] {
        let mut claim = claims(&f, field);
        claim.as_object_mut().unwrap().remove(field);
        cases.push((field, header("RS256"), claim));
    }
    for field in ["jwk", "jku", "x5u", "x5c", "crit"] {
        let mut head = header("RS256");
        head[field] = if field == "jwk" {
            keys("RS256")["keys"][0].clone()
        } else {
            json!([])
        };
        cases.push((field, head, claims(&f, field)));
    }
    for alg in ["none", "HS256", "HS384", "HS512", "RS512"] {
        cases.push((alg, header(alg), claims(&f, alg)));
    }
    for (name, head, claim) in cases {
        let signer = match name {
            "none" => "none",
            "HS256" | "HS384" | "HS512" | "RS512" => name,
            _ => "RS256",
        };
        let jwt = sign(signer, &head, &claim);
        assert!(
            assertion::parse(
                &jwt,
                Some(CLIENT),
                ISSUER,
                campfire_db::Timestamp::from_jiff(f.clock.now())
            )
            .is_err(),
            "{name}: parser must reject the invalid header or claim"
        );
        let reply = send(&f.app, &code_form(&code), &jwt).await;
        assert_eq!(
            reply.json()["error"],
            "invalid_client",
            "{name}: {}",
            reply.text()
        );
        invalid_client(reply);
    }
    let jwt = sign("RS256", &header("RS256"), &claims(&f, "mechanisms"));
    assert!(
        assertion::parse(
            &jwt,
            Some(CLIENT),
            ISSUER,
            campfire_db::Timestamp::from_jiff(f.clock.now())
        )
        .is_ok()
    );
    let mut form = code_form(&code);
    form.push(("token_endpoint_auth_method", "none"));
    invalid_client(send(&f.app, &form, &jwt).await);
    for ty in [None, Some("wrong")] {
        let mut form = code_form(&code);
        form.push(("client_assertion", &jwt));
        if let Some(ty) = ty {
            form.push(("client_assertion_type", ty));
        }
        invalid_client(
            f.app
                .anonymous()
                .send(Req::new(Method::POST, "/oauth/token").form(&form))
                .await,
        );
    }
    let mut form = code_form(&code);
    form[2].1 = "https://other.example/metadata.json";
    invalid_client(send(&f.app, &form, &jwt).await);
    let duplicate_header = br#"{"alg":"none","alg":"RS256","kid":"rsa"}"#;
    invalid_client(
        send(
            &f.app,
            &code_form(&code),
            &sign_parts(
                "RS256",
                duplicate_header,
                &serde_json::to_vec(&claims(&f, "duplicate header")).unwrap(),
            ),
        )
        .await,
    );
    let duplicate_claims = format!(
        "{{\"iss\":\"{CLIENT}\",\"sub\":\"{CLIENT}\",\"aud\":\"{ISSUER}\",\"exp\":0,\"exp\":{},\"jti\":\"duplicate claim\"}}",
        now + 120
    );
    invalid_client(
        send(
            &f.app,
            &code_form(&code),
            &sign_parts(
                "RS256",
                &serde_json::to_vec(&header("RS256")).unwrap(),
                duplicate_claims.as_bytes(),
            ),
        )
        .await,
    );
    let corrupt = format!(
        "{}.{}",
        jwt.rsplit_once('.').unwrap().0,
        URL_SAFE_NO_PAD.encode([0u8; 256])
    );
    invalid_client(send(&f.app, &code_form(&code), &corrupt).await);
    let reply = send(&f.app, &code_form(&code), &jwt).await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    assert_eq!(
        key_fetches(&f),
        1,
        "bad signatures must not trigger refetch"
    );
}

#[tokio::test]
async fn oauth_jwt_unknown_kid_refetches_once_and_key_rotation_succeeds() {
    let f = jwt_fixture("RS256").await;
    let code = code(&f).await;
    let mut head = header("RS256");
    head["kid"] = json!("rotated");
    let jwt = sign("RS256", &head, &claims(&f, "rotation"));
    invalid_client(send(&f.app, &code_form(&code), &jwt).await);
    assert_eq!(
        key_fetches(&f),
        2,
        "initial fetch plus one unknown-kid refetch"
    );
    let mut rotated = keys("RS256");
    rotated["keys"][0]["kid"] = json!("rotated");
    f.server.replace_routes(vec![key_route(&rotated)]);
    let reply = send(&f.app, &code_form(&code), &jwt).await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    assert_eq!(
        key_fetches(&f),
        3,
        "cached unknown kid makes exactly one refetch"
    );
}

#[tokio::test]
async fn oauth_jwt_concurrent_assertion_has_one_winner_without_revoking_connection() {
    let f = jwt_fixture("RS256").await;
    let code = code(&f).await;
    let jwt = sign("RS256", &header("RS256"), &claims(&f, "race"));
    let form = code_form(&code);
    let (a, b) = tokio::join!(send(&f.app, &form, &jwt), send(&f.app, &form, &jwt));
    let (winner, loser) = if a.status.as_u16() == 200 {
        (a, b)
    } else {
        (b, a)
    };
    assert_eq!(winner.status.as_u16(), 200, "{}", winner.text());
    invalid_client(loser);
    assert_eq!(
        mcp(&f.app, winner.json()["access_token"].as_str().unwrap())
            .await
            .status
            .as_u16(),
        200
    );
    assert_eq!(key_fetches(&f), 1);
}

#[tokio::test]
async fn oauth_jwt_cache_expires_and_timing_boundaries_are_accepted() {
    let f = jwt_fixture("RS256").await;
    let code = code(&f).await;
    let mut claim = claims(&f, "skew boundary");
    let now = f.clock.now().as_second();
    claim["exp"] = json!(now - 59);
    claim["iat"] = json!(now - 60);
    claim["nbf"] = json!(now + 60);
    claim["aud"] = json!([ISSUER]);
    let pair = send(
        &f.app,
        &code_form(&code),
        &sign("RS256", &header("RS256"), &claim),
    )
    .await;
    assert_eq!(pair.status.as_u16(), 200, "{}", pair.text());
    let pair = pair.json();
    f.clock.advance(jiff::SignedDuration::from_secs(600));
    f.server.replace_routes(vec![
        key_route(&keys("RS256")),
        Route::new("GET", "client.example", "/metadata.json", 200)
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(&document()).unwrap()),
    ]);
    let mut claim = claims(&f, "max lifetime");
    claim["exp"] = json!(f.clock.now().as_second() + 300);
    claim["aud"] = json!(format!("{ISSUER}/oauth/token"));
    let reply = send(
        &f.app,
        &refresh_form(pair["refresh_token"].as_str().unwrap()),
        &sign("RS256", &header("RS256"), &claim),
    )
    .await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    assert_eq!(key_fetches(&f), 2, "expired cache is fetched again");
}

#[tokio::test]
async fn oauth_jwt_replay_survives_a_new_app_on_the_same_database() {
    let f = jwt_fixture("RS256").await;
    let code = code(&f).await;
    let jwt = sign("RS256", &header("RS256"), &claims(&f, "persistent"));
    let pair = send(&f.app, &code_form(&code), &jwt).await;
    assert_eq!(pair.status.as_u16(), 200, "{}", pair.text());
    let pair = pair.json();
    let token = pair["refresh_token"].as_str().unwrap();
    invalid_client(send(&f.app, &refresh_form(token), &jwt).await);
    let fresh_jwt = sign("RS256", &header("RS256"), &claims(&f, "after restart"));
    let network = f.app.booted.app.subscription_network.clone();
    let app = f
        .app
        .restart_with_network(
            f.clock,
            network,
            &[("APP_URL", ISSUER), ("SMARTFIRE_MCP_OAUTH", "1")],
        )
        .await
        .without_job_runner()
        .await;
    assert!(
        app.booted.app.json_cache.is_empty(),
        "new process has no JWKS cache"
    );
    let fresh = send(&app, &refresh_form(token), &fresh_jwt).await;
    assert_eq!(fresh.status.as_u16(), 200, "{}", fresh.text());
    let fresh = fresh.json();
    let fresh_token = fresh["refresh_token"].as_str().unwrap();
    invalid_client(send(&app, &refresh_form(fresh_token), &jwt).await);
    let secret = fresh_token.to_string();
    app.db()
        .read(move |conn| {
            assert!(
                campfire_db::models::oauth::OAuthToken::find_by_secret(conn, &secret)?
                    .unwrap()
                    .used_at
                    .is_none()
            );
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM oauth_assertion_jtis", [], |r| r
                    .get::<_, i64>(0))?,
                2
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn oauth_jwt_refuses_cross_origin_and_guarded_jwks_failures() {
    for uri in [
        "https://other.example/jwks.json",
        "https://client.example:444/jwks.json",
        "http://client.example/jwks.json",
    ] {
        let mut doc = document();
        doc["jwks_uri"] = json!(uri);
        let f = fixture(doc).await;
        let code = code(&f).await;
        invalid_client(
            send(
                &f.app,
                &code_form(&code),
                &sign("RS256", &header("RS256"), &claims(&f, "origin")),
            )
            .await,
        );
        assert_eq!(key_fetches(&f), 0);
    }
    for case in ["private DNS", "bad JSON"] {
        let f = jwt_fixture("RS256").await;
        let code = code(&f).await;
        let mut route = key_route(&keys("RS256"));
        if case == "private DNS" {
            f.resolver
                .set("client.example", vec![vec!["127.0.0.1".parse().unwrap()]]);
        } else {
            route = route.body("{");
        }
        f.server.replace_routes(vec![route]);
        invalid_client(
            send(
                &f.app,
                &code_form(&code),
                &sign("RS256", &header("RS256"), &claims(&f, case)),
            )
            .await,
        );
        assert_eq!(key_fetches(&f), if case == "private DNS" { 0 } else { 1 });
    }
}

#[tokio::test]
async fn oauth_jwt_key_constraints_and_missing_kid_selection() {
    for (alg, case) in [
        ("RS256", "algorithm mismatch"),
        ("ES256", "wrong curve"),
        ("ES256", "wrong point"),
        ("RS256", "duplicate kid"),
        ("RS256", "ambiguous missing kid"),
        ("RS256", "key use"),
        ("RS256", "key ops"),
        ("RS256", "wrong type"),
    ] {
        let f = jwt_fixture(alg).await;
        let code = code(&f).await;
        let mut values = keys(alg);
        let mut head = header(alg);
        match case {
            "algorithm mismatch" => values["keys"][0]["alg"] = json!("ES256"),
            "wrong curve" => values["keys"][0]["crv"] = json!("P-384"),
            "wrong point" => values["keys"][0]["x"] = json!(URL_SAFE_NO_PAD.encode([0u8; 31])),
            "duplicate kid" | "ambiguous missing kid" => {
                let duplicate = values["keys"][0].clone();
                values["keys"].as_array_mut().unwrap().push(duplicate);
                if case == "ambiguous missing kid" {
                    head.as_object_mut().unwrap().remove("kid");
                }
            }
            "key use" => values["keys"][0]["use"] = json!("enc"),
            "key ops" => values["keys"][0]["key_ops"] = json!(["encrypt"]),
            "wrong type" => values["keys"][0]["kty"] = json!("oct"),
            _ => unreachable!(),
        }
        f.server.replace_routes(vec![key_route(&values)]);
        invalid_client(
            send(
                &f.app,
                &code_form(&code),
                &sign(alg, &head, &claims(&f, case)),
            )
            .await,
        );
        assert_eq!(key_fetches(&f), 1, "{case} is not an unknown kid");
    }
    for alg in ["RS256", "ES256"] {
        let f = jwt_fixture(alg).await;
        let code = code(&f).await;
        let mut head = header(alg);
        head.as_object_mut().unwrap().remove("kid");
        let mut claim = claims(&f, "missing kid");
        claim.as_object_mut().unwrap().remove("iat");
        let reply = send(&f.app, &code_form(&code), &sign(alg, &head, &claim)).await;
        assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    }
}

#[tokio::test]
async fn oauth_jwt_review_private_key_only_client_revokes_access_and_refresh() {
    for alg in ["RS256", "ES256"] {
        let mut doc = document();
        doc.as_object_mut()
            .unwrap()
            .remove("token_endpoint_auth_methods_supported");
        doc["token_endpoint_auth_method"] = json!("private_key_jwt");
        let f = jwt_fixture_with_document(alg, doc).await;
        let code = code(&f).await;
        let exchange_jwt = sign(alg, &header(alg), &claims(&f, "exchange"));
        let pair = send(&f.app, &code_form(&code), &exchange_jwt).await;
        assert_eq!(pair.status.as_u16(), 200, "{}", pair.text());
        let pair = pair.json();
        let access = pair["access_token"].as_str().unwrap();
        let form = [("client_id", CLIENT), ("token", access)];
        assert_eq!(
            f.app
                .anonymous()
                .send(Req::new(Method::POST, "/oauth/revoke").form(&form))
                .await
                .status
                .as_u16(),
            200
        );
        assert_eq!(
            mcp(&f.app, access).await.status.as_u16(),
            200,
            "none cannot revoke a JWT connection"
        );
        let revoke_jwt = sign(alg, &header(alg), &claims(&f, "revoke access"));
        // The assertion identifies this client without a client_id form field.
        let reply = send_to(&f.app, "/oauth/revoke", &[("token", access)], &revoke_jwt).await;
        assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
        assert_eq!(
            mcp(&f.app, access).await.status.as_u16(),
            401,
            "revoked access token must stop authenticating"
        );
        let renewed = send(
            &f.app,
            &refresh_form(pair["refresh_token"].as_str().unwrap()),
            &sign(alg, &header(alg), &claims(&f, "refresh")),
        )
        .await;
        assert_eq!(renewed.status.as_u16(), 200, "{}", renewed.text());
        let renewed = renewed.json();
        let access = renewed["access_token"].as_str().unwrap();
        invalid_client(
            send_to(
                &f.app,
                "/oauth/revoke",
                &[("client_id", CLIENT), ("token", access)],
                &sign(
                    "HS256",
                    &header("HS256"),
                    &claims(&f, "bad revoke signature"),
                ),
            )
            .await,
        );
        for used in [&exchange_jwt, &revoke_jwt] {
            invalid_client(
                send_to(
                    &f.app,
                    "/oauth/revoke",
                    &[("client_id", CLIENT), ("token", access)],
                    used,
                )
                .await,
            );
        }
        assert_eq!(
            mcp(&f.app, access).await.status.as_u16(),
            200,
            "replayed revocation assertion cannot revoke another token"
        );
        let refresh = renewed["refresh_token"].as_str().unwrap();
        let reply = send_to(
            &f.app,
            "/oauth/revoke",
            &[("client_id", CLIENT), ("token", refresh)],
            &sign(alg, &header(alg), &claims(&f, "revoke refresh")),
        )
        .await;
        assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
        assert_eq!(
            mcp(&f.app, access).await.status.as_u16(),
            401,
            "revoking refresh revokes the connection"
        );
        invalid(
            send(
                &f.app,
                &refresh_form(refresh),
                &sign(alg, &header(alg), &claims(&f, "after revocation")),
            )
            .await,
        );
    }
}

#[tokio::test]
async fn oauth_jwt_review_revocation_requires_recorded_method_even_when_both_are_allowed() {
    for method in ["none", "private_key_jwt"] {
        let f = jwt_fixture("RS256").await;
        let code = code(&f).await;
        let pair = if method == "none" {
            exchange(&f.app, &code, CLIENT, REDIRECT, VERIFIER, RESOURCE).await
        } else {
            send(
                &f.app,
                &code_form(&code),
                &sign("RS256", &header("RS256"), &claims(&f, "exchange")),
            )
            .await
        };
        assert_eq!(pair.status.as_u16(), 200, "{}", pair.text());
        let pair = pair.json();
        let access = pair["access_token"].as_str().unwrap();
        let form = [("client_id", CLIENT), ("token", access)];
        let wrong = if method == "none" {
            send_to(
                &f.app,
                "/oauth/revoke",
                &form,
                &sign("RS256", &header("RS256"), &claims(&f, "wrong method")),
            )
            .await
        } else {
            f.app
                .anonymous()
                .send(Req::new(Method::POST, "/oauth/revoke").form(&form))
                .await
        };
        assert_eq!(wrong.status.as_u16(), 200);
        assert_eq!(
            mcp(&f.app, access).await.status.as_u16(),
            200,
            "wrong method must not revoke"
        );
        let correct = if method == "none" {
            f.app
                .anonymous()
                .send(Req::new(Method::POST, "/oauth/revoke").form(&form))
                .await
        } else {
            send_to(
                &f.app,
                "/oauth/revoke",
                &form,
                &sign("RS256", &header("RS256"), &claims(&f, "correct method")),
            )
            .await
        };
        assert_eq!(correct.status.as_u16(), 200);
        assert_eq!(mcp(&f.app, access).await.status.as_u16(), 401);
    }
}

#[tokio::test]
async fn oauth_jwt_review_jwks_accepts_registered_media_type() {
    for media_type in [
        "application/json",
        "application/jwk-set+json",
        "Application/JWK-Set+JSON; charset=utf-8",
    ] {
        let f = jwt_fixture("RS256").await;
        f.server
            .replace_routes(vec![key_route_with_type(&keys("RS256"), media_type)]);
        let code = code(&f).await;
        let pair = send(
            &f.app,
            &code_form(&code),
            &sign("RS256", &header("RS256"), &claims(&f, "media type")),
        )
        .await;
        assert_eq!(pair.status.as_u16(), 200, "{media_type}: {}", pair.text());
        assert_eq!(
            mcp(&f.app, pair.json()["access_token"].as_str().unwrap())
                .await
                .status
                .as_u16(),
            200
        );
    }
}

#[tokio::test]
async fn oauth_jwt_review_cimd_remains_json_only() {
    let f = fixture(document()).await;
    f.server.replace_routes(vec![
        Route::new("GET", "client.example", "/metadata.json", 200)
            .header("Content-Type", "application/jwk-set+json")
            .body(serde_json::to_vec(&document()).unwrap()),
    ]);
    let reply = f
        .app
        .anonymous()
        .get(&authorize_url(CLIENT, REDIRECT))
        .await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/app/oauth/error?error=invalid_client")
    );
}

#[tokio::test]
async fn oauth_jwt_review_mixed_mechanisms_return_invalid_request_without_consuming_assertion() {
    let f = jwt_fixture("RS256").await;
    let code = code(&f).await;
    for path in ["/oauth/token", "/oauth/revoke"] {
        let jwt = sign("RS256", &header("RS256"), &claims(&f, path));
        for basic in [false, true] {
            let mut form = if path == "/oauth/token" {
                code_form(&code)
            } else {
                vec![("client_id", CLIENT), ("token", "unknown")]
            };
            form.extend([
                ("client_assertion", jwt.as_str()),
                ("client_assertion_type", assertion::TYPE),
            ]);
            if !basic {
                form.push(("client_secret", "secret"));
            }
            let mut req = Req::new(Method::POST, path).form(&form);
            if basic {
                req = req.header("authorization", "Basic dGVzdDpzZWNyZXQ=");
            }
            auth_error(f.app.anonymous().send(req).await, "invalid_request");
        }
        let reply = f
            .app
            .anonymous()
            .send(
                Req::new(Method::POST, path)
                    .form(&[
                        ("client_id", CLIENT),
                        ("client_secret", "secret"),
                        ("token", "unknown"),
                    ])
                    .header("authorization", "Basic dGVzdDpzZWNyZXQ="),
            )
            .await;
        auth_error(reply, "invalid_request");
    }
    let jwt = sign("RS256", &header("RS256"), &claims(&f, "/oauth/token"));
    let reply = send(&f.app, &code_form(&code), &jwt).await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    let pair = reply.json();
    let access = pair["access_token"].as_str().unwrap();
    let jwt = sign("RS256", &header("RS256"), &claims(&f, "/oauth/revoke"));
    let reply = send_to(
        &f.app,
        "/oauth/revoke",
        &[("client_id", CLIENT), ("token", access)],
        &jwt,
    )
    .await;
    assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
    assert_eq!(mcp(&f.app, access).await.status.as_u16(), 401);
}

#[tokio::test]
async fn oauth_jwt_missing_required_authentication_returns_invalid_client_without_consuming_code() {
    for alg in ["RS256", "ES256"] {
        let f = if alg == "RS256" {
            let mut doc = document();
            doc["token_endpoint_auth_methods_supported"] = json!(["private_key_jwt"]);
            jwt_fixture_with_document(alg, doc).await
        } else {
            jwt_fixture(alg).await
        };
        let code = code(&f).await;
        for requested_method in [None, Some("none"), Some("private_key_jwt")] {
            let mut form = code_form(&code);
            if let Some(method) = requested_method {
                form.push(("token_endpoint_auth_method", method));
            }
            auth_error(
                f.app
                    .anonymous()
                    .send(Req::new(Method::POST, "/oauth/token").form(&form))
                    .await,
                "invalid_client",
            );
        }
        let reply = send(
            &f.app,
            &code_form(&code),
            &sign(alg, &header(alg), &claims(&f, "authenticated exchange")),
        )
        .await;
        assert_eq!(reply.status.as_u16(), 200, "{}", reply.text());
        assert_eq!(
            mcp(&f.app, reply.json()["access_token"].as_str().unwrap())
                .await
                .status
                .as_u16(),
            200
        );
    }
}
