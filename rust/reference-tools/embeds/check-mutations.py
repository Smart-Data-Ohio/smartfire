#!/usr/bin/env python3
"""Prove the security/parity regressions fail when their protection is broken."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
env = os.environ.copy()
env.setdefault("TMPDIR", str(root.parent / ".scratch"))
env.setdefault("CARGO_TARGET_DIR", str(root / ".scratch/target"))
mutations = [
    ("net/redirect.rs", 'let relative = uri::parse(location).ok()?;', 'return uri::parse(location).ok().filter(Uri::is_http);\n    #[allow(unreachable_code)]\n    let relative = uri::parse(location).ok()?;', "ws15e_follows_relative_redirects"),
    ("opengraph/fetch.rs", 'for _ in 0..=max_redirects {', 'let _ = max_redirects;\n    for _ in 0..=MAX_REDIRECTS {', "ws15e_honors_zero_and_three_redirect_budgets"),
    ("opengraph/fetch.rs", 'match deadline {', 'let deadline = deadline.map(|_| Duration::from_secs(60));\n    match deadline {', "ws15e_deadline_covers_body_reads"),
    ("opengraph/fetch.rs", 'let ip = guard::resolve(net.resolver.as_ref(), url.host.as_deref().unwrap_or("")).await?;', 'let ip = "93.184.216.34".parse().unwrap();', "ws15e_rejects_private_redirect_before_dialing"),
    ("net/guard.rs", 'match ip {\n        IpAddr::V4', 'return false;\n    #[allow(unreachable_code)]\n    match ip {\n        IpAddr::V4', "ws15e_matches_our_rails_guard_corpus"),
    ("image_proxy.rs", '"image/jpeg", "image/png",', '"image/svg+xml", "image/jpeg", "image/png",', "ws15e_image_proxy_has_rails_status_body_and_header_matrix"),
    ("image_proxy.rs", 'pub const MAX_BODY_SIZE: usize = 5 * 1024 * 1024;', 'pub const MAX_BODY_SIZE: usize = 7 * 1024 * 1024;', "ws15e_image_proxy_has_rails_status_body_and_header_matrix"),
    ("image_proxy.rs", 'let ip = guard::resolve(net.resolver.as_ref(), url.host.as_deref().unwrap_or("")).await?;', 'let ip = "93.184.216.34".parse().unwrap();', "ws15e_image_proxy_denies_ssrf_and_dns_failures_before_fetching"),
    ("../controllers/embeds.rs", 'before_actions(c, Before::default()).await?;', '', "ws15e_image_proxy_requires_sign_in_and_rejects_invalid_signatures"),
    ("../controllers/embeds.rs", 'let Some(url) = image_proxy::verified_url(&c.app().secrets, signed, c.now()) else {', 'let Some(url) = Some("http://images.example.com/image.png".to_string()) else {', "ws15e_image_proxy_requires_sign_in_and_rejects_invalid_signatures"),
    ("../controllers/presenters/rich_text.rs", 'Ok(crate::integrations::image_proxy::signed_path(self.secrets, url))', 'Err(campfire_richtext::Error::Raised("embed_image signer unavailable"))', "ws15e_rendered_embed_html_matches_rails_and_uses_the_proxy"),
    ("link_embed/url_classifier.rs", 'if urls.len() == 3', 'if urls.len() == 4', "ws15e_link_url_policy_matches_rails"),
    ("link_embed/url_classifier.rs", 'suppressed.contains(&normalized)', 'false', "ws15e_link_url_policy_matches_rails"),
    ("linkedin.rs", 'allowed_urn_trailer(&text[urn.end()..])', 'true', "ws15e_linkedin_url_and_html_corpus_matches_rails"),
    ("opengraph.rs", 'if classifier {', 'if false && classifier {', "ws15e_composer_skips_github_and_fizzy_cards_without_dns"),
    ("net/http.rs", 'if timer.as_mut().poll(cx).is_ready()', 'if false && timer.as_mut().poll(cx).is_ready()', "ws15e_write_timeout_bounds_a_stalled_transport"),
    ("link_embed/store.rs", 'tx.emit_after_commit(Event::job(&FetchJob { embed_id: embed.id }));', 'let _ = embed;', "ws15e_http_link_jobs_and_references_roll_back_with_rejected_enqueue"),
    ("../controllers/message_embed_suppressions.rs", 'message.creator_id!=user_id || message.system_note || locked', 'message.system_note || locked', "ws15e_suppression_requires_membership_author_and_eligible_conversation"),
    ("link_embed/store.rs", 'tx.emit_after_commit(Event::broadcast(&CardUpdate { embed_id: self.id }));', 'tx.emit_now(Event::broadcast(&CardUpdate { embed_id: self.id }));', "ws15e_link_card_broadcasts_only_after_commit_with_message_key_and_scroll"),
    ("link_embed/fetcher.rs", '["image/jpeg", "image/png", "image/gif", "image/webp", "image/avif"]', '["image/jpeg", "image/png", "image/gif", "image/webp", "image/avif", "image/svg+xml"]', "ws15e_link_fetch_records_positive_negative_and_guarded_image_results"),
    ("action_claims.rs", "\"json_extract(agent_events.metadata, '$.status') = 'running'\"", '"1"', "github_claim_sweep_and_late_finish_keep_the_first_outcome_and_enqueue_once"),
    ("action_claims.rs", 'created_at < ?', 'created_at <= ?', "github_claim_sweep_fails_only_overdue_running_claims_and_preserves_metadata"),
]
for name, old, new, test in mutations:
    path = root / "crates/campfire/src/integrations" / name
    source = path.read_text()
    assert source.count(old) == 1, (name, old)
    try:
        path.write_text(source.replace(old, new))
        result = subprocess.run(["cargo", "test", "-j", "4", "-p", "campfire", test, "--", "--nocapture"], cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        log = root.parent / ".scratch" / f"mutation-{test}.log"
        log.write_text(result.stdout)
        assert result.returncode != 0 and "test result: FAILED" in result.stdout, f"mutation survived or did not compile: {test}\n{result.stdout[-4000:]}"
        print(f"{test}: " + next(line for line in result.stdout.splitlines() if line.startswith("test result:")))
    finally:
        path.write_text(source)
print(f"WS15e mutation checks: {len(mutations)} detected, 0 survived")
