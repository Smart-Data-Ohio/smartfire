//! The navigation URL contract (`crates/spa/compat/urls.json`, cutover step 1): every
//! navigation URL family and retained boundary, asserted over real HTTP against full booted
//! `TestApp` routers. The JSON holds ALL expectation state (statuses, locations, content
//! markers); this runner only substitutes fixtures, serves the apps, and compares. Later
//! default flips and prefix drops must need only JSON expectation edits here.

use std::collections::BTreeMap;
use std::net::SocketAddr;

use campfire_db::models::user::ui_preference::{self, UiPreference};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::controllers::presenters::test_support::{
    BENDER, DAVID, JASON, KEVIN, TestApp, encode, seed_clock,
};

const CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../spa/compat/urls.json"
));

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    version: u32,
    #[allow(dead_code)]
    title: String,
    #[allow(dead_code)]
    notes: String,
    placeholders: BTreeMap<String, String>,
    runtime_placeholders: Vec<String>,
    entries: Vec<Entry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    #[allow(dead_code)]
    family: String,
    #[allow(dead_code)]
    pattern: String,
    #[allow(dead_code)]
    category: String,
    #[allow(dead_code)]
    classic_endpoint: Option<String>,
    owner_today: String,
    after_cutover: AfterCutover,
    #[allow(dead_code)]
    bookmarks: bool,
    #[allow(dead_code)]
    notifications: bool,
    #[allow(dead_code)]
    bookmark_notes: String,
    #[allow(dead_code)]
    notification_notes: String,
    #[serde(default)]
    #[allow(dead_code)]
    known_bug: bool,
    #[allow(dead_code)]
    notes: String,
    cases: Vec<Case>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AfterCutover {
    action: String,
    #[allow(dead_code)]
    target: String,
    #[allow(dead_code)]
    notes: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    #[serde(default = "get")]
    method: String,
    path: String,
    #[serde(default = "default_seed")]
    seed: String,
    auth: String,
    preference: Option<String>,
    spa_enabled: bool,
    #[serde(default)]
    spa_default_next: bool,
    #[serde(default)]
    sudo: bool,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    form: Option<BTreeMap<String, String>>,
    #[serde(default)]
    csrf: bool,
    #[serde(default)]
    json: Option<String>,
    #[serde(default)]
    only_dist: Option<String>,
    #[serde(default)]
    expect: Option<Expect>,
    #[serde(default)]
    expect_stub: Option<Expect>,
    #[serde(default)]
    expect_built: Option<Expect>,
}

fn get() -> String {
    "GET".into()
}

fn default_seed() -> String {
    "default".into()
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    status: u16,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    location_prefix: Option<String>,
    #[serde(default)]
    location_absent: bool,
    #[serde(default)]
    content_type_prefix: Option<String>,
    #[serde(default)]
    markers: Vec<String>,
    #[serde(default)]
    absent: Vec<String>,
    #[serde(default)]
    body_empty: bool,
    #[serde(default)]
    body_nonempty: bool,
}

/// Every entry has runnable cases, and the contract only says things the runner understands.
fn validate(contract: &Contract) -> Vec<String> {
    let mut errors = Vec::new();
    if contract.version != 1 {
        errors.push(format!("version is {}, want 1", contract.version));
    }
    if contract.entries.is_empty() {
        errors.push("no entries".into());
    }
    let mut entry_ids = std::collections::HashSet::new();
    let mut case_ids = std::collections::HashSet::new();
    for entry in &contract.entries {
        if !entry_ids.insert(entry.id.clone()) {
            errors.push(format!("duplicate entry {}", entry.id));
        }
        if !["classic", "spa", "both", "server", "static", "api"]
            .contains(&entry.owner_today.as_str())
        {
            errors.push(format!(
                "{}.owner_today = {:?}",
                entry.id, entry.owner_today
            ));
        }
        if !["spa", "alias", "retain"].contains(&entry.after_cutover.action.as_str()) {
            errors.push(format!(
                "{}.after_cutover.action = {:?}",
                entry.id, entry.after_cutover.action
            ));
        }
        if entry.cases.is_empty() {
            errors.push(format!("{} has no runnable cases", entry.id));
        }
        for case in &entry.cases {
            let at = format!("{}.{}", entry.id, case.id);
            if !case_ids.insert(format!("{}.{}", entry.id, case.id)) {
                errors.push(format!("duplicate case {at}"));
            }
            if !["GET", "HEAD", "POST", "PATCH", "PUT", "DELETE"].contains(&case.method.as_str()) {
                errors.push(format!("{at}.method = {:?}", case.method));
            }
            if !["default", "first_run"].contains(&case.seed.as_str()) {
                errors.push(format!("{at}.seed = {:?}", case.seed));
            }
            if !["david", "jason", "kevin", "bender", "anonymous"].contains(&case.auth.as_str()) {
                errors.push(format!("{at}.auth = {:?}", case.auth));
            }
            // Preference must be explicit, except where no choice can exist: anonymous has no
            // row, and jason/bender stay pristine so default-flip cases see a true None.
            let needs_none = matches!(case.auth.as_str(), "anonymous" | "jason" | "bender");
            match (&case.preference, needs_none) {
                (None, false) => errors.push(format!("{at}: preference must be explicit")),
                (Some(_), true) => errors.push(format!("{at}: preference must be null")),
                (Some(p), false) if !["next", "classic"].contains(&p.as_str()) => {
                    errors.push(format!("{at}.preference = {p:?}"));
                }
                _ => {}
            }
            if case.sudo && case.auth == "anonymous" {
                errors.push(format!("{at}: sudo needs a signed-in user"));
            }
            if let Some(dist) = &case.only_dist
                && !["stub", "built"].contains(&dist.as_str())
            {
                errors.push(format!("{at}.only_dist = {dist:?}"));
            }
            match (&case.expect, &case.expect_stub, &case.expect_built) {
                (Some(_), None, None) | (None, Some(_), Some(_)) => {}
                _ => errors.push(format!(
                    "{at}: want expect or both expect_stub+expect_built"
                )),
            }
            for expect in [&case.expect, &case.expect_stub, &case.expect_built]
                .into_iter()
                .flatten()
            {
                if !(100..=599).contains(&expect.status) {
                    errors.push(format!("{at}.status = {}", expect.status));
                }
                if expect.location.is_some() && expect.location_prefix.is_some() {
                    errors.push(format!("{at}: location and location_prefix are exclusive"));
                }
                if expect.location_absent
                    && (expect.location.is_some() || expect.location_prefix.is_some())
                {
                    errors.push(format!("{at}: location_absent with a location"));
                }
                if expect.body_empty && expect.body_nonempty {
                    errors.push(format!("{at}: body_empty and body_nonempty"));
                }
                if expect.body_empty && !expect.markers.is_empty() {
                    errors.push(format!("{at}: body_empty with markers"));
                }
            }
            for text in [&case.path]
                .into_iter()
                .chain(case.headers.values())
                .chain(
                    case.form
                        .as_ref()
                        .map(|form| form.values().collect::<Vec<_>>())
                        .unwrap_or_default(),
                )
                .chain(case.json.as_ref())
            {
                for name in placeholders_in(text) {
                    if !contract.placeholders.contains_key(&name)
                        && !contract.runtime_placeholders.contains(&name)
                    {
                        errors.push(format!("{at}: unknown placeholder {{{name}}}"));
                    }
                }
            }
        }
    }
    errors
}

fn placeholders_in(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        rest = &rest[start + 1..];
        if let Some(end) = rest.find('}') {
            let name = &rest[..end];
            if !name.is_empty()
                && name
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
            {
                names.push(name.to_string());
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    names
}

#[test]
fn url_contract_entries_all_have_runnable_cases() {
    let contract: Contract = serde_json::from_str(CONTRACT).expect("urls.json parses");
    let errors = validate(&contract);
    assert!(
        errors.is_empty(),
        "{} contract error(s):\n{}",
        errors.len(),
        errors.join("\n")
    );
    let cases: usize = contract.entries.iter().map(|entry| entry.cases.len()).sum();
    assert!(
        cases > 0,
        "the contract must hold executable cases, not just patterns"
    );
}

struct Live {
    addr: SocketAddr,
    jar: BTreeMap<String, String>,
}

struct Reply {
    status: u16,
    headers: BTreeMap<String, Vec<String>>,
    body: Vec<u8>,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name)?.first().map(String::as_str)
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

impl Live {
    async fn request(
        &mut self,
        method: &str,
        path: &str,
        headers: &BTreeMap<String, String>,
        body: &[u8],
    ) -> Reply {
        let mut head =
            format!("{method} {path} HTTP/1.1\r\nHost: campfire.test\r\nConnection: close\r\n");
        if !self.jar.is_empty() {
            head.push_str(&format!(
                "Cookie: {}\r\n",
                self.jar
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        let mut has_content_type = false;
        let mut has_accept = false;
        for (name, value) in headers {
            if name.eq_ignore_ascii_case("content-type") {
                has_content_type = true;
            }
            if name.eq_ignore_ascii_case("accept") {
                has_accept = true;
            }
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        // The app's own tests navigate with this Accept; JSON/stream cases set their own.
        if !has_accept && (method == "GET" || method == "HEAD") {
            head.push_str("Accept: text/html,application/xhtml+xml\r\n");
        }
        if !body.is_empty() {
            if !has_content_type {
                head.push_str("Content-Type: application/x-www-form-urlencoded\r\n");
            }
            head.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        head.push_str("\r\n");
        let mut stream = tokio::net::TcpStream::connect(self.addr).await.unwrap();
        stream.write_all(head.as_bytes()).await.unwrap();
        stream.write_all(body).await.unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await.unwrap();
        let split = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("an HTTP head");
        let text = std::str::from_utf8(&raw[..split]).expect("an ASCII head");
        let mut lines = text.split("\r\n");
        let status: u16 = lines
            .next()
            .unwrap()
            .split(' ')
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let mut response_headers: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for line in lines {
            let (name, value) = line.split_once(':').unwrap();
            response_headers
                .entry(name.trim().to_ascii_lowercase())
                .or_default()
                .push(value.trim().to_string());
        }
        let mut body = raw[split + 4..].to_vec();
        if response_headers
            .get("transfer-encoding")
            .is_some_and(|encodings| {
                encodings
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case("chunked"))
            })
        {
            body = dechunk(&body);
        }
        if let Some(cookies) = response_headers.get("set-cookie") {
            for cookie in cookies {
                if let Some((name, value)) = cookie.split(';').next().unwrap_or("").split_once('=')
                {
                    if value.is_empty() {
                        self.jar.remove(name);
                    } else {
                        self.jar.insert(name.to_string(), value.to_string());
                    }
                }
            }
        }
        Reply {
            status,
            headers: response_headers,
            body,
        }
    }
}

fn dechunk(body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(end) = rest.windows(2).position(|w| w == b"\r\n") {
        let size =
            usize::from_str_radix(std::str::from_utf8(&rest[..end]).unwrap_or("").trim(), 16)
                .unwrap_or(0);
        rest = &rest[end + 2..];
        if size == 0 {
            break;
        }
        let take = size.min(rest.len());
        out.extend_from_slice(&rest[..take]);
        rest = &rest[take..];
        rest = rest.strip_prefix(b"\r\n").unwrap_or(rest);
    }
    out
}

fn substitute(text: &str, values: &BTreeMap<String, String>) -> String {
    let mut out = text.to_string();
    for (name, value) in values {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// The `Location` without its origin: `/app/r/1?x=2`, or the value itself when relative.
fn location_path(location: &str) -> &str {
    match location.find("://").map(|i| &location[i + 3..]) {
        Some(after) => after.find('/').map(|i| &after[i..]).unwrap_or("/"),
        None => location,
    }
}

fn user_id(auth: &str) -> i64 {
    match auth {
        "david" => DAVID,
        "jason" => JASON,
        "kevin" => KEVIN,
        "bender" => BENDER,
        auth => panic!("unknown auth {auth:?}"),
    }
}

fn seed_cookies(header: &str) -> BTreeMap<String, String> {
    let mut jar = BTreeMap::new();
    for pair in header.split(';') {
        if let Some((name, value)) = pair.trim().split_once('=') {
            jar.insert(name.to_string(), value.to_string());
        }
    }
    jar
}

fn csrf_token(html: &str) -> Option<String> {
    html.split("<meta name=\"csrf-token\" content=\"")
        .nth(1)?
        .split('"')
        .next()
        .map(str::to_string)
}

struct Group {
    app: TestApp,
    addr: SocketAddr,
    values: BTreeMap<String, String>,
    _server: tokio::task::JoinHandle<()>,
}

async fn slack_fixtures(app: &TestApp) -> (i64, i64) {
    app.db()
        .write(|tx| {
            // Keep provider credentials absent: only local import pages need records.
            tx.conn().execute(
                "INSERT INTO slack_workspaces(team_id,team_name,created_at,updated_at) VALUES('TCONTRACT','URL contract',?,?)",
                rusqlite::params![tx.now(), tx.now()],
            )?;
            let workspace = tx.conn().last_insert_rowid();
            let stats = serde_json::json!({
                "conversations": [{"id": "C1", "name": "general", "type": "public_channel", "messages": 1}],
                "users": {"total": 1, "matched": 1, "placeholders": 0},
                "counts": {"messages": 1}
            });
            let mut ids = Vec::new();
            for (kind, mode) in [("workspace", "dry_run"), ("personal", "import")] {
                tx.conn().execute(
                    "INSERT INTO slack_imports(slack_workspace_id,user_id,kind,mode,status,options,stats,started_at,finished_at,created_at,updated_at) VALUES(?,?,?,?,'completed','{}',?,?,?,?,?)",
                    rusqlite::params![workspace, DAVID, kind, mode, stats.to_string(), tx.now(), tx.now(), tx.now(), tx.now()],
                )?;
                ids.push(tx.conn().last_insert_rowid());
            }
            Ok((ids[0], ids[1]))
        })
        .await
        .unwrap()
}

impl Group {
    async fn boot(
        contract: &Contract,
        seed: &str,
        spa_enabled: bool,
        spa_default_next: bool,
    ) -> Option<Group> {
        let mut env = Vec::new();
        if spa_enabled {
            env.push(("SPA_ENABLED", "1"));
        }
        if spa_default_next {
            env.push(("SPA_DEFAULT", "next"));
        }
        let app = TestApp::boot_seed_with_env(seed, seed_clock(), &env)
            .await?
            .without_job_runner()
            .await;
        let join_code: String = app
            .db()
            .read(|conn| Ok(campfire_db::Account::first(conn)?.map(|account| account.join_code)))
            .await
            .unwrap()
            .unwrap_or_default();
        let mut values = contract.placeholders.clone();
        values.insert("JOIN_CODE".into(), join_code);
        if seed == "default" {
            let (run, personal) = slack_fixtures(&app).await;
            values.insert("SLACK_RUN".into(), run.to_string());
            values.insert("SLACK_PERSONAL".into(), personal.to_string());
        }
        values.insert(
            "SPA_ASSET".into(),
            campfire_spa::files()
                .iter()
                .find(|file| file.path.starts_with("assets/") && file.path.ends_with(".js"))
                .map_or("assets/contract-no-dist.js", |file| file.path)
                .to_owned(),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let router = app.booted.router.clone();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        Some(Group {
            app,
            addr,
            values,
            _server: server,
        })
    }
}

#[tokio::test]
async fn url_contract_matches_over_real_http() {
    let contract: Contract = serde_json::from_str(CONTRACT).expect("urls.json parses");
    let errors = validate(&contract);
    assert!(errors.is_empty(), "contract errors:\n{}", errors.join("\n"));

    let built = campfire_spa::built();
    let mut groups: BTreeMap<(String, bool, bool), Group> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();
    let mut ran = 0usize;
    let mut skipped = 0usize;
    let mut ran_per_entry: BTreeMap<&str, usize> = BTreeMap::new();

    for entry in &contract.entries {
        for case in &entry.cases {
            let expect = match (&case.expect, &case.expect_stub, &case.expect_built) {
                (Some(expect), _, _) => Some(expect),
                (None, stub, built_expect) if built => built_expect.as_ref().or(stub.as_ref()),
                (None, stub, _) => stub.as_ref(),
            };
            let (Some(expect), only) = (expect, case.only_dist.as_deref()) else {
                failures.push(format!("{}.{}: no expectation", entry.id, case.id));
                continue;
            };
            if only.is_some_and(|dist| dist != if built { "built" } else { "stub" }) {
                skipped += 1;
                continue;
            }
            let key = (case.seed.clone(), case.spa_enabled, case.spa_default_next);
            if !groups.contains_key(&key) {
                match Group::boot(&contract, &key.0, key.1, key.2).await {
                    Some(group) => {
                        groups.insert(key.clone(), group);
                    }
                    None => {
                        eprintln!(
                            "skipping url contract: parity/.seed/{} isn't restored",
                            key.0
                        );
                        return;
                    }
                }
            }
            let group = groups.get(&key).unwrap();
            if let Some(preference) = &case.preference {
                let user = user_id(&case.auth);
                let preference = match preference.as_str() {
                    "next" => UiPreference::Next,
                    "classic" => UiPreference::Classic,
                    preference => panic!("unknown preference {preference:?}"),
                };
                group
                    .app
                    .db()
                    .write(move |tx| ui_preference::store(tx, user, preference))
                    .await
                    .unwrap();
            }
            let mut client = Live {
                addr: group.addr,
                jar: BTreeMap::new(),
            };
            if case.auth != "anonymous" {
                let mut browser = group.app.sign_in(user_id(&case.auth)).await;
                if case.sudo {
                    browser.grant_sudo().await;
                }
                client.jar = seed_cookies(&browser.cookie_header());
            }
            // Writes carry the session's token, as the app's own pages send it.
            let mut headers: BTreeMap<String, String> = case
                .headers
                .iter()
                .map(|(name, value)| (name.clone(), substitute(value, &group.values)))
                .collect();
            let mut body = Vec::new();
            if let Some(form) = &case.form {
                body = form
                    .iter()
                    .map(|(name, value)| {
                        format!(
                            "{}={}",
                            encode(name),
                            encode(&substitute(value, &group.values))
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("&")
                    .into_bytes();
            } else if let Some(json) = &case.json {
                body = substitute(json, &group.values).into_bytes();
            }
            if case.form.is_some() || case.csrf || !body.is_empty() {
                let scrape = if case.auth == "anonymous" {
                    "/session/new"
                } else {
                    "/users/me/profile?classic=1"
                };
                let page = client.request("GET", scrape, &BTreeMap::new(), &[]).await;
                match csrf_token(&page.text()) {
                    Some(token) => {
                        headers.insert(campfire_kit::csrf::HEADER.to_string(), token);
                    }
                    None => {
                        failures.push(format!(
                            "{}.{}: no CSRF token on {scrape} ({} {})",
                            entry.id,
                            case.id,
                            page.status,
                            snippet(&page.text())
                        ));
                        continue;
                    }
                }
            }
            let path = substitute(&case.path, &group.values);
            let reply = client.request(&case.method, &path, &headers, &body).await;
            ran += 1;
            *ran_per_entry.entry(entry.id.as_str()).or_default() += 1;
            let mut mismatches = Vec::new();
            if reply.status != expect.status {
                mismatches.push(format!("status {} != {}", reply.status, expect.status));
            }
            match (
                &expect.location,
                &expect.location_prefix,
                expect.location_absent,
            ) {
                (Some(want), _, _) => {
                    let want = substitute(want, &group.values);
                    match reply.header("location").map(location_path) {
                        Some(got) if got == want => {}
                        got => mismatches.push(format!("location {got:?} != {want:?}")),
                    }
                }
                (_, Some(prefix), _) => {
                    let prefix = substitute(prefix, &group.values);
                    match reply.header("location").map(location_path) {
                        Some(got) if got.starts_with(prefix.as_str()) => {}
                        got => {
                            mismatches.push(format!("location {got:?} has no prefix {prefix:?}"))
                        }
                    }
                }
                (_, _, true) => {
                    if let Some(got) = reply.header("location") {
                        mismatches.push(format!("location {got:?} should be absent"));
                    }
                }
                _ => {}
            }
            if let Some(prefix) = &expect.content_type_prefix {
                match reply.header("content-type") {
                    Some(got) if got.starts_with(prefix.as_str()) => {}
                    got => {
                        mismatches.push(format!("content-type {got:?} has no prefix {prefix:?}"))
                    }
                }
            }
            let text = reply.text();
            for marker in &expect.markers {
                let marker = substitute(marker, &group.values);
                if !text.contains(marker.as_str()) {
                    mismatches.push(format!("missing {marker:?}"));
                }
            }
            for marker in &expect.absent {
                let marker = substitute(marker, &group.values);
                if text.contains(marker.as_str()) {
                    mismatches.push(format!("should not contain {marker:?}"));
                }
            }
            if expect.body_empty && !reply.body.is_empty() {
                mismatches.push(format!(
                    "body should be empty, got {} bytes",
                    reply.body.len()
                ));
            }
            if expect.body_nonempty && reply.body.is_empty() {
                mismatches.push("body should not be empty".to_string());
            }
            if !mismatches.is_empty() {
                failures.push(format!(
                    "{}.{} ({} {}, auth={} pref={:?} seed={} spa={}/{})\n  {}\n  actual: status={} location={:?} content-type={:?} body={}",
                    entry.id,
                    case.id,
                    case.method,
                    path,
                    case.auth,
                    case.preference,
                    case.seed,
                    case.spa_enabled,
                    case.spa_default_next,
                    mismatches.join("\n  "),
                    reply.status,
                    reply.header("location"),
                    reply.header("content-type"),
                    snippet(&text),
                ));
            }
        }
    }

    for entry in &contract.entries {
        if ran_per_entry.get(entry.id.as_str()).copied().unwrap_or(0) == 0 {
            failures.push(format!("{}: no case ran in this dist mode", entry.id));
        }
    }
    println!(
        "URL_CONTRACT_RECEIPT families={} cases={ran} skipped={skipped} mismatches={} dist={}",
        contract.entries.len(),
        failures.len(),
        if built { "built" } else { "stub" }
    );
    assert!(
        failures.is_empty(),
        "{ran} ran, {skipped} skipped, {} mismatch(es):\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

fn snippet(text: &str) -> String {
    const LIMIT: usize = 400;
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.len() > LIMIT {
        format!("{}…", &flat[..LIMIT])
    } else {
        flat
    }
}
