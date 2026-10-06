import { Layer } from "effect";
import { Cursor } from "./cursor.ts";
import { Engine } from "./engine.ts";
import { SyncLink } from "./link.ts";
import { Outbox } from "./outbox.ts";
import { Presence } from "./presence.ts";
import { Topics } from "./topics.ts";
import { Typing } from "./typing.ts";

/**
 * Every sync service, sharing one `SyncLink`. Needs the platform pieces: a `SyncSocket`, a
 * `Lifecycle` and an `ApiClient` (the browser's in the app, in-memory ones in tests).
 */
export const SyncServices = Layer.mergeAll(Engine.layer, Outbox.layer).pipe(
  Layer.provideMerge(Layer.mergeAll(Topics.layer, Typing.layer, Presence.layer, Cursor.layer)),
  Layer.provideMerge(SyncLink.layer),
);
