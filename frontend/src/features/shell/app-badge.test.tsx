import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { sidebarFixture, sidebarRowFixture } from "../../api/testing.ts";
import type { SidebarRow } from "../../store/model.ts";
import { mutations } from "../../store/store.ts";
import { useAppBadge } from "./app-badge.ts";

function unreadRoom(id: number, involvement: SidebarRow["membership"]["involvement"]): SidebarRow {
  const row = sidebarRowFixture(id, `Room ${id}`);

  return {
    ...row,
    unreadCount: 10,
    membership: { ...row.membership, involvement, unreadAt: "2026-10-07T09:00:00Z" },
  };
}

describe("the installed app's badge", () => {
  afterEach(() => {
    mutations.reset();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("waits for the sidebar, counts unread rooms once, and clears at zero", () => {
    const set = vi.fn().mockResolvedValue(undefined);
    const clear = vi.fn().mockResolvedValue(undefined);

    vi.stubGlobal("navigator", { setAppBadge: set, clearAppBadge: clear });
    renderHook(useAppBadge);

    expect(set).not.toHaveBeenCalled();
    expect(clear).not.toHaveBeenCalled();

    act(() =>
      mutations.loadSidebar(
        sidebarFixture([
          unreadRoom(1, "everything"),
          unreadRoom(2, "muted"),
          unreadRoom(3, "invisible"),
        ]),
      ),
    );
    expect(set).toHaveBeenLastCalledWith(2);

    act(() => mutations.loadSidebar(sidebarFixture([sidebarRowFixture(1, "Room 1")])));
    expect(clear).toHaveBeenCalledOnce();
  });

  it("tolerates unsupported or rejected badge APIs", async () => {
    const set = vi.fn().mockRejectedValue(new Error("Badge unavailable"));

    vi.stubGlobal("navigator", { setAppBadge: set });
    mutations.loadSidebar(sidebarFixture([unreadRoom(1, "everything")]));
    renderHook(useAppBadge);

    await waitFor(() => expect(set).toHaveBeenCalledWith(1));
    vi.stubGlobal("navigator", {});
    expect(() => renderHook(useAppBadge)).not.toThrow();
  });
});
