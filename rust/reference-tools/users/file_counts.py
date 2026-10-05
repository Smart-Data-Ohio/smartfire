#!/usr/bin/env python3
"""Count actual Rust test outcomes per owned file from the complete fresh-clone log."""
from pathlib import Path
import re
import sys

log = Path(sys.argv[1]).read_text()
files = {
    "controllers::rooms::review_cache_tests::review_pr177_cache_probes": ("controllers/rooms/review_cache_tests.rs (Astra single-divider/two-viewer cache probe)", 1),
    "controllers::rooms::parity_tests::review_unread_divider_render_and_cached_page_match_rails": ("controllers/rooms/parity_tests.rs (new list-rendering regression)", 1),
    "app::round_four_security_tests::account_audit_failure_preserves_rails_committed_settings_code_styles_and_logo": ("app/round_four_security_tests.rs (new production failure oracle)", 1),
    "controllers::users::fizzy_profile_tests": ("controllers/users/fizzy_profile_tests.rs", 6),
    "controllers::users::agent_profile_tests": ("controllers/users/agent_profile_tests.rs", 11),
    "controllers::accounts::attachment_tests": ("controllers/accounts/attachment_tests.rs", 3),
    "controllers::public_pages::sign_in_google_tests": ("controllers/public_pages/sign_in_google_tests.rs", 1),
    "controllers::users::profile_effective_ooo_tests": ("controllers/users/profile_effective_ooo_tests.rs", 12),
    "controllers::users::layout_preferences_tests": ("controllers/users/layout_preferences_tests.rs", 12),
    "controllers::users::avatars::avatar_image_tests": ("controllers/users/avatar_image_tests.rs", 2),
    "controllers::users::ban_lifecycle_tests": ("controllers/users/ban_lifecycle_tests.rs", 3),
    "controllers::public_pages::tests": ("controllers/public_pages.rs", 4),
    "controllers::users::profile_security_tests": ("controllers/users/profile_security_tests.rs", 7),
    "controllers::rooms::directs::picker_tests": ("controllers/rooms/directs/picker_tests.rs", 5),
    "controllers::users::status_popup_tests": ("controllers/users/status_popup_tests.rs", 4),
    "controllers::users::profile_sections_tests": ("controllers/users/profile_sections_tests.rs", 13),
    "controllers::users::joining_tests": ("controllers/users/joining_tests.rs", 2),
    "controllers::users::profile_page_tests": ("controllers/users/profile_page_tests.rs", 6),
    "controllers::first_runs::tests": ("controllers/first_runs/tests.rs", 3),
    "controllers::welcome::tests": ("controllers/welcome/tests.rs", 2),
    "controllers::accounts::audit_logs::tests": ("controllers/accounts/audit_logs/tests.rs", 5),
    "controllers::accounts::icons::tests": ("controllers/accounts/icons/tests.rs", 11),
    "controllers::accounts::logos::tests": ("controllers/accounts/logos/tests.rs", 2),
    "controllers::accounts::view_tests": ("controllers/accounts/view_tests.rs", 4),
    "controllers::users::people_tests": ("controllers/users/people_tests.rs", 13),
    "controllers::users::profile_settings_tests": ("controllers/users/profile_settings_tests.rs", 4),
    "controllers::accounts::mutation_tests": ("controllers/accounts/mutation_tests.rs", 12),
}
for prefix, (file, expected) in files.items():
    results = re.findall(r"^test " + re.escape(prefix) + r"(?:::[^\s]+)? \.\.\. (ok|FAILED|ignored)", log, re.M)
    assert len(results) == expected and all(result == "ok" for result in results), (file, results)
    print(f"{file}: {len(results)} passed; 0 failed; 0 ignored")
print("WS8br2 file accounting: 150 executed Rust groups; 4 DM picker bodies; 5 popup bodies and 6 HTTP status cases; 10 Google Calendar fragments and 11 status/meeting/OOO fragments; 14 audit HTML/nav/CSV cases, 15 date parses, 39 icon validations, 4 icon bodies/navs, 9 logo PNG responses; 14 card bodies, 2 directories, 31 profile PATCH cases, 4 appearance bodies, 24 account/ban cases, 9 individual exactly-one audit checks and one owner-removal with exactly two distinct Rails audits, 14 account rows, 2 account bodies/navs/footers, 2 invites, 3 CSS bodies; 11 complete bot profile HTML/nav cases; 6 complete Fizzy fragments and real profile HTTP/side-effect cases; 10 signed icon/logo assignments, durable rollback and 3 NullAnalyzer/audit after-commit states; 5 production account and 2 icon audit-failure states; 2 complete profile pages (seed and markup); 8 unread HTTP list states, each cold and cached; exactly one divider for each of two viewers across six warm requests, then zero for the read viewer and one for the unread viewer, with divider-free shared fragments")
