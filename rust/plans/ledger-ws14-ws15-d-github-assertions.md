# Rails assertion to Rust assertion map

Reference: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. Each row names an original Rails assertion call, its discriminating Rust assertion and the real test that executes it. Shared helper assertions and repeated loop cases are cited explicitly. These tables retain compiler checks as compiler checks; they do not claim matching exception classes between Ruby and the Rust type system.

## WS15g-002

Rails declaration: `test/controllers/agents/github_action_delivery_test.rb:25` — an approved action records a completed ledger row with the GitHub url

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_job_complete_failure_poll_ack_readability_and_ledger`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/agents/github_action_delivery_test.rb:30](../../test/controllers/agents/github_action_delivery_test.rb#L30)<br>`assert_difference -> { @agent.agent_events.where(event_type: "github_action_completed").count }, 1 do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1267](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1267)<br>`assert_eq!(after, before + 1)` |
| [test/controllers/agents/github_action_delivery_test.rb:36](../../test/controllers/agents/github_action_delivery_test.rb#L36)<br>`assert_equal "delivered", event.outcome` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1268](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1268)<br>`assert_eq!(event.2, "delivered")` |
| [test/controllers/agents/github_action_delivery_test.rb:37](../../test/controllers/agents/github_action_delivery_test.rb#L37)<br>`assert_equal @room, event.room` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1269](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1269)<br>`assert_eq!(event.1, id("watercooler"))` |
| [test/controllers/agents/github_action_delivery_test.rb:38](../../test/controllers/agents/github_action_delivery_test.rb#L38)<br>`assert_nil event.message_id` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1270](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1270)<br>`assert_eq!(event.3, None)` |
| [test/controllers/agents/github_action_delivery_test.rb:39](../../test/controllers/agents/github_action_delivery_test.rb#L39)<br>`assert_equal(` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1282](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1282)<br>`assert_eq!(event.4, expected)` |
| [test/controllers/agents/github_action_delivery_test.rb:211](../../test/controllers/agents/github_action_delivery_test.rb#L211)<br>`assert action.valid?, action.errors.full_messages.to_sentence` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1124](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1124)<br>`assert!(a.errors().is_empty())` |

## WS15g-003

Rails declaration: `test/controllers/agents/github_action_delivery_test.rb:50` — a failed action records the reason without a url

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_job_complete_failure_poll_ack_readability_and_ledger`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/agents/github_action_delivery_test.rb:58](../../test/controllers/agents/github_action_delivery_test.rb#L58)<br>`assert_equal(` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1282](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1282)<br>`assert_eq!(event.4, expected)` |
| [test/controllers/agents/github_action_delivery_test.rb:211](../../test/controllers/agents/github_action_delivery_test.rb#L211)<br>`assert action.valid?, action.errors.full_messages.to_sentence` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1124](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1124)<br>`assert!(a.errors().is_empty())` |

## WS15g-004

Rails declaration: `test/controllers/agents/github_action_delivery_test.rb:69` — completion appears in event polling with the github_action payload

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_job_complete_failure_poll_ack_readability_and_ledger`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/agents/github_action_delivery_test.rb:78](../../test/controllers/agents/github_action_delivery_test.rb#L78)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1290](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1290)<br>`assert_eq!(status, 200)` |
| [test/controllers/agents/github_action_delivery_test.rb:80](../../test/controllers/agents/github_action_delivery_test.rb#L80)<br>`assert row, "expected a github_action_completed row in #{response.parsed_body["events"].inspect}"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1296](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1296)<br>`assert!(row.is_some(), "completion absent from {poll}")` |
| [test/controllers/agents/github_action_delivery_test.rb:81](../../test/controllers/agents/github_action_delivery_test.rb#L81)<br>`assert_equal "delivered", row["outcome"]` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1299](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1299)<br>`assert_eq!(row["outcome"], "delivered")` |
| [test/controllers/agents/github_action_delivery_test.rb:82](../../test/controllers/agents/github_action_delivery_test.rb#L82)<br>`assert_nil row["message"]` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1300](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1300)<br>`assert!(row["message"].is_null())` |
| [test/controllers/agents/github_action_delivery_test.rb:83](../../test/controllers/agents/github_action_delivery_test.rb#L83)<br>`assert_equal(` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1301](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1301)<br>`assert_eq!(row["github_action"], expected)` |
| [test/controllers/agents/github_action_delivery_test.rb:92](../../test/controllers/agents/github_action_delivery_test.rb#L92)<br>`assert_equal @room.id, row.dig("room", "id")` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1302](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1302)<br>`assert_eq!(row["room"]["id"], id("watercooler"))` |
| [test/controllers/agents/github_action_delivery_test.rb:211](../../test/controllers/agents/github_action_delivery_test.rb#L211)<br>`assert action.valid?, action.errors.full_messages.to_sentence` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1124](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1124)<br>`assert!(a.errors().is_empty())` |

## WS15g-005

Rails declaration: `test/controllers/agents/github_action_delivery_test.rb:95` — failed completions poll with the message and no url

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_job_complete_failure_poll_ack_readability_and_ledger`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/agents/github_action_delivery_test.rb:106](../../test/controllers/agents/github_action_delivery_test.rb#L106)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1290](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1290)<br>`assert_eq!(status, 200)` |
| [test/controllers/agents/github_action_delivery_test.rb:108](../../test/controllers/agents/github_action_delivery_test.rb#L108)<br>`assert row, "expected a github_action_completed row in #{response.parsed_body["events"].inspect}"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1296](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1296)<br>`assert!(row.is_some(), "completion absent from {poll}")` |
| [test/controllers/agents/github_action_delivery_test.rb:109](../../test/controllers/agents/github_action_delivery_test.rb#L109)<br>`assert_equal "failed", row.dig("github_action", "status")` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1380](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1380)<br>`assert_eq!(row["github_action"]["status"], "failed")` |
| [test/controllers/agents/github_action_delivery_test.rb:110](../../test/controllers/agents/github_action_delivery_test.rb#L110)<br>`assert_equal "Agent is no longer a member of the room", row.dig("github_action", "message")` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1381](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1381)<br>`assert_eq!( row["github_action"]["message"], "Agent is no longer a member of the room" )` |
| [test/controllers/agents/github_action_delivery_test.rb:111](../../test/controllers/agents/github_action_delivery_test.rb#L111)<br>`assert_not row["github_action"].key?("url")` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1385](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1385)<br>`assert!( !row["github_action"] .as_object() .unwrap() .contains_key("url") )` |
| [test/controllers/agents/github_action_delivery_test.rb:211](../../test/controllers/agents/github_action_delivery_test.rb#L211)<br>`assert action.valid?, action.errors.full_messages.to_sentence` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1124](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1124)<br>`assert!(a.errors().is_empty())` |

## WS15g-006

Rails declaration: `test/controllers/agents/github_action_delivery_test.rb:114` — ack works on completion rows

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_job_complete_failure_poll_ack_readability_and_ledger`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/agents/github_action_delivery_test.rb:124](../../test/controllers/agents/github_action_delivery_test.rb#L124)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1376](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1376)<br>`assert_eq!(status, 200)` |
| [test/controllers/agents/github_action_delivery_test.rb:125](../../test/controllers/agents/github_action_delivery_test.rb#L125)<br>`assert_equal "acknowledged", response.parsed_body["outcome"]` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1377](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1377)<br>`assert_eq!(body["outcome"], "acknowledged")` |
| [test/controllers/agents/github_action_delivery_test.rb:126](../../test/controllers/agents/github_action_delivery_test.rb#L126)<br>`assert_equal "acknowledged", event.reload.outcome` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1378](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1378)<br>`assert_eq!(completion(&f, approval).await.2, "acknowledged")` |
| [test/controllers/agents/github_action_delivery_test.rb:211](../../test/controllers/agents/github_action_delivery_test.rb#L211)<br>`assert action.valid?, action.errors.full_messages.to_sentence` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1124](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1124)<br>`assert!(a.errors().is_empty())` |

## WS15g-007

Rails declaration: `test/controllers/agents/github_action_delivery_test.rb:129` — completion posts the webhook with agent and github_action keys

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_completion_webhook_has_exact_agent_and_action`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/agents/github_action_delivery_test.rb:138](../../test/controllers/agents/github_action_delivery_test.rb#L138)<br>`assert_requested stub, times: 2 # the approval decision plus the completion` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1792](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1792)<br>`assert_eq!(requests.len(), 2)` |
| [test/controllers/agents/github_action_delivery_test.rb:140](../../test/controllers/agents/github_action_delivery_test.rb#L140)<br>`assert_requested :post, webhooks(:bender).url, body: hash_including(` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1800](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1800)<br>`assert_eq!(matched.len(), 1)`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1802](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1802)<br>`assert_eq!(matched[0].method, "POST")`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1803](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1803)<br>`assert_eq!(matched[0].target, "/bender")`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1804](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1804)<br>`assert_eq!(body["agent"]["id"], agent)`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1805](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1805)<br>`assert_eq!(body["agent"]["name"], "Bender Bot")`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1806](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1806)<br>`assert_eq!(body["agent"]["delivery_id"], completion.0)`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1807](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1807)<br>`assert_eq!( body["github_action"], json!({"approval_id":approval,"action":"github.comment","status":"completed","url":"https://github.com/rails/rails/pull/12#issuecomment-1"}) )` |
| [test/controllers/agents/github_action_delivery_test.rb:211](../../test/controllers/agents/github_action_delivery_test.rb#L211)<br>`assert action.valid?, action.errors.full_messages.to_sentence` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1124](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1124)<br>`assert!(a.errors().is_empty())` |

## WS15g-008

Rails declaration: `test/controllers/agents/github_action_delivery_test.rb:151` — completion rows are readable by their own agent only

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_job_complete_failure_poll_ack_readability_and_ledger`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/agents/github_action_delivery_test.rb:159](../../test/controllers/agents/github_action_delivery_test.rb#L159)<br>`assert_includes @agent.agent_events.readable_by(@agent).to_a, event` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1307](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1307)<br>`assert!( campfire_db::models::agent_event_access::readable_page(c, agent, 0, None)? .iter() .any(\|e\| e.id == event_id) )` |
| [test/controllers/agents/github_action_delivery_test.rb:165](../../test/controllers/agents/github_action_delivery_test.rb#L165)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1352](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1352)<br>`assert_eq!(status, 200)` |
| [test/controllers/agents/github_action_delivery_test.rb:166](../../test/controllers/agents/github_action_delivery_test.rb#L166)<br>`assert_empty response.parsed_body["events"].select { \|entry\| entry["event_type"] == "github_action_completed" }` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1353](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1353)<br>`assert!( poll["events"] .as_array() .unwrap() .iter() .all(\|r\| r["event_type"] != "github_action_completed") )` |
| [test/controllers/agents/github_action_delivery_test.rb:169](../../test/controllers/agents/github_action_delivery_test.rb#L169)<br>`assert_response :not_found` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1367](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1367)<br>`assert_eq!(status, 404)` |
| [test/controllers/agents/github_action_delivery_test.rb:170](../../test/controllers/agents/github_action_delivery_test.rb#L170)<br>`assert_equal "delivered", event.reload.outcome` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1368](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1368)<br>`assert_eq!(completion(&f, approval).await.2, "delivered")` |
| [test/controllers/agents/github_action_delivery_test.rb:211](../../test/controllers/agents/github_action_delivery_test.rb#L211)<br>`assert action.valid?, action.errors.full_messages.to_sentence` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1124](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1124)<br>`assert!(a.errors().is_empty())` |

## WS15g-009

Rails declaration: `test/controllers/agents/github_action_delivery_test.rb:173` — the ledger page lists the completion with its status

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_job_complete_failure_poll_ack_readability_and_ledger`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/agents/github_action_delivery_test.rb:183](../../test/controllers/agents/github_action_delivery_test.rb#L183)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1400](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1400)<br>`assert_eq!(status, 200)` |
| [test/controllers/agents/github_action_delivery_test.rb:184](../../test/controllers/agents/github_action_delivery_test.rb#L184)<br>`assert_match "github_action_completed", response.body` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1402](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1402)<br>`assert!(html.contains("github_action_completed"))` |
| [test/controllers/agents/github_action_delivery_test.rb:185](../../test/controllers/agents/github_action_delivery_test.rb#L185)<br>`assert_match "GitHub github.comment: completed", response.body` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1403](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1403)<br>`assert!(html.contains("GitHub github.comment: completed"))` |
| [test/controllers/agents/github_action_delivery_test.rb:211](../../test/controllers/agents/github_action_delivery_test.rb#L211)<br>`assert action.valid?, action.errors.full_messages.to_sentence` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1124](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1124)<br>`assert!(a.errors().is_empty())` |

## WS15g-010

Rails declaration: `test/controllers/agents/github_action_delivery_test.rb:188` — the ledger page lists failures with their reason

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_job_complete_failure_poll_ack_readability_and_ledger`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/agents/github_action_delivery_test.rb:197](../../test/controllers/agents/github_action_delivery_test.rb#L197)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1400](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1400)<br>`assert_eq!(status, 200)` |
| [test/controllers/agents/github_action_delivery_test.rb:198](../../test/controllers/agents/github_action_delivery_test.rb#L198)<br>`assert_match "GitHub github.comment: failed", response.body` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1405](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1405)<br>`assert!(html.contains("GitHub github.comment: failed"))` |
| [test/controllers/agents/github_action_delivery_test.rb:199](../../test/controllers/agents/github_action_delivery_test.rb#L199)<br>`assert_match "Agent is no longer a member of the room", response.body` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1406](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1406)<br>`assert!(html.contains("Agent is no longer a member of the room"))` |
| [test/controllers/agents/github_action_delivery_test.rb:211](../../test/controllers/agents/github_action_delivery_test.rb#L211)<br>`assert action.valid?, action.errors.full_messages.to_sentence` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1124](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1124)<br>`assert!(a.errors().is_empty())` |

## WS15g-011

Rails declaration: `test/controllers/github/connections_controller_test.rb:71` — the profile cannot edit the login while a verified account is linked

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_profile_verified_login_rejects_edit_until_disconnected`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/connections_controller_test.rb:75](../../test/controllers/github/connections_controller_test.rb#L75)<br>`assert_select "input[name=?][disabled]", "user[github_login]"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:844](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L844)<br>`assert!(input.contains("disabled"))` |
| [test/controllers/github/connections_controller_test.rb:78](../../test/controllers/github/connections_controller_test.rb#L78)<br>`assert_redirected_to user_profile_url` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:853](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L853)<br>`assert_eq!(status, 302)`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:854](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L854)<br>`assert_eq!(headers["location"], "http://example.org/users/me/profile")` |
| [test/controllers/github/connections_controller_test.rb:79](../../test/controllers/github/connections_controller_test.rb#L79)<br>`assert_equal "octocat", users(:david).reload.github_login` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:859](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L859)<br>`assert_eq!( c.query_row( "SELECT github_login FROM users WHERE id=?", [id("david")], \|r\| r.get::<_, Option<String>>(0) )? .as_deref(), Some("octocat") )` |
| [test/controllers/github/connections_controller_test.rb:80](../../test/controllers/github/connections_controller_test.rb#L80)<br>`assert_equal "Dave", users(:david).name` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:868](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L868)<br>`assert_eq!(u.name, "Dave")` |
| [test/controllers/github/connections_controller_test.rb:82](../../test/controllers/github/connections_controller_test.rb#L82)<br>`assert_not users(:david).update(github_login: "someone-else")` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:884](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L884)<br>`assert!(rejected.is_err())` |
| [test/controllers/github/connections_controller_test.rb:83](../../test/controllers/github/connections_controller_test.rb#L83)<br>`assert_includes users(:david).errors[:github_login], "is set by your linked GitHub account"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:888](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L888)<br>`assert!( e.on("github_login") .contains(&"is set by your linked GitHub account") )` |

## WS15g-012

Rails declaration: `test/controllers/github/connections_controller_test.rb:86` — the profile edits the login again once the link is disconnected

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_profile_verified_login_rejects_edit_until_disconnected`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/connections_controller_test.rb:92](../../test/controllers/github/connections_controller_test.rb#L92)<br>`assert_redirected_to user_profile_url` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:905](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L905)<br>`assert_eq!(status, 302)`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:906](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L906)<br>`assert_eq!(headers["location"], "http://example.org/users/me/profile")` |
| [test/controllers/github/connections_controller_test.rb:93](../../test/controllers/github/connections_controller_test.rb#L93)<br>`assert_equal "david-gh", users(:david).reload.github_login` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:910](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L910)<br>`assert_eq!( c.query_row( "SELECT github_login FROM users WHERE id=?", [id("david")], \|r\| r.get::<_, Option<String>>(0) )? .as_deref(), Some("david-gh") )` |

## WS15g-013

Rails declaration: `test/controllers/github/connections_controller_test.rb:144` — linking never logs the pasted token

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_request_logs_filter_link_comment_and_review_credentials`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/connections_controller_test.rb:152](../../test/controllers/github/connections_controller_test.rb#L152)<br>`assert_includes log.string, "github/connection"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1589](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1589)<br>`assert!( logs.text().contains(&path), "missing request log: {}", logs.text() )` |
| [test/controllers/github/connections_controller_test.rb:153](../../test/controllers/github/connections_controller_test.rb#L153)<br>`assert_not_includes log.string, "github_pat_secret_xyz"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1594](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1594)<br>`assert!(!logs.text().contains(token))` |

## WS15g-014

Rails declaration: `test/controllers/github/connections_controller_test.rb:156` — the token parameter is filtered from request logs

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_request_logs_filter_link_comment_and_review_credentials`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/connections_controller_test.rb:159](../../test/controllers/github/connections_controller_test.rb#L159)<br>`assert_equal "[FILTERED]", filter.filter(access_token: "github_pat_secret_xyz")[:access_token]` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1596](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1596)<br>`assert_eq!( crate::security::parameter_filter() .filter(&json!({"access_token":"github_pat_secret_xyz"}))["access_token"], "[FILTERED]" )` |

## WS15g-015

Rails declaration: `test/controllers/github/pull_request_comments_controller_test.rb:156` — posting never logs the member's token

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_request_logs_filter_link_comment_and_review_credentials`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/pull_request_comments_controller_test.rb:167](../../test/controllers/github/pull_request_comments_controller_test.rb#L167)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1587](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1587)<br>`assert_eq!(status, 200)` |
| [test/controllers/github/pull_request_comments_controller_test.rb:168](../../test/controllers/github/pull_request_comments_controller_test.rb#L168)<br>`assert_includes log.string, "pull_request_comments"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1589](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1589)<br>`assert!( logs.text().contains(&path), "missing request log: {}", logs.text() )` |
| [test/controllers/github/pull_request_comments_controller_test.rb:169](../../test/controllers/github/pull_request_comments_controller_test.rb#L169)<br>`assert_not_includes log.string, "user-token-secret-xyz"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1594](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1594)<br>`assert!(!logs.text().contains(token))` |

## WS15g-016

Rails declaration: `test/controllers/github/pull_request_review_requests_controller_test.rb:210` — requesting never logs the member's token

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_request_logs_filter_link_comment_and_review_credentials`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/pull_request_review_requests_controller_test.rb:221](../../test/controllers/github/pull_request_review_requests_controller_test.rb#L221)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1587](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1587)<br>`assert_eq!(status, 200)` |
| [test/controllers/github/pull_request_review_requests_controller_test.rb:222](../../test/controllers/github/pull_request_review_requests_controller_test.rb#L222)<br>`assert_includes log.string, "pull_request_review_requests"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1589](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1589)<br>`assert!( logs.text().contains(&path), "missing request log: {}", logs.text() )` |
| [test/controllers/github/pull_request_review_requests_controller_test.rb:223](../../test/controllers/github/pull_request_review_requests_controller_test.rb#L223)<br>`assert_not_includes log.string, "user-token-secret-xyz"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1594](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1594)<br>`assert!(!logs.text().contains(token))` |

## WS15g-017

Rails declaration: `test/controllers/github/pull_request_threads_controller_test.rb:54` — discuss reuses the winner and drops the loser when the race is lost at the unique index

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_mapping_race_recovers_unique_index_and_validation_losers_over_http`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary. The create! race boundary mirrors Rails simulate_mapping_race: inserts the real winning message/thread/membership/mapping before the one-shot uniqueness error; the real HTTP producer removes the loser. assert_mapping_race_loser_cleaned_up calls the three original helper assertions, each separately cited below.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/pull_request_threads_controller_test.rb:58](../../test/controllers/github/pull_request_threads_controller_test.rb#L58)<br>`assert_difference -> { Github::PullRequestThread.count }, 1 do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1709](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1709)<br>`assert_eq!( c.query_row( "SELECT COUNT(*) FROM github_pull_request_threads", [], \|r\| r.get::<_, i64>(0) )?, 1 )` |
| [test/controllers/github/pull_request_threads_controller_test.rb:63](../../test/controllers/github/pull_request_threads_controller_test.rb#L63)<br>`assert_mapping_race_loser_cleaned_up(thread_ids_before)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1709](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1709)<br>`assert_eq!( c.query_row( "SELECT COUNT(*) FROM github_pull_request_threads", [], \|r\| r.get::<_, i64>(0) )?, 1 )`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1704](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1704)<br>`assert_eq!(status, 303, "index={index}: {body}")`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1733](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1733)<br>`assert_eq!( headers["location"], format!( "http://example.org/rooms/{}/threads/{}", id("designers"), mapping.channel_thread_id ) )`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1722](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1722)<br>`assert_eq!( ids.into_iter() .filter(\|i\| !before.contains(i)) .collect::<Vec<_>>(), vec![mapping.channel_thread_id] )` |
| [test/controllers/github/pull_request_threads_controller_test.rb:143](../../test/controllers/github/pull_request_threads_controller_test.rb#L143)<br>`assert_equal 1, Github::PullRequestThread.count` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1709](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1709)<br>`assert_eq!( c.query_row( "SELECT COUNT(*) FROM github_pull_request_threads", [], \|r\| r.get::<_, i64>(0) )?, 1 )` |
| [test/controllers/github/pull_request_threads_controller_test.rb:145](../../test/controllers/github/pull_request_threads_controller_test.rb#L145)<br>`assert_redirected_to room_thread_path(@room, mapping.channel_thread)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1704](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1704)<br>`assert_eq!(status, 303, "index={index}: {body}")`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1733](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1733)<br>`assert_eq!( headers["location"], format!( "http://example.org/rooms/{}/threads/{}", id("designers"), mapping.channel_thread_id ) )` |
| [test/controllers/github/pull_request_threads_controller_test.rb:146](../../test/controllers/github/pull_request_threads_controller_test.rb#L146)<br>`assert_equal [ mapping.channel_thread_id ], ChannelThread.ids - thread_ids_before` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1722](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1722)<br>`assert_eq!( ids.into_iter() .filter(\|i\| !before.contains(i)) .collect::<Vec<_>>(), vec![mapping.channel_thread_id] )` |

## WS15g-018

Rails declaration: `test/controllers/github/pull_request_threads_controller_test.rb:66` — discuss reuses the winner and drops the loser when the race is lost at the validation

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_mapping_race_recovers_unique_index_and_validation_losers_over_http`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary. The create! race boundary mirrors Rails simulate_mapping_race: inserts the real winning message/thread/membership/mapping before the one-shot uniqueness error; the real HTTP producer removes the loser. assert_mapping_race_loser_cleaned_up calls the three original helper assertions, each separately cited below.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/pull_request_threads_controller_test.rb:72](../../test/controllers/github/pull_request_threads_controller_test.rb#L72)<br>`assert_difference -> { Github::PullRequestThread.count }, 1 do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1709](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1709)<br>`assert_eq!( c.query_row( "SELECT COUNT(*) FROM github_pull_request_threads", [], \|r\| r.get::<_, i64>(0) )?, 1 )` |
| [test/controllers/github/pull_request_threads_controller_test.rb:77](../../test/controllers/github/pull_request_threads_controller_test.rb#L77)<br>`assert_mapping_race_loser_cleaned_up(thread_ids_before)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1709](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1709)<br>`assert_eq!( c.query_row( "SELECT COUNT(*) FROM github_pull_request_threads", [], \|r\| r.get::<_, i64>(0) )?, 1 )`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1704](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1704)<br>`assert_eq!(status, 303, "index={index}: {body}")`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1733](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1733)<br>`assert_eq!( headers["location"], format!( "http://example.org/rooms/{}/threads/{}", id("designers"), mapping.channel_thread_id ) )`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1722](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1722)<br>`assert_eq!( ids.into_iter() .filter(\|i\| !before.contains(i)) .collect::<Vec<_>>(), vec![mapping.channel_thread_id] )` |
| [test/controllers/github/pull_request_threads_controller_test.rb:143](../../test/controllers/github/pull_request_threads_controller_test.rb#L143)<br>`assert_equal 1, Github::PullRequestThread.count` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1709](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1709)<br>`assert_eq!( c.query_row( "SELECT COUNT(*) FROM github_pull_request_threads", [], \|r\| r.get::<_, i64>(0) )?, 1 )` |
| [test/controllers/github/pull_request_threads_controller_test.rb:145](../../test/controllers/github/pull_request_threads_controller_test.rb#L145)<br>`assert_redirected_to room_thread_path(@room, mapping.channel_thread)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1704](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1704)<br>`assert_eq!(status, 303, "index={index}: {body}")`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1733](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1733)<br>`assert_eq!( headers["location"], format!( "http://example.org/rooms/{}/threads/{}", id("designers"), mapping.channel_thread_id ) )` |
| [test/controllers/github/pull_request_threads_controller_test.rb:146](../../test/controllers/github/pull_request_threads_controller_test.rb#L146)<br>`assert_equal [ mapping.channel_thread_id ], ChannelThread.ids - thread_ids_before` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1722](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1722)<br>`assert_eq!( ids.into_iter() .filter(\|i\| !before.contains(i)) .collect::<Vec<_>>(), vec![mapping.channel_thread_id] )` |

## WS15g-020

Rails declaration: `test/controllers/github/webhooks_controller_test.rb:26` — valid pull_request signature updates the record and broadcasts once

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_webhook_signed_fetch_updates_and_broadcasts_once_to_its_room`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/webhooks_controller_test.rb:32](../../test/controllers/github/webhooks_controller_test.rb#L32)<br>`assert_broadcasts stream, 1 do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2245](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2245)<br>`assert_eq!( publications .take() .iter() .filter(\|(stream, _)\| stream == &room) .count(), 1 )` |
| [test/controllers/github/webhooks_controller_test.rb:38](../../test/controllers/github/webhooks_controller_test.rb#L38)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2241](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2241)<br>`assert_eq!(webhook_post(&f,json!({"repository":{"full_name":"rails/rails"},"pull_request":{"number":123,"base":{"repo":{"full_name":"rails/rails"}}}}),"d-valid-fetch").await,200)` |
| [test/controllers/github/webhooks_controller_test.rb:39](../../test/controllers/github/webhooks_controller_test.rb#L39)<br>`assert_equal "Add shiny things", @pull_request.reload.title` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2256](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2256)<br>`assert_eq!( PullRequest::find(c, pr)?.title.as_deref(), Some("Add shiny things") )` |

## WS15g-021

Rails declaration: `test/controllers/github/webhooks_controller_test.rb:220` — subscribed repositories enqueue subscription delivery and post once

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_webhook_subscription_queues_posts_and_deduplicates_redelivery`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/webhooks_controller_test.rb:224](../../test/controllers/github/webhooks_controller_test.rb#L224)<br>`assert_enqueued_jobs 1, only: Github::DeliverSubscriptionEventJob do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2291](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2291)<br>`assert_eq!(jobs.len() as i64,job_before+1)` |
| [test/controllers/github/webhooks_controller_test.rb:226](../../test/controllers/github/webhooks_controller_test.rb#L226)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2288](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2288)<br>`assert_eq!(webhook_post(&f, body.clone(), "d-subscription").await, 200)` |
| [test/controllers/github/webhooks_controller_test.rb:229](../../test/controllers/github/webhooks_controller_test.rb#L229)<br>`assert_difference -> { @room.messages.count }, 1 do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2296](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2296)<br>`assert_eq!(c.query_row("SELECT COUNT(*) FROM messages WHERE room_id=?",[id("designers")],\|r\|r.get::<_,i64>(0))?,before+1)` |
| [test/controllers/github/webhooks_controller_test.rb:234](../../test/controllers/github/webhooks_controller_test.rb#L234)<br>`assert_equal User.active_bots.find_by!(name: "GitHub"), message.creator` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2299](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2299)<br>`assert_eq!(bot.name,"GitHub")`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2300](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2300)<br>`assert!(bot.is_bot()&&bot.is_active())` |
| [test/controllers/github/webhooks_controller_test.rb:235](../../test/controllers/github/webhooks_controller_test.rb#L235)<br>`assert_includes message.markdown_source, "https://github.com/rails/rails/pull/12"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2301](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2301)<br>`assert!(message.markdown_source.as_deref().unwrap().contains("https://github.com/rails/rails/pull/12"))` |
| [test/controllers/github/webhooks_controller_test.rb:236](../../test/controllers/github/webhooks_controller_test.rb#L236)<br>`assert_equal [ 12 ], message.github_pull_requests.map(&:number)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2302](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2302)<br>`assert_eq!(PullRequest::for_message(c,message.id)?.iter().map(\|p\|p.number).collect::<Vec<_>>(),vec![12])` |

## WS15g-022

Rails declaration: `test/controllers/github/webhooks_controller_test.rb:239` — redelivered subscription events post nothing

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_webhook_subscription_queues_posts_and_deduplicates_redelivery`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/github/webhooks_controller_test.rb:245](../../test/controllers/github/webhooks_controller_test.rb#L245)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2288](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2288)<br>`assert_eq!(webhook_post(&f, body.clone(), "d-subscription").await, 200)` |
| [test/controllers/github/webhooks_controller_test.rb:247](../../test/controllers/github/webhooks_controller_test.rb#L247)<br>`assert_equal 1, @room.messages.where("markdown_source LIKE ?", "%opened pull request%").count` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2303](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2303)<br>`assert_eq!(c.query_row("SELECT COUNT(*) FROM messages WHERE room_id=? AND markdown_source LIKE '%opened pull request%'",[id("designers")],\|r\|r.get::<_,i64>(0))?,1)` |
| [test/controllers/github/webhooks_controller_test.rb:249](../../test/controllers/github/webhooks_controller_test.rb#L249)<br>`assert_no_enqueued_jobs only: Github::DeliverSubscriptionEventJob do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2309](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2309)<br>`assert_eq!(jobs_after, jobs_before)` |
| [test/controllers/github/webhooks_controller_test.rb:251](../../test/controllers/github/webhooks_controller_test.rb#L251)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2307](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2307)<br>`assert_eq!(webhook_post(&f, body, "d-subscription").await, 200)` |

## WS15g-023

Rails declaration: `test/controllers/rooms/github/pull_request_cards_controller_test.rb:105` — a cached denial no longer applies after the member relinks

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_private_card_relink_retires_denial_and_transport_error_is_not_cached`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:109](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L109)<br>`assert_redirected_to user_profile_path` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1845](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1845)<br>`assert_eq!(status, 302)`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1846](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1846)<br>`assert_eq!(headers["location"], "http://example.org/users/me/profile")` |
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:116](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L116)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1853](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1853)<br>`assert_eq!(status, 200)` |
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:117](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L117)<br>`assert_select ".github-pr-card", count: 0` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1854](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1854)<br>`assert_eq!(class_count(&html, "github-pr-card"), 0)` |
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:126](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L126)<br>`assert_redirected_to user_profile_path` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1868](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1868)<br>`assert_eq!(status, 302)`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1869](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1869)<br>`assert_eq!(headers["location"], "http://example.org/users/me/profile")` |
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:132](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L132)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1876](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1876)<br>`assert_eq!(status, 200)` |
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:133](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L133)<br>`assert_select ".github-pr-card__title", text: "Secret plans"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1877](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1877)<br>`assert!(html.contains("class=\"github-pr-card__title\">Secret plans</p>"))` |
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:135](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L135)<br>`assert_requested :get, "https://api.github.com/repos/acme/secret", times: 2` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1878](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1878)<br>`assert_eq!( f.server .received() .iter() .filter(\|r\| r.method == "GET" && r.target == "/repos/acme/secret") .count(), 2 )` |

## WS15g-024

Rails declaration: `test/controllers/rooms/github/pull_request_cards_controller_test.rb:139` — a transport error renders the empty frame and caches nothing

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_private_card_relink_retires_denial_and_transport_error_is_not_cached`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:146](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L146)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1853](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1853)<br>`assert_eq!(status, 200)` |
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:147](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L147)<br>`assert_select ".github-pr-card", count: 0` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1854](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1854)<br>`assert_eq!(class_count(&html, "github-pr-card"), 0)` |
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:148](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L148)<br>`assert_not_includes response.body, "Secret plans"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1856](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1856)<br>`assert!(!html.contains("Secret plans"))` |
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:157](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L157)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1876](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1876)<br>`assert_eq!(status, 200)` |
| [test/controllers/rooms/github/pull_request_cards_controller_test.rb:158](../../test/controllers/rooms/github/pull_request_cards_controller_test.rb#L158)<br>`assert_select ".github-pr-card__title", text: "Secret plans"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1877](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1877)<br>`assert!(html.contains("class=\"github-pr-card__title\">Secret plans</p>"))` |

## WS15g-025

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:35` — cache key for a message without pull requests is just the message

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_no_pr_key_matches_original_slots`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:36](../../test/helpers/github_pull_requests_helper_test.rb#L36)<br>`assert_equal [ messages(:first), nil, nil, nil, nil, false, false, nil, nil, nil, MessagesHelper::PRESENTATION_CACHE_VERSION ], message_with_pr_cards_cache_key(messages(:first))` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:176](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L176)<br>`assert_eq!(actual, format!("{record}/////false/false////3"))` |

## WS15g-026

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:41` — cache key changes when a thread reply is posted

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_frozen_reply_counts_add_delete_and_zero_queries`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:50](../../test/helpers/github_pull_requests_helper_test.rb#L50)<br>`assert_not_equal before, message_with_pr_cards_cache_key(Message.with_rendering_details.find(parent.id))` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:239](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L239)<br>`assert_ne!( before, key(&f, parent).await, "frozen reply-count dependency delete={delete}" )` |

## WS15g-027

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:53` — cache key changes when an older thread reply is deleted

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_frozen_reply_counts_add_delete_and_zero_queries`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:63](../../test/helpers/github_pull_requests_helper_test.rb#L63)<br>`assert_not_equal before, message_with_pr_cards_cache_key(Message.with_rendering_details.find(parent.id))` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:239](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L239)<br>`assert_ne!( before, key(&f, parent).await, "frozen reply-count dependency delete={delete}" )` |

## WS15g-028

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:66` — cache key reads the reply count without a query

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_frozen_reply_counts_add_delete_and_zero_queries`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:73](../../test/helpers/github_pull_requests_helper_test.rb#L73)<br>`assert_no_queries(include_schema: false) do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:294](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L294)<br>`assert_eq!(reads.load(std::sync::atomic::Ordering::SeqCst), 0)` |
| [test/helpers/github_pull_requests_helper_test.rb:74](../../test/helpers/github_pull_requests_helper_test.rb#L74)<br>`assert_includes message_with_pr_cards_cache_key(loaded), 1` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:293](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L293)<br>`assert!(key.split('/').any(\|slot\| slot == "1"))` |

## WS15g-029

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:78` — cache key carries the streaming flag

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_streaming_steps_quotes_polls_and_system_notes`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:86](../../test/helpers/github_pull_requests_helper_test.rb#L86)<br>`assert_not_equal before, message_with_pr_cards_cache_key(message.reload)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:314](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L314)<br>`assert_ne!(before, key(&f, first).await)` |

## WS15g-030

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:89` — cache key changes when a step is added to the message

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_streaming_steps_quotes_polls_and_system_notes`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:100](../../test/helpers/github_pull_requests_helper_test.rb#L100)<br>`assert_not_equal before, message_with_pr_cards_cache_key(message.reload)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:351](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L351)<br>`assert_ne!(before, key(&f, step_message).await)` |

## WS15g-034

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:151` — cache key changes when a quoted source is deleted

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_streaming_steps_quotes_polls_and_system_notes`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:164](../../test/helpers/github_pull_requests_helper_test.rb#L164)<br>`assert_not_equal before, message_with_pr_cards_cache_key(Message.with_rendering_details.find(quote.id))` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:387](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L387)<br>`assert_ne!(before, key(&f, quote).await)` |

## WS15g-035

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:167` — cache key changes when a poll is voted and retracted

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_streaming_steps_quotes_polls_and_system_notes`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:178](../../test/helpers/github_pull_requests_helper_test.rb#L178)<br>`assert_not_equal before, voted_key` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:430](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L430)<br>`assert_ne!(before, voted)` |
| [test/helpers/github_pull_requests_helper_test.rb:183](../../test/helpers/github_pull_requests_helper_test.rb#L183)<br>`assert_not_equal voted_key, message_with_pr_cards_cache_key(message.reload)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:438](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L438)<br>`assert_ne!(voted, key(&f, question).await)` |

## WS15g-036

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:186` — cache key carries the system note flag

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_streaming_steps_quotes_polls_and_system_notes`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:190](../../test/helpers/github_pull_requests_helper_test.rb#L190)<br>`assert_includes message_with_pr_cards_cache_key(note), true` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:459](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L459)<br>`assert_eq!(key(&f, note).await.rsplit('/').nth(5), Some("true"))` |
| [test/helpers/github_pull_requests_helper_test.rb:191](../../test/helpers/github_pull_requests_helper_test.rb#L191)<br>`assert_includes message_with_pr_cards_cache_key(messages(:first)), false` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:460](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L460)<br>`assert_eq!(key(&f, first).await.rsplit('/').nth(5), Some("false"))` |

## WS15g-037

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:194` — cache key changes when the message is pinned and unpinned

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_pins_and_newer_card_unpin`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:203](../../test/helpers/github_pull_requests_helper_test.rb#L203)<br>`assert_not_equal before, pinned_key` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:492](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L492)<br>`assert_ne!(before, pinned)` |
| [test/helpers/github_pull_requests_helper_test.rb:208](../../test/helpers/github_pull_requests_helper_test.rb#L208)<br>`assert_not_equal pinned_key, message_with_pr_cards_cache_key(message.reload)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:521](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L521)<br>`assert_ne!( pinned, key(&f, message).await, "newer referenced card={newer}" )` |

## WS15g-038

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:211` — cache key changes on unpin even when a referenced card is newer

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_pins_and_newer_card_unpin`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:224](../../test/helpers/github_pull_requests_helper_test.rb#L224)<br>`assert_not_equal pinned_key, message_with_pr_cards_cache_key(message.reload)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:521](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L521)<br>`assert_ne!( pinned, key(&f, message).await, "newer referenced card={newer}" )`<br><br>[rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:519](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L519)<br>`assert_ne!(before_unpin, key(&f, message).await)` |

## WS15g-039

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:227` — cache key changes when a referenced X post is fetched

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_x_and_link_fetch_dependencies`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:238](../../test/helpers/github_pull_requests_helper_test.rb#L238)<br>`assert_not_equal before, message_with_pr_cards_cache_key(message.reload)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:564](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L564)<br>`assert_ne!(before, key(&f, message).await)` |

## WS15g-040

Rails declaration: `test/helpers/github_pull_requests_helper_test.rb:241` — cache key changes when a referenced link embed is fetched

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_cache_x_and_link_fetch_dependencies`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/helpers/github_pull_requests_helper_test.rb:252](../../test/helpers/github_pull_requests_helper_test.rb#L252)<br>`assert_not_equal before, message_with_pr_cards_cache_key(message.reload)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:587](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L587)<br>`assert_ne!(before, key(&f, message).await)` |

## WS15g-042

Rails declaration: `test/integration/github_pr_cards_test.rb:156` — rendering a room page costs no extra queries per message with a PR link

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_room_http_query_count_stays_flat_for_two_then_six_pr_messages`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/github_pr_cards_test.rb:162](../../test/integration/github_pr_cards_test.rb#L162)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1905](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1905)<br>`assert_eq!( request(&f, "GET", &path, Value::Null, json!({})).await.0, 200 )` |
| [test/integration/github_pr_cards_test.rb:169](../../test/integration/github_pr_cards_test.rb#L169)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1914](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1914)<br>`assert_eq!(small.0, 200)` |
| [test/integration/github_pr_cards_test.rb:174](../../test/integration/github_pr_cards_test.rb#L174)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1932](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1932)<br>`assert_eq!(large.0, 200)` |
| [test/integration/github_pr_cards_test.rb:176](../../test/integration/github_pr_cards_test.rb#L176)<br>`assert_equal small, large,` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1933](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1933)<br>`assert_eq!( small_queries, large_queries, "room SELECTs with two/six PR messages" )` |

## WS15g-043

Rails declaration: `test/integration/github_pr_cards_test.rb:254` — the open-room join page leaks no card content to non-members

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_open_room_join_page_omits_card_and_offers_join`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/github_pr_cards_test.rb:268](../../test/integration/github_pr_cards_test.rb#L268)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:805](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L805)<br>`assert_eq!(status, 200)` |
| [test/integration/github_pr_cards_test.rb:269](../../test/integration/github_pr_cards_test.rb#L269)<br>`assert_select ".github-pr-card", count: 0` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:806](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L806)<br>`assert_eq!(class_count(&html, "github-pr-card"), 0)` |
| [test/integration/github_pr_cards_test.rb:270](../../test/integration/github_pr_cards_test.rb#L270)<br>`assert_select "button", text: "Join channel"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:807](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L807)<br>`assert!(html.contains("Join channel</button>"))` |

## WS15g-047

Rails declaration: `test/integration/github_pr_threads_test.rb:62` — the files summary omits the more line when everything is shown

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_thread_files_loaded_exact_loading_ordinary_xss_and_private`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/github_pr_threads_test.rb:73](../../test/integration/github_pr_threads_test.rb#L73)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:710](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L710)<br>`assert_eq!(status, 200, "{case}")` |
| [test/integration/github_pr_threads_test.rb:74](../../test/integration/github_pr_threads_test.rb#L74)<br>`assert_select ".github-pr-files__file", count: 1` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:733](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L733)<br>`assert_eq!(class_count(&html, "github-pr-files__file"), 1)` |
| [test/integration/github_pr_threads_test.rb:75](../../test/integration/github_pr_threads_test.rb#L75)<br>`assert_select ".github-pr-files__more", count: 0` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:734](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L734)<br>`assert_eq!(class_count(&html, "github-pr-files__more"), 0)` |

## WS15g-048

Rails declaration: `test/integration/github_pr_threads_test.rb:78` — a PR thread without fetched files shows a loading summary

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_thread_files_loaded_exact_loading_ordinary_xss_and_private`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/github_pr_threads_test.rb:86](../../test/integration/github_pr_threads_test.rb#L86)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:710](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L710)<br>`assert_eq!(status, 200, "{case}")` |
| [test/integration/github_pr_threads_test.rb:87](../../test/integration/github_pr_threads_test.rb#L87)<br>`assert_select ".github-pr-thread-header .github-pr-card", count: 1` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:737](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L737)<br>`assert_eq!(class_count(h, "github-pr-card"), 1)` |
| [test/integration/github_pr_threads_test.rb:88](../../test/integration/github_pr_threads_test.rb#L88)<br>`assert_select ".github-pr-files__loading", text: /Loading files/` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:740](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L740)<br>`assert!(dom.descendants(root).into_iter().any(\|node\| { dom.attr(node, "class").is_some_and(\|classes\| { classes .split_whitespace() .any(\|class\| class == "github-pr-files__loading") }) && dom.text_content(node).contains("Loading files") }))` |
| [test/integration/github_pr_threads_test.rb:89](../../test/integration/github_pr_threads_test.rb#L89)<br>`assert_select ".github-pr-files__file", count: 0` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:747](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L747)<br>`assert_eq!(class_count(&html, "github-pr-files__file"), 0)` |

## WS15g-049

Rails declaration: `test/integration/github_pr_threads_test.rb:92` — an ordinary thread shows no PR header

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_thread_files_loaded_exact_loading_ordinary_xss_and_private`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/github_pr_threads_test.rb:101](../../test/integration/github_pr_threads_test.rb#L101)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:710](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L710)<br>`assert_eq!(status, 200, "{case}")` |
| [test/integration/github_pr_threads_test.rb:102](../../test/integration/github_pr_threads_test.rb#L102)<br>`assert_select ".github-pr-thread-header", count: 0` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:712](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L712)<br>`assert_eq!(class_count(&html, "github-pr-thread-header"), 0)` |

## WS15g-050

Rails declaration: `test/integration/github_pr_threads_test.rb:105` — file paths from the API render as text

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_thread_files_loaded_exact_loading_ordinary_xss_and_private`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/github_pr_threads_test.rb:117](../../test/integration/github_pr_threads_test.rb#L117)<br>`assert_response :success` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:710](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L710)<br>`assert_eq!(status, 200, "{case}")` |
| [test/integration/github_pr_threads_test.rb:118](../../test/integration/github_pr_threads_test.rb#L118)<br>`assert_select ".github-pr-files__path", text: malicious_name` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:752](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L752)<br>`assert!(paths.iter().any(\|&path\| dom.text_content(path) == "<img src=x onerror=\"window.__prFilesXss = true\">"))` |
| [test/integration/github_pr_threads_test.rb:119](../../test/integration/github_pr_threads_test.rb#L119)<br>`assert_select ".github-pr-files__path img", count: 0` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:754](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L754)<br>`assert_eq!( paths .iter() .flat_map(\|&path\| dom.descendants(path)) .filter(\|&node\| dom.local_name(node) == Some("img")) .count(), 0 )` |

## WS15g-053

Rails declaration: `test/jobs/github/deliver_subscription_event_job_test.rb:77` — review_requested posts and records an inbox item for the linked member

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_review_notification_registered_job_scopes_access_preference_and_thread_source`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/github/deliver_subscription_event_job_test.rb:80](../../test/jobs/github/deliver_subscription_event_job_test.rb#L80)<br>`assert_difference -> { ActivityItem.where(user: users(:kevin), event_type: "pr_review_request").count }, 1 do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1988](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1988)<br>`assert_eq!(items,before_items+1)` |
| [test/jobs/github/deliver_subscription_event_job_test.rb:85](../../test/jobs/github/deliver_subscription_event_job_test.rb#L85)<br>`assert_equal "**bob** requested a review from **Kevin-GH** on #12: Fix login\nhttps://github.com/rails/rails/pull/12", message.markdown_source` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1989](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1989)<br>`assert_eq!(message.1,"**bob** requested a review from **Kevin-GH** on #12: Fix login\nhttps://github.com/rails/rails/pull/12")` |
| [test/jobs/github/deliver_subscription_event_job_test.rb:88](../../test/jobs/github/deliver_subscription_event_job_test.rb#L88)<br>`assert_equal message, item.source` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1993](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1993)<br>`assert_eq!((item.source_type.as_str(),item.source_id),("Message",message.0))` |
| [test/jobs/github/deliver_subscription_event_job_test.rb:89](../../test/jobs/github/deliver_subscription_event_job_test.rb#L89)<br>`assert_includes ActivityItem.accessible_to(users(:kevin)), item` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1995](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1995)<br>`assert!(campfire_db::ActivityItem::accessible_to(tx.conn(),&kevin)?.iter().any(\|i\|i.id==item.id))` |
| [test/jobs/github/deliver_subscription_event_job_test.rb:92](../../test/jobs/github/deliver_subscription_event_job_test.rb#L92)<br>`assert_not ActivityItem.accessible_to(users(:kevin)).exists?(item.id)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1998](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1998)<br>`assert!(campfire_db::ActivityItem::find_accessible(tx.conn(),&kevin,item.id)?.is_none())` |

## WS15g-054

Rails declaration: `test/jobs/github/deliver_subscription_event_job_test.rb:111` — review_requested skips the item when the reviewer switched them off

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_review_notification_registered_job_scopes_access_preference_and_thread_source`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/github/deliver_subscription_event_job_test.rb:114](../../test/jobs/github/deliver_subscription_event_job_test.rb#L114)<br>`assert_difference -> { @room.messages.count }, 1 do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1982](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1982)<br>`assert_eq!(count,before_messages+1)` |
| [test/jobs/github/deliver_subscription_event_job_test.rb:115](../../test/jobs/github/deliver_subscription_event_job_test.rb#L115)<br>`assert_no_difference -> { ActivityItem.where(event_type: "pr_review_request").count } do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1984](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1984)<br>`assert_eq!(items,before_items)` |
| [test/jobs/github/deliver_subscription_event_job_test.rb:125](../../test/jobs/github/deliver_subscription_event_job_test.rb#L125)<br>`assert_equal "mention", ActivityItem.find_by!(user: users(:kevin), source: mention).event_type` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1986](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1986)<br>`assert_eq!(campfire_db::ActivityItem::find_by_user_and_source(tx.conn(),id("kevin"),"Message",mention.id)?.unwrap().event_type,"mention")` |

## WS15g-055

Rails declaration: `test/jobs/github/deliver_subscription_event_job_test.rb:289` — review_requested in a PR thread points the inbox item at the thread message

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_review_notification_registered_job_scopes_access_preference_and_thread_source`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/github/deliver_subscription_event_job_test.rb:296](../../test/jobs/github/deliver_subscription_event_job_test.rb#L296)<br>`assert_equal "**bob** requested a review from **Kevin-GH** on #12: Fix login\nhttps://github.com/rails/rails/pull/12", reply.markdown_source` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1989](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1989)<br>`assert_eq!(message.1,"**bob** requested a review from **Kevin-GH** on #12: Fix login\nhttps://github.com/rails/rails/pull/12")` |
| [test/jobs/github/deliver_subscription_event_job_test.rb:299](../../test/jobs/github/deliver_subscription_event_job_test.rb#L299)<br>`assert_equal reply, item.source` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1993](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1993)<br>`assert_eq!((item.source_type.as_str(),item.source_id),("Message",message.0))` |
| [test/jobs/github/deliver_subscription_event_job_test.rb:300](../../test/jobs/github/deliver_subscription_event_job_test.rb#L300)<br>`assert_includes ActivityItem.accessible_to(users(:kevin)), item` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1995](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1995)<br>`assert!(campfire_db::ActivityItem::accessible_to(tx.conn(),&kevin)?.iter().any(\|i\|i.id==item.id))` |

## WS15g-056

Rails declaration: `test/jobs/github/fetch_pull_request_job_test.rb:305` — card updates broadcast the thread header to mapped thread streams

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_fetch_registered_job_replaces_mapped_header_and_is_silent_without_references`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/github/fetch_pull_request_job_test.rb:322](../../test/jobs/github/fetch_pull_request_job_test.rb#L322)<br>`assert header_stream, "expected a thread header replace stream, got: #{fragment.to_html}"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2163](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2163)<br>`assert_eq!(header_streams.len(), 1, "{html}")` |
| [test/jobs/github/fetch_pull_request_job_test.rb:323](../../test/jobs/github/fetch_pull_request_job_test.rb#L323)<br>`assert_equal 1, header_stream.css(".github-pr-card").size` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2165](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2165)<br>`assert_eq!(class_nodes(&dom, header_stream, "github-pr-card").len(), 1)` |
| [test/jobs/github/fetch_pull_request_job_test.rb:324](../../test/jobs/github/fetch_pull_request_job_test.rb#L324)<br>`assert_includes header_stream.at_css(".github-pr-card__title").text, "Add shiny things"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2166](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2166)<br>`assert!( dom.text_content(class_nodes(&dom, header_stream, "github-pr-card__title")[0]) .contains("Add shiny things") )` |
| [test/jobs/github/fetch_pull_request_job_test.rb:325](../../test/jobs/github/fetch_pull_request_job_test.rb#L325)<br>`assert_includes header_stream.at_css(".github-pr-files__path").text, "app/models/user.rb"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2170](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2170)<br>`assert!( dom.text_content(class_nodes(&dom, header_stream, "github-pr-files__path")[0]) .contains("app/models/user.rb") )` |

## WS15g-057

Rails declaration: `test/jobs/github/fetch_pull_request_job_test.rb:328` — card updates broadcast nothing without referencing messages or mappings

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_fetch_registered_job_replaces_mapped_header_and_is_silent_without_references`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/github/fetch_pull_request_job_test.rb:334](../../test/jobs/github/fetch_pull_request_job_test.rb#L334)<br>`Turbo::StreamsChannel.expects(:broadcast_replace_to).never` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:2186](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L2186)<br>`assert!(publications.take().is_empty())` |

## WS15g-058

Rails declaration: `test/jobs/github/perform_agent_action_job_test.rb:34` — approving a github action enqueues the job, denying does not

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_enqueues_exact_argument_only_for_approved_github_action`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/github/perform_agent_action_job_test.rb:37](../../test/jobs/github/perform_agent_action_job_test.rb#L37)<br>`assert_enqueued_with(job: Github::PerformAgentActionJob, args: [ approvable.id ]) do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1419](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1419)<br>`assert_eq!(jobs, vec![json!({"approval_id":approve})])` |
| [test/jobs/github/perform_agent_action_job_test.rb:42](../../test/jobs/github/perform_agent_action_job_test.rb#L42)<br>`assert_no_enqueued_jobs only: Github::PerformAgentActionJob do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1422](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1422)<br>`assert_eq!(denied, jobs)` |

## WS15g-059

Rails declaration: `test/jobs/github/perform_agent_action_job_test.rb:78` — approving a non-github action enqueues nothing

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_approval_enqueues_exact_argument_only_for_approved_github_action`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/github/perform_agent_action_job_test.rb:81](../../test/jobs/github/perform_agent_action_job_test.rb#L81)<br>`assert_no_enqueued_jobs only: Github::PerformAgentActionJob do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1425](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1425)<br>`assert_eq!(non_github, jobs)` |

## WS15g-061

Rails declaration: `test/models/github/write_client_test.rb:102` — network errors raise Error without logging the token

Executed test: `campfire::bin/campfire controllers::github::cutover_d_tests::cutover_d_write_timeout_logs_warning_without_member_token`.

Original Rails fixtures, explicit variants and mutation starting state; real signed HTTP, original model API, registered durable jobs, rendered markup or subscribed Action Cable. Counts retain Rails room/agent/source scope. External GitHub responses are the only fake boundary.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/github/write_client_test.rb:107](../../test/models/github/write_client_test.rb#L107)<br>`error = assert_raises(Github::WriteClient::Error) do` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1636](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1636)<br>`assert_eq!( error.kind, crate::integrations::github::client::ErrorKind::Other )` |
| [test/models/github/write_client_test.rb:110](../../test/models/github/write_client_test.rb#L110)<br>`assert_match(/Could not reach GitHub/, error.message)` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1640](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1640)<br>`assert!(error.message.contains("Could not reach GitHub"))` |
| [test/models/github/write_client_test.rb:113](../../test/models/github/write_client_test.rb#L113)<br>`assert_includes log.string, "Github::WriteClient request failed"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1641](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1641)<br>`assert!(logs.text().contains("Github::WriteClient request failed"))` |
| [test/models/github/write_client_test.rb:114](../../test/models/github/write_client_test.rb#L114)<br>`assert_not_includes log.string, "user-token-123"` | [rust/crates/campfire/src/controllers/github/cutover_d_tests.rs:1642](../../rust/crates/campfire/src/controllers/github/cutover_d_tests.rs#L1642)<br>`assert!(!logs.text().contains("user-token-123"))` |

## WS15g-062

Rails declaration: `test/system/github_pr_write_actions_test.rb:6` — a linked member comments from a PR thread and sees the inline confirmation

Executed test: `campfire::bin/campfire controllers::ws15_original_github_browser_tests::original_browser_ws15g_062`.

The byte-identical original Rails test body executes in native Capybara/Selenium against real Rust HTTP, Turbo forms and rendered markup. Original account, message and GitHub fixture setup and exact WebMock request assertions are preserved.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/system/github_pr_write_actions_test.rb:40](../../test/system/github_pr_write_actions_test.rb#L40)<br>`assert_selector ".github-pr-thread-header .github-pr-card__title", text: "Fix login", wait: 10` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:40](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L40)<br>`assert_selector ".github-pr-thread-header .github-pr-card__title", text: "Fix login", wait: 10`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/system/github_pr_write_actions_test.rb:47](../../test/system/github_pr_write_actions_test.rb#L47)<br>`assert_selector ".github-pr-write__notice", text: "Comment posted on GitHub as @jz", wait: 10` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:47](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L47)<br>`assert_selector ".github-pr-write__notice", text: "Comment posted on GitHub as @jz", wait: 10`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/system/github_pr_write_actions_test.rb:48](../../test/system/github_pr_write_actions_test.rb#L48)<br>`assert_field "Comment on GitHub", with: ""` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:48](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L48)<br>`assert_field "Comment on GitHub", with: ""`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/system/github_pr_write_actions_test.rb:49](../../test/system/github_pr_write_actions_test.rb#L49)<br>`assert_requested stub` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:49](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L49)<br>`assert_requested stub`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/test_helpers/system_test_helper.rb:65](../../test/test_helpers/system_test_helper.rb#L65)<br>`assert_selector "a.btn", text: "Designers", wait: 10` | [rust/reference-tools/users/original_browser/test/test_helpers/system_test_helper.rb:65](../../rust/reference-tools/users/original_browser/test/test_helpers/system_test_helper.rb#L65)<br>`assert_selector "a.btn", text: "Designers", wait: 10`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |

## WS15g-063

Rails declaration: `test/system/github_pr_write_actions_test.rb:52` — a linked member requests a review from a PR thread and sees the inline confirmation

Executed test: `campfire::bin/campfire controllers::ws15_original_github_browser_tests::original_browser_ws15g_063`.

The byte-identical original Rails test body executes in native Capybara/Selenium against real Rust HTTP, Turbo forms and rendered markup. Original account, message and GitHub fixture setup and exact WebMock request assertions are preserved.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/system/github_pr_write_actions_test.rb:86](../../test/system/github_pr_write_actions_test.rb#L86)<br>`assert_selector ".github-pr-thread-header .github-pr-card__title", text: "Fix login", wait: 10` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:86](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L86)<br>`assert_selector ".github-pr-thread-header .github-pr-card__title", text: "Fix login", wait: 10`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/system/github_pr_write_actions_test.rb:93](../../test/system/github_pr_write_actions_test.rb#L93)<br>`assert_selector ".github-pr-write__notice", text: "Requested review from @alice, @bob on GitHub as @jz", wait: 10` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:93](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L93)<br>`assert_selector ".github-pr-write__notice", text: "Requested review from @alice, @bob on GitHub as @jz", wait: 10`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/system/github_pr_write_actions_test.rb:94](../../test/system/github_pr_write_actions_test.rb#L94)<br>`assert_field "GitHub usernames", with: ""` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:94](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L94)<br>`assert_field "GitHub usernames", with: ""`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/system/github_pr_write_actions_test.rb:95](../../test/system/github_pr_write_actions_test.rb#L95)<br>`assert_requested stub` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:95](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L95)<br>`assert_requested stub`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/test_helpers/system_test_helper.rb:65](../../test/test_helpers/system_test_helper.rb#L65)<br>`assert_selector "a.btn", text: "Designers", wait: 10` | [rust/reference-tools/users/original_browser/test/test_helpers/system_test_helper.rb:65](../../rust/reference-tools/users/original_browser/test/test_helpers/system_test_helper.rb#L65)<br>`assert_selector "a.btn", text: "Designers", wait: 10`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |

## WS15g-064

Rails declaration: `test/system/github_pr_write_actions_test.rb:98` — a member without a linked token sees the connect prompt in the thread

Executed test: `campfire::bin/campfire controllers::ws15_original_github_browser_tests::original_browser_ws15g_064`.

The byte-identical original Rails test body executes in native Capybara/Selenium against real Rust HTTP, Turbo forms and rendered markup. Original account, message and GitHub fixture setup and exact WebMock request assertions are preserved.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/system/github_pr_write_actions_test.rb:123](../../test/system/github_pr_write_actions_test.rb#L123)<br>`assert_selector ".github-pr-thread-header .github-pr-card__title", text: "Fix login", wait: 10` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:123](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L123)<br>`assert_selector ".github-pr-thread-header .github-pr-card__title", text: "Fix login", wait: 10`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/system/github_pr_write_actions_test.rb:124](../../test/system/github_pr_write_actions_test.rb#L124)<br>`assert_selector ".github-pr-write__connect", text: /Connect GitHub/, wait: 10` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:124](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L124)<br>`assert_selector ".github-pr-write__connect", text: /Connect GitHub/, wait: 10`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/system/github_pr_write_actions_test.rb:125](../../test/system/github_pr_write_actions_test.rb#L125)<br>`assert_no_selector ".github-pr-write__comment"` | [rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb:125](../../rust/reference-tools/users/original_browser/test/system/github_pr_write_actions_test.rb#L125)<br>`assert_no_selector ".github-pr-write__comment"`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
| [test/test_helpers/system_test_helper.rb:65](../../test/test_helpers/system_test_helper.rb#L65)<br>`assert_selector "a.btn", text: "Designers", wait: 10` | [rust/reference-tools/users/original_browser/test/test_helpers/system_test_helper.rb:65](../../rust/reference-tools/users/original_browser/test/test_helpers/system_test_helper.rb#L65)<br>`assert_selector "a.btn", text: "Designers", wait: 10`<br><br>[rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs:220](../../rust/crates/campfire/src/controllers/ws14_original_browser_tests.rs#L220)<br>`assert!( output.status.success(), "{key}: exact original browser declaration failed\n{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr) );` |
