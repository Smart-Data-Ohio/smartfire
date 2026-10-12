import { Schema } from "effect";
import type { CreateSoundboardSound as GeneratedCreateSoundboardSound } from "../../gen/CreateSoundboardSound.ts";
import type { HuddleSoundboardPlayed as GeneratedHuddleSoundboardPlayed } from "../../gen/HuddleSoundboardPlayed.ts";
import type { PlaySoundboardSound as GeneratedPlaySoundboardSound } from "../../gen/PlaySoundboardSound.ts";
import type { SoundboardCatalog as GeneratedSoundboardCatalog } from "../../gen/SoundboardCatalog.ts";
import type { SoundboardSound as GeneratedSoundboardSound } from "../../gen/SoundboardSound.ts";
import type { UpdateSoundboardSound as GeneratedUpdateSoundboardSound } from "../../gen/UpdateSoundboardSound.ts";
import { RoomId, UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

export const SoundboardSoundId = Schema.Int.pipe(Schema.brand("SoundboardSoundId"));

export type SoundboardSoundId = typeof SoundboardSoundId.Type;

export const SoundboardSound = Schema.Struct({
  id: SoundboardSoundId,
  name: Schema.String,
  emoji: Schema.NullOr(Schema.String),
  volume: Schema.Number,
  uploaderId: UserId,
  audioUrl: Schema.String,
  createdAt: Timestamp,
  updatedAt: Timestamp,
});

export type SoundboardSound = typeof SoundboardSound.Type;

export type SoundboardSoundPin = Assert<Pinned<typeof SoundboardSound, GeneratedSoundboardSound>>;

export const SoundboardCatalog = Schema.Struct({
  sounds: Schema.Array(SoundboardSound),
  capacity: Schema.Int,
});

export type SoundboardCatalog = typeof SoundboardCatalog.Type;

export type SoundboardCatalogPin = Assert<
  Pinned<typeof SoundboardCatalog, GeneratedSoundboardCatalog>
>;

export const CreateSoundboardSound = Schema.Struct({
  name: Schema.String,
  emoji: Schema.NullOr(Schema.String),
  volume: Schema.NullOr(Schema.Number),
  signedId: Schema.String,
});

export type CreateSoundboardSound = typeof CreateSoundboardSound.Type;

export type CreateSoundboardSoundPin = Assert<
  Pinned<typeof CreateSoundboardSound, GeneratedCreateSoundboardSound>
>;

export const UpdateSoundboardSound = Schema.Struct({
  name: Schema.String,
  emoji: Schema.NullOr(Schema.String),
  volume: Schema.Number,
});

export type UpdateSoundboardSound = typeof UpdateSoundboardSound.Type;

export type UpdateSoundboardSoundPin = Assert<
  Pinned<typeof UpdateSoundboardSound, GeneratedUpdateSoundboardSound>
>;

export const PlaySoundboardSound = Schema.Struct({
  identity: Schema.String,
  soundId: SoundboardSoundId,
});

export type PlaySoundboardSound = typeof PlaySoundboardSound.Type;

export type PlaySoundboardSoundPin = Assert<
  Pinned<typeof PlaySoundboardSound, GeneratedPlaySoundboardSound>
>;

/** The `huddle.soundboard` event: someone in the room's call played a sound. */
export const HuddleSoundboardPlayed = Schema.Struct({
  roomId: RoomId,
  soundId: SoundboardSoundId,
  userId: UserId,
  identity: Schema.String,
  playedAt: Timestamp,
});

export type HuddleSoundboardPlayed = typeof HuddleSoundboardPlayed.Type;

export type HuddleSoundboardPlayedPin = Assert<
  Pinned<typeof HuddleSoundboardPlayed, GeneratedHuddleSoundboardPlayed>
>;
