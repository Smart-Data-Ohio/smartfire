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
covered_before = {
    "controllers/users/avatars_controller_test.rb": {"show initials", "show image with invalid token responds 404"},
    "controllers/pwa_controller_test.rb": {
        "service worker serves as JavaScript with the fetch and notification handlers", "service worker caches static assets only",
        "notification clicks focus an existing window before opening a new one", "offline shell renders signed-out with reconnect behavior",
    },
}
covered = {file: set(names) for file, names in covered_before.items()}
covered.update({
    "controllers/users_controller_test.rb": {
        "profile message buttons carry the accessible name", "index lists active members with presence and selection",
        "index lists starred people first with a star marker", "index requires sign-in",
    },
    "controllers/users/cards_controller_test.rb": {
        "card shows identity, presence, role, and actions for a peer", "offline peers read offline",
        "card shows the presence dot and custom status badge", "your own card offers editing your profile instead",
        "agents can be messaged but not called", "inactive users show status without message actions", "card requires sign-in",
    },
    "controllers/users/profiles_controller_test.rb": {
        "show", "update", "updates are limited to the current user", "linking a github login strips and downcases it",
        "profile saves the notification switches", "profile saves the call settings", "clearing a github login unlinks it",
        "changing email requires the current password", "changing email with a wrong current password is refused",
        "a new password cannot stand in for the current one", "changing email with the current password records a self-change",
        "other profile edits and case-only email edits need no password and record nothing",
        "profile asks for the current password only when the account has one",
        "update saves the theme and time zone", "update saves the text size", "an IANA time zone round-trips through the form",
        "a legacy Rails time zone name still shows selected", "choosing a time zone or Not set records an explicit choice",
        "the layout marks an explicit Not set so the browser skips detection", "update rejects an unknown theme or time zone",
    },
    "controllers/users/bans_controller_test.rb": {
        "create bans user and creates ban records from sessions", "create destroys user sessions", "non-admins cannot ban users",
        "destroy removes ban records and sets user to active", "non-admins cannot unban users",
    },
    "controllers/accounts_controller_test.rb": {"update", "non-admins cannot update"},
    # The original destroy case removes David, an agent owner: it still needs WS11.
    "controllers/accounts/users_controller_test.rb": {"update", "non-admins cannot perform actions"},
    "controllers/accounts/custom_styles_controller_test.rb": {"update", "non-admins cannot update"},
    "controllers/accounts/join_codes_controller_test.rb": {"create new join code", "only administrators can create new join codes"},
})
rows = []
starting_count = 0
for file, owner in files.items():
    source = subprocess.check_output(["git", "show", f"d7c7de92:test/{file}"], cwd=root, text=True)
    names = re.findall(r'^\s*test\s+"([^"]+)"', source, re.M)
    if file.startswith("controllers/public_pages"):
        names = [name for name in names if name.startswith("sign-in page")]
    assert covered.get(file, set()) <= set(names), (file, "unknown covered case")
    starting = [name for name in names if name not in covered_before.get(file, set())]
    starting_count += len(starting)
    names = [name for name in starting if name not in covered.get(file, set())]
    rows.append((len(source.splitlines()), file, owner, len(starting), len(starting)-len(names), names))
assert starting_count == 194, starting_count
rows.sort(key=lambda row: (-row[0], row[1]))
print("| Rails file (largest first) | Lines | Starting deferred | Criteria covered | Still deferred |")
print("|---|---:|---:|---:|---:|")
for lines, file, _, starting, passed, names in rows:
    print(f"| `test/{file}` | {lines} | {starting} | {passed} | {len(names)} |")
count = 0
for _, file, owner, _, _, names in rows:
    if names:
        print(f"\n`test/{file}` — **{owner}**:")
        for name in names:
            print(f"- {name}")
        count += len(names)
print(f"\nDeferred inventory: {count} named Rails controller/system cases remain from the original 194; {starting_count-count} criteria now have equivalent Rust coverage. This is a criterion mapping, not a claim that the Ruby test files or browser system tests ran.")
