import { Effect, Option } from "effect";
import { describe, expect, it } from "vitest";
import {
  roomDetailFixture,
  sidebarFixture,
  sidebarRowFixture,
  userFixture,
} from "../api/testing.ts";
import { conversationNameOf, mergeConversationNames } from "../store/conversations.ts";
import { applyEvents, loadSidebar, mergeUsers, setRoomDetail } from "../store/reducers.ts";
import { rowClock } from "../store/row-touches.ts";
import { initialState } from "../store/state.ts";
import { decodeServerFrame } from "./protocol.ts";

describe("public identity updates", () => {
  it("refreshes cached authors, open headers and saved conversation labels without a new message", async () => {
    const original = userFixture(8, "Account Example");
    const row = sidebarRowFixture(20, "Account", "direct", [8]);
    const sidebar = { ...sidebarFixture([row]), users: [original] };

    const detail = {
      ...roomDetailFixture(20),
      room: row.room,
      displayName: "Account Example",
      directMemberIds: [8],
      users: [original],
    };

    const loaded = mergeConversationNames(
      setRoomDetail(loadSidebar(initialState, sidebar, 0), detail),
      [
        {
          roomId: 20,
          threadId: null,
          roomKind: "direct",
          roomName: "Account",
          roomIconName: null,
          threadName: null,
        },
      ],
    );

    const changed = {
      ...original,
      name: "Nick Example",
      pronouns: "they/them",
      updatedAt: "2026-10-10T12:00:00.000000Z",
    };

    const frame = Option.getOrThrow(
      await Effect.runPromise(
        decodeServerFrame(
          JSON.stringify({
            t: "batch",
            events: [{ seq: 1, topic: "user", type: "user.updated", data: changed }],
          }),
        ),
      ),
    );

    if (frame.t !== "batch") throw new Error("Expected an identity update batch");

    const next = applyEvents(
      loaded,
      [
        ...frame.events,
        {
          seq: 2,
          topic: "user",
          type: "sidebar.row.upserted",
          data: { ...row, displayName: "Nick", revision: 1 },
        },
      ],
      0,
    );

    expect(next.users[8]).toEqual(changed);
    expect(next.sidebar.rows[20]?.displayName).toBe("Nick");
    expect(next.rooms[20]?.detail?.displayName).toBe("Nick");
    expect(conversationNameOf(next, 20, null)?.roomName).toBe("Nick");
    expect(mergeUsers(next, [original]).users[8]?.name).toBe("Nick Example");
    expect(loadSidebar(next, sidebar, rowClock(loaded)).sidebar.rows[20]?.displayName).toBe("Nick");

    const refreshed = setRoomDetail(next, {
      ...detail,
      displayName: "Nick Example",
      users: [changed],
    });

    expect(conversationNameOf(refreshed, 20, null)?.roomName).toBe("Nick Example");
  });
});
