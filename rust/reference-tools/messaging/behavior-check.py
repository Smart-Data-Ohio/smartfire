#!/usr/bin/env python3
"""Seeded Rails/Rust behaviour checks. Builds its own seeds, binary and browser inputs.

Each writing case gets independent copies of the seed. Read-only message-list
regressions share one verified fixture/server but get fresh viewer contexts.
All cases exercise real HTTP/Action Cable. No pre-existing target/scratch, screenshot or response mask.
"""
from contextlib import ExitStack
from behavior_drive_transport import drive_transport
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import sqlite3
import subprocess
import tempfile
import time
import textwrap
import urllib.request
from behavior_action_rows import assert_action_rows
from behavior_upload_bytes import uploaded_bytes
from behavior_server_cleanup import stop_behavior_servers
from behavior_mutation_jobs import probe_jobs
from browser_host import build_host

ROOT = Path(__file__).resolve().parents[3]
RUST = ROOT / "rust"
SCRATCH = ROOT / ".scratch"
PIN = "d7c7de9264c63015be398001d7a1094e7695a6db"
CASES = {
    "drive_attachments": ["attach Drive files from the picker, send textless, and remove through edit", "edit a room message in the composer and remove one of two attachments", "attach a Drive file from the thread composer"],
    "motion": ["motion is off by default in the test environment",'mobile drawer animates in, lands in place, and returns focus with motion on', 'member selection mode moves no rows and resizes nothing', 'people directory bar shifts no rows when toggling', 'people directory bar stays stuck while scrolling', 'room menu measures at full scale when clamping to the viewport edge', 'mobile drawer keeps the room list scroll position across close and reopen', 'mobile drawer reveals a current room far down the list on first open', 'mobile drawer reopens on the current room when it is already in view'],
    "mobile_layout": ['the profile page fits phone widths without scrolling sideways', 'headers outside the workspace shell stay opaque over scrolled content', 'headers outside the workspace shell never cover the page or its scrollbar', 'pages outside the workspace shell show no drawer toggle that opens nothing', 'every drawer destination has one toggle that opens the drawer on itself'],
    "channel_threads_controller": ['converts a thread to work, assigns an eligible owner, and keeps an audit trail', 'work owner must be an eligible parent-room member and a revoked owner stays visible as unavailable', 'assigned owner can change work status but cannot reassign it', 'only a thread manager can remove work tracking', 'a manager can assign an eligible agent and the agent is notified', 'the owner picker lists eligible agents with profiles and excludes ineligible ones', 'a member who cannot manage the thread cannot assign an agent', 'ordinary thread fields remain separate from work tracking'],
    "sending_messages": ["sending messages between two users", "editing messages", "deleting messages"],
    "workspace_markdown": [
        "workspace follows the system theme and mobile navigation remains reachable",
        "Markdown messages reach other users and editing preserves the original source",
        "desktop keyboard composition keeps line breaks and sends once after composition ends",
        "untrusted markup stays inert in the delivered message",
        "Markdown replies and file attachments remain usable",
        "mention suggestions select a room member without sending the unfinished message",
        "a rejected message can be recovered corrected and sent",
        "sending preserves the submitted source and a newer draft",
    ],
    "threads": [
        'tracks work, assigns an owner, completes and reopens it without losing the conversation',
        'shows work-thread guidance in the new-thread form and on the work page',
        'keeps the new-thread guidance usable on a phone',
        'shows work assignment activity to the owner and opens the exact thread',
        'keeps the thread drawer usable on a phone and preserves the channel',
        'marks a joined thread read only while the conversation is visible',
        'opens a shared thread message link around an older post',
        'keeps an anchored older thread unread when a new reply arrives',

        "creates a thread from a channel message and keeps the channel draft separate",
        "the thread root counts its replies live and hides the count when none remain",
        "a stray create re-entry does not wipe the half-filled thread name",
        "browses active and closed threads and can join or leave a closed one",
        "rejects an external thread deep link before fetching it",
        "renders untrusted thread metadata as text",
        "discusses a pull request from its card",
    ],
    "message_list_a11y": [
        "the message list is a single tab stop with a roving tabindex",
        "arrow keys move between messages",
        "a stream replacing the focused message keeps focus and the tab stop on its replacement",
        "a stream replacing the tab-stop message while focus is elsewhere keeps the tab stop on the replacement",
        "a direct DOM swap of the focused message keeps focus and the tab stop on its replacement",
        "deleting the focused message moves focus to the surviving tab stop",
        "deleting an older focused message hands focus to its neighbour, not the newest",
        "a focus move during a stream render survives Turbo's focus restore",
        "a no-change room refresh does not yank focus back to the composer",
        "the ContextMenu key opens the shared menu and Escape returns focus",
        "a late composer autofocus does not steal focus from a message",
        "up arrow from an empty composer still edits my last message",
        "up-arrow-to-edit shows an error when the actions endpoint fails",
        "forward reuses the menu-open metadata request instead of fetching again",
        "a menu opened while an action waits does not redirect the pending action",
        "the menu closes before Turbo caches the page",
        "the main message list is a live log",
        "paginated history stays quiet past the insert, then the live region comes back",
        "an edit replacement is not announced as an addition",
        "an own message is not re-announced when its broadcast replaces the pending copy",
        "search results keep their menus and focusability",
        "the message-list top padding does not apply to search results",
        "the standalone thread page keeps menus and focusability",
        "the standalone message page keeps its menu and focusability",
        "the viewport allows pinch zoom",
        "profile message and ban buttons have accessible names",
        "flash persists its 5-second minimum under reduced motion",
        "flash dismisses on demand under reduced motion",
        "text fields stay at 16px on touch devices without changing the desktop look",
    ],
    "search_forward_edit": [
        "search tolerates operators, shows an empty state and pages older results",
        "forwarded Markdown keeps tables and code blocks",
        "editing to add a URL renders its card live and the edited marker on load",
    ],
    "unread_divider": [
        "few unread render the divider above the first new message and keep the bottom scroll",
        "many unread scroll the room to the divider",
        "the jump pill shows while the divider is off-screen and returns to it",
        "unread older than the last page keeps the last page and the pill links to the first unread",
        "mark unread from the message menu points the divider at that message",
    ],
    "composer": [
        "blurring an open autocomplete does not leave a zombie that swallows Enter",
        "a stale icon response does not poison the suggestion commit",
        "mention queries are URL-encoded",
        "composer autocomplete exposes combobox semantics over a polite listbox",
        "composing text does not commit a suggestion or send the message",
        "clicking a reply preview scrolls to the loaded message instead of navigating",
        "clicking a reply preview falls back to the permalink when the target is not loaded",
        "deleting a replied-to message turns open reply previews into a tombstone",
        "two typers with the same name do not merge",
        "composer drafts persist per room and clear on send",
        "thread drafts persist per thread without touching the channel draft",
    ],
    "composer_attach_menu": [
        "From Google Drive starts the legacy picker flow",
        "From Google Drive starts the enhanced share flow when sharing is configured",
        "+ shows both attach options when Drive is available",
        "From this device triggers the file input",
        "+ opens the file picker directly without Drive",
        "arrow keys move between items and Escape closes back onto +",
        "a tap outside closes the menu",
        "phone layout keeps the menu above the composer with no horizontal overflow",
        "device files, paste, and drag-and-drop still preview uploads",
    ],
    "boosting_messages": [
        "boosting a message", "deleting a boost", "message update preserves the input state",
        "boost by another user preserves the input state",
    ],
    "message_interactions": [
        "opens message actions from context menu and keyboard, and cancels a moving long press",
        "a release click landing on the just-opened menu does not activate it",
        "shows the message action menu as a bottom sheet on phones",
        "edits through the normal composer and restores the saved draft on cancel and success",
        "a duplicate delivery does not replace the message while its actions are open",
        "keeps newer typing through an asynchronous edit and leaves failures in edit mode",
        "replies with notify off and renders a tombstone when the target is deleted",
        "copies message text and link and forwards to a server-provided thread destination",
        "forwarding twice in a row submits only once",
        "groups emoji reactions, updates the live count, and highlights the current user",
    ],
    "message_actions_mobile": [
        "message action menu is a bottom sheet with touch-sized targets on phones",
        "message action menu stays a floating popover on desktop",
    ],
    "message_toolbar": [
        "the toolbar stays hidden until hover or focus and labels every action",
        "quick-react creates a boost from the toolbar",
        "reply and thread buttons drive the composer and the thread panel",
        "the more button opens the shared menu for its message",
        "keyboard users reach the toolbar from a focused message",
        "the emoji picker searches and reacts",
        "the picker shows category tabs and switches between them",
        "the picker loads its emoji data only on first open",
        "the picker remembers recent reactions",
        "the picker Custom tab reacts with a workspace icon",
        "the picker reacts with a brand icon shortcode",
        "picker arrows move through options, Enter selects, and Escape returns focus",
        "picker tabs move with arrow keys and switch the grid",
    ],
    "code_highlighting": [
        "thread code stays readable in both themes and scrolls within a narrow screen",
        "language fences highlight common code without changing its text",
        "unlabelled code is detected while text unknown languages and inline code stay literal",
        "search results highlight code on initial load and after returning to the channel",
        "code and copying remain available when the highlighter cannot load",
        "editing a code block replaces its language colors and copied source",
    ],
}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("files", nargs="*", choices=CASES)
parser.add_argument("--slice", choices=["continuation"], help="run this checkpoint's new declarations, without changing existing scopes")
parser.add_argument("--case", help="run one exact pinned declaration from the selected files")
parser.add_argument("--exclude-case", action="append", default=[], help="explicitly omit an unresolved mapped declaration; default still runs it")
parser.add_argument("--negative", action="store_true", help="require each selected case to reject its deliberately broken served implementation")
parser.add_argument("--mutant", help="select one served mutant variant; without --negative, diagnose its acceptance on both apps (not parity credit)")
parser.add_argument("--mutant-set", choices=["visible-assertions", "visible-lookups", "instantaneous-opacity", "element-scopes", "hidden-scopes", "categories", "labels"], help="diagnose all new visibility assertion mutants without parity credit")
parser.add_argument("--repeat",type=int,default=1,help="repeat one positive case with independent fixtures and unchanged deadlines")
parser.add_argument("--keep-going", action="store_true", help="report every selected flow; failures still produce a nonzero exit")
args = parser.parse_args()
assert args.repeat>=1 and (args.repeat==1 or (args.case and not args.negative and not args.mutant and not args.mutant_set)), "repetition is for one positive case only"
files = args.files or list(CASES)
if args.mutant_set:
    assert not args.mutant and not args.case and (not args.negative or args.mutant_set in {"element-scopes", "categories", "labels"}), "only element-scopes/categories/labels support a negative mutant-set run"
    diagnostic_export = {"visible-lookups": "visibilityLookupMutations", "instantaneous-opacity": "instantaneousOpacityMutations", "element-scopes": "elementScopeMutations", "hidden-scopes": "hiddenScopeProbes", "categories": "categoryMutations", "labels": "labelMutations"}.get(args.mutant_set, "visibilityAssertionMutations")
    diagnostic_variants = json.loads(subprocess.check_output([
        "node", "--input-type=module", "-e",
        f"import {{{diagnostic_export}}} from './rust/reference-tools/messaging/behavior-mutations.mjs'; "
        f"console.log(JSON.stringify(Object.fromEntries([...{diagnostic_export}].map(([name,variants])=>[name,[...variants.keys()]]))))"
    ], cwd=ROOT, text=True))
if args.case:
    assert any(args.case in CASES[file] for file in files), "unknown/unmapped named case"
if args.mutant:
    assert args.case, "a selected mutant probe requires one exact named case"
for case in args.exclude_case:
    assert any(case in CASES[file] for file in files), "unknown excluded named case"
SCRATCH.mkdir(exist_ok=True)
env = dict(os.environ, CARGO_BUILD_JOBS="2", RUST_TEST_THREADS="8", PARITY_CPUS="2",
           PARITY_NAMESPACE="ws8bm-behavior", PARITY_OWNER="ws8bm", TMPDIR=str(SCRATCH), CAMPFIRE_REFERENCE=str(ROOT))
image = os.environ.get("PARITY_IMAGE", "triage-reference-d7c7de92")
revision = subprocess.check_output(["docker", "image", "inspect", "--format", "{{range .Config.Env}}{{println .}}{{end}}", image], text=True)
assert any(f"GIT_REVISION={value}" in revision.splitlines() for value in [PIN, PIN[:8]]), "browser reference must be the pinned Rails image"
env["PARITY_IMAGE"] = image
env["WS8BM_NEGATIVE"] = "1" if args.negative else "0"
env["WS8BM_KEEP_GOING"] = "1" if args.keep_going else "0"
if args.mutant:
    env["WS8BM_MUTANT"] = args.mutant
else:
    env.pop("WS8BM_MUTANT", None)
browser_image = "ws8bm-browser-reference-d7c7de92"
subprocess.run(["docker", "build", "--build-context", f"current_schema={ROOT / 'db'}", "--build-arg", f"BASE_IMAGE={image}", "-f", str(RUST / "reference-tools/messaging/browser.Dockerfile"), "-t", browser_image, str(RUST)], cwd=ROOT, check=True)
env["PARITY_IMAGE"] = browser_image
subprocess.run(["bash", "rust/parity/bin/seed", "build", "default", "first_run"], cwd=ROOT, env=env, check=True)
# Build every host this invocation uses, including on a cold target. A
# paused-only case uses TestApp's real binary and needs no second app build.
# Preserve continuation's upload boundary as well as URL/PR jobs.
paused_job_cases={"editing to add a URL renders its card live and the edited marker on load", "discusses a pull request from its card", "Markdown replies and file attachments remain usable", "workspace follows the system theme and mobile navigation remains reachable"}
drive_cases=set(CASES["drive_attachments"]+["From Google Drive starts the legacy picker flow"])
paused_job_cases |= drive_cases
motion_default="motion is off by default in the test environment"
test_environment_cases=drive_cases|{motion_default,"Markdown replies and file attachments remain usable"}
paused_job_cases.add(motion_default)
selected_names=[name for file in files for name in CASES[file] if (not args.case or name==args.case) and name not in args.exclude_case]
needs_paused_jobs=not args.slice and any(name in paused_job_cases for name in selected_names)
needs_test_environment=motion_default in selected_names
needs_drive=any(name in drive_cases for name in selected_names)
if args.slice or any(name not in paused_job_cases for name in selected_names):
    subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "build", "--locked", "-j2", "--manifest-path", "rust/Cargo.toml", "-p", "campfire", "--bin", "campfire"], cwd=ROOT, env=env, check=True)
test_host=build_host(ROOT,env) if needs_paused_jobs or needs_drive or needs_test_environment else None
subprocess.run(["npm", "ci", "--prefix", "rust/parity"], cwd=ROOT, check=True)
subprocess.run(["npm", "exec", "--prefix", "rust/parity", "--", "playwright", "install", "chromium"], cwd=ROOT, check=True)
visibility_atom = subprocess.check_output([
    "docker", "run", "--rm", "--entrypoint", "bundle", browser_image,
    "exec", "ruby", "-rselenium-webdriver", "-e",
    'print File.binread(File.join(Gem::Specification.find_by_name("selenium-webdriver").full_gem_path, "lib/selenium/webdriver/atoms/isDisplayed.js"))'
], cwd=ROOT)
assert visibility_atom == (RUST / "reference-tools/messaging/selenium/isDisplayed.js").read_bytes(), "visibility helper must use the unmodified pinned Selenium atom"
print(f"WS8bm visibility atom: pinned Selenium SHA256 {hashlib.sha256(visibility_atom).hexdigest()} verified", flush=True)
for line in (RUST / "parity/.env.reference").read_text().splitlines():
    if line and not line.startswith("#"):
        key, value = line.split("=", 1)
        env[key] = value
env.update(CAMPFIRE_FROZEN_TIME="2026-03-02T16:00:00Z", CAMPFIRE_LOG="error", TARGET_BIND="127.0.0.1")
target = Path(env.get("CARGO_TARGET_DIR", RUST / "target"))
reference = str(RUST / "parity/bin/reference")
# Keep the historical default, while allowing a worker to choose slots outside
# its OS ephemeral-client range. No application wait or retry is changed.
port_base = int(env.get("WS8BM_BROWSER_PORT_BASE", "52020"))
if not 1 <= port_base <= 65533:
    raise ValueError("WS8BM_BROWSER_PORT_BASE must leave room for three ports")
ports = [port_base + offset for offset in range(3)]
# Refuse occupied ports; never stop another worker's listener.
reservations = []
try:
    for port in ports:
        reservation = socket.socket()
        # Refuse live listeners, while permitting our just-closed listener's
        # TIME_WAIT sockets between independent invocations.
        reservation.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        reservation.bind(("127.0.0.1", port))
        reservations.append(reservation)
finally:
    for reservation in reservations:
        reservation.close()

with sqlite3.connect(RUST / "parity/.seed/default/db/production.sqlite3") as conn:
    protected_counts = {table: conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]
                        for table in ["channel_threads", "messages", "thread_memberships"]}
passed = 0
passed_named = set()
failed_cases = []
escaped_cases = set()
invalid_attempts=0
retry_attempts={}
mutation_names = set(json.loads(subprocess.check_output([
    "node", "--input-type=module", "-e",
    "import {mutationNames} from './rust/reference-tools/messaging/behavior-mutations.mjs'; console.log(JSON.stringify(mutationNames))"
], cwd=ROOT, text=True))) if args.negative else set()
mutation_variants = json.loads(subprocess.check_output([
    "node", "--input-type=module", "-e",
    "import {mutationNames,mutationVariants} from './rust/reference-tools/messaging/behavior-mutations.mjs'; "
    "console.log(JSON.stringify(Object.fromEntries(mutationNames.map(name=>[name,mutationVariants(name)]))))"
], cwd=ROOT, env=env, text=True)) if args.negative else {}
for file in files:
    source_path = f"test/{'controllers/channel_threads_controller' if file == 'channel_threads_controller' else 'system/'+file}_test.rb"
    source = subprocess.check_output(["git", "show", f"{PIN}:{source_path}"], cwd=ROOT)
    selected = [case for case in CASES[file] if (not args.case or case == args.case) and case not in args.exclude_case]
    if args.slice:
        continuation = {"channel_threads_controller":CASES["channel_threads_controller"],"mobile_layout":CASES["mobile_layout"],"threads":CASES["threads"][:8],"message_list_a11y":["text fields stay at 16px on touch devices without changing the desktop look"],"code_highlighting":["thread code stays readable in both themes and scrolls within a narrow screen"]}
        selected = [case for case in selected if case in continuation.get(file,[])]
    if args.mutant_set:
        selected = [case for case in selected if case in diagnostic_variants]
    if args.negative:
        if args.case:
            assert args.case in mutation_names, "no served mutant for this named check"
        selected = [case for case in selected if case in mutation_names]
    # These first seventeen cases only change the current browser DOM/focus or
    # open controls; a shared server avoids gratuitous Docker network churn.
    # Fresh contexts still isolate drafts, menus, focus and observers. Every
    # batch verifies saved message rows are unchanged. All writing/history
    # cases retain separate fixture/database/server copies.
    readonly = CASES["message_list_a11y"][:17] if file == "message_list_a11y" else []
    if file == "message_toolbar":
        readonly = [CASES[file][index] for index in [0, 2, 3, 6, 7, 12]]
    elif file == "message_actions_mobile":
        readonly = CASES[file]
    elif file == "message_interactions":
        readonly = [CASES[file][index] for index in [0, 1, 2, 4, 5]]
    navigation = CASES["message_list_a11y"][20:26] if file == "message_list_a11y" else []
    batches = [[case for case in selected if case in readonly]] if readonly else []
    if navigation:
        batches += [[case for case in selected if case in navigation]]
    batches += [[case] for case in selected if case not in readonly + navigation]
    if file == "composer_attach_menu":
        enhanced=[case for case in selected if case.startswith("From Google Drive starts")]
        ordinary=[case for case in selected if case not in enhanced]
        batches=([ordinary] if ordinary else [])+[[case] for case in enhanced] # Distinct server configuration.
    jobs = probe_jobs(batches, selected, negative=args.negative, mutant=args.mutant,
                      mutation_variants=mutation_variants,
                      diagnostic_variants=diagnostic_variants if args.mutant_set else None)
    jobs *= args.repeat
    for batch, variant in jobs:
        retry_key=(tuple(batch),variant)
        retry_attempts[retry_key]=retry_attempts.get(retry_key,0)+1
        case = batch[0]
        for name in batch:
            assert f'test "{name}"'.encode() in source, "case must be named in the pin"
        with tempfile.TemporaryDirectory(prefix="ws8bm-behavior-", dir=SCRATCH) as directory:
            work = Path(directory)
            fixture = work / "fixture"
            shutil.copytree(RUST / "parity/.seed/default", fixture)
            if file == "motion":
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze", str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), "motion", case], cwd=ROOT, env=env, check=True)
            elif file == "channel_threads_controller":
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze", str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), "work-controller", case], cwd=ROOT, env=env, check=True)
            elif file == "mobile_layout":
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze", str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), "mobile-layout"], cwd=ROOT, env=env, check=True)
            elif file == "workspace_markdown" and case == "Markdown replies and file attachments remain usable":
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze", str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), "workspace-upload"], cwd=ROOT, env=env, check=True)
            elif file == "message_list_a11y":
                fixture_kind = "board-touch" if case.startswith("text fields") else "history" if case.startswith("paginated history") else "message_list"
                if any(name in CASES[file][20:28] for name in batch):
                    fixture_kind = "message_destinations"
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), fixture_kind], cwd=ROOT, env=env, check=True)
            elif file == "search_forward_edit":
                fixture_kind = "search" if case.startswith("search tolerates") else "edit-card" if case.startswith("editing to add") else "forward"
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), fixture_kind], cwd=ROOT, env=env, check=True)
            elif file == "unread_divider":
                fixture_kind = "unread-" + ["few", "many", "pill", "offpage", "menu"][CASES[file].index(case)]
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), fixture_kind], cwd=ROOT, env=env, check=True)
            elif file == "composer":
                fixture_kind = "composer-typing" if case.startswith("two typers") else "composer-drafts"
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), fixture_kind], cwd=ROOT, env=env, check=True)
            elif file == "drive_attachments" or case == "From Google Drive starts the legacy picker flow":
                (fixture / "db/google-calendar-test-helper.rb").write_bytes(subprocess.check_output(["git","show",f"{PIN}:test/test_helpers/google_calendar_test_helper.rb"],cwd=ROOT))
                subprocess.run([reference,"runner","--storage",str(fixture),"--time","2026-03-02T16:00:00Z","--freeze",str(RUST / "reference-tools/messaging/behavior-fixtures.rb"),"drive",case],cwd=ROOT,env=env,check=True)
            elif file == "composer_attach_menu":
                fixture_kind="attach-share" if case.startswith("From Google Drive starts the enhanced") else "attach-menu"
                if fixture_kind=="attach-share":
                    (fixture / "db/drive-share-mocks.rb").write_bytes(subprocess.check_output(["git","show",f"{PIN}:test/support/drive_share_mocks.rb"],cwd=ROOT))
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), fixture_kind], cwd=ROOT, env=env, check=True)
            elif file == "boosting_messages":
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), "boosts"], cwd=ROOT, env=env, check=True)
            elif file in ["message_interactions", "message_actions_mobile", "message_toolbar"]:
                fixture_kind = {"message_interactions": "interactions", "message_actions_mobile": "actions-mobile", "message_toolbar": "toolbar"}[file]
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), fixture_kind], cwd=ROOT, env=env, check=True)
            elif file == "threads" and (case.startswith("opens a shared") or case.startswith("keeps an anchored")):
                fixture_kind = "thread-anchor" if case.startswith("opens a shared") else "thread-anchor-race"
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze", str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), fixture_kind], cwd=ROOT, env=env, check=True)
            elif file == "threads" and case == "discusses a pull request from its card":
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), "thread-pr"], cwd=ROOT, env=env, check=True)
            elif file == "code_highlighting":
                # The production image intentionally excludes test sources.
                # Materialize them from the pin in this fresh fixture, never
                # from a previous local reference copy or scratch directory.
                (fixture / "db/code-highlighting-reference.rb").write_bytes(source)
                (fixture / "db/application-system-reference.rb").write_bytes(subprocess.check_output(
                    ["git", "show", f"{PIN}:test/application_system_test_case.rb"], cwd=ROOT))
                fixture_kind = "highlight-thread" if case.startswith("thread code stays") else "highlight-search" if case.startswith("search results") else "highlight"
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), fixture_kind], cwd=ROOT, env=env, check=True)
            if case in test_environment_cases:
                shutil.copy2(fixture / "db/production.sqlite3",fixture / "db/test.sqlite3")
            shutil.copytree(fixture / "db", work / "db")
            shutil.copytree(fixture / "storage", work / "files")
            database_name="test.sqlite3" if case in test_environment_cases else "production.sqlite3"
            run_env = dict(env, CAMPFIRE_STORAGE_PATH=str(work), HTTP_PORT=str(ports[1]), TARGET_PORT=str(ports[2]), PARITY_SEED_DIR=str(work))
            if file in {"channel_threads_controller","drive_attachments"} or case in {"workspace follows the system theme and mobile navigation remains reachable", "Markdown replies and file attachments remain usable"}:
                run_env['WS8BM_WORK_DATABASES']=json.dumps({
                    f'http://127.0.0.1:{ports[0]}':str(work / f'.instances/{ports[0]}/db' / database_name),
                    f'http://127.0.0.1:{ports[1]}':str(work / 'db' / database_name),
                })
            if args.negative or args.mutant_set:
                run_env["WS8BM_MUTANT"] = variant
            if file in {"composer_attach_menu", "drive_attachments"}:
                run_env.update(GOOGLE_CLIENT_ID="test-client-id", GOOGLE_CLIENT_SECRET="test-client-secret")
            paused_jobs=any(name in paused_job_cases for name in batch)
            if case.startswith("From Google Drive starts the enhanced"):
                run_env.update(GOOGLE_PICKER_API_KEY="test-picker-key",GOOGLE_CLOUD_PROJECT_NUMBER="123456789012")
            if paused_jobs:
                run_env["WS8BM_BROWSER_HOST"]="1"
            if case in test_environment_cases:
                run_env.update(RAILS_ENV="test",CAMPFIRE_DATABASE_PATH=str(work / "db/test.sqlite3"))
            if case in drive_cases:
                run_env.update(WS8BM_DRIVE_PORT=str(port_base+4),WS8BM_DRIVE_STUBS="1",WS8BM_DRIVE_TWO="1" if case.startswith("edit a room message") else "0")
            process = None
            with (SCRATCH / "ws8bm-behavior-servers.log").open("a") as log, ExitStack() as transports:
                try:
                    metadata_path=fixture / "db/browser-fixture.json"
                    metadata=json.loads(metadata_path.read_text()) if metadata_path.exists() else {}
                    drive_calls=transports.enter_context(drive_transport(metadata["drive_payloads"],port_base+4)) if case in drive_cases else None
                    reference_up=[reference, "up", "--seed", "fixture", "--port", str(ports[0]), "--time", "2026-03-02T16:00:00Z", "--freeze"]
                    if case.startswith("From Google Drive starts the enhanced"):
                        for key in ["GOOGLE_CLIENT_ID","GOOGLE_CLIENT_SECRET","GOOGLE_PICKER_API_KEY","GOOGLE_CLOUD_PROJECT_NUMBER"]:
                            reference_up += ["-e",key+"="+run_env[key]]
                    if case in drive_cases:
                        for key in ["GOOGLE_CLIENT_ID","GOOGLE_CLIENT_SECRET","WS8BM_DRIVE_STUBS","WS8BM_DRIVE_TWO"]:
                            reference_up += ["-e",key+"="+run_env[key]]
                    if case in test_environment_cases:
                        reference_up += ["-e","RAILS_ENV=test"]
                        # config/environments/test.rb uses the caller's CI flag
                        # to eager-load the same models/routes as pinned tests.
                        if "CI" in run_env:
                            reference_up += ["-e","CI="+run_env["CI"]]
                    if case=="Markdown replies and file attachments remain usable":
                        reference_up += ["-e","WS8BM_TEST_FORGERY_PROTECTION=1"]
                    if paused_jobs:
                        reference_up += ["-e", "WS8BM_TEST_JOB_ADAPTER=1"]
                    subprocess.run(reference_up, cwd=ROOT, env=run_env, stdout=log, stderr=log, check=True)
                    host_command=[test_host,"controllers::presenters::test_support::ws8bm_browser_host_without_jobs","--exact","--ignored","--nocapture","--test-threads=1"] if paused_jobs else [str(target / "debug/campfire"), "server"]
                    process = subprocess.Popen(host_command, cwd=ROOT, env=run_env, stdout=log, stderr=log)
                    if paused_jobs:
                        print("WS8bm job boundary: Rails ActiveJob::TestAdapter; Rust TestApp::without_job_runner; no selector deadline changes",flush=True)
                    deadline = time.monotonic() + 120
                    while True:
                        if process.poll() is not None:
                            raise RuntimeError("candidate stopped; see .scratch/ws8bm-behavior-servers.log")
                        try:
                            with urllib.request.urlopen(f"http://127.0.0.1:{ports[1]}/up", timeout=2) as response:
                                if response.status == 200:
                                    break
                        except OSError:
                            pass
                        if time.monotonic() > deadline:
                            raise TimeoutError("candidate not ready")
                        time.sleep(.2)
                    fixture_data = fixture / "db/browser-fixture.json"
                    metadata = json.loads(fixture_data.read_text()) if fixture_data.exists() else {}
                    command = ["node", str(RUST / "reference-tools/messaging/behavior.mjs"), f"http://127.0.0.1:{ports[0]}", f"http://127.0.0.1:{ports[1]}", file, json.dumps(batch), json.dumps(metadata)]
                    result = subprocess.run(command, cwd=ROOT, env=run_env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
                    print(result.stdout, end="", flush=True)
                    if drive_calls is not None:
                        print(f"WS8bm outbound Drive calls: {json.dumps(drive_calls)}",flush=True)
                        assert drive_calls and all(call["valid"] for call in drive_calls), "unregistered external Drive request"
                    if args.negative:
                        # An escape belongs to the entire run, not just this
                        # attempt. Record it before retrying an invalid peer.
                        for line in result.stdout.splitlines():
                            if not line.startswith("WS8bm escaped discrimination run: "):
                                continue
                            record = json.JSONDecoder().raw_decode(line.split(": ", 1)[1])[0]
                            assert record["caseName"] in batch and record["variant"] == variant
                            assert record["app"] in {"Rails", "Rust"}
                            escaped_cases.add((file, record["caseName"], variant))
                            failure = f"{file}: {record['caseName']}"
                            if failure not in failed_cases:
                                failed_cases.append(failure)
                    if args.negative and result.returncode and "WS8bm invalid discrimination run:" in result.stdout:
                        invalid_attempts+=1
                        if retry_attempts[retry_key]<int(env.get("WS8BM_DISCRIMINATION_RETRIES","3")):
                            print(f"WS8bm invalid attempt retry: {file}: {batch}: {variant}: fresh fixtures, attempt {retry_attempts[retry_key]+1}/3",flush=True)
                            jobs.append((batch,variant))
                            continue
                    if result.returncode and not args.keep_going:
                        raise subprocess.CalledProcessError(result.returncode, command)
                    if args.negative:
                        variants = json.loads(subprocess.check_output(["node", "--input-type=module", "-e",
                            "import {mutationVariants} from './rust/reference-tools/messaging/behavior-mutations.mjs'; "
                            f"console.log(JSON.stringify({json.dumps(batch)}.map(name=>mutationVariants(name))))"], cwd=ROOT, env=run_env, text=True))
                        succeeded = [name for name, names in zip(batch, variants) if names and not any(
                            (file, name, registered) in escaped_cases for registered in names) and all(
                            any(line.startswith(f"WS8bm discrimination: {file}: {name}: {variant}: {app} served mutant REJECTED (") for line in result.stdout.splitlines())
                            for variant in names for app in ["Rails", "Rust"])]
                    elif args.mutant or args.mutant_set:
                        succeeded = [name for name in batch if all(
                            f"WS8bm review escape: {file}: {name}: {variant}: {app} ACCEPTED" in result.stdout.splitlines() for app in ["Rails", "Rust"])]
                    else:
                        succeeded = [name for name in batch if f"WS8bm browser flow: {file}: {name}: Rails PASS; Rust PASS" in result.stdout.splitlines()]
                    failed_cases.extend(f"{file}: {name}" for name in batch if name not in succeeded
                                        and (not args.negative or f"{file}: {name}" not in failed_cases))
                    if args.negative:
                        passed += sum(len(names) for name, names in zip(batch, variants) if name in succeeded)
                        passed_named.update((file, name) for name in succeeded)
                        continue
                    if args.mutant or args.mutant_set:
                        passed += len(succeeded)
                        continue
                    if not succeeded:
                        continue
                    databases = [work / f".instances/{ports[0]}/db/production.sqlite3", work / "db/production.sqlite3"]
                    if case in test_environment_cases:
                        databases=[path.with_name("test.sqlite3") for path in databases]
                    for database in databases:
                        with sqlite3.connect(database) as conn:
                            if case == "workspace follows the system theme and mobile navigation remains reachable":
                                reference_source = subprocess.check_output(["git","show",f"{PIN}:test/system/workspace_markdown_test.rb"],cwd=ROOT,text=True)
                                literal=textwrap.dedent(reference_source.split("MARKDOWN = <<~'MARKDOWN'.freeze\n")[1].split("  MARKDOWN")[0])
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE creator_id=773523953 AND room_id=654632876 AND markdown_source=?",(literal,)).fetchone()[0]==1
                                mobile=conn.execute("SELECT creator_id,room_id,markdown_source FROM messages WHERE markdown_source LIKE 'Mobile draft%' ORDER BY id").fetchall()
                                print(f"WS8bm workspace final write: {database}: {mobile!r}",flush=True)
                                assert mobile==[(773523953,201306877,"Mobile draft\n")]
                                with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                    assert conn.execute("SELECT COUNT(*) FROM messages").fetchone()[0]==seed.execute("SELECT COUNT(*) FROM messages").fetchone()[0]+2
                            elif file == "drive_attachments":
                                with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                    old_ids={row[0] for row in seed.execute("SELECT id FROM messages")}
                                rows=conn.execute("SELECT id,creator_id,room_id,thread_id,markdown_source FROM messages ORDER BY id").fetchall()
                                new=[row for row in rows if row[0] not in old_ids]
                                assert len(rows)==len(old_ids)+1 and old_ids<= {row[0] for row in rows}, "exact global message count and preserved history"
                                assert len(new)==1 and new[0][1:3]==(773523953,654632876), "exact global message delta and identity"
                                row=new[0]
                                expected_thread=metadata.get("drive_thread_id")
                                assert row[3]==expected_thread
                                expected_body="thread file attached" if expected_thread else "the file moved elsewhere" if case.startswith("attach Drive files") else "two files attached"
                                assert row[4]==expected_body
                                attachments=conn.execute("SELECT file_id FROM drive_attachments WHERE message_id=? ORDER BY id",(row[0],)).fetchall()
                                assert attachments==([] if case.startswith("attach Drive files") else [("1AbcDefGhIjKlMnOpQrSt",)]), "exact saved Drive attachment set"
                            elif file == "motion" or file == "mobile_layout" or case.startswith("text fields"):
                                with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                    for table in (["messages","channel_threads","users","rooms"] if file == "motion" else ["messages","channel_threads"]):
                                        query=("SELECT id,name,email_address,role,status FROM users ORDER BY id" if table=="users" else "SELECT id,name,type,creator_id FROM rooms ORDER BY id" if table=="rooms" else f"SELECT * FROM {table} ORDER BY id")
                                        assert conn.execute(query).fetchall()==seed.execute(query).fetchall()
                            elif file == "channel_threads_controller":
                                from behavior_work_rows import assert_work_rows
                                with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                    assert_work_rows(conn, seed, case, metadata)
                            elif file == "threads" and case in ['tracks work, assigns an owner, completes and reopens it without losing the conversation', 'shows work-thread guidance in the new-thread form and on the work page', 'keeps the new-thread guidance usable on a phone', 'shows work assignment activity to the owner and opens the exact thread', 'keeps the thread drawer usable on a phone and preserves the channel', 'marks a joined thread read only while the conversation is visible', 'opens a shared thread message link around an older post', 'keeps an anchored older thread unread when a new reply arrives']:
                                from behavior_thread_rows import assert_thread_rows
                                with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                    assert_thread_rows(conn, seed, case, metadata)
                            elif file == "threads" and case == "discusses a pull request from its card":
                                mapping = conn.execute("SELECT channel_thread_id FROM github_pull_request_threads WHERE github_pull_request_id=? AND room_id=654632876", (metadata["pr_id"],)).fetchall()
                                assert len(mapping) == 1, "reopening a PR discussion never creates a second mapping"
                                assert conn.execute("SELECT parent_message_id,creator_id,room_id FROM channel_threads WHERE id=?", (mapping[0][0],)).fetchone() == (metadata["pr_message_id"], 773523953, 654632876)
                                assert conn.execute("SELECT markdown_source FROM messages WHERE id=?", (metadata["pr_message_id"],)).fetchone() == ("review https://github.com/rails/rails/pull/12",)
                            elif file == "code_highlighting":
                                if case.startswith(("search results", "thread code stays")):
                                    with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                        expected = seed.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall()
                                    assert conn.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall() == expected
                                else:
                                    body = ("\n\n".join(f"```{language}\n{code}\n```" for language, code in metadata["samples"]) if case.startswith("language fences") else
                                            metadata["literal_code_source"] if case.startswith("unlabelled") else
                                            metadata["code_replacement"].replace("\n", "\r\n") if case.startswith("editing") else metadata["code_source"])
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=? AND creator_id=773523953 AND room_id=654632876 AND thread_id IS NULL", (body,)).fetchone()[0] == 1, "exact saved code source"
                                    with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                        count = seed.execute("SELECT COUNT(*) FROM messages").fetchone()[0]
                                    assert conn.execute("SELECT COUNT(*) FROM messages").fetchone()[0] == count + 1
                            elif file in ["message_interactions", "message_actions_mobile", "message_toolbar"]:
                                with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                    assert_action_rows(conn, seed, file, case, metadata)
                            elif case == "sending messages between two users":
                                for body in ["Is this thing on?", "👍👍"]:
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", (body,)).fetchone()[0] == 1
                            elif case == "editing messages":
                                assert conn.execute("SELECT markdown_source FROM messages WHERE id=607264868").fetchone() == ("Redacted!",)
                            elif case == "deleting messages":
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE id=607264868").fetchone()[0] == 0
                            elif case == "Markdown messages reach other users and editing preserves the original source":
                                markdown = textwrap.dedent(source.decode().split("MARKDOWN = <<~'MARKDOWN'.freeze\n")[1].split("  MARKDOWN")[0])
                                # The actual browser edit uses multipart FormData, whose
                                # wire serialization preserves CRLF in the saved string.
                                # This is observed in pinned Rails; compare raw saved bytes
                                # on Rust as well, without normalizing either database.
                                edited = markdown.replace("Design review", "Review complete").replace("\n", "\r\n")
                                actual = conn.execute("SELECT markdown_source FROM messages WHERE markdown_source LIKE '## Review complete%'").fetchall()
                                assert actual == [(edited,)], f"{database}: saved source {actual!r}; expected {edited!r}"
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", (markdown,)).fetchone()[0] == 0
                            elif case == "desktop keyboard composition keeps line breaks and sends once after composition ends":
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", ("First line\nSecond line",)).fetchone()[0] == 1
                            elif case == "untrusted markup stays inert in the delivered message":
                                payload = textwrap.dedent(source.decode().split("payload = <<~'MARKDOWN'\n")[1].split("  MARKDOWN")[0])
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", (payload,)).fetchone()[0] == 1
                            elif case == "Markdown replies and file attachments remain usable":
                                parent = conn.execute("SELECT id FROM messages WHERE markdown_source='**A useful point** with `inline code`.'").fetchone()
                                assert parent is not None
                                attachments = conn.execute("SELECT messages.reply_notify_author,blobs.filename,blobs.byte_size,blobs.key,blobs.service_name FROM messages JOIN active_storage_attachments AS attachments ON attachments.record_type='Message' AND attachments.record_id=messages.id JOIN active_storage_blobs AS blobs ON blobs.id=attachments.blob_id WHERE messages.reply_to_message_id=?", (parent[0],)).fetchall()
                                contents = b"An attachment sent from the Markdown composer.\n"
                                assert len(attachments) == 1 and attachments[0][:3] == (0, "markdown-workspace-attachment.txt", len(contents))
                                storage = work / (f".instances/{ports[0]}/storage" if database == databases[0] else "files")
                                key = attachments[0][3]
                                rails_test_port = ports[0] if database == databases[0] and attachments[0][4] == "test" else None
                                assert uploaded_bytes(storage, key, rails_test_port=rails_test_port) == contents
                            elif case == "mention suggestions select a room member without sending the unfinished message":
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source IN ('@Kev','@[Kevin] ')").fetchone()[0] == 0
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source='@[Kevin] please review **the layout**.'").fetchone()[0] == 1
                            elif case == "a rejected message can be recovered corrected and sent":
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE length(markdown_source)>50000").fetchone()[0] == 0
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source='**Recovered** after correcting the draft.'").fetchone()[0] == 1
                            elif case == "sending preserves the submitted source and a newer draft":
                                for body in ["**First message** stays exact.", "A newer draft is still here."]:
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", (body,)).fetchone()[0] == 1
                            elif case == "creates a thread from a channel message and keeps the channel draft separate":
                                thread = conn.execute("SELECT id,parent_message_id,auto_archive_after_minutes FROM channel_threads WHERE name='Design review thread'").fetchone()
                                assert thread is not None and thread[1:] == (607264868, 1440)
                                assert conn.execute("SELECT involvement FROM thread_memberships WHERE thread_id=? AND user_id=773523953", (thread[0],)).fetchone() == ("nothing",)
                                for body in ["A reply from the thread drawer.", "A reply to the drawer message.", "The edited thread starter."]:
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE thread_id=? AND markdown_source=?", (thread[0], body)).fetchone()[0] == 1
                                reply = conn.execute("SELECT reply_to_message_id FROM messages WHERE thread_id=? AND markdown_source='A reply to the drawer message.'", (thread[0],)).fetchone()[0]
                                assert conn.execute("SELECT markdown_source FROM messages WHERE id=?", (reply,)).fetchone() == ("A reply from the thread drawer.",)
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source='A channel draft stays here.'").fetchone()[0] == 0
                                assert conn.execute("SELECT COUNT(*) FROM boosts JOIN messages ON messages.id=boosts.message_id WHERE messages.thread_id=? AND boosts.content='👍'", (thread[0],)).fetchone()[0] == 1
                            elif case == "the thread root counts its replies live and hides the count when none remain":
                                thread = conn.execute("SELECT id,parent_message_id FROM channel_threads WHERE name='Indicator thread'").fetchone()
                                assert thread is not None and thread[1] == 607264868
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE thread_id=?", (thread[0],)).fetchone()[0] == 0
                            elif case == "browses active and closed threads and can join or leave a closed one":
                                active = conn.execute("SELECT id,closed_at,creator_id FROM channel_threads WHERE name='Active planning thread'").fetchone()
                                closed = conn.execute("SELECT id,closed_at,creator_id FROM channel_threads WHERE name='Closed planning thread'").fetchone()
                                assert active is not None and active[1:] == (None, 773523953)
                                assert closed is not None and closed[1] is not None and closed[2] == 127326141
                                assert conn.execute("SELECT COUNT(*) FROM thread_memberships WHERE thread_id=? AND user_id=773523953", (closed[0],)).fetchone()[0] == 0
                                assert conn.execute("SELECT COUNT(*) FROM thread_memberships WHERE thread_id=? AND user_id=127326141", (closed[0],)).fetchone()[0] == 1
                                for thread, body in [(active, "The active planning conversation."), (closed, "The closed planning conversation.")]:
                                    assert conn.execute("SELECT markdown_source FROM messages WHERE thread_id=?", (thread[0],)).fetchall() == [(body,)]
                            elif case == "rejects an external thread deep link before fetching it":
                                for table, count in protected_counts.items():
                                    assert conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0] == count
                            elif case == "renders untrusted thread metadata as text":
                                name = '<img src=x onerror="window.__threadXss = true">'
                                thread = conn.execute("SELECT id FROM channel_threads WHERE name=?", (name,)).fetchone()
                                assert thread is not None
                                assert conn.execute("SELECT markdown_source FROM messages WHERE thread_id=?", (thread[0],)).fetchall() == [("A safe thread body.",)]
                            elif case == "a stray create re-entry does not wipe the half-filled thread name":
                                thread = conn.execute("SELECT id FROM channel_threads WHERE name='Survives a stray reset'").fetchone()
                                assert thread is not None
                                assert conn.execute("SELECT markdown_source FROM messages WHERE thread_id=?", (thread[0],)).fetchall() == [("The name survives the re-entry.",)]
                            elif file == "message_list_a11y":
                                if case.startswith("flash "):
                                    bio = "Reduced motion flash check" if case.startswith("flash persists") else "Reduced motion dismiss check"
                                    assert conn.execute("SELECT bio FROM users WHERE id=773523953").fetchone() == (bio,)
                                if case == "an edit replacement is not announced as an addition":
                                    assert conn.execute("SELECT markdown_source FROM messages WHERE id=607264868").fetchone() == ("Edited quietly",)
                                elif case == "an own message is not re-announced when its broadcast replaces the pending copy":
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source='Announce me once'").fetchone()[0] == 1
                                else:
                                    with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                        expected = seed.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall()
                                    assert conn.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall() == expected, "navigation/focus/history must not change saved messages"
                            elif file == "search_forward_edit":
                                if case.startswith("search tolerates"):
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source LIKE 'system paging %'").fetchone()[0] == 42
                                    with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                        expected = seed.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall()
                                    assert conn.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall() == expected
                                elif case.startswith("editing to add"):
                                    assert conn.execute("SELECT markdown_source,edited_at IS NOT NULL FROM messages WHERE id=?", (metadata["edit_card_id"],)).fetchone() == ("now with https://x.com/jack/status/424242", 1)
                                    assert conn.execute("SELECT posts.post_id FROM twitter_post_references refs JOIN twitter_posts posts ON posts.id=refs.twitter_post_id WHERE refs.message_id=?", (metadata["edit_card_id"],)).fetchall() == [("424242",)]
                                    if database == databases[1]:
                                        held=conn.execute("SELECT status,attempts FROM background_jobs WHERE job_class='Twitter::FetchPostJob'").fetchall()
                                        assert held and all(row==('ready',0) for row in held), "URL jobs enqueue durably but never execute under the pinned test boundary"
                                        print(f"WS8bm held URL jobs: Rust {len(held)} ready; 0 attempts",flush=True)
                                else:
                                    source_row = conn.execute("SELECT id,markdown_source FROM messages WHERE client_message_id='system-forward-source'").fetchone()
                                    assert source_row is not None
                                    copies = conn.execute("SELECT id,markdown_source,forwarded_markdown,room_id FROM messages WHERE forwarded_from_message_id=?", (source_row[0],)).fetchall()
                                    assert len(copies) == 1 and copies[0][1:] == (None, 1, 654632876), "Rails forwards store a rendered snapshot, not source Markdown"
                                    original = conn.execute("SELECT body FROM action_text_rich_texts WHERE record_type='Message' AND record_id=? AND name='body'", (source_row[0],)).fetchone()
                                    assert conn.execute("SELECT body FROM action_text_rich_texts WHERE record_type='Message' AND record_id=? AND name='body'", (copies[0][0],)).fetchone() == original
                            elif file == "unread_divider":
                                with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                    expected = seed.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall()
                                assert conn.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall() == expected
                            elif file == "composer":
                                if case.startswith("blurring"):
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source='hello'").fetchone()[0] == 1
                                elif case.startswith("clicking a reply") or case.startswith("deleting a replied"):
                                    body = {"clicking a reply preview scrolls to the loaded message instead of navigating": "A reply for preview click",
                                            "clicking a reply preview falls back to the permalink when the target is not loaded": "A reply for preview fallback",
                                            "deleting a replied-to message turns open reply previews into a tombstone": "A reply whose source goes away"}[case]
                                    parent = None if case.startswith("deleting") else 607264868
                                    assert conn.execute("SELECT reply_to_message_id FROM messages WHERE markdown_source=?", (body,)).fetchall() == [(parent,)]
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE id=607264868").fetchone()[0] == (0 if case.startswith("deleting") else 1)
                                elif case.startswith("composer drafts"):
                                    assert conn.execute("SELECT room_id FROM messages WHERE markdown_source='Pets draft'").fetchall() == [(metadata["pets_id"],)]
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source='Designers draft'").fetchone()[0] == 0
                                elif case.startswith("thread drafts"):
                                    thread = conn.execute("SELECT id FROM channel_threads WHERE name='Composer draft thread'").fetchone()
                                    assert thread is not None
                                    for body in ["The thread for draft persistence.", "Thread draft sent"]:
                                        assert conn.execute("SELECT thread_id FROM messages WHERE markdown_source=?", (body,)).fetchall() == [(thread[0],)]
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source IN ('Channel draft','Thread draft')").fetchone()[0] == 0
                                else:
                                    with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                        expected = seed.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall()
                                    assert conn.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall() == expected
                            elif file == "composer_attach_menu":
                                with sqlite3.connect(fixture / "db/production.sqlite3") as seed:
                                    expected = seed.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall()
                                    blobs = {row[0] for row in seed.execute("SELECT id FROM active_storage_blobs")}
                                assert conn.execute("SELECT id,markdown_source FROM messages ORDER BY id").fetchall() == expected
                                actual_blobs = {row[0] for row in conn.execute("SELECT id FROM active_storage_blobs")}
                                # Live apps may purge old unattached seed blobs. The
                                # invariant is no new uploads, not suppressing purge.
                                # Loading seeded image messages can legitimately create a
                                # tracked variant (observed on pinned Rails). Only
                                # derivatives of seed blobs may add storage rows.
                                variants = {blob_id for blob_id, parent_id in conn.execute("SELECT attachment.blob_id,variant.blob_id FROM active_storage_attachments attachment JOIN active_storage_variant_records variant ON variant.id=attachment.record_id WHERE attachment.record_type='ActiveStorage::VariantRecord' AND attachment.name='image'") if parent_id in blobs}
                                added_blobs = actual_blobs - blobs - variants
                                if added_blobs:
                                    rows = [(blob_id, conn.execute("SELECT filename,content_type,byte_size FROM active_storage_blobs WHERE id=?", (blob_id,)).fetchone(), conn.execute("SELECT record_type,record_id,name FROM active_storage_attachments WHERE blob_id=?", (blob_id,)).fetchall()) for blob_id in sorted(added_blobs)]
                                    print(f"WS8bm unsent blob diagnostics: {database}: {rows!r}", flush=True)
                                assert not added_blobs, "previews/pickers must not upload unsent files"
                            elif file == "boosting_messages":
                                if case == "boosting a message":
                                    assert conn.execute("SELECT booster_id FROM boosts WHERE message_id=607264868 AND content='Good morning'").fetchall() == [(712064548,)]
                                elif case == "deleting a boost":
                                    assert conn.execute("SELECT COUNT(*) FROM boosts WHERE content='Hello'").fetchone()[0] == 0
                                elif case.startswith("message update"):
                                    assert conn.execute("SELECT markdown_source FROM messages WHERE id=607264868").fetchone() == ("Redacted!",)
                                else:
                                    assert conn.execute("SELECT booster_id FROM boosts WHERE message_id=607264868 AND content='Morning'").fetchall() == [(127326141,)]
                                assert conn.execute("SELECT COUNT(*) FROM boosts WHERE content='Hey!'").fetchone()[0] == 0, "another viewer's stream never submits the unfinished boost"
                    for case in succeeded:
                        passed += 1
                        print(f"WS8bm behaviour: {file}: {case}: Rails PASS; Rust PASS; persisted rows PASS", flush=True)
                finally:
                    stop_behavior_servers(process, reference, ports[0], cwd=ROOT, env=run_env, log=log)
    print(f"WS8bm behaviour source: {source_path} SHA256 {hashlib.sha256(source).hexdigest()}", flush=True)
if args.negative:
    print(f"WS8bm invalid discrimination attempts: {invalid_attempts}; bounded fresh-fixture retries only",flush=True)
    print(f"WS8bm discrimination check: {passed} served mutants rejected on Rails and Rust across {len(passed_named)} named checks; {len(failed_cases)} invalid or escaped", flush=True)
elif args.mutant or args.mutant_set:
    print(f"WS8bm review escape check: {passed} served mutants accepted on Rails and Rust; {len(failed_cases)} failed probes; no parity credit", flush=True)
else:
    if args.repeat>1:
        print(f"WS8bm behaviour repetition: {passed} paired attempts; 1 named declaration; {len(failed_cases)} failed",flush=True)
    else:
        print(f"WS8bm behaviour check: {passed} named cases passed on Rails and Rust; {len(failed_cases)} failed; no pixel checks", flush=True)
if failed_cases:
    print("WS8bm failed named checks:\n" + "\n".join(failed_cases), flush=True)
    raise SystemExit(1)
