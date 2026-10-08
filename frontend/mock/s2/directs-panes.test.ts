import { describe, expect, it } from "vitest";
import type { DirectCandidateList } from "../../src/gen/DirectCandidateList.ts";
import type { FileList } from "../../src/gen/FileList.ts";
import type { MemberList } from "../../src/gen/MemberList.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { RoomDetail } from "../../src/gen/RoomDetail.ts";
import type { SidebarRow } from "../../src/gen/SidebarRow.ts";
import type { StarState } from "../../src/gen/StarState.ts";
import type { Switcher } from "../../src/gen/Switcher.ts";
import { SEED_IDS } from "../server.ts";
import { collect, expectStatus, get, harness, messageBody, send } from "./testing.ts";

const { rooms, users, threads, messages, viewer } = SEED_IDS;

describe("directs", () => {
  it("lists candidates, starred first", async () => {
    const { server } = harness();
    const list = await get<DirectCandidateList>(server, "/api/v1/directs/candidates");

    expect(list.candidates.slice(0, 3).map((row) => row.userId)).toEqual([
      users.maya,
      users.priya,
      users.theo,
    ]);
    expect(list.candidates.slice(0, 3).every((row) => row.starred)).toBe(true);
    expect(list.candidates.find((row) => row.userId === users.ember)?.agent).toBe(true);
    expect(list.candidates.map((row) => row.userId)).not.toContain(viewer);
    expect(list.candidates.map((row) => row.userId)).not.toContain(users.dana);
  });

  it("reuses the room with the same people", async () => {
    const { server } = harness();
    const events = collect(server);

    const existing = await expectStatus<SidebarRow>(
      server,
      "POST",
      "/api/v1/directs",
      { userIds: [users.maya] },
      200,
    );

    expect(existing.room.id).toBe(rooms.dmMaya);
    expect(events).toEqual([]);
  });

  it("creates a new room and publishes its sidebar row", async () => {
    const { server } = harness();
    const events = collect(server);

    const created = await expectStatus<SidebarRow>(
      server,
      "POST",
      "/api/v1/directs",
      { userIds: [users.sam, users.dana, viewer] },
      201,
    );

    expect(created.room.kind).toBe("direct");
    expect(created.directMemberIds).toEqual([users.sam]);
    expect(created.displayName).toBe("Sam Whitfield");
    expect(events.map((event) => event.type)).toEqual(["sidebar.row.upserted"]);

    const self = await expectStatus<SidebarRow>(
      server,
      "POST",
      "/api/v1/directs",
      { userIds: [] },
      201,
    );

    expect(self.directMemberIds).toEqual([viewer]);

    const crowd = await send(server, "POST", "/api/v1/directs", {
      userIds: [2, 3, 4, 5, 6, 7, 8, 9, 99],
    });

    expect(crowd.status).toBe(201);

    // Unknown and inactive ids are dropped, so this is the same room again.
    const same = await send(server, "POST", "/api/v1/directs", {
      userIds: [2, 3, 4, 5, 6, 7, 8, 9, 11, users.dana],
    });

    expect(same.status).toBe(200);
  });

  it("grows a group with a system note, and refuses a one-to-one room", async () => {
    const { server } = harness();
    const events = collect(server, [`room:${rooms.groupDm}`]);

    const detail = await expectStatus<RoomDetail>(
      server,
      "POST",
      `/api/v1/directs/${rooms.groupDm}/members`,
      { userIds: [users.sam, users.grace] },
      200,
    );

    expect(detail.memberCount).toBe(5);
    expect(events.map((event) => event.type)).toEqual([
      "message.created",
      "sidebar.row.upserted",
      "sidebar.row.upserted",
    ]);

    const page = await get<MessagePage>(server, `/api/v1/rooms/${rooms.groupDm}/messages`);

    expect(page.messages.at(-1)).toMatchObject({
      systemNote: true,
      markdownSource: null,
      bodyHtml: "added Sam Whitfield and Grace Adeyemi to the group",
    });

    const oneToOne = await send(server, "POST", `/api/v1/directs/${rooms.dmMaya}/members`, {
      userIds: [users.sam],
    });

    const nobody = await send(server, "POST", `/api/v1/directs/${rooms.groupDm}/members`, {
      userIds: [users.sam],
    });

    const full = await send(server, "POST", `/api/v1/directs/${rooms.groupDm}/members`, {
      userIds: [users.maya, users.lucia, users.theo, users.ember, 2],
    });

    expect(oneToOne.status).toBe(422);
    // Nobody new is the model's no-op: the room as it is.
    expect(nobody.status).toBe(200);
    expect(full.status).toBe(200);
  });

  it("treats adding only inactive people as a no-op", async () => {
    // The seed has 9 active people, so a group can't be pushed past 10 here.
    const { server } = harness();

    const group = await expectStatus<SidebarRow>(
      server,
      "POST",
      "/api/v1/directs",
      { userIds: [2, 3, 4, 5, 6, 7, 8, 9] },
      201,
    );

    const events = collect(server);

    const unchanged = await expectStatus<RoomDetail>(
      server,
      "POST",
      `/api/v1/directs/${group.room.id}/members`,
      { userIds: [users.dana] },
      200,
    );

    expect(unchanged.memberCount).toBe(9);
    expect(events).toEqual([]);
  });

  it("renames a group and clears the name", async () => {
    const { server } = harness();
    const path = `/api/v1/directs/${rooms.groupDm}`;

    const renamed = await expectStatus<RoomDetail>(
      server,
      "PATCH",
      path,
      { name: " Launch crew " },
      200,
    );

    expect(renamed.displayName).toBe("Launch crew");

    const cleared = await expectStatus<RoomDetail>(server, "PATCH", path, { name: null }, 200);

    expect(cleared.room.name).toBeNull();
    expect((await send(server, "PATCH", path, { name: "x".repeat(101) })).status).toBe(422);
    expect(
      (await send(server, "PATCH", `/api/v1/directs/${rooms.dmMaya}`, { name: "Us" })).status,
    ).toBe(422);
  });
});

describe("members and stars", () => {
  it("lists a room's active members with presence and stars", async () => {
    const { server } = harness();
    const list = await get<MemberList>(server, `/api/v1/rooms/${rooms.design}/members`);

    expect(list.members.map((member) => member.userId)).toEqual([
      users.grace,
      users.lucia,
      users.maya,
      viewer,
      users.theo,
    ]);
    expect(list.members.find((member) => member.userId === users.maya)?.starred).toBe(true);
    expect(list.members.find((member) => member.userId === viewer)?.starred).toBe(false);
    expect(list.users).toHaveLength(5);
  });

  it("stars and unstars people, but not yourself", async () => {
    const { server } = harness();
    const path = `/api/v1/users/${users.sam}/star`;

    expect(await expectStatus<StarState>(server, "PUT", path, null, 200)).toEqual({
      userId: users.sam,
      starred: true,
    });
    expect(await expectStatus<StarState>(server, "DELETE", path, null, 200)).toEqual({
      userId: users.sam,
      starred: false,
    });
    expect((await send(server, "PUT", `/api/v1/users/${viewer}/star`)).status).toBe(422);
    expect((await send(server, "PUT", `/api/v1/users/${users.ember}/star`)).status).toBe(422);
  });
});

describe("files", () => {
  it("lists room and thread attachments, newest first, by type and name", async () => {
    const { server } = harness();
    const all = await get<FileList>(server, `/api/v1/rooms/${rooms.general}/files`);

    expect(all.files.map((file) => file.attachment.filename)).toEqual([
      "signups-by-week.svg",
      "signup-funnel.svg",
      "Q3-board-update.pdf",
    ]);
    expect(all.files[1]).toMatchObject({
      messageId: messages.generalThreadFunnel,
      threadId: threads.generalActive,
    });
    expect(all.nextPage).toBeNull();

    const documents = await get<FileList>(
      server,
      `/api/v1/rooms/${rooms.general}/files?type=documents`,
    );

    const named = await get<FileList>(
      server,
      `/api/v1/rooms/${rooms.general}/files?filename=FUNNEL`,
    );

    const code = await get<FileList>(
      server,
      `/api/v1/rooms/${rooms.engineering}/files?type=documents`,
    );

    expect(documents.files.map((file) => file.attachment.filename)).toEqual([
      "Q3-board-update.pdf",
    ]);
    expect(named.files.map((file) => file.attachment.filename)).toEqual(["signup-funnel.svg"]);
    expect(code.files.map((file) => file.attachment.filename)).toEqual(["rate_limiter.rs"]);
  });

  it("pages 30 at a time", async () => {
    const { server } = harness();

    for (let index = 0; index < 31; index++) {
      const upload = await expectStatus<{ signedId: string; uploadUrl: string }>(
        server,
        "POST",
        "/api/v1/uploads",
        {
          filename: `note-${index}.txt`,
          byteSize: 1,
          checksum: "DMF1ucDxtqgxw5niaXcmYQ==",
          contentType: "text/plain",
        },
        200,
      );

      await server.handleBinary({
        method: "PUT",
        path: upload.uploadUrl,
        headers: { "Content-Type": "text/plain" },
        bytes: new TextEncoder().encode("a"),
      });
      await expectStatus(
        server,
        "POST",
        `/api/v1/rooms/${rooms.quiet}/messages`,
        messageBody(`file-${index}`, "", { attachmentSignedId: upload.signedId }),
        201,
      );
    }

    const first = await get<FileList>(server, `/api/v1/rooms/${rooms.quiet}/files`);
    const second = await get<FileList>(server, `/api/v1/rooms/${rooms.quiet}/files?page=2`);

    expect(first.files).toHaveLength(30);
    expect(first.nextPage).toBe(2);
    expect(second.files.map((file) => file.attachment.filename)).toEqual(["note-0.txt"]);
    expect(second.nextPage).toBeNull();
  });
});

describe("switcher", () => {
  it("lists rooms, people and recent threads", async () => {
    const { server } = harness();
    const switcher = await get<Switcher>(server, "/api/v1/switcher");
    const group = switcher.rooms.find((room) => room.roomId === rooms.groupDm);
    const maya = switcher.people.find((person) => person.userId === users.maya);

    expect(group).toMatchObject({ kind: "group", name: "Jonah Lindqvist and Priya Raman" });
    expect(switcher.rooms.find((room) => room.roomId === rooms.random)).toMatchObject({
      kind: "channel",
      muted: true,
      unread: true,
    });
    expect(switcher.rooms.find((room) => room.roomId === rooms.lounge)?.kind).toBe("voice");
    expect(maya?.directRoomId).toBe(rooms.dmMaya);
    expect(switcher.people.find((person) => person.userId === users.sam)?.directRoomId).toBeNull();
    expect(switcher.people.map((person) => person.userId)).not.toContain(users.ember);
    expect(switcher.threads[0]).toMatchObject({ roomName: expect.any(String) });
    expect(switcher.rooms.find((room) => room.roomId === 900)).toMatchObject({
      kind: "board",
      name: "Roadmap",
    });
    expect(switcher.threads.map((thread) => thread.threadId).sort((a, b) => a - b)).toEqual([
      1, 4, 5, 6, 901, 9001, 9002, 9004, 9005, 9006, 9008, 9009, 9010, 9011, 9012,
    ]);
  });
});
