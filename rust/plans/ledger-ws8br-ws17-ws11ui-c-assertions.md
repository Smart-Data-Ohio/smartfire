# Individual pinned original assertion audit

Current reference: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. Historical mapping: `dae59026` / `d7c7de92`, retained in JSON history.

Every original assertion is paired with the actual routed/browser predicate. Explicit `visible: :all` checks retain attached-node semantics; default selectors count rendered nodes.

## P0239: admins can browse the log

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_admin_browse_labels_and_export_link`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:17` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:18` — `assert_select "h1", text: "Audit log"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:19` — `assert_select "tbody td code", text: "user.ban"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194; rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:43 |
| `test/controllers/accounts/audit_logs_controller_test.rb:20` — `assert_select "tbody td code", text: "user.role.change"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194; rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:43 |
| `test/controllers/accounts/audit_logs_controller_test.rb:21` — `assert_select "tbody td", text: "Kevin <kevin@37signals.com>"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:22` — `assert_select "a[href='#{account_audit_log_path(format: :csv)}']", text: "Export CSV"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0240: members are forbidden

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_members_forbidden`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:29` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0241: visitors are sent to sign in

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_visitors_redirect_to_sign_in`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:35` — `assert_redirected_to new_session_url` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0242: visitors cannot export CSV

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_visitors_cannot_export`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:41` — `assert_redirected_to new_session_url` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0243: members cannot export CSV

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_members_cannot_export`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:48` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0244: filtering by actor matches names and emails in labels

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_actor_filter_matches_label_email`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:57` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:58` — `assert_select "tbody tr", count: 1` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:59` — `assert_select "tbody td", text: "Jason <jason@37signals.com>"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0245: filtering by action and target type

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_action_and_target_type_filters`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:68` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:69` — `assert_select "tbody tr", count: 1` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:70` — `assert_select "tbody td code", text: "room.create"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194; rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:43 |
| `test/controllers/accounts/audit_logs_controller_test.rb:74` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:75` — `assert_select "tbody tr", count: 0` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0246: unknown filter values are ignored

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_unknown_filters_are_ignored`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:83` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:84` — `assert_select "tbody td code", text: "user.ban"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194; rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:43 |
| `test/controllers/accounts/audit_logs_controller_test.rb:85` — `assert_select "tbody td code", text: "user.role.change"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194; rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:43 |

## P0247: filtering by date range

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_date_range_excludes_the_other_action`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:94` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:95` — `assert_select "tbody td code", text: "user.ban", count: 0` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194; rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:43 |
| `test/controllers/accounts/audit_logs_controller_test.rb:96` — `assert_select "tbody td code", text: "user.role.change"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194; rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:43 |
| `test/controllers/accounts/audit_logs_controller_test.rb:100` — `assert_select "tbody td code", text: "user.ban"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194; rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:43 |
| `test/controllers/accounts/audit_logs_controller_test.rb:101` — `assert_select "tbody td code", text: "user.role.change", count: 0` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194; rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:43 |

## P0248: paging walks older entries

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_paging_links_and_row_count`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:113` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:114` — `assert_match "Older entries", response.body` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:115` — `assert_select "tbody tr", count: Accounts::AuditLogsController::PAGE_SIZE` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:119` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:120` — `assert_match "Newest entries", response.body` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0249: CSV export carries headers and the filtered rows

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_filtered_csv_headers_labels_changes_and_ip`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:129` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:130` — `assert_equal "text/csv", response.media_type` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:132` — `assert_equal %w[ time action actor target_type target changes ip_address user_agent ], rows.headers` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:133` — `assert_equal 1, rows.length` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:134` — `assert_equal "user.ban", rows[0]["action"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:135` — `assert_equal "David <david@37signals.com>", rows[0]["actor"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:136` — `assert_equal "User", rows[0]["target_type"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:137` — `assert_equal "Kevin <kevin@37signals.com>", rows[0]["target"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:138` — `assert_equal "203.0.113.7", rows[0]["ip_address"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:139` — `assert_equal({ "status" => %w[ active banned ] }, JSON.parse(rows[0]["changes"]))` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0250: CSV export neutralizes formula injection

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_formula_target_is_quoted`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:153` — `assert targets.any? { &#124;target&#124; target.start_with?("'=") }, "expected a quoted formula cell in #{targets.inspect}"` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0253: CSV export neutralizes formula injection in request columns

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_tests::original_formula_request_column_is_quoted`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:202` — `assert_equal 1, rows.length` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |
| `test/controllers/accounts/audit_logs_controller_test.rb:203` — `assert_equal "'=cmd&#124;'/c calc'!A0", rows[0]["user_agent"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_tests.rs:194 |

## P0230: update

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_admin_self_role_is_preserved`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/users_controller_test.rb:10` — `assert users(:david).administrator?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/accounts/users_controller_test.rb:14` — `assert_redirected_to edit_account_url` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/accounts/users_controller_test.rb:15` — `assert users(:david).reload.administrator?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0231: destroy

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_self_removal_active_count_and_lookup`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/users_controller_test.rb:19` — `assert_difference -> { User.active.count }, -1 do` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/accounts/users_controller_test.rb:23` — `assert_redirected_to edit_account_url` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/accounts/users_controller_test.rb:24` — `assert_nil User.active.find_by(id: users(:david).id)` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0232: non-admins cannot perform actions

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_member_cannot_change_admin_role`
- `controllers::accounts::original_control_tests::original_member_cannot_remove_admin`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/users_controller_test.rb:31` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/accounts/users_controller_test.rb:34` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0254: edit

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_styles_edit_success`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/custom_styles_controller_test.rb:11` — `assert_response :ok` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0255: update

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_styles_exact_value_is_saved`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/custom_styles_controller_test.rb:15` — `assert users(:david).administrator?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/accounts/custom_styles_controller_test.rb:19` — `assert_redirected_to edit_account_custom_styles_url` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/accounts/custom_styles_controller_test.rb:20` — `assert_equal accounts(:signal).custom_styles, ":root { --color-text: red; }"` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0256: non-admins cannot update

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_member_styles_refused`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/custom_styles_controller_test.rb:25` — `assert users(:kevin).member?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/accounts/custom_styles_controller_test.rb:28` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0263: create new join code

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_join_code_changes_and_redirects`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/join_codes_controller_test.rb:10` — `assert_changes -> { accounts(:signal).reload.join_code } do` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/accounts/join_codes_controller_test.rb:12` — `assert_redirected_to edit_account_url` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0264: only administrators can create new join codes

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_jz_cannot_reset_join_code`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/join_codes_controller_test.rb:19` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0214: create bans user and creates ban records from sessions

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_ban_has_two_exact_ip_records`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/bans_controller_test.rb:14` — `assert_difference -> { Ban.count }, 2 do` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/users/bans_controller_test.rb:18` — `assert_redirected_to user_url(user)` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/users/bans_controller_test.rb:19` — `assert Ban.exists?(ip_address: "203.0.113.1", user: user)` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/users/bans_controller_test.rb:20` — `assert Ban.exists?(ip_address: "203.0.113.2", user: user)` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0215: create destroys user sessions

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_ban_destroys_the_single_session`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/bans_controller_test.rb:27` — `assert_difference -> { user.sessions.count }, -1 do` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0219: non-admins cannot ban users

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_kevin_cannot_ban_jz`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/bans_controller_test.rb:70` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0220: destroy removes ban records and sets user to active

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_unban_deletes_record_and_activates`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/bans_controller_test.rb:78` — `assert user.reload.banned?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/users/bans_controller_test.rb:79` — `assert_equal 1, user.bans.count` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/users/bans_controller_test.rb:81` — `assert_difference -> { Ban.count }, -1 do` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/users/bans_controller_test.rb:85` — `assert_redirected_to user_url(user)` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |
| `test/controllers/users/bans_controller_test.rb:86` — `assert user.reload.active?` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0221: non-admins cannot unban users

Status: **closed**. Executed identities:

- `controllers::accounts::original_control_tests::original_kevin_cannot_unban_jz`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/bans_controller_test.rb:97` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/original_control_tests.rs:82 |

## P0216: create succeeds when the user has a pending two-factor setup secret

Status: **closed**. Executed identities:

- `controllers::users::ban_lifecycle_tests::banning_a_user_removes_pending_two_factor_setup_and_sessions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/bans_controller_test.rb:37` — `assert_difference -> { user.sessions.count }, -1 do` | rust/crates/campfire/src/controllers/users/ban_lifecycle_tests.rs:90 |
| `test/controllers/users/bans_controller_test.rb:41` — `assert_redirected_to user_url(user)` | rust/crates/campfire/src/controllers/users/ban_lifecycle_tests.rs:71 |
| `test/controllers/users/bans_controller_test.rb:42` — `assert_empty TwoFactorSetupSecret.where(session_id: session.id)` | rust/crates/campfire/src/controllers/users/ban_lifecycle_tests.rs:90 |

## P0217: create enqueues RemoveBannedContentJob

Status: **closed**. Executed identities:

- `controllers::users::ban_lifecycle_tests::ban_http_enqueue_is_atomic_and_writes_one_durable_remove_job`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/bans_controller_test.rb:48` — `assert_enqueued_with(job: RemoveBannedContentJob, args: [ user ]) do` | rust/crates/campfire/src/controllers/users/ban_lifecycle_tests.rs:172 |

## P0218: RemoveBannedContentJob deletes messages

Status: **closed**. Executed identities:

- `controllers::users::ban_lifecycle_tests::ban_http_removes_the_users_messages_through_the_real_runner`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/bans_controller_test.rb:62` — `assert_empty user.reload.messages` | rust/crates/campfire/src/controllers/users/ban_lifecycle_tests.rs:221 |

## P0233: index lists icons with previews shortcodes titles and uploaders

Status: **closed**. Executed identities:

- `controllers::accounts::icons::original_tests::original_acme_upload_list_and_audit_shortcodes`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:13` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:59 |
| `test/controllers/accounts/icons_controller_test.rb:14` — `assert_select "img[src=?]", workspace_icon_path(name: "acme")` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:64 |
| `test/controllers/accounts/icons_controller_test.rb:15` — `assert_select "code", text: ":acme:"` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:71 |
| `test/controllers/accounts/icons_controller_test.rb:16` — `assert_match "Acme Corp", response.body` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:78 |
| `test/controllers/accounts/icons_controller_test.rb:17` — `assert_match "Uploaded by #{icon.creator.name}", response.body` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:79 |

## P0234: create uploads an icon

Status: **closed**. Executed identities:

- `controllers::accounts::icons::original_tests::original_acme_upload_list_and_audit_shortcodes`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:21` — `assert_difference "WorkspaceIcon.count", 1 do` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:50 |
| `test/controllers/accounts/icons_controller_test.rb:29` — `assert_redirected_to account_icons_url` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:45 |
| `test/controllers/accounts/icons_controller_test.rb:34` — `assert_equal "Acme Corp", icon.title` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:52 |
| `test/controllers/accounts/icons_controller_test.rb:35` — `assert_equal users(:david), icon.creator` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:52 |
| `test/controllers/accounts/icons_controller_test.rb:36` — `assert icon.image.attached?` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:53 |

## P0088: workspace icon create and destroy are recorded

Status: **closed**. Executed identities:

- `controllers::accounts::icons::original_tests::original_acme_upload_list_and_audit_shortcodes`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:209` — `assert_difference -> { AuditLog.where(action: "workspace_icon.create").count }, +1 do` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:55 |
| `test/controllers/audit_log/rooms_audit_test.rb:220` — `assert_equal icon.id, create.target_id` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:55 |
| `test/controllers/audit_log/rooms_audit_test.rb:221` — `assert_equal "WorkspaceIcon", create.target_type` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:55 |
| `test/controllers/audit_log/rooms_audit_test.rb:222` — `assert_equal ":acme:", create.target_label` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:55 |
| `test/controllers/audit_log/rooms_audit_test.rb:224` — `assert_difference -> { AuditLog.where(action: "workspace_icon.destroy").count }, +1 do` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:92 |
| `test/controllers/audit_log/rooms_audit_test.rb:229` — `assert_equal icon.id, destroy.target_id` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:92 |
| `test/controllers/audit_log/rooms_audit_test.rb:230` — `assert_equal ":acme:", destroy.target_label` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:92 |

## P0238: members get forbidden on list create and delete

Status: **closed**. Executed identities:

- `controllers::accounts::icons::original_tests::original_jz_cannot_list_create_or_remove_the_existing_icon`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:88` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:106 |
| `test/controllers/accounts/icons_controller_test.rb:96` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:110 |
| `test/controllers/accounts/icons_controller_test.rb:99` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:128 |
| `test/controllers/accounts/icons_controller_test.rb:101` — `assert WorkspaceIcon.exists?(icon.id)` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:135 |

## P0235: create renders validation errors inline

Status: **closed**. Executed identities:

- `controllers::accounts::icons::tests::invalid_and_duplicate_uploads_return_inline_errors_without_rows_or_audits`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:40` — `assert_no_difference "WorkspaceIcon.count" do` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:221 |
| `test/controllers/accounts/icons_controller_test.rb:48` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:222 |
| `test/controllers/accounts/icons_controller_test.rb:51` — `assert_match "already taken by a built-in icon", response.body` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:223 |

## P0236: create reports a name that raced past validation as taken

Status: **closed**. Executed identities:

- `controllers::accounts::icons::tests::uniqueness_index_races_render_taken_and_roll_back_upload_and_audit`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:57` — `assert_no_difference "WorkspaceIcon.count" do` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:420 |
| `test/controllers/accounts/icons_controller_test.rb:65` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:416 |
| `test/controllers/accounts/icons_controller_test.rb:68` — `assert_match "has already been taken", response.body` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:417 |

## P0237: destroy removes the icon and its blob

Status: **closed**. Executed identities:

- `controllers::accounts::icons::tests::destroy_removes_attachment_and_blob_and_records_one_snapshot_audit`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/icons_controller_test.rb:75` — `assert_difference [ "WorkspaceIcon.count", "ActiveStorage::Blob.count" ], -1 do` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:320 |
| `test/controllers/accounts/icons_controller_test.rb:78` — `assert_redirected_to account_icons_url` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:297 |

## P0265: serves an SVG with the documented headers

Status: **closed**. Executed identities:

- `controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:13` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:332 |
| `test/controllers/workspace_icons_controller_test.rb:14` — `assert_equal "image/svg+xml", response.media_type` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:334 |
| `test/controllers/workspace_icons_controller_test.rb:15` — `assert_equal "inline", response.headers["Content-Disposition"].split(";").first` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:340 |
| `test/controllers/workspace_icons_controller_test.rb:16` — `assert_equal "nosniff", response.headers["X-Content-Type-Options"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:339 |
| `test/controllers/workspace_icons_controller_test.rb:17` — `assert_equal "default-src 'none'; style-src 'unsafe-inline'", response.headers["Content-Security-Policy"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:345 |
| `test/controllers/workspace_icons_controller_test.rb:18` — `assert_equal "max-age=3600, private", response.headers["Cache-Control"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:335 |
| `test/controllers/workspace_icons_controller_test.rb:19` — `assert_equal %("#{@icon.image.blob.checksum}"), response.headers["ETag"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:343 |
| `test/controllers/workspace_icons_controller_test.rb:20` — `assert_equal @icon.image.blob.download, response.body` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:333 |

## P0266: serves a PNG without the SVG-only headers

Status: **closed**. Executed identities:

- `controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:29` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:332 |
| `test/controllers/workspace_icons_controller_test.rb:30` — `assert_equal "image/png", response.media_type` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:334 |
| `test/controllers/workspace_icons_controller_test.rb:31` — `assert_equal "max-age=3600, private", response.headers["Cache-Control"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:335 |
| `test/controllers/workspace_icons_controller_test.rb:32` — `assert_equal "nosniff", response.headers["X-Content-Type-Options"]` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:339 |
| `test/controllers/workspace_icons_controller_test.rb:34` — `assert_includes response.headers["Content-Security-Policy"].to_s, "script-src 'self'"` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:350 |

## P0267: supports conditional GETs with the blob checksum

Status: **closed**. Executed identities:

- `controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:42` — `assert_response :not_modified` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:359 |
| `test/controllers/workspace_icons_controller_test.rb:43` — `assert_empty response.body` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:360 |

## P0268: returns not found for unknown names

Status: **closed**. Executed identities:

- `controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:51` — `assert_response :not_found` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:366 |

## P0269: returns not found for signed-out users

Status: **closed**. Executed identities:

- `controllers::accounts::icons::tests::serving_svg_and_png_uses_private_bytes_and_checksum_conditionals`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:57` — `assert_response :not_found` | rust/crates/campfire/src/controllers/accounts/icons/tests.rs:361 |

## P0270: unenrolled sessions are sent to setup instead of served the icon

Status: **closed**. Executed identities:

- `controllers::accounts::icons::original_tests::original_password_session_enrollment_and_stale_icon_access`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:65` — `assert_redirected_to two_factor_setup_url` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:185 |

## P0271: stale enrolled sessions are signed out instead of served the icon

Status: **closed**. Executed identities:

- `controllers::accounts::icons::original_tests::original_password_session_enrollment_and_stale_icon_access`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/workspace_icons_controller_test.rb:75` — `assert_redirected_to new_session_url` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:203 |
| `test/controllers/workspace_icons_controller_test.rb:76` — `assert_nil Session.find_by(token: token)` | rust/crates/campfire/src/controllers/accounts/icons/original_tests.rs:204 |

## P0257: show stock

Status: **closed**. Executed identities:

- `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:11` — `assert_valid_png_response size: 512` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:60; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:75; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:80 |
| `test/controllers/accounts/logos_controller_test.rb:50` — `assert_equal @response.headers["content-type"], "image/png"` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:55 |
| `test/controllers/accounts/logos_controller_test.rb:53` — `assert_equal size, image.width` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:72 |
| `test/controllers/accounts/logos_controller_test.rb:54` — `assert_equal size, image.height` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:77 |

## P0258: show stock small size

Status: **closed**. Executed identities:

- `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:16` — `assert_valid_png_response size: 192` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:60; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:75; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:80 |
| `test/controllers/accounts/logos_controller_test.rb:50` — `assert_equal @response.headers["content-type"], "image/png"` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:55 |
| `test/controllers/accounts/logos_controller_test.rb:53` — `assert_equal size, image.width` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:72 |
| `test/controllers/accounts/logos_controller_test.rb:54` — `assert_equal size, image.height` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:77 |

## P0259: show custom

Status: **closed**. Executed identities:

- `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:23` — `assert_valid_png_response size: 512` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:60; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:75; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:80 |
| `test/controllers/accounts/logos_controller_test.rb:50` — `assert_equal @response.headers["content-type"], "image/png"` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:55 |
| `test/controllers/accounts/logos_controller_test.rb:53` — `assert_equal size, image.width` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:72 |
| `test/controllers/accounts/logos_controller_test.rb:54` — `assert_equal size, image.height` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:77 |

## P0260: show custom small size

Status: **closed**. Executed identities:

- `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:30` — `assert_valid_png_response size: 192` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:60; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:75; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:80 |
| `test/controllers/accounts/logos_controller_test.rb:50` — `assert_equal @response.headers["content-type"], "image/png"` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:55 |
| `test/controllers/accounts/logos_controller_test.rb:53` — `assert_equal size, image.width` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:72 |
| `test/controllers/accounts/logos_controller_test.rb:54` — `assert_equal size, image.height` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:77 |

## P0261: show stock when custom logo cannot be resized

Status: **closed**. Executed identities:

- `controllers::accounts::logos::tests::stock_uploaded_and_unresizable_logo_responses_match_rails_bytes_and_headers`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:37` — `assert_valid_png_response size: 512` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:60; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:75; rust/crates/campfire/src/controllers/accounts/logos/tests.rs:80 |
| `test/controllers/accounts/logos_controller_test.rb:50` — `assert_equal @response.headers["content-type"], "image/png"` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:55 |
| `test/controllers/accounts/logos_controller_test.rb:53` — `assert_equal size, image.width` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:72 |
| `test/controllers/accounts/logos_controller_test.rb:54` — `assert_equal size, image.height` (helper/setup) | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:77 |

## P0262: destroy

Status: **closed**. Executed identities:

- `controllers::accounts::logos::tests::logo_upload_replacement_deletion_audits_and_cache_validation_match_rails`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/logos_controller_test.rb:44` — `assert_redirected_to edit_account_url` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:135 |
| `test/controllers/accounts/logos_controller_test.rb:45` — `assert_not accounts(:signal).reload.logo.attached?` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:142 |

## P0085: logo removal is recorded

Status: **closed**. Executed identities:

- `controllers::accounts::logos::tests::logo_upload_replacement_deletion_audits_and_cache_validation_match_rails`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:177` — `assert_difference -> { AuditLog.where(action: "account.settings.change").count }, +1 do` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:145 |
| `test/controllers/audit_log/rooms_audit_test.rb:181` — `assert_equal({ "before" => true, "after" => false }, AuditLog.where(action: "account.settings.change").last.details["logo"])` | rust/crates/campfire/src/controllers/accounts/logos/tests.rs:144 |

## P0072: room create is recorded

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_open_creation_actor_target_label_and_details`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:10` — `assert_difference -> { AuditLog.where(action: "room.create").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:15` — `assert_equal users(:david).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:16` — `assert_equal Rooms::Open.last.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:17` — `assert_equal "Room", entry.target_type` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:18` — `assert_equal "Audited Room", entry.target_label` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:19` — `assert_equal({ "name" => "Audited Room" }, entry.details)` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0073: failed room create writes no row

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_refused_closed_creation_has_no_audit`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:23` — `assert_no_difference -> { AuditLog.where(action: "room.create").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:27` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0074: opening an existing direct room writes no row

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_group_creation_then_reopening_has_one_audit`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:33` — `assert_difference -> { AuditLog.where(action: "room.create").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:37` — `assert_no_difference -> { AuditLog.where(action: "room.create").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0075: room destroy is recorded with the room name

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_room_destroy_label_and_actor`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:45` — `assert_difference -> { AuditLog.where(action: "room.destroy").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:50` — `assert_equal users(:david).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:51` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:52` — `assert_equal "Doomed", entry.target_label` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0076: admin membership changes are recorded with names

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_membership_granted_and_revoked_names`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:62` — `assert_difference -> { AuditLog.where(action: "room.membership.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:69` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:70` — `assert_equal [ add.name ], entry.details["granted"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:71` — `assert_equal [ remove.name ], entry.details["revoked"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0077: re-saving unchanged membership writes no row

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_unchanged_membership_has_no_audit`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:77` — `assert_no_difference -> { AuditLog.where(action: "room.membership.change").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0078: adding group members is recorded with the added users

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_group_add_jz_has_actor_and_granted_name`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:85` — `assert_difference -> { AuditLog.where(action: "room.membership.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:90` — `assert_equal users(:david).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:91` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:92` — `assert_equal [ "JZ" ], entry.details["granted"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0079: adding no new members writes no row

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_group_add_existing_jason_has_no_audit`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:98` — `assert_no_difference -> { AuditLog.where(action: "room.membership.change").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0080: the last member out destroys the group and records it

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_last_leaver_kevin_destroys_weekend_plans`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:111` — `assert_difference -> { AuditLog.where(action: "room.destroy").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:116` — `assert_equal users(:kevin).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:117` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:118` — `assert_equal "Weekend Plans", entry.target_label` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0081: leaving without destroying the group writes no row

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_other_leaver_has_no_destroy_audit`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:124` — `assert_no_difference -> { AuditLog.where(action: "room.destroy").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0082: leaving a channel is recorded with the leaver as actor

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_jz_channel_leave_actor_label_and_revocation`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:133` — `assert_difference -> { AuditLog.where(action: "room.membership.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:138` — `assert_equal users(:jz).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:139` — `assert_equal "JZ <jz@37signals.com>", entry.actor_label` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:140` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:141` — `assert_equal [ "JZ" ], entry.details["revoked"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0083: leaving a group without destroying it is recorded

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_group_leave_david_actor_and_revocation`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:147` — `assert_difference -> { AuditLog.where(action: "room.membership.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:152` — `assert_equal users(:david).id, entry.actor_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:153` — `assert_equal room.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:154` — `assert_equal [ "David" ], entry.details["revoked"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0084: account settings changes are recorded

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_combined_account_settings_audit_fields`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:158` — `assert_difference -> { AuditLog.where(action: "account.settings.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:168` — `assert_equal Current.account.id, entry.target_id` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:169` — `assert_equal "Account", entry.target_type` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:170` — `assert entry.details.key?("name")` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:171` — `assert entry.details.key?("restrict_room_creation_to_administrators")` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0086: custom styles changes are recorded as size and digest

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_styles_audit_sizes_and_digests`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:187` — `assert_difference -> { AuditLog.where(action: "account.custom_styles.change").count }, +1 do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:193` — `assert_equal previous.to_s.bytesize, pair["before"]["size"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:194` — `assert_equal "body { color: red; }".bytesize, pair["after"]["size"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:195` — `assert_equal Digest::SHA256.hexdigest(previous.to_s)[0, 12], pair["before"]["digest"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:196` — `assert_equal Digest::SHA256.hexdigest("body { color: red; }")[0, 12], pair["after"]["digest"]` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |
| `test/controllers/audit_log/rooms_audit_test.rb:197` — `assert_no_match "color: red", entry.details.to_json` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0087: unchanged custom styles write no row

Status: **closed**. Executed identities:

- `controllers::rooms::original_audit_tests::original_same_styles_have_no_audit`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/audit_log/rooms_audit_test.rb:203` — `assert_no_difference -> { AuditLog.where(action: "account.custom_styles.change").count } do` | rust/crates/campfire/src/controllers/rooms/original_audit_tests.rs:124 |

## P0278: create

Status: **closed**. Executed identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_create`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:12` — `assert_response :success` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:109 |
| `test/controllers/unfurl_links_controller_test.rb:15` — `assert_equal "Hey!", json_response["title"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:111 |
| `test/controllers/unfurl_links_controller_test.rb:16` — `assert_equal "https://example.com", json_response["url"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:112 |
| `test/controllers/unfurl_links_controller_test.rb:17` — `assert_equal "https://example.com/image.png", json_response["image"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:113 |
| `test/controllers/unfurl_links_controller_test.rb:18` — `assert_equal "desc..", json_response["description"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:114 |

## P0279: create strips markup from the title and description

Status: **closed**. Executed identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_encoded_markup`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:26` — `assert_response :success` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:123 |
| `test/controllers/unfurl_links_controller_test.rb:29` — `assert_equal "Hey!", json_response["title"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:125 |
| `test/controllers/unfurl_links_controller_test.rb:30` — `assert_equal "desc..", json_response["description"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:126 |

## P0280: create with missing opengraph meta tags

Status: **closed**. Executed identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_missing_tags`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:37` — `assert_response :no_content` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:133 |

## P0281: create returns no content when the title and description are only a markup tag

Status: **closed**. Executed identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_only_markup`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:52` — `assert_response :no_content` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:141 |

## P0282: create for a GitHub PR URL returns no content (PR cards render instead)

Status: **closed**. Executed identities:

- `controllers::unfurl_links::rails_tests::original_github_pr_has_no_unfurl`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:57` — `assert_response :no_content` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:183 |

## P0283: create for a Fizzy card URL returns no content (Fizzy cards render instead)

Status: **closed**. Executed identities:

- `controllers::unfurl_links::rails_tests::original_fizzy_card_has_no_unfurl`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:62` — `assert_response :no_content` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:183 |

## P0284: create with a missing URL

Status: **closed**. Executed identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_missing_url`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:66` — `assert_raise ActionController::ParameterMissing do` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:149 |
| `test/controllers/unfurl_links_controller_test.rb:68` — `assert_response :bad_request` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:148 |

## P0285: create for twitter.com

Status: **closed**. Executed identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_twitter`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:76` — `assert_response :success` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:164 |
| `test/controllers/unfurl_links_controller_test.rb:77` — `assert_equal "Hey!", JSON.parse(response.body)["title"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:165 |

## P0286: create for x.com

Status: **closed**. Executed identities:

- `controllers::unfurl_links::rails_tests::ws15e_rails_composer_x`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/unfurl_links_controller_test.rb:84` — `assert_response :success` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:164 |
| `test/controllers/unfurl_links_controller_test.rb:85` — `assert_equal "Hey!", JSON.parse(response.body)["title"]` | rust/crates/campfire/src/controllers/unfurl_links/rails_tests.rs:165 |

## P0203: show initials

Status: **closed**. Executed identities:

- `controllers::users::avatars::original_tests::original_kevin_initials_text`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/avatars_controller_test.rb:10` — `assert_select "text", text: "K"` | rust/crates/campfire/src/controllers/users/avatars.rs:182 |

## P0206: show image with invalid token responds 404

Status: **closed**. Executed identities:

- `controllers::users::avatars::original_tests::original_invalid_avatar_token_is_not_found`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/avatars_controller_test.rb:32` — `assert_response :not_found` | rust/crates/campfire/src/controllers/users/avatars.rs:181 |

## P0102: configured installation names the operator and contact

Status: **closed**. Executed identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/public_pages_controller_test.rb:171` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 |
| `test/controllers/public_pages_controller_test.rb:173` — `assert_match(/Acme Widgets/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 |
| `test/controllers/public_pages_controller_test.rb:174` — `assert_select 'a[href="mailto:privacy@example.com"]'` | rust/crates/campfire/src/controllers/public_pages.rs:201 |
| `test/controllers/public_pages_controller_test.rb:177` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 |
| `test/controllers/public_pages_controller_test.rb:178` — `assert_match(/Acme Widgets/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 |

## P0176: linking a github login strips and downcases it

Status: **closed**. Executed identities:

- `controllers::users::profile_settings_tests::original_github_login_normalizes_david_gh`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/profiles_controller_test.rb:307` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:246 |
| `test/controllers/users/profiles_controller_test.rb:308` — `assert_equal "david-gh", users(:david).reload.github_login` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:265 |

## P0186: clearing a github login unlinks it

Status: **closed**. Executed identities:

- `controllers::users::profile_settings_tests::original_linked_github_login_is_cleared`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/profiles_controller_test.rb:404` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:246 |
| `test/controllers/users/profiles_controller_test.rb:405` — `assert_nil users(:david).reload.github_login` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:265 |

## P0154: profile shows the meeting fetch notice when a refresh failed

Status: **closed**. Executed identities:

- `controllers::users::profile_gap_original_tests::original_meeting_unreachable_notice`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/profiles_controller_test.rb:82` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 |
| `test/controllers/users/profiles_controller_test.rb:83` — `assert_includes response.body, CGI.escapeHTML(Calendar::MeetingRefresh::UNREACHABLE_MESSAGE)` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 |

## P0165: profile shows the connected account with a disconnect button

Status: **closed**. Executed identities:

- `controllers::users::profile_gap_original_tests::original_connected_david_gmail_email`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/profiles_controller_test.rb:195` — `assert_includes response.body, "Connected as david@gmail.test"` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 |
| `test/controllers/users/profiles_controller_test.rb:196` — `assert_includes response.body, "Disconnect"` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 |

## P0190: changing email with the current password records a self-change

Status: **closed**. Executed identities:

- `controllers::users::profile_gap_original_tests::original_email_change_with_current_password_marks_now`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/profiles_controller_test.rb:437` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 |
| `test/controllers/users/profiles_controller_test.rb:438` — `assert_equal "david@smartdata.net", users(:david).reload.email_address` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 |
| `test/controllers/users/profiles_controller_test.rb:439` — `assert_equal Time.current, users(:david).email_self_changed_at` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 |

## P0191: other profile edits and case-only email edits need no password and record nothing

Status: **closed**. Executed identities:

- `controllers::users::profile_gap_original_tests::original_case_only_email_and_dave_name_leave_marker_null`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/profiles_controller_test.rb:446` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 |
| `test/controllers/users/profiles_controller_test.rb:447` — `assert_equal "Dave", users(:david).reload.name` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 |
| `test/controllers/users/profiles_controller_test.rb:448` — `assert_nil users(:david).email_self_changed_at` | rust/crates/campfire/src/controllers/users/profile_gap_original_tests.rs:88 |

## S0054: channel row shows the live huddle stack with names and count

Status: **closed**. Executed identities:

- `controllers::users::sidebar_original_tests::original_channel_live_stack_names_count_and_attributes`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:69` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 |
| `test/controllers/users/sidebars_controller_test.rb:70` — `assert_select "##{dom_id(room, :list)} .voice-stack--live.voice-stack--huddle" do` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 |
| `test/controllers/users/sidebars_controller_test.rb:71` — `assert_select ".voice-stack__count", text: "2"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 |
| `test/controllers/users/sidebars_controller_test.rb:72` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:david).id}'][title='David']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 |
| `test/controllers/users/sidebars_controller_test.rb:73` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:jason).id}'][title='Jason']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 |
| `test/controllers/users/sidebars_controller_test.rb:75` — `assert_select "##{dom_id(room, :list)} .voice-stack" +` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:103 |

## S0073: board row shows the live huddle stack with names and count

Status: **closed**. Executed identities:

- `controllers::users::sidebar_original_tests::original_board_live_stack_names_count_and_attributes`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:88` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 |
| `test/controllers/users/sidebars_controller_test.rb:89` — `assert_select "##{dom_id(board, :list)} .voice-room__trailing .voice-stack--live.voice-stack--huddle" do` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 |
| `test/controllers/users/sidebars_controller_test.rb:90` — `assert_select ".voice-stack__count", text: "2"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 |
| `test/controllers/users/sidebars_controller_test.rb:91` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:david).id}'][title='David']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 |
| `test/controllers/users/sidebars_controller_test.rb:92` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:jz).id}'][title='JZ']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 |
| `test/controllers/users/sidebars_controller_test.rb:94` — `assert_select "##{dom_id(board, :list)} .voice-stack" +` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:110 |

## S0092: direct row shows the live huddle stack when the peer is in the call

Status: **closed**. Executed identities:

- `controllers::users::sidebar_original_tests::original_direct_live_stack_peer_count_and_attributes`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:106` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:117 |
| `test/controllers/users/sidebars_controller_test.rb:107` — `assert_select "##{dom_id(room, :list)} .voice-stack--live" do` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:117 |
| `test/controllers/users/sidebars_controller_test.rb:108` — `assert_select ".voice-stack__count", text: "1"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:117 |
| `test/controllers/users/sidebars_controller_test.rb:109` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:jason).id}']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:117 |
| `test/controllers/users/sidebars_controller_test.rb:111` — `assert_select "##{dom_id(room, :list)} .voice-stack[aria-label='1 in huddle: Jason']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:117 |

## S0106: quiet rows keep an empty stack target with no visible presence

Status: **closed**. Executed identities:

- `controllers::users::sidebar_original_tests::original_quiet_rows_have_empty_hidden_stack_targets`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:117` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:123 |
| `test/controllers/users/sidebars_controller_test.rb:118` — `assert_select "##{dom_id(rooms(:watercooler), :list)} .voice-stack:not(.voice-stack--live)" +` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:123 |
| `test/controllers/users/sidebars_controller_test.rb:120` — `assert_select ".voice-stack__avatar", count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:123 |
| `test/controllers/users/sidebars_controller_test.rb:121` — `assert_select ".voice-stack__count[hidden]", text: "", visible: false` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:123 |
| `test/controllers/users/sidebars_controller_test.rb:123` — `assert_select "##{dom_id(rooms(:david_and_jason), :list)} .voice-stack:not(.voice-stack--live)" +` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:123 |

## S0119: direct row re-renders when a participant joins

Status: **closed**. Executed identities:

- `controllers::users::sidebar_original_tests::original_cached_direct_rename_then_join_invalidates_only_on_participants`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:133` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 |
| `test/controllers/users/sidebars_controller_test.rb:134` — `assert_select "##{dom_id(room, :list)} .voice-stack:not(.voice-stack--live)"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 |
| `test/controllers/users/sidebars_controller_test.rb:135` — `assert_select "##{dom_id(room, :list)}", text: /Jason/` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 |
| `test/controllers/users/sidebars_controller_test.rb:141` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 |
| `test/controllers/users/sidebars_controller_test.rb:142` — `assert_select "##{dom_id(room, :list)}", text: /Jason/` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 |
| `test/controllers/users/sidebars_controller_test.rb:143` — `assert_select "##{dom_id(room, :list)}", text: /Jordan/, count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 |
| `test/controllers/users/sidebars_controller_test.rb:149` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 |
| `test/controllers/users/sidebars_controller_test.rb:150` — `assert_select "##{dom_id(room, :list)} .voice-stack--live .voice-stack__count", text: "1"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 |
| `test/controllers/users/sidebars_controller_test.rb:151` — `assert_select "##{dom_id(room, :list)}", text: /Jordan/` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:135 |

## S0147: group direct rooms render member names and a huddle stack

Status: **closed**. Executed identities:

- `controllers::users::sidebar_original_tests::original_group_direct_name_and_live_stack`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:161` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:81; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:129 |
| `test/controllers/users/sidebars_controller_test.rb:162` — `assert_select "##{dom_id(group, :list)}", text: /Ping with Jason, Kevin/` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:129 |
| `test/controllers/users/sidebars_controller_test.rb:163` — `assert_select "##{dom_id(group, :list)} .voice-stack--live.voice-stack--huddle" do` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:129 |
| `test/controllers/users/sidebars_controller_test.rb:164` — `assert_select ".voice-stack__count", text: "1"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:129 |
| `test/controllers/users/sidebars_controller_test.rb:165` — `assert_select "img.voice-stack__avatar[data-user-id='#{users(:jason).id}'][title='Jason']"` | rust/crates/campfire/src/controllers/users/people_tests.rs:158; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:129 |

## S0181: no channel or DM stacks without huddle configuration

Status: **closed**. Executed identities:

- `controllers::users::sidebar_original_tests::original_unconfigured_huddle_has_no_stacks_or_presence_controllers`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:195` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:168; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:164 |
| `test/controllers/users/sidebars_controller_test.rb:196` — `assert_select "#shared_rooms .voice-stack", count: 0` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:177; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:164 |
| `test/controllers/users/sidebars_controller_test.rb:197` — `assert_select "#direct_rooms .voice-stack", count: 0` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:177; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:164 |
| `test/controllers/users/sidebars_controller_test.rb:198` — `assert_select "[data-controller~='huddle-presence']", count: 0` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:188; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:164 |

## S0193: sidebar query count does not grow with quiet channels, DMs, boards, and stages

Status: **closed**. Executed identities:

- `controllers::users::sidebar_original_tests::original_mixed_quiet_sidebar_read_counts_stay_flat_with_one_grants_query`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/users/sidebars_controller_test.rb:208` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:232; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:247 |
| `test/controllers/users/sidebars_controller_test.rb:216` — `assert_response :success` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:232; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:247 |
| `test/controllers/users/sidebars_controller_test.rb:218` — `assert_equal baseline.count, with_more_rooms.count,` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:254; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:247 |
| `test/controllers/users/sidebars_controller_test.rb:220` — `assert_equal 1, with_more_rooms.count { &#124;sql&#124; sql.include?("FROM \"huddle_grants\"") }` | rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:259; rust/crates/campfire/src/controllers/users/sidebar_original_tests.rs:247 |

## P0251: past the export cap the page warns and the CSV filename says truncated

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_caps_tests::original_two_row_cap_warns_names_and_limits_csv`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:163` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:64 |
| `test/controllers/accounts/audit_logs_controller_test.rb:164` — `assert_match "newest 2", response.body` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 |
| `test/controllers/accounts/audit_logs_controller_test.rb:167` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:66 |
| `test/controllers/accounts/audit_logs_controller_test.rb:168` — `assert_match "truncated-to-2", response.headers["Content-Disposition"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 |
| `test/controllers/accounts/audit_logs_controller_test.rb:169` — `assert_equal 2, CSV.parse(response.body, headers: true).length` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 |

## P0252: within the export cap there is no truncation notice

Status: **closed**. Executed identities:

- `controllers::accounts::audit_logs::original_caps_tests::original_five_row_cap_keeps_all_rows_without_warning`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/accounts/audit_logs_controller_test.rb:178` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:64 |
| `test/controllers/accounts/audit_logs_controller_test.rb:179` — `assert_no_match "newest 5", response.body` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 |
| `test/controllers/accounts/audit_logs_controller_test.rb:182` — `assert_response :success` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:66 |
| `test/controllers/accounts/audit_logs_controller_test.rb:183` — `assert_no_match "truncated", response.headers["Content-Disposition"]` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 |
| `test/controllers/accounts/audit_logs_controller_test.rb:187` — `assert_equal AuditLog.count, CSV.parse(response.body, headers: true).length` | rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:79; rust/crates/campfire/src/controllers/accounts/audit_logs/original_caps_tests.rs:77 |
