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
        status, owner, evidence = "deferred", "WS12", "Outside the completed stars, inbox-access/state and board room/list/row slices."
        if "activity_items_controller" in relative or "activity_items_helper" in relative or "system/activity_inbox" in relative:
            owner, evidence = "WS11-UI controller/rendering + WS12 domain integration", "Merge WS11-UI and replace its flagged activity adapters with ActivityItem domain APIs; verify full response bytes and inbox interactions."
        elif any(part in relative for part in ("channel_thread_agent_assignment", "agents/work", "agent_boards", "agent_working_presence", "agent_work_assignment", "agents/posts_controller", "agents/mcp_handoff")):
            owner,evidence = "WS12 after WS11 #176", "Agent work services and eligible-agent owner integration remain pending the lead's WS11 merge."
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
            elif "agent owner" in title or "owner availability" in title:
                owner,evidence="WS12 after WS11 #176","Read-only owner labels/filtering match the actual Rails seed and revoked human membership vectors; the original agent owner/grant mutation and candidate assertions wait for WS11."
            else:
                evidence="WS12 board post/result/work slice. Existing tracking validation is retained, but removing tracking, comma-separated HTTP tag input, result writes/events/recipients and run URL writes are not closed by board listing tests."
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
        cases.append(dict(file=relative,line=line,test=title,status=status,owner=owner,evidence=evidence))
out = root / "rust/plans/ws12-rails-cases.json"
out.write_text(json.dumps(dict(reference="d7c7de92; approved board drift uses origin/main on continuation",partial=True,cases=cases),indent=2)+"\n")
counts = {status:sum(c["status"]==status for c in cases) for status in sorted({c["status"] for c in cases})}
print(f"WS12 Rails inventory: {len(cases)} declarations; " + "; ".join(f"{n} {s}" for s,n in counts.items()))
