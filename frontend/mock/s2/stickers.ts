/** The sticker catalog and message references, kept per mock world. */
import type { MessageSticker } from "../../src/gen/MessageSticker.ts";
import type { Sticker } from "../../src/gen/Sticker.ts";
import type { StickerCatalog } from "../../src/gen/StickerCatalog.ts";
import {
  forbidden,
  type MockBinaryRequest,
  type MockBinaryResponse,
  type MockResponse,
  notFound,
  stickerLimitReached,
  validation,
} from "../http.ts";
import { type Json, stringField } from "../json.ts";
import { timestamp, VIEWER_ID, type World } from "../seed.ts";
import { firstId, type Route, route, type S2Context } from "./context.ts";
import { topicOf, touched } from "./model.ts";
import { readProfileImage } from "./profile-image.ts";

export const STICKER_CAPACITY = 60;

const MAX_BYTES = 512 * 1024;

const decode = (encoded: string) =>
  Uint8Array.from(atob(encoded), (character) => character.charCodeAt(0));

// The same tiny generated fixtures as test-support/attachments/stickers.
export const STICKER_PNG = decode(
  "iVBORw0KGgoAAAANSUhEUgAAAAIAAAADCAYAAAC56t6BAAAAEElEQVR4nGNgYGD4D8VoDABH0wX7kHb2kgAAAABJRU5ErkJggg==",
);

export const STICKER_GIF = decode(
  "R0lGODlhAgADAIAAAAAAAP///yH5BAAKAAAALAAAAAACAAMAAAIFBEEQBFEAIfkEAAoAAAAsAAAAAAIAAwAAAgUMwzAMUwA7",
);

export const STICKER_APNG = decode(
  "iVBORw0KGgoAAAANSUhEUgAAAAIAAAADCAYAAAC56t6BAAAACGFjVEwAAAACAAAAAPONk3AAAAAaZmNUTAAAAAAAAAACAAAAAwAAAAAAAAAAAAEACgAAUa8H6AAAABBJREFUeJxjYGBg+A/FaAwAR9MF+5B29pIAAAAaZmNUTAAAAAEAAAACAAAAAwAAAAAAAAAAAAEACgAAytztPAAAABJmZEFUAAAAAnicY/gPBQwYDABC5hfpEiiS0gAAAABJRU5ErkJggg==",
);

interface Entry {
  sticker: Sticker;
  readonly bytes: Uint8Array;
  readonly contentType: string;
}

interface State {
  entries: Entry[];
  nextId: number;
}

export interface Stickers {
  readonly routes: readonly Route[];
  readonly reference: (id: number) => MessageSticker;
  readonly handleBinary: (request: MockBinaryRequest) => MockBinaryResponse | null;
}

/** Reads animation control at PNG chunk boundaries, including the end marker. */
function pngFrames(bytes: Uint8Array): number | null {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const text = new TextDecoder();
  let frames = 1;
  let controls = 0;
  let offset = 8;

  while (offset + 12 <= bytes.length) {
    const size = view.getUint32(offset);
    const kind = text.decode(bytes.subarray(offset + 4, offset + 8));

    if (offset + size + 12 > bytes.length) return null;

    if (kind === "acTL") {
      if (size !== 8) return null;

      frames = view.getUint32(offset + 8);
    }

    if (kind === "fcTL") controls += 1;

    if (kind === "IEND") return controls === 0 || controls === frames ? frames : null;

    offset += size + 12;
  }

  return null;
}

export function createStickers(ctx: S2Context): Stickers {
  const worlds = new WeakMap<World, State>();

  const entry = (
    id: number,
    name: string,
    bytes: Uint8Array,
    animated: boolean,
    contentType: string,
  ): Entry => ({
    sticker: {
      id,
      name,
      description: "Greeting",
      tags: "👋 hello",
      imageUrl: `/stickers/${id}`,
      stillUrl: animated ? `/stickers/${id}?still=1` : `/stickers/${id}`,
      animated,
      uploaderId: VIEWER_ID,
      createdAt: timestamp(ctx.now()),
      updatedAt: timestamp(ctx.now()),
    },
    bytes,
    contentType,
  });

  const current = (): State => {
    const world = ctx.world();
    let state = worlds.get(world);

    if (state === undefined) {
      state = {
        entries: [
          entry(1, "Wave", STICKER_PNG, false, "image/png"),
          entry(2, "Dance", STICKER_GIF, true, "image/gif"),
          entry(3, "Hello", STICKER_APNG, true, "image/png"),
        ],
        nextId: 4,
      };
      worlds.set(world, state);
    }

    return state;
  };

  const reference = (id: number): MessageSticker => {
    const sticker = current().entries.find((entry) => entry.sticker.id === id)?.sticker;

    if (sticker === undefined) throw validation("stickerId", "must exist");

    return {
      kind: "available",
      id,
      name: sticker.name,
      imageUrl: sticker.imageUrl,
      stillUrl: sticker.stillUrl,
      animated: sticker.animated,
    };
  };

  const catalog = (status = 200): MockResponse => {
    const json: StickerCatalog = {
      stickers: current()
        .entries.map((entry) => entry.sticker)
        .sort((a, b) => a.name.localeCompare(b.name)),
      capacity: STICKER_CAPACITY,
    };

    return { status, json };
  };

  const administrator = () => {
    if (ctx.world().users.get(VIEWER_ID)?.role !== "administrator") throw forbidden();
  };

  const values = (body: Json | undefined, id: number | null) => {
    const name = (stringField(body, "name") ?? "").trim();

    if ([...name].length < 2 || [...name].length > 30)
      throw validation("name", "must be between 2 and 30 characters");

    if (
      current().entries.some(
        (entry) =>
          entry.sticker.id !== id && entry.sticker.name.toLowerCase() === name.toLowerCase(),
      )
    )
      throw validation("name", "has already been taken");

    return {
      name,
      description: (stringField(body, "description") ?? "").trim(),
      tags: (stringField(body, "tags") ?? "").trim(),
    };
  };

  const refresh = (id: number, replacement: MessageSticker) => {
    const world = ctx.world();

    const lists = [...world.rooms.values()]
      .map((room) => room.messages)
      .concat([...world.threads.values()].map((thread) => thread.messages));

    for (const list of lists) {
      list.forEach((message, index) => {
        if (message.sticker?.kind !== "available" || message.sticker.id !== id) return;

        const next = {
          ...message,
          sticker: replacement,
          updatedAt: touched(ctx.now(), message.updatedAt),
        };

        list[index] = next;

        for (const [key, saved] of world.sentByClientId) {
          if (saved.id === message.id) world.sentByClientId.set(key, next);
        }

        ctx.publish([{ topic: topicOf(next), type: "message.updated", data: next }]);
      });
    }
  };

  const create = (body: Json | undefined): MockResponse => {
    administrator();
    const signed = stringField(body, "signedId") ?? "";
    const blob = ctx.world().blobs.get(signed);

    if (blob === undefined || blob.bytes === null)
      throw validation("image", "isn't an uploaded file");

    if (blob.byteSize > MAX_BYTES) throw validation("image", "must be 512 KB or smaller");

    const info = readProfileImage(blob.bytes);

    if (info === null || (info.format !== "png" && info.format !== "gif"))
      throw validation("image", "must be a PNG, APNG or GIF image");

    if (info.width > 320 || info.height > 320)
      throw validation("image", "must be at most 320 × 320 pixels");

    const frames = info.format === "png" ? pngFrames(blob.bytes) : info.animated ? 2 : 1;

    if (frames === null || frames === 0 || (info.format === "gif" && blob.bytes.at(-1) !== 0x3b))
      throw validation("image", "could not be read");

    if (frames > 100) throw validation("image", "must contain at most 100 frames");

    const fields = values(body, null);
    const state = current();

    if (state.entries.length >= STICKER_CAPACITY) throw stickerLimitReached(STICKER_CAPACITY);

    const saved = entry(
      state.nextId++,
      fields.name,
      blob.bytes,
      frames > 1,
      `image/${info.format}`,
    );

    saved.sticker = { ...saved.sticker, ...fields };
    state.entries.push(saved);

    return catalog(201);
  };

  const update = (id: number, body: Json | undefined): MockResponse => {
    administrator();
    const saved = current().entries.find((entry) => entry.sticker.id === id);

    if (saved === undefined) throw notFound();

    saved.sticker = {
      ...saved.sticker,
      ...values(body, id),
      updatedAt: touched(ctx.now(), saved.sticker.updatedAt),
    };
    refresh(id, reference(id));

    return catalog();
  };

  const destroy = (id: number): MockResponse => {
    administrator();
    const state = current();
    const saved = state.entries.find((entry) => entry.sticker.id === id);

    if (saved === undefined) throw notFound();

    state.entries = state.entries.filter((entry) => entry.sticker.id !== id);
    refresh(id, { kind: "deleted", name: saved.sticker.name });

    return catalog();
  };

  return {
    reference,
    routes: [
      route("GET", /^\/stickers$/, () => catalog()),
      route("GET", /^\/admin\/stickers$/, () => {
        administrator();

        return catalog();
      }),
      route("POST", /^\/admin\/stickers$/, ({ body }) => create(body)),
      route("PATCH", /^\/admin\/stickers\/(\d+)$/, (request) =>
        update(firstId(request), request.body),
      ),
      route("DELETE", /^\/admin\/stickers\/(\d+)$/, (request) => destroy(firstId(request))),
    ],
    handleBinary: (request) => {
      const url = new URL(request.path, "https://campfire.test");
      const match = /^\/stickers\/(\d+)$/.exec(url.pathname);

      if (match === null) return null;

      const saved = current().entries.find((entry) => entry.sticker.id === Number(match[1]));

      if (saved === undefined)
        return { status: 404, contentType: null, bytes: new Uint8Array(), headers: {} };

      const still = saved.sticker.animated && url.searchParams.get("still") === "1";

      return {
        status: 200,
        contentType: still ? "image/png" : saved.contentType,
        bytes: still ? STICKER_PNG : saved.bytes,
        headers: { "Cache-Control": "max-age=3600, private" },
      };
    },
  };
}
