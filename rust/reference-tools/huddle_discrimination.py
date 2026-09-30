#!/usr/bin/env python3
"""Compile deliberate regressions, require assertion failures, then restore every file.

Use only in the owning WS13 worktree, without another Cargo process editing/building it.
Logs stay on disk in .scratch/ws13-discrimination (never /tmp).
"""
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
SCRATCH = ROOT / ".scratch" / "ws13-discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
H = ROOT / "rust/crates/campfire/src/huddle.rs"
L = ROOT / "rust/crates/rails_compat/src/jwt/livekit.rs"
D = ROOT / "rust/crates/db/src/models/huddle_cleanup.rs"
J = ROOT / "rust/crates/campfire/src/jobs/huddle.rs"
G = ROOT / "rust/crates/db/src/models/huddle_grant.rs"
I = ROOT / "rust/crates/campfire/src/controllers/internal_huddle.rs"
E = ROOT / "rust/crates/db/src/models/huddle_effects.rs"
B = ROOT / "rust/crates/campfire/src/channels/huddle_effects.rs"
N = ROOT / "rust/crates/db/src/models/huddle_notices.rs"
V = ROOT / "rust/crates/db/src/models/huddle_invitations.rs"
T = ROOT / "rust/crates/db/src/models/huddle_stream_liveness.rs"
M = ROOT / "rust/crates/db/src/models/membership.rs"
S = ROOT / "rust/crates/db/src/models/stream.rs"
A = ROOT / "rust/crates/db/src/models/stage.rs"
U = ROOT / "rust/crates/db/src/models/user.rs"
W = ROOT / "rust/crates/views/src/huddle_stage.rs"
C = ROOT / "rust/crates/db/src/models/call_moderation.rs"
P = ROOT / "rust/crates/db/src/models/stage_streams.rs"
Q = ROOT / "rust/crates/db/src/models/stage_participation.rs"
R = ROOT / "rust/crates/campfire/src/controllers/rooms/stage_participation.rs"
K = ROOT / "rust/crates/campfire/src/controllers/rooms/call_channels.rs"
O = ROOT / "rust/crates/campfire/src/controllers/rooms.rs"
Z = ROOT / "rust/crates/campfire/src/channels/sink.rs"
F = ROOT / "rust/crates/campfire/src/controllers/rooms/huddles.rs"


def replace_once(source, before, after):
    assert source.count(before) == 1, before
    return source.replace(before, after, 1)


def replace_body(source, marker, body):
    start = source.index("{", source.index(marker))
    depth = 1
    end = start + 1
    # These selected function bodies contain only balanced braces in their strings.
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[:start + 1] + "\n" + body + "\n" + source[end - 1:]


mutations = [
    ("public-huddle-returned-grant-id-corrupted", F, lambda s: replace_once(s, '"grant_id":grant.id', '"grant_id":grant.id + 1'), "campfire", "huddle_controller_reuses_the_actual_opaque_grant_across_post_requests"),
    ("public-huddle-leave-effects-bypassed", F, lambda s: replace_once(s, "for id in ids {", "for id in ids.into_iter().take(0) {"), "campfire", "huddle_controller_leave_keeps_the_other_device_and_sends_one_room_refresh"),
    ("composed-sidebar-call-sections-bypassed", ROOT / "rust/crates/views/templates/users/sidebars/show.html", lambda s: replace_once(s, "{{ self.call_sections()|safe }}", ""), "campfire", "stage_sidebar_live_dot_and_call_sections_follow_current_stream_state"),
    ("composed-stage-publish-hint-bypassed", ROOT / "rust/crates/views/src/rooms/navigation.rs", lambda s: replace_once(s,'.map(|s| s.viewer().role != "listener"),','.map(|_| true),'),"campfire","stage_page_composes_listener_permissions_and_sti_targets"),
    ("composed-edit-github-section-bypassed", ROOT / "rust/crates/views/templates/rooms/calls/_edit.html", lambda s: replace_once(s,'{{ form.github_section(ctx)|safe }}','{{ "" }}'),"campfire","complete_voice_and_stage_form_pages_match_fourteen_rails_renders"),
    ("stream-controller-id-header-bypassed", ROOT / "rust/crates/campfire/src/controllers/rooms/stage_streams.rs", lambda s: replace_once(s,'"X-Stream-Id"','"X-Broken-Stream-Id"'),"campfire","stream_controller_start_stop_and_silent_noop_deliver_exact_rails_fanout"),
    ("call-channel-update-policy-bypassed", K, lambda s: replace_once(s,"    ensure_can_administer(c, &room)?;",""),"campfire","call_channel_updates_deny_unprivileged_members_and_wrong_namespaces"),
    ("call-channel-sole-host-check-bypassed", K, lambda s: replace_once(s,"if room.stage() && has_remaining_ids {","if false && room.stage() && has_remaining_ids {"),"campfire","stage_member_edit_cannot_remove_the_sole_host_or_commit_the_rename"),
    ("call-channel-broadcast-bypassed", K, lambda s: replace_body(s,"async fn broadcast(","Ok(())"),"campfire","call_channel_create_and_member_revision_deliver_ordered_sidebar_and_header_frames"),
    ("call-channel-removal-header-bypassed", Z, lambda s: replace_once(s,"app.config.huddle.configured()","false && app.config.huddle.configured()"),"campfire","call_channel_create_and_member_revision_deliver_ordered_sidebar_and_header_frames"),
    ("call-channel-deletion-seam-bypassed", K, lambda s: replace_once(s,"campfire_db::models::room_delete::begin_destroy(tx, &deleted, &config)?;","deleted.destroy(tx)?;"),"campfire","deleting_a_call_channel_uses_ws8a_marking_and_ends_grants_and_streams_before_reply"),
    ("public-auth-overrides-bypassed", F, lambda s: replace_once(s,"Some(request_authentication),","None,"),"campfire","public_huddle_authentication_errors_precede_csrf_and_configuration"),
    ("public-cache-prepend-bypassed", F, lambda s: replace_once(s,'c.set_header("cache-control", "no-store");',""),"campfire","public_huddle_authentication_errors_precede_csrf_and_configuration"),
    ("public-deleted-room-exposed", F, lambda s: replace_once(s,"if !room.deleted()","if true"),"campfire","public_huddle_http_matches_production_rails"),
    ("public-session-leave-scope-bypassed", F, lambda s: replace_once(s,"session_id=? AND room_id=?", "session_id!=? AND room_id=?"),"campfire","public_huddle_http_matches_production_rails"),
    ("public-in-call-boundary-included", F, lambda s: replace_once(s,"last_seen_at>?", "last_seen_at>=?"),"campfire","public_huddle_http_matches_production_rails"),

    ("stage-note-delivery-bypassed", B, lambda s: replace_body(s,"pub(crate) fn stage_ended_note(","Ok(())"),"campfire","last_host_departure_delivers_a_quiet_note_to_the_room_socket"),
    ("role-rank-bypassed", Q, lambda s: replace_once(s,"&& !administrator\n    {","&& false && !administrator\n    {"),"campfire","stage_role_and_hand_security_keep_grants_and_imported_hands_on_denial"),
    ("role-demotion-without-grant-left-live", Q, lambda s: replace_once(s,"Stream::end_for_membership(tx, room_id, target.id)?;",""),"campfire_db","stage_roles_and_hands_match_thirty_four_rails_controller_scenarios"),
    ("role-personal-panel-bypassed", Q, lambda s: replace_once(s,"tx.emit_after_commit(Event::broadcast(&StagePanel {","let _ = Event::broadcast(&StagePanel {").replace("membership_id: target.id,\n    }));","membership_id: target.id,\n    });",1),"campfire","stage_role_roster_panel_and_single_rejoin_reach_real_sockets_after_commit"),
    ("role-unnecessary-rejoin", Q, lambda s: replace_once(s,"if (before == Some(StageRole::Listener)) != (role == StageRole::Listener) {","if before.is_some() {"),"campfire","stage_role_roster_panel_and_single_rejoin_reach_real_sockets_after_commit"),
    ("hand-repeat-roster-noisy", Q, lambda s: replace_once(s,"if member.raise_hand(tx)? {","if member.raise_hand(tx)? || true {"),"campfire_db","stage_roles_and_hands_match_thirty_four_rails_controller_scenarios"),
    ("hand-rate-limit-bypassed", R, lambda s: replace_once(s,"if count > 10 {","if false && count > 10 {"),"campfire","stage_hand_rate_limit_matches_rails_and_is_per_membership_per_minute"),

    ("moderation-enqueue-failure-swallowed", C, lambda s: replace_once(s,"target.set_server_muted(tx, true, config)?","target.set_server_muted(tx, true, config).unwrap_or(false)"),"campfire_db","moderation_enqueue_failure_rolls_back_mute_revocation_stream_and_frames"),

    ("stream-start-seen-bypassed", P, lambda s: replace_once(s,"if !seen {","if false && !seen {"),"campfire","stage_stream_start_requires_role_host_unmuted_and_seen_grant"),
    ("stream-stale-stop-protection-bypassed", P, lambda s: replace_once(s,"requested_id.is_none_or(|id| id == s.id.to_string())","requested_id.is_none_or(|_|true)"),"campfire","stage_stream_http_start_conflict_and_stale_stop_preserve_the_new_presenter"),

    ("stream-quality-bypassed", S, lambda s: replace_body(s,"fn validate_quality(","Ok(())"),"campfire_db","stream_lifecycle_and_host_departure_match_twenty_rails_scenarios"),
    ("stream-committed-callback-bypassed", S, lambda s: replace_body(s,"fn broadcast_changed(",""),"campfire","stage_stream_callbacks_reach_real_sockets_and_rollback_stays_silent"),
    ("stream-explicit-stopped-bypassed", S, lambda s: replace_once(s,"ended_by.is_some_and(|actor| actor != self.user_id)","ended_by.is_some_and(|_|false)"),"campfire_db","stream_lifecycle_and_host_departure_match_twenty_rails_scenarios"),
    ("stage-admin-successor-bypassed", A, lambda s: replace_once(s,"user.is_active() && user.is_administrator()","false && user.is_active() && user.is_administrator()"),"campfire_db","stream_lifecycle_and_host_departure_match_twenty_rails_scenarios"),
    ("stage-succession-note-noisy", A, lambda s: replace_once(s,"system_note: true","system_note:false"),"campfire_db","stream_lifecycle_and_host_departure_match_twenty_rails_scenarios"),
    ("stream-user-without-grant-left-live", U, lambda s: replace_once(s,"super::stream::Stream::end_for_user(tx,self.id)?;",""),"campfire_db","stream_lifecycle_and_host_departure_match_twenty_rails_scenarios"),
    ("stage-forms-visible-to-listeners", W, lambda s: replace_body(s,"fn can_manage(","true"),"campfire_views","stage_fragments_match_four_hundred_rails_renders"),
    ("moderation-administrator-rank-bypassed", C, lambda s: replace_once(s,"target_user.is_administrator() && !user.is_administrator()","false && target_user.is_administrator() && !user.is_administrator()"),"campfire","call_moderation_security_denies_rank_self_and_outsiders_before_any_write"),

    ("issuance-callback-bypassed", V, lambda s: replace_body(s,"pub(crate) fn after_issued(","Ok(())"), "campfire_db", "issuance_invitations_match_forty_nine_rails_scenarios"),
    ("issuance-banned-caller", G, lambda s: replace_once(s,"JOIN users u ON u.id=s.user_id AND u.status=0 AND u.role!=2","JOIN users u ON u.id=s.user_id AND u.role!=2"),"campfire_db","issuance_invitations_match_forty_nine_rails_scenarios"),
    ("issuance-bot-caller", G, lambda s: replace_once(s,"JOIN users u ON u.id=s.user_id AND u.status=0 AND u.role!=2","JOIN users u ON u.id=s.user_id AND u.status=0"),"campfire_db","issuance_invitations_match_forty_nine_rails_scenarios"),
    ("issuance-off-hidden-recipient", V, lambda s: replace_once(s,"AND (m.involvement IS NULL OR m.involvement NOT IN ('nothing','invisible'))",""),"campfire_db","issuance_invitations_match_forty_nine_rails_scenarios"),
    ("issuance-inbox-suppression-bypassed", V, lambda s: replace_once(s,"if !huddle_notices::invitations_enabled","if false && !huddle_notices::invitations_enabled"),"campfire_db","issuance_invitations_match_forty_nine_rails_scenarios"),
    ("issuance-reused-grant-dedupe-bypassed", V, lambda s: replace_once(s,"previous_issue.is_some_and(|at|at>=dedup)","previous_issue.is_some_and(|_|false)"),"campfire_db","issuance_invitations_match_forty_nine_rails_scenarios"),
    ("issuance-owned-priority-bypassed", V, lambda s: replace_once(s,"if let Some(item) = owned {","if let Some(item) = owned.filter(|_|false) {"),"campfire_db","issuance_invitations_match_forty_nine_rails_scenarios"),
    ("issuance-sound-policy-ignored", V, lambda s: replace_once(s,"Bool(!sound_allowed)","Bool(false)"),"campfire_db","issuance_invitations_match_forty_nine_rails_scenarios"),
    ("resolver-wait-shortened", V, lambda s: s.replace("SignedDuration::from_secs(45)","SignedDuration::from_secs(44)"),"campfire_db","overdue_invitations_match_twenty_nine_rails_scenarios"),
    ("resolver-revoked-join-evidence-ignored", V, lambda s: replace_once(s,"WHERE room_id=? AND user_id=? AND (last_issued_at>=? OR last_seen_at>?)","WHERE room_id=? AND user_id=? AND revoked_at IS NULL AND (last_issued_at>=? OR last_seen_at>?)"),"campfire_db","overdue_invitations_match_twenty_nine_rails_scenarios"),
    ("stale-exact-thirty-kept", T, lambda s: replace_once(s,"last_seen_at>?","last_seen_at>=?"),"campfire_db","stale_stream_state_matches_sixteen_rails_scenarios"),
    ("stale-other-presenter-kept", T, lambda s: replace_once(s,"AND membership_id=? AND revoked_at","AND ? IS NOT NULL AND revoked_at"),"campfire_db","stale_stream_state_matches_sixteen_rails_scenarios"),
    ("reconciler-admin-gate-skips-invitations", J, lambda s: replace_body(s,"pub(crate) async fn reconcile(","if !service.admin_configured() { return Ok(0); } resolve_invitations(db).await?; Ok(0)"),"campfire","huddle_in_process_loop_resolves_invitations_without_livekit_admin_configuration"),
    ("reconciler-stale-pass-bypassed", J, lambda s: replace_body(s,"async fn end_stale_streams(","Ok(())"),"campfire","huddle_reconciler_ends_a_quiet_presenter_without_admin_configuration"),
    ("hand-speaker-permission-bypassed", M, lambda s: replace_once(s,"self.validate_call_attributes(tx.conn(),self.stage_role,Some(tx.now()),self.server_muted_at)?;",""),"campfire_db","hand_mutations_and_role_clearing_match_seventeen_rails_scenarios"),
    ("hand-repeat-idempotence-bypassed", M, lambda s: replace_once(s,"if self.hand_raised_at.is_some() { return Ok(false); }",""),"campfire_db","hand_mutations_and_role_clearing_match_seventeen_rails_scenarios"),
    ("push-connection-scope-bypassed", N, lambda s: replace_once(s, "AND (m.connected_at IS NULL OR m.connected_at<?)", "AND (? IS NOT NULL)"), "campfire_db", "huddle_push_scopes_and_throttle_match_thirty_six_rails_scenarios"),
    ("push-throttle-shortened", N, lambda s: replace_once(s, "pub const JOIN_PUSH_THROTTLE_WINDOW: i64 = 600;", "pub const JOIN_PUSH_THROTTLE_WINDOW: i64 = 1;"), "campfire_db", "huddle_push_scopes_and_throttle_match_thirty_six_rails_scenarios"),
    ("push-policy-bypassed", N, lambda s: replace_once(s, "if !policy_allowed {", "if false && !policy_allowed {"), "campfire_db", "huddle_push_scopes_and_throttle_match_thirty_six_rails_scenarios"),
    ("push-denied-burns-throttle", N, lambda s: replace_once(s, "if !policy_allowed {\n        return Ok(None);", 'if !policy_allowed {\n        tx.conn().execute_cached("UPDATE memberships SET last_huddle_join_push_at=? WHERE id=?", params![tx.now(), request.room_membership_id])?;\n        return Ok(None);'), "campfire_db", "huddle_push_scopes_and_throttle_match_thirty_six_rails_scenarios"),
    ("notice-revoked-and-stale-joiner", N, lambda s: replace_once(s, "if grant.revoked() || !grant.in_call(tx.now())", "if false && (grant.revoked() || !grant.in_call(tx.now()))"), "campfire_db", "huddle_join_leave_and_call_ended_match_sixty_seven_rails_scenarios"),
    ("notice-inactive-humans", N, lambda s: replace_body(s, "fn human(", "User::find_by_id(conn, id)"), "campfire_db", "huddle_join_leave_and_call_ended_match_sixty_seven_rails_scenarios"),
    ("notice-second-device-leave", N, lambda s: replace_once(s, "if another {", "if false && another {"), "campfire_db", "huddle_join_leave_and_call_ended_match_sixty_seven_rails_scenarios"),
    ("notice-rejoin-window", N, lambda s: replace_once(s, "SignedDuration::from_secs(5)", "SignedDuration::from_secs(1)"), "campfire_db", "huddle_join_leave_and_call_ended_match_sixty_seven_rails_scenarios"),
    ("notice-call-ended-while-others-remain", N, lambda s: replace_once(s, "if others {", "if false && others {"), "campfire_db", "huddle_join_leave_and_call_ended_match_sixty_seven_rails_scenarios"),
    ("notice-join-worker-bypassed", J, lambda s: replace_body(s, "async fn join(", "Ok(Outcome::Done)"), "campfire", "huddle_join_and_invitation_workers_persist_the_payload_for_ws17"),
    ("notice-invitation-worker-bypassed", J, lambda s: replace_body(s, "async fn invitation(", "Ok(Outcome::Done)"), "campfire", "huddle_join_and_invitation_workers_persist_the_payload_for_ws17"),
    ("notice-push-request-dropped", N, lambda s: replace_body(s, "pub fn enqueue_huddle_push(", ""), "campfire", "huddle_push_enqueue_failure_rolls_back_the_notice_transaction"),
    ("notice-revocation-callback-bypassed", G, lambda s: replace_once(s, ".filter(|grant| grant.in_call(tx.now()))", ".filter(|grant| false && grant.in_call(tx.now()))"), "campfire_db", "huddle_join_leave_and_call_ended_match_sixty_seven_rails_scenarios"),
    ("notice-leave-callback-bypassed", G, lambda s: replace_once(s, "if was_in_call {", "if false && was_in_call {"), "campfire_db", "huddle_join_leave_and_call_ended_match_sixty_seven_rails_scenarios"),
    ("presence-before-commit", E, lambda s: replace_once(s, "tx.emit_after_commit", "tx.emit_now"), "campfire", "huddle_presence_reaches_real_sockets_and_rolled_back_revocation_stays_silent"),
    ("presence-worker-bypassed", J, lambda s: replace_body(s, "async fn presence(", "Ok(Outcome::Done)"), "campfire", "huddle_presence_reaches_real_sockets_and_rolled_back_revocation_stays_silent"),
    ("presence-sink-bypassed", B, lambda s: replace_body(s, "pub(crate) fn presence(", "Ok(())"), "campfire", "huddle_presence_reaches_real_sockets_and_rolled_back_revocation_stays_silent"),
    ("presence-recovery-disabled", J, lambda s: replace_body(s, "pub(crate) async fn recover_unregistered(", "Ok(0)"), "campfire", "huddle_presence_recovers_only_the_previous_unknown_class_failures"),
    ("gateway-secret-bypassed", I, lambda s: replace_body(s, "fn authenticate(", 'c.set_header("cache-control", "no-store"); Some(c.app().config.huddle.clone())'), "campfire", "huddle_gateway_missing_and_wrong_secret_fail_closed"),
    ("revoked-grant-authorized", G, lambda s: replace_body(s, "pub fn authorized(", "Ok(true)"), "campfire", "huddle_gateway_revoked_and_removed_member_grants_fail_closed"),
    ("removed-member-authorized", G, lambda s: replace_body(s, "pub fn authorized(", "Ok(!self.revoked())"), "campfire", "huddle_gateway_removed_member_without_callbacks_is_revoked"),
    ("grant-revocation-bypassed", G, lambda s: replace_body(s, "pub fn revoke(", "Ok(())"), "campfire_db", "huddle_grant_test"),
    ("disconnect-floor-bypassed", G, lambda s: replace_once(s, "if seen_after.is_some_and", "if false && seen_after.is_some_and"), "campfire_db", "huddle_liveness_touch_and_first_sighting_jobs_match_rails"),
    ("broadcast-config-port-truncation", H, lambda s: replace_once(s, "Some((host.to_lowercase(), uri.port.unwrap_or(default_port)))", "let port = uri.port.unwrap_or(default_port); if port > 65535 { None } else { Some((host.to_lowercase(), port)) }"), "campfire", "channels::sink::tests::huddle_configuration_matches_ws13_rails_vectors"),
    ("twirp-placeholder", H, lambda s: replace_body(s, "async fn post(", "Ok(())"), "campfire", "huddle::tests::twirp_"),
    ("endpoint-separation", H, lambda s: replace_once(s, "if public != internal", "if true"), "campfire", "huddle::tests::configured_endpoints_and_twirp_prefixes_match_rails"),
    ("listener-publishing", L, lambda s: replace_body(s, "pub fn can_publish(", "true"), "campfire", "huddle::tests::listener_and_server_muted_members_never_receive_publish_grants"),
    ("server-muted-publishing", L, lambda s: replace_once(s, "!server_muted && (!stage_room", "(!stage_room"), "campfire", "huddle::tests::listener_and_server_muted_members_never_receive_publish_grants"),
    ("forbidden-token-permissions", L, lambda s: replace_once(s, "if FORBIDDEN_VIDEO_PERMISSIONS.iter().any", "if false && FORBIDDEN_VIDEO_PERMISSIONS.iter().any"), "campfire", "huddle::tests::strict_token_shapes_match_pinned_rails"),
    ("expired-token", L, lambda s: replace_once(s, "let validation = Validation { issuer:", "let validation = Validation { verify_expiration: false, issuer:"), "campfire", "huddle::tests::strict_token_shapes_match_pinned_rails"),
    ("cleanup-placeholder", D, lambda s: replace_body(s, "pub fn claim(", "Ok(None)"), "campfire_db", "huddle_cleanup"),
    ("cleanup-backoff", D, lambda s: replace_once(s, "pub const INITIAL_RETRY_DELAY_SECONDS: i64 = 15;", "pub const INITIAL_RETRY_DELAY_SECONDS: i64 = 1;"), "campfire_db", "huddle_cleanup_retry_schedule_and_timestamps_match_rails"),
    ("cleanup-worker-bypassed", J, lambda s: replace_body(s, "async fn cleanup(", "Ok(Outcome::Done)"), "campfire", "huddle::tests::cleanup_background_queue_and_http_enqueue_rollback"),
]

environment = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"),
                   CI="1", CABLE_TEST_PORT_RANGE="52300-52349", MAIL_TEST_PORT_RANGE="52350-52399")
if len(sys.argv)>1:
    if sys.argv[1]=="--only":
        assert len(sys.argv)==3, "usage: --only REGEX"
        mutations=[entry for entry in mutations if re.search(sys.argv[2],entry[0])]
        assert mutations
    else:
        selected=set(sys.argv[1:])
        assert selected <= {entry[0] for entry in mutations},selected
        mutations=[entry for entry in mutations if entry[0] in selected]
for name, path, mutate, package, test in mutations:
    original = path.read_text()
    try:
        path.write_text(mutate(original))
        command = ["mise", "exec", "rust@1.98.1", "--", "cargo", "test", "--locked", "-j", "4",
                   "--manifest-path", str(ROOT / "rust/Cargo.toml"), "-p", package]
        if package == "campfire":
            command += ["--bin", "campfire"]
        command += [test, "--", "--nocapture"]
        result = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        summaries = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert result.returncode != 0 and summaries and "could not compile" not in output, output[-5000:]
        assert "panicked at" in output, output[-5000:]
        print(f"{name}: {summaries[-1]}", flush=True)
    finally:
        path.write_text(original)
print(f"WS13 discrimination: {len(mutations)} compiled regressions detected; sources restored", flush=True)
