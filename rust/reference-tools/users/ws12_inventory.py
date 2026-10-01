#!/usr/bin/env python3
"""List every owned Rails declaration and its port/defer boundary for this partial WS12 slice."""
from pathlib import Path
import json
import re

root = Path(__file__).resolve().parents[3]
patterns = [
    "test/models/rooms/board_test.rb", "test/models/board_*test.rb", "test/models/board_automations/*test.rb",
    "test/models/channel_thread_board_test.rb", "test/models/channel_thread_auto_assign_test.rb",
    "test/models/channel_thread_agent_assignment_test.rb", "test/models/channel_thread_handoff_test.rb",
    "test/models/message_board_test.rb", "test/models/work_*test.rb", "test/models/thread_tag_test.rb",
    "test/models/activity_item*test.rb", "test/models/user_star_test.rb",
    "test/models/agents/work_payload_test.rb", "test/models/agent_working_presence_test.rb",
    "test/services/activity_items/*test.rb", "test/services/agents/board_posts*test.rb",
    "test/services/agents/work_*test.rb", "test/controllers/rooms/boards*test.rb",
    "test/controllers/rooms/boards/*test.rb", "test/controllers/channel_threads_board_test.rb",
    "test/controllers/work_threads*test.rb", "test/controllers/threads/work/*test.rb",
    "test/controllers/users/stars_controller_test.rb", "test/controllers/users/sidebars_boards_test.rb",
    "test/controllers/activity_items_controller_test.rb", "test/controllers/agents/work*test.rb",
    "test/controllers/agents/posts_controller_test.rb", "test/controllers/agents/mcp_handoff_test.rb",
    "test/integration/agent_boards_test.rb", "test/integration/work_thread_links_test.rb",
    "test/jobs/board_automations/*test.rb", "test/jobs/room/destroy_job_board_automations_test.rb",
    "test/channels/activity_channel_test.rb", "test/helpers/activity_items_helper_test.rb",
    "test/system/boards_test.rb", "test/system/board_automations_test.rb",
    "test/system/activity_inbox_test.rb", "test/system/starred_people_test.rb", "test/system/agent_work_assignment_test.rb",
]
files = sorted({p for pattern in patterns for p in root.glob(pattern)})
work_cases = set(range(304,518))
files.append(root / "test/controllers/channel_threads_controller_test.rb")
cases = []
for path in files:
    relative = str(path.relative_to(root))
    for line, text in enumerate(path.read_text().splitlines(),1):
        match = re.match(r'\s*test "(.*?)" do',text)
        if not match or (path.name=="channel_threads_controller_test.rb" and line not in work_cases):
            continue
        title = match[1]
        status, owner, evidence = "deferred", "WS12", "Outside the completed stars, inbox-access/state, board room/list/post and human work mutation slices."
        if "activity_items_controller" in relative or "activity_items_helper" in relative or "system/activity_inbox" in relative:
            owner, evidence = "WS11-UI controller/rendering + WS12 domain integration", "Merge WS11-UI and replace its flagged activity adapters with ActivityItem domain APIs; verify full response bytes and inbox interactions."
        elif any(part in relative for part in ("channel_thread_agent_assignment", "agents/work", "agent_boards", "agent_working_presence", "agent_work_assignment", "agents/posts_controller", "agents/mcp_handoff")):
            owner,evidence = "WS12 using merged WS11 #176", "WS11 #176 is merged. Owner eligibility and assignment ledgers use its real APIs; the remaining agent work services/declarations are still WS12-owned and unblocked."
        elif relative=="test/models/user_star_test.rb":
            status,evidence="ported","crates/db/src/tests/user_star_test.rs (5 discriminating model/independent-writer tests)."
        elif relative=="test/controllers/rooms/boards_controller_test.rb":
            status,evidence="ported","controllers/rooms/boards_rails_cases.rs: all 16 declarations, actual sessions, CSRF, membership/audit rows and real Cable; boards_read_tests.rs: full Rails form/error responses."
        elif relative=="test/controllers/users/sidebars_boards_test.rb":
            status,evidence="ported","controllers/rooms/boards_rails_cases.rs: sidebar section order, unread viewer/creator rows, creation policy."
        elif relative=="test/models/rooms/board_test.rb":
            status,evidence="ported","crates/db/src/tests/board_test.rs: exclusive type predicates, board/non-direct scopes, mentions defaults and user deactivation."
        elif relative=="test/models/thread_tag_test.rb":
            status,evidence="ported","crates/db/src/tests/board_test.rs: real required parent, syntax/length, per-post uniqueness; channel_thread_test.rs retains original tag tests."
        elif relative=="test/models/channel_thread_board_test.rb":
            completed = {
                "close_stale_in skips board posts but still archives channel threads",
                "tag writer accepts an array of names as well as a comma-separated string",
                "tag count, length, and format are validated without destroying existing tags",
                "tag edits replace the previous set",
                "board post query filters by status, owner, and tag",
                "board posts page cumulatively at fifty per page with a clamped page",
                "board page counts group replies and links without per-row queries",
                "board posts skip muted members for unread and broadcast only to marked members",
            }
            if title in completed:
                status,evidence="ported","crates/db/src/tests/board_test.rs, controllers/rooms/boards_domain_tests.rs and actual Rails boards_domain.json. Tags array normalization/clearing is tested; HTTP comma-separated tag coercion remains with board post endpoints."
            elif "owner availability" in title:
                status,evidence="ported","crates/db/src/tests/work_read_test.rs covers every assigned owner kind, membership removal and revoked agent grants, plus HTTP owner labels."
            elif "agent owner" in title:
                owner,evidence="WS12 using merged WS11 #176","Eligible owner reads and assignment writes exist. This declaration has not been mapped to a complete assertion set; its remaining combined scenarios are still WS12-owned."
            else:
                evidence="WS12 board post/result/work slice. Completed mutation cases are mapped below; this remaining declaration needs its own complete assertion set (including run URL and other combined scenarios)."
        elif relative=="test/controllers/channel_threads_board_test.rb":
            if title in {"new post form renders in boards and 404s in channels and for non-members", "post page shows the board header, result, and manage controls without tracking controls"}:
                status,evidence="ported","channel_threads/board_read_tests.rs: 29 complete Rails HTTP bodies, including new forms, pages, permissions, links/history/result, pane/anchors and missing memberships."
        elif relative=="test/controllers/users/stars_controller_test.rb":
            status,evidence="ported","controllers/users/stars_tests.rs (7 HTTP tests, 21 Rails JSON responses, 2 fragment/stream byte vectors)."
        elif relative=="test/system/starred_people_test.rb":
            status,evidence="ported","reference-tools/users/browser_stars.mjs: matching four browser interactions on Rails and Rust, including original inline-row and phone-overflow assertions."
        elif relative=="test/models/activity_item_test.rb":
            if "broadcast carries" in title or "invitation to missed" in title:
                owner,evidence="WS13 preserved + WS12 integration","Merged WS13b huddle broadcasts/ring jobs preserved. Its complete callback/banner integration is outside the new WS12 access/state assertions."
            else:
                status,evidence="ported","controllers/activity_domain_tests.rs: exact access IDs/counts/filters across all 11 types, FrozenClock state rows, commit/rollback signals and tuple cursors."
        elif relative=="test/models/activity_item_approvals_test.rb" and "non-deciders never receive" not in title:
            status,evidence="ported","activity_domain.json all-source/owner/active-user matrix compared by activity_domain_tests.rs; inbox fanout remains the recorder's separate scope."
        elif relative=="test/channels/activity_channel_test.rb":
            status,owner,evidence="existing peer tests","WS7","channels/tests/reference_test.rs already tests own-stream subscription and bot/inactive rejection; WS12 keeps that implementation."
        elif "services/activity_items" in relative:
            owner,evidence="WS12 with existing WS17 message candidates","Full generic recorder/source authorization and work grouping/recipient hooks remain. Existing message-recorder concurrency is checked by WS12, not a claim that every original declaration is closed."
        board_writes = {
            "creates a post with a first message, owner, status, and tags",
            "creates a post without a first message and defaults to planned",
            "creates a post with an agent owner through the assignment path",
            "rejects posts with invalid titles, owners, statuses, and tags",
            "non-members cannot create posts",
            "a new post notifies everything-followers and always the human owner",
            "a messageless post with an owner still writes the assignment event and notifies the owner",
            "post owner, creator, board creator, and admins can change the status",
            "post creator, board creator, and admins can assign the owner",
            "post owner can edit the title and tags like the status managers",
            "result edits follow the status rule and write a result_updated event",
            "stopping work tracking is rejected for posts",
            "auto-archive changes are rejected for posts",
            "only the board creator and admins can close, lock, or delete a post",
            "post payloads do not advertise removing work tracking",
        }
        if relative=="test/controllers/channel_threads_board_test.rb" and title in board_writes:
            status,owner,evidence="ported","WS12","channel_threads/board_write_tests.rs: complete Rails HTTP write responses, actual opener recipients and atomic queue-failure rollback. work_mutations_test.rs: creation assignment/inbox, permission, event/result and real WS11 ledger writes."
        if relative=="test/models/channel_thread_board_test.rb" and title in {
            "board posts require work tracking while channel threads stay optional",
            "tag writer normalises names",
            "result edits record a result_updated event with an excerpt",
            "result edits notify the creator and owner other than the actor",
            "result edits require status permission and respect the length limit",
        }:
            status,owner,evidence="ported","WS12","work_mutations_test.rs and board_write_tests.rs: real conversion/removal rejection, tag input, result Unicode excerpt/limits/clear/no-op, audit snapshots and current recipient rules."
        if relative=="test/controllers/channel_threads_controller_test.rb" and title in {
            "converts a thread to work, assigns an eligible owner, and keeps an audit trail",
            "assigned owner can change work status but cannot reassign it",
            "only a thread manager can remove work tracking",
            "the work model also protects conversion when the owner field is omitted",
            "work status updates from separate stale instances produce one event per real change",
            "a manager can assign an eligible agent and the agent is notified",
            "a member who cannot manage the thread cannot assign an agent",
        }:
            status,owner,evidence="ported","WS12","work_mutations_test.rs: actual manager/owner mutations, independent SQLite writers, stamps/no-ops, audit/agent-ledger failure rollback; boards_write.json compares ordinary conversion/removal HTTP responses."
        if relative=="test/models/channel_thread_agent_assignment_test.rb" and title in {
            "a legacy agent keeps post eligibility through the fallback",
            "a suspended agent is rejected with a validation error",
            "unassignment after the agent left the room writes the ledger row but enqueues no webhook",
            "assignment writes one work_assigned row in the same transaction as the work event",
            "the work event rolls back when the assignment row fails",
            "assignment rows roll back when the work event fails",
        }:
            status,owner,evidence="ported","WS12 using merged WS11 #176","work_mutations_test.rs: real owner writer, legacy eligibility, suspension, ledger assignment/unassignment after access loss, and failing event/ledger triggers. Existing WS11 APIs supply ledger/webhook behavior."
        if relative=="test/services/activity_items/recorder_test.rb" and title in {
            "work updates for one thread collapse into a single item",
            "work assigned by a bot without an agent ignores the agent_work switch",
            "recording the same source twice is idempotent",
        }:
            status,owner,evidence="ported","WS12","work_mutations_test.rs: work grouping/repointing/read reset, handled-source idempotency across independent SQLite writers and real Agent-row preference gating. Full original message/keyword query and source matrix remains separate."
        if relative=="test/system/boards_test.rb" and title=="replying on a board post sends and clears the composer":
            status,owner,evidence="ported","WS12","reference-tools/boards/write_browser.mjs: actual signed-session Markdown reply form, rendered message, cleared composer and persisted plain text through the JSON read on both Rails and Rust."
        if relative=="test/system/boards_test.rb" and title=="board pages align to the top under the header":
            status,owner,evidence="ported","WS12","Body-class assertions are covered by full board post/new-page response comparisons and write_browser.mjs. Geometric assertions are excluded from acceptance by the pixel-phase cut in wave4/_common.md and decisions.md; no pixel work is deferred."
        cases.append(dict(file=relative,line=line,test=title,status=status,owner=owner,evidence=evidence))
out = root / "rust/plans/ws12-rails-cases.json"
out.write_text(json.dumps(dict(reference="d7c7de92; approved board drift uses origin/main on continuation",partial=True,cases=cases),indent=2)+"\n")
counts = {status:sum(c["status"]==status for c in cases) for status in sorted({c["status"] for c in cases})}
print(f"WS12 Rails inventory: {len(cases)} declarations; " + "; ".join(f"{n} {s}" for s,n in counts.items()))
