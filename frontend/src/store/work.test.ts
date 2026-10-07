import { describe, expect, it } from "vitest";
import { threadFixture } from "../features/threads/test-fixtures.ts";
import {
  factsFixture,
  linkFixture,
  rowFixture,
  threadDetailFixture,
  workDetailFixture,
} from "../features/work/test-fixtures.ts";
import { applyEvents } from "./reducers.ts";
import { initialState, type State } from "./state.ts";
import { loadThreadDetail } from "./threads.ts";
import {
  countWorkWrite,
  landWorkDetail,
  landWorkList,
  optimisticFacts,
  putWorkFacts,
  sameWorkFacts,
  setWorkListFailed,
  setWorkListLoading,
  workDetailStale,
  workListOf,
} from "./work.ts";

const THREAD = 7;

function held(facts = factsFixture()): State {
  const detail = threadDetailFixture(THREAD, facts, workDetailFixture());

  return landWorkDetail(loadThreadDetail(initialState, detail), detail);
}

function updated(state: State, work: ReturnType<typeof factsFixture> | null, seq = 1): State {
  return applyEvents(
    state,
    [
      {
        seq,
        topic: `thread:${THREAD}`,
        type: "thread.updated",
        data: threadFixture(THREAD, { work }),
      },
    ],
    Date.UTC(2026, 9, 6, 11),
  );
}

describe("work in the store", () => {
  it("keeps the work facts thread.updated and thread.created carry", () => {
    const facts = factsFixture({ links: [linkFixture(1)], runUrl: "https://ci.example/1" });
    const state = updated(initialState, facts);

    expect(state.threads[THREAD]?.work).toEqual(facts);

    const created = applyEvents(
      initialState,
      [
        {
          seq: 1,
          topic: "room:4",
          type: "thread.created",
          data: threadFixture(THREAD, { work: facts }),
        },
      ],
      0,
    );

    expect(created.threads[THREAD]?.work).toEqual(facts);
    expect(updated(state, null, 2).threads[THREAD]?.work).toBeNull();
  });

  it("holds a landed detail with its facts, and drops it once untracked", () => {
    const state = held();

    expect(state.work.details[THREAD]).toBeDefined();
    expect(state.work.heldFacts[THREAD]?.status).toBe("in_progress");

    const untracked = threadDetailFixture(THREAD, null, null);
    const next = landWorkDetail(state, untracked);

    expect(next.work.details[THREAD]).toBeUndefined();
    expect(next.work.heldFacts[THREAD]).toBeNull();
  });

  it("calls the detail stale when live facts differ, but not while a write is on its way", () => {
    const state = held();

    expect(workDetailStale(state, THREAD)).toBe(false);

    const moved = updated(state, factsFixture({ status: "blocked" }));

    expect(workDetailStale(moved, THREAD)).toBe(true);
    expect(workDetailStale(countWorkWrite(moved, THREAD, 1), THREAD)).toBe(false);
    expect(countWorkWrite(countWorkWrite(moved, THREAD, 1), THREAD, -1).work.writes).toEqual({});
    // A thread no pane holds is never stale.
    expect(workDetailStale(updated(initialState, factsFixture()), THREAD)).toBe(false);
  });

  it("shows optimistic facts on the thread and the held copy alike", () => {
    const state = putWorkFacts(held(), THREAD, factsFixture({ status: "done" }));

    expect(state.threads[THREAD]?.work?.status).toBe("done");
    expect(workDetailStale(state, THREAD)).toBe(false);
    expect(putWorkFacts(initialState, 99, null)).toBe(initialState);
  });

  it("builds optimistic facts for a start, a move and a stop", () => {
    expect(optimisticFacts(null, "planned")).toEqual({
      status: "planned",
      ownerId: null,
      ownerActive: false,
      runUrl: null,
      resultUpdatedAt: null,
      links: [],
    });
    expect(optimisticFacts(factsFixture({ ownerId: 3 }), "done")).toMatchObject({
      status: "done",
      ownerId: 3,
    });
    expect(optimisticFacts(factsFixture(), null)).toBeNull();
  });

  it("compares facts field by field, links included", () => {
    const facts = factsFixture({ links: [linkFixture(1)] });

    expect(sameWorkFacts(facts, factsFixture({ links: [linkFixture(1)] }))).toBe(true);
    expect(
      sameWorkFacts(
        facts,
        factsFixture({ links: [linkFixture(1, { pullRequestState: "merged" })] }),
      ),
    ).toBe(false);
    expect(sameWorkFacts(facts, factsFixture())).toBe(false);
    expect(sameWorkFacts(null, null)).toBe(true);
    expect(sameWorkFacts(facts, null)).toBe(false);
  });

  it("loads lists per filter, keeping rows through a reload and ignoring older replies", () => {
    const loading = setWorkListLoading(initialState, "open");

    expect(workListOf(loading, "open")).toMatchObject({ status: "loading", generation: 1 });

    const landed = landWorkList(loading, "open", { threads: [rowFixture(1)], users: [] }, 1);

    expect(workListOf(landed, "open").status).toBe("ready");

    const reloading = setWorkListLoading(landed, "open");

    expect(workListOf(reloading, "open")).toMatchObject({ status: "ready", generation: 2 });
    expect(landWorkList(reloading, "open", { threads: [], users: [] }, 1)).toBe(reloading);

    const failed = setWorkListFailed(reloading, "open", "Down", 2);

    expect(workListOf(failed, "open")).toMatchObject({ status: "ready", error: "Down" });
    expect(workListOf(failed, "open").rows).toHaveLength(1);
    expect(
      workListOf(
        setWorkListFailed(setWorkListLoading(initialState, "done"), "done", "x", 1),
        "done",
      ).status,
    ).toBe("error");
  });
});
