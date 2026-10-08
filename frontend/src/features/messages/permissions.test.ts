import { describe, expect, it } from "vitest";
import { messageFixture } from "../../api/testing.ts";
import { menuSections } from "./message-menu.tsx";
import { type MessageContext, messagePermissions } from "./permissions.ts";

const ROOM = 3;

const member: MessageContext = {
  viewerId: 7,
  viewerRole: "member",
  roomKind: "open",
  threadStatus: null,
};

const own = messageFixture(1, ROOM, { creatorId: 7 });

const theirs = messageFixture(2, ROOM, { creatorId: 9 });

describe("message permissions", () => {
  it("lets the creator edit and delete their own message", () => {
    expect(messagePermissions(own, member)).toMatchObject({ edit: true, remove: true });
  });

  it("keeps a member off someone else's message, but not an administrator", () => {
    expect(messagePermissions(theirs, member)).toMatchObject({ edit: false, remove: false });

    expect(messagePermissions(theirs, { ...member, viewerRole: "administrator" })).toMatchObject({
      edit: false,
      remove: true,
    });
  });

  it("refuses edits and deletes on system notes", () => {
    const note = messageFixture(3, ROOM, { creatorId: 7, systemNote: true });

    expect(messagePermissions(note, { ...member, viewerRole: "administrator" })).toMatchObject({
      edit: false,
      remove: false,
      forward: false,
      thread: false,
    });
  });

  it("refuses edits to a reply in a locked thread", () => {
    const reply = messageFixture(4, ROOM, { creatorId: 7, threadId: 5 });

    expect(messagePermissions(reply, { ...member, threadStatus: "locked" }).edit).toBe(false);
    expect(messagePermissions(reply, { ...member, threadStatus: "active" }).edit).toBe(true);
  });

  it("lets any human pin, save, react and forward", () => {
    expect(messagePermissions(theirs, member)).toMatchObject({
      pin: true,
      save: true,
      react: true,
      forward: true,
    });
  });

  it("gives a bot nothing, and nobody anything before boot", () => {
    const bot = messagePermissions(own, { ...member, viewerRole: "bot" });
    const nobody = messagePermissions(own, { ...member, viewerId: null });

    expect(Object.values(bot).some(Boolean)).toBe(false);
    expect(Object.values(nobody).some(Boolean)).toBe(false);
  });

  it("offers a Fizzy card on every message but a system note, replies and locked threads too", () => {
    const reply = messageFixture(7, ROOM, { threadId: 5 });
    const note = messageFixture(8, ROOM, { systemNote: true });

    expect(messagePermissions(theirs, member).fizzy).toBe(true);
    expect(messagePermissions(reply, { ...member, threadStatus: "locked" }).fizzy).toBe(true);
    expect(messagePermissions(theirs, { ...member, roomKind: "direct" }).fizzy).toBe(true);
    expect(messagePermissions(note, member).fizzy).toBe(false);
  });

  it("offers threads on root messages outside direct rooms, and mark unread on the timeline", () => {
    const reply = messageFixture(6, ROOM, { threadId: 5 });

    expect(messagePermissions(theirs, member).thread).toBe(true);
    expect(messagePermissions(theirs, { ...member, roomKind: "direct" }).thread).toBe(false);
    expect(messagePermissions(reply, member)).toMatchObject({ markUnread: false });
  });
});

describe("message menu", () => {
  const commands = (sections: ReturnType<typeof menuSections>) =>
    sections.map((section) => section.map((entry) => entry.command));

  it("lists everything, in sections, for an administrator on their own message", () => {
    const permissions = messagePermissions(own, { ...member, viewerRole: "administrator" });

    expect(
      commands(menuSections({ permissions, pinned: false, saved: false, inThread: false })),
    ).toEqual([
      ["thread", "react", "boost"],
      ["edit", "copy-text", "copy-link"],
      ["pin", "save", "forward", "unread", "fizzy"],
      ["delete"],
    ]);
  });

  it("drops what the viewer can't do, and a reply-in-thread inside the thread pane", () => {
    const permissions = messagePermissions(theirs, member);

    const sections = commands(
      menuSections({ permissions, pinned: false, saved: false, inThread: true }),
    );

    expect(sections.flat()).not.toContain("edit");
    expect(sections.flat()).not.toContain("delete");
    expect(sections.flat()).not.toContain("thread");
    expect(sections).toHaveLength(3);
  });

  it("flips pin and save to their undo labels", () => {
    const permissions = messagePermissions(theirs, member);

    const labels = menuSections({ permissions, pinned: true, saved: true, inThread: false })
      .flat()
      .map((entry) => entry.label);

    expect(labels).toContain("Unpin from conversation");
    expect(labels).toContain("Remove from Saved");
  });

  it("marks only delete as dangerous", () => {
    const permissions = messagePermissions(own, member);

    const danger = menuSections({ permissions, pinned: false, saved: false, inThread: false })
      .flat()
      .filter((entry) => entry.danger);

    expect(danger.map((entry) => entry.command)).toEqual(["delete"]);
  });
});
