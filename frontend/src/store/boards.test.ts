import { describe, expect, it } from "vitest";
import { meFixture } from "../api/testing.ts";
import type { AgentStep } from "../gen/AgentStep.ts";
import type { Thread } from "../gen/Thread.ts";
import { BOARD, boardDetail, boardListing, boardThread } from "../test/board-fixtures.ts";
import {
  addBoardPost,
  boardColumns,
  boardPostIds,
  defaultBoardQuery,
  loadBoardListing,
  setBoardLoading,
} from "./boards.ts";
import { applyEvents } from "./reducers.ts";
import { initialState, type State } from "./state.ts";
import { loadThreadDetail, MAX_REMOVED_THREADS } from "./threads.ts";

const threads = [
  boardThread(1, "planned", 7, ["api"]),
  boardThread(2, "in_progress", 8, ["design"]),
  boardThread(3, "blocked", 9, ["api", "bug"]),
  boardThread(4, "done"),
  boardThread(5, "planned", 10),
];

const agent = threads[2];

if (agent?.work?.owner) agent.work.owner.role = "bot";

const unavailable = threads[4];

if (unavailable?.work) unavailable.work.ownerActive = false;

function loaded(query = { ...defaultBoardQuery, status: "all" as const }): State {
  const state = setBoardLoading({ ...initialState, me: meFixture }, BOARD, query);

  return loadBoardListing(state, { ...boardListing(threads), ...query }, 1);
}

function filter(
  state: State,
  owner: string,
  status: "open" | "done" | "all" = "all",
  tag = "",
): State {
  const board = state.boards[BOARD];

  if (board === undefined) throw new Error("Board missing");

  return {
    ...state,
    boards: { ...state.boards, [BOARD]: { ...board, query: { owner, status, tag } } },
  };
}

function event(state: State, type: "thread.created" | "thread.updated", thread: Thread): State {
  return applyEvents(state, [{ seq: 1, topic: `room:${thread.roomId}`, type, data: thread }], 0);
}

describe("board selectors and reducers", () => {
  it("filters all/open/done work independently of lifecycle status", () => {
    const state = loaded();
    expect(boardPostIds(state, BOARD)).toEqual([5, 4, 3, 2, 1]);
    expect(boardPostIds(filter(state, "anyone", "open"), BOARD)).toEqual([5, 3, 2, 1]);
    expect(boardPostIds(filter(state, "anyone", "done"), BOARD)).toEqual([4]);
  });
  it("filters anyone, me, agent and numeric owners, including unavailable owners", () => {
    const state = loaded();
    expect(boardPostIds(filter(state, "me"), BOARD)).toEqual([1]);
    expect(boardPostIds(filter(state, "agents"), BOARD)).toEqual([3]);
    expect(boardPostIds(filter(state, "10"), BOARD)).toEqual([5]);
    const badgeAgent = boardThread(6, "planned", 11);

    if (badgeAgent.work?.owner)
      badgeAgent.work.owner.agent = {
        agentId: 11,
        kind: "workspace",
        status: "idle",
        suspended: false,
      };
    expect(
      boardPostIds(filter(event(state, "thread.created", badgeAgent), "agents"), BOARD),
    ).toEqual([6, 3]);
  });
  it("uses exact lowercase tags and sorts by activity before descending id", () => {
    const state = loaded();
    expect(boardPostIds(filter(state, "anyone", "open", "api"), BOARD)).toEqual([3, 1]);
    expect(boardPostIds(filter(state, "anyone", "all", "ap"), BOARD)).toEqual([]);
    expect(boardPostIds(filter(state, "anyone", "all", "API"), BOARD)).toEqual([]);

    const changed = {
      ...threads[0],
      ...boardThread(1),
      lastActivityAt: "2026-10-07T11:00:00.000Z",
    };

    expect(boardPostIds(event(state, "thread.updated", changed), BOARD)).toEqual([1, 5, 4, 3, 2]);
  });
  it("returns ordered columns and moves rows on work updates", () => {
    const state = loaded();
    expect(boardColumns(state, BOARD)).toEqual({
      planned: [5, 1],
      in_progress: [2],
      blocked: [3],
      done: [4],
    });
    expect(Object.keys(boardColumns(state, BOARD))).toEqual([
      "planned",
      "in_progress",
      "blocked",
      "done",
    ]);
    expect(
      boardColumns(event(state, "thread.updated", boardThread(1, "done")), BOARD).done,
    ).toEqual([4, 1]);
  });
  it("admits only listed and live-created rows, deduplicates creation, and drops removed posts", () => {
    let state = loaded();
    state = event(state, "thread.updated", boardThread(99));
    expect(boardPostIds(state, BOARD)).not.toContain(99);
    state = event(state, "thread.created", boardThread(99));
    state = event(state, "thread.created", boardThread(99));
    expect(boardPostIds(state, BOARD)).toEqual([99, 5, 4, 3, 2, 1]);
    state = applyEvents(
      state,
      [
        {
          seq: 2,
          topic: "room:900",
          type: "thread.removed",
          data: { threadId: 99, roomId: BOARD },
        },
      ],
      0,
    );
    expect(boardPostIds(state, BOARD)).not.toContain(99);
    expect(state.boards[BOARD]?.tagCounts).toEqual([{ name: "api", count: 2 }]);
    expect(event(initialState, "thread.created", boardThread(99)).boards).toEqual({});
  });
  it("upserts users and memberships and retains detail work in the pane", () => {
    const state = loadThreadDetail(loaded(), boardDetail(), 0);
    expect(state.users[7]?.id).toBe(7);
    expect(state.threadMemberships[1]).toBeNull();
    expect(state.threadPanes[1]?.work).toEqual(boardDetail().work);
    expect(state.threadPanes[1]?.workFacts).toEqual(boardDetail().thread.work);
  });
  it("keeps live-created rows when a cumulative snapshot arrives", () => {
    const state = event(
      setBoardLoading(loaded(), BOARD, { status: "all", owner: "anyone", tag: "" }, true),
      "thread.created",
      boardThread(99),
    );

    const landed = loadBoardListing(state, { ...boardListing(threads), page: 2 }, 2);
    expect(landed.boards[BOARD]?.postIds).toContain(99);
    expect(landed.boards[BOARD]?.page).toBe(2);
  });
});

it("does not revert live updates or resurrect removals when a listing lands", () => {
  const pending = setBoardLoading(
    loaded(),
    BOARD,
    { status: "all", owner: "anyone", tag: "" },
    true,
  );

  const changed = event(pending, "thread.updated", boardThread(1, "done", 7, ["bug"]));

  const removed = applyEvents(
    changed,
    [{ seq: 2, topic: "room:900", type: "thread.removed", data: { threadId: 2, roomId: BOARD } }],
    0,
  );

  const landed = loadBoardListing(removed, boardListing(threads), 2);
  expect(landed.threads[1]?.work?.status).toBe("done");
  expect(landed.threads[1]?.work?.tags).toEqual(["bug"]);
  expect(boardPostIds(landed, BOARD)).not.toContain(2);
  expect(landed.threads[2]).toBeUndefined();
});

describe("removed posts", () => {
  function removed(state: State, threadId: number): State {
    return applyEvents(
      state,
      [
        {
          seq: 2,
          topic: `room:${BOARD}`,
          type: "thread.removed",
          data: { threadId, roomId: BOARD },
        },
      ],
      0,
    );
  }

  it("ignores a create or update published after the removal", () => {
    const post = boardThread(6, "planned");
    let state = removed(event(loaded(), "thread.created", post), 6);

    state = event(state, "thread.created", post);
    state = event(state, "thread.updated", { ...post, name: "Back again" });

    expect(state.threads[6]).toBeUndefined();
    expect(boardPostIds(state, BOARD)).not.toContain(6);
    expect(state.threadPanes[6]?.status).toBe("error");
  });

  it("keeps the removal through a listing requested before it", () => {
    const query = { ...defaultBoardQuery, status: "all" as const };
    const asked = setBoardLoading({ ...initialState, me: meFixture }, BOARD, query);
    const state = loadBoardListing(removed(asked, 3), { ...boardListing(threads), ...query }, 1);

    expect(boardPostIds(state, BOARD)).not.toContain(3);
    expect(state.removedThreads[3]).toBe(1);
  });

  it("lets a reply to a request sent after the removal bring it back", () => {
    const post = boardThread(6, "planned");
    const gone = removed(loaded(), 6);
    const state = loadThreadDetail(gone, boardDetail(post), gone.removalCount);

    expect(state.threads[6]?.name).toBe(post.name);
    expect(state.removedThreads[6]).toBeUndefined();
    expect(event(state, "thread.updated", { ...post, name: "Renamed" }).threads[6]?.name).toBe(
      "Renamed",
    );
  });
});

describe("replies that began before a removal", () => {
  function removed(state: State, threadId: number): State {
    return applyEvents(
      state,
      [
        {
          seq: 2,
          topic: `room:${BOARD}`,
          type: "thread.removed",
          data: { threadId, roomId: BOARD },
        },
      ],
      0,
    );
  }

  it("don't bring the post back, card or pane", () => {
    const post = boardThread(6, "planned");
    const since = loaded().removalCount;
    const gone = removed(event(loaded(), "thread.created", post), 6);
    const state = addBoardPost(loadThreadDetail(gone, boardDetail(post), since), post);

    expect(state).toBe(gone);
    expect(state.threads[6]).toBeUndefined();
    expect(boardPostIds(state, BOARD)).not.toContain(6);
    expect(state.threadPanes[6]?.status).toBe("error");
  });

  it("are judged per removal: a later removal of another post doesn't lift this one", () => {
    const first = removed(loaded(), 6);
    const since = first.removalCount;
    const second = removed(first, 7);

    expect(loadThreadDetail(second, boardDetail(boardThread(7)), since).threads[7]).toBeUndefined();
    expect(loadThreadDetail(second, boardDetail(boardThread(6)), since).threads[6]).toBeDefined();
  });

  it("forget the oldest removals past the cap", () => {
    let state = loaded();

    for (let id = 10_000; id < 10_000 + MAX_REMOVED_THREADS + 5; id += 1) {
      state = removed(state, id);
    }

    expect(Object.keys(state.removedThreads)).toHaveLength(MAX_REMOVED_THREADS);
    expect(state.removedThreads[10_000]).toBeUndefined();
    expect(state.removedThreads[10_000 + MAX_REMOVED_THREADS + 4]).toBe(state.removalCount);
  });
});

describe("agent steps on a post", () => {
  function step(id: number, status: AgentStep["status"], updatedAt: string, position = id) {
    return {
      id,
      messageId: null,
      threadId: 1,
      name: `Step ${id}`,
      status,
      inputSummary: null,
      outputSummary: null,
      durationMs: null,
      position,
      createdAt: "2026-10-07T10:00:00Z",
      updatedAt,
    } satisfies AgentStep;
  }

  function steps(state: State, list: AgentStep[], messageId: number | null = null): State {
    return applyEvents(
      state,
      [
        {
          seq: 3,
          topic: "thread:1",
          type: "agent.steps",
          data: { roomId: BOARD, messageId, threadId: 1, steps: list },
        },
      ],
      0,
    );
  }

  it("merges a post's steps by id, keeping the later copy, in position order", () => {
    let state = loadThreadDetail(loaded(), boardDetail(boardThread(1)), 0);

    state = steps(state, [step(2, "running", "2026-10-07T10:02:00Z", 1)]);
    state = steps(state, [
      step(1, "done", "2026-10-07T10:01:00Z", 0),
      step(2, "pending", "2026-10-07T10:00:30Z", 1),
    ]);

    expect(state.threadPanes[1]?.work?.steps.map((s) => [s.id, s.status])).toEqual([
      [1, "done"],
      [2, "running"],
    ]);
  });

  it("keeps a newer step when an older detail reply lands", () => {
    let state = loadThreadDetail(loaded(), boardDetail(boardThread(1)), 0);

    state = steps(state, [step(1, "done", "2026-10-07T10:05:00Z")]);

    const stale = boardDetail(boardThread(1));

    if (stale.work !== null) {
      stale.work = {
        ...stale.work,
        steps: [
          step(1, "running", "2026-10-07T10:01:00Z"),
          step(2, "pending", "2026-10-07T10:02:00Z"),
        ],
      };
    }

    state = loadThreadDetail(state, stale, 0);
    expect(state.threadPanes[1]?.work?.steps.map((s) => [s.id, s.status])).toEqual([
      [1, "done"],
      [2, "pending"],
    ]);
  });

  it("leaves a post alone for steps on a message in it", () => {
    const state = loadThreadDetail(loaded(), boardDetail(boardThread(1)), 0);

    expect(steps(state, [step(1, "done", "2026-10-07T10:01:00Z")], 44)).toBe(state);
  });
});
