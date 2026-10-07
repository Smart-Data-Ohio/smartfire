import { describe, expect, it } from "vitest";
import { meFixture } from "../api/testing.ts";
import type { Thread } from "../gen/Thread.ts";
import { BOARD, boardDetail, boardListing, boardThread } from "../test/board-fixtures.ts";
import {
  boardColumns,
  boardPostIds,
  defaultBoardQuery,
  loadBoardListing,
  setBoardLoading,
} from "./boards.ts";
import { applyEvents } from "./reducers.ts";
import { initialState, type State } from "./state.ts";
import { loadThreadDetail } from "./threads.ts";

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
    const state = loadThreadDetail(loaded(), boardDetail());
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
