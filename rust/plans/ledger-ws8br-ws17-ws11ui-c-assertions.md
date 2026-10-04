# Individual original assertion audit

Original declarations use Rails d7c7de92. Runtime fixtures follow the current reference pin.

Every row names the original assertion and the actual Rust check. Whole-byte/DOM checks retain tags, attributes, text and cardinality; their fixture cases are named below. Reopened gaps are explicit and retain the previous insufficient claim in JSON history.

## P0239: admins can browse the log

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_admin_browse_labels_and_export_link`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:17` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: browse. |
| `test/controllers/accounts/audit_logs_controller_test.rb:18` — `assert_select "h1", text: "Audit log"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: browse. |
| `test/controllers/accounts/audit_logs_controller_test.rb:19` — `assert_select "tbody td code", text: "user.ban"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: browse. |
| `test/controllers/accounts/audit_logs_controller_test.rb:20` — `assert_select "tbody td code", text: "user.role.change"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: browse. |
| `test/controllers/accounts/audit_logs_controller_test.rb:21` — `assert_select "tbody td", text: "Kevin <kevin@37signals.com>"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: browse. |
| `test/controllers/accounts/audit_logs_controller_test.rb:22` — `assert_select "a[href='#{account_audit_log_path(format: :csv)}']", text: "Export CSV"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: browse. |

## P0240: members are forbidden

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_members_forbidden`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:29` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: member. |

## P0241: visitors are sent to sign in

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_visitors_redirect_to_sign_in`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:35` — `assert_redirected_to new_session_url` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: visitor. |

## P0242: visitors cannot export CSV

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_visitors_cannot_export`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:41` — `assert_redirected_to new_session_url` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: visitor_csv. |

## P0243: members cannot export CSV

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_members_cannot_export`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:48` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: member_csv. |

## P0244: filtering by actor matches names and emails in labels

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_actor_filter_matches_label_email`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:57` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: actor. |
| `test/controllers/accounts/audit_logs_controller_test.rb:58` — `assert_select "tbody tr", count: 1` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: actor. |
| `test/controllers/accounts/audit_logs_controller_test.rb:59` — `assert_select "tbody td", text: "Jason <jason@37signals.com>"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: actor. |

## P0245: filtering by action and target type

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_action_and_target_type_filters`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:68` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: action. |
| `test/controllers/accounts/audit_logs_controller_test.rb:69` — `assert_select "tbody tr", count: 1` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: action. |
| `test/controllers/accounts/audit_logs_controller_test.rb:70` — `assert_select "tbody td code", text: "room.create"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: action. |
| `test/controllers/accounts/audit_logs_controller_test.rb:74` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: action. |
| `test/controllers/accounts/audit_logs_controller_test.rb:75` — `assert_select "tbody tr", count: 0` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: action. |

## P0246: unknown filter values are ignored

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_unknown_filters_are_ignored`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:83` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: unknown. |
| `test/controllers/accounts/audit_logs_controller_test.rb:84` — `assert_select "tbody td code", text: "user.ban"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: unknown. |
| `test/controllers/accounts/audit_logs_controller_test.rb:85` — `assert_select "tbody td code", text: "user.role.change"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: unknown. |

## P0247: filtering by date range

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_date_range_excludes_the_other_action`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:94` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: dates. |
| `test/controllers/accounts/audit_logs_controller_test.rb:95` — `assert_select "tbody td code", text: "user.ban", count: 0` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: dates. |
| `test/controllers/accounts/audit_logs_controller_test.rb:96` — `assert_select "tbody td code", text: "user.role.change"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: dates. |
| `test/controllers/accounts/audit_logs_controller_test.rb:100` — `assert_select "tbody td code", text: "user.ban"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: dates. |
| `test/controllers/accounts/audit_logs_controller_test.rb:101` — `assert_select "tbody td code", text: "user.role.change", count: 0` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: dates. |

## P0248: paging walks older entries

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_paging_links_and_row_count`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:113` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: paging. |
| `test/controllers/accounts/audit_logs_controller_test.rb:114` — `assert_match "Older entries", response.body` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: paging. |
| `test/controllers/accounts/audit_logs_controller_test.rb:115` — `assert_select "tbody tr", count: Accounts::AuditLogsController::PAGE_SIZE` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: paging. |
| `test/controllers/accounts/audit_logs_controller_test.rb:119` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: paging. |
| `test/controllers/accounts/audit_logs_controller_test.rb:120` — `assert_match "Newest entries", response.body` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: paging. |

## P0249: CSV export carries headers and the filtered rows

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_filtered_csv_headers_labels_changes_and_ip`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:129` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: csv. |
| `test/controllers/accounts/audit_logs_controller_test.rb:130` — `assert_equal "text/csv", response.media_type` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: csv. |
| `test/controllers/accounts/audit_logs_controller_test.rb:132` — `assert_equal %w[ time action actor target_type target changes ip_address user_agent ], rows.headers` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: csv. |
| `test/controllers/accounts/audit_logs_controller_test.rb:133` — `assert_equal 1, rows.length` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: csv. |
| `test/controllers/accounts/audit_logs_controller_test.rb:134` — `assert_equal "user.ban", rows[0]["action"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: csv. |
| `test/controllers/accounts/audit_logs_controller_test.rb:135` — `assert_equal "David <david@37signals.com>", rows[0]["actor"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: csv. |
| `test/controllers/accounts/audit_logs_controller_test.rb:136` — `assert_equal "User", rows[0]["target_type"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: csv. |
| `test/controllers/accounts/audit_logs_controller_test.rb:137` — `assert_equal "Kevin <kevin@37signals.com>", rows[0]["target"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: csv. |
| `test/controllers/accounts/audit_logs_controller_test.rb:138` — `assert_equal "203.0.113.7", rows[0]["ip_address"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: csv. |
| `test/controllers/accounts/audit_logs_controller_test.rb:139` — `assert_equal({ "status" => %w[ active banned ] }, JSON.parse(rows[0]["changes"]))` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: csv. |

## P0250: CSV export neutralizes formula injection

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_formula_target_is_quoted`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:153` — `assert targets.any? { &#124;target&#124; target.start_with?("'=") }, "expected a quoted formula cell in #{targets.inspect}"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Exact actual HTTP observation: status/location; DOM h1, tbody codes/cells/row counts and Export CSV href; actual filtered CSV complete bytes, including its header and every quoted column. Formula target case compares the actual parsed target-prefix predicate. Cases: formula. |

## P0253: CSV export neutralizes formula injection in request columns

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_tests::original_formula_request_column_is_quoted`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:202` — `assert_equal 1, rows.length` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Actual selected CSV complete bytes and media type: one row and the exact quoted user-agent cell. Cases: request_formula. |
| `test/controllers/accounts/audit_logs_controller_test.rb:203` — `assert_equal "'=cmd&#124;'/c calc'!A0", rows[0]["user_agent"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 | Actual selected CSV complete bytes and media type: one row and the exact quoted user-agent cell. Cases: request_formula. |

## P0230: update

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_admin_self_role_is_preserved`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/users_controller_test.rb:10` — `assert users(:david).administrator?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: role_self. |
| `test/controllers/accounts/users_controller_test.rb:14` — `assert_redirected_to edit_account_url` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: role_self. |
| `test/controllers/accounts/users_controller_test.rb:15` — `assert users(:david).reload.administrator?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: role_self. |

## P0231: destroy

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_self_removal_active_count_and_lookup`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/users_controller_test.rb:19` — `assert_difference -> { User.active.count }, -1 do` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: remove_self. |
| `test/controllers/accounts/users_controller_test.rb:23` — `assert_redirected_to edit_account_url` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: remove_self. |
| `test/controllers/accounts/users_controller_test.rb:24` — `assert_nil User.active.find_by(id: users(:david).id)` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: remove_self. |

## P0232: non-admins cannot perform actions

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_member_cannot_change_admin_role`
- `controllers::accounts::original_control_tests::original_member_cannot_remove_admin`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/users_controller_test.rb:31` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: member_role, member_remove. |
| `test/controllers/accounts/users_controller_test.rb:34` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: member_role, member_remove. |

## P0254: edit

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_styles_edit_success`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/custom_styles_controller_test.rb:11` — `assert_response :ok` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: styles_edit. |

## P0255: update

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_styles_exact_value_is_saved`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/custom_styles_controller_test.rb:15` — `assert users(:david).administrator?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: styles. |
| `test/controllers/accounts/custom_styles_controller_test.rb:19` — `assert_redirected_to edit_account_custom_styles_url` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: styles. |
| `test/controllers/accounts/custom_styles_controller_test.rb:20` — `assert_equal accounts(:signal).custom_styles, ":root { --color-text: red; }"` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: styles. |

## P0256: non-admins cannot update

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_member_styles_refused`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/custom_styles_controller_test.rb:25` — `assert users(:kevin).member?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: member_styles. |
| `test/controllers/accounts/custom_styles_controller_test.rb:28` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: member_styles. |

## P0263: create new join code

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_join_code_changes_and_redirects`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/join_codes_controller_test.rb:10` — `assert_changes -> { accounts(:signal).reload.join_code } do` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: join. |
| `test/controllers/accounts/join_codes_controller_test.rb:12` — `assert_redirected_to edit_account_url` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: join. |

## P0264: only administrators can create new join codes

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_jz_cannot_reset_join_code`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/join_codes_controller_test.rb:19` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: member_join. |

## P0214: create bans user and creates ban records from sessions

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_ban_has_two_exact_ip_records`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/bans_controller_test.rb:14` — `assert_difference -> { Ban.count }, 2 do` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: ban_two. |
| `test/controllers/users/bans_controller_test.rb:18` — `assert_redirected_to user_url(user)` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: ban_two. |
| `test/controllers/users/bans_controller_test.rb:19` — `assert Ban.exists?(ip_address: "203.0.113.1", user: user)` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: ban_two. |
| `test/controllers/users/bans_controller_test.rb:20` — `assert Ban.exists?(ip_address: "203.0.113.2", user: user)` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: ban_two. |

## P0215: create destroys user sessions

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_ban_destroys_the_single_session`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/bans_controller_test.rb:27` — `assert_difference -> { user.sessions.count }, -1 do` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: ban_session. |

## P0219: non-admins cannot ban users

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_kevin_cannot_ban_jz`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/bans_controller_test.rb:70` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: member_ban. |

## P0220: destroy removes ban records and sets user to active

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_unban_deletes_record_and_activates`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/bans_controller_test.rb:78` — `assert user.reload.banned?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: unban. |
| `test/controllers/users/bans_controller_test.rb:79` — `assert_equal 1, user.bans.count` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: unban. |
| `test/controllers/users/bans_controller_test.rb:81` — `assert_difference -> { Ban.count }, -1 do` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: unban. |
| `test/controllers/users/bans_controller_test.rb:85` — `assert_redirected_to user_url(user)` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: unban. |
| `test/controllers/users/bans_controller_test.rb:86` — `assert user.reload.active?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: unban. |

## P0221: non-admins cannot unban users

Status: **closed**. Native identities:

- `controllers::accounts::original_control_tests::original_kevin_cannot_unban_jz`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/bans_controller_test.rb:97` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 | Actual status and Location plus the reloaded fields selected by these original cases: administrator/member preconditions; active-user global count delta and active lookup; exact CSS; join-code change; global Ban count delta and exact target IPs; session count delta and active/banned preconditions. Cases: member_unban. |

## P0216: create succeeds when the user has a pending two-factor setup secret

Status: **closed**. Native identities:

- `controllers::users::ban_lifecycle_tests::banning_a_user_removes_pending_two_factor_setup_and_sessions`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/bans_controller_test.rb:37` — `assert_difference -> { user.sessions.count }, -1 do` | rust/crates/campfire/src/controllers/users/ban_lifecycle_tests.rs:90 | Before the ban there is exactly one target session and one setup secret. The real response is 302 to Kevin, and actual post-ban session/setup counts are zero. Cases: original public session/queued removal. |
| `test/controllers/users/bans_controller_test.rb:41` — `assert_redirected_to user_url(user)` | rust/crates/campfire/src/controllers/users/ban_lifecycle_tests.rs:71 | Before the ban there is exactly one target session and one setup secret. The real response is 302 to Kevin, and actual post-ban session/setup counts are zero. Cases: original public session/queued removal. |
| `test/controllers/users/bans_controller_test.rb:42` — `assert_empty TwoFactorSetupSecret.where(session_id: session.id)` | rust/crates/campfire/src/controllers/users/ban_lifecycle_tests.rs:90 | Before the ban there is exactly one target session and one setup secret. The real response is 302 to Kevin, and actual post-ban session/setup counts are zero. Cases: original public session/queued removal. |

## P0217: create enqueues RemoveBannedContentJob

Status: **closed**. Native identities:

- `controllers::users::ban_lifecycle_tests::ban_http_enqueue_is_atomic_and_writes_one_durable_remove_job`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/bans_controller_test.rb:48` — `assert_enqueued_with(job: RemoveBannedContentJob, args: [ user ]) do` | rust/crates/campfire/src/controllers/users/ban_lifecycle_tests.rs:172 | Real durable queue insert: exactly one RemoveBannedContentJob with the actual Kevin user_id argument. TestApp holds the job runner; rejected enqueue rolls back the transaction. Cases: original public session/queued removal. |

## P0218: RemoveBannedContentJob deletes messages

Status: **closed**. Native identities:

- `controllers::users::ban_lifecycle_tests::ban_http_removes_the_users_messages_through_the_real_runner`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/bans_controller_test.rb:62` — `assert_empty user.reload.messages` | rust/crates/campfire/src/controllers/users/ban_lifecycle_tests.rs:221 | The actual registered runner deletes all Kevin messages after the real POST; loop exits only at count zero, with a fixed existing deadline. The inserted message is also absent and no failed job exists. Cases: original public session/queued removal. |

## P0233: index lists icons with previews shortcodes titles and uploaders

Status: **closed**. Native identities:

- `controllers::accounts::icons::original_tests::original_acme_upload_list_and_audit_shortcodes`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:13` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:59 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/accounts/icons_controller_test.rb:14` — `assert_select "img[src=?]", workspace_icon_path(name: "acme")` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:64 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/accounts/icons_controller_test.rb:15` — `assert_select "code", text: ":acme:"` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:71 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/accounts/icons_controller_test.rb:16` — `assert_match "Acme Corp", response.body` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:78 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/accounts/icons_controller_test.rb:17` — `assert_match "Uploaded by #{icon.creator.name}", response.body` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:79 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |

## P0234: create uploads an icon

Status: **closed**. Native identities:

- `controllers::accounts::icons::original_tests::original_acme_upload_list_and_audit_shortcodes`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:21` — `assert_difference "WorkspaceIcon.count", 1 do` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:50 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/accounts/icons_controller_test.rb:29` — `assert_redirected_to account_icons_url` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:45 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/accounts/icons_controller_test.rb:34` — `assert_equal "Acme Corp", icon.title` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:52 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/accounts/icons_controller_test.rb:35` — `assert_equal users(:david), icon.creator` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:52 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/accounts/icons_controller_test.rb:36` — `assert icon.image.attached?` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:53 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |

## P0088: workspace icon create and destroy are recorded

Status: **closed**. Native identities:

- `controllers::accounts::icons::original_tests::original_acme_upload_list_and_audit_shortcodes`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:209` — `assert_difference -> { AuditLog.where(action: "workspace_icon.create").count }, +1 do` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:55 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/audit_log/rooms_audit_test.rb:220` — `assert_equal icon.id, create.target_id` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:55 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/audit_log/rooms_audit_test.rb:221` — `assert_equal "WorkspaceIcon", create.target_type` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:55 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/audit_log/rooms_audit_test.rb:222` — `assert_equal ":acme:", create.target_label` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:55 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/audit_log/rooms_audit_test.rb:224` — `assert_difference -> { AuditLog.where(action: "workspace_icon.destroy").count }, +1 do` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:92 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/audit_log/rooms_audit_test.rb:229` — `assert_equal icon.id, destroy.target_id` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:92 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |
| `test/controllers/audit_log/rooms_audit_test.rb:230` — `assert_equal ":acme:", destroy.target_label` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:92 | Actual saved acme icon, attachment, David creator, exact Acme Corp title; list img[src=/icons/acme], code :acme:, title/uploader text; separate +1 create/destroy audit counts, exact type/id and :acme: labels. Cases: acme create/index/remove. |

## P0238: members get forbidden on list create and delete

Status: **closed**. Native identities:

- `controllers::accounts::icons::original_tests::original_jz_cannot_list_create_or_remove_the_existing_icon`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:88` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:106 | Actual JZ requests all return 403 and the original icon remains in the reloaded ordered collection. Cases: JZ list/create/remove. |
| `test/controllers/accounts/icons_controller_test.rb:96` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:110 | Actual JZ requests all return 403 and the original icon remains in the reloaded ordered collection. Cases: JZ list/create/remove. |
| `test/controllers/accounts/icons_controller_test.rb:99` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:128 | Actual JZ requests all return 403 and the original icon remains in the reloaded ordered collection. Cases: JZ list/create/remove. |
| `test/controllers/accounts/icons_controller_test.rb:101` — `assert WorkspaceIcon.exists?(icon.id)` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:135 | Actual JZ requests all return 403 and the original icon remains in the reloaded ordered collection. Cases: JZ list/create/remove. |

## P0235: create renders validation errors inline

Status: **closed**. Native identities:

- `controllers::accounts::icons::tests::invalid_and_duplicate_uploads_return_inline_errors_without_rows_or_audits`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:40` — `assert_no_difference "WorkspaceIcon.count" do` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:221 | Actual original openai/blank title/script.svg request: no row-count change, 422, and original inline built-in-name error. Cases: original upload/validation/delete. |
| `test/controllers/accounts/icons_controller_test.rb:48` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:222 | Actual original openai/blank title/script.svg request: no row-count change, 422, and original inline built-in-name error. Cases: original upload/validation/delete. |
| `test/controllers/accounts/icons_controller_test.rb:51` — `assert_match "already taken by a built-in icon", response.body` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:223 | Actual original openai/blank title/script.svg request: no row-count change, 422, and original inline built-in-name error. Cases: original upload/validation/delete. |

## P0236: create reports a name that raced past validation as taken

Status: **closed**. Native identities:

- `controllers::accounts::icons::tests::uniqueness_index_races_render_taken_and_roll_back_upload_and_audit`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:57` — `assert_no_difference "WorkspaceIcon.count" do` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:420 | Real SQLite uniqueness race after validation instead of Ruby any-instance mock: 422, inline taken error, zero saved rows/audits. Cases: original upload/validation/delete. |
| `test/controllers/accounts/icons_controller_test.rb:65` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:416 | Real SQLite uniqueness race after validation instead of Ruby any-instance mock: 422, inline taken error, zero saved rows/audits. Cases: original upload/validation/delete. |
| `test/controllers/accounts/icons_controller_test.rb:68` — `assert_match "has already been taken", response.body` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:417 | Real SQLite uniqueness race after validation instead of Ruby any-instance mock: 422, inline taken error, zero saved rows/audits. Cases: original upload/validation/delete. |

## P0237: destroy removes the icon and its blob

Status: **closed**. Native identities:

- `controllers::accounts::icons::tests::destroy_removes_attachment_and_blob_and_records_one_snapshot_audit`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:75` — `assert_difference [ "WorkspaceIcon.count", "ActiveStorage::Blob.count" ], -1 do` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:320 | After actual DELETE and actual PurgeJob, global WorkspaceIcon and Blob counts both decrease by one; exact redirect/status and target blob absence are checked. Cases: original upload/validation/delete. |
| `test/controllers/accounts/icons_controller_test.rb:78` — `assert_redirected_to account_icons_url` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:297 | After actual DELETE and actual PurgeJob, global WorkspaceIcon and Blob counts both decrease by one; exact redirect/status and target blob absence are checked. Cases: original upload/validation/delete. |

## P0265: serves an SVG with the documented headers

Status: **closed**. Native identities:

- `controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:13` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:332 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:14` — `assert_equal "image/svg+xml", response.media_type` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:334 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:15` — `assert_equal "inline", response.headers["Content-Disposition"].split(";").first` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:340 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:16` — `assert_equal "nosniff", response.headers["X-Content-Type-Options"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:339 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:17` — `assert_equal "default-src 'none'; style-src 'unsafe-inline'", response.headers["Content-Security-Policy"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:345 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:18` — `assert_equal "max-age=3600, private", response.headers["Cache-Control"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:335 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:19` — `assert_equal %("#{@icon.image.blob.checksum}"), response.headers["ETag"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:343 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:20` — `assert_equal @icon.image.blob.download, response.body` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:333 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |

## P0266: serves a PNG without the SVG-only headers

Status: **closed**. Native identities:

- `controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:29` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:332 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:30` — `assert_equal "image/png", response.media_type` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:334 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:31` — `assert_equal "max-age=3600, private", response.headers["Cache-Control"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:335 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:32` — `assert_equal "nosniff", response.headers["X-Content-Type-Options"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:339 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:34` — `assert_includes response.headers["Content-Security-Policy"].to_s, "script-src 'self'"` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:350 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |

## P0267: supports conditional GETs with the blob checksum

Status: **closed**. Native identities:

- `controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:42` — `assert_response :not_modified` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:359 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |
| `test/controllers/workspace_icons_controller_test.rb:43` — `assert_empty response.body` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:360 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |

## P0268: returns not found for unknown names

Status: **closed**. Native identities:

- `controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:51` — `assert_response :not_found` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:366 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |

## P0269: returns not found for signed-out users

Status: **closed**. Native identities:

- `controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:57` — `assert_response :not_found` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:361 | Actual JZ media responses: exact bytes/MIME, inline disposition, nosniff, SVG CSP or app PNG CSP, private max-age, quoted stored blob checksum, conditional 304/empty body, and signed-out/unknown 404. Cases: acme SVG, pixel PNG, conditional, unknown, anonymous. |

## P0270: unenrolled sessions are sent to setup instead of served the icon

Status: **closed**. Native identities:

- `controllers::accounts::icons::original_tests::original_password_session_enrollment_and_stale_icon_access`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:65` — `assert_redirected_to two_factor_setup_url` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:185 | Actual password-created JZ session: icon redirects to setup while unenrolled; after enabling enrollment it redirects to sign-in and the exact recorded session token no longer exists. Cases: JZ real password sign-in/unenrolled/stale. |

## P0271: stale enrolled sessions are signed out instead of served the icon

Status: **closed**. Native identities:

- `controllers::accounts::icons::original_tests::original_password_session_enrollment_and_stale_icon_access`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:75` — `assert_redirected_to new_session_url` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:203 | Actual password-created JZ session: icon redirects to setup while unenrolled; after enabling enrollment it redirects to sign-in and the exact recorded session token no longer exists. Cases: JZ real password sign-in/unenrolled/stale. |
| `test/controllers/workspace_icons_controller_test.rb:76` — `assert_nil Session.find_by(token: token)` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:204 | Actual password-created JZ session: icon redirects to setup while unenrolled; after enabling enrollment it redirects to sign-in and the exact recorded session token no longer exists. Cases: JZ real password sign-in/unenrolled/stale. |

## P0257: show stock

Status: **closed**. Native identities:

- `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:11` — `assert_valid_png_response size: 512` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:60; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:75, rust/crates/campfire/src/controllers/accounts/logos/tests.rs:80 | Actual routed stock/custom/fallback PNG complete bytes and content type; explicit PNG signature, 512/192 width and height assertions expand assert_valid_png_response. Cases: nil large, nil small, moon.jpg large, moon.jpg small, pixel.bmp large. |
| `test/controllers/accounts/logos_controller_test.rb:50` — `assert_equal @response.headers["content-type"], "image/png"` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:55 | The calling original case executes this routed-response assertion. |
| `test/controllers/accounts/logos_controller_test.rb:53` — `assert_equal size, image.width` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:72 | The calling original case executes this routed-response assertion. |
| `test/controllers/accounts/logos_controller_test.rb:54` — `assert_equal size, image.height` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:77 | The calling original case executes this routed-response assertion. |

## P0258: show stock small size

Status: **closed**. Native identities:

- `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:16` — `assert_valid_png_response size: 192` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:60; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:75, rust/crates/campfire/src/controllers/accounts/logos/tests.rs:80 | Actual routed stock/custom/fallback PNG complete bytes and content type; explicit PNG signature, 512/192 width and height assertions expand assert_valid_png_response. Cases: nil large, nil small, moon.jpg large, moon.jpg small, pixel.bmp large. |
| `test/controllers/accounts/logos_controller_test.rb:50` — `assert_equal @response.headers["content-type"], "image/png"` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:55 | The calling original case executes this routed-response assertion. |
| `test/controllers/accounts/logos_controller_test.rb:53` — `assert_equal size, image.width` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:72 | The calling original case executes this routed-response assertion. |
| `test/controllers/accounts/logos_controller_test.rb:54` — `assert_equal size, image.height` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:77 | The calling original case executes this routed-response assertion. |

## P0259: show custom

Status: **closed**. Native identities:

- `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:23` — `assert_valid_png_response size: 512` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:60; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:75, rust/crates/campfire/src/controllers/accounts/logos/tests.rs:80 | Actual routed stock/custom/fallback PNG complete bytes and content type; explicit PNG signature, 512/192 width and height assertions expand assert_valid_png_response. Cases: nil large, nil small, moon.jpg large, moon.jpg small, pixel.bmp large. |
| `test/controllers/accounts/logos_controller_test.rb:50` — `assert_equal @response.headers["content-type"], "image/png"` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:55 | The calling original case executes this routed-response assertion. |
| `test/controllers/accounts/logos_controller_test.rb:53` — `assert_equal size, image.width` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:72 | The calling original case executes this routed-response assertion. |
| `test/controllers/accounts/logos_controller_test.rb:54` — `assert_equal size, image.height` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:77 | The calling original case executes this routed-response assertion. |

## P0260: show custom small size

Status: **closed**. Native identities:

- `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:30` — `assert_valid_png_response size: 192` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:60; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:75, rust/crates/campfire/src/controllers/accounts/logos/tests.rs:80 | Actual routed stock/custom/fallback PNG complete bytes and content type; explicit PNG signature, 512/192 width and height assertions expand assert_valid_png_response. Cases: nil large, nil small, moon.jpg large, moon.jpg small, pixel.bmp large. |
| `test/controllers/accounts/logos_controller_test.rb:50` — `assert_equal @response.headers["content-type"], "image/png"` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:55 | The calling original case executes this routed-response assertion. |
| `test/controllers/accounts/logos_controller_test.rb:53` — `assert_equal size, image.width` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:72 | The calling original case executes this routed-response assertion. |
| `test/controllers/accounts/logos_controller_test.rb:54` — `assert_equal size, image.height` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:77 | The calling original case executes this routed-response assertion. |

## P0261: show stock when custom logo cannot be resized

Status: **closed**. Native identities:

- `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:37` — `assert_valid_png_response size: 512` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:60; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:75, rust/crates/campfire/src/controllers/accounts/logos/tests.rs:80 | Actual routed stock/custom/fallback PNG complete bytes and content type; explicit PNG signature, 512/192 width and height assertions expand assert_valid_png_response. Cases: nil large, nil small, moon.jpg large, moon.jpg small, pixel.bmp large. |
| `test/controllers/accounts/logos_controller_test.rb:50` — `assert_equal @response.headers["content-type"], "image/png"` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:55 | The calling original case executes this routed-response assertion. |
| `test/controllers/accounts/logos_controller_test.rb:53` — `assert_equal size, image.width` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:72 | The calling original case executes this routed-response assertion. |
| `test/controllers/accounts/logos_controller_test.rb:54` — `assert_equal size, image.height` (private helper) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:77 | The calling original case executes this routed-response assertion. |

## P0262: destroy

Status: **closed**. Native identities:

- `controllers::accounts::logos::tests::logo_upload_replacement_deletion_audits_and_cache_validation_match_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:44` — `assert_redirected_to edit_account_url` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:135 | Actual delete 302 Location; attachment gone; exactly one new audit, with original logo before=true/after=false details. Cases: uploaded logo/delete. |
| `test/controllers/accounts/logos_controller_test.rb:45` — `assert_not accounts(:signal).reload.logo.attached?` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:142 | Actual delete 302 Location; attachment gone; exactly one new audit, with original logo before=true/after=false details. Cases: uploaded logo/delete. |

## P0085: logo removal is recorded

Status: **closed**. Native identities:

- `controllers::accounts::logos::tests::logo_upload_replacement_deletion_audits_and_cache_validation_match_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:177` — `assert_difference -> { AuditLog.where(action: "account.settings.change").count }, +1 do` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:145 | Actual delete 302 Location; attachment gone; exactly one new audit, with original logo before=true/after=false details. Cases: uploaded logo/delete. |
| `test/controllers/audit_log/rooms_audit_test.rb:181` — `assert_equal({ "before" => true, "after" => false }, AuditLog.where(action: "account.settings.change").last.details["logo"])` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:144 | Actual delete 302 Location; attachment gone; exactly one new audit, with original logo before=true/after=false details. Cases: uploaded logo/delete. |

## P0072: room create is recorded

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_open_creation_actor_target_label_and_details`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:10` — `assert_difference -> { AuditLog.where(action: "room.create").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: create. |
| `test/controllers/audit_log/rooms_audit_test.rb:15` — `assert_equal users(:david).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: create. |
| `test/controllers/audit_log/rooms_audit_test.rb:16` — `assert_equal Rooms::Open.last.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: create. |
| `test/controllers/audit_log/rooms_audit_test.rb:17` — `assert_equal "Room", entry.target_type` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: create. |
| `test/controllers/audit_log/rooms_audit_test.rb:18` — `assert_equal "Audited Room", entry.target_label` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: create. |
| `test/controllers/audit_log/rooms_audit_test.rb:19` — `assert_equal({ "name" => "Audited Room" }, entry.details)` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: create. |

## P0073: failed room create writes no row

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_refused_closed_creation_has_no_audit`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:23` — `assert_no_difference -> { AuditLog.where(action: "room.create").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: failed. |
| `test/controllers/audit_log/rooms_audit_test.rb:27` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: failed. |

## P0074: opening an existing direct room writes no row

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_group_creation_then_reopening_has_one_audit`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:33` — `assert_difference -> { AuditLog.where(action: "room.create").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: direct_reuse. |
| `test/controllers/audit_log/rooms_audit_test.rb:37` — `assert_no_difference -> { AuditLog.where(action: "room.create").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: direct_reuse. |

## P0075: room destroy is recorded with the room name

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_room_destroy_label_and_actor`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:45` — `assert_difference -> { AuditLog.where(action: "room.destroy").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: destroy. |
| `test/controllers/audit_log/rooms_audit_test.rb:50` — `assert_equal users(:david).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: destroy. |
| `test/controllers/audit_log/rooms_audit_test.rb:51` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: destroy. |
| `test/controllers/audit_log/rooms_audit_test.rb:52` — `assert_equal "Doomed", entry.target_label` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: destroy. |

## P0076: admin membership changes are recorded with names

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_membership_granted_and_revoked_names`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:62` — `assert_difference -> { AuditLog.where(action: "room.membership.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: membership. |
| `test/controllers/audit_log/rooms_audit_test.rb:69` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: membership. |
| `test/controllers/audit_log/rooms_audit_test.rb:70` — `assert_equal [ add.name ], entry.details["granted"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: membership. |
| `test/controllers/audit_log/rooms_audit_test.rb:71` — `assert_equal [ remove.name ], entry.details["revoked"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: membership. |

## P0077: re-saving unchanged membership writes no row

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_unchanged_membership_has_no_audit`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:77` — `assert_no_difference -> { AuditLog.where(action: "room.membership.change").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: unchanged. |

## P0078: adding group members is recorded with the added users

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_group_add_jz_has_actor_and_granted_name`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:85` — `assert_difference -> { AuditLog.where(action: "room.membership.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: add. |
| `test/controllers/audit_log/rooms_audit_test.rb:90` — `assert_equal users(:david).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: add. |
| `test/controllers/audit_log/rooms_audit_test.rb:91` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: add. |
| `test/controllers/audit_log/rooms_audit_test.rb:92` — `assert_equal [ "JZ" ], entry.details["granted"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: add. |

## P0079: adding no new members writes no row

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_group_add_existing_jason_has_no_audit`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:98` — `assert_no_difference -> { AuditLog.where(action: "room.membership.change").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: add_none. |

## P0080: the last member out destroys the group and records it

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_last_leaver_kevin_destroys_weekend_plans`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:111` — `assert_difference -> { AuditLog.where(action: "room.destroy").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: last_leave. |
| `test/controllers/audit_log/rooms_audit_test.rb:116` — `assert_equal users(:kevin).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: last_leave. |
| `test/controllers/audit_log/rooms_audit_test.rb:117` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: last_leave. |
| `test/controllers/audit_log/rooms_audit_test.rb:118` — `assert_equal "Weekend Plans", entry.target_label` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: last_leave. |

## P0081: leaving without destroying the group writes no row

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_other_leaver_has_no_destroy_audit`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:124` — `assert_no_difference -> { AuditLog.where(action: "room.destroy").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: leave_no_destroy. |

## P0082: leaving a channel is recorded with the leaver as actor

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_jz_channel_leave_actor_label_and_revocation`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:133` — `assert_difference -> { AuditLog.where(action: "room.membership.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: leave_channel. |
| `test/controllers/audit_log/rooms_audit_test.rb:138` — `assert_equal users(:jz).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: leave_channel. |
| `test/controllers/audit_log/rooms_audit_test.rb:139` — `assert_equal "JZ <jz@37signals.com>", entry.actor_label` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: leave_channel. |
| `test/controllers/audit_log/rooms_audit_test.rb:140` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: leave_channel. |
| `test/controllers/audit_log/rooms_audit_test.rb:141` — `assert_equal [ "JZ" ], entry.details["revoked"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: leave_channel. |

## P0083: leaving a group without destroying it is recorded

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_group_leave_david_actor_and_revocation`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:147` — `assert_difference -> { AuditLog.where(action: "room.membership.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: leave_group. |
| `test/controllers/audit_log/rooms_audit_test.rb:152` — `assert_equal users(:david).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: leave_group. |
| `test/controllers/audit_log/rooms_audit_test.rb:153` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: leave_group. |
| `test/controllers/audit_log/rooms_audit_test.rb:154` — `assert_equal [ "David" ], entry.details["revoked"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: leave_group. |

## P0084: account settings changes are recorded

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_combined_account_settings_audit_fields`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:158` — `assert_difference -> { AuditLog.where(action: "account.settings.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: settings. |
| `test/controllers/audit_log/rooms_audit_test.rb:168` — `assert_equal Current.account.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: settings. |
| `test/controllers/audit_log/rooms_audit_test.rb:169` — `assert_equal "Account", entry.target_type` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: settings. |
| `test/controllers/audit_log/rooms_audit_test.rb:170` — `assert entry.details.key?("name")` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: settings. |
| `test/controllers/audit_log/rooms_audit_test.rb:171` — `assert entry.details.key?("restrict_room_creation_to_administrators")` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual per-request audit count delta and every reloaded row field: actor ID/label, target ID/type/label, granted/revoked names and complete JSON details. Repeated requests and each leave step are compared independently. Cases: settings. |

## P0086: custom styles changes are recorded as size and digest

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_styles_audit_sizes_and_digests`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:187` — `assert_difference -> { AuditLog.where(action: "account.custom_styles.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual audit count delta and complete details including exact UTF-8 sizes and 12-character SHA256 prefixes; no raw style text exists in the matched details. No-op has zero new rows. Cases: styles. |
| `test/controllers/audit_log/rooms_audit_test.rb:193` — `assert_equal previous.to_s.bytesize, pair["before"]["size"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual audit count delta and complete details including exact UTF-8 sizes and 12-character SHA256 prefixes; no raw style text exists in the matched details. No-op has zero new rows. Cases: styles. |
| `test/controllers/audit_log/rooms_audit_test.rb:194` — `assert_equal "body { color: red; }".bytesize, pair["after"]["size"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual audit count delta and complete details including exact UTF-8 sizes and 12-character SHA256 prefixes; no raw style text exists in the matched details. No-op has zero new rows. Cases: styles. |
| `test/controllers/audit_log/rooms_audit_test.rb:195` — `assert_equal Digest::SHA256.hexdigest(previous.to_s)[0, 12], pair["before"]["digest"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual audit count delta and complete details including exact UTF-8 sizes and 12-character SHA256 prefixes; no raw style text exists in the matched details. No-op has zero new rows. Cases: styles. |
| `test/controllers/audit_log/rooms_audit_test.rb:196` — `assert_equal Digest::SHA256.hexdigest("body { color: red; }")[0, 12], pair["after"]["digest"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual audit count delta and complete details including exact UTF-8 sizes and 12-character SHA256 prefixes; no raw style text exists in the matched details. No-op has zero new rows. Cases: styles. |
| `test/controllers/audit_log/rooms_audit_test.rb:197` — `assert_no_match "color: red", entry.details.to_json` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual audit count delta and complete details including exact UTF-8 sizes and 12-character SHA256 prefixes; no raw style text exists in the matched details. No-op has zero new rows. Cases: styles. |

## P0087: unchanged custom styles write no row

Status: **closed**. Native identities:

- `controllers::rooms::original_audit_tests::original_same_styles_have_no_audit`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:203` — `assert_no_difference -> { AuditLog.where(action: "account.custom_styles.change").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 | Actual audit count delta and complete details including exact UTF-8 sizes and 12-character SHA256 prefixes; no raw style text exists in the matched details. No-op has zero new rows. Cases: styles_same. |

## P0278: create

Status: **closed**. Native identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_create`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:12` — `assert_response :success` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:109 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_create. |
| `test/controllers/unfurl_links_controller_test.rb:15` — `assert_equal "Hey!", json_response["title"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:111 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_create. |
| `test/controllers/unfurl_links_controller_test.rb:16` — `assert_equal "https://example.com", json_response["url"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:112 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_create. |
| `test/controllers/unfurl_links_controller_test.rb:17` — `assert_equal "https://example.com/image.png", json_response["image"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:113 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_create. |
| `test/controllers/unfurl_links_controller_test.rb:18` — `assert_equal "desc..", json_response["description"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:114 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_create. |

## P0279: create strips markup from the title and description

Status: **closed**. Native identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_encoded_markup`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:26` — `assert_response :success` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:123 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_encoded_markup. |
| `test/controllers/unfurl_links_controller_test.rb:29` — `assert_equal "Hey!", json_response["title"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:125 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_encoded_markup. |
| `test/controllers/unfurl_links_controller_test.rb:30` — `assert_equal "desc..", json_response["description"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:126 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_encoded_markup. |

## P0280: create with missing opengraph meta tags

Status: **closed**. Native identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_missing_tags`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:37` — `assert_response :no_content` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:133 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_missing_tags. |

## P0281: create returns no content when the title and description are only a markup tag

Status: **closed**. Native identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_only_markup`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:52` — `assert_response :no_content` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:141 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_only_markup. |

## P0282: create for a GitHub PR URL returns no content (PR cards render instead)

Status: **closed**. Native identities:

- `controllers::unfurl_links::rails_tests::original_github_pr_has_no_unfurl`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:57` — `assert_response :no_content` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:183 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: original_github_pr_has_no_unfurl. |

## P0283: create for a Fizzy card URL returns no content (Fizzy cards render instead)

Status: **closed**. Native identities:

- `controllers::unfurl_links::rails_tests::original_fizzy_card_has_no_unfurl`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:62` — `assert_response :no_content` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:183 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: original_fizzy_card_has_no_unfurl. |

## P0284: create with a missing URL

Status: **closed**. Native identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_missing_url`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:66` — `assert_raise ActionController::ParameterMissing do` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:149 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_missing_url. |
| `test/controllers/unfurl_links_controller_test.rb:68` — `assert_response :bad_request` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:148 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_missing_url. |

## P0285: create for twitter.com

Status: **closed**. Native identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_twitter`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:76` — `assert_response :success` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:164 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_twitter. |
| `test/controllers/unfurl_links_controller_test.rb:77` — `assert_equal "Hey!", JSON.parse(response.body)["title"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:165 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_twitter. |

## P0286: create for x.com

Status: **closed**. Native identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_x`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:84` — `assert_response :success` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:164 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_x. |
| `test/controllers/unfurl_links_controller_test.rb:85` — `assert_equal "Hey!", JSON.parse(response.body)["title"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:165 | Actual registered action with held ephemeral TLS origin and resolver: exact original title/description/URL/image, no-content responses, special-card skip, and fxtwitter request. Missing URL captures actual ParameterMissing(url) before the adapter maps it to 400. Cases: ws15e_rails_composer_x. |

## P0203: show initials

Status: **closed**. Native identities:

- `controllers::users::avatars::original_tests::original_kevin_initials_text`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/avatars_controller_test.rb:10` — `assert_select "text", text: "K"` | rust/crates/campfire/src/controllers/users/avatars.rs:182 | Actual signed-token HTTP response: full original Kevin K SVG or exact invalid-token 404. Cases: initials. |

## P0206: show image with invalid token responds 404

Status: **closed**. Native identities:

- `controllers::users::avatars::original_tests::original_invalid_avatar_token_is_not_found`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/avatars_controller_test.rb:32` — `assert_response :not_found` | rust/crates/campfire/src/controllers/users/avatars.rs:181 | Actual signed-token HTTP response: full original Kevin K SVG or exact invalid-token 404. Cases: invalid. |

## P0102: configured installation names the operator and contact

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:171` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:194 | Actual complete public response bytes for original Acme Widgets and privacy@example.com configuration on about/privacy/terms. Cases: original_configured. |
| `test/controllers/public_pages_controller_test.rb:173` — `assert_match(/Acme Widgets/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:197 | Actual complete public response bytes for original Acme Widgets and privacy@example.com configuration on about/privacy/terms. Cases: original_configured. |
| `test/controllers/public_pages_controller_test.rb:174` — `assert_select 'a[href="mailto:privacy@example.com"]'` | rust/crates/campfire/src/controllers/public_pages.rs:197 | Actual complete public response bytes for original Acme Widgets and privacy@example.com configuration on about/privacy/terms. Cases: original_configured. |
| `test/controllers/public_pages_controller_test.rb:177` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:194 | Actual complete public response bytes for original Acme Widgets and privacy@example.com configuration on about/privacy/terms. Cases: original_configured. |
| `test/controllers/public_pages_controller_test.rb:178` — `assert_match(/Acme Widgets/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:197 | Actual complete public response bytes for original Acme Widgets and privacy@example.com configuration on about/privacy/terms. Cases: original_configured. |

## P0176: linking a github login strips and downcases it

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::original_github_login_normalizes_david_gh`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:307` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:237 | Actual request status/Location and reloaded github_login: David-GH becomes david-gh; clearing starts linked and produces null. Cases: original_github_normalize. |
| `test/controllers/users/profiles_controller_test.rb:308` — `assert_equal "david-gh", users(:david).reload.github_login` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:256 | Actual request status/Location and reloaded github_login: David-GH becomes david-gh; clearing starts linked and produces null. Cases: original_github_normalize. |

## P0186: clearing a github login unlinks it

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::original_linked_github_login_is_cleared`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:404` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:237 | Actual request status/Location and reloaded github_login: David-GH becomes david-gh; clearing starts linked and produces null. Cases: original_github_unlink. |
| `test/controllers/users/profiles_controller_test.rb:405` — `assert_nil users(:david).reload.github_login` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:256 | Actual request status/Location and reloaded github_login: David-GH becomes david-gh; clearing starts linked and produces null. Cases: original_github_unlink. |

## P0154: profile shows the meeting fetch notice when a refresh failed

Status: **closed**. Native identities:

- `controllers::users::profile_gap_original_tests::original_meeting_unreachable_notice`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:82` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 | Actual HTTP status, exact escaped Calendar UNREACHABLE_MESSAGE or Connected as david@gmail.test and Disconnect; real PUT saves exact david@smartdata.net/now or Dave/null marker, with original current-password rules. Cases: meeting_error. |
| `test/controllers/users/profiles_controller_test.rb:83` — `assert_includes response.body, CGI.escapeHTML(Calendar::MeetingRefresh::UNREACHABLE_MESSAGE)` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 | Actual HTTP status, exact escaped Calendar UNREACHABLE_MESSAGE or Connected as david@gmail.test and Disconnect; real PUT saves exact david@smartdata.net/now or Dave/null marker, with original current-password rules. Cases: meeting_error. |

## P0165: profile shows the connected account with a disconnect button

Status: **closed**. Native identities:

- `controllers::users::profile_gap_original_tests::original_connected_david_gmail_email`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:195` — `assert_includes response.body, "Connected as david@gmail.test"` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 | Actual HTTP status, exact escaped Calendar UNREACHABLE_MESSAGE or Connected as david@gmail.test and Disconnect; real PUT saves exact david@smartdata.net/now or Dave/null marker, with original current-password rules. Cases: connected_email. |
| `test/controllers/users/profiles_controller_test.rb:196` — `assert_includes response.body, "Disconnect"` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 | Actual HTTP status, exact escaped Calendar UNREACHABLE_MESSAGE or Connected as david@gmail.test and Disconnect; real PUT saves exact david@smartdata.net/now or Dave/null marker, with original current-password rules. Cases: connected_email. |

## P0190: changing email with the current password records a self-change

Status: **closed**. Native identities:

- `controllers::users::profile_gap_original_tests::original_email_change_with_current_password_marks_now`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:437` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 | Actual HTTP status, exact escaped Calendar UNREACHABLE_MESSAGE or Connected as david@gmail.test and Disconnect; real PUT saves exact david@smartdata.net/now or Dave/null marker, with original current-password rules. Cases: email_change. |
| `test/controllers/users/profiles_controller_test.rb:438` — `assert_equal "david@smartdata.net", users(:david).reload.email_address` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 | Actual HTTP status, exact escaped Calendar UNREACHABLE_MESSAGE or Connected as david@gmail.test and Disconnect; real PUT saves exact david@smartdata.net/now or Dave/null marker, with original current-password rules. Cases: email_change. |
| `test/controllers/users/profiles_controller_test.rb:439` — `assert_equal Time.current, users(:david).email_self_changed_at` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 | Actual HTTP status, exact escaped Calendar UNREACHABLE_MESSAGE or Connected as david@gmail.test and Disconnect; real PUT saves exact david@smartdata.net/now or Dave/null marker, with original current-password rules. Cases: email_change. |

## P0191: other profile edits and case-only email edits need no password and record nothing

Status: **closed**. Native identities:

- `controllers::users::profile_gap_original_tests::original_case_only_email_and_dave_name_leave_marker_null`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:446` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 | Actual HTTP status, exact escaped Calendar UNREACHABLE_MESSAGE or Connected as david@gmail.test and Disconnect; real PUT saves exact david@smartdata.net/now or Dave/null marker, with original current-password rules. Cases: email_case. |
| `test/controllers/users/profiles_controller_test.rb:447` — `assert_equal "Dave", users(:david).reload.name` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 | Actual HTTP status, exact escaped Calendar UNREACHABLE_MESSAGE or Connected as david@gmail.test and Disconnect; real PUT saves exact david@smartdata.net/now or Dave/null marker, with original current-password rules. Cases: email_case. |
| `test/controllers/users/profiles_controller_test.rb:448` — `assert_nil users(:david).email_self_changed_at` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 | Actual HTTP status, exact escaped Calendar UNREACHABLE_MESSAGE or Connected as david@gmail.test and Disconnect; real PUT saves exact david@smartdata.net/now or Dave/null marker, with original current-password rules. Cases: email_case. |

## S0054: channel row shows the live huddle stack with names and count

Status: **closed**. Native identities:

- `controllers::users::sidebar_original_tests::original_channel_live_stack_names_count_and_attributes`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:62` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: channel. |
| `test/controllers/users/sidebars_controller_test.rb:63` — `assert_select "##{dom_id(room, :list)} .voice-stack--live.voice-stack--huddle" do` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: channel. |
| `test/controllers/users/sidebars_controller_test.rb:64` — `assert_select ".voice-stack__count", text: "2"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: channel. |
| `test/controllers/users/sidebars_controller_test.rb:65` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:david).id}'][title='David']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: channel. |
| `test/controllers/users/sidebars_controller_test.rb:66` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:jason).id}'][title='Jason']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: channel. |
| `test/controllers/users/sidebars_controller_test.rb:68` — `assert_select "##{dom_id(room, :list)} .voice-stack" +` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: channel. |

## S0073: board row shows the live huddle stack with names and count

Status: **closed**. Native identities:

- `controllers::users::sidebar_original_tests::original_board_live_stack_names_count_and_attributes`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:81` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: board. |
| `test/controllers/users/sidebars_controller_test.rb:82` — `assert_select "##{dom_id(board, :list)} .voice-room__trailing .voice-stack--live.voice-stack--huddle" do` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: board. |
| `test/controllers/users/sidebars_controller_test.rb:83` — `assert_select ".voice-stack__count", text: "2"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: board. |
| `test/controllers/users/sidebars_controller_test.rb:84` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:david).id}'][title='David']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: board. |
| `test/controllers/users/sidebars_controller_test.rb:85` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:jz).id}'][title='JZ']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: board. |
| `test/controllers/users/sidebars_controller_test.rb:87` — `assert_select "##{dom_id(board, :list)} .voice-stack" +` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: board. |

## S0092: direct row shows the live huddle stack when the peer is in the call

Status: **closed**. Native identities:

- `controllers::users::sidebar_original_tests::original_direct_live_stack_peer_count_and_attributes`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:99` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:117 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: direct. |
| `test/controllers/users/sidebars_controller_test.rb:100` — `assert_select "##{dom_id(room, :list)} .voice-stack--live" do` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:117 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: direct. |
| `test/controllers/users/sidebars_controller_test.rb:101` — `assert_select ".voice-stack__count", text: "1"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:117 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: direct. |
| `test/controllers/users/sidebars_controller_test.rb:102` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:jason).id}']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:117 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: direct. |
| `test/controllers/users/sidebars_controller_test.rb:104` — `assert_select "##{dom_id(room, :list)} .voice-stack[aria-label='1 in huddle: Jason']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:117 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: direct. |

## S0106: quiet rows keep an empty stack target with no visible presence

Status: **closed**. Native identities:

- `controllers::users::sidebar_original_tests::original_quiet_rows_have_empty_hidden_stack_targets`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:110` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:123 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: quiet_channel, quiet_direct. |
| `test/controllers/users/sidebars_controller_test.rb:111` — `assert_select "##{dom_id(rooms(:watercooler), :list)} .voice-stack:not(.voice-stack--live)" +` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:123 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: quiet_channel, quiet_direct. |
| `test/controllers/users/sidebars_controller_test.rb:113` — `assert_select ".voice-stack__avatar", count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:123 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: quiet_channel, quiet_direct. |
| `test/controllers/users/sidebars_controller_test.rb:114` — `assert_select ".voice-stack__count[hidden]", text: "", visible: false` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:123 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: quiet_channel, quiet_direct. |
| `test/controllers/users/sidebars_controller_test.rb:116` — `assert_select "##{dom_id(rooms(:david_and_jason), :list)} .voice-stack:not(.voice-stack--live)" +` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:123 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: quiet_channel, quiet_direct. |

## S0119: direct row re-renders when a participant joins

Status: **closed**. Native identities:

- `controllers::users::sidebar_original_tests::original_cached_direct_rename_then_join_invalidates_only_on_participants`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:126` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: cache_before, cache_rename, cache_join. |
| `test/controllers/users/sidebars_controller_test.rb:127` — `assert_select "##{dom_id(room, :list)} .voice-stack:not(.voice-stack--live)"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: cache_before, cache_rename, cache_join. |
| `test/controllers/users/sidebars_controller_test.rb:128` — `assert_select "##{dom_id(room, :list)}", text: /Jason/` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: cache_before, cache_rename, cache_join. |
| `test/controllers/users/sidebars_controller_test.rb:134` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: cache_before, cache_rename, cache_join. |
| `test/controllers/users/sidebars_controller_test.rb:135` — `assert_select "##{dom_id(room, :list)}", text: /Jason/` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: cache_before, cache_rename, cache_join. |
| `test/controllers/users/sidebars_controller_test.rb:136` — `assert_select "##{dom_id(room, :list)}", text: /Jordan/, count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: cache_before, cache_rename, cache_join. |
| `test/controllers/users/sidebars_controller_test.rb:142` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: cache_before, cache_rename, cache_join. |
| `test/controllers/users/sidebars_controller_test.rb:143` — `assert_select "##{dom_id(room, :list)} .voice-stack--live .voice-stack__count", text: "1"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: cache_before, cache_rename, cache_join. |
| `test/controllers/users/sidebars_controller_test.rb:144` — `assert_select "##{dom_id(room, :list)}", text: /Jordan/` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: cache_before, cache_rename, cache_join. |

## S0147: group direct rooms render member names and a huddle stack

Status: **closed**. Native identities:

- `controllers::users::sidebar_original_tests::original_group_direct_name_and_live_stack`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:154` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:129 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: group. |
| `test/controllers/users/sidebars_controller_test.rb:155` — `assert_select "##{dom_id(group, :list)}", text: /Ping with Jason, Kevin/` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:129 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: group. |
| `test/controllers/users/sidebars_controller_test.rb:156` — `assert_select "##{dom_id(group, :list)} .voice-stack--live.voice-stack--huddle" do` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:129 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: group. |
| `test/controllers/users/sidebars_controller_test.rb:157` — `assert_select ".voice-stack__count", text: "1"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:129 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: group. |
| `test/controllers/users/sidebars_controller_test.rb:158` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:jason).id}'][title='Jason']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:129 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: group. |

## S0181: no channel or DM stacks without huddle configuration

Status: **closed**. Native identities:

- `controllers::users::sidebar_original_tests::original_unconfigured_huddle_has_no_stacks_or_presence_controllers`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:188` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:168; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:164 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: unconfigured. |
| `test/controllers/users/sidebars_controller_test.rb:189` — `assert_select "#shared_rooms .voice-stack", count: 0` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:177; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:164 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: unconfigured. |
| `test/controllers/users/sidebars_controller_test.rb:190` — `assert_select "#direct_rooms .voice-stack", count: 0` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:177; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:164 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: unconfigured. |
| `test/controllers/users/sidebars_controller_test.rb:191` — `assert_select "[data-controller~='huddle-presence']", count: 0` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:188; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:164 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: unconfigured. |

## S0193: sidebar query count does not grow with quiet channels, DMs, boards, and stages

Status: **closed**. Native identities:

- `controllers::users::sidebar_original_tests::original_mixed_quiet_sidebar_read_counts_stay_flat_with_one_grants_query`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:201` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:232; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:247 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: queries.baseline, queries.more. |
| `test/controllers/users/sidebars_controller_test.rb:209` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:232; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:247 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: queries.baseline, queries.more. |
| `test/controllers/users/sidebars_controller_test.rb:211` — `assert_equal baseline.count, with_more_rooms.count,` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:254; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:247 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: queries.baseline, queries.more. |
| `test/controllers/users/sidebars_controller_test.rb:213` — `assert_equal 1, with_more_rooms.count { &#124;sql&#124; sql.include?("FROM \"huddle_grants\"") }` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:259; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:247 | Actual complete scoped sidebar DOM preserves tags, text, all attributes and cardinality, comparing every original nested selector/count. Cached rename stays Jason until participant IDs change; unconfigured selectors are empty; quiet-room SELECT growth is flat and grants are queried once. Cases: queries.baseline, queries.more. |

## P0251: past the export cap the page warns and the CSV filename says truncated

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_caps_tests::original_two_row_cap_warns_names_and_limits_csv`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:163` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:64 | Real HTTP router and normal action; only the same per-request limit override as Rails' temporary constant replacement differs. HTML success and CSV success are separate checks. Exact newest-limit warning, filename truncation predicate, parsed CSV row count and live table count compare against the pinned Rails reduced-cap oracle. Cases: past. |
| `test/controllers/accounts/audit_logs_controller_test.rb:164` — `assert_match "newest 2", response.body` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 | Real HTTP router and normal action; only the same per-request limit override as Rails' temporary constant replacement differs. HTML success and CSV success are separate checks. Exact newest-limit warning, filename truncation predicate, parsed CSV row count and live table count compare against the pinned Rails reduced-cap oracle. Cases: past. |
| `test/controllers/accounts/audit_logs_controller_test.rb:167` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:66 | Real HTTP router and normal action; only the same per-request limit override as Rails' temporary constant replacement differs. HTML success and CSV success are separate checks. Exact newest-limit warning, filename truncation predicate, parsed CSV row count and live table count compare against the pinned Rails reduced-cap oracle. Cases: past. |
| `test/controllers/accounts/audit_logs_controller_test.rb:168` — `assert_match "truncated-to-2", response.headers["Content-Disposition"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 | Real HTTP router and normal action; only the same per-request limit override as Rails' temporary constant replacement differs. HTML success and CSV success are separate checks. Exact newest-limit warning, filename truncation predicate, parsed CSV row count and live table count compare against the pinned Rails reduced-cap oracle. Cases: past. |
| `test/controllers/accounts/audit_logs_controller_test.rb:169` — `assert_equal 2, CSV.parse(response.body, headers: true).length` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 | Real HTTP router and normal action; only the same per-request limit override as Rails' temporary constant replacement differs. HTML success and CSV success are separate checks. Exact newest-limit warning, filename truncation predicate, parsed CSV row count and live table count compare against the pinned Rails reduced-cap oracle. Cases: past. |

## P0252: within the export cap there is no truncation notice

Status: **closed**. Native identities:

- `controllers::accounts::audit_logs::original_caps_tests::original_five_row_cap_keeps_all_rows_without_warning`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:178` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:64 | Real HTTP router and normal action; only the same per-request limit override as Rails' temporary constant replacement differs. HTML success and CSV success are separate checks. Exact newest-limit warning, filename truncation predicate, parsed CSV row count and live table count compare against the pinned Rails reduced-cap oracle. Cases: within. |
| `test/controllers/accounts/audit_logs_controller_test.rb:179` — `assert_no_match "newest 5", response.body` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 | Real HTTP router and normal action; only the same per-request limit override as Rails' temporary constant replacement differs. HTML success and CSV success are separate checks. Exact newest-limit warning, filename truncation predicate, parsed CSV row count and live table count compare against the pinned Rails reduced-cap oracle. Cases: within. |
| `test/controllers/accounts/audit_logs_controller_test.rb:182` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:66 | Real HTTP router and normal action; only the same per-request limit override as Rails' temporary constant replacement differs. HTML success and CSV success are separate checks. Exact newest-limit warning, filename truncation predicate, parsed CSV row count and live table count compare against the pinned Rails reduced-cap oracle. Cases: within. |
| `test/controllers/accounts/audit_logs_controller_test.rb:183` — `assert_no_match "truncated", response.headers["Content-Disposition"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 | Real HTTP router and normal action; only the same per-request limit override as Rails' temporary constant replacement differs. HTML success and CSV success are separate checks. Exact newest-limit warning, filename truncation predicate, parsed CSV row count and live table count compare against the pinned Rails reduced-cap oracle. Cases: within. |
| `test/controllers/accounts/audit_logs_controller_test.rb:187` — `assert_equal AuditLog.count, CSV.parse(response.body, headers: true).length` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 | Real HTTP router and normal action; only the same per-request limit override as Rails' temporary constant replacement differs. HTML success and CSV success are separate checks. Exact newest-limit warning, filename truncation predicate, parsed CSV row count and live table count compare against the pinned Rails reduced-cap oracle. Cases: within. |

