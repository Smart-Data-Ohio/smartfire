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
    "system/people_group_dms_test.rb": {
        "the new-DM picker filters as you type with no suggestion bubble or submit button",
        "picker selections survive filtering and Message starts the DM",
        "Enter in the picker filter selects the single visible match",
        "clicking a picker row toggles it while the name still opens the profile card",
        "the new-DM picker does not overflow at phone width",
        "Esc with a closed profile card stays unhandled for later listeners",
        "multi-selecting three people in the directory lands in their group DM",
        "shift-click extends the checkbox range", "long-press selects a row on touch",
        "agents are selectable for messages but excluded from huddles",
        "start huddle keeps agents in the DM but out of the call",
    },
    "controllers/first_runs_controller_test.rb": {"new is permitted when no other users exit", "new is not permitted when account exist", "create", "create is not vulnerable to race conditions"},
    "controllers/welcome_controller_test.rb": {"redirects to the first created visible room the user has access to", "redirects to the last room visited, if we have one"},
    "controllers/accounts/logos_controller_test.rb": {"show stock", "show stock small size", "show custom", "show custom small size", "show stock when custom logo cannot be resized", "destroy"},
    "controllers/accounts/audit_logs_controller_test.rb": {
        "admins can browse the log", "members are forbidden", "visitors are sent to sign in", "visitors cannot export CSV", "members cannot export CSV",
        "filtering by actor matches names and emails in labels", "filtering by action and target type", "unknown filter values are ignored", "filtering by date range", "paging walks older entries",
        "CSV export carries headers and the filtered rows", "CSV export neutralizes formula injection", "past the export cap the page warns and the CSV filename says truncated",
        "within the export cap there is no truncation notice", "CSV export neutralizes formula injection in request columns",
    },
    "controllers/accounts/icons_controller_test.rb": {
        "index lists icons with previews shortcodes titles and uploaders", "create uploads an icon", "create renders validation errors inline",
        "create reports a name that raced past validation as taken", "destroy removes the icon and its blob", "members get forbidden on list create and delete",
    },
    "controllers/workspace_icons_controller_test.rb": {
        "serves an SVG with the documented headers", "serves a PNG without the SVG-only headers", "supports conditional GETs with the blob checksum",
        "returns not found for unknown names", "returns not found for signed-out users", "unenrolled sessions are sent to setup instead of served the icon", "stale enrolled sessions are signed out instead of served the icon",
    },
    "controllers/users_controller_test.rb": {
        "show", "new", "new does not allow a signed in user", "new requires a join code", "create", "creating a new user with an existing email address will redirect to login screen",
        "profile message buttons carry the accessible name", "index lists active members with presence and selection",
        "index lists starred people first with a star marker", "index requires sign-in",
    },
    "controllers/users/cards_controller_test.rb": {
        "card shows identity, presence, role, and actions for a peer", "offline peers read offline",
        "card shows the presence dot and custom status badge", "your own card offers editing your profile instead",
        "agents can be messaged but not called", "inactive users show status without message actions", "card requires sign-in",
    },
    "controllers/users/profiles_controller_test.rb": {
        "profile offers a connect button without an account", "profile shows the connected account with a disconnect button",
        "profile offers a reconnect when Google rejected the connection", "profile offers Drive previews for a connected account without the Drive scope",
        "profile shows Drive previews as enabled when the account has the Drive scope", "profile offers Drive previews again for the retired metadata grant",
        "profile asks to reconnect when the grant lacks the calendar scope", "profile shows Disconnect for a partial grant with Drive still active",
        "reconnect preserves a granted Drive scope", "reconnect without Drive requests the calendar scope only",
        "profile links to connect for meeting status without an account", "profile offers the meeting toggle for a connected account",
        "profile shows the meeting fetch notice when a refresh failed", "profile asks to reconnect for meeting status left on after disconnect",
        "show gives the Edge install instructions to a browser identifying only as Edge", "profile shows Google Calendar as not configured without credentials", "profile shows no Drive row when Google is not configured", "profile lists the quiet-during-meetings switch", "profile lists the notification switches with explanations", "profile lists the call settings with their defaults", "a github login cannot be claimed by a second user", "profile rejects non-boolean notification input", "profile rejects an unknown microphone mode", "DND switch reflects the effective state after a timed expiry", "DND switch stays on while a timer runs",
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
    "controllers/accounts_controller_test.rb": {"edit","edit groups administrators separately from members with a divider","update", "non-admins cannot update"},
    # The original destroy case removes David, an agent owner: it still needs WS11.
    "controllers/accounts/users_controller_test.rb": {"update", "non-admins cannot perform actions"},
    "controllers/accounts/custom_styles_controller_test.rb": {"edit","update", "non-admins cannot update"},
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
