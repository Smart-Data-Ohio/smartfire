/**
 * The voice soundboard for the mock backend: the catalog every member reads, and the
 * administrators' writes (each answering the whole catalog), validated as the server words its
 * refusals (name, emoji, volume, the audio file's type and size, and the 48-sound limit). The
 * seeded sounds are tiny generated WAV files stored as finished blobs, so their `audioUrl`s play
 * from the mock's blob route like any upload. Kept per world, so `reset()` starts it over.
 */
import type { SoundboardCatalog } from "../../src/gen/SoundboardCatalog.ts";
import type { SoundboardSound } from "../../src/gen/SoundboardSound.ts";
import { type MockResponse, notFound } from "../http.ts";
import { field, isNumber, type Json, stringField } from "../json.ts";
import { firstId, type Route, route, type S2Context } from "../s2/context.ts";
import { attachmentOf, storeBlob, type Uploads } from "../s2/uploads.ts";
import { invalid } from "../s4/http.ts";
import { timestamp, VIEWER_ID, type World } from "../seed.ts";

/** How many sounds a workspace may hold, as the server's `CAPACITY`. */
export const SOUNDBOARD_CAPACITY = 48;

const MAX_BYTES = 512 * 1024;

const AUDIO_TYPES = new Set(["audio/mpeg", "audio/ogg", "audio/wav", "audio/x-wav", "audio/webm"]);

/** One pictograph (with its variation selector, skin tone or joined parts), or a flag. */
const SINGLE_EMOJI =
  /^(?:\p{Regional_Indicator}{2}|[#*0-9]️?⃣|\p{Extended_Pictographic}️?\p{Emoji_Modifier}?(?:‍\p{Extended_Pictographic}️?\p{Emoji_Modifier}?)*)$/u;

interface SoundState {
  sounds: SoundboardSound[];
  nextId: number;
}

export interface Soundboard {
  readonly routes: readonly Route[];
  /** Whether the catalog holds `soundId`. */
  readonly has: (soundId: number) => boolean;
}

/** A short 8-bit mono WAV: `tone` Hz for a tenth of a second, enough to play and end. */
export function tinyWav(tone: number): Uint8Array {
  const rate = 8000;
  const samples = rate / 10;
  const bytes = new Uint8Array(44 + samples);
  const view = new DataView(bytes.buffer);

  const text = (offset: number, value: string) => {
    for (let index = 0; index < value.length; index += 1) {
      bytes[offset + index] = value.charCodeAt(index);
    }
  };

  text(0, "RIFF");
  view.setUint32(4, 36 + samples, true);
  text(8, "WAVE");
  text(12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, rate, true);
  view.setUint32(28, rate, true);
  view.setUint16(32, 1, true);
  view.setUint16(34, 8, true);
  text(36, "data");
  view.setUint32(40, samples, true);

  for (let index = 0; index < samples; index += 1) {
    bytes[44 + index] = 128 + Math.round(40 * Math.sin((2 * Math.PI * tone * index) / rate));
  }

  return bytes;
}

const SEEDED = [
  { name: "Airhorn", emoji: "📯", volume: 1, tone: 440 },
  { name: "Applause", emoji: "👏", volume: 0.8, tone: 660 },
  { name: "Rimshot", emoji: null, volume: 0.6, tone: 880 },
] as const;

/** A field and the server's message about it. */
type Refusal = readonly [field: string, message: string];

const fold = (name: string) => name.toLocaleLowerCase("en");

export function createSoundboard(ctx: S2Context, uploads: Uploads): Soundboard {
  const worlds = new WeakMap<World, SoundState>();

  const seed = (): SoundState => {
    const createdAt = timestamp(ctx.now());

    const sounds = SEEDED.map((each, index): SoundboardSound => {
      const blob = storeBlob(ctx, `${fold(each.name)}.wav`, "audio/wav", tinyWav(each.tone));

      return {
        id: index + 1,
        name: each.name,
        emoji: each.emoji,
        volume: each.volume,
        uploaderId: VIEWER_ID,
        audioUrl: attachmentOf(blob).url,
        createdAt,
        updatedAt: createdAt,
      };
    });

    return { sounds, nextId: sounds.length + 1 };
  };

  const current = (): SoundState => {
    const world = ctx.world();
    let state = worlds.get(world);

    if (state === undefined) {
      state = seed();
      worlds.set(world, state);
    }

    return state;
  };

  const catalog = (status = 200): MockResponse => {
    const json: SoundboardCatalog = {
      sounds: [...current().sounds].sort((a, b) => fold(a.name).localeCompare(fold(b.name))),
      capacity: SOUNDBOARD_CAPACITY,
    };

    return { status, json };
  };

  /** The editable fields' refusals, as the model words them. */
  const valuesErrors = (
    name: string,
    emoji: string | null,
    volume: number,
    id: number | null,
  ): Refusal[] => {
    const refusals: Refusal[] = [];
    const length = [...name].length;

    if (length < 1 || length > 32) {
      refusals.push(["name", "must be between 1 and 32 characters"]);
    } else if (
      current().sounds.some((sound) => sound.id !== id && fold(sound.name) === fold(name))
    ) {
      refusals.push(["name", "has already been taken"]);
    }

    if (emoji !== null && !SINGLE_EMOJI.test(emoji)) {
      refusals.push(["emoji", "must be a single Unicode emoji"]);
    }

    if (!Number.isFinite(volume) || volume < 0 || volume > 1) {
      refusals.push(["volume", "must be between 0.0 and 1.0"]);
    }

    return refusals;
  };

  const values = (body: Json | undefined, fallbackVolume: number | null) => {
    const raw = field(body, "volume");
    const emoji = (stringField(body, "emoji") ?? "").trim();

    return {
      name: (stringField(body, "name") ?? "").trim(),
      emoji: emoji === "" ? null : emoji,
      volume: isNumber(raw) ? raw : (fallbackVolume ?? Number.NaN),
    };
  };

  const create = (body: Json | undefined): MockResponse => {
    const held = current();
    const signedId = stringField(body, "signedId") ?? "";
    let audio: { url: string; contentType: string; byteSize: number };

    try {
      audio = uploads.attachment(signedId);
    } catch {
      throw invalid([["audio", "isn't an uploaded file"]]);
    }

    if (audio.byteSize < 1 || audio.byteSize > MAX_BYTES) {
      throw invalid([["audio", "must be between 1 byte and 512 KB"]]);
    }

    if (!AUDIO_TYPES.has(audio.contentType)) {
      throw invalid([["audio", "must be a readable MP3, Ogg, WAV or audio-only WebM file"]]);
    }

    const { name, emoji, volume } = values(body, 1);
    const refusals = valuesErrors(name, emoji, volume, null);

    if (held.sounds.length >= SOUNDBOARD_CAPACITY) {
      refusals.push(["base", `soundboard capacity reached (limit: ${SOUNDBOARD_CAPACITY} sounds)`]);
    }

    if (refusals.length > 0) throw invalid(refusals);

    const now = timestamp(ctx.now());

    held.sounds = [
      ...held.sounds,
      {
        id: held.nextId,
        name,
        emoji,
        volume,
        uploaderId: VIEWER_ID,
        audioUrl: audio.url,
        createdAt: now,
        updatedAt: now,
      },
    ];
    held.nextId += 1;

    return catalog(201);
  };

  const update = (id: number, body: Json | undefined): MockResponse => {
    const held = current();
    const sound = held.sounds.find((each) => each.id === id);

    if (sound === undefined) throw notFound();

    const { name, emoji, volume } = values(body, null);
    const refusals = valuesErrors(name, emoji, volume, id);

    if (refusals.length > 0) throw invalid(refusals);

    held.sounds = held.sounds.map((each) =>
      each.id === id ? { ...each, name, emoji, volume, updatedAt: timestamp(ctx.now()) } : each,
    );

    return catalog();
  };

  const destroy = (id: number): MockResponse => {
    const held = current();

    if (!held.sounds.some((each) => each.id === id)) throw notFound();

    held.sounds = held.sounds.filter((each) => each.id !== id);

    return catalog();
  };

  return {
    routes: [
      route("GET", /^\/soundboard\/sounds$/, () => catalog()),
      route("POST", /^\/admin\/soundboard\/sounds$/, ({ body }) => create(body)),
      route("PATCH", /^\/admin\/soundboard\/sounds\/(\d+)$/, (request) =>
        update(firstId(request), request.body),
      ),
      route("DELETE", /^\/admin\/soundboard\/sounds\/(\d+)$/, (request) =>
        destroy(firstId(request)),
      ),
    ],
    has: (soundId) => current().sounds.some((sound) => sound.id === soundId),
  };
}
