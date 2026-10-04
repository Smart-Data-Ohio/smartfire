# Rails assertion to Rust assertion map

Reference: `d7c7de92`. Each row names an original Rails assertion call, its discriminating Rust assertion and the real test that executes it. Shared helper assertions and repeated loop cases are cited explicitly. These tables retain compiler checks as compiler checks; they do not claim matching exception classes between Ruby and the Rust type system.

## WS14e-051

Rails declaration: `test/models/event/reminder_pusher_test.rb:32` — the push body names the venue

Executed test: `campfire::bin/campfire integrations::web_push::tests::ws17_delivery::cutover_c_reminder_real_job_delivers_venue_suffix_in_encrypted_push`.

The original Mocha queue expectation includes an exact body predicate. Rust executes the registered durable job, receives its encrypted Web Push on a local fake service, decrypts it and compares the exact body. No real Google or push service is contacted.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/reminder_pusher_test.rb:38](../../test/models/event/reminder_pusher_test.rb#L38)<br>`pool.expects(:queue).with do \|payload, _subscriptions\|` | [rust/crates/campfire/src/integrations/web_push/ws17_delivery_tests.rs:330](../../rust/crates/campfire/src/integrations/web_push/ws17_delivery_tests.rs#L330)<br>`assert_eq!(requests.len(), 1)`<br><br>[rust/crates/campfire/src/integrations/web_push/ws17_delivery_tests.rs:332](../../rust/crates/campfire/src/integrations/web_push/ws17_delivery_tests.rs#L332)<br>`assert_eq!( payload["options"]["body"], "Starts in 15 minutes: Launch party planning in Lounge" )` |

## WS14e-057

Rails declaration: `test/models/event/reminder_pusher_test.rb:115` — an event that already ended is skipped

Executed test: `campfire::bin/campfire integrations::web_push::tests::ws17_delivery::cutover_c_reminder_real_job_suppresses_ended_events_with_recent_running_control`.

The original already-ended event is tested, plus a recently ended event (start -3m/end -1m) and a still-running control. All execute the real ReminderPushJob. reminder-ended-control.rb runs the added control on pinned Rails.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/event/reminder_pusher_test.rb:121](../../test/models/event/reminder_pusher_test.rb#L121)<br>`Rails.configuration.x.web_push_pool.expects(:queue).never` | [rust/crates/campfire/src/integrations/web_push/ws17_delivery_tests.rs:361](../../rust/crates/campfire/src/integrations/web_push/ws17_delivery_tests.rs#L361)<br>`assert!(service.server.received().is_empty())` |

## WS14g-004

Rails declaration: `test/controllers/messages_drive_attachments_test.rb:221` — viewers with and without Drive consent receive identical attachment markup

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_drive_chip_markup_is_identical_with_and_without_viewer_consent`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/messages_drive_attachments_test.rb:226](../../test/controllers/messages_drive_attachments_test.rb#L226)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:100](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L100)<br>`assert_eq!(without.status, 200)` |
| [test/controllers/messages_drive_attachments_test.rb:232](../../test/controllers/messages_drive_attachments_test.rb#L232)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:107](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L107)<br>`assert_eq!(with.status, 200)` |
| [test/controllers/messages_drive_attachments_test.rb:235](../../test/controllers/messages_drive_attachments_test.rb#L235)<br>`assert_equal without_consent, with_consent` | [rust/crates/campfire/src/app/cutover_c_tests.rs:112](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L112)<br>`assert_eq!(first, second)` |
| [test/controllers/messages_drive_attachments_test.rb:236](../../test/controllers/messages_drive_attachments_test.rb#L236)<br>`assert_includes with_consent, "Google Drive file"` | [rust/crates/campfire/src/app/cutover_c_tests.rs:113](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L113)<br>`assert!(second.contains("Google Drive file"))` |

## WS14g-005

Rails declaration: `test/controllers/messages_drive_attachments_test.rb:239` — edit form lists attachments as removable chips with the blank sentinel

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_drive_edit_form_has_two_removable_chips_and_exact_hidden_sentinels`.

The hidden-input assertion executes independently for FILE_A, FILE_B and the blank sentinel. All selectors are applied to the real edit HTTP DOM.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/messages_drive_attachments_test.rb:245](../../test/controllers/messages_drive_attachments_test.rb#L245)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:124](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L124)<br>`assert_eq!(response.status, 200)` |
| [test/controllers/messages_drive_attachments_test.rb:246](../../test/controllers/messages_drive_attachments_test.rb#L246)<br>`assert_select ".drive-attachment-chip", count: 2` | [rust/crates/campfire/src/app/cutover_c_tests.rs:126](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L126)<br>`assert_eq!( nodes(&d, root, \|d, n\| class(d, n, "drive-attachment-chip")).len(), 2 )` |
| [test/controllers/messages_drive_attachments_test.rb:247](../../test/controllers/messages_drive_attachments_test.rb#L247)<br>`assert_select "input[type='hidden'][name='message[drive_file_ids][]'][value='#{FILE_A}']", count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:131](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L131)<br>`assert_eq!( nodes(&d, root, \|d, n\| d.local_name(n) == Some("input") && d.attr(n, "type") == Some("hidden") && d.attr(n, "name") == Some("message[drive_file_ids][]") && d.attr(n, "value") == Some(expected)) .len(), 1 )` |
| [test/controllers/messages_drive_attachments_test.rb:248](../../test/controllers/messages_drive_attachments_test.rb#L248)<br>`assert_select "input[type='hidden'][name='message[drive_file_ids][]'][value='#{FILE_B}']", count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:131](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L131)<br>`assert_eq!( nodes(&d, root, \|d, n\| d.local_name(n) == Some("input") && d.attr(n, "type") == Some("hidden") && d.attr(n, "name") == Some("message[drive_file_ids][]") && d.attr(n, "value") == Some(expected)) .len(), 1 )` |
| [test/controllers/messages_drive_attachments_test.rb:249](../../test/controllers/messages_drive_attachments_test.rb#L249)<br>`assert_select "input[type='hidden'][name='message[drive_file_ids][]'][value='']", count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:131](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L131)<br>`assert_eq!( nodes(&d, root, \|d, n\| d.local_name(n) == Some("input") && d.attr(n, "type") == Some("hidden") && d.attr(n, "name") == Some("message[drive_file_ids][]") && d.attr(n, "value") == Some(expected)) .len(), 1 )` |
| [test/controllers/messages_drive_attachments_test.rb:250](../../test/controllers/messages_drive_attachments_test.rb#L250)<br>`assert_select ".drive-attachment-chip__remove", count: 2` | [rust/crates/campfire/src/app/cutover_c_tests.rs:140](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L140)<br>`assert_eq!( nodes(&d, root, \|d, n\| class( d, n, "drive-attachment-chip__remove" )) .len(), 2 )` |

## WS14g-007

Rails declaration: `test/controllers/sudos_controller_test.rb:29` — confirming with the password verifies and audit-logs

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_sudo_password_confirmation_adds_one_success_audit_and_verifies`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:32](../../test/controllers/sudos_controller_test.rb#L32)<br>`assert_difference -> { AuditLog.where(action: "sudo.confirm.success").count }, +1 do` | [rust/crates/campfire/src/app/cutover_c_tests.rs:170](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L170)<br>`assert_eq!(audit_success(&a).await - before, 1)` |
| [test/controllers/sudos_controller_test.rb:36](../../test/controllers/sudos_controller_test.rb#L36)<br>`assert_redirected_to root_url` | [rust/crates/campfire/src/app/cutover_c_tests.rs:171](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L171)<br>`assert_eq!(response.location(), Some("http://campfire.test/"))` |

## WS14g-011

Rails declaration: `test/controllers/sudos_controller_test.rb:67` — register_verifier adds a verifier

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_sudo_registered_extra_verifier_is_retained_once_and_unsupported_without_an_implementation`.

The app-owned verifier registry is read by the real sudo prompt. Registration retains passkey once; it supplies no implementation, so a real POST remains unsupported as Rails verify_with does.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:70](../../test/controllers/sudos_controller_test.rb#L70)<br>`assert_includes SudoMode.extra_verifiers, :passkey` | [rust/crates/campfire/src/app/cutover_c_tests.rs:185](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L185)<br>`assert!( a.booted .app .sudo .extra_verifiers() .contains(&"passkey".into()) )` |

## WS14g-015

Rails declaration: `test/controllers/sudos_controller_test.rb:109` — confirming with the password still works for enrolled users

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_sudo_enrolled_user_can_still_confirm_with_password`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:115](../../test/controllers/sudos_controller_test.rb#L115)<br>`assert_redirected_to root_url` | [rust/crates/campfire/src/app/cutover_c_tests.rs:232](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L232)<br>`assert_eq!(response.location(), Some("http://campfire.test/"))` |

## WS14g-022

Rails declaration: `test/controllers/sudos_controller_test.rb:210` — browsing the audit log needs no confirmation

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_sudo_audit_log_read_requires_no_confirmation`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:215](../../test/controllers/sudos_controller_test.rb#L215)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:237](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L237)<br>`assert_eq!( a.sign_in(DAVID) .await .get("/account/audit_log") .await .status, 200 )` |

## WS14g-025

Rails declaration: `test/controllers/sudos_controller_test.rb:238` — signing in again starts unverified

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_sudo_signing_in_again_drops_previous_confirmation`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:249](../../test/controllers/sudos_controller_test.rb#L249)<br>`assert_redirected_to new_sudo_url` | [rust/crates/campfire/src/app/cutover_c_tests.rs:272](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L272)<br>`assert_eq!( b.write(Req::new(Method::POST, "/account/join_code")) .await .location(), Some("http://campfire.test/sudo/new") )` |

## WS14g-027

Rails declaration: `test/controllers/sudos_controller_test.rb:264` — the replay form rebuilds nested params

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_sudo_replay_form_rebuilds_nested_user_role`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:268](../../test/controllers/sudos_controller_test.rb#L268)<br>`assert_redirected_to new_sudo_url` | [rust/crates/campfire/src/app/cutover_c_tests.rs:284](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L284)<br>`assert_eq!( b.write(Req::new(Method::PATCH, &path).form(&[("user[role]", "administrator")])) .await .location(), Some("http://campfire.test/sudo/new") )` |
| [test/controllers/sudos_controller_test.rb:271](../../test/controllers/sudos_controller_test.rb#L271)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:293](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L293)<br>`assert_eq!(r.status, 200)` |
| [test/controllers/sudos_controller_test.rb:272](../../test/controllers/sudos_controller_test.rb#L272)<br>`assert_select "form[action=?]", account_user_path(users(:kevin)) do` | [rust/crates/campfire/src/app/cutover_c_tests.rs:298](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L298)<br>`assert_eq!(forms.len(), 1)` |
| [test/controllers/sudos_controller_test.rb:273](../../test/controllers/sudos_controller_test.rb#L273)<br>`assert_select "input[name=?][value=administrator]", "user[role]"` | [rust/crates/campfire/src/app/cutover_c_tests.rs:299](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L299)<br>`assert_eq!( nodes(&d, forms[0], \|d, n\| d.local_name(n) == Some("input") && d.attr(n, "name") == Some("user[role]") && d.attr(n, "value") == Some("administrator")) .len(), 1 )` |

## WS14g-034

Rails declaration: `test/controllers/sudos_controller_test.rb:449` — the confirmation limit lives in the shared rate-limit store, not per-process memory

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_sudo_confirmation_limit_is_reset_by_clearing_the_shared_store`.

Ten real production sudo requests, an eleventh rate-limit control, shared Kit store clear and another real request. Only the store-clear route is a test seam; verification and audit writes use the production controller.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/controllers/sudos_controller_test.rb:458](../../test/controllers/sudos_controller_test.rb#L458)<br>`assert_response :unauthorized` | [rust/crates/campfire/src/app/cutover_c_tests.rs:382](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L382)<br>`assert_eq!(router.oneshot(send("/sudo")).await.unwrap().status(), 401)` |

## WS14g-035

Rails declaration: `test/integration/drive_picker_test.rb:11` — composer omits the Drive menu item without Drive consent

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_picker_without_drive_consent_omits_legacy_menu_even_with_calendar_grant`.

The same assertions run before any Google grant and after a calendar-only grant.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/drive_picker_test.rb:14](../../test/integration/drive_picker_test.rb#L14)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:406](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L406)<br>`assert_eq!(r.status, 200)` |
| [test/integration/drive_picker_test.rb:15](../../test/integration/drive_picker_test.rb#L15)<br>`assert_not_includes response.body, "google-drive-previews"` | [rust/crates/campfire/src/app/cutover_c_tests.rs:407](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L407)<br>`assert!(!r.text().contains("google-drive-previews"))` |
| [test/integration/drive_picker_test.rb:16](../../test/integration/drive_picker_test.rb#L16)<br>`assert_select '[data-controller="drive-picker"]', count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:409](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L409)<br>`assert_eq!(controller_count(&d, root, "drive-picker"), 0)` |
| [test/integration/drive_picker_test.rb:17](../../test/integration/drive_picker_test.rb#L17)<br>`assert_select ".attach-menu", count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:410](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L410)<br>`assert_eq!(nodes(&d, root, \|d, n\| class(d, n, "attach-menu")).len(), 0)` |
| [test/integration/drive_picker_test.rb:18](../../test/integration/drive_picker_test.rb#L18)<br>`assert_select "button.composer__attachment-btn[aria-haspopup]", count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:411](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L411)<br>`assert_eq!( nodes(&d, root, \|d, n\| d.local_name(n) == Some("button") && class(d, n, "composer__attachment-btn") && d.has_attr(n, "aria-haspopup")) .len(), 0 )` |
| [test/integration/drive_picker_test.rb:24](../../test/integration/drive_picker_test.rb#L24)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:406](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L406)<br>`assert_eq!(r.status, 200)` |
| [test/integration/drive_picker_test.rb:25](../../test/integration/drive_picker_test.rb#L25)<br>`assert_not_includes response.body, "google-drive-previews"` | [rust/crates/campfire/src/app/cutover_c_tests.rs:407](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L407)<br>`assert!(!r.text().contains("google-drive-previews"))` |
| [test/integration/drive_picker_test.rb:26](../../test/integration/drive_picker_test.rb#L26)<br>`assert_select '[data-controller="drive-picker"]', count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:409](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L409)<br>`assert_eq!(controller_count(&d, root, "drive-picker"), 0)` |
| [test/integration/drive_picker_test.rb:27](../../test/integration/drive_picker_test.rb#L27)<br>`assert_select ".attach-menu", count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:410](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L410)<br>`assert_eq!(nodes(&d, root, \|d, n\| class(d, n, "attach-menu")).len(), 0)` |

## WS14g-036

Rails declaration: `test/integration/drive_picker_test.rb:30` — composer carries the Drive menu item with the Drive scope

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_picker_drive_scope_renders_legacy_menu_buttons_and_dialog`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/drive_picker_test.rb:35](../../test/integration/drive_picker_test.rb#L35)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:425](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L425)<br>`assert_eq!(r.status, 200)` |
| [test/integration/drive_picker_test.rb:36](../../test/integration/drive_picker_test.rb#L36)<br>`assert_includes response.body, '<meta name="google-drive-previews" content="enabled">'` | [rust/crates/campfire/src/app/cutover_c_tests.rs:426](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L426)<br>`assert!( r.text() .contains("<meta name=\"google-drive-previews\" content=\"enabled\">") )` |
| [test/integration/drive_picker_test.rb:37](../../test/integration/drive_picker_test.rb#L37)<br>`assert_select '[data-controller="drive-picker"]', count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:431](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L431)<br>`assert_eq!(controller_count(&d, root, "drive-picker"), 1)` |
| [test/integration/drive_picker_test.rb:38](../../test/integration/drive_picker_test.rb#L38)<br>`assert_select "button.composer__attachment-btn[aria-haspopup='menu']", count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:432](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L432)<br>`assert_eq!( nodes(&d, root, \|d, n\| d.local_name(n) == Some("button") && class(d, n, "composer__attachment-btn") && d.attr(n, "aria-haspopup") == Some("menu")) .len(), 1 )` |
| [test/integration/drive_picker_test.rb:39](../../test/integration/drive_picker_test.rb#L39)<br>`assert_select ".attach-menu [role='menuitem']", count: 2` | [rust/crates/campfire/src/app/cutover_c_tests.rs:442](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L442)<br>`assert_eq!(items.len(), 2)` |
| [test/integration/drive_picker_test.rb:40](../../test/integration/drive_picker_test.rb#L40)<br>`assert_select ".attach-menu [role='menuitem']", text: "From this device"` | [rust/crates/campfire/src/app/cutover_c_tests.rs:443](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L443)<br>`assert!( items .iter() .any(\|&n\| d.text_content(n).trim() == "From this device") )` |
| [test/integration/drive_picker_test.rb:41](../../test/integration/drive_picker_test.rb#L41)<br>`assert_select ".attach-menu [role='menuitem']", text: "From Google Drive"` | [rust/crates/campfire/src/app/cutover_c_tests.rs:448](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L448)<br>`assert!( items .iter() .any(\|&n\| d.text_content(n).trim() == "From Google Drive") )` |
| [test/integration/drive_picker_test.rb:42](../../test/integration/drive_picker_test.rb#L42)<br>`assert_select '.drive-picker__panel[role="dialog"][aria-label="Find a Drive file"]', count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:453](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L453)<br>`assert_eq!( nodes(&d, root, \|d, n\| class(d, n, "drive-picker__panel") && d.attr(n, "role") == Some("dialog") && d.attr(n, "aria-label") == Some("Find a Drive file")) .len(), 1 )` |

## WS14g-037

Rails declaration: `test/integration/drive_share_picker_test.rb:16` — composer carries a single enhanced Drive menu item when sharing is configured

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_picker_configured_sharing_renders_single_enhanced_menu_and_public_metas`.

The public-meta assertion runs for all four exact Rails meta elements separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/drive_share_picker_test.rb:21](../../test/integration/drive_share_picker_test.rb#L21)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:465](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L465)<br>`assert_eq!(r.status, 200)` |
| [test/integration/drive_share_picker_test.rb:22](../../test/integration/drive_share_picker_test.rb#L22)<br>`assert_includes response.body, '<meta name="google-drive-share" content="enabled">'` | [rust/crates/campfire/src/app/cutover_c_tests.rs:472](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L472)<br>`assert!(r.text().contains(meta))` |
| [test/integration/drive_share_picker_test.rb:23](../../test/integration/drive_share_picker_test.rb#L23)<br>`assert_includes response.body, '<meta name="google-picker-client-id" content="test-client-id">'` | [rust/crates/campfire/src/app/cutover_c_tests.rs:472](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L472)<br>`assert!(r.text().contains(meta))` |
| [test/integration/drive_share_picker_test.rb:24](../../test/integration/drive_share_picker_test.rb#L24)<br>`assert_includes response.body, '<meta name="google-picker-api-key" content="test-picker-key">'` | [rust/crates/campfire/src/app/cutover_c_tests.rs:472](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L472)<br>`assert!(r.text().contains(meta))` |
| [test/integration/drive_share_picker_test.rb:25](../../test/integration/drive_share_picker_test.rb#L25)<br>`assert_includes response.body, '<meta name="google-cloud-project-number" content="123456789012">'` | [rust/crates/campfire/src/app/cutover_c_tests.rs:472](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L472)<br>`assert!(r.text().contains(meta))` |
| [test/integration/drive_share_picker_test.rb:26](../../test/integration/drive_share_picker_test.rb#L26)<br>`assert_select '[data-controller="drive-share"]', count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:475](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L475)<br>`assert_eq!(controller_count(&d, root, "drive-share"), 1)` |
| [test/integration/drive_share_picker_test.rb:27](../../test/integration/drive_share_picker_test.rb#L27)<br>`assert_select "button.composer__attachment-btn[aria-haspopup='menu']", count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:476](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L476)<br>`assert_eq!( nodes(&d, root, \|d, n\| d.local_name(n) == Some("button") && class(d, n, "composer__attachment-btn") && d.attr(n, "aria-haspopup") == Some("menu")) .len(), 1 )` |
| [test/integration/drive_share_picker_test.rb:28](../../test/integration/drive_share_picker_test.rb#L28)<br>`assert_select ".attach-menu [role='menuitem']", text: "From Google Drive", count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:485](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L485)<br>`assert_eq!( nodes(&d, menus[0], \|d, n\| d.attr(n, "role") == Some("menuitem") && d.text_content(n).trim() == "From Google Drive") .len(), 1 )` |
| [test/integration/drive_share_picker_test.rb:29](../../test/integration/drive_share_picker_test.rb#L29)<br>`assert_select '[data-controller="drive-picker"]', count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:491](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L491)<br>`assert_eq!(controller_count(&d, root, "drive-picker"), 0)` |

## WS14g-038

Rails declaration: `test/integration/drive_share_picker_test.rb:32` — enhanced menu item needs no Drive consent and wins over the legacy picker

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_picker_enhanced_menu_precedes_legacy_picker_with_existing_consent`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/drive_share_picker_test.rb:38](../../test/integration/drive_share_picker_test.rb#L38)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:498](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L498)<br>`assert_eq!(r.status, 200)` |
| [test/integration/drive_share_picker_test.rb:39](../../test/integration/drive_share_picker_test.rb#L39)<br>`assert_select '[data-controller="drive-share"]', count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:500](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L500)<br>`assert_eq!(controller_count(&d, root, "drive-share"), 1)` |
| [test/integration/drive_share_picker_test.rb:40](../../test/integration/drive_share_picker_test.rb#L40)<br>`assert_select '[data-controller="drive-picker"]', count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:501](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L501)<br>`assert_eq!(controller_count(&d, root, "drive-picker"), 0)` |
| [test/integration/drive_share_picker_test.rb:41](../../test/integration/drive_share_picker_test.rb#L41)<br>`assert_includes response.body, '<meta name="google-drive-previews" content="enabled">'` | [rust/crates/campfire/src/app/cutover_c_tests.rs:502](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L502)<br>`assert!( r.text() .contains("<meta name=\"google-drive-previews\" content=\"enabled\">") )` |

## WS14g-039

Rails declaration: `test/integration/drive_share_picker_test.rb:44` — composer falls back to the legacy picker when sharing is not configured

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_picker_missing_sharing_configuration_falls_back_to_legacy`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/drive_share_picker_test.rb:49](../../test/integration/drive_share_picker_test.rb#L49)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:512](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L512)<br>`assert_eq!(r.status, 200)` |
| [test/integration/drive_share_picker_test.rb:50](../../test/integration/drive_share_picker_test.rb#L50)<br>`assert_not_includes response.body, "google-drive-share"` | [rust/crates/campfire/src/app/cutover_c_tests.rs:513](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L513)<br>`assert!(!r.text().contains("google-drive-share"))` |
| [test/integration/drive_share_picker_test.rb:51](../../test/integration/drive_share_picker_test.rb#L51)<br>`assert_select '[data-controller="drive-share"]', count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:515](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L515)<br>`assert_eq!(controller_count(&d, root, "drive-share"), 0)` |
| [test/integration/drive_share_picker_test.rb:52](../../test/integration/drive_share_picker_test.rb#L52)<br>`assert_select '[data-controller="drive-picker"]', count: 1` | [rust/crates/campfire/src/app/cutover_c_tests.rs:516](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L516)<br>`assert_eq!(controller_count(&d, root, "drive-picker"), 1)` |

## WS14g-040

Rails declaration: `test/integration/drive_share_picker_test.rb:55` — composer omits every Drive menu item without sharing or Drive consent

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_picker_unconfigured_without_drive_grant_has_no_menu`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/drive_share_picker_test.rb:58](../../test/integration/drive_share_picker_test.rb#L58)<br>`assert_response :success` | [rust/crates/campfire/src/app/cutover_c_tests.rs:522](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L522)<br>`assert_eq!(r.status, 200)` |
| [test/integration/drive_share_picker_test.rb:59](../../test/integration/drive_share_picker_test.rb#L59)<br>`assert_not_includes response.body, "google-drive-share"` | [rust/crates/campfire/src/app/cutover_c_tests.rs:523](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L523)<br>`assert!(!r.text().contains("google-drive-share"))` |
| [test/integration/drive_share_picker_test.rb:60](../../test/integration/drive_share_picker_test.rb#L60)<br>`assert_select '[data-controller="drive-share"]', count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:525](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L525)<br>`assert_eq!(controller_count(&d, root, "drive-share"), 0)` |
| [test/integration/drive_share_picker_test.rb:61](../../test/integration/drive_share_picker_test.rb#L61)<br>`assert_select '[data-controller="drive-picker"]', count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:526](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L526)<br>`assert_eq!(controller_count(&d, root, "drive-picker"), 0)` |
| [test/integration/drive_share_picker_test.rb:62](../../test/integration/drive_share_picker_test.rb#L62)<br>`assert_select ".attach-menu", count: 0` | [rust/crates/campfire/src/app/cutover_c_tests.rs:527](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L527)<br>`assert_eq!(nodes(&d, root, \|d, n\| class(d, n, "attach-menu")).len(), 0)` |

## WS14g-041

Rails declaration: `test/integration/drive_share_picker_test.rb:65` — signed-out visitors see no share metas or buttons

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_picker_signed_out_visitors_are_redirected_before_sharing_markup`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/integration/drive_share_picker_test.rb:71](../../test/integration/drive_share_picker_test.rb#L71)<br>`assert_redirected_to new_session_url` | [rust/crates/campfire/src/app/cutover_c_tests.rs:535](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L535)<br>`assert_eq!(r.location(), Some("http://campfire.test/session/new"))` |

## WS14g-053

Rails declaration: `test/jobs/calendar/disconnect_cleanup_job_test.rb:27` — refreshes an expired access token from the snapshot

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_cleanup_expired_snapshot_refreshes_before_delete_with_new_access_token`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:39](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L39)<br>`assert_requested :post, GOOGLE_TOKEN_URL, body: hash_including({ "grant_type" => "refresh_token" })` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:130](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L130)<br>`assert_eq!( c.iter() .map(\|v\| (v["method"].as_str().unwrap(), v["path"].as_str().unwrap())) .collect::<Vec<_>>(), vec![("POST", "/token"), ("DELETE", ORPHAN), ("POST", "/revoke")] )`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:136](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L136)<br>`assert!( url::form_urlencoded::parse(c[0]["body"].as_str().unwrap().as_bytes()) .any(\|(key, value)\| key == "grant_type" && value == "refresh_token") )` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:40](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L40)<br>`assert_requested delete_stub, headers: { "Authorization" => "Bearer #{refreshed_token}" }` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:130](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L130)<br>`assert_eq!( c.iter() .map(\|v\| (v["method"].as_str().unwrap(), v["path"].as_str().unwrap())) .collect::<Vec<_>>(), vec![("POST", "/token"), ("DELETE", ORPHAN), ("POST", "/revoke")] )`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:140](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L140)<br>`assert_eq!(c[1]["access_token"], "second-access-token")` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:41](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L41)<br>`assert_requested revoke` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:130](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L130)<br>`assert_eq!( c.iter() .map(\|v\| (v["method"].as_str().unwrap(), v["path"].as_str().unwrap())) .collect::<Vec<_>>(), vec![("POST", "/token"), ("DELETE", ORPHAN), ("POST", "/revoke")] )`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:141](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L141)<br>`assert_eq!(c[2]["method"], "POST")` |

## WS14g-054

Rails declaration: `test/jobs/calendar/disconnect_cleanup_job_test.rb:44` — an invalid_grant skips deletes but still revokes

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_cleanup_invalid_grant_skips_deletes_but_revokes_snapshot_refresh_token`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:54](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L54)<br>`assert_not_requested :delete, "#{GOOGLE_EVENTS_URL}/orphan-id"` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:158](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L158)<br>`assert!(!c.iter().any(\|v\| v["method"] == "DELETE"))` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:55](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L55)<br>`assert_requested revoke` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:159](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L159)<br>`assert_eq!( c.iter() .map(\|v\| (v["method"].as_str().unwrap(), v["path"].as_str().unwrap())) .collect::<Vec<_>>(), vec![("POST", "/token"), ("POST", "/revoke")] )` |

## WS14g-056

Rails declaration: `test/jobs/calendar/disconnect_cleanup_job_test.rb:73` — a refresh 503 retries without revoking until the deletes succeed

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_cleanup_refresh_503_retries_without_deletes_or_revoke_then_recovers`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:83](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L83)<br>`assert_enqueued_with(job: Calendar::DisconnectCleanupJob, args: [ [ "orphan-id" ], snapshot ]) do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:176](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L176)<br>`assert!(attempt(&a, CLEANUP, 1).await.is_some())`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:178](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L178)<br>`assert_eq!(retry.len(), 1)`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:179](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L179)<br>`assert_eq!(retry[0].arguments, args)` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:87](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L87)<br>`assert_not_requested delete_stub` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:180](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L180)<br>`assert_eq!( calls(&r) .iter() .map(\|v\| (v["method"].as_str().unwrap(), v["path"].as_str().unwrap())) .collect::<Vec<_>>(), vec![("POST", "/token")] )` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:88](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L88)<br>`assert_not_requested revoke` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:180](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L180)<br>`assert_eq!( calls(&r) .iter() .map(\|v\| (v["method"].as_str().unwrap(), v["path"].as_str().unwrap())) .collect::<Vec<_>>(), vec![("POST", "/token")] )` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:94](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L94)<br>`assert_requested delete_stub, headers: { "Authorization" => "Bearer #{recovered_token}" }` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:206](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L206)<br>`assert_eq!( (c[1]["method"].as_str(), c[1]["path"].as_str()), (Some("DELETE"), Some(ORPHAN)) )`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:210](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L210)<br>`assert_eq!(c[1]["access_token"], "recovered-access-token")` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:95](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L95)<br>`assert_requested revoke` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:211](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L211)<br>`assert_eq!( (c[2]["method"].as_str(), c[2]["path"].as_str()), (Some("POST"), Some("/revoke")) )` |

## WS14g-058

Rails declaration: `test/jobs/calendar/disconnect_cleanup_job_test.rb:113` — a revoke 5xx schedules a retry

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_cleanup_revoke_500_keeps_same_durable_retry_payload`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:120](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L120)<br>`assert_enqueued_with(job: Calendar::DisconnectCleanupJob, args: [ [ "orphan-id" ], snapshot ]) do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:227](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L227)<br>`assert!(attempt(&a, CLEANUP, 1).await.is_some())`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:229](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L229)<br>`assert_eq!(rows.len(), 1)`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:230](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L230)<br>`assert_eq!(rows[0].arguments, args)` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:124](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L124)<br>`assert_requested delete_stub` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:232](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L232)<br>`assert_eq!( (c[0]["method"].as_str(), c[0]["path"].as_str()), (Some("DELETE"), Some(ORPHAN)) )` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:125](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L125)<br>`assert_requested revoke` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:236](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L236)<br>`assert_eq!( (c[1]["method"].as_str(), c[1]["path"].as_str()), (Some("POST"), Some("/revoke")) )` |

## WS14g-059

Rails declaration: `test/jobs/calendar/disconnect_cleanup_job_test.rb:128` — an exhausted retry logs the failure at error level

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_cleanup_exhausted_retry_logs_error_and_never_revokes`.

Initial serialized exception group count is 8, matching the original Rails setup. The registered handler commits terminal deletion and logs ERROR; no revoke occurs.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:139](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L139)<br>`assert_no_enqueued_jobs(only: Calendar::DisconnectCleanupJob) do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:306](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L306)<br>`assert_eq!(attempt(&a, CLEANUP, 9).await, None)`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:307](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L307)<br>`assert!(pending(&a, CLEANUP).await.is_empty())` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:144](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L144)<br>`assert_includes log, "Calendar::DisconnectCleanupJob failed after retries"` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:308](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L308)<br>`assert!(logs.text().lines().any(\|line\| line.contains("ERROR") && line.contains("Calendar::DisconnectCleanupJob failed after retries")))` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:145](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L145)<br>`assert_not_requested revoke` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:310](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L310)<br>`assert!(!calls(&r).iter().any(\|c\| c["path"] == "/revoke"))` |

## WS14g-062

Rails declaration: `test/jobs/calendar/disconnect_cleanup_job_test.rb:197` — an unreadable credentials blob logs a warning with the account id

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_cleanup_unreadable_blob_warns_with_account_and_without_tokens_or_google_http`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:211](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L211)<br>`assert_includes log, "account #{account_id}"` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:333](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L333)<br>`assert!( log.lines() .any(\|line\| line.contains("WARN") && line.contains(&format!("account {account_id}"))) )` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:212](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L212)<br>`assert_not_includes log, access_token` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:337](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L337)<br>`assert!(!log.contains("access-token"))` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:213](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L213)<br>`assert_not_includes log, refresh_token` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:338](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L338)<br>`assert!(!log.contains("refresh-token"))` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:214](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L214)<br>`assert_not_requested :delete, %r{\A#{Regexp.escape(GOOGLE_EVENTS_URL)}/}` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:339](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L339)<br>`assert!(calls(&r).is_empty())` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:215](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L215)<br>`assert_not_requested :post, GOOGLE_REVOKE_URL` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:339](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L339)<br>`assert!(calls(&r).is_empty())` |

## WS14g-063

Rails declaration: `test/jobs/calendar/disconnect_cleanup_job_test.rb:218` — tokens never reach the job logs

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_cleanup_encrypted_credentials_and_job_logs_never_contain_plaintext_tokens`.

Rust has no Ruby log_arguments class predicate. The original property is tested at enqueue/execution: even the encrypted argument blob is absent from actual job logs, as are both plaintext tokens.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:225](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L225)<br>`assert_not_includes blob.inspect, access_token` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:347](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L347)<br>`assert!(!blob.contains("access-token"))` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:226](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L226)<br>`assert_not_includes blob.inspect, refresh_token` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:348](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L348)<br>`assert!(!blob.contains("refresh-token"))` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:227](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L227)<br>`assert_not_predicate Calendar::DisconnectCleanupJob, :log_arguments?` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:359](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L359)<br>`assert!(!logs.text().contains(&blob))` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:237](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L237)<br>`assert_not_includes log, access_token` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:360](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L360)<br>`assert!(!logs.text().contains("access-token"))` |
| [test/jobs/calendar/disconnect_cleanup_job_test.rb:238](../../test/jobs/calendar/disconnect_cleanup_job_test.rb#L238)<br>`assert_not_includes log, refresh_token` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:361](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L361)<br>`assert!(!logs.text().contains("refresh-token"))` |

## WS14g-066

Rails declaration: `test/jobs/calendar/remote_delete_job_test.rb:10` — deletes the remote copy with the user's current credentials

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_remote_delete_uses_current_account_access_credentials`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/remote_delete_job_test.rb:16](../../test/jobs/calendar/remote_delete_job_test.rb#L16)<br>`assert_requested delete_stub, headers: { "Authorization" => "Bearer #{account.access_token}" }` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:373](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L373)<br>`assert_eq!(calls(&r).len(), 1)`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:374](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L374)<br>`assert_eq!( ( calls(&r)[0]["method"].as_str(), calls(&r)[0]["path"].as_str() ), (Some("DELETE"), Some(ORPHAN)) )`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:381](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L381)<br>`assert_eq!(calls(&r)[0]["access_token"], "access-token")` |

## WS14g-067

Rails declaration: `test/jobs/calendar/remote_delete_job_test.rb:19` — skips a missing account without a request

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_remote_delete_without_account_makes_no_request`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/remote_delete_job_test.rb:22](../../test/jobs/calendar/remote_delete_job_test.rb#L22)<br>`assert_not_requested :delete, "#{GOOGLE_EVENTS_URL}/orphan-id"` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:387](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L387)<br>`assert!(calls(&r).is_empty())` |

## WS14g-068

Rails declaration: `test/jobs/calendar/remote_delete_job_test.rb:25` — skips a disconnected account without a request

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_remote_delete_disconnected_account_makes_no_request`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/remote_delete_job_test.rb:30](../../test/jobs/calendar/remote_delete_job_test.rb#L30)<br>`assert_not_requested :delete, "#{GOOGLE_EVENTS_URL}/orphan-id"` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:402](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L402)<br>`assert!(calls(&r).is_empty())` |

## WS14g-069

Rails declaration: `test/jobs/calendar/remote_delete_job_test.rb:33` — skips a grant without the calendar scope

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_remote_delete_without_calendar_scope_makes_no_request`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/remote_delete_job_test.rb:38](../../test/jobs/calendar/remote_delete_job_test.rb#L38)<br>`assert_not_requested :delete, "#{GOOGLE_EVENTS_URL}/orphan-id"` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:419](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L419)<br>`assert!(calls(&r).is_empty())` |

## WS14g-070

Rails declaration: `test/jobs/calendar/remote_delete_job_test.rb:41` — treats gone and revoked copies as success

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_remote_delete_not_found_and_gone_complete_without_retry`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/remote_delete_job_test.rb:49](../../test/jobs/calendar/remote_delete_job_test.rb#L49)<br>`assert_requested missing_delete` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:432](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L432)<br>`assert_eq!( (c[0]["method"].as_str(), c[0]["path"].as_str()), ( Some("DELETE"), Some("/calendar/v3/calendars/primary/events/missing-id") ) )` |
| [test/jobs/calendar/remote_delete_job_test.rb:50](../../test/jobs/calendar/remote_delete_job_test.rb#L50)<br>`assert_requested gone_delete` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:439](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L439)<br>`assert_eq!( (c[1]["method"].as_str(), c[1]["path"].as_str()), ( Some("DELETE"), Some("/calendar/v3/calendars/primary/events/gone-id") ) )` |

## WS14g-071

Rails declaration: `test/jobs/calendar/remote_delete_job_test.rb:53` — an invalid_grant disconnects and counts as success

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_remote_delete_invalid_grant_disconnects_without_delete_or_retry`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/remote_delete_job_test.rb:60](../../test/jobs/calendar/remote_delete_job_test.rb#L60)<br>`assert_equal "Google rejected the connection", account.reload.disconnected_reason` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:453](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L453)<br>`assert_eq!( a.db() .read(\|c\| GoogleAccount::for_user(c, DAVID)) .await .unwrap() .unwrap() .disconnected_reason .as_deref(), Some("Google rejected the connection") )` |
| [test/jobs/calendar/remote_delete_job_test.rb:61](../../test/jobs/calendar/remote_delete_job_test.rb#L61)<br>`assert_not_requested :delete, "#{GOOGLE_EVENTS_URL}/orphan-id"` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:463](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L463)<br>`assert!(!calls(&r).iter().any(\|c\| c["method"] == "DELETE"))` |

## WS14g-072

Rails declaration: `test/jobs/calendar/remote_delete_job_test.rb:64` — a transient failure schedules a retry

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_remote_delete_transport_failure_schedules_same_payload_retry`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/remote_delete_job_test.rb:68](../../test/jobs/calendar/remote_delete_job_test.rb#L68)<br>`assert_enqueued_with(job: Calendar::RemoteDeleteJob, args: [ @david.id, "orphan-id" ]) do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:470](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L470)<br>`assert!(remote(&a).await.is_some())`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:472](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L472)<br>`assert_eq!(rows.len(), 1)`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:473](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L473)<br>`assert_eq!(rows[0].arguments, json!([DAVID, "orphan-id"]))` |

## WS14g-073

Rails declaration: `test/jobs/calendar/remote_delete_job_test.rb:73` — other Google failures are logged without raising or retrying

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_remote_delete_permanent_google_failure_logs_and_does_not_retry`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/remote_delete_job_test.rb:77](../../test/jobs/calendar/remote_delete_job_test.rb#L77)<br>`assert_no_enqueued_jobs(only: Calendar::RemoteDeleteJob) do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:482](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L482)<br>`assert_eq!(remote(&a).await, None)`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:483](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L483)<br>`assert!(pending(&a, REMOTE).await.is_empty())`<br><br>[rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:484](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L484)<br>`assert_eq!(emitted(&a, REMOTE).await.len(), 1)` |

## WS14g-074

Rails declaration: `test/jobs/calendar/sync_entry_job_test.rb:444` — responding going on the first event of a series syncs every occurrence

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_series_head_rsvp_commits_three_sync_jobs_and_three_distinct_google_entries`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/sync_entry_job_test.rb:451](../../test/jobs/calendar/sync_entry_job_test.rb#L451)<br>`assert_equal 3, occurrences.size` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:557](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L557)<br>`assert_eq!(occurrences.len(), 3)` |
| [test/jobs/calendar/sync_entry_job_test.rb:454](../../test/jobs/calendar/sync_entry_job_test.rb#L454)<br>`assert_enqueued_jobs 3, only: Calendar::SyncEntryJob do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:585](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L585)<br>`assert_eq!(emitted(&a, SYNC).await.len(), 3)` |
| [test/jobs/calendar/sync_entry_job_test.rb:460](../../test/jobs/calendar/sync_entry_job_test.rb#L460)<br>`assert_requested :post, GOOGLE_EVENTS_URL, times: 3` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:598](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L598)<br>`assert_eq!( calls(&r) .iter() .filter( \|c\| c["method"] == "POST" && c["path"] == "/calendar/v3/calendars/primary/events" ) .count(), 3 )` |
| [test/jobs/calendar/sync_entry_job_test.rb:462](../../test/jobs/calendar/sync_entry_job_test.rb#L462)<br>`assert_equal 3, google_ids.size` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:608](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L608)<br>`assert_eq!(rows.len(), 3)` |
| [test/jobs/calendar/sync_entry_job_test.rb:463](../../test/jobs/calendar/sync_entry_job_test.rb#L463)<br>`assert_equal 3, google_ids.uniq.size` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:613](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L613)<br>`assert_eq!(ids.len(), 3)` |

## WS14g-077

Rails declaration: `test/jobs/calendar/sync_entry_job_test.rb:484` — updating event times enqueues syncs for connected going/maybe attendees

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_time_update_enqueues_only_connected_notifying_attendees`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/sync_entry_job_test.rb:489](../../test/jobs/calendar/sync_entry_job_test.rb#L489)<br>`assert_enqueued_jobs 1, only: Calendar::SyncEntryJob do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:653](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L653)<br>`assert_eq!(jobs.len(), 1)` |
| [test/jobs/calendar/sync_entry_job_test.rb:494](../../test/jobs/calendar/sync_entry_job_test.rb#L494)<br>`assert_equal [ @event.id, @david.id ], job[:args]` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:654](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L654)<br>`assert_eq!(jobs[0], json!({"event_id":event_id(),"user_id":DAVID}))` |

## WS14g-078

Rails declaration: `test/jobs/calendar/sync_entry_job_test.rb:497` — an update inside a transaction enqueues only after commit

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_event_sync_is_invisible_to_queue_readers_until_source_transaction_commits`.

WS17 persists jobs within the source write for atomicity. An independent queue reader sees no uncommitted job; after commit the observed durable payload is exact. This is the observable after-commit contract, not a claim that the writer cannot see its own inserted row.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/sync_entry_job_test.rb:502](../../test/jobs/calendar/sync_entry_job_test.rb#L502)<br>`assert_no_enqueued_jobs only: Calendar::SyncEntryJob` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:680](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L680)<br>`assert_eq!(visible, 0)` |
| [test/jobs/calendar/sync_entry_job_test.rb:505](../../test/jobs/calendar/sync_entry_job_test.rb#L505)<br>`assert_enqueued_with(job: Calendar::SyncEntryJob, args: [ @event.id, @david.id ])` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:685](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L685)<br>`assert_eq!( emitted(&a, SYNC).await, vec![json!({"event_id":event_id(),"user_id":DAVID})] )` |

## WS14g-079

Rails declaration: `test/jobs/calendar/sync_entry_job_test.rb:508` — setting or clearing the venue enqueues a sync, like a title change

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_setting_and_clearing_venue_each_enqueue_connected_attendee_sync`.

The payload assertion executes once for setting the real voice venue and once for clearing it.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/sync_entry_job_test.rb:512](../../test/jobs/calendar/sync_entry_job_test.rb#L512)<br>`assert_enqueued_with(job: Calendar::SyncEntryJob, args: [ @event.id, @david.id ]) do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:717](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L717)<br>`assert_eq!( emitted(&a, SYNC).await, vec![json!({"event_id":event_id(),"user_id":DAVID})] )` |
| [test/jobs/calendar/sync_entry_job_test.rb:516](../../test/jobs/calendar/sync_entry_job_test.rb#L516)<br>`assert_enqueued_with(job: Calendar::SyncEntryJob, args: [ @event.id, @david.id ]) do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:717](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L717)<br>`assert_eq!( emitted(&a, SYNC).await, vec![json!({"event_id":event_id(),"user_id":DAVID})] )` |

## WS14g-080

Rails declaration: `test/jobs/calendar/sync_entry_job_test.rb:521` — updating only the title enqueues a sync, an unchanged save does not

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_title_change_enqueues_sync_and_unchanged_save_does_not`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/sync_entry_job_test.rb:524](../../test/jobs/calendar/sync_entry_job_test.rb#L524)<br>`assert_enqueued_with(job: Calendar::SyncEntryJob, args: [ @event.id, @david.id ]) do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:743](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L743)<br>`assert_eq!( emitted(&a, SYNC).await, vec![json!({"event_id":event_id(),"user_id":DAVID})] )` |
| [test/jobs/calendar/sync_entry_job_test.rb:528](../../test/jobs/calendar/sync_entry_job_test.rb#L528)<br>`assert_no_enqueued_jobs do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:764](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L764)<br>`assert!(emitted(&a, SYNC).await.is_empty())` |

## WS14g-083

Rails declaration: `test/jobs/calendar/sync_entry_job_test.rb:552` — destroying a room enqueues remote deletes for its entries

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_room_destroy_commits_remote_delete_identity_and_real_google_request`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/sync_entry_job_test.rb:557](../../test/jobs/calendar/sync_entry_job_test.rb#L557)<br>`assert_enqueued_with(job: Calendar::RemoteDeleteJob, args: [ @david.id, "room-destroy-id" ]) do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:791](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L791)<br>`assert_eq!( emitted(&a, REMOTE).await, vec![json!([DAVID, "room-destroy-id"])] )` |
| [test/jobs/calendar/sync_entry_job_test.rb:563](../../test/jobs/calendar/sync_entry_job_test.rb#L563)<br>`assert_requested delete_stub` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:796](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L796)<br>`assert!(calls(&r).iter().any(\|c\| c["method"] == "DELETE" && c["path"] == "/calendar/v3/calendars/primary/events/room-destroy-id"))` |

## WS14g-084

Rails declaration: `test/jobs/calendar/sync_entry_job_test.rb:566` — shrinking a series enqueues remote deletes for destroyed occurrences

Executed test: `campfire::bin/campfire app::cutover_c_tests::calendar_jobs::cutover_c_series_shrink_commits_doomed_remote_identity_and_deletes_google_copy`.

Nonselected jobs are parked for the frozen-clock runner, matching Rails perform_enqueued_jobs only: RemoteDeleteJob. Their payloads remain intact. The tested remote job uses the captured identity and the real recorded Google client.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/jobs/calendar/sync_entry_job_test.rb:576](../../test/jobs/calendar/sync_entry_job_test.rb#L576)<br>`assert_enqueued_with(job: Calendar::RemoteDeleteJob, args: [ @david.id, "doomed-entry" ]) do` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:882](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L882)<br>`assert_eq!( emitted(&a, REMOTE).await, vec![json!([DAVID, "doomed-entry"])] )` |
| [test/jobs/calendar/sync_entry_job_test.rb:582](../../test/jobs/calendar/sync_entry_job_test.rb#L582)<br>`assert_requested delete_stub` | [rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs:897](../../rust/crates/campfire/src/app/cutover_c_tests/calendar_jobs.rs#L897)<br>`assert!(calls(&r).iter().any(\|c\| c["method"] == "DELETE" && c["path"] == "/calendar/v3/calendars/primary/events/doomed-entry"))` |

## WS14g-121

Rails declaration: `test/models/drive_attachment_test.rb:9` — a message with attachments and no text is valid

Executed test: `campfire_db tests::cutover_drive_attachment_test::cutover_c_drive_only_message_is_valid_saved_and_reads_exact_file_ids`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/drive_attachment_test.rb:13](../../test/models/drive_attachment_test.rb#L13)<br>`assert message.valid?, message.errors.full_messages.to_sentence` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:22](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L22)<br>`assert!(t.read(\|c\| Message::validate(c, &a)).is_empty())` |
| [test/models/drive_attachment_test.rb:14](../../test/models/drive_attachment_test.rb#L14)<br>`assert message.save` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:24](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L24)<br>`assert!(saved.is_ok())` |
| [test/models/drive_attachment_test.rb:15](../../test/models/drive_attachment_test.rb#L15)<br>`assert_equal %w[ 1AbcDefGhIjKlMnOpQrSt ], message.reload.drive_attachments.map(&:file_id)` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:26](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L26)<br>`assert_eq!(t.read(\|c\| m.drive_file_ids(c)), vec![FILE])` |

## WS14g-122

Rails declaration: `test/models/drive_attachment_test.rb:18` — a textless message without attachments is still invalid

Executed test: `campfire_db tests::cutover_drive_attachment_test::cutover_c_textless_message_without_attachments_has_blank_source_error`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/drive_attachment_test.rb:21](../../test/models/drive_attachment_test.rb#L21)<br>`assert_not message.valid?` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:33](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L33)<br>`assert!(!errors.is_empty())` |
| [test/models/drive_attachment_test.rb:22](../../test/models/drive_attachment_test.rb#L22)<br>`assert_includes message.errors[:markdown_source], "can't be blank"` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:34](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L34)<br>`assert!(errors.on("markdown_source").contains(&"can't be blank"))` |

## WS14g-123

Rails declaration: `test/models/drive_attachment_test.rb:25` — an invalid file id is rejected

Executed test: `campfire_db tests::cutover_drive_attachment_test::cutover_c_each_bad_drive_id_is_rejected_by_the_real_message_association_validation`.

The same four invalid IDs run through Message.validate, the real production autosaved Drive association validation; the scoped nested file_id error is checked. The parent stays unsaved, as in Rails.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/drive_attachment_test.rb:31](../../test/models/drive_attachment_test.rb#L31)<br>`assert_not attachment.valid?, "expected #{bad_id.inspect} to be invalid"` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:47](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L47)<br>`assert!(!errors.is_empty())` |
| [test/models/drive_attachment_test.rb:32](../../test/models/drive_attachment_test.rb#L32)<br>`assert_includes attachment.errors[:file_id], "is invalid"` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:48](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L48)<br>`assert!( errors .on("drive_attachments.file_id") .contains(&"is invalid") )` |

## WS14g-124

Rails declaration: `test/models/drive_attachment_test.rb:37` — duplicate file ids on one message collapse to a single row

Executed test: `campfire_db tests::cutover_drive_attachment_test::cutover_c_saved_drive_id_duplicate_is_invalid_with_scoped_file_error`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/drive_attachment_test.rb:43](../../test/models/drive_attachment_test.rb#L43)<br>`assert_not duplicate.valid?` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:62](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L62)<br>`assert!(!errors.is_empty())` |
| [test/models/drive_attachment_test.rb:44](../../test/models/drive_attachment_test.rb#L44)<br>`assert_not_empty duplicate.errors[:file_id]` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:63](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L63)<br>`assert!(!errors.on("file_id").is_empty())` |

## WS14g-125

Rails declaration: `test/models/drive_attachment_test.rb:47` — the same file id may attach to different messages

Executed test: `campfire_db tests::cutover_drive_attachment_test::cutover_c_same_drive_id_attaches_once_to_each_distinct_message`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/drive_attachment_test.rb:54](../../test/models/drive_attachment_test.rb#L54)<br>`assert_equal 1, first.drive_attachments.count` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:74](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L74)<br>`assert_eq!( t.read(\|c\| DriveAttachment::for_message(c, first.id)).len(), 1 )` |
| [test/models/drive_attachment_test.rb:55](../../test/models/drive_attachment_test.rb#L55)<br>`assert_equal 1, second.drive_attachments.count` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:78](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L78)<br>`assert_eq!( t.read(\|c\| DriveAttachment::for_message(c, second.id)).len(), 1 )` |

## WS14g-126

Rails declaration: `test/models/drive_attachment_test.rb:58` — the 11th attachment is rejected

Executed test: `campfire_db tests::cutover_drive_attachment_test::cutover_c_eleventh_drive_attachment_makes_message_invalid_with_exact_limit_error`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/drive_attachment_test.rb:62](../../test/models/drive_attachment_test.rb#L62)<br>`assert message.valid?, message.errors.full_messages.to_sentence` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:91](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L91)<br>`assert!(t.read(\|c\| Message::validate(c, &a)).is_empty())` |
| [test/models/drive_attachment_test.rb:66](../../test/models/drive_attachment_test.rb#L66)<br>`assert_not message.valid?` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:94](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L94)<br>`assert!(!errors.is_empty())` |
| [test/models/drive_attachment_test.rb:67](../../test/models/drive_attachment_test.rb#L67)<br>`assert_equal [ "are limited to 10 per message" ], message.errors[:drive_attachments]` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:95](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L95)<br>`assert_eq!( errors.on("drive_attachments"), vec!["are limited to 10 per message"] )` |

## WS14g-127

Rails declaration: `test/models/drive_attachment_test.rb:70` — destroying the message destroys its attachments

Executed test: `campfire_db tests::cutover_drive_attachment_test::cutover_c_message_destroy_removes_its_persisted_drive_attachment`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/drive_attachment_test.rb:74](../../test/models/drive_attachment_test.rb#L74)<br>`assert_difference -> { DriveAttachment.count }, -1 do` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:120](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L120)<br>`assert_eq!(after - before, -1)` |

## WS14g-128

Rails declaration: `test/models/drive_attachment_test.rb:79` — url is the open link for the file id

Executed test: `campfire_db tests::cutover_drive_attachment_test::cutover_c_drive_attachment_url_is_the_open_link_for_the_id`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/drive_attachment_test.rb:82](../../test/models/drive_attachment_test.rb#L82)<br>`assert_equal "https://drive.google.com/open?id=1AbcDefGhIjKlMnOpQrSt", attachment.url` | [rust/crates/db/src/tests/cutover_drive_attachment_test.rs:125](../../rust/crates/db/src/tests/cutover_drive_attachment_test.rs#L125)<br>`assert_eq!( attachment.url(), "https://drive.google.com/open?id=1AbcDefGhIjKlMnOpQrSt" )` |

## WS14g-184

Rails declaration: `test/models/google_account_test.rb:44` — connected, usable, and expiry predicates

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_google_account_connected_usable_and_expiry_follow_persisted_changes`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/google_account_test.rb:47](../../test/models/google_account_test.rb#L47)<br>`assert_predicate account, :connected?` | [rust/crates/campfire/src/app/cutover_c_tests.rs:557](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L557)<br>`assert!(connected)` |
| [test/models/google_account_test.rb:48](../../test/models/google_account_test.rb#L48)<br>`assert_predicate account, :usable?` | [rust/crates/campfire/src/app/cutover_c_tests.rs:558](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L558)<br>`assert!(usable)` |
| [test/models/google_account_test.rb:49](../../test/models/google_account_test.rb#L49)<br>`assert_not account.access_token_expired?` | [rust/crates/campfire/src/app/cutover_c_tests.rs:559](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L559)<br>`assert!(!expired)` |
| [test/models/google_account_test.rb:53](../../test/models/google_account_test.rb#L53)<br>`assert account.access_token_expired?` | [rust/crates/campfire/src/app/cutover_c_tests.rs:572](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L572)<br>`assert!( a.db() .read(move \|c\| Ok(GoogleAccount::for_user(c, DAVID)? .unwrap() .access_token_expired(&crypto, now) .unwrap())) .await .unwrap() )` |
| [test/models/google_account_test.rb:57](../../test/models/google_account_test.rb#L57)<br>`assert_not_predicate account, :connected?` | [rust/crates/campfire/src/app/cutover_c_tests.rs:598](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L598)<br>`assert!(!connected)` |
| [test/models/google_account_test.rb:58](../../test/models/google_account_test.rb#L58)<br>`assert_not_predicate account, :usable?` | [rust/crates/campfire/src/app/cutover_c_tests.rs:599](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L599)<br>`assert!(!usable)` |

## WS14g-185

Rails declaration: `test/models/google_account_test.rb:61` — calendar? treats blank scopes as granted and requires calendar.events otherwise

Executed test: `campfire::bin/campfire app::cutover_c_tests::cutover_c_google_account_calendar_grant_accepts_blank_scopes_and_requires_events_scope_otherwise`.

Each cited assertion executes through this named real model/HTTP/registered-job test. Repeated loop cases are discriminated separately.

| Rails assertion | Discriminating Rust assertion |
|---|---|
| [test/models/google_account_test.rb:64](../../test/models/google_account_test.rb#L64)<br>`assert_predicate account, :calendar?` | [rust/crates/campfire/src/app/cutover_c_tests.rs:607](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L607)<br>`assert!( a.db() .read(\|c\| Ok(GoogleAccount::for_user(c, DAVID)?.unwrap().calendar())) .await .unwrap() )` |
| [test/models/google_account_test.rb:68](../../test/models/google_account_test.rb#L68)<br>`assert_not_predicate account, :calendar?` | [rust/crates/campfire/src/app/cutover_c_tests.rs:623](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L623)<br>`assert!( !a.db() .read(\|c\| Ok(GoogleAccount::for_user(c, DAVID)?.unwrap().calendar())) .await .unwrap() )` |
| [test/models/google_account_test.rb:72](../../test/models/google_account_test.rb#L72)<br>`assert_predicate account, :calendar?` | [rust/crates/campfire/src/app/cutover_c_tests.rs:630](../../rust/crates/campfire/src/app/cutover_c_tests.rs#L630)<br>`assert!( a.db() .read(\|c\| Ok(GoogleAccount::for_user(c, DAVID)?.unwrap().calendar())) .await .unwrap() )` |

