import type { CreatedFizzyCard } from "../../src/gen/CreatedFizzyCard.ts";
import type { FizzyBoard } from "../../src/gen/FizzyBoard.ts";
import type { FizzyMessageCardForm } from "../../src/gen/FizzyMessageCardForm.ts";
import { HttpError, notFound, ok, plainError, validation } from "../http.ts";
import { type Json, stringField } from "../json.ts";
import { type Route, route, type S2Context } from "../s2/context.ts";
import { plainDraft } from "../s2/model.ts";
import type { Threads } from "../s2/threads.ts";
import { VIEWER_ID, type World } from "../seed.ts";

const VALIDATION = "Validation";

const FIZZY_REPLY_FAILED = "FizzyReplyFailed";

const boards: FizzyBoard[] = [
  { id: "engineering", name: "Engineering" },
  { id: "support", name: "Support" },
  { id: "roadmap", name: "Roadmap" },
];

type Mode =
  | "connected"
  | "not-connected"
  | "unreachable"
  | "rejected"
  | "read-only"
  | "probe-unreachable"
  | "refused"
  | "reply-failed";

interface State {
  mode: Mode;
  nextNumber: number;
}

/** Per-world credentials reset with the mock world. Tokens never enter the mock wire data. */
export function createFizzy(ctx: S2Context, threads: Threads) {
  const states = new WeakMap<World, State>();

  const state = () => {
    const world = ctx.world();
    const held = states.get(world);

    if (held !== undefined) return held;

    const fresh: State = { mode: "connected", nextNumber: 580 };
    states.set(world, fresh);

    return fresh;
  };

  const source = (roomId: number, threadId: number | null, messageId: number) => {
    const room = ctx.roomOr404(roomId);
    const thread = threadId === null ? null : threads.threadOr404(threadId);

    if (thread !== null && thread.roomId !== roomId) throw notFound();

    const message = (thread === null ? room.messages : thread.messages).find(
      (held) => held.id === messageId,
    );

    if (message === undefined) throw notFound();

    const link =
      thread === null
        ? `/rooms/${roomId}/@${messageId}`
        : `/rooms/${roomId}?thread=${threadId}&message_id=${messageId}`;

    // The mock's canonical host is stable so source links are reproducible in tests.
    return { room, thread, message, link: `https://smartfire.test${link}` };
  };

  const rejected = () => {
    state().mode = "not-connected";

    return plainError(
      422,
      "FizzyTokenRejected",
      "Fizzy rejected the linked token. Reconnect on your profile.",
    );
  };

  const load = (roomId: number, threadId: number | null, messageId: number) => {
    const held = source(roomId, threadId, messageId);
    const mode = state().mode;

    if (mode === "rejected") throw rejected();

    if (mode === "unreachable") {
      throw plainError(503, "FizzyUnreachable", "Could not reach Fizzy. Try again.");
    }

    const text = held.message.bodyHtml
      .replace(/<br\s*\/?\s*>/gi, "\n")
      .replace(/<\/(?:p|div|li|h[1-6])>/gi, "\n")
      .replace(/<[^>]*>/g, "")
      .replace(/&lt;/g, "<")
      .replace(/&gt;/g, ">")
      .replace(/&quot;/g, '"')
      .replace(/&#39;/g, "'")
      .replace(/&amp;/g, "&")
      .trimEnd();

    const truncate = (value: string, length: number) => {
      const characters = [...value];

      return characters.length > length ? `${characters.slice(0, length - 3).join("")}...` : value;
    };

    const form: FizzyMessageCardForm = {
      connected: mode !== "not-connected",
      boards: mode === "not-connected" ? [] : boards,
      title: truncate(text.split("\n")[0]?.trim() ?? "", 120),
      description: text.trim() === "" ? `Source: ${held.link}` : `${text}\n\nSource: ${held.link}`,
      excerpt: truncate(text, 280),
      authorName: ctx.usersFor([held.message.creatorId])[0]?.name ?? "",
      roomDisplayName: ctx.displayName(held.room),
      fizzyUserName: ctx.usersFor([VIEWER_ID])[0]?.name ?? "",
      accountName: "Smart Data",
    };

    return ok(form);
  };

  const create = (
    roomId: number,
    threadId: number | null,
    messageId: number,
    body: Json | undefined,
  ) => {
    const held = source(roomId, threadId, messageId);
    const mode = state().mode;

    if (mode === "not-connected") {
      throw plainError(422, "FizzyNotConnected", "Connect Fizzy on your profile first.");
    }

    if (held.thread?.locked) throw plainError(409, "FizzyThreadLocked", "This thread is locked");

    const boardId = stringField(body, "boardId") ?? "";
    const title = (stringField(body, "title") ?? "").trim();
    const fields: Record<string, string[]> = {};

    if (boardId.trim() === "") fields.boardId = ["Choose a board."];

    if (title === "") fields.title = ["Enter a title."];

    if (Object.keys(fields).length > 0) {
      // Classic re-fetches boards before rendering a failed validation.
      load(roomId, threadId, messageId);
      throw new HttpError(422, {
        _tag: VALIDATION,
        message: "Choose a board and enter a title.",
        fields,
      });
    }

    if (mode === "rejected") throw rejected();

    if (mode === "unreachable") {
      throw plainError(
        503,
        "FizzyUnreachable",
        "Fizzy refused the new card (Could not reach Fizzy (Econnrefused)).",
      );
    }

    if (mode === "probe-unreachable") {
      throw plainError(
        503,
        "FizzyUnreachable",
        "Could not reach Fizzy to verify the token. Try again.",
      );
    }

    if (mode === "read-only") {
      throw plainError(
        422,
        "FizzyReadOnly",
        "That Fizzy token is read-only. Generate a Read + Write token to create cards.",
      );
    }

    if (mode === "refused" || !boards.some((board) => board.id === boardId)) {
      throw plainError(
        422,
        "FizzyRefused",
        "Fizzy refused the new card (Fizzy refused: Board is unavailable).",
      );
    }

    const number = String(state().nextNumber++);
    const url = `https://fizzy.test/897362094/cards/${number}`;

    if (mode === "reply-failed") {
      throw new HttpError(422, {
        _tag: FIZZY_REPLY_FAILED,
        number,
        url,
        message: `Fizzy card #${number} created, but the reply could not be posted (Body is too long).`,
      });
    }

    const draft = {
      ...plainDraft(VIEWER_ID, `Created from ${held.link}:\n${url}`, ctx.uuid()),
      replyToMessageId: messageId,
    };

    const message =
      held.thread === null
        ? ctx.postToRoom(held.room, draft)
        : threads.postReply(held.thread, draft);

    const created: CreatedFizzyCard = {
      number,
      url,
      message,
      notice: `Fizzy card #${number} created.`,
    };

    return ok(created, 201);
  };

  const routes: Route[] = [];

  for (const threaded of [false, true]) {
    const base = threaded
      ? /^\/rooms\/(\d+)\/threads\/(\d+)\/messages\/(\d+)\/fizzy_cards/
      : /^\/rooms\/(\d+)\/messages\/(\d+)\/fizzy_cards/;

    const scope = (ids: readonly number[]) => ({
      roomId: ids[0] ?? 0,
      threadId: threaded ? (ids[1] ?? 0) : null,
      messageId: ids[threaded ? 2 : 1] ?? 0,
    });

    routes.push(
      route("GET", new RegExp(`${base.source}/new$`), ({ ids }) => {
        const held = scope(ids);

        return load(held.roomId, held.threadId, held.messageId);
      }),
      route("POST", new RegExp(`${base.source}$`), ({ ids, body }) => {
        const held = scope(ids);

        return create(held.roomId, held.threadId, held.messageId, body);
      }),
    );
  }

  const control = (body: Json | undefined): Json => {
    const mode = stringField(body, "mode");

    switch (mode) {
      case "connected":
      case "not-connected":
      case "unreachable":
      case "rejected":
      case "read-only":
      case "probe-unreachable":
      case "refused":
      case "reply-failed":
        state().mode = mode;

        return { mode };
      default:
        throw validation("mode", "Choose a Fizzy mode");
    }
  };

  return { routes, control };
}
