/**
 * The voice soundboard: its catalog for any signed-in member, the administrators' writes (each
 * answers the whole catalog, as the server does), and playing a sound to the viewer's call. A
 * play is relayed by the server, which checks the caller is in the call and may speak there, then
 * sends `huddle.soundboard` to everyone in it; each client plays the sound itself.
 */
import { Effect } from "effect";
import type { CreateSoundboardSound } from "../gen/CreateSoundboardSound.ts";
import type { PlaySoundboardSound } from "../gen/PlaySoundboardSound.ts";
import type { SoundboardCatalog } from "../gen/SoundboardCatalog.ts";
import type { UpdateSoundboardSound } from "../gen/UpdateSoundboardSound.ts";
import { call, get, noContent } from "./call.ts";
import { SoundboardCatalog as SoundboardCatalogSchema } from "./schema/soundboard.ts";
import { wire } from "./wire.ts";

const catalogReply = wire<SoundboardCatalog>(SoundboardCatalogSchema);

/** `GET /soundboard/sounds`: every sound, by name, and how many the workspace may hold. */
export const soundboardCatalog = Effect.fn("api.soundboardCatalog")(function* () {
  return yield* call(get("/soundboard/sounds"), catalogReply);
});

/** `POST /admin/soundboard/sounds`: a new sound from a finished direct upload. */
export const createSoundboardSound = Effect.fn("api.createSoundboardSound")(function* (
  body: CreateSoundboardSound,
) {
  return yield* call({ method: "POST", path: "/admin/soundboard/sounds", body }, catalogReply);
});

/** `PATCH /admin/soundboard/sounds/:id`: a new name, emoji or default volume. */
export const updateSoundboardSound = Effect.fn("api.updateSoundboardSound")(function* (
  soundId: number,
  body: UpdateSoundboardSound,
) {
  return yield* call(
    { method: "PATCH", path: `/admin/soundboard/sounds/${soundId}`, body },
    catalogReply,
  );
});

/** `DELETE /admin/soundboard/sounds/:id`. */
export const destroySoundboardSound = Effect.fn("api.destroySoundboardSound")(function* (
  soundId: number,
) {
  return yield* call(
    { method: "DELETE", path: `/admin/soundboard/sounds/${soundId}` },
    catalogReply,
  );
});

/** `POST /rooms/:id/huddle/soundboard`: plays a sound to everyone in the room's call. */
export const playSoundboardSound = Effect.fn("api.playSoundboardSound")(function* (
  roomId: number,
  body: PlaySoundboardSound,
) {
  return yield* call(
    { method: "POST", path: `/rooms/${roomId}/huddle/soundboard`, body },
    noContent,
  );
});
