import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { roomDetailFixture, sidebarRowFixture, userFixture } from "../../api/testing.ts";
import { beginRoomRequest } from "../../store/join-state.ts";
import type { RoomDetail } from "../../store/model.ts";
import { mutations } from "../../store/store.ts";
import { RoomIntro } from "./dividers.tsx";

/** A DM's detail as the server sends it: `displayName` the header's, `sidebarDisplayName` in full. */
function direct(
  memberIds: readonly number[],
  displayName: string,
  sidebarDisplayName: string,
  noteToSelf = false,
): RoomDetail {
  const row = sidebarRowFixture(9, displayName, "direct", memberIds);

  const detail: RoomDetail = {
    ...roomDetailFixture(9),
    room: row.room,
    membership: row.membership,
    displayName,
    sidebarDisplayName,
    directMemberIds: [...memberIds],
    users: memberIds.map((id) => userFixture(id, `User ${id}`)),
  };

  return noteToSelf ? { ...detail, noteToSelf } : detail;
}

function show(detail: RoomDetail) {
  mutations.setRoomDetail(detail, beginRoomRequest());
  render(<RoomIntro roomId={detail.room.id} />);
}

afterEach(() => mutations.reset());

describe("RoomIntro", () => {
  it("names a 1:1 DM's other person in full, even when the header name is only their first", () => {
    show(direct([8], "Ashley", "Ashley Brown"));

    expect(screen.getByRole("heading", { level: 2 }).textContent).toBe("Ashley Brown");
    expect(
      screen.getByText("This is the very beginning of your conversation with Ashley Brown.", {
        exact: false,
      }),
    ).toBeTruthy();
  });

  it("lists an unnamed group DM's people with their last names, over the pair of avatars", () => {
    show(direct([8, 9], "Ada, Grace", "Ada Lovelace, Grace Hopper"));

    expect(screen.getByRole("heading", { level: 2 }).textContent).toBe(
      "Ada Lovelace, Grace Hopper",
    );
    expect(
      screen.getByText(
        "This is the very beginning of your conversation with Ada Lovelace, Grace Hopper.",
        { exact: false },
      ),
    ).toBeTruthy();
    expect(document.querySelector(".room-intro .group-avatars")).not.toBeNull();
  });

  it("keeps a note-to-self about you", () => {
    mutations.mergeUsers([userFixture(7, "Ada Lovelace")]);
    show(direct([7], "Ada", "Ada Lovelace", true));

    const intro = document.querySelector(".room-intro");

    expect(intro?.hasAttribute("data-note-to-self")).toBe(true);
    expect(intro?.textContent).not.toContain("beginning of your conversation");
  });

  it("welcomes you to a channel by its name", () => {
    show(roomDetailFixture(4));

    expect(screen.getByRole("heading", { level: 2 }).textContent).toBe("Welcome to #room-4");
    expect(
      screen.getByText("This is the very beginning of #room-4.", { exact: false }),
    ).toBeTruthy();
  });
});
