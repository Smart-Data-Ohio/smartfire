# Rails assertion to Rust assertion map

Reference: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. Each row names an original Rails assertion call, its discriminating Rust assertion and the real test that executes it. Shared helper assertions and repeated loop cases are cited explicitly. These tables retain compiler checks as compiler checks; they do not claim matching exception classes between Ruby and the Rust type system.

## WS14g-010

Rails declaration: `test/controllers/sudos_controller_test.rb:61` — totp is registered at boot but unsupported without enrollment

Executed test: `campfire::bin/campfire app::cutover_d_tests::cutover_d_totp_is_registered_unsupported_and_unavailable_without_enrollment`.

Pinned Rails fixtures and starting state; real production model API or HTTP router. TOTP rejects before any successful confirmation; clock tests grant through HTTP then advance exactly 14 or 16 minutes. Escaping JSON parser error is injected at the Google client operation boundary used by production MeetingRefresh, distinct from malformed HTTP classification.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:62](../../test/controllers/sudos_controller_test.rb#L62)<br>`assert_includes SudoMode.extra_verifiers, :totp` | [rust/crates/campfire/src/app/cutover_d_tests.rs:19](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L19)<br>`assert!( a.booted .app .sudo .extra_verifiers() .iter() .any(\|name\| name == "totp") )` |
| [test/controllers/sudos_controller_test.rb:63](../../test/controllers/sudos_controller_test.rb#L63)<br>`assert_equal :unsupported, SudoMode.verify_with(:totp, users(:david), { totp_code: "123456" })` | [rust/crates/campfire/src/app/cutover_d_tests.rs:38](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L38)<br>`assert_eq!(verification, None)` |
| [test/controllers/sudos_controller_test.rb:64](../../test/controllers/sudos_controller_test.rb#L64)<br>`assert_not SudoMode.verifier_available?(:totp, users(:david))` | [rust/crates/campfire/src/app/cutover_d_tests.rs:45](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L45)<br>`assert!(!available)` |

## WS14g-016

Rails declaration: `test/controllers/sudos_controller_test.rb:118` — confirming with a wrong TOTP code fails and stays gated

Executed test: `campfire::bin/campfire app::cutover_d_tests::cutover_d_wrong_totp_audits_failure_and_initial_session_stays_gated`.

Pinned Rails fixtures and starting state; real production model API or HTTP router. TOTP rejects before any successful confirmation; clock tests grant through HTTP then advance exactly 14 or 16 minutes. Escaping JSON parser error is injected at the Google client operation boundary used by production MeetingRefresh, distinct from malformed HTTP classification.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:122](../../test/controllers/sudos_controller_test.rb#L122)<br>`assert_difference -> { AuditLog.where(action: "sudo.confirm.failure").count }, +1 do` | [rust/crates/campfire/src/app/cutover_d_tests.rs:91](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L91)<br>`assert_eq!(after, before + 1)` |
| [test/controllers/sudos_controller_test.rb:126](../../test/controllers/sudos_controller_test.rb#L126)<br>`assert_response :unauthorized` | [rust/crates/campfire/src/app/cutover_d_tests.rs:92](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L92)<br>`assert_eq!(rejected.status, StatusCode::UNAUTHORIZED)` |
| [test/controllers/sudos_controller_test.rb:127](../../test/controllers/sudos_controller_test.rb#L127)<br>`assert_equal "totp", AuditLog.where(action: "sudo.confirm.failure").order(:id).last.details["verifier"]` | [rust/crates/campfire/src/app/cutover_d_tests.rs:95](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L95)<br>`assert_eq!(verifier, "totp")` |
| [test/controllers/sudos_controller_test.rb:130](../../test/controllers/sudos_controller_test.rb#L130)<br>`assert_redirected_to new_sudo_url` | [rust/crates/campfire/src/app/cutover_d_tests.rs:97](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L97)<br>`assert_eq!(gated.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/app/cutover_d_tests.rs:98](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L98)<br>`assert_eq!(gated.location(), Some("http://campfire.test/sudo/new"))` |

## WS14g-021

Rails declaration: `test/controllers/sudos_controller_test.rb:196` — a gated GET continues automatically after confirmation

Executed test: `campfire::bin/campfire app::cutover_d_tests::cutover_d_gated_audit_csv_get_continues_and_returns_csv`.

Pinned Rails fixtures and starting state; real production model API or HTTP router. TOTP rejects before any successful confirmation; clock tests grant through HTTP then advance exactly 14 or 16 minutes. Escaping JSON parser error is injected at the Google client operation boundary used by production MeetingRefresh, distinct from malformed HTTP classification.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:200](../../test/controllers/sudos_controller_test.rb#L200)<br>`assert_redirected_to new_sudo_url` | [rust/crates/campfire/src/app/cutover_d_tests.rs:106](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L106)<br>`assert_eq!(gated.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/app/cutover_d_tests.rs:107](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L107)<br>`assert_eq!(gated.location(), Some("http://campfire.test/sudo/new"))` |
| [test/controllers/sudos_controller_test.rb:203](../../test/controllers/sudos_controller_test.rb#L203)<br>`assert_redirected_to account_audit_log_url(format: :csv)` | [rust/crates/campfire/src/app/cutover_d_tests.rs:111](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L111)<br>`assert_eq!(confirmed.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/app/cutover_d_tests.rs:112](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L112)<br>`assert_eq!( confirmed.location(), Some("http://campfire.test/account/audit_log.csv") )` |
| [test/controllers/sudos_controller_test.rb:206](../../test/controllers/sudos_controller_test.rb#L206)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_d_tests.rs:117](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L117)<br>`assert_eq!(csv.status, StatusCode::OK)` |
| [test/controllers/sudos_controller_test.rb:207](../../test/controllers/sudos_controller_test.rb#L207)<br>`assert_equal "text/csv", response.media_type` | [rust/crates/campfire/src/app/cutover_d_tests.rs:118](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L118)<br>`assert_eq!( csv.headers .get("content-type") .unwrap() .to_str() .unwrap() .split(';') .next() .unwrap(), "text/csv" )` |

## WS14g-023

Rails declaration: `test/controllers/sudos_controller_test.rb:218` — a fresh confirmation lasts fifteen minutes

Executed test: `campfire::bin/campfire app::cutover_d_tests::cutover_d_fourteen_minute_confirmation_allows_real_join_code_post`.

Pinned Rails fixtures and starting state; real production model API or HTTP router. TOTP rejects before any successful confirmation; clock tests grant through HTTP then advance exactly 14 or 16 minutes. Escaping JSON parser error is injected at the Google client operation boundary used by production MeetingRefresh, distinct from malformed HTTP classification.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:224](../../test/controllers/sudos_controller_test.rb#L224)<br>`assert_redirected_to edit_account_url` | [rust/crates/campfire/src/app/cutover_d_tests.rs:151](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L151)<br>`assert_eq!(response.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/app/cutover_d_tests.rs:152](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L152)<br>`assert_eq!( response.location(), Some("http://campfire.test/account/edit") )` |

## WS14g-024

Rails declaration: `test/controllers/sudos_controller_test.rb:228` — a stale confirmation prompts again

Executed test: `campfire::bin/campfire app::cutover_d_tests::cutover_d_sixteen_minute_confirmation_gates_real_join_code_post`.

Pinned Rails fixtures and starting state; real production model API or HTTP router. TOTP rejects before any successful confirmation; clock tests grant through HTTP then advance exactly 14 or 16 minutes. Escaping JSON parser error is injected at the Google client operation boundary used by production MeetingRefresh, distinct from malformed HTTP classification.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:234](../../test/controllers/sudos_controller_test.rb#L234)<br>`assert_redirected_to new_sudo_url` | [rust/crates/campfire/src/app/cutover_d_tests.rs:160](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L160)<br>`assert_eq!(response.status, StatusCode::FOUND)`<br><br>[rust/crates/campfire/src/app/cutover_d_tests.rs:161](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L161)<br>`assert_eq!(response.location(), Some("http://campfire.test/sudo/new"))` |

## WS14g-108

Rails declaration: `test/models/calendar/meeting_refresh_test.rb:142` — a JSON parse failure escaping the client is recorded instead of raising

Executed test: `campfire::bin/campfire app::cutover_d_tests::cutover_d_escaping_json_parser_keeps_existing_busy_intervals_and_records_error`.

Pinned Rails fixtures and starting state; real production model API or HTTP router. TOTP rejects before any successful confirmation; clock tests grant through HTTP then advance exactly 14 or 16 minutes. Escaping JSON parser error is injected at the Google client operation boundary used by production MeetingRefresh, distinct from malformed HTTP classification.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/calendar/meeting_refresh_test.rb:147](../../test/models/calendar/meeting_refresh_test.rb#L147)<br>`assert_equal :error, Calendar::MeetingRefresh.refresh(@user.id)` | [rust/crates/campfire/src/app/cutover_d_tests.rs:258](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L258)<br>`assert_eq!(result, meeting_refresh::Result::Error)` |
| [test/models/calendar/meeting_refresh_test.rb:150](../../test/models/calendar/meeting_refresh_test.rb#L150)<br>`assert_equal busy, cache.busy_intervals` | [rust/crates/campfire/src/app/cutover_d_tests.rs:265](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L265)<br>`assert_eq!(cache.busy, busy)` |
| [test/models/calendar/meeting_refresh_test.rb:151](../../test/models/calendar/meeting_refresh_test.rb#L151)<br>`assert_equal Calendar::MeetingRefresh::UNREACHABLE_MESSAGE, cache.fetch_error` | [rust/crates/campfire/src/app/cutover_d_tests.rs:266](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L266)<br>`assert_eq!( cache.fetch_error.as_deref(), Some(meeting_refresh::UNREACHABLE) )` |

## WS14g-181

Rails declaration: `test/models/google_account_test.rb:6` — one account per user

Executed test: `campfire::bin/campfire app::cutover_d_tests::cutover_d_duplicate_google_account_is_invalid_before_persistence`.

Pinned Rails fixtures and starting state; real production model API or HTTP router. TOTP rejects before any successful confirmation; clock tests grant through HTTP then advance exactly 14 or 16 minutes. Escaping JSON parser error is injected at the Google client operation boundary used by production MeetingRefresh, distinct from malformed HTTP classification.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/google_account_test.rb:11](../../test/models/google_account_test.rb#L11)<br>`assert_not duplicate.valid?` | [rust/crates/campfire/src/app/cutover_d_tests.rs:181](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L181)<br>`assert!(!errors.is_empty())` |
| [test/models/google_account_test.rb:12](../../test/models/google_account_test.rb#L12)<br>`assert_equal [ "has already been taken" ], duplicate.errors[:user_id]` | [rust/crates/campfire/src/app/cutover_d_tests.rs:182](../../rust/crates/campfire/src/app/cutover_d_tests.rs#L182)<br>`assert_eq!(errors.on("user_id"), ["has already been taken"])` |
