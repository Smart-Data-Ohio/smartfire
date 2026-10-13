import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { meFixture, sidebarFixture, sidebarRowFixture, userFixture } from "../../api/testing.ts";
import type { Sidebar } from "../../gen/Sidebar.ts";
import { applyEvents, loadSidebar, setMe } from "../../store/reducers.ts";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { directs } from "../../sync/directs.ts";
import { FilterBar } from "./search-page.tsx";

const VIEWER = meFixture.user.id;

const SELF_ROOM = 40;

afterEach(() => {
  vi.restoreAllMocks();
  store.setState(initialState, true);
});

describe("the search filter bar", () => {
  it("names your hidden notes as you in an in: chip", async () => {
    vi.spyOn(directs, "switcher").mockResolvedValue({
      rooms: [],
      people: [],
      threads: [],
      users: [],
    });

    const sidebar: Sidebar = sidebarFixture(
      [{ ...sidebarRowFixture(SELF_ROOM, "Ada", "direct", [VIEWER]), noteToSelf: true }],
      [],
    );

    sidebar.noteToSelfRoomId = SELF_ROOM;
    sidebar.users = [userFixture(VIEWER, "Ada Lovelace")];

    const hidden = applyEvents(
      loadSidebar(setMe(initialState, meFixture), sidebar, 0),
      [{ seq: 1, topic: "user", type: "sidebar.row.removed", data: { roomId: SELF_ROOM } }],
      0,
    );

    store.setState(hidden, true);

    const chip = {
      operator: "in_id",
      value: String(SELF_ROOM),
      token: `in_id:${SELF_ROOM}`,
      label: `in_id: ${SELF_ROOM}`,
      removeQuery: "",
    } as const;

    render(
      <FilterBar
        query={chip.token}
        chips={[chip]}
        onQuery={vi.fn()}
        onCompose={vi.fn()}
        announce={vi.fn()}
      />,
    );

    expect(await screen.findByText("In: Ada Lovelace (you)")).toBeTruthy();
    expect(screen.queryByText(`in_id: ${SELF_ROOM}`)).toBeNull();
  });
});
