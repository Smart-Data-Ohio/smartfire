#!/usr/bin/env python3
"""Enumerate every deferred Rails test in this split from the fixed source pin."""
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parents[2]
files = {
    "controllers/users_controller_test.rb": "WS8br2; agent/bot presentation facts from WS11",
    "controllers/users/profiles_controller_test.rb": "WS8br2 integration; WS9 security panels, WS17 status/notifications, WS13 calls, WS14/WS15 Google, WS15g GitHub seams",
    "controllers/users/profiles_two_factor_test.rb": "WS9; WS8br2 profile panel integration",
    "controllers/users/cards_controller_test.rb": "WS8br2; WS11 agents, WS12 stars, WS17 presence/status seams",
    "controllers/users/bans_controller_test.rb": "WS8br2; WS9 sudo, WS11/WS13 revocation seams",
    "controllers/accounts_controller_test.rb": "WS8br2",
    "controllers/accounts/users_controller_test.rb": "WS8br2; WS9 sudo/security metadata seams",
    "controllers/accounts/icons_controller_test.rb": "WS8br2",
    "controllers/accounts/audit_logs_controller_test.rb": "WS8br2; WS9 CSV sudo seam",
    "controllers/accounts/custom_styles_controller_test.rb": "WS8br2; WS9 sudo seam",
    "controllers/accounts/logos_controller_test.rb": "WS8br2",
    "controllers/accounts/join_codes_controller_test.rb": "WS8br2; WS9 sudo seam",
    "controllers/workspace_icons_controller_test.rb": "WS8br2",
    "controllers/first_runs_controller_test.rb": "WS8br2; WS9 session seam",
    "controllers/welcome_controller_test.rb": "WS8br2",
    "controllers/users/avatars_controller_test.rb": "WS8br2",
    "controllers/public_pages_controller_test.rb": "WS9 sign-in page integration",
    "controllers/pwa_controller_test.rb": "WS8br2 service-worker event harness",
    "system/first_run_tour_test.rb": "WS8br2 with WS8b-m composer/room integration",
    "system/timezone_detection_test.rb": "WS8br2",
    "system/workspace_icons_test.rb": "WS8br2",
    "system/audit_log_test.rb": "WS8br2; WS9 export sudo seam",
    "system/service_worker_test.rb": "WS8br2",
    "system/people_group_dms_test.rb": "WS8br2 people/cards; WS8br DM actions",
    "system/starred_people_test.rb": "WS12 stars; WS8br2 cards/directory integration",
    "system/icons_test.rb": "WS8br2 workspace icons/profile names; WS8b-m message icons",
}
covered = {
    "controllers/users/avatars_controller_test.rb": {"show initials", "show image with invalid token responds 404"},
    "controllers/pwa_controller_test.rb": {
        "service worker serves as JavaScript with the fetch and notification handlers", "service worker caches static assets only",
        "notification clicks focus an existing window before opening a new one", "offline shell renders signed-out with reconnect behavior",
    },
}
count = 0
for file, owner in files.items():
    source = subprocess.check_output(["git", "show", f"d7c7de92:test/{file}"], cwd=root, text=True)
    names = re.findall(r'^\s*test\s+"([^"]+)"', source, re.M)
    if file.startswith("controllers/public_pages"):
        names = [name for name in names if name.startswith("sign-in page")]
    names = [name for name in names if name not in covered.get(file, set())]
    if names:
        print(f"\n`test/{file}` — **{owner}**:")
        for name in names:
            print(f"- {name}")
        count += len(names)
print(f"\nDeferred inventory: {count} named Rails controller/system cases. Existing upstream Rust tests do not constitute a re-diff or browser acceptance of these cases.")
