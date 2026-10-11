import { afterEach, describe, expect, it, vi } from "vitest";
import { sidebarRowFixture } from "../../api/testing.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { moveSharedRoom } from "./organize-commands.ts";

afterEach(() => {
  vi.restoreAllMocks();

  for (const shown of toastSnapshot()) {
    removeToast(shown.id);
  }
});

/** #general, starred: a drop on the shared layout takes it out of the favourites. */
const starred = (() => {
  const row = sidebarRowFixture(1, "general");

  return { ...row, membership: { ...row.membership, favoritePosition: 0 } };
})();

describe("moving a room for everyone", () => {
  it("takes the room out of its personal place once the shared move has landed", async () => {
    const order: string[] = [];

    vi.spyOn(actions.workspaceLayout, "moveRoom").mockImplementation(async () => {
      order.push("shared");
    });

    const personal = vi.spyOn(actions.organize, "moveRoom").mockImplementation(async () => {
      order.push("personal");
    });

    await moveSharedRoom(starred, 30, null);

    expect(order).toEqual(["shared", "personal"]);
    expect(personal).toHaveBeenCalledWith(1, { kind: "channels" });
  });

  it("keeps the room's personal place when the shared move is refused", async () => {
    vi.spyOn(actions.workspaceLayout, "moveRoom").mockRejectedValue(
      new ActionError("NotFound", "Workspace category not found"),
    );

    const personal = vi.spyOn(actions.organize, "moveRoom");

    await moveSharedRoom(starred, 30, null);

    expect(personal).not.toHaveBeenCalled();
    expect(toastSnapshot().map((shown) => shown.title)).toEqual([
      "Couldn't move general for everyone",
    ]);
  });

  it("leaves the personal place alone for a move from the menu", async () => {
    vi.spyOn(actions.workspaceLayout, "moveRoom").mockResolvedValue(undefined);

    const personal = vi.spyOn(actions.organize, "moveRoom");

    await moveSharedRoom(starred, 30, null, false);

    expect(personal).not.toHaveBeenCalled();
  });
});
