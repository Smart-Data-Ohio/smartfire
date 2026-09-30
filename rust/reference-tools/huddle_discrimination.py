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

if len(sys.argv) > 1:
    selected = set(sys.argv[1:])
    assert selected <= {mutation[0] for mutation in mutations}, selected
    mutations = [mutation for mutation in mutations if mutation[0] in selected]

environment = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"),
                   CI="1", CABLE_TEST_PORT_RANGE="52300-52349", MAIL_TEST_PORT_RANGE="52350-52399")
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
