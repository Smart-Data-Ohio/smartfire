import { describe, expect, it } from "vitest";
import type { IconList } from "../../src/gen/IconList.ts";
import type { Me } from "../../src/gen/Me.ts";
import type { MessagePage } from "../../src/gen/MessagePage.ts";
import type { MessagePreview } from "../../src/gen/MessagePreview.ts";
import type { ScheduledMessage } from "../../src/gen/ScheduledMessage.ts";
import type { ScheduledMessageList } from "../../src/gen/ScheduledMessageList.ts";
import type { SlashCommandList } from "../../src/gen/SlashCommandList.ts";
import type { SlashCommandResult } from "../../src/gen/SlashCommandResult.ts";
import type { UserSuggestionList } from "../../src/gen/UserSuggestionList.ts";
import { SEED_IDS } from "../server.ts";
import { SHRUG } from "./composer.ts";
import { expectStatus, get, harness, NOW, send } from "./testing.ts";

const { rooms, users, threads, scheduled, viewer } = SEED_IDS;

const HOUR = 3_600_000;

describe("autocomplete", () => {
  it("suggests a room's members with a mention token", async () => {
    const { server } = harness();

    const list = await get<UserSuggestionList>(
      server,
      `/api/v1/autocomplete/users?roomId=${rooms.design}&query=a`,
    );

    expect(list.suggestions.map((row) => row.user.name)).toEqual([
      "Grace Adeyemi",
      "Lucía Fernández",
      "Maya Okafor",
      "Riel St. Amand",
      "Theo Nakamura",
    ]);
    expect(list.suggestions[2]?.mentionToken).toBe("@[Maya Okafor]");

    const everyone = await get<UserSuggestionList>(server, "/api/v1/autocomplete/users?query=em");

    expect(everyone.suggestions.map((row) => row.user.id)).toEqual([users.ember, users.grace]);
    expect(
      (await server.handle({ method: "GET", path: "/api/v1/autocomplete/users?roomId=99" })).status,
    ).toBe(404);
  });

  it("ranks icons exact, prefix, substring, at most 8", async () => {
    const { server } = harness();
    const fire = await get<IconList>(server, "/api/v1/autocomplete/icons?query=fire");

    expect(fire.icons).toHaveLength(8);
    expect(fire.icons[0]).toMatchObject({ name: "fire", kind: "emoji", character: "🔥" });
    expect(fire.icons[1]?.name.startsWith("fire")).toBe(true);
    expect(fire.icons.some((icon) => icon.name === "smartfire")).toBe(true);

    const ship = await get<IconList>(server, "/api/v1/autocomplete/icons?query=shipit");

    expect(ship.icons[0]).toEqual({
      name: "shipit",
      title: "Ship it",
      kind: "custom",
      character: null,
      imageUrl: "/icons/shipit",
    });
    expect((await get<IconList>(server, "/api/v1/autocomplete/icons?query=")).icons).toEqual([]);
  });

  it("lists brand and workspace icons, whose images are served", async () => {
    const { server } = harness();
    const all = await get<IconList>(server, "/api/v1/icons");

    expect(all.icons.map((icon) => icon.kind)).toEqual([
      ...Array(8).fill("brand"),
      ...Array(5).fill("custom"),
    ]);

    for (const icon of all.icons) {
      const image = await server.handleBinary({
        method: "GET",
        path: icon.imageUrl ?? "",
        bytes: null,
      });

      expect(image.status).toBe(200);
      expect(image.contentType).toBe("image/svg+xml");
      expect(new TextDecoder().decode(image.bytes)).toMatch(/^<svg /);
    }
  });
});

describe("slash commands", () => {
  it("lists the built-ins in registry order, then the room's agent commands", async () => {
    const { server } = harness();

    const engineering = await get<SlashCommandList>(
      server,
      `/api/v1/rooms/${rooms.engineering}/slash_commands`,
    );

    expect(engineering.commands.map((command) => command.name)).toEqual([
      "huddle",
      "event",
      "poll",
      "remind",
      "status",
      "dnd",
      "ooo",
      "shrug",
      "me",
      "play",
      "deploy-status",
      "triage",
    ]);
    expect(engineering.commands.at(-1)).toMatchObject({
      description: "Custom command",
      agentName: "Ember",
      argHint: "",
    });

    const inThread = await get<SlashCommandList>(
      server,
      `/api/v1/rooms/${rooms.general}/slash_commands?threadId=${threads.generalActive}`,
    );

    expect(inThread.commands.map((command) => command.name)).not.toContain("poll");
  });

  const run = (
    server: Parameters<typeof send>[0],
    roomId: number,
    text: string,
    threadId: number | null = null,
  ) =>
    expectStatus<SlashCommandResult>(
      server,
      "POST",
      `/api/v1/rooms/${roomId}/slash_commands`,
      { text, threadId },
      200,
    );

  it("posts for /shrug, /me and /play", async () => {
    const { server } = harness();
    const shrug = await run(server, rooms.quiet, "/shrug fine");
    const me = await run(server, rooms.quiet, "/me is reviewing the deploy");
    const play = await run(server, rooms.quiet, "/play trombone");
    const page = await get<MessagePage>(server, `/api/v1/rooms/${rooms.quiet}/messages`);

    expect(shrug).toMatchObject({ status: "posted", notice: null });
    expect(page.messages.map((message) => message.markdownSource)).toEqual([
      `fine ${SHRUG}`,
      "is reviewing the deploy",
      "/play trombone",
    ]);
    expect(page.messages[0]?.bodyHtml).toBe("<p>fine ¯_(ツ)_/¯</p>");
    expect(page.messages[1]?.action).toBe(true);
    expect(me.status).toBe("posted");
    expect(play.status).toBe("posted");
  });

  it("sets a reminder that saves the posted message", async () => {
    const { server } = harness();
    const result = await run(server, rooms.quiet, "/remind in 20 minutes check the deploy");

    expect(result).toMatchObject({
      status: "posted",
      notice: "Reminder set for October 06, 2026 12:50.",
    });

    const page = await get<MessagePage>(server, `/api/v1/rooms/${rooms.quiet}/messages`);

    expect(page.messages[0]?.markdownSource).toBe("check the deploy");
    expect(page.saved).toHaveLength(1);
    expect((await run(server, rooms.quiet, "/remind")).status).toBe("error");
  });

  it("answers ephemerally for /status, /dnd and /ooo, and the profile follows", async () => {
    const { server } = harness();

    expect(await run(server, rooms.general, "/status 🚂 On a train")).toEqual({
      status: "ephemeral",
      message: "Status set to “🚂 On a train”.",
    });
    expect(await run(server, rooms.general, "/dnd")).toEqual({
      status: "ephemeral",
      message: "Do Not Disturb is on.",
    });
    expect((await get<Me>(server, "/api/v1/me")).doNotDisturb).toEqual({
      enabled: true,
      until: null,
    });
    expect(await run(server, rooms.general, "/dnd")).toEqual({
      status: "ephemeral",
      message: "Do Not Disturb is off.",
    });
    expect(await run(server, rooms.general, "/dnd 2h")).toEqual({
      status: "ephemeral",
      message: "Do Not Disturb is on until October 06, 2026 14:30.",
    });
    expect(await run(server, rooms.general, "/ooo friday Back Monday")).toEqual({
      status: "ephemeral",
      message: "Out of office until October 09, 2026. Note: “Back Monday”.",
    });

    const me = await get<Me>(server, "/api/v1/me");

    expect(me.outOfOffice).toMatchObject({ note: "Back Monday" });
    expect(me.user.customStatus).toMatchObject({ emoji: "🚂", text: "On a train" });
    expect(await run(server, rooms.general, "/ooo off")).toEqual({
      status: "ephemeral",
      message: "Out of office is off.",
    });
  });

  it("opens forms and huddles, and explains mistakes", async () => {
    const { server } = harness();

    expect(await run(server, rooms.general, "/poll")).toEqual({ status: "open_poll" });
    expect(await run(server, rooms.general, "/huddle")).toEqual({
      status: "start_huddle",
      roomId: rooms.general,
      roomName: "general",
    });
    expect(await run(server, rooms.general, "/event")).toEqual({
      status: "open_url",
      url: `/rooms/${rooms.general}/events/new`,
    });

    const event = await run(server, rooms.general, "/event Launch party friday 5pm");

    expect(event).toMatchObject({ status: "open_url" });
    expect(event.status === "open_url" ? decodeURIComponent(event.url) : "").toContain(
      "event[title]=Launch+party",
    );
    expect(await run(server, rooms.general, "/poll", threads.generalActive)).toEqual({
      status: "error",
      message: "“/poll” is only available in the channel, not in threads.",
    });
    expect(await run(server, rooms.general, "/shrug", threads.generalLocked)).toEqual({
      status: "error",
      message: "This thread is locked.",
    });
    expect(await run(server, rooms.general, "/nope")).toEqual({
      status: "error",
      message:
        "Unknown command “/nope”. Available: /huddle, /event, /poll, /remind, /status, /dnd, /ooo, /shrug, /me, /play.",
    });
    expect(await run(server, rooms.engineering, "/triage the flaky test")).toEqual({
      status: "ephemeral",
      message: "Sent to Ember",
    });
    expect(await run(server, rooms.general, "hello")).toEqual({
      status: "error",
      message: "Type / to see available commands.",
    });
  });
});

describe("preview", () => {
  it("renders Markdown with mentions without posting", async () => {
    const { server } = harness();

    const preview = await expectStatus<MessagePreview>(
      server,
      "POST",
      `/api/v1/rooms/${rooms.general}/messages/preview`,
      { markdownSource: "**hi** @[Maya Okafor]" },
      200,
    );

    expect(preview.bodyHtml).toContain("<strong>hi</strong>");
    expect(preview.bodyHtml).toContain(`data-user-id="${users.maya}"`);

    const long = await send(server, "POST", `/api/v1/rooms/${rooms.general}/messages/preview`, {
      markdownSource: "x".repeat(50_001),
    });

    expect(long.status).toBe(422);
  });
});

describe("scheduled messages", () => {
  it("creates, edits, lists and cancels", async () => {
    const { server } = harness();
    const sendAt = new Date(NOW + HOUR).toISOString();

    const created = await expectStatus<ScheduledMessage>(
      server,
      "POST",
      `/api/v1/rooms/${rooms.quiet}/scheduled_messages`,
      { markdownSource: "Later", sendAt, threadId: null, replyToMessageId: null },
      201,
    );

    expect(created).toMatchObject({ roomId: rooms.quiet, sendAt, sentAt: null, droppedAt: null });

    const moved = await expectStatus<ScheduledMessage>(
      server,
      "PATCH",
      `/api/v1/scheduled_messages/${created.id}`,
      { markdownSource: "Later still", sendAt: new Date(NOW + 2 * HOUR).toISOString() },
      200,
    );

    expect(moved.markdownSource).toBe("Later still");

    const quiet = await get<ScheduledMessageList>(
      server,
      `/api/v1/scheduled_messages?roomId=${rooms.quiet}`,
    );

    const all = await get<ScheduledMessageList>(server, "/api/v1/scheduled_messages");

    expect(quiet.scheduledMessages.map((message) => message.id)).toEqual([created.id]);
    expect(all.scheduledMessages[0]?.id).toBe(created.id);
    expect(all.scheduledMessages.map((message) => message.id)).toContain(scheduled.generalPending);
    expect((await send(server, "DELETE", `/api/v1/scheduled_messages/${created.id}`)).status).toBe(
      204,
    );
    expect((await send(server, "DELETE", `/api/v1/scheduled_messages/${created.id}`)).status).toBe(
      404,
    );
  });

  it("refuses a past time, a blank text and a thread from another room", async () => {
    const { server } = harness();
    const path = `/api/v1/rooms/${rooms.quiet}/scheduled_messages`;
    const later = new Date(NOW + HOUR).toISOString();

    const past = await send(server, "POST", path, {
      markdownSource: "x",
      sendAt: new Date(NOW - 1).toISOString(),
      threadId: null,
      replyToMessageId: null,
    });

    const blank = await send(server, "POST", path, {
      markdownSource: " ",
      sendAt: later,
      threadId: null,
      replyToMessageId: null,
    });

    const thread = await send(server, "POST", path, {
      markdownSource: "x",
      sendAt: later,
      threadId: threads.generalActive,
      replyToMessageId: null,
    });

    expect([past.status, blank.status, thread.status]).toEqual([422, 422, 422]);
  });

  it("posts on its timer, at once with send_now, or through the control", async () => {
    const { server, clock } = harness();
    const path = `/api/v1/rooms/${rooms.quiet}/scheduled_messages`;

    await expectStatus(
      server,
      "POST",
      path,
      {
        markdownSource: "On the timer",
        sendAt: new Date(NOW + HOUR).toISOString(),
        threadId: null,
        replyToMessageId: null,
      },
      201,
    );

    const now = await expectStatus<ScheduledMessage>(
      server,
      "POST",
      path,
      {
        markdownSource: "Right now",
        sendAt: new Date(NOW + 2 * HOUR).toISOString(),
        threadId: null,
        replyToMessageId: null,
      },
      201,
    );

    const sent = await expectStatus<ScheduledMessage>(
      server,
      "POST",
      `/api/v1/scheduled_messages/${now.id}/send_now`,
      null,
      200,
    );

    expect(sent.sentAt).toBe(new Date(NOW).toISOString());

    clock.advance(HOUR);

    const page = await get<MessagePage>(server, `/api/v1/rooms/${rooms.quiet}/messages`);

    expect(page.messages.map((message) => [message.markdownSource, message.creatorId])).toEqual([
      ["Right now", viewer],
      ["On the timer", viewer],
    ]);

    const waiting = await get<ScheduledMessageList>(server, "/api/v1/scheduled_messages");
    const due = await send(server, "POST", "/__mock/schedule-due", { all: true });

    expect(due.json).toEqual({ sent: waiting.scheduledMessages.length });
    expect(
      (await get<ScheduledMessageList>(server, "/api/v1/scheduled_messages")).scheduledMessages,
    ).toEqual([]);
  });
});
