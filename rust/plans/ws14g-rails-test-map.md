# WS14g Rails test ledger (partial)

Source: this checkout at Rails pin `d7c7de92`. Named declarations are listed individually. A domain property exercised by a native test does not claim its HTTP/browser case is ported. Outside-slice cases have a named owner; consult that owner's report for completion.
Path-glob ledger: 48 files; 611 named cases; 146 ported domain/API/controller cases; 465 partial, deferred or outside slice. Additional Google cases in other controller files: 37.


The 70 signed Google ID-token vectors also run through the local TLS fake. Those cases are not added to these Rails declaration counts.

## test/controllers/accounts/users/google_links_controller_test.rb

- **Ported** — administrators allow a Google link for a self-changed email — `app::google_admin_tests::google_admin_link_controls_and_all_security_audits_match_pinned_rails` (allow + idempotent repeat; exact audit fields).
- **Ported** — administrators unlink a Google identity — `app::google_admin_tests::google_admin_link_controls_and_all_security_audits_match_pinned_rails` (unlink + idempotent repeat; exact audit fields).
- **Ported** — members cannot allow or remove Google links — `app::google_admin_tests::google_admin_link_controls_and_all_security_audits_match_pinned_rails`; removing the permission guard fails at member_allow (302 versus Rails 403).
- **Ported** — bots have no Google link to manage — `app::google_admin_tests::google_admin_link_controls_and_all_security_audits_match_pinned_rails` (bot_target).
- **Ported** — the account page offers the controls to administrators only — `app::google_admin_tests::google_admin_link_controls_and_all_security_audits_match_pinned_rails`; `campfire_views::core::google_administrator_link_forms_match_complete_pinned_rails_bytes` compares every complete form across eleven row states.

## test/controllers/agents/drive_attachments_delivery_test.rb

- **Deferred** — polling carries drive_attachments with file_id and url only — WS14g Drive model + WS11-api caller.
- **Deferred** — polling carries an empty drive_attachments array without attachments — WS14g Drive model + WS11-api caller.
- **Deferred** — agent webhook posts drive_attachments without names — WS14g Drive model + WS11-api caller.

## test/controllers/channel_thread_messages_drive_attachments_test.rb

- **Deferred** — create with drive_file_ids stores them and reports them in JSON — WS14g association + WS8b-m2 message producer.
- **Deferred** — create with attachments and no text is valid — WS14g association + WS8b-m2 message producer.
- **Deferred** — create with an invalid id answers 422 and creates nothing — WS14g association + WS8b-m2 message producer.
- **Deferred** — create with a scalar drive_file_ids answers 422 and creates nothing — WS14g association + WS8b-m2 message producer.
- **Deferred** — create with more than 10 attachments answers 422 — WS14g association + WS8b-m2 message producer.
- **Deferred** — update with a new set replaces the stored set — WS14g association + WS8b-m2 message producer.
- **Deferred** — update without the key leaves the set alone — WS14g association + WS8b-m2 message producer.
- **Deferred** — update with only the blank sentinel removes all attachments — WS14g association + WS8b-m2 message producer.
- **Deferred** — update with an invalid id answers 422 and keeps the stored set — WS14g association + WS8b-m2 message producer.
- **Deferred** — update with a scalar drive_file_ids answers 422 and keeps the stored set — WS14g association + WS8b-m2 message producer.
- **Deferred** — a non-creator cannot change thread attachments — WS14g association + WS8b-m2 message producer.
- **Deferred** — update with a submitted set broadcasts the attachments block over the thread stream — WS14g association + WS8b-m2 message producer.
- **Deferred** — update without the key does not broadcast the attachments block — WS14g association + WS8b-m2 message producer.

## test/controllers/google/calendar_notifications_controller_test.rb

- **Ported API case** — unknown channel answers 404 and enqueues nothing — app::google_webhook_tests (native request/claim/queue tests).
- **Ported API case** — wrong token answers 403 and enqueues nothing — app::google_webhook_tests (native request/claim/queue tests).
- **Ported API case** — sync handshake is acknowledged without work — app::google_webhook_tests (native request/claim/queue tests).
- **Ported API case** — a change notification enqueues one inbound sync — app::google_webhook_tests (native request/claim/queue tests).
- **Ported API case** — a redelivered notification is acknowledged without a second sync — app::google_webhook_tests (native request/claim/queue tests).
- **Ported API case** — not_exists drops the channel and enqueues a re-watch — app::google_webhook_tests (native request/claim/queue tests).
- **Ported API case** — a change notification also enqueues a meeting refresh — app::google_webhook_tests (native request/claim/queue tests).
- **Ported API case** — a redelivered notification enqueues no second meeting refresh — app::google_webhook_tests (native request/claim/queue tests).

## test/controllers/google/connections_controller_test.rb

- **Ported** — connect redirects to Google with the right scope and a state — `app::google_connection_tests::google_connection_authorize_parameters_configuration_and_sign_in_guards`.
- **Ported** — connect requires sign-in — `app::google_connection_tests::google_connection_authorize_parameters_configuration_and_sign_in_guards`.
- **Ported** — connect with features[]=drive requests Calendar plus the per-file Drive scope — `app::google_connection_tests::google_connection_authorize_parameters_configuration_and_sign_in_guards`.
- **Ported** — connect without features requests only the Calendar scope — `app::google_connection_tests::google_connection_authorize_parameters_configuration_and_sign_in_guards`.
- **Ported** — callback with a bad state redirects to the profile with an alert — `app::google_connection_tests::google_connection_failed_exchanges_and_invalid_id_tokens_never_store_grants`.
- **Ported** — callback with a connection failure redirects without storing — `app::google_connection_tests::google_connection_failed_exchanges_and_invalid_id_tokens_never_store_grants`.
- **Ported** — callback without the calendar scope stores the grant but does not claim a connection — `app::google_connection_tests::google_connection_without_calendar_stores_grant_without_audit_or_jobs`.
- **Partial** — callback keeps the calendar scope when Drive is granted alongside — WS14g connection/domain/consumer implementation exists; this named case needs its remaining event/attendance, producer or remote cleanup assertions.
- **Partial** — callback success stores the account and enqueues syncs for upcoming going/maybe attendances — WS14g connection/domain/consumer implementation exists; this named case needs its remaining event/attendance, producer or remote cleanup assertions.
- **Ported** — callback stores the granted scope string — `app::google_connection_tests::google_connection_scope_retention_and_reconnect_clear_disconnected_reason`.
- **Ported** — callback without a scope string leaves scopes unset — `app::google_connection_tests::google_connection_scope_retention_and_reconnect_clear_disconnected_reason`.
- **Ported** — callback without a scope string keeps previously stored scopes — `app::google_connection_tests::google_connection_scope_retention_and_reconnect_clear_disconnected_reason`.
- **Ported** — callback clears a previous disconnected reason on reconnect — `app::google_connection_tests::google_connection_scope_retention_and_reconnect_clear_disconnected_reason`.
- **Ported** — callback with a denied grant redirects without storing — `app::google_connection_tests::google_connection_failed_exchanges_and_invalid_id_tokens_never_store_grants`.
- **Ported** — callback with a failed exchange redirects without storing — `app::google_connection_tests::google_connection_failed_exchanges_and_invalid_id_tokens_never_store_grants`.
- **Ported** — callback without an id_token redirects without storing — `app::google_connection_tests::google_connection_failed_exchanges_and_invalid_id_tokens_never_store_grants`.
- **Ported** — callback with an id_token for another client redirects without storing — `app::google_connection_tests::google_connection_failed_exchanges_and_invalid_id_tokens_never_store_grants`.
- **Ported** — disconnect clears local state without waiting on Google, then cleans up remotely — `app::google_lifecycle_tests::google_lifecycle_matches_pinned_rails_state_jobs_and_remote_stop_order` (disconnect scenarios; exact committed state at channels/stop, snapshot and cleanup job).
- **Ported** — disconnect without readable tokens skips cleanup but still disconnects — `app::google_connection_tests::google_connection_unreadable_disconnect_drops_cache_preserves_flags_and_other_grants`.
- **Ported** — disconnect drops the meeting cache — `app::google_connection_tests::google_connection_unreadable_disconnect_drops_cache_preserves_flags_and_other_grants`.
- **Deferred** — disconnect while in a meeting broadcasts the cleared badge — WS14g integration with WS17 status/OOO broadcast claims; cache removal is ported, the cleared badge broadcast remains absent.
- **Partial** — callback enqueues a meeting refresh for members who left meeting status on — WS14g connection/domain/consumer implementation exists; this named case needs its remaining event/attendance, producer or remote cleanup assertions.
- **Partial** — callback enqueues no meeting refresh without the opt-in — WS14g connection/domain/consumer implementation exists; this named case needs its remaining event/attendance, producer or remote cleanup assertions.
- **Ported** — disconnect without a connection still redirects — `app::google_connection_tests::google_connection_unreadable_disconnect_drops_cache_preserves_flags_and_other_grants`.
- **Partial** — disconnect clears the organizer's stored Meet links but keeps the request — WS14g connection/domain/consumer implementation exists; this named case needs its remaining event/attendance, producer or remote cleanup assertions.
- **Partial** — disconnect leaves another organizer's Meet links alone — WS14g connection/domain/consumer implementation exists; this named case needs its remaining event/attendance, producer or remote cleanup assertions.
- **Partial** — disconnect only touches the current user's entries — WS14g connection/domain/consumer implementation exists; this named case needs its remaining event/attendance, producer or remote cleanup assertions.
- **Ported** — routes 404 when GOOGLE_CLIENT_ID is unset — `app::google_connection_tests::google_connection_authorize_parameters_configuration_and_sign_in_guards`.
## test/controllers/google/drive_files_controller_test.rb

- **Ported** — show renders the file JSON for a connected account with the Drive scope — `app::google_drive_tests::google_drive_file_json_and_all_mime_kinds_match_rails`.
- **Ported** — show maps MIME types to kinds — `app::google_drive_tests::google_drive_file_json_and_all_mime_kinds_match_rails`.
- **Partial** — show is 404 with an empty body without an account — WS14g; endpoint implementation and coalesced tests exist in `app/google_drive_tests.rs`; retain the missing full Rails assertion or router-level expiry/user-budget scenario.
- **Ported** — show is 404 with an empty body for a disconnected account — `app::google_drive_tests::google_drive_dead_unreadable_or_calendar_only_accounts_do_not_call_google`.
- **Ported** — show is 404 with an empty body without the Drive scope — `app::google_drive_tests::google_drive_dead_unreadable_or_calendar_only_accounts_do_not_call_google`.
- **Partial** — show is 404 with an empty body for the retired metadata grant — WS14g; endpoint implementation and coalesced tests exist in `app/google_drive_tests.rs`; retain the missing full Rails assertion or router-level expiry/user-budget scenario.
- **Ported** — show is 404 with an empty body when Google answers 403 or 404 — `app::google_drive_tests::google_drive_transport_quota_and_forbidden_fail_with_rails_statuses`.
- **Ported** — show is 404 with an empty body for a malformed id — `app::google_drive_tests::google_drive_requires_viewer_grant_and_valid_id_and_has_distinct_anonymous_behavior`.
- **Partial** — show is 503 on a Google transport failure — WS14g; endpoint implementation and coalesced tests exist in `app/google_drive_tests.rs`; retain the missing full Rails assertion or router-level expiry/user-budget scenario.
- **Ported** — show requires sign-in — `app::google_drive_tests::google_drive_requires_viewer_grant_and_valid_id_and_has_distinct_anonymous_behavior`.
- **Partial** — show caches the file for five minutes per viewer — WS14g; endpoint implementation and coalesced tests exist in `app/google_drive_tests.rs`; retain the missing full Rails assertion or router-level expiry/user-budget scenario.
- **Ported** — show never reuses another viewer\'s cache entry — `app::google_drive_tests::google_drive_viewer_inaccessible_file_is_empty_404_and_never_reuses_other_viewer_cache`.
- **Partial** — index lists recent files when q is blank — WS14g; endpoint implementation and coalesced tests exist in `app/google_drive_tests.rs`; retain the missing full Rails assertion or router-level expiry/user-budget scenario.
- **Ported** — index treats whitespace-only q as a recent list — `app::google_drive_tests::google_drive_index_refreshes_expired_access_and_invalid_grant_disconnects`.
- **Ported** — index searches by name with quote and backslash escaping — `app::google_drive_tests::google_drive_lists_never_cache_trim_and_cap_terms_and_return_502_on_failure`.
- **Ported** — index trims q and caps it at 100 characters — `app::google_drive_tests::google_drive_lists_never_cache_trim_and_cap_terms_and_return_502_on_failure`.
- **Ported** — index maps unknown MIME types to file — `app::google_drive_tests::google_drive_file_json_and_all_mime_kinds_match_rails`.
- **Ported** — index is 404 with an empty body without Drive consent — `app::google_drive_tests::google_drive_dead_unreadable_or_calendar_only_accounts_do_not_call_google`.
- **Ported** — index is 404 with an empty body for a disconnected account — `app::google_drive_tests::google_drive_dead_unreadable_or_calendar_only_accounts_do_not_call_google`.
- **Ported** — index is 404 when signed out — `app::google_drive_tests::google_drive_requires_viewer_grant_and_valid_id_and_has_distinct_anonymous_behavior`.
- **Ported** — index is 502 when Google fails — `app::google_drive_tests::google_drive_lists_never_cache_trim_and_cap_terms_and_return_502_on_failure`.
- **Ported** — index is 404 when Drive answers forbidden — `app::google_drive_tests::google_drive_transport_quota_and_forbidden_fail_with_rails_statuses`.
- **Ported** — index is 404 when the refresh fails with invalid_grant — `app::google_drive_tests::google_drive_index_refreshes_expired_access_and_invalid_grant_disconnects`.
- **Ported** — index refreshes an expired access token before listing — `app::google_drive_tests::google_drive_index_refreshes_expired_access_and_invalid_grant_disconnects`.
- **Partial** — index throttles each user to 30 lists per minute — WS14g; endpoint implementation and coalesced tests exist in `app/google_drive_tests.rs`; retain the missing full Rails assertion or router-level expiry/user-budget scenario.
- **Ported** — index never caches results — `app::google_drive_tests::google_drive_lists_never_cache_trim_and_cap_terms_and_return_502_on_failure`.
- **Partial** — show throttles each user to 60 views per minute — WS14g; endpoint implementation and coalesced tests exist in `app/google_drive_tests.rs`; retain the missing full Rails assertion or router-level expiry/user-budget scenario.
- **Partial** — show is 503 on a connection failure — WS14g; endpoint implementation and coalesced tests exist in `app/google_drive_tests.rs`; retain the missing full Rails assertion or router-level expiry/user-budget scenario.
- **Ported** — show is 503 when Google rate limits — `app::google_drive_tests::google_drive_transport_quota_and_forbidden_fail_with_rails_statuses`.
- **Ported** — show is 404 with an empty body for an unreadable token — `app::google_drive_tests::google_drive_dead_unreadable_or_calendar_only_accounts_do_not_call_google`.
- **Ported** — index is 404 with an empty body for an unreadable token — `app::google_drive_tests::google_drive_dead_unreadable_or_calendar_only_accounts_do_not_call_google`.
- **Partial** — index is 502 on a connection failure — WS14g; endpoint implementation and coalesced tests exist in `app/google_drive_tests.rs`; retain the missing full Rails assertion or router-level expiry/user-budget scenario.
- **Ported** — index rejects an unenrolled session instead of listing files — `app::google_drive_tests::google_drive_index_rejects_unenrolled_and_stale_enrolled_sessions_before_http`.
- **Ported** — index terminates a stale enrolled session — `app::google_drive_tests::google_drive_index_rejects_unenrolled_and_stale_enrolled_sessions_before_http`.

## test/controllers/messages_drive_attachments_test.rb

- **Deferred** — create with drive_file_ids stores them in order — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — create with attachments and no text is valid — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — create deduplicates repeated ids and strips blanks — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — create with an invalid id answers 422 and creates nothing — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — create with more than 10 attachments answers 422 — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — update with a new set replaces the stored set — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — update with a submitted set broadcasts the attachments block to the room — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — update without the key does not broadcast the attachments block — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — update with a scalar drive_file_ids answers 422 and keeps the stored set — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — update without the key leaves the set alone — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — update with only the blank sentinel removes all attachments — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — removing every attachment from a textless message answers 422 and keeps the set — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — update with an invalid id answers 422 and keeps the stored set — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — a non-creator cannot change attachments — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — JSON message shape includes drive_attachments with file_id and url only — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — JSON message shape carries an empty drive_attachments array without attachments — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — rendered message carries the generic chip with the open link and no file name — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — viewers with and without Drive consent receive identical attachment markup — WS14g association + WS8b-m/m2 message producer.
- **Deferred** — edit form lists attachments as removable chips with the blank sentinel — WS14g association + WS8b-m/m2 message producer.

## test/controllers/rooms/drive_recipients_controller_test.rb

- **Deferred** — index previews eligible members without the requester, ordered by name — WS14g continuation.
- **Deferred** — index needs no Calendar or Drive consent — WS14g continuation.
- **Deferred** — index excludes bots — WS14g continuation.
- **Deferred** — index excludes agent-backed users, deactivated, banned, and unusable emails — WS14g continuation.
- **Deferred** — index spans both company domains and password-login external members with no domain filter — WS14g continuation.
- **Deferred** — index is 404 when sharing is not configured — WS14g continuation.
- **Deferred** — index is 404 when the project number is missing — WS14g continuation.
- **Deferred** — index is 401 when signed out and leaks no emails — WS14g continuation.
- **Deferred** — index is 404 for a nonmember and leaks no emails — WS14g continuation.
- **Deferred** — index is 403 for bot-key and agent-token requests — WS14g continuation.
- **Deferred** — index is 403 for an agent-backed user over a session — WS14g continuation.
- **Deferred** — index throttles each user to 60 previews per minute — WS14g continuation.
- **Deferred** — validate returns the canonical recipients for a current selection — WS14g continuation.
- **Deferred** — validate collapses duplicate ids — WS14g continuation.
- **Deferred** — validate rejects a removed member selected from a stale preview — WS14g continuation.
- **Deferred** — validate rejects self, bots, nonmembers, and unknown ids — WS14g continuation.
- **Deferred** — validate rejects an agent-backed selection and a deactivated member — WS14g continuation.
- **Deferred** — validate rejects a scalar, a missing key, and arbitrary emails — WS14g continuation.
- **Deferred** — validate rejects a selection larger than 100 — WS14g continuation.
- **Deferred** — validate is 404 when sharing is not configured — WS14g continuation.
- **Deferred** — validate is 401 when signed out, 404 for a nonmember, 403 for bots — WS14g continuation.
- **Deferred** — validate requires a CSRF token — WS14g continuation.
- **Deferred** — validate shares the recipient throttle bucket — WS14g continuation.

## test/controllers/sessions/google_configuration_test.rb

- **Deferred** — missing domain configuration disables Google while password login remains available — WS14g continuation.
- **Deferred** — another company can enable its own domain without application changes — WS14g continuation.
- **Deferred** — removing a domain takes effect on an already started Google login — WS14g continuation.
- **Deferred** — allowed email and hosted domains can differ for secondary Workspace domains — WS14g continuation.

## test/controllers/sessions/google_controller_test.rb

- **Partial** — login page offers Google sign-in with the mark, domains, and password note — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Ported** — login page hides the Google button when credentials are missing — `app::google_tests::google_sessions_configured_page_and_disabled_credentials_or_domains_keep_password_login`.
- **Ported** — login page hides the Google button when domains are explicitly empty — `app::google_tests::google_sessions_configured_page_and_disabled_credentials_or_domains_keep_password_login`.
- **Partial** — start redirects to Google with identity-only scope, nonce, and PKCE — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Ported** — start requires CSRF protection — `app::google_tests::google_sessions_csrf_pkce_and_signed_in_guards_match_rails`.
- **Ported** — start and callback 404 when Google credentials are missing — `app::google_tests::google_sessions_configured_page_and_disabled_credentials_or_domains_keep_password_login`.
- **Ported** — start and callback 404 when sign-in domains are explicitly empty — `app::google_tests::google_sessions_configured_page_and_disabled_credentials_or_domains_keep_password_login`.
- **Ported** — signed-in users are sent home instead of starting or finishing Google sign-in — `app::google_tests::google_sessions_csrf_pkce_and_signed_in_guards_match_rails`.
- **Ported** — first-run setup cannot be bypassed through Google sign-in — `app::google_tests::google_sessions_first_run_cannot_be_bypassed`.
- **Partial** — new smartdata.net user is auto-provisioned as an ordinary member — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — new cnbssoftware.com user is auto-provisioned as an ordinary member — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — existing account links by verified email, preserving id, history, role, and password — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — subsequent logins resolve the immutable subject across email changes — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Ported** — post-auth return destination survives the Google round trip — `app::google_tests::google_sessions_secondary_domains_multi_audience_and_return_path_provision_only_identity`.
- **Partial** — external Google account is rejected while password sign-in still works — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Ported** — missing hd is rejected: the email suffix alone proves nothing — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — spoofed hd with an external email domain is rejected — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — allowed email with an external hd is rejected — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — Google-only user cannot sign in with a password — `app::google_tests::google_sessions_secondary_domains_multi_audience_and_return_path_provision_only_identity`.
- **Partial** — deactivated user with a retained identity is rejected, never revived — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — deactivated predecessor without an identity is not recreated — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Ported** — banned user is rejected — `app::google_tests::google_sessions_ineligible_users_and_ambiguous_emails_never_create_identity_or_session`.
- **Ported** — bot user is rejected — `app::google_tests::google_sessions_ineligible_users_and_ambiguous_emails_never_create_identity_or_session`.
- **Ported** — agent user is rejected — `app::google_tests::google_sessions_ineligible_users_and_ambiguous_emails_never_create_identity_or_session`.
- **Ported** — ambiguous duplicate emails are rejected — `app::google_tests::google_sessions_ineligible_users_and_ambiguous_emails_never_create_identity_or_session`.
- **Ported** — a different subject cannot link onto an already-linked user — `app::google_tests::google_sessions_ineligible_users_and_ambiguous_emails_never_create_identity_or_session`.
- **Ported** — malformed id_token is rejected — `app::google_tests::google_sessions_malformed_signature_algorithm_and_unknown_key_fail_closed_through_router`.
- **Ported** — id_token signed by the wrong key is rejected — `app::google_tests::google_sessions_malformed_signature_algorithm_and_unknown_key_fail_closed_through_router`.
- **Ported** — expired id_token is rejected — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — id_token for another audience is rejected — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — multi-audience id_token requires a matching azp — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — multi-audience id_token with a matching azp succeeds — `app::google_tests::google_sessions_secondary_domains_multi_audience_and_return_path_provision_only_identity`.
- **Ported** — id_token with a wrong azp is rejected — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — id_token from an unknown issuer is rejected — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — id_token with a missing or wrong nonce is rejected — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — id_token with a missing or unverified email is rejected — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — id_token with a missing subject is rejected — `app::google_tests::google_sessions_claim_rejections_preserve_all_rows_and_audit_each_attempt`.
- **Ported** — id_token with a non-RS256 algorithm is rejected — `app::google_tests::google_sessions_malformed_signature_algorithm_and_unknown_key_fail_closed_through_router`.
- **Ported** — unknown signing key refetches once and still fails closed — `app::google_tests::google_sessions_malformed_signature_algorithm_and_unknown_key_fail_closed_through_router`.
- **Partial** — key rotation succeeds through a bounded refetch — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — callback with a forged or missing state is rejected without contacting Google — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Ported** — callback with an expired flow is rejected — `app::google_tests::google_sessions_cancel_missing_code_expired_and_replaced_flow_do_not_contact_google`.
- **Partial** — callback state cannot be replayed — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Ported** — cancelled grant redirects without signing in — `app::google_tests::google_sessions_cancel_missing_code_expired_and_replaced_flow_do_not_contact_google`.
- **Ported** — callback without a code is rejected — `app::google_tests::google_sessions_cancel_missing_code_expired_and_replaced_flow_do_not_contact_google`.
- **Ported** — failed code exchange sends PKCE and fails without signing in — `app::google_tests::google_sessions_denied_missing_token_and_transport_outages_have_exact_notices`.
- **Ported** — token exchange without an id_token fails without signing in — `app::google_tests::google_sessions_denied_missing_token_and_transport_outages_have_exact_notices`.
- **Ported** — token endpoint outage fails closed with a retry message — `app::google_tests::google_sessions_denied_missing_token_and_transport_outages_have_exact_notices`; real router/SQLite and recorded transport. Timeout/refused transport errors share the client's `Unavailable` type; real socket timeout coverage is in `sign_in::tests`.
- **Ported** — token endpoint connection failure fails closed with a retry message — `app::google_tests::google_sessions_denied_missing_token_and_transport_outages_have_exact_notices`; real router/SQLite and recorded transport. Timeout/refused transport errors share the client's `Unavailable` type; real socket timeout coverage is in `sign_in::tests`.
- **Ported** — signing key outage fails closed with a retry message — `app::google_tests::google_sessions_denied_missing_token_and_transport_outages_have_exact_notices`; real router/SQLite and recorded transport. Timeout/refused transport errors share the client's `Unavailable` type; real socket timeout coverage is in `sign_in::tests`.
- **Ported** — rejection logs carry no tokens or codes — `app::google_tests::google_sessions_rejection_log_names_reason_without_token_or_authorization_code`.
- **Partial** — authorization code is filtered from logs — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — Google sign-in creates no Calendar/Drive connection and stores no tokens — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — Calendar connection is never treated as login identity — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — connecting Calendar creates no login identity — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — disconnecting Calendar keeps the login identity — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.

## test/controllers/sessions/google_pre_hijack_test.rb

- **Deferred** — a self-changed email is not auto-linked and asks for an administrator — WS14g continuation.
- **Deferred** — an administrator allowing the link lets the next Google sign-in link — WS14g continuation.
- **Deferred** — an account from before the rule, with its original email, still auto-links — WS14g continuation.
- **Deferred** — a join-code signup never auto-links by email, even untouched — WS14g continuation.
- **Deferred** — a Google-provisioned account keeps signing in by subject after a self-change — WS14g continuation.

## test/controllers/sessions/google_responses_test.rb

- **Deferred** — malformed JSON structures from the token endpoint return to password sign-in — WS14g continuation.
- **Deferred** — malformed JSON structures from the key endpoint return a retry message — WS14g continuation.

## test/controllers/sessions/google_status_race_test.rb

- **Deferred** — #{action} racing #{linked ? 'linked' : 'first'} Google login leaves no usable session — WS14g continuation.

## test/controllers/sudos_controller_test.rb

- **Outside slice** — the prompt shows the password form for password users — WS9.
- **Deferred** — the prompt shows Google confirmation for Google-only users — WS14g adapter + WS9 integration.
- **Outside slice** — confirming with the password verifies and audit-logs — WS9.
- **Outside slice** — confirming with the wrong password fails and stays gated — WS9.
- **Outside slice** — an unknown verifier is rejected as unavailable — WS9.
- **Outside slice** — totp is registered at boot but unsupported without enrollment — WS9.
- **Outside slice** — register_verifier adds a verifier — WS9.
- **Outside slice** — the prompt shows the TOTP form alongside the password for enrolled users — WS9.
- **Outside slice** — the prompt hides the TOTP form without enrollment — WS9.
- **Outside slice** — confirming with a current TOTP code verifies and audit-logs — WS9.
- **Outside slice** — confirming with the password still works for enrolled users — WS9.
- **Outside slice** — confirming with a wrong TOTP code fails and stays gated — WS9.
- **Outside slice** — a TOTP code cannot confirm sudo twice — WS9.
- **Outside slice** — wrong sudo codes share the challenge lockout — WS9.
- **Outside slice** — a backup code does not confirm sudo and is not spent — WS9.
- **Outside slice** — a gated POST redirects to the prompt, then the replay form continues the action — WS9.
- **Outside slice** — a gated GET continues automatically after confirmation — WS9.
- **Outside slice** — browsing the audit log needs no confirmation — WS9.
- **Outside slice** — a fresh confirmation lasts fifteen minutes — WS9.
- **Outside slice** — a stale confirmation prompts again — WS9.
- **Outside slice** — signing in again starts unverified — WS9.
- **Outside slice** — requests carrying secrets are never replayed — WS9.
- **Outside slice** — the replay form rebuilds nested params — WS9.
- **Outside slice** — resuming after confirmation never leaves the app — WS9.
- **Outside slice** — non-replayable requests resume on the originating page — WS9.
- **Outside slice** — editing a bot without touching the webhook needs no confirmation — WS9.
- **Outside slice** — changing a bot webhook needs confirmation — WS9.
- **Outside slice** — submitting a bot webhook unchanged needs no confirmation — WS9.
- **Deferred** — Google re-auth confirms a Google-only user and continues — WS14g adapter + WS9 integration.
- **Deferred** — Google re-auth with a different Google account is rejected — WS14g adapter + WS9 integration.
- **Ported** — Google re-auth forces a fresh Google login — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete pinned Rails request, flash, identities, password, session markers and audits).
- **Deferred** — Google re-auth with a stale Google login is rejected — WS14g adapter + WS9 integration.
- **Deferred** — Google re-auth without an auth_time is rejected — WS14g adapter + WS9 integration.
- **Ported** — Google confirmation is unavailable without a linked identity — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete pinned Rails request, flash, identities, password, session markers and audits).
- **Outside slice** — confirmation attempts are rate limited — WS9.
- **Outside slice** — the confirmation limit lives in the shared rate-limit store, not per-process memory — WS9.

## test/controllers/users/google_sign_in_links_controller_test.rb

- **Partial** — the profile offers the link and the flow links the verified subject to the signed-in member — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Ported** — the link flow keeps sign-in's domain allowlist — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete pinned Rails request, flash, identities, password, session markers and audits).
- **Ported** — the link flow keeps sign-in's nonce check — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete pinned Rails request, flash, identities, password, session markers and audits).
- **Ported** — a Google account that already signs in as someone else is refused — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete pinned Rails request, flash, identities, password, session markers and audits).
- **Ported** — a member already linked to another subject is refused — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete pinned Rails request, flash, identities, password, session markers and audits).
- **Ported** — a link flow finished by a different signed-in member links nobody — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete pinned Rails request, flash, identities, password, session markers and audits).
- **Ported** — a link flow whose member signed out does not sign anyone in — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete pinned Rails request, flash, identities, password, session markers and audits).
- **Partial** — starting a link requires a signed-in member and CSRF protection — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.

## test/integration/drive_picker_test.rb

- **Deferred** — composer omits the Drive menu item without Drive consent — WS14g continuation.
- **Deferred** — composer carries the Drive menu item with the Drive scope — WS14g continuation.

## test/integration/drive_share_picker_test.rb

- **Deferred** — composer carries a single enhanced Drive menu item when sharing is configured — WS14g continuation.
- **Deferred** — enhanced menu item needs no Drive consent and wins over the legacy picker — WS14g continuation.
- **Deferred** — composer falls back to the legacy picker when sharing is not configured — WS14g continuation.
- **Deferred** — composer omits every Drive menu item without sharing or Drive consent — WS14g continuation.
- **Deferred** — signed-out visitors see no share metas or buttons — WS14g continuation.

## test/integration/ooo_dm_notice_test.rb

- **Outside slice** — a DM with an OOO member shows the notice above the composer — WS17.
- **Outside slice** — the notice escapes the member's note — WS17.
- **Outside slice** — the notice renders per viewer, never from a shared fragment — WS17.
- **Outside slice** — a group DM shows one line per OOO recipient — WS17.
- **Outside slice** — a channel shows no notice even while a member is out — WS17.
- **Outside slice** — a DM with nobody out shows no notice — WS17.
- **Outside slice** — an invisible member's manual OOO shows no notice — WS17.
- **Outside slice** — an invisible member's calendar OOO shows no notice — WS17.
- **Outside slice** — an invisible member's OOO flip broadcasts an emptied notice line — WS17.
- **Outside slice** — an OOO end broadcasts an emptied notice line — WS17.

## test/jobs/calendar/disconnect_cleanup_job_test.rb

- **Deferred** — deletes every remote copy then revokes the grant — WS14g continuation.
- **Deferred** — refreshes an expired access token from the snapshot — WS14g continuation.
- **Deferred** — an invalid_grant skips deletes but still revokes — WS14g continuation.
- **Deferred** — permanent delete failures are best effort and still revoke — WS14g continuation.
- **Deferred** — a refresh 503 retries without revoking until the deletes succeed — WS14g continuation.
- **Deferred** — a transient delete failure schedules a retry without revoking — WS14g continuation.
- **Deferred** — a revoke 5xx schedules a retry — WS14g continuation.
- **Deferred** — an exhausted retry logs the failure at error level — WS14g continuation.
- **Deferred** — an exhausted retry reports to the error service with the account id — WS14g continuation.
- **Deferred** — a missing id list still revokes the grant — WS14g continuation.
- **Deferred** — blank credentials are a no-op — WS14g continuation.
- **Deferred** — an unreadable credentials blob logs a warning with the account id — WS14g continuation.
- **Deferred** — tokens never reach the job logs — WS14g continuation.

## test/jobs/calendar/inbound_sync_job_test.rb

- **Ported** — a copy cancelled in Google declines the event locally — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (cancelled_going + head/follower series states), through WS14e respond.
- **Ported** — a copy deleted in Google declines the event locally — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (deleted_maybe).
- **Ported** — a confirmed copy never flips a local decline back to going — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (confirmed_declined; response and update stamp).
- **Ported** — a confirmed copy leaves a going response alone — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (confirmed_going; attendance updated_at starts sixty seconds earlier and stays unchanged).
- **Ported** — cancelled events are never touched — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (cancelled_local; zero Google requests).
- **Ported** — nothing happens without a usable account — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (missing_account, disconnected, wrong_scope).
- **Ported** — a revoked grant aborts the sweep instead of failing every entry — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (revoked_aborts; one entry request and invalid_grant disconnect).
- **Partial** — the sweep preloads events instead of querying per entry — the native joined projection batches upcoming entries without per-entry Event loading; the SQL-read-count regression remains WS14g work.

## test/jobs/calendar/meet_link_job_test.rb

- **Ported** — provisions a Meet link through the organizer's calendar copy — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (provisions; exact POST/PATCH payloads, no PUT).
- **Ported** — nothing happens without a request — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (not_requested).
- **Ported** — no Meet link unless the organizer has connected Google — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (organizer_unconnected).
- **Ported** — a disconnected organizer account provisions nothing — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (organizer_disconnected).
- **Ported** — a second run is a no-op once the link exists — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (existing_link; same pre-existing-link setup as Rails).
- **Ported** — a permanent Google refusal is logged, not raised — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (patch_refused; no exception, nil link and request flag retained, matching the Rails assertions).
- **Ported** — a pending conference without a link raises for the job to retry — `app::google_consumer_tests::google_consumers_match_every_recorded_rails_state_and_request` (pending; exact Unavailable, nil link and request flag retained).
- **Ported** — a pending conference schedules a job retry with backoff — `app::google_consumer_tests::google_pending_meet_conference_uses_the_real_durable_retry_handler`; actual runner persists the original args, first group count, and Rails polynomial delay.
- **Outside slice** — creating an event with a request enqueues provisioning — WS14e owns the Event producer. WS14g consumes its exact class and payload; see WS14e tests.
- **Outside slice** — a series copies the request to every occurrence — WS14e owns recurrence and Event producers; WS14g consumes the emitted jobs.

## test/jobs/calendar/remote_delete_job_test.rb

- **Deferred** — deletes the remote copy with the user's current credentials — WS14g continuation.
- **Deferred** — skips a missing account without a request — WS14g continuation.
- **Deferred** — skips a disconnected account without a request — WS14g continuation.
- **Deferred** — skips a grant without the calendar scope — WS14g continuation.
- **Deferred** — treats gone and revoked copies as success — WS14g continuation.
- **Deferred** — an invalid_grant disconnects and counts as success — WS14g continuation.
- **Deferred** — a transient failure schedules a retry — WS14g continuation.
- **Deferred** — other Google failures are logged without raising or retrying — WS14g continuation.

## test/jobs/calendar/sync_entry_job_test.rb

- **Deferred** — going creates one Google event with the expected payload — WS14g continuation.
- **Deferred** — an event with a venue syncs its location and join line — WS14g continuation.
- **Deferred** — events without an end default to one hour — WS14g continuation.
- **Deferred** — changing to maybe keeps the entry — WS14g continuation.
- **Deferred** — declined deletes the entry — WS14g continuation.
- **Deferred** — an event time change patches the same id — WS14g continuation.
- **Deferred** — cancellation deletes the entry — WS14g continuation.
- **Deferred** — leaving the room deletes the entry — WS14g continuation.
- **Deferred** — a user without a connection never triggers a request — WS14g continuation.
- **Deferred** — a disconnected account never triggers a request — WS14g continuation.
- **Deferred** — an invalid_grant during sync disconnects and drops the local entry — WS14g continuation.
- **Deferred** — two runs for the same state make no second insert — WS14g continuation.
- **Deferred** — google event ids are deterministic per event and user and use Google's charset — WS14g continuation.
- **Deferred** — concurrent first runs share one id and converge through the conflict path — WS14g continuation.
- **Deferred** — a 409 on insert falls back to updating the same id — WS14g continuation.
- **Deferred** — an update 404 falls back to inserting the same id — WS14g continuation.
- **Deferred** — delete treats a Google 404 as deleted — WS14g continuation.
- **Deferred** — a failed insert records last_error without raising — WS14g continuation.
- **Deferred** — a transport failure records last_error and enqueues a retry — WS14g continuation.
- **Deferred** — the reconciler re-raises transient failures for the job to retry — WS14g continuation.
- **Deferred** — a 429 schedules a retry instead of parking the entry — WS14g continuation.
- **Deferred** — a quota 403 schedules a retry — WS14g continuation.
- **Deferred** — a permission 403 records last_error without retrying — WS14g continuation.
- **Deferred** — a delete transport failure keeps the row and enqueues a retry — WS14g continuation.
- **Deferred** — going again after declining resurrects the remote copy as confirmed — WS14g continuation.
- **Deferred** — delete treats a Google 410 as deleted — WS14g continuation.
- **Deferred** — a grant without the calendar scope never triggers a request — WS14g continuation.
- **Deferred** — losing the calendar scope drops the entry without a request — WS14g continuation.
- **Deferred** — an unreadable token drops the entry without a request — WS14g continuation.
- **Deferred** — a reconciled decline does not enqueue a remote delete — WS14g continuation.
- **Deferred** — a failed delete keeps the row with last_error for a retry — WS14g continuation.
- **Deferred** — missing records are a no-op — WS14g continuation.
- **Deferred** — responding going on the first event of a series syncs every occurrence — WS14g continuation.
- **Deferred** — creating an attendance enqueues a sync — WS14g continuation.
- **Deferred** — changing a response enqueues a sync but other saves do not — WS14g continuation.
- **Deferred** — updating event times enqueues syncs for connected going/maybe attendees — WS14g continuation.
- **Deferred** — an update inside a transaction enqueues only after commit — WS14g continuation.
- **Deferred** — setting or clearing the venue enqueues a sync, like a title change — WS14g continuation.
- **Deferred** — updating only the title enqueues a sync, an unchanged save does not — WS14g continuation.
- **Deferred** — cancelling enqueues a sync for every entry — WS14g continuation.
- **Deferred** — destroying a membership enqueues syncs for that room's entries only — WS14g continuation.
- **Deferred** — destroying a room enqueues remote deletes for its entries — WS14g continuation.
- **Deferred** — shrinking a series enqueues remote deletes for destroyed occurrences — WS14g continuation.

## test/models/calendar/meeting_cache_test.rb

- **Outside slice** — in_meeting? is true inside an interval, with an inclusive start and exclusive end — WS17 claims/readers.
- **Outside slice** — in_meeting? is false without intervals — WS17 claims/readers.
- **Outside slice** — in_meeting? ignores malformed pairs instead of raising — WS17 claims/readers.
- **Outside slice** — quiet_window_epochs returns epoch windows and skips malformed pairs — WS17 claims/readers.
- **Outside slice** — in_ooo? is true inside an OOO interval, with an inclusive start and exclusive end — WS17 claims/readers.
- **Outside slice** — in_ooo? reads only the OOO intervals, not the busy ones — WS17 claims/readers.
- **Outside slice** — ooo_end_covering returns the latest covering end — WS17 claims/readers.
- **Outside slice** — ooo_end_covering is nil while uncovered — WS17 claims/readers.
- **Outside slice** — ooo_window_epochs returns epoch windows and skips malformed pairs — WS17 claims/readers.
- **Outside slice** — claim_broadcast! wins the first claim and each flip, and loses re-runs — WS17 claims/readers.
- **Outside slice** — claim_broadcast! lets only one concurrent claimant win — WS17 claims/readers.
- **Deferred** — one cache per user — WS14g cache persistence.
- **Outside slice** — the cache row references its member with a cascading foreign key — WS17 claims/readers.

## test/models/calendar/meeting_dispatcher_test.rb

- **Outside slice** — a meeting start broadcasts the badge — WS17.
- **Outside slice** — a re-run with no flip broadcasts nothing — WS17.
- **Outside slice** — a steady-state tick issues no claim write — WS17.
- **Outside slice** — a meeting end broadcasts the badge — WS17.
- **Outside slice** — a broadcast carries the meeting label — WS17.
- **Outside slice** — a stale cache enqueues a refresh — WS17.
- **Outside slice** — a missing cache enqueues a refresh without broadcasting — WS17.
- **Outside slice** — a fresh cache enqueues nothing — WS17.
- **Outside slice** — members who never opted in are ignored — WS17.
- **Outside slice** — deactivated members are ignored — WS17.
- **Outside slice** — one failing member does not stop the sweep — WS17.

## test/models/calendar/meeting_intervals_test.rb

- **Deferred** — derives busy intervals from timed events — WS14g continuation.
- **Deferred** — cancelled events never count — WS14g continuation.
- **Deferred** — out-of-office events never count as busy — WS14g continuation.
- **Deferred** — focus-time events never count as busy — WS14g continuation.
- **Deferred** — events declined by the member never count — WS14g continuation.
- **Deferred** — transparent (show-as-free) events never count — WS14g continuation.
- **Deferred** — all-day events never count — WS14g continuation.
- **Deferred** — tentative and needs-action events count as busy — WS14g continuation.
- **Deferred** — organizer-only events without attendees count as busy — WS14g continuation.
- **Deferred** — malformed items are skipped without raising — WS14g continuation.
- **Deferred** — a declined event from another attendee still counts — WS14g continuation.

## test/models/calendar/meeting_refresh_test.rb

- **Deferred** — a successful refresh stores busy intervals and clears the error — WS14g continuation.
- **Deferred** — only free/busy fields are requested: no titles or attendee identities — WS14g continuation.
- **Deferred** — a member who never opted in is skipped without a request — WS14g continuation.
- **Deferred** — a deactivated member is skipped without a request — WS14g continuation.
- **Deferred** — a member without a usable account records a connect notice and clears intervals — WS14g continuation.
- **Deferred** — a revoked grant records a reconnect notice and turns the status off — WS14g continuation.
- **Deferred** — rate limits keep the last good intervals and record a retry notice — WS14g continuation.
- **Deferred** — a quota 403 keeps the last good intervals and records a retry notice — WS14g continuation.
- **Deferred** — a server error keeps the last good intervals and records a retry notice — WS14g continuation.
- **Deferred** — a malformed response body keeps the last good intervals and records a retry notice — WS14g continuation.
- **Deferred** — a JSON parse failure escaping the client is recorded instead of raising — WS14g continuation.
- **Deferred** — a fresh cache is not refetched — WS14g continuation.
- **Deferred** — a throttled refresh enqueues one delayed follow-up — WS14g continuation.
- **Deferred** — a second throttled refresh inside the window enqueues no further follow-up — WS14g continuation.
- **Deferred** — a completed fetch clears the follow-up claim — WS14g continuation.
- **Deferred** — an OOO-only member's refresh stores OOO intervals with the wider lookahead — WS14g continuation.
- **Deferred** — a member with both opt-ins stores both interval sets — WS14g continuation.
- **Deferred** — a meeting-only member stores no OOO intervals — WS14g continuation.
- **Deferred** — a meeting-only member fetches the meeting lookahead — WS14g continuation.
- **Deferred** — a server error keeps OOO intervals too — WS14g continuation.
- **Deferred** — a revoked grant clears both meeting and OOO intervals — WS14g continuation.

## test/models/calendar/ooo_dispatcher_test.rb

- **Outside slice** — a manual OOO start broadcasts the badge and the DM notice — WS17.
- **Outside slice** — a re-run with no flip broadcasts nothing — WS17.
- **Outside slice** — a steady-state tick issues no claim write — WS17.
- **Outside slice** — an OOO end broadcasts and clears the manual columns — WS17.
- **Outside slice** — the broadcasts carry the OOO label, the note, and the return date — WS17.
- **Outside slice** — a calendar OOO start broadcasts the badge and the notice — WS17.
- **Outside slice** — a stale OOO-only cache enqueues a refresh — WS17.
- **Outside slice** — a member with both opt-ins refreshes through the meeting dispatcher only — WS17.
- **Outside slice** — a missing cache enqueues a refresh without broadcasting — WS17.
- **Outside slice** — members with neither a manual OOO nor the calendar opt-in are ignored — WS17.
- **Outside slice** — deactivated members are ignored — WS17.
- **Outside slice** — one failing member does not stop the sweep — WS17.

## test/models/calendar/ooo_intervals_test.rb

- **Deferred** — derives intervals from timed out-of-office events — WS14g continuation.
- **Deferred** — ordinary events never count, even timed ones — WS14g continuation.
- **Deferred** — cancelled out-of-office events never count — WS14g continuation.
- **Deferred** — all-day out-of-office events resolve in the member's zone — WS14g continuation.
- **Deferred** — malformed items are skipped without raising — WS14g continuation.

## test/models/calendar/push_channel_test.rb

- **Deferred** — watching is disabled without a callback URL — WS14g continuation.
- **Deferred** — watching is disabled without a usable account — WS14g continuation.
- **Deferred** — watch_for opens a channel and stores only the token digest — WS14g continuation.
- **Deferred** — watch_for retires the previous channel only after the new watch succeeds — WS14g continuation.
- **Deferred** — a failed re-watch leaves the old channel alive with no gap — WS14g continuation.
- **Deferred** — token matching is constant-time against the digest — WS14g continuation.
- **Deferred** — notification claims dedupe by message number — WS14g continuation.
- **Deferred** — renew_expiring renews soon-expiring channels and leaves fresh ones — WS14g continuation.
- **Deferred** — renew_expiring drops channels whose account went away — WS14g continuation.
- **Deferred** — renew_expiring opens a channel for a connected account missing one — WS14g continuation.
- **Deferred** — renew_expiring opens nothing for a disconnected account missing one — WS14g continuation.
- **Deferred** — renew_expiring never raises — WS14g continuation.
- **Deferred** — renew_expiring preloads users instead of querying per channel — WS14g continuation.

## test/models/drive_attachment_test.rb

- **Deferred** — a message with attachments and no text is valid — WS14g continuation.
- **Deferred** — a textless message without attachments is still invalid — WS14g continuation.
- **Deferred** — an invalid file id is rejected — WS14g continuation.
- **Deferred** — duplicate file ids on one message collapse to a single row — WS14g continuation.
- **Deferred** — the same file id may attach to different messages — WS14g continuation.
- **Deferred** — the 11th attachment is rejected — WS14g continuation.
- **Deferred** — destroying the message destroys its attachments — WS14g continuation.
- **Deferred** — url is the open link for the file id — WS14g continuation.

## test/models/event_calendar_entry_test.rb

- **Deferred** — destroying an entry enqueues its remote delete — WS14g continuation.
- **Deferred** — delete skips the remote delete for already-reconciled rows — WS14g continuation.
- **Deferred** — delete_all skips remote deletes for disconnect cleanup — WS14g continuation.

## test/models/google/client_test.rb

- **Deferred** — configured? requires both client id and secret — WS14g continuation.
- **Deferred** — authorize_url carries the calendar scope, offline access, and state — WS14g continuation.
- **Deferred** — refreshes an expired access token before calling — WS14g continuation.
- **Deferred** — retries once after a 401 cured by a refresh — WS14g continuation.
- **Deferred** — a 401 that survives refresh raises Unauthorized — WS14g continuation.
- **Deferred** — invalid_grant marks the account disconnected and raises Unauthorized — WS14g continuation.
- **Deferred** — a failed refresh without invalid_grant raises Error and keeps the connection — WS14g continuation.
- **Deferred** — 404 maps to NotFound — WS14g continuation.
- **Deferred** — 409 maps to Conflict — WS14g continuation.
- **Deferred** — other API failures map to Error — WS14g continuation.
- **Deferred** — a timeout maps to Unavailable — WS14g continuation.
- **Deferred** — a malformed response body maps to Unavailable — WS14g continuation.
- **Deferred** — email_from_id_token returns the verified email — WS14g continuation.
- **Deferred** — email_from_id_token accepts the short issuer — WS14g continuation.
- **Deferred** — email_from_id_token rejects a missing or malformed token — WS14g continuation.
- **Deferred** — email_from_id_token rejects a wrong issuer, audience, expiry, or missing email — WS14g continuation.
- **Deferred** — exchange_code returns the token response — WS14g continuation.
- **Deferred** — a failed code exchange raises Error — WS14g continuation.
- **Deferred** — authorize_url with drive requests Calendar plus the per-file Drive scope — WS14g continuation.
- **Deferred** — authorize_url without drive omits the Drive scope — WS14g continuation.
- **Deferred** — drive_file fetches metadata with the Drive fields — WS14g continuation.
- **Deferred** — drive_file refreshes an expired access token first — WS14g continuation.
- **Deferred** — drive_file maps 403 and 404 to NotFound — WS14g continuation.
- **Deferred** — drive_file maps a timeout to Unavailable — WS14g continuation.
- **Deferred** — list_drive_files requests the recent list with the Drive list parameters — WS14g continuation.
- **Deferred** — list_drive_files searches by name and escapes quotes and backslashes — WS14g continuation.
- **Deferred** — list_drive_files treats a blank query as a recent list — WS14g continuation.
- **Deferred** — list_drive_files refreshes an expired access token first — WS14g continuation.
- **Deferred** — 429 maps to RateLimited, which retries as Unavailable — WS14g continuation.
- **Deferred** — a 403 carrying a quota reason maps to RateLimited — WS14g continuation.
- **Deferred** — a 403 with a permission reason maps to Forbidden — WS14g continuation.
- **Deferred** — a 403 with an unparseable body maps to Forbidden — WS14g continuation.
- **Deferred** — 410 maps to NotFound — WS14g continuation.
- **Deferred** — connection failures map to Unavailable — WS14g continuation.
- **Deferred** — a dropped connection maps to Unavailable — WS14g continuation.
- **Deferred** — a 429 on refresh raises Unavailable and keeps the connection — WS14g continuation.
- **Deferred** — a Drive 429 maps to RateLimited — WS14g continuation.
- **Deferred** — revoke_token posts the token and accepts success or already-revoked — WS14g continuation.
- **Deferred** — revoke_token raises Unavailable on a 5xx so the caller retries — WS14g continuation.
- **Deferred** — revoke_token raises Unavailable on a 429 so the caller retries — WS14g continuation.
- **Deferred** — revoke_token returns false on other client errors — WS14g continuation.
- **Deferred** — revoke_token maps transport failures to Unavailable without the token — WS14g continuation.
- **Deferred** — an unreadable token marks the account disconnected and raises Unauthorized — WS14g continuation.
- **Deferred** — list_events queries a single-event window with the free/busy fields mask — WS14g continuation.
- **Deferred** — list_events follows nextPageToken and merges pages — WS14g continuation.
- **Deferred** — list_events stops paging at the cap — WS14g continuation.
- **Deferred** — a 429 on list_events raises RateLimited — WS14g continuation.
- **Deferred** — a quota 403 on list_events raises RateLimited — WS14g continuation.
- **Deferred** — a revoked grant on list_events disconnects and raises Unauthorized — WS14g continuation.

## test/models/google/drive_link_test.rb

- **Ported domain case** — docs editor URLs — models::google_drive_link::tests::all_pinned_rails_url_and_id_vectors.
- **Ported domain case** — drive file, open, and folder URLs — models::google_drive_link::tests::all_pinned_rails_url_and_id_vectors.
- **Ported domain case** — account switcher prefixes — models::google_drive_link::tests::all_pinned_rails_url_and_id_vectors.
- **Ported domain case** — negatives — models::google_drive_link::tests::all_pinned_rails_url_and_id_vectors.
- **Ported domain case** — valid_id? accepts bare file ids only — models::google_drive_link::tests::all_pinned_rails_url_and_id_vectors.

## test/models/google/picker_test.rb

- **Deferred** — configured only when all three public values are present — WS14g continuation.
- **Deferred** — blank values count as missing — WS14g continuation.

## test/models/google/sign_in/account_linker_test.rb

- **Ported domain case** — subject collision through a lost race still resolves to the subject owner — models::google_identity::tests (subject, escaped predecessors, names, trust markers).
- **Ported domain case** — deactivated predecessor match escapes LIKE wildcards — models::google_identity::tests (subject, escaped predecessors, names, trust markers).
- **Ported domain case** — provisioned name falls back from claims to the email local part — models::google_identity::tests (subject, escaped predecessors, names, trust markers).
- **Ported domain case** — a self-changed email is refused for linking and never provisions a duplicate — models::google_identity::tests (subject, escaped predecessors, names, trust markers).
- **Ported domain case** — an already-linked subject signs in even after its account self-changed email — models::google_identity::tests (subject, escaped predecessors, names, trust markers).

## test/models/google/sign_in/key_store_test.rb

- **Ported domain case** — caches keys across lookups — integrations::google::sign_in::tests (cache, rotation, expiry, real timeout, malformed responses).
- **Ported domain case** — refetches once on an unknown kid and fails closed — integrations::google::sign_in::tests (cache, rotation, expiry, real timeout, malformed responses).
- **Ported domain case** — outage raises Unavailable — integrations::google::sign_in::tests (cache, rotation, expiry, real timeout, malformed responses).
- **Ported domain case** — non-JSON and keyless answers raise Unavailable — integrations::google::sign_in::tests (cache, rotation, expiry, real timeout, malformed responses).

## test/models/google/sign_in_test.rb

- **Ported domain case** — missing domain configuration disables sign-in without company defaults — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — allowed_domains honors explicit configuration — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — allowed_domains drops invalid entries — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — explicitly empty domains disables sign-in — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — configured? requires client credentials and domains — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — authorize_url requests identity-only scopes with nonce and PKCE — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — authorize_url sends prompt and max_age only when asked (sudo re-auth) — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — pkce_pair produces a matching verifier and challenge — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — safe_return_path keeps relative paths and same-host URLs — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — safe_return_path rejects off-origin and malformed values — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — exchange_code returns only the id_token — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).
- **Ported domain case** — exchange_code raises Rejected on denial and Unavailable on outage — integrations::google::sign_in::tests (configuration, URLs, PKCE, returns, real sockets, timeout, exchange).

## test/models/google_account_test.rb

- **Partial** — one account per user — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — tokens round-trip encrypted at rest — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — drive? reflects the stored scopes — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — connected, usable, and expiry predicates — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — calendar? treats blank scopes as granted and requires calendar.events otherwise — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — an unreadable token reads as unusable and marks the account disconnected — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.
- **Partial** — cleanup_snapshot returns an encrypted blob, or nil when unreadable — WS14g continuation; controller/domain implementation exists with representative native tests; the full named Rails case remains to be mapped.

## test/models/user/meeting_status_test.rb

- **Outside slice** — meeting status and quiet-during-meetings default off — WS17.
- **Outside slice** — in_meeting? needs the opt-in and a covering interval — WS17.
- **Outside slice** — in_meeting? is false without a cache row — WS17.
- **Outside slice** — the meeting label shows while in a meeting — WS17.
- **Outside slice** — a custom status wins over the meeting label — WS17.
- **Outside slice** — an expired custom status yields to the meeting label — WS17.
- **Outside slice** — manual DND wins over the meeting label — WS17.
- **Outside slice** — the DND presence wins over the meeting label — WS17.
- **Outside slice** — quiet hours win over the meeting label — WS17.
- **Outside slice** — invisible hides the meeting label — WS17.
- **Outside slice** — quiet-during-meetings never suppresses the meeting label — WS17.
- **Outside slice** — quiet-during-meetings only works while meeting status is on — WS17.
- **Outside slice** — quiet-during-meetings applies through a custom status — WS17.
- **Outside slice** — quiet-during-meetings is off outside busy intervals — WS17.

## test/models/user/out_of_office_test.rb

- **Outside slice** — out of office defaults off — WS17.
- **Outside slice** — a manual OOO is active until its end, then reads as off — WS17.
- **Outside slice** — setting an OOO end in the past is invalid, but an expired end left behind still saves — WS17.
- **Outside slice** — a note longer than 140 characters is invalid — WS17.
- **Outside slice** — the status line names the return date and the note — WS17.
- **Outside slice** — the return date renders in the OOO member's own zone — WS17.
- **Outside slice** — OOO wins over a custom status, DND, and the meeting label — WS17.
- **Outside slice** — invisible hides the OOO label but OOO still reads as active — WS17.
- **Outside slice** — OOO quiet never suppresses the OOO label — WS17.
- **Outside slice** — calendar OOO needs the opt-in and a covering interval — WS17.
- **Outside slice** — a calendar OOO outside its intervals reads as off — WS17.
- **Outside slice** — overlapping manual and calendar OOO show the later end — WS17.
- **Outside slice** — the note shows only while the manual OOO is active — WS17.
- **Outside slice** — OOO presets run to the end of the day in the member's zone — WS17.
- **Outside slice** — the Monday preset is a week out on Mondays — WS17.
- **Outside slice** — the custom preset parses a datetime-local value in the member's zone — WS17.
- **Outside slice** — an unknown preset raises — WS17.
- **Outside slice** — claim_ooo_broadcast! wins the first claim and each flip, and loses re-runs — WS17.
- **Outside slice** — claiming an end clears the expired manual columns — WS17.
- **Outside slice** — claiming an end keeps a manual OOO set racing the sweep — WS17.
- **Outside slice** — OOO quiets notifications unless the member keeps them on — WS17.
- **Outside slice** — deactivating clears the manual OOO columns — WS17.

## test/system/drive_attachments_test.rb

- **Deferred** — attach Drive files from the picker, send textless, and remove through edit — WS14g continuation.
- **Deferred** — edit a room message in the composer and remove one of two attachments — WS14g continuation.
- **Deferred** — attach a Drive file from the thread composer — WS14g continuation.

## test/system/drive_link_previews_test.rb

- **Deferred** — a viewer with the Drive scope sees a picked file upgraded to a preview chip — WS14g continuation.
- **Deferred** — a viewer with the Drive scope keeps a plain chip for a file never picked — WS14g continuation.
- **Deferred** — a viewer without the Drive scope sees a plain chip and fetches nothing — WS14g continuation.
- **Deferred** — composer Drive picker inserts the chosen file link at the caret — WS14g continuation.
- **Deferred** — composer omits the Drive menu item without the Drive scope — WS14g continuation.

## test/system/drive_share_test.rb

- **Deferred** — review dialog offers attach-only and an explicit grant with names and emails — WS14g continuation.
- **Deferred** — attach-only pins the chip and writes no Drive permissions — WS14g continuation.
- **Deferred** — grant validates, preserves writers, and creates only missing readers — WS14g continuation.
- **Deferred** — partial failure reports per recipient and retries only outstanding grants — WS14g continuation.
- **Deferred** — cancelled picker selection shares nothing and can be retried — WS14g continuation.
- **Deferred** — cancelled Google authorization shares nothing and can be retried — WS14g continuation.
- **Deferred** — picker cancel returns quietly and the drive button works again — WS14g continuation.
- **Deferred** — closed consent popup returns quietly to the composer — WS14g continuation.
- **Deferred** — consent popup closed via GIS error callback returns quietly — WS14g continuation.
- **Deferred** — real error dialog closes with the Close button and the drive button works again — WS14g continuation.
- **Deferred** — real error dialog closes with Esc — WS14g continuation.
- **Deferred** — try again re-opens the picker after a real error — WS14g continuation.
- **Deferred** — script load failure offers retry without hanging the composer — WS14g continuation.
- **Deferred** — first use loads scripts then continues on a fresh gesture — WS14g continuation.
- **Deferred** — unshareable file disables the grant but keeps attach-only — WS14g continuation.
- **Deferred** — folder selection disables the grant but keeps attach-only — WS14g continuation.
- **Deferred** — expired Google session reconnects on an explicit gesture and continues — WS14g continuation.
- **Deferred** — grant rejects a recipient who left mid-review and refreshes the list — WS14g continuation.
- **Deferred** — thread composer grants against the parent room membership — WS14g continuation.
- **Deferred** — navigation disposes the dialog, token, and picker state — WS14g continuation.
- **Deferred** — mobile viewport keeps the review dialog usable — WS14g continuation.
- **Deferred** — grant requires fresh review when a recipient email changes — WS14g continuation.
- **Deferred** — changed identity can be re-approved against the new email — WS14g continuation.
- **Deferred** — retry revalidates membership before writing — WS14g continuation.
- **Deferred** — reconnect revalidates before resuming grants — WS14g continuation.
- **Deferred** — cancelled authorization invalidates a delayed token callback — WS14g continuation.
- **Deferred** — cancelled picker invalidates a delayed selection callback — WS14g continuation.
- **Deferred** — grant is blocked when attachments are already full — WS14g continuation.
- **Deferred** — mid-flight capacity loss preserves grant outcomes — WS14g continuation.
- **Deferred** — rate-limited grants are classified and retry cleanly — WS14g continuation.
- **Deferred** — dialog checkboxes are visible and long names wrap — WS14g continuation.
- **Deferred** — retry does not write while attachment capacity is unavailable — WS14g continuation.
- **Deferred** — completed outcomes stay visible through reconnect and refreshed review — WS14g continuation.
- **Deferred** — reconciled access is confirmed, not claimed as newly granted — WS14g continuation.
- **Deferred** — existing and reconciled access are distinguished with no new grants — WS14g continuation.
- **Deferred** — server errors are classified as service failures, not policy denials — WS14g continuation.

## test/system/drive_teardown_race_test.rb

- **Deferred** — an in-flight Drive metadata fetch completes against its own stubs at teardown — WS14g continuation.

## test/system/meeting_status_test.rb

- **Outside slice** — opting in shows In a meeting for a stubbed busy interval, then clears after it ends — WS17.
- **Outside slice** — the profile links to connect without a Google account — WS17.

## test/system/out_of_office_test.rb

- **Outside slice** — set OOO until tomorrow, badge and DM notice show for another user, then clear it — WS17.

## test/system/sudo_mode_test.rb

- **Outside slice** — one prompt, then the action continues automatically — WS9.
- **Outside slice** — a wrong password keeps the action gated — WS9.

## test/controllers/audit_log/sign_in_audit_test.rb (Google/Calendar/Drive cases only)

- **Deferred** — Google sign-in success is recorded — WS14g audit producer + WS9 integration.
- **Deferred** — Google sign-in that provisions a user records the creation — WS14g audit producer + WS9 integration.
- **Deferred** — Google sign-in that auto-links an allowed address records the link — WS14g audit producer + WS9 integration.
- **Deferred** — repeat Google sign-in records no link or creation row — WS14g audit producer + WS9 integration.
- **Deferred** — rejected Google sign-in is recorded as a failure — WS14g audit producer + WS9 integration.

## test/controllers/two_factor/reauthentications_controller_test.rb (Google/Calendar/Drive cases only)

- **Ported** — create redirects linked members to Google — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete callback/audit plus real protected actions; 599/600/601-second consumption boundary).
- **Ported** — create refuses members without a linked Google account — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete callback/audit plus real protected actions; 599/600/601-second consumption boundary).
- **Ported** — create refuses when Google is not configured — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete callback/audit plus real protected actions; 599/600/601-second consumption boundary).
- **Ported** — create forces a fresh Google sign-in — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete callback/audit plus real protected actions; 599/600/601-second consumption boundary).
- **Ported** — callback refuses a different Google account — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete callback/audit plus real protected actions; 599/600/601-second consumption boundary).
- **Ported** — callback refuses a stale Google authentication — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete callback/audit plus real protected actions; 599/600/601-second consumption boundary).

## test/controllers/two_factor/remembered_devices_controller_test.rb (Google/Calendar/Drive cases only)

- **Ported** — destroy_all accepts a completed Google re-auth — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete callback/audit plus real protected actions; 599/600/601-second consumption boundary).

## test/controllers/two_factor/setups_controller_test.rb (Google/Calendar/Drive cases only)

- **Ported** — destroy accepts a completed Google re-auth — `app::google_tests::parity_cases::google_controller_cases_match_complete_pinned_rails_observations` (complete callback/audit plus real protected actions; 599/600/601-second consumption boundary).

## test/controllers/users/profiles_two_factor_test.rb (Google/Calendar/Drive cases only)

- **Deferred** — profile offers Google confirmation to linked members — WS14g adapter + WS9 profile.
- **Deferred** — profile hides Google confirmation without a linked account — WS14g adapter + WS9 profile.

## test/controllers/users/profiles_controller_test.rb (Google/Calendar/Drive cases only)

- **Deferred** — profile shows Google Calendar as not configured without credentials — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile links to connect for meeting status without an account — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile offers the meeting toggle for a connected account — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile shows the meeting fetch notice when a refresh failed — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile asks to reconnect for meeting status left on after disconnect — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile lists the quiet-during-meetings switch — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — the layout sends meeting windows for the live sound gate — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — the layout sends future meeting windows before the meeting starts — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — the layout sends no meeting windows without cached intervals — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — the layout sends no meeting windows when meeting status itself is off — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — the layout sends future calendar OOO windows before the OOO starts — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — the layout leaves sounds alone for meetings when quiet-during-meetings is off — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile offers a reconnect when Google rejected the connection — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile offers Drive previews for a connected account without the Drive scope — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile shows Drive previews as enabled when the account has the Drive scope — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile offers Drive previews again for the retired metadata grant — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile shows no Drive row when Google is not configured — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile asks to reconnect when the grant lacks the calendar scope — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — profile shows Disconnect for a partial grant with Drive still active — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — reconnect preserves a granted Drive scope — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — reconnect without Drive requests the calendar scope only — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.
- **Deferred** — layout carries the Drive previews meta tag only with the Drive scope — WS14g Calendar/Drive source + WS8b-r profile + WS17 status.


## Continuation coverage and remaining seams

Main was merged at `35c0e65b` to consume WS14e's `CalendarEvent::respond` and `CalendarEvent::save_meet_link`. All seven Calendar classes, including InboundSync and MeetLink, now have registered consumers. The 38 pinned consumer scenarios exercise exact HTTP, responses, attendance timestamps, link/request flags, queue emissions, channel errors and disconnects. Real-runner tests verify declines, provisioning and pending retries. Late enqueue failures roll back domain changes and callback jobs; complete Meet-card and disconnect status/OOO socket frames are compared with Rails after commit. Compound attendance parameters have 24 Rails-generated GET/PATCH vectors with exact opening-frame and hidden-input bytes.

Retry budgets now persist by Rails exception-handler group, survive restarts and reset on manual retry. Six serialized Rails retry sequences cover mixed inherited/Google errors and exhaustion. Native tests cover writer rollback, expired process leases, explicit re-enqueue and terminal cleanup. Rails.error subscriber integration and cleanup account-id error context remain unfinished.

The profile connection/sign-in panels match seven connection and three identity states, Drive chips match empty/populated states, and administrator allow/unlink forms match eleven account rows byte for byte. Complete configured pages and interaction assertions remain WS14g work with the page/layout owners. No browser pixel-diff work is required or listed as remaining.

The WS11 agent credential/authentication seam is now on merged main; it is consumed rather than reimplemented. Drive attachment message submission/JSON/thread broadcasts remain with WS8b-m/m2 and agent polling/REST delivery with WS11-api. Existing Message attachment domain validation/save/touch/cascade code remains shared. Human recipient policy and viewer metadata authorization are implemented; remaining named tests below their respective headings remain explicitly partial/deferred.

The remaining session/link/step-up controller cases include account history/password preservation, domain/rotation changes across HTTP, authenticated replay, deactivation/ban races and the full original audits/page assertions. Existing representative tests do not count as completion of an unmapped Rails case. The SQL-read-count assertion for inbound preloading is also unfinished.

### October 2 continuation

The real combined RSVP/Meet runner now uses a frozen clock, deterministic inbound-scope capture and a change-driven committed queue drain. Its original five-second deadline and normal runner concurrency remain. The application encryption provider now caches its 65536-round derivation per Secrets, matching Rails; repeated derivation was the CPU bottleneck. Thirty trials at eight concurrent tests on four CPUs passed after the fix (5/30 before it).

The new controller-cases probe captures 66 complete Rails request observations, including signed-out/other-member callbacks, domain/nonce/subject checks, signed-in replay, malformed token/key payloads, and strict auth_time cutoff ±1 second for both reauth and sudo. Reauth observations follow the callback and exercise backup rotation (including single consumption), device removal, disabling two factor, wrong credentials, and the 599/600/601-second expiration boundary. Sudo protected-action continuation, whole-page composition and Google-only provisioning transitions still have their own remaining ledger entries. Eight additional controller declarations are now fully mapped; these are outside the primary 611-case count.
