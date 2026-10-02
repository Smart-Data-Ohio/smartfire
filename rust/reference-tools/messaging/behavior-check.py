#!/usr/bin/env python3
"""Seeded Rails/Rust behaviour checks. Builds its own seeds, binary and browser inputs.

Each writing case gets independent copies of the seed. Read-only message-list
regressions share one verified fixture/server but get fresh viewer contexts.
All cases exercise real HTTP/Action Cable. No pre-existing target/scratch, screenshot or response mask.
"""
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
from behavior_mutation_jobs import probe_jobs

ROOT = Path(__file__).resolve().parents[3]
RUST = ROOT / "rust"
SCRATCH = ROOT / ".scratch"
PIN = "d7c7de9264c63015be398001d7a1094e7695a6db"
CASES = {
    "sending_messages": ["sending messages between two users", "editing messages", "deleting messages"],
    "workspace_markdown": [
        "Markdown messages reach other users and editing preserves the original source",
        "desktop keyboard composition keeps line breaks and sends once after composition ends",
        "untrusted markup stays inert in the delivered message",
        "Markdown replies and file attachments remain usable",
        "mention suggestions select a room member without sending the unfinished message",
        "a rejected message can be recovered corrected and sent",
        "sending preserves the submitted source and a newer draft",
    ],
    "threads": [
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
        "language fences highlight common code without changing its text",
        "unlabelled code is detected while text unknown languages and inline code stay literal",
        "search results highlight code on initial load and after returning to the channel",
        "code and copying remain available when the highlighter cannot load",
        "editing a code block replaces its language colors and copied source",
    ],
}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("files", nargs="*", choices=CASES)
parser.add_argument("--case", help="run one exact pinned declaration from the selected files")
parser.add_argument("--exclude-case", action="append", default=[], help="explicitly omit an unresolved mapped declaration; default still runs it")
parser.add_argument("--negative", action="store_true", help="require each selected case to reject its deliberately broken served implementation")
parser.add_argument("--mutant", help="select one served mutant variant; without --negative, diagnose its acceptance on both apps (not parity credit)")
parser.add_argument("--mutant-set", choices=["visible-assertions", "visible-lookups", "instantaneous-opacity", "element-scopes", "hidden-scopes", "categories"], help="diagnose all new visibility assertion mutants without parity credit")
parser.add_argument("--keep-going", action="store_true", help="report every selected flow; failures still produce a nonzero exit")
args = parser.parse_args()
files = args.files or list(CASES)
if args.mutant_set:
    assert not args.mutant and not args.case and (not args.negative or args.mutant_set in {"element-scopes", "categories"}), "only element-scopes/categories support a negative mutant-set run"
    diagnostic_export = {"visible-lookups": "visibilityLookupMutations", "instantaneous-opacity": "instantaneousOpacityMutations", "element-scopes": "elementScopeMutations", "hidden-scopes": "hiddenScopeProbes", "categories": "categoryMutations"}.get(args.mutant_set, "visibilityAssertionMutations")
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
           PARITY_NAMESPACE="ws8bm-behavior", PARITY_OWNER="ws8bm", TMPDIR=str(SCRATCH))
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
subprocess.run(["bash", "rust/parity/bin/seed", "build", "default", "first_run"], cwd=ROOT, env=env, check=True)
subprocess.run(["mise", "exec", "rust@1.98.1", "--", "cargo", "build", "--locked", "-j2", "--manifest-path", "rust/Cargo.toml", "-p", "campfire", "--bin", "campfire"], cwd=ROOT, env=env, check=True)
subprocess.run(["npm", "ci", "--prefix", "rust/parity"], cwd=ROOT, check=True)
subprocess.run(["npm", "exec", "--prefix", "rust/parity", "--", "playwright", "install", "chromium"], cwd=ROOT, check=True)
browser_image = "ws8bm-browser-reference-d7c7de92"
subprocess.run(["docker", "build", "--build-arg", f"BASE_IMAGE={image}", "-f", str(RUST / "reference-tools/rooms/browser.Dockerfile"), "-t", browser_image, str(RUST / "parity/docker")], cwd=ROOT, check=True)
env["PARITY_IMAGE"] = browser_image
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
ports = [52020, 52021, 52022]
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
    source = subprocess.check_output(["git", "show", f"{PIN}:test/system/{file}_test.rb"], cwd=ROOT)
    selected = [case for case in CASES[file] if (not args.case or case == args.case) and case not in args.exclude_case]
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
        batches = [selected]  # No server writes; new contexts for each case.
    jobs = probe_jobs(batches, selected, negative=args.negative, mutant=args.mutant,
                      mutation_variants=mutation_variants,
                      diagnostic_variants=diagnostic_variants if args.mutant_set else None)
    for batch, variant in jobs:
        case = batch[0]
        for name in batch:
            assert f'test "{name}"'.encode() in source, "case must be named in the pin"
        with tempfile.TemporaryDirectory(prefix="ws8bm-behavior-", dir=SCRATCH) as directory:
            work = Path(directory)
            fixture = work / "fixture"
            shutil.copytree(RUST / "parity/.seed/default", fixture)
            if file == "message_list_a11y":
                fixture_kind = "history" if case.startswith("paginated history") else "message_list"
                if any(name in CASES[file][20:] for name in batch):
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
            elif file == "composer_attach_menu":
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), "attach-menu"], cwd=ROOT, env=env, check=True)
            elif file == "boosting_messages":
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), "boosts"], cwd=ROOT, env=env, check=True)
            elif file in ["message_interactions", "message_actions_mobile", "message_toolbar"]:
                fixture_kind = {"message_interactions": "interactions", "message_actions_mobile": "actions-mobile", "message_toolbar": "toolbar"}[file]
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), fixture_kind], cwd=ROOT, env=env, check=True)
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
                fixture_kind = "highlight-search" if case.startswith("search results") else "highlight"
                subprocess.run([reference, "runner", "--storage", str(fixture), "--time", "2026-03-02T16:00:00Z", "--freeze",
                                str(RUST / "reference-tools/messaging/behavior-fixtures.rb"), fixture_kind], cwd=ROOT, env=env, check=True)
            shutil.copytree(fixture / "db", work / "db")
            shutil.copytree(fixture / "storage", work / "files")
            run_env = dict(env, CAMPFIRE_STORAGE_PATH=str(work), HTTP_PORT=str(ports[1]), TARGET_PORT=str(ports[2]), PARITY_SEED_DIR=str(work))
            if args.negative or args.mutant_set:
                run_env["WS8BM_MUTANT"] = variant
            if file == "composer_attach_menu":
                run_env.update(GOOGLE_CLIENT_ID="test-client-id", GOOGLE_CLIENT_SECRET="test-client-secret")
            process = None
            with (SCRATCH / "ws8bm-behavior-servers.log").open("a") as log:
                try:
                    subprocess.run([reference, "up", "--seed", "fixture", "--port", str(ports[0]), "--time", "2026-03-02T16:00:00Z", "--freeze"], cwd=ROOT, env=run_env, stdout=log, stderr=log, check=True)
                    process = subprocess.Popen([str(target / "debug/campfire"), "server"], cwd=ROOT, env=run_env, stdout=log, stderr=log)
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
                    if result.returncode and not args.keep_going:
                        raise subprocess.CalledProcessError(result.returncode, command)
                    if args.negative:
                        variants = json.loads(subprocess.check_output(["node", "--input-type=module", "-e",
                            "import {mutationVariants} from './rust/reference-tools/messaging/behavior-mutations.mjs'; "
                            f"console.log(JSON.stringify({json.dumps(batch)}.map(name=>mutationVariants(name))))"], cwd=ROOT, env=run_env, text=True))
                        succeeded = [name for name, names in zip(batch, variants) if names and all(
                            any(line.startswith(f"WS8bm discrimination: {file}: {name}: {variant}: {app} served mutant REJECTED (") for line in result.stdout.splitlines())
                            for variant in names for app in ["Rails", "Rust"])]
                    elif args.mutant or args.mutant_set:
                        succeeded = [name for name in batch if all(
                            f"WS8bm review escape: {file}: {name}: {variant}: {app} ACCEPTED" in result.stdout.splitlines() for app in ["Rails", "Rust"])]
                    else:
                        succeeded = [name for name in batch if f"WS8bm browser flow: {file}: {name}: Rails PASS; Rust PASS" in result.stdout.splitlines()]
                    failed_cases.extend(f"{file}: {name}" for name in batch if name not in succeeded)
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
                    for database in databases:
                        with sqlite3.connect(database) as conn:
                            if file == "threads" and case == "discusses a pull request from its card":
                                mapping = conn.execute("SELECT channel_thread_id FROM github_pull_request_threads WHERE github_pull_request_id=? AND room_id=654632876", (metadata["pr_id"],)).fetchall()
                                assert len(mapping) == 1, "reopening a PR discussion never creates a second mapping"
                                assert conn.execute("SELECT parent_message_id,creator_id,room_id FROM channel_threads WHERE id=?", (mapping[0][0],)).fetchone() == (metadata["pr_message_id"], 773523953, 654632876)
                                assert conn.execute("SELECT markdown_source FROM messages WHERE id=?", (metadata["pr_message_id"],)).fetchone() == ("review https://github.com/rails/rails/pull/12",)
                            elif file == "code_highlighting":
                                if case.startswith("search results"):
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
                            elif case == CASES["workspace_markdown"][0]:
                                markdown = textwrap.dedent(source.decode().split("MARKDOWN = <<~'MARKDOWN'.freeze\n")[1].split("  MARKDOWN")[0])
                                # The actual browser edit uses multipart FormData, whose
                                # wire serialization preserves CRLF in the saved string.
                                # This is observed in pinned Rails; compare raw saved bytes
                                # on Rust as well, without normalizing either database.
                                edited = markdown.replace("Design review", "Review complete").replace("\n", "\r\n")
                                actual = conn.execute("SELECT markdown_source FROM messages WHERE markdown_source LIKE '## Review complete%'").fetchall()
                                assert actual == [(edited,)], f"{database}: saved source {actual!r}; expected {edited!r}"
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", (markdown,)).fetchone()[0] == 0
                            elif case == CASES["workspace_markdown"][1]:
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", ("First line\nSecond line",)).fetchone()[0] == 1
                            elif case == CASES["workspace_markdown"][2]:
                                payload = textwrap.dedent(source.decode().split("payload = <<~'MARKDOWN'\n")[1].split("    MARKDOWN")[0])
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", (payload,)).fetchone()[0] == 1
                            elif case == "Markdown replies and file attachments remain usable":
                                parent = conn.execute("SELECT id FROM messages WHERE markdown_source='**A useful point** with `inline code`.'").fetchone()
                                assert parent is not None
                                attachments = conn.execute("SELECT messages.reply_notify_author,blobs.filename,blobs.byte_size,blobs.key FROM messages JOIN active_storage_attachments AS attachments ON attachments.record_type='Message' AND attachments.record_id=messages.id JOIN active_storage_blobs AS blobs ON blobs.id=attachments.blob_id WHERE messages.reply_to_message_id=?", (parent[0],)).fetchall()
                                contents = b"An attachment sent from the Markdown composer.\n"
                                assert len(attachments) == 1 and attachments[0][:3] == (0, "markdown-workspace-attachment.txt", len(contents))
                                storage = work / (f".instances/{ports[0]}/storage" if database == databases[0] else "files")
                                key = attachments[0][3]
                                assert (storage / key[:2] / key[2:4] / key).read_bytes() == contents
                            elif case == "mention suggestions select a room member without sending the unfinished message":
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source IN ('@Kev','@[Kevin] ')").fetchone()[0] == 0
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source='@[Kevin] please review **the layout**.'").fetchone()[0] == 1
                            elif case == "a rejected message can be recovered corrected and sent":
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE length(markdown_source)>50000").fetchone()[0] == 0
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source='**Recovered** after correcting the draft.'").fetchone()[0] == 1
                            elif case == "sending preserves the submitted source and a newer draft":
                                for body in ["**First message** stays exact.", "A newer draft is still here."]:
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source=?", (body,)).fetchone()[0] == 1
                            elif case == CASES["threads"][0]:
                                thread = conn.execute("SELECT id,parent_message_id,auto_archive_after_minutes FROM channel_threads WHERE name='Design review thread'").fetchone()
                                assert thread is not None and thread[1:] == (607264868, 1440)
                                assert conn.execute("SELECT involvement FROM thread_memberships WHERE thread_id=? AND user_id=773523953", (thread[0],)).fetchone() == ("nothing",)
                                for body in ["A reply from the thread drawer.", "A reply to the drawer message.", "The edited thread starter."]:
                                    assert conn.execute("SELECT COUNT(*) FROM messages WHERE thread_id=? AND markdown_source=?", (thread[0], body)).fetchone()[0] == 1
                                reply = conn.execute("SELECT reply_to_message_id FROM messages WHERE thread_id=? AND markdown_source='A reply to the drawer message.'", (thread[0],)).fetchone()[0]
                                assert conn.execute("SELECT markdown_source FROM messages WHERE id=?", (reply,)).fetchone() == ("A reply from the thread drawer.",)
                                assert conn.execute("SELECT COUNT(*) FROM messages WHERE markdown_source='A channel draft stays here.'").fetchone()[0] == 0
                                assert conn.execute("SELECT COUNT(*) FROM boosts JOIN messages ON messages.id=boosts.message_id WHERE messages.thread_id=? AND boosts.content='👍'", (thread[0],)).fetchone()[0] == 1
                            elif case == CASES["threads"][1]:
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
                            elif case == CASES["threads"][2]:
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
                    if process is not None:
                        process.terminate()
                        try:
                            process.wait(timeout=15)
                        except subprocess.TimeoutExpired:
                            process.kill()
                            process.wait()
                    subprocess.run([reference, "down", "--port", str(ports[0])], cwd=ROOT, env=run_env, stdout=log, stderr=log, check=True)
    print(f"WS8bm behaviour source: test/system/{file}_test.rb SHA256 {hashlib.sha256(source).hexdigest()}", flush=True)
if args.negative:
    print(f"WS8bm discrimination check: {passed} served mutants rejected on Rails and Rust across {len(passed_named)} named checks; {len(failed_cases)} invalid or escaped", flush=True)
elif args.mutant or args.mutant_set:
    print(f"WS8bm review escape check: {passed} served mutants accepted on Rails and Rust; {len(failed_cases)} failed probes; no parity credit", flush=True)
else:
    print(f"WS8bm behaviour check: {passed} named cases passed on Rails and Rust; {len(failed_cases)} failed; no pixel checks", flush=True)
if failed_cases:
    print("WS8bm failed named checks:\n" + "\n".join(failed_cases), flush=True)
    raise SystemExit(1)
