/** The S5 endpoints: huddles (calls), moderation and stages. */
import { Effect } from "effect";
import type { HuddleCredentials } from "../gen/HuddleCredentials.ts";
import type { HuddleDetail } from "../gen/HuddleDetail.ts";
import type { HuddlePresenceList } from "../gen/HuddlePresenceList.ts";
import type { ModerateHuddle } from "../gen/ModerateHuddle.ts";
import type { StageDetail } from "../gen/StageDetail.ts";
import type { StageRole } from "../gen/StageRole.ts";
import type { StageState } from "../gen/StageState.ts";
import type { StageStream } from "../gen/StageStream.ts";
import type { StreamQuality } from "../gen/StreamQuality.ts";
import { call, get, noContent } from "./call.ts";
import { ApiClient, ApiConfig } from "./client.ts";
import {
  HuddleCredentials as HuddleCredentialsSchema,
  HuddleDetail as HuddleDetailSchema,
  HuddlePresenceList as HuddlePresenceListSchema,
} from "./schema/huddle.ts";
import {
  StageDetail as StageDetailSchema,
  StageState as StageStateSchema,
  StageStream as StageStreamSchema,
} from "./schema/stage.ts";
import { wire } from "./wire.ts";

/** `GET /huddles`: every call in the viewer's rooms (the sidebar's aggregate poll). */
export const huddles = Effect.fn("api.huddles")(function* () {
  return yield* call(get("/huddles"), wire<HuddlePresenceList>(HuddlePresenceListSchema));
});

/** `GET /rooms/:id/huddle`: one room's call; a 401, 403 or 404 means the viewer's access ended. */
export const huddle = Effect.fn("api.huddle")(function* (roomId: number) {
  return yield* call(get(`/rooms/${roomId}/huddle`), wire<HuddleDetail>(HuddleDetailSchema));
});

/** `POST /rooms/:id/huddle`: a fresh grant and its LiveKit credentials. */
export const joinHuddle = Effect.fn("api.joinHuddle")(function* (roomId: number) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/huddle`, body: {} },
    wire<HuddleCredentials>(HuddleCredentialsSchema),
  );
});

/** `POST /rooms/:id/huddle/leave`: this session is out of the room's call. */
export const leaveHuddle = Effect.fn("api.leaveHuddle")(function* (roomId: number) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/huddle/leave`, body: {} },
    noContent,
  );
});

/** `POST /rooms/:id/huddle/moderation`: mute, unmute or disconnect someone in the call. */
export const moderateHuddle = Effect.fn("api.moderateHuddle")(function* (
  roomId: number,
  body: ModerateHuddle,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/huddle/moderation`, body: { ...body } },
    noContent,
  );
});

/** `GET /rooms/:id/stage`: the roster, the live stream and every member's profile. */
export const stage = Effect.fn("api.stage")(function* (roomId: number) {
  return yield* call(get(`/rooms/${roomId}/stage`), wire<StageDetail>(StageDetailSchema));
});

/** `PATCH /rooms/:id/stage/members/:membershipId`: a host moves someone between roles. */
export const changeStageRole = Effect.fn("api.changeStageRole")(function* (
  roomId: number,
  membershipId: number,
  role: StageRole,
) {
  return yield* call(
    { method: "PATCH", path: `/rooms/${roomId}/stage/members/${membershipId}`, body: { role } },
    wire<StageState>(StageStateSchema),
  );
});

/** `POST /rooms/:id/stage/hand`: a listener asks to speak. */
export const raiseHand = Effect.fn("api.raiseHand")(function* (roomId: number) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/stage/hand`, body: {} },
    wire<StageState>(StageStateSchema),
  );
});

/** `DELETE /rooms/:id/stage/hand`: the viewer's own hand, or a host lowering someone else's. */
export const lowerHand = Effect.fn("api.lowerHand")(function* (
  roomId: number,
  membershipId: number | null,
) {
  const path = `/rooms/${roomId}/stage/hand`;

  return yield* call(
    membershipId === null
      ? { method: "DELETE", path }
      : { method: "DELETE", path, query: { membershipId: String(membershipId) } },
    wire<StageState>(StageStateSchema),
  );
});

/** `POST /rooms/:id/stage/stream`: go live. */
export const startStream = Effect.fn("api.startStream")(function* (
  roomId: number,
  quality: StreamQuality,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/stage/stream`, body: { quality } },
    wire<StageStream>(StageStreamSchema),
  );
});

function streamPath(roomId: number, streamId: number | null): string {
  return streamId === null
    ? `/rooms/${roomId}/stage/stream`
    : `/rooms/${roomId}/stage/stream?streamId=${streamId}`;
}

/** `DELETE /rooms/:id/stage/stream`: end the live stream (only `streamId`, when given). */
export const stopStream = Effect.fn("api.stopStream")(function* (
  roomId: number,
  streamId: number | null,
) {
  return yield* call(
    streamId === null
      ? { method: "DELETE", path: `/rooms/${roomId}/stage/stream` }
      : {
          method: "DELETE",
          path: `/rooms/${roomId}/stage/stream`,
          query: { streamId: String(streamId) },
        },
    noContent,
  );
});

/**
 * `PATCH /activity/:id` (S3's activity endpoint): a ring answered (`handled`) or dismissed
 * (`read`). The reply is S3's `ActivityItemChanged`; the call only needs it to succeed.
 */
export const answerRing = Effect.fn("api.answerRing")(function* (
  activityItemId: number,
  action: "handled" | "read",
) {
  return yield* call(
    { method: "PATCH", path: `/activity/${activityItemId}`, body: { action } },
    () => Effect.void,
  );
});

/**
 * Requests that must outlive the page (leaving a call or ending a stream while the tab closes):
 * a `keepalive` fetch with the CSRF token, which the app's HTTP client can't send. Best effort:
 * the server's liveness window and stale-stream reconciler end both anyway.
 */
export const sendOnUnload = Effect.fn("api.sendOnUnload")(function* (
  method: "POST" | "DELETE",
  path: string,
) {
  const { baseUrl } = yield* ApiConfig;
  const token = yield* ApiClient.use((client) => client.csrfToken);

  if (token === null) {
    return;
  }

  const init: RequestInit = {
    method,
    credentials: "same-origin",
    keepalive: true,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "X-CSRF-Token": token,
    },
  };

  if (method === "POST") {
    init.body = "{}";
  }

  yield* Effect.sync(() => {
    void fetch(`${baseUrl}${path}`, init).catch(() => undefined);
  });
});

/** The unload-time leave. */
export const leaveOnUnload = (roomId: number) =>
  sendOnUnload("POST", `/rooms/${roomId}/huddle/leave`);

/** The unload-time stream end. */
export const stopStreamOnUnload = (roomId: number, streamId: number | null) =>
  sendOnUnload("DELETE", streamPath(roomId, streamId));
