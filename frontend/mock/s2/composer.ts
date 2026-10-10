/**
 * What the composer asks the server for: mention and icon autocomplete, the icon picker,
 * slash commands (listed and run, mirroring the Rust registry), Markdown preview and scheduled
 * messages, which a timer on the scheduler posts when they fall due.
 */
import type { IconList } from "../../src/gen/IconList.ts";
import type { MessageDTO } from "../../src/gen/MessageDTO.ts";
import type { MessagePreview } from "../../src/gen/MessagePreview.ts";
import type { ScheduledMessage } from "../../src/gen/ScheduledMessage.ts";
import type { ScheduledMessageList } from "../../src/gen/ScheduledMessageList.ts";
import type { SlashCommand } from "../../src/gen/SlashCommand.ts";
import type { SlashCommandList } from "../../src/gen/SlashCommandList.ts";
import type { SlashCommandResult } from "../../src/gen/SlashCommandResult.ts";
import type { UserSuggestionList } from "../../src/gen/UserSuggestionList.ts";
import {
  conflict,
  forbidden,
  HttpError,
  type MockResponse,
  noContent,
  notFound,
  ok,
  validation,
} from "../http.ts";
import { field, intField, type Json, stringField } from "../json.ts";
import { renderMarkdown } from "../markdown.ts";
import { conversationNames } from "../s3/conversations.ts";
import { beforeOf, keysetPage } from "../s3/model.ts";
import { type RoomRecord, VIEWER_ID } from "../seed.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";
import { imageIcons, searchIcons } from "./emoji.ts";
import { iso, type MessageDraft, plainDraft, SOURCE_LIMIT, type ThreadRecord } from "./model.ts";
import { checkMarkdown } from "./posting.ts";
import type { Threads } from "./threads.ts";
import { endOfLocalDay, leadingTime, longDate, longTime, trailingTime } from "./when.ts";

/** The viewer's time zone (`Me.preferences.timeZone`), which slash command times are read in. */
export const VIEWER_TIME_ZONE = "America/New_York";

/** At most this many people in a mention list. */
export const USER_SUGGESTIONS = 20;

/** What `/shrug` appends: both backslashes escaped, so the arm renders (`¯\_(ツ)_/¯`). */
export const SHRUG = String.raw`¯\\\_(ツ)\_/¯`;

/** One built-in command (`slash_commands::Command`). */
interface BuiltIn {
  readonly name: string;
  readonly description: string;
  readonly argHint: string;
  readonly takesArguments: boolean;
  /** Available in threads. */
  readonly thread: boolean;
}

/** `slash_commands::registry()`, in order. */
export const SLASH_COMMANDS: readonly BuiltIn[] = [
  {
    name: "huddle",
    description: "Start a call in this room",
    argHint: "",
    takesArguments: false,
    thread: true,
  },
  {
    name: "event",
    description: "Open the event form prefilled",
    argHint: "<title> <when>",
    takesArguments: false,
    thread: true,
  },
  {
    name: "poll",
    description: "Open the poll builder",
    argHint: "",
    takesArguments: false,
    thread: false,
  },
  {
    name: "remind",
    description: "Post and remind yourself about it later",
    argHint: "<when> <text>",
    takesArguments: true,
    thread: true,
  },
  {
    name: "status",
    description: "Set your custom status",
    argHint: "<emoji> <text>",
    takesArguments: true,
    thread: true,
  },
  {
    name: "dnd",
    description: "Toggle Do Not Disturb, optionally for a while",
    argHint: "[duration|off]",
    takesArguments: true,
    thread: true,
  },
  {
    name: "ooo",
    description: "Set out of office with an optional note",
    argHint: "<when> [note]|off",
    takesArguments: true,
    thread: true,
  },
  {
    name: "shrug",
    description: "Post with a shrug",
    argHint: "[text]",
    takesArguments: true,
    thread: true,
  },
  {
    name: "me",
    description: "Post an action line",
    argHint: "<action>",
    takesArguments: true,
    thread: true,
  },
  {
    name: "play",
    description: "Play a chat sound",
    argHint: "<sound>",
    takesArguments: true,
    thread: true,
  },
];

const COMMAND = /^\/([a-zA-Z][a-zA-Z0-9_-]*)(?:\s+([\s\S]*))?$/;

/** The composer module. */
export interface Composer {
  readonly routes: readonly Route[];
  /** Posts every pending scheduled message that's due (or every pending one, with `all`). */
  fireScheduled(all: boolean): ScheduledMessage[];
  /** Cancels the timers and arms one per pending scheduled message in the current world. */
  arm(): void;
  /** Cancels every timer. */
  stop(): void;
  /**
   * Marks a pending one as held by another runner (`sending`: edits and cancels get 409,
   * send_now 202), or lets it go again.
   */
  markSending(id: number, on: boolean): ScheduledMessage;
}

/** What the composer tells other modules about scheduled messages. */
export interface ScheduledHooks {
  /** One was dropped (when due, or by send_now): the inbox records it. */
  readonly dropped: (message: ScheduledMessage) => void;
  /** One was cancelled: its inbox items go. */
  readonly cancelled: (id: number) => void;
}

const NO_HOOKS: ScheduledHooks = { dropped: () => undefined, cancelled: () => undefined };

/** Scheduled messages a page (`GET /scheduled_messages`). */
export const SCHEDULED_PAGE_SIZE = 50;

/**
 * Creates the composer module. `save` sets a reminder on a message (`/remind`); `hooks` hear
 * about dropped and cancelled scheduled messages.
 */
export function createComposer(
  ctx: S2Context,
  threads: Threads,
  save: (messageId: number, remindAt: string) => void,
  hooks: ScheduledHooks = NO_HOOKS,
): Composer {
  const timers = new Map<number, number>();

  // --- autocomplete ---

  const userSuggestions = (query: URLSearchParams): UserSuggestionList => {
    const world = ctx.world();
    const rawRoom = query.get("roomId");
    const roomId = rawRoom === null || rawRoom === "" ? null : Number(rawRoom);
    const scope = roomId === null ? [...world.users.keys()] : ctx.roomOr404(roomId).memberIds;
    const people = ctx.usersFor(scope).filter((user) => user.status === "active");
    const needle = (query.get("query") ?? "").trim().toLowerCase();
    const counts = new Map<string, number>();

    for (const person of people) counts.set(person.name, (counts.get(person.name) ?? 0) + 1);

    const suggestions = people
      .filter((user) => user.name.toLowerCase().includes(needle))
      .sort((a, b) => {
        const [left, right] = [a.name.toLowerCase(), b.name.toLowerCase()];

        return left < right ? -1 : left > right ? 1 : a.id - b.id;
      })
      .slice(0, USER_SUGGESTIONS)
      .map((user) => ({
        user,
        mentionToken:
          counts.get(user.name) === 1 && !/[[\]\n\r]/.test(user.name) ? `@[${user.name}]` : null,
      }));

    return { suggestions };
  };

  const iconSuggestions = (query: URLSearchParams): IconList => ({
    icons: searchIcons(query.get("query") ?? ""),
  });

  // --- slash commands ---

  const agentCommands = (roomId: number) =>
    [...(ctx.world().agentCommands.get(roomId) ?? [])].sort((a, b) =>
      a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
    );

  const threadIn = (record: RoomRecord, threadId: number | null): ThreadRecord | null => {
    if (threadId === null) return null;

    const thread = threads.threadOr404(threadId);

    if (thread.roomId !== record.room.id) throw notFound("Thread not found");

    return thread;
  };

  const commands = (roomId: number, query: URLSearchParams): SlashCommandList => {
    const record = ctx.roomOr404(roomId);
    const rawThread = query.get("threadId");

    const inThread =
      threadIn(record, rawThread === null || rawThread === "" ? null : Number(rawThread)) !== null;

    const builtIns: SlashCommand[] = SLASH_COMMANDS.flatMap((command) =>
      inThread && !command.thread
        ? []
        : [
            {
              name: command.name,
              description: command.description,
              argHint: command.argHint,
              takesArguments: command.takesArguments,
              agentName: null,
            },
          ],
    );

    const agents: SlashCommand[] = agentCommands(roomId).map((command) => ({
      name: command.name,
      description: command.description ?? "Custom command",
      argHint: "",
      takesArguments: true,
      agentName: command.agentName,
    }));

    return { commands: [...builtIns, ...agents] };
  };

  const errorResult = (message: string): SlashCommandResult => ({ status: "error", message });

  const ephemeral = (message: string): SlashCommandResult => ({ status: "ephemeral", message });

  const pastResult = (at: number) =>
    errorResult(`“${longTime(at, VIEWER_TIME_ZONE)}” is in the past.`);

  const runCommand = (roomId: number, body: Json | undefined): SlashCommandResult => {
    const world = ctx.world();
    const record = ctx.roomOr404(roomId);
    const viewer = world.users.get(VIEWER_ID);

    if (viewer === undefined || viewer.status !== "active" || viewer.role === "bot") {
      throw forbidden("Only people can run commands");
    }

    const thread = threadIn(record, intField(body, "threadId"));
    const match = COMMAND.exec((stringField(body, "text") ?? "").trim());

    if (match === null) return errorResult("Type / to see available commands.");

    const name = (match[1] ?? "").toLowerCase();
    const args = (match[2] ?? "").trim();
    const command = SLASH_COMMANDS.find((candidate) => candidate.name === name);

    if (command === undefined) {
      const agent = agentCommands(roomId).find((candidate) => candidate.name === name);

      if (agent !== undefined) return ephemeral(`Sent to ${agent.agentName}`);

      const names = [
        ...SLASH_COMMANDS.filter((candidate) => thread === null || candidate.thread),
        ...agentCommands(roomId),
      ].map((candidate) => `/${candidate.name}`);

      return errorResult(`Unknown command “/${name}”. Available: ${names.join(", ")}.`);
    }

    if (thread !== null && !command.thread) {
      return errorResult(`“/${name}” is only available in the channel, not in threads.`);
    }

    const post = (markdown: string, action: boolean): MessageDTO => {
      const draft: MessageDraft = { ...plainDraft(VIEWER_ID, markdown, ctx.uuid()), action };

      if (thread === null) return ctx.postToRoom(record, draft);

      if (thread.locked) throw new LockedThread();

      return threads.postReply(thread, draft);
    };

    const posted = (message: MessageDTO, notice: string | null = null): SlashCommandResult => ({
      status: "posted",
      messageId: message.id,
      notice,
    });

    const now = ctx.now();

    try {
      switch (name) {
        case "poll":
          return { status: "open_poll" };
        case "huddle":
          return { status: "start_huddle", roomId, roomName: ctx.displayName(record) };
        case "event":
          return eventForm(roomId, args, now);
        case "play":
          return posted(post(`/play ${args}`.trim(), false));
        case "shrug":
          return posted(post(args === "" ? SHRUG : `${args} ${SHRUG}`, false));
        case "me":
          if (args === "") {
            return errorResult("Usage: /me <action> — for example “/me is reviewing the deploy”.");
          }

          return posted(post(args, true));
        case "remind":
          return remind(args, now, post, posted);
        case "status":
          return status(args, now);
        case "dnd":
          return dnd(args, now);
        default:
          return outOfOffice(args, now);
      }
    } catch (error) {
      if (error instanceof LockedThread) return errorResult("This thread is locked.");

      if (error instanceof HttpError && error.status === 422)
        return errorResult(error.error.message);

      throw error;
    }
  };

  const eventForm = (roomId: number, args: string, now: number): SlashCommandResult => {
    const path = `/rooms/${roomId}/events/new`;

    if (args === "") return { status: "open_url", url: path };

    const { title, at } = trailingTime(args, now, VIEWER_TIME_ZONE);

    if (title === "") {
      return errorResult(
        "Usage: /event <title> <when> — for example “/event Launch party friday 5pm”.",
      );
    }

    if (at !== null && at <= now) return pastResult(at);

    const query = new URLSearchParams();

    if (at !== null) query.set("event[starts_at]", iso(at));

    query.set("event[time_zone]", VIEWER_TIME_ZONE);
    query.set("event[title]", title);

    return { status: "open_url", url: `${path}?${query.toString()}` };
  };

  const remind = (
    args: string,
    now: number,
    post: (markdown: string, action: boolean) => MessageDTO,
    posted: (message: MessageDTO, notice: string | null) => SlashCommandResult,
  ): SlashCommandResult => {
    if (args === "") {
      return errorResult(
        "Usage: /remind <when> <text> — for example “/remind in 20 minutes review the deploy”.",
      );
    }

    const parsed = leadingTime(args, now, VIEWER_TIME_ZONE, "morning");

    if (parsed === null || parsed.rest === "") {
      return errorResult(
        "Usage: /remind <when> <text> — for example “/remind tomorrow 9am file expenses”.",
      );
    }

    if (parsed.at <= now) return pastResult(parsed.at);

    const message = post(parsed.rest, false);

    save(message.id, iso(parsed.at));

    return posted(message, `Reminder set for ${longTime(parsed.at, VIEWER_TIME_ZONE)}.`);
  };

  const status = (args: string, now: number): SlashCommandResult => {
    const world = ctx.world();
    const [emoji = "", ...words] = args.split(/\s+/);
    const text = words.join(" ").trim();

    if (emoji === "" || text === "") {
      return errorResult("Usage: /status <emoji> <text> — for example “/status 🚂 On a train”.");
    }

    const user = world.users.get(VIEWER_ID);
    const endOfDay = endOfLocalDay(now, VIEWER_TIME_ZONE);

    if (user !== undefined) {
      world.users.set(VIEWER_ID, {
        ...user,
        customStatus: { emoji, text, expiresAt: iso(endOfDay) },
      });
    }

    const presence = world.presence.get(VIEWER_ID);

    if (presence !== undefined) {
      const next = { ...presence, statusText: `${emoji} ${text}` };

      world.presence.set(VIEWER_ID, next);
      ctx.publish([{ topic: "user", type: "presence", data: next }]);
    }

    return ephemeral(`Status set to “${emoji} ${text}”.`);
  };

  const dnd = (args: string, now: number): SlashCommandResult => {
    const world = ctx.world();
    const lower = args.toLowerCase();
    let active: boolean;
    let until: number | null = null;

    if (args === "") {
      const current = world.doNotDisturb;

      active = !(current.enabled && (current.until === null || Date.parse(current.until) > now));
    } else if (lower === "off" || lower === "on") {
      active = lower === "on";
    } else {
      const parsed = leadingTime(args.replace(/^until\s+/i, ""), now, VIEWER_TIME_ZONE, "morning");

      if (parsed === null || parsed.rest !== "" || parsed.at <= now) {
        return errorResult("Usage: /dnd [30m|2h|until 5pm|off] — bare /dnd toggles.");
      }

      active = true;
      until = parsed.at;
    }

    world.doNotDisturb = { enabled: active, until: until === null ? null : iso(until) };

    if (!active) return ephemeral("Do Not Disturb is off.");

    return ephemeral(
      until === null
        ? "Do Not Disturb is on."
        : `Do Not Disturb is on until ${longTime(until, VIEWER_TIME_ZONE)}.`,
    );
  };

  const outOfOffice = (args: string, now: number): SlashCommandResult => {
    const world = ctx.world();

    if (args.toLowerCase() === "off") {
      world.outOfOffice = null;

      return ephemeral("Out of office is off.");
    }

    const parsed = leadingTime(args, now, VIEWER_TIME_ZONE, "end_of_day");

    if (parsed === null) {
      return errorResult(
        "Usage: /ooo <when> [note] — for example “/ooo tomorrow Back soon”, “/ooo friday”, “/ooo 2026-10-05”, or “/ooo 3d”. Bare days and dates run to the end of the day; “/ooo friday 5pm” keeps the time. “/ooo off” clears it.",
      );
    }

    if (parsed.at <= now) return pastResult(parsed.at);

    const note = parsed.rest === "" ? null : parsed.rest;

    world.outOfOffice = { until: iso(parsed.at), note, keepNotifications: false };

    const tail = note === null ? "" : ` Note: “${note}”.`;

    return ephemeral(`Out of office until ${longDate(parsed.at, VIEWER_TIME_ZONE)}.${tail}`);
  };

  // --- preview ---

  const preview = (roomId: number, body: Json | undefined): MessagePreview => {
    ctx.roomOr404(roomId);

    const markdown = stringField(body, "markdownSource") ?? "";

    if (markdown.length > SOURCE_LIMIT) {
      throw validation(
        "markdownSource",
        `Markdown source is too long (maximum is ${SOURCE_LIMIT} characters)`,
      );
    }

    return { bodyHtml: renderMarkdown(markdown, ctx.mentionables()) };
  };

  // --- scheduled messages ---

  /** Not yet sent or dropped (`sending` ones too: a runner holds them). */
  const pending = (message: ScheduledMessage) =>
    message.sentAt === null && message.droppedAt === null;

  /** Whether a pending one could be posted now (`ScheduledMessage::sendable_ids`). */
  const sendableNow = (message: ScheduledMessage): boolean => {
    try {
      ctx.roomOr404(message.roomId);
    } catch {
      return false;
    }

    if (message.threadId === null) return true;

    return ctx.world().threads.get(message.threadId)?.roomId === message.roomId;
  };

  /** As the wire shows it: `sendable` worked out now for a pending one, `false` otherwise. */
  const shown = (message: ScheduledMessage): ScheduledMessage => ({
    ...message,
    sendable: pending(message) && sendableNow(message),
    replyTarget: replyPreview(message),
  });

  const replyPreview = (message: ScheduledMessage): ScheduledMessage["replyTarget"] => {
    if (!sendableNow(message) || message.replyToMessageId === null) return null;
    const world = ctx.world();

    const timeline =
      message.threadId === null
        ? world.rooms.get(message.roomId)?.messages
        : world.threads.get(message.threadId)?.messages;

    const target = timeline?.find(
      (candidate) => candidate.id === message.replyToMessageId && !candidate.systemNote,
    );

    if (target === undefined) return null;

    const excerpt = target.bodyHtml
      .replace(/<[^>]*>/g, "")
      .replace(/\s+/g, " ")
      .trim();

    return {
      messageId: target.id,
      roomId: target.roomId,
      threadId: target.threadId,
      creatorId: target.creatorId,
      authorName: world.users.get(target.creatorId)?.name ?? "Someone",
      roomLabel: world.rooms.get(target.roomId)?.room.name ?? "a direct message",
      excerpt: excerpt.length > 200 ? `${excerpt.slice(0, 197)}...` : excerpt,
      createdAt: target.createdAt,
    };
  };

  const changed = (message: ScheduledMessage) => {
    ctx.publish([{ topic: "user", type: "scheduled.changed", data: shown(message) }]);
  };

  const scheduledOr404 = (id: number): ScheduledMessage => {
    const message = ctx.world().scheduled.get(id);

    if (message === undefined || !pending(message)) throw notFound("Scheduled message not found");

    return message;
  };

  /** A pending one that isn't `sending` (editing or cancelling that one is a 409). */
  const idleOr409 = (id: number): ScheduledMessage => {
    const message = scheduledOr404(id);

    if (message.state === "sending") {
      throw conflict("That message is sending right now; try again in a moment.");
    }

    return message;
  };

  const sendAtOf = (body: Json | undefined): string => {
    const raw = stringField(body, "sendAt");
    const at = raw === null ? Number.NaN : Date.parse(raw);

    if (Number.isNaN(at) || at <= ctx.now())
      throw validation("sendAt", "Send at must be in the future");

    return iso(at);
  };

  const markdownOf = (body: Json | undefined): string => {
    const markdown = checkMarkdown(stringField(body, "markdownSource"), false);

    if (markdown.trim() === "") throw validation("body", "Body can't be blank");

    return markdown;
  };

  const arm = (message: ScheduledMessage) => {
    const existing = timers.get(message.id);

    if (existing !== undefined) ctx.scheduler.cancel(existing);

    timers.set(
      message.id,
      ctx.scheduler.schedule(Math.max(0, Date.parse(message.sendAt) - ctx.now()), () => {
        timers.delete(message.id);

        const current = ctx.world().scheduled.get(message.id);

        if (current !== undefined && pending(current) && current.state !== "sending") {
          send(current);
        }
      }),
    );
  };

  const disarm = (id: number) => {
    const timer = timers.get(id);

    if (timer !== undefined) ctx.scheduler.cancel(timer);

    timers.delete(id);
  };

  const drop = (message: ScheduledMessage, reason: string | null): ScheduledMessage => {
    const next: ScheduledMessage = {
      ...message,
      state: "dropped",
      sendable: false,
      droppedAt: iso(ctx.now()),
      dropReason: reason,
    };

    ctx.world().scheduled.set(message.id, next);
    changed(next);
    hooks.dropped(next);

    return next;
  };

  /**
   * Posts it as the viewer, or drops it when it can't be posted there any more: no access to the
   * room (no reason, "access lost"), its thread gone, or the post refused. A locked thread holds
   * it instead: it stays scheduled, unchanged.
   */
  const send = (message: ScheduledMessage): ScheduledMessage => {
    const world = ctx.world();
    let record: RoomRecord;

    disarm(message.id);

    try {
      record = ctx.roomOr404(message.roomId);
    } catch {
      return drop(message, null);
    }

    const thread = message.threadId === null ? null : world.threads.get(message.threadId);

    if (thread === undefined || (thread !== null && thread.roomId !== message.roomId)) {
      return drop(message, "its thread was deleted");
    }

    if (thread?.locked === true) return message;

    const target = (thread === null ? record.messages : thread.messages).find(
      (candidate) => candidate.id === message.replyToMessageId,
    );

    if (target?.systemNote) {
      return drop(message, "reply target is no longer visible in the same conversation");
    }

    const draft: MessageDraft = {
      ...plainDraft(VIEWER_ID, message.markdownSource, ctx.uuid()),
      replyToMessageId: replyPreview(message)?.messageId ?? null,
    };

    let posted: MessageDTO;

    try {
      posted = thread === null ? ctx.postToRoom(record, draft) : threads.postReply(thread, draft);
    } catch (error) {
      if (!(error instanceof HttpError)) throw error;

      return drop(message, error.message);
    }

    const next: ScheduledMessage = {
      ...message,
      state: "sent",
      sendable: false,
      sentAt: iso(ctx.now()),
      sentMessageId: posted.id,
    };

    world.scheduled.set(message.id, next);
    changed(next);

    return next;
  };

  const checkTarget = (roomId: number, threadId: number | null, replyTo: number | null) => {
    const world = ctx.world();
    let timeline: readonly MessageDTO[] = ctx.roomOr404(roomId).messages;

    if (threadId !== null) {
      const thread = world.threads.get(threadId);

      if (thread === undefined || thread.roomId !== roomId) {
        throw validation("threadId", "Thread must be in this room");
      }

      timeline = thread.messages;
    }

    if (
      replyTo !== null &&
      !timeline.some((message) => message.id === replyTo && !message.systemNote)
    ) {
      throw validation("replyToMessageId", "Reply to message must be in the same conversation");
    }
  };

  /** Pending ones soonest first; past ones (sent or dropped) most recent first. 50 a page. */
  const listScheduled = (query: URLSearchParams): ScheduledMessageList => {
    const rawRoom = query.get("roomId");
    const roomId = rawRoom === null || rawRoom === "" ? null : Number(rawRoom);
    const status = query.get("status") === "past" ? "past" : "pending";

    if (roomId !== null) ctx.roomOr404(roomId);

    const matching = [...ctx.world().scheduled.values()].filter(
      (message) =>
        pending(message) === (status === "pending") &&
        (roomId === null || message.roomId === roomId),
    );

    const page = keysetPage(
      matching,
      (message) => ({ at: message.sendAt, id: message.id }),
      beforeOf(query),
      SCHEDULED_PAGE_SIZE,
      status === "pending",
    );

    const scheduledMessages = page.rows.map(shown);

    return {
      scheduledMessages,
      conversations: conversationNames(ctx, scheduledMessages),
      nextCursor: page.nextCursor,
    };
  };

  const createScheduled = (roomId: number, body: Json | undefined): MockResponse => {
    const world = ctx.world();
    const threadId = intField(body, "threadId");
    const replyTo = intField(body, "replyToMessageId");

    ctx.roomOr404(roomId);

    const markdown = markdownOf(body);
    const sendAt = sendAtOf(body);

    checkTarget(roomId, threadId, replyTo);

    const message: ScheduledMessage = {
      id: world.nextScheduledId++,
      roomId,
      threadId,
      replyToMessageId: replyTo,
      replyTarget: null,
      markdownSource: markdown,
      sendAt,
      state: "pending",
      sendable: true,
      sentAt: null,
      sentMessageId: null,
      droppedAt: null,
      dropReason: null,
      createdAt: iso(ctx.now()),
    };

    world.scheduled.set(message.id, message);
    arm(message);
    changed(message);

    return ok(shown(message), 201);
  };

  /** Absent fields keep their values. */
  const updateScheduled = (id: number, body: Json | undefined): ScheduledMessage => {
    const current = idleOr409(id);

    const next = {
      ...current,
      markdownSource:
        stringField(body, "markdownSource") === null ? current.markdownSource : markdownOf(body),
      sendAt: stringField(body, "sendAt") === null ? current.sendAt : sendAtOf(body),
      replyToMessageId:
        field(body, "replyToMessageId") === undefined
          ? current.replyToMessageId
          : intField(body, "replyToMessageId"),
    };

    if (next.replyToMessageId !== null)
      checkTarget(next.roomId, next.threadId, next.replyToMessageId);

    ctx.world().scheduled.set(id, next);
    arm(next);
    changed(next);

    return shown(next);
  };

  const cancelScheduled = (id: number): MockResponse => {
    const current = idleOr409(id);

    disarm(id);
    ctx.world().scheduled.delete(id);
    hooks.cancelled(id);
    ctx.publish([
      { topic: "user", type: "scheduled.removed", data: { id, roomId: current.roomId } },
    ]);

    return noContent();
  };

  /** 200 sent; 202 still pending (another runner holds it, or its thread is locked); 422 dropped. */
  const sendNow = (id: number): MockResponse => {
    const current = scheduledOr404(id);

    if (current.state === "sending") return ok(shown(current), 202);

    const after = send(current);

    if (after.droppedAt !== null) {
      // The contract's `message` is the drop reason itself; a drop for lost access has none, and
      // reads as the classic page's "Not sent (channel access lost)".
      throw validation("base", after.dropReason ?? "channel access lost");
    }

    if (after.sentAt === null) {
      arm(after);

      return ok(shown(after), 202);
    }

    return ok(shown(after));
  };

  return {
    routes: [
      route("GET", /^\/autocomplete\/users$/, (request) => ok(userSuggestions(request.query))),
      route("GET", /^\/autocomplete\/icons$/, (request) => ok(iconSuggestions(request.query))),
      route("GET", /^\/icons$/, () => ok({ icons: imageIcons() })),
      route("GET", /^\/rooms\/(\d+)\/slash_commands$/, (request) =>
        ok(commands(firstId(request), request.query)),
      ),
      route("POST", /^\/rooms\/(\d+)\/slash_commands$/, (request) =>
        ok(runCommand(firstId(request), request.body)),
      ),
      route("POST", /^\/rooms\/(\d+)\/messages\/preview$/, (request) =>
        ok(preview(firstId(request), request.body)),
      ),
      route("GET", /^\/scheduled_messages$/, (request) => ok(listScheduled(request.query))),
      route("POST", /^\/rooms\/(\d+)\/scheduled_messages$/, (request) =>
        createScheduled(firstId(request), request.body),
      ),
      route("PATCH", /^\/scheduled_messages\/(\d+)$/, (request) =>
        ok(updateScheduled(firstId(request), request.body)),
      ),
      route("DELETE", /^\/scheduled_messages\/(\d+)$/, (request) =>
        cancelScheduled(firstId(request)),
      ),
      route("POST", /^\/scheduled_messages\/(\d+)\/send_now$/, (request) =>
        sendNow(firstId(request)),
      ),
    ],
    fireScheduled(all) {
      const due = [...ctx.world().scheduled.values()]
        .filter(
          (message) =>
            pending(message) &&
            message.state !== "sending" &&
            (all || Date.parse(message.sendAt) <= ctx.now()),
        )
        .sort((a, b) => Date.parse(a.sendAt) - Date.parse(b.sendAt) || a.id - b.id);

      return due.map(send).filter((message) => !pending(message));
    },
    arm() {
      for (const id of [...timers.keys()]) disarm(id);

      for (const message of ctx.world().scheduled.values()) {
        if (pending(message)) arm(message);
      }
    },
    stop() {
      for (const id of [...timers.keys()]) disarm(id);
    },
    markSending(id, on) {
      const current = scheduledOr404(id);
      const next: ScheduledMessage = { ...current, state: on ? "sending" : "pending" };

      ctx.world().scheduled.set(id, next);
      changed(next);

      if (!on) arm(next);

      return shown(next);
    },
  };
}

/** Posting into a locked thread, which a command reports instead of failing. */
class LockedThread extends Error {}
