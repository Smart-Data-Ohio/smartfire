import { describe, expect, it } from "vitest";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { PinList } from "../../src/gen/PinList.ts";
import type { RoomDetail } from "../../src/gen/RoomDetail.ts";
import type { ScheduledMessageList } from "../../src/gen/ScheduledMessageList.ts";
import type { Sidebar } from "../../src/gen/Sidebar.ts";
import type { ThreadDetail } from "../../src/gen/ThreadDetail.ts";
import type { ThreadList } from "../../src/gen/ThreadList.ts";
import { SEED_IDS } from "../server.ts";
import { FORWARD_NOTE, STARRED_USER_IDS } from "./seed.ts";
import { get, harness, NOW } from "./testing.ts";

const { rooms, users, threads, messages, viewer } = SEED_IDS;

const around = (id: number) => `?around=${id}`;

async function message(roomId: number, id: number): Promise<MessageDTO | undefined> {
  const { server } = harness();
  const page = await get<MessagePage>(server, `/api/v1/rooms/${roomId}/messages${around(id)}`);

  return page.messages.find((candidate) => candidate.id === id);
}

describe("the S2 seed", () => {
  it("leaves the S1 contract alone", async () => {
    const { server } = harness();
    const sidebar = await get<Sidebar>(server, "/api/v1/sidebar");
    const general = sidebar.rows.find((row) => row.room.id === rooms.general);

    expect(general?.unreadCount).toBe(52);
    expect(general?.mentionCount).toBe(1);

    const newest = await get<MessagePage>(server, `/api/v1/rooms/${rooms.general}/messages`);

    expect(newest.messages.at(-1)?.id).toBe(10_399);
  });

  it("is deterministic for a seed and clock", async () => {
    const a = harness().server;
    const b = harness().server;
    const path = `/api/v1/rooms/${rooms.general}/messages`;

    expect(await get(a, path)).toEqual(await get(b, path));
    expect(await get(a, `/api/v1/threads/${threads.generalActive}/messages`)).toEqual(
      await get(b, `/api/v1/threads/${threads.generalActive}/messages`),
    );
  });

  it("gives #general reactions, boosts, a chart and a PDF", async () => {
    const reacted = await message(rooms.general, messages.generalReactions);

    expect(reacted?.reactions.map((pill) => pill.content)).toEqual(["🎉", "🔥", "👏"]);
    expect(reacted?.reactions[0]?.reactorIds).toContain(viewer);
    expect(reacted?.reactions[0]?.title).toBe("Party popper");

    const boosted = await message(rooms.general, messages.generalBoosts);

    expect(boosted?.reactions[0]).toMatchObject({
      content: ":shipit:",
      title: "Ship it",
      imageUrl: "/icons/shipit",
    });
    expect(boosted?.boosts.map((boost) => boost.content)).toEqual(["nice work", "finally!"]);

    const chart = await message(rooms.general, messages.generalChart);

    expect(chart?.creatorId).toBe(users.maya);
    expect(chart?.attachment).toMatchObject({
      filename: "signups-by-week.svg",
      contentType: "image/svg+xml",
      width: 1200,
      height: 675,
      preview: "image",
    });
    expect(chart?.attachment?.url).toMatch(
      /^\/rails\/active_storage\/blobs\/redirect\/[^/]+\/signups-by-week\.svg$/,
    );
    expect(chart?.attachment?.downloadUrl).toBe(`${chart?.attachment?.url}?disposition=attachment`);
    expect(chart?.attachment?.thumbnailUrl).toMatch(/^\/rails\/active_storage\/representations\//);

    const pdf = await message(rooms.general, messages.generalPdf);

    expect(pdf?.attachment).toMatchObject({
      filename: "Q3-board-update.pdf",
      contentType: "application/pdf",
      preview: "file",
      thumbnailUrl: null,
      width: null,
    });
    expect(pdf?.attachment?.byteSize).toBeGreaterThan(100_000);
    expect(pdf?.pinned).toBe(true);
  });

  it("pins three #general messages", async () => {
    const { server } = harness();
    const detail = await get<RoomDetail>(server, `/api/v1/rooms/${rooms.general}`);
    const pins = await get<PinList>(server, `/api/v1/rooms/${rooms.general}/pins`);

    expect(detail.pinsCount).toBe(3);
    expect(pins.pins.map((pin) => pin.messageId).sort()).toEqual(
      [messages.generalPdf, messages.generalPinned, messages.generalPinnedOld].sort(),
    );
    expect(pins.messages).toHaveLength(3);
  });

  it("has an active thread the viewer follows, with unread replies", async () => {
    const { server } = harness();
    const detail = await get<ThreadDetail>(server, `/api/v1/threads/${threads.generalActive}`);

    const replies = await get<MessagePage>(
      server,
      `/api/v1/threads/${threads.generalActive}/messages`,
    );

    const root = await message(rooms.general, messages.generalThreadRoot);

    expect(detail.thread.status).toBe("active");
    expect(detail.thread.replyCount).toBe(6);
    expect(detail.membership?.involvement).toBe("everything");
    expect(detail.membership?.unreadAt).toBe(replies.messages[3]?.createdAt);
    expect(new Set(replies.messages.map((reply) => reply.creatorId))).toEqual(
      new Set([users.jonah, users.priya, viewer]),
    );
    expect(replies.messages[0]?.attachment?.filename).toBe("signup-funnel.svg");
    expect(Date.parse(replies.messages.at(-1)?.createdAt ?? "")).toBeLessThan(NOW);
    expect(root?.thread).toEqual({
      threadId: threads.generalActive,
      replyCount: 6,
      lastReplyAt: detail.thread.lastActivityAt,
      replierIds: [users.jonah, users.priya, viewer],
    });
  });

  it("has a closed and a locked thread in #general and one in #design", async () => {
    const { server } = harness();
    const all = await get<ThreadList>(server, `/api/v1/rooms/${rooms.general}/threads?state=all`);
    const status = new Map(all.threads.map((row) => [row.thread.id, row.thread.status]));

    expect(status.get(threads.generalClosed)).toBe("closed");
    expect(status.get(threads.generalLocked)).toBe("locked");

    const design = await get<ThreadList>(server, `/api/v1/rooms/${rooms.design}/threads?state=all`);

    expect(design.threads.map((row) => row.thread.id)).toEqual([threads.design]);
  });

  it("marks the saved message on its page and has a pending scheduled message", async () => {
    const { server } = harness();

    const page = await get<MessagePage>(
      server,
      `/api/v1/rooms/${rooms.general}/messages${around(messages.generalSaved)}`,
    );

    expect(page.saved).toEqual([{ messageId: messages.generalSaved, savedItemId: 1 }]);

    const scheduled = await get<ScheduledMessageList>(
      server,
      `/api/v1/scheduled_messages?roomId=${rooms.general}`,
    );

    expect(scheduled.scheduledMessages).toHaveLength(1);
    expect(Date.parse(scheduled.scheduledMessages[0]?.sendAt ?? "")).toBeGreaterThan(
      NOW + 2 * 3_600_000,
    );
  });

  it("forwards a #design message into #general with a note", async () => {
    const copy = await message(rooms.general, messages.generalForward);

    expect(copy?.creatorId).toBe(users.theo);
    expect(copy?.forwardNote).toBe(FORWARD_NOTE);
    expect(Math.floor((copy?.forwardedFromMessageId ?? 0) / 10_000)).toBe(rooms.design);
  });

  it("attaches an image in #design and a code file in #engineering", async () => {
    const image = await message(rooms.design, messages.designImage);
    const code = await message(rooms.engineering, messages.engineeringCode);

    expect(image?.attachment).toMatchObject({
      filename: "onboarding-v3.png",
      width: 1200,
      height: 750,
    });
    expect(code?.attachment).toMatchObject({
      filename: "rate_limiter.rs",
      contentType: "text/x-rust",
    });
    expect(STARRED_USER_IDS).toEqual([users.maya, users.priya, users.theo]);
  });
});
