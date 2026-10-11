import { describe, expect, it } from "vitest";
import type { AnnouncementFollows } from "../../src/gen/AnnouncementFollows.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { Room } from "../../src/gen/Room.ts";
import { collect, errorOf, expectStatus, get, harness, messageBody, send } from "../s2/testing.ts";
import { SEED_IDS } from "../server.ts";

const { rooms, boards } = SEED_IDS;

const follows = (roomId: number) => `/api/v1/rooms/${roomId}/announcement_follows`;

describe("announcement mock", () => {
  it("flags a room, follows it, publishes and delivers an attributed copy", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${rooms.engineering}`]);

    const flagged = await expectStatus<Room>(
      server,
      "PUT",
      `/api/v1/rooms/${rooms.announcements}/announcement`,
      { announcement: true },
      200,
    );

    expect(flagged.announcement).toBe(true);
    expect(
      events.some(
        (event) =>
          event.type === "sidebar.row.upserted" &&
          event.data.room.id === rooms.announcements &&
          event.data.refreshRoom === true,
      ),
    ).toBe(true);

    // The reply is the source room's list, as the server answers.
    const created = await expectStatus<AnnouncementFollows>(
      server,
      "POST",
      follows(rooms.announcements),
      { targetRoomId: rooms.engineering },
      200,
    );

    expect(created.followers.map((each) => each.targetRoomId)).toEqual([rooms.engineering]);

    const read = await get<AnnouncementFollows>(server, follows(rooms.engineering));

    expect(read.following.map((each) => each.sourceRoomId)).toEqual([rooms.announcements]);

    const source = await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/rooms/${rooms.announcements}/messages`,
      messageBody("news-1", "Launch is Thursday"),
      201,
    );

    const published = await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/messages/${source.id}/publish`,
      null,
      200,
    );

    expect(published.published).toBe(true);
    expect(Date.parse(published.updatedAt)).toBeGreaterThan(Date.parse(source.updatedAt));

    const copy = events
      .flatMap((event) => (event.type === "message.updated" ? [event.data] : []))
      .find((message) => message.roomId === rooms.engineering);

    expect(copy?.announcementSource).toEqual({
      roomId: rooms.announcements,
      messageId: source.id,
      roomName: "announcements",
    });
    expect(copy?.creatorId).toBe(source.creatorId);

    // Publishing again changes nothing; a copy can't be published.
    await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/messages/${source.id}/publish`,
      null,
      200,
    );

    const again = await send(server, "POST", `/api/v1/messages/${copy?.id ?? 0}/publish`);

    expect(again.status).toBe(422);

    await expectStatus(
      server,
      "DELETE",
      `${follows(rooms.announcements)}/${rooms.engineering}`,
      null,
      204,
    );
    expect((await get<AnnouncementFollows>(server, follows(rooms.engineering))).following).toEqual(
      [],
    );
  });

  it("refuses what the server refuses", async () => {
    const { server } = harness();

    const plain = await send(server, "POST", follows(rooms.general), {
      targetRoomId: rooms.engineering,
    });

    expect(plain.status).toBe(422);
    expect(errorOf(plain.json).message).toBe("Source room must be an announcement room");

    const message = await expectStatus<MessageDTO>(
      server,
      "POST",
      `/api/v1/rooms/${rooms.general}/messages`,
      messageBody("plain-1", "Not news"),
      201,
    );

    const unflagged = await send(server, "POST", `/api/v1/messages/${message.id}/publish`);

    expect(errorOf(unflagged.json).message).toBe("Message must be in an announcement room");

    for (const roomId of [rooms.announcements, rooms.engineering]) {
      await send(server, "PUT", `/api/v1/rooms/${roomId}/announcement`, { announcement: true });
    }

    await send(server, "POST", follows(rooms.announcements), { targetRoomId: rooms.engineering });

    const cycle = await send(server, "POST", follows(rooms.engineering), {
      targetRoomId: rooms.announcements,
    });

    expect(errorOf(cycle.json).message).toBe("Target room would create an announcement cycle");

    const board = await send(server, "POST", follows(rooms.announcements), {
      targetRoomId: boards.roomId,
    });

    expect(board.status).toBe(422);

    const direct = await send(server, "PUT", `/api/v1/rooms/${rooms.dmMaya}/announcement`, {
      announcement: true,
    });

    expect(direct.status).toBe(422);
  });
});
