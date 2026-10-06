/**
 * The `/api/v1/sync` wire protocol: decoding server frames (each batch event on its own, so one
 * the client doesn't know is dropped without losing the rest) and encoding client frames.
 */
import { Effect, Option, Schema, Stream } from "effect";
import {
  ServerFrame as ServerFrameSchema,
  SyncEvent as SyncEventSchema,
} from "../api/schema/sync.ts";
import { wire } from "../api/wire.ts";
import type { ClientFrame } from "../gen/ClientFrame.ts";
import type { ServerFrame } from "../gen/ServerFrame.ts";
import type { SyncEvent } from "../gen/SyncEvent.ts";

/** Every server frame, with a batch's events left undecoded until they're taken one by one. */
const Envelope = Schema.Union([
  Schema.Struct({ t: Schema.Literal("batch"), events: Schema.Array(Schema.Json) }),
  Schema.Struct({ t: Schema.Literals(["welcome", "resync", "bye", "ping"]) }),
]);

const decodeJsonText = Schema.decodeUnknownEffect(Schema.fromJsonString(Schema.Json));

const decodeEnvelope = Schema.decodeUnknownEffect(Envelope);

const decodeEvent = wire<SyncEvent>(SyncEventSchema);

const decodeOther = wire<ServerFrame>(ServerFrameSchema);

const dropped = (what: string) => (error: Schema.SchemaError) =>
  Effect.logWarning(`sync: dropped ${what}`, error.message);

/**
 * One text frame from the server, decoded and typed as the generated `ServerFrame` (the wire
 * value, as the store keeps it). A frame that doesn't decode is logged and dropped (`None`), and so
 * is each batch event of a type this client doesn't know yet: the server may deploy first.
 */
export const decodeServerFrame = Effect.fnUntraced(function* (text: string) {
  const decoded = yield* decodeJsonText(text).pipe(
    Effect.flatMap((json) => Effect.map(decodeEnvelope(json), (envelope) => ({ json, envelope }))),
    Effect.tapError(dropped("an undecodable frame")),
    Effect.option,
  );

  if (Option.isNone(decoded)) {
    return Option.none<ServerFrame>();
  }

  const { json, envelope } = decoded.value;

  if (!("events" in envelope)) {
    return yield* decodeOther(json).pipe(Effect.tapError(dropped("a frame")), Effect.option);
  }

  const events: SyncEvent[] = [];

  for (const raw of envelope.events) {
    const event = yield* decodeEvent(raw).pipe(Effect.tapError(dropped("an event")), Effect.option);

    if (Option.isSome(event)) {
      events.push(event.value);
    }
  }

  return Option.some<ServerFrame>({ t: "batch", events });
});

/** Decodes a stream of text frames, dropping what doesn't decode. */
export function decodeFrames<E>(texts: Stream.Stream<string, E>): Stream.Stream<ServerFrame, E> {
  return texts.pipe(
    Stream.mapEffect(decodeServerFrame),
    Stream.filter(Option.isSome),
    Stream.map((frame) => frame.value),
  );
}

/** A client frame as sent on the socket. */
export function encodeClientFrame(frame: ClientFrame): string {
  return JSON.stringify(frame);
}
