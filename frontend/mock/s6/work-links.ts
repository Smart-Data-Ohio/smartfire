import type { WorkLink } from "../../src/gen/WorkLink.ts";
import type { WorkLinkForm } from "../../src/gen/WorkLinkForm.ts";
import { notFound, ok, validation } from "../http.ts";
import { field, intField, type Json, stringField } from "../json.ts";
import { firstId, route, type S2Context } from "../s2/context.ts";
import { type ThreadRecord, threadDto } from "../s2/model.ts";
import type { Threads } from "../s2/threads.ts";
import { VIEWER_ID } from "../seed.ts";

export interface WorkLinkEvent {
  id: number;
  roomId: number;
  title: string;
  startsAt: string;
  endsAt: string | null;
  timeZone: string;
  cancelled: boolean;
}

const duplicate = "That is already linked to this work thread.";

const prPrompt = "Enter a GitHub pull request URL, like https://github.com/owner/repo/pull/123.";

const drivePrompt = "Enter a Google Drive, Docs, Sheets, Slides, or Forms link.";

const invalid = (input: string, message: string) => validation(input, message, message);

/** The classic Ruby strip removes ASCII whitespace (and NUL), not Unicode spacing. */
function strip(text: string): string {
  const whitespace = (code: number) => code === 0 || code === 32 || (code >= 9 && code <= 13);
  let start = 0;
  let end = text.length;

  while (start < end && whitespace(text.charCodeAt(start))) start += 1;

  while (end > start && whitespace(text.charCodeAt(end - 1))) end -= 1;

  return text.slice(start, end);
}

const drivePatterns = [
  /^https:\/\/docs\.google\.com\/(?:u\/\d+\/)?(?:document|spreadsheets|presentation|forms)\/(?:u\/\d+\/)?d\/([A-Za-z0-9_-]{10,})/i,
  /^https:\/\/drive\.google\.com\/(?:u\/\d+\/)?file\/(?:u\/\d+\/)?d\/([A-Za-z0-9_-]{10,})/i,
  /^https:\/\/drive\.google\.com\/(?:u\/\d+\/)?drive\/(?:u\/\d+\/)?folders\/([A-Za-z0-9_-]{10,})/i,
  /^https:\/\/drive\.google\.com\/(?:u\/\d+\/)?open\?(?:[^#]*&)?id=([A-Za-z0-9_-]{10,})(?:&|#|$)/i,
];

function pullRequest(text: string): { label: string; url: string } | null {
  for (const match of text.matchAll(
    /https:\/\/github\.com\/([A-Za-z0-9_.-]+)\/([A-Za-z0-9_.-]+)\/(?:pull|pulls)\/(\d+)\b/g,
  )) {
    const owner = match[1] ?? "";
    const repo = match[2] ?? "";

    if ([".", ".."].includes(owner) || [".", ".."].includes(repo)) continue;
    const number = BigInt(match[3] ?? "0");

    if (number > 9223372036854775807n) throw new Error("PR number does not fit SQLite");

    if (number === 0n) throw invalid("pullRequestUrl", "Number must be greater than 0");
    const name = `${owner.toLowerCase()}/${repo.toLowerCase()}`;

    return { label: `${name}#${number}`, url: `https://github.com/${name}/pull/${number}` };
  }

  return null;
}

export function createWorkLinks(
  ctx: Pick<S2Context, "world" | "now" | "publish">,
  threads: Pick<Threads, "threadOr404" | "detail">,
) {
  const scope = (threadId: number) => {
    const thread = threads.threadOr404(threadId);
    const viewer = ctx.world().users.get(VIEWER_ID);

    if (
      viewer?.status !== "active" ||
      viewer.role === "bot" ||
      !ctx.world().rooms.get(thread.roomId)?.memberIds.includes(VIEWER_ID)
    )
      throw notFound();

    if (thread.work == null) throw invalid("base", "This thread isn't tracked as work");

    return thread;
  };

  const form = (threadId: number): WorkLinkForm => {
    const thread = scope(threadId);

    const linked = new Set(
      thread.work?.links.filter(({ kind }) => kind === "event").map(({ url }) => url),
    );

    return {
      events: [...ctx.world().workLinkEvents.values()]
        .filter(
          (event) =>
            event.roomId === thread.roomId &&
            !event.cancelled &&
            Date.parse(event.endsAt ?? event.startsAt) >= ctx.now() &&
            !linked.has(`/rooms/${thread.roomId}/events/${event.id}`),
        )
        .sort((a, b) => a.startsAt.localeCompare(b.startsAt) || a.id - b.id)
        .map(({ id, title, startsAt, timeZone }) => ({ id, title, startsAt, timeZone })),
    };
  };

  const publish = (thread: ThreadRecord) => {
    const data = threadDto(thread, ctx.now());
    ctx.publish([
      { topic: `room:${thread.roomId}`, type: "thread.updated", data },
      { topic: `thread:${thread.id}`, type: "thread.updated", data },
    ]);
  };

  const add = (threadId: number, body: Json | undefined) => {
    const thread = scope(threadId);
    const work = thread.work;

    if (work == null) throw new Error("Scoped work missing");
    const id = ctx.world().nextWorkLinkId;

    const base = {
      id,
      pullRequestState: null,
      title: null,
      eventStartsAt: null,
      eventTimeZone: null,
      eventCancelled: false,
    };

    const kind = stringField(body, "kind");
    let link: WorkLink;
    let input: string;

    if (kind === "pull_request") {
      const reference = pullRequest(stringField(body, "pullRequestUrl") ?? "");

      if (reference === null) throw invalid("pullRequestUrl", prPrompt);
      link = { ...base, kind, ...reference, pullRequestState: "open" };
      input = "pullRequestUrl";
    } else if (kind === "event") {
      if (field(body, "eventId") == null) throw invalid("eventId", "Choose an event to link.");
      const event = ctx.world().workLinkEvents.get(intField(body, "eventId") ?? 0);

      if (event === undefined || event.roomId !== thread.roomId) throw notFound();
      link = {
        ...base,
        kind,
        label: event.title,
        url: `/rooms/${thread.roomId}/events/${event.id}`,
        eventStartsAt: event.startsAt,
        eventTimeZone: event.timeZone,
        eventCancelled: event.cancelled,
      };
      input = "eventId";
    } else if (kind === "drive_file") {
      const url = strip(stringField(body, "driveUrl") ?? "");

      if (!drivePatterns.some((pattern) => pattern.test(url)))
        throw invalid("driveUrl", drivePrompt);
      link = { ...base, kind, label: url, url };
      input = "driveUrl";
    } else {
      throw invalid("kind", "Choose a pull request, event, or Drive file to link.");
    }

    if (
      work.links.some(
        (existing) =>
          existing.kind === link.kind &&
          (kind === "pull_request"
            ? pullRequest(existing.url)?.url === link.url
            : existing.url === link.url),
      )
    )
      throw invalid(input, duplicate);
    thread.work = { ...work, links: [...work.links, link] };
    ctx.world().nextWorkLinkId += 1;
    publish(thread);

    return threads.detail(thread);
  };

  const remove = (threadId: number, linkId: number) => {
    const thread = scope(threadId);
    const work = thread.work;

    if (work == null) throw new Error("Scoped work missing");

    if (!work.links.some(({ id }) => id === linkId)) throw notFound();
    thread.work = { ...work, links: work.links.filter(({ id }) => id !== linkId) };
    publish(thread);

    return threads.detail(thread);
  };

  return {
    routes: [
      route("GET", /^\/threads\/(\d+)\/work\/links\/new$/, (request) => ok(form(firstId(request)))),
      route("POST", /^\/threads\/(\d+)\/work\/links$/, (request) =>
        ok(add(firstId(request), request.body), 201),
      ),
      route("DELETE", /^\/threads\/(\d+)\/work\/links\/(\d+)$/, (request) =>
        ok(remove(firstId(request), request.ids[1] ?? 0)),
      ),
    ],
  };
}
