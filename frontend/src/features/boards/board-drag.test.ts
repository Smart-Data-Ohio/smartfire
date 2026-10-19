import { describe, expect, it } from "vitest";
import { meFixture } from "../../api/testing.ts";
import type { Thread } from "../../gen/Thread.ts";
import { defaultBoardQuery, loadBoardListing, setBoardLoading } from "../../store/boards.ts";
import { initialState, type State } from "../../store/state.ts";
import { BOARD, boardListing, boardThread } from "../../test/board-fixtures.ts";
import { canMoveWork, landingFor, statusBeside } from "./board-drag.ts";

function createdBy(thread: Thread, creatorId: number): Thread {
  return { ...thread, creatorId };
}

function board(threads: Thread[], canAdminister = true): State {
  const query = { ...defaultBoardQuery, status: "all" as const };
  const state = setBoardLoading({ ...initialState, me: meFixture }, BOARD, query);

  return loadBoardListing(state, { ...boardListing(threads), ...query, canAdminister }, 1);
}

describe("canMoveWork", () => {
  const threads = [
    createdBy(boardThread(1, "planned", 8), 8),
    createdBy(boardThread(2, "planned", 7), 8),
    boardThread(3, "planned", 8),
    createdBy(boardThread(4, "planned"), 8),
  ];

  it("lets the board's creator or an administrator move every post", () => {
    const state = board(threads);

    expect(threads.map(({ id }) => canMoveWork(state, BOARD, id))).toEqual([
      true,
      true,
      true,
      true,
    ]);
  });

  it("otherwise lets the viewer move only posts they created or own", () => {
    const state = board(threads, false);

    expect(threads.map(({ id }) => canMoveWork(state, BOARD, id))).toEqual([
      false,
      true,
      true,
      false,
    ]);
  });

  it("follows the owner as it changes", () => {
    const state = board(threads, false);
    const handedOver = boardThread(1, "planned", 7);

    expect(
      canMoveWork(
        { ...state, threads: { ...state.threads, 1: createdBy(handedOver, 8) } },
        BOARD,
        1,
      ),
    ).toBe(true);
  });

  it("refuses a post that isn't work or a board that isn't loaded", () => {
    const state = board(threads);
    const plain = { ...boardThread(5), work: null };

    expect(canMoveWork({ ...state, threads: { ...state.threads, 5: plain } }, BOARD, 5)).toBe(
      false,
    );
    expect(canMoveWork(state, BOARD + 1, 1)).toBe(false);
  });
});

describe("landingFor", () => {
  const state = board([
    boardThread(1, "in_progress"),
    boardThread(2, "planned"),
    boardThread(3, "in_progress"),
    boardThread(4, "planned"),
    boardThread(5, "in_progress"),
  ]);

  it("places the post among the column's cards by the board's order", () => {
    expect(landingFor(state, BOARD, 4, "in_progress")).toEqual({ status: "in_progress", index: 1 });
    expect(landingFor(state, BOARD, 2, "in_progress")).toEqual({ status: "in_progress", index: 2 });
    expect(landingFor(state, BOARD, 4, "done")).toEqual({ status: "done", index: 0 });
  });

  it("has no landing in the post's own column, nowhere, or for an unknown post", () => {
    expect(landingFor(state, BOARD, 4, "planned")).toBeNull();
    expect(landingFor(state, BOARD, 4, null)).toBeNull();
    expect(landingFor(state, BOARD, 99, "done")).toBeNull();
  });
});

describe("statusBeside", () => {
  it("steps across the columns and stops at either end", () => {
    expect(statusBeside("planned", 1)).toBe("in_progress");
    expect(statusBeside("blocked", -2)).toBe("planned");
    expect(statusBeside("planned", -1)).toBe("planned");
    expect(statusBeside("blocked", 3)).toBe("done");
  });
});
