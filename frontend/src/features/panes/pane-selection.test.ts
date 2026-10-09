import { describe, expect, it } from "vitest";
import {
  closeStep,
  isPaneShowing,
  PANE_TITLES,
  paneRoute,
  selectRightPaneView,
  viewKey,
} from "./pane-selection.ts";
import { clampPaneWidth } from "./right-pane.tsx";

describe("selectRightPaneView", () => {
  it("opens an automations URL on any room, and a remembered one only on a board", () => {
    const input = {
      threadId: null,
      newThreadParent: null,
      routePane: "automations",
      openPane: "files",
    } as const;

    expect(selectRightPaneView({ ...input, roomKind: "board" })).toEqual({
      kind: "pane",
      pane: "automations",
    });

    for (const roomKind of ["open", "closed", "direct", "voice", "stage", undefined] as const) {
      // The URL still opens the pane, so the pane can explain the refusal.
      expect(selectRightPaneView({ ...input, roomKind })).toEqual({
        kind: "pane",
        pane: "automations",
      });
      expect(
        selectRightPaneView({ ...input, roomKind, routePane: null, openPane: "automations" }),
      ).toBeNull();
    }

    expect(selectRightPaneView({ ...input, roomKind: "board", threadId: 7 })).toEqual({
      kind: "thread",
      threadId: 7,
    });
    expect(paneRoute("automations")).toBe("/r/$roomId/automations");
    expect(PANE_TITLES.automations).toBe("Automations");
  });
  it("opens posts/new without a remembered right pane", () => {
    expect(
      selectRightPaneView({
        newBoardPost: true,
        threadId: null,
        newThreadParent: null,
        routePane: "files",
        openPane: "pins",
      }),
    ).toBeNull();
  });
  it("opens a URL pane ahead of a locally remembered pane", () => {
    expect(
      selectRightPaneView({
        threadId: null,
        newThreadParent: null,
        routePane: "files",
        openPane: "pins",
      }),
    ).toEqual({ kind: "pane", pane: "files" });
  });

  it("shows the thread from the URL over everything else", () => {
    expect(selectRightPaneView({ threadId: 7, newThreadParent: 3, openPane: "pins" })).toEqual({
      kind: "thread",
      threadId: 7,
    });
  });

  it("shows the new-thread draft over a side pane", () => {
    expect(selectRightPaneView({ threadId: null, newThreadParent: 3, openPane: "files" })).toEqual({
      kind: "new-thread",
      parentId: 3,
    });
  });

  it("falls back to the side pane, then to nothing", () => {
    expect(
      selectRightPaneView({ threadId: null, newThreadParent: null, openPane: "members" }),
    ).toEqual({ kind: "pane", pane: "members" });

    expect(
      selectRightPaneView({ threadId: null, newThreadParent: null, openPane: null }),
    ).toBeNull();
  });
});

describe("viewKey", () => {
  it("differs per thread, draft and pane", () => {
    const keys = [
      viewKey({ kind: "thread", threadId: 1 }),
      viewKey({ kind: "thread", threadId: 2 }),
      viewKey({ kind: "new-thread", parentId: 1 }),
      viewKey({ kind: "pane", pane: "pins" }),
      viewKey({ kind: "pane", pane: "files" }),
      viewKey({ kind: "pane", pane: "automations" }),
    ];

    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe("isPaneShowing", () => {
  it("is true only for the side pane that's on top", () => {
    expect(isPaneShowing({ kind: "pane", pane: "pins" }, "pins")).toBe(true);
    expect(isPaneShowing({ kind: "pane", pane: "pins" }, "files")).toBe(false);
    expect(isPaneShowing({ kind: "thread", threadId: 4 }, "threads")).toBe(false);
    expect(isPaneShowing(null, "pins")).toBe(false);
  });
});

describe("closeStep", () => {
  it("leaves a thread or draft, closes a side pane, and does nothing when closed", () => {
    expect(closeStep({ kind: "thread", threadId: 4 })).toBe("leave-thread");
    expect(closeStep({ kind: "new-thread", parentId: 4 })).toBe("leave-thread");
    expect(closeStep({ kind: "pane", pane: "members" })).toBe("close-pane");
    expect(closeStep(null)).toBe("none");
  });
});

describe("clampPaneWidth", () => {
  it("keeps the column between 320 and 600 px, whole pixels", () => {
    expect(clampPaneWidth(100)).toBe(320);
    expect(clampPaneWidth(900)).toBe(600);
    expect(clampPaneWidth(455.6)).toBe(456);
  });

  it("falls back to 400 px for a width that isn't a number", () => {
    expect(clampPaneWidth(Number.NaN)).toBe(400);
  });
});
