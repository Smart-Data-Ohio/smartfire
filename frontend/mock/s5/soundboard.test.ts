import { Schema } from "effect";
import { describe, expect, it } from "vitest";
import { SoundboardCatalog as SoundboardCatalogSchema } from "../../src/api/schema/soundboard.ts";
import type { DirectUpload } from "../../src/gen/DirectUpload.ts";
import type { HuddleCredentials } from "../../src/gen/HuddleCredentials.ts";
import type { SoundboardCatalog } from "../../src/gen/SoundboardCatalog.ts";
import { field, type Json } from "../json.ts";
import { md5Base64 } from "../s2/digest.ts";
import { collect, errorOf, expectStatus, get, harness, send } from "../s2/testing.ts";
import { type MockServer, SEED_IDS } from "../server.ts";
import { SOUNDBOARD_CAPACITY, tinyWav } from "./soundboard.ts";

const { rooms, users, viewer } = SEED_IDS;

const lounge = rooms.lounge;

/** Uploads `bytes` as the browser's direct upload does; the signed id. */
async function uploaded(server: MockServer, filename: string, type: string, bytes: Uint8Array) {
  const upload = await expectStatus<DirectUpload>(
    server,
    "POST",
    "/api/v1/uploads",
    { filename, byteSize: bytes.length, checksum: md5Base64(bytes), contentType: type },
    200,
  );

  await server.handleBinary({
    method: "PUT",
    path: upload.uploadUrl,
    headers: { "Content-Type": type },
    bytes,
  });

  return upload.signedId;
}

const fieldsOf = (json: Json) => field(field(json, "error"), "fields");

describe("the mock soundboard catalog", () => {
  it("seeds a few sounds whose audio plays from the blob route", async () => {
    const { server } = harness();
    const catalog = await get<SoundboardCatalog>(server, "/api/v1/soundboard/sounds");

    Schema.decodeUnknownSync(SoundboardCatalogSchema)(catalog);
    expect(catalog.capacity).toBe(SOUNDBOARD_CAPACITY);
    expect(catalog.sounds.map((sound) => sound.name)).toEqual(["Airhorn", "Applause", "Rimshot"]);

    const audio = await server.handleBinary({
      method: "GET",
      path: catalog.sounds[0]?.audioUrl ?? "",
      headers: {},
      bytes: new Uint8Array(),
    });

    expect(audio.status).toBe(200);
    expect(audio.contentType).toBe("audio/wav");
  });

  it("adds, edits and deletes a sound, answering the whole catalog each time", async () => {
    const { server } = harness();
    const signedId = await uploaded(server, "boing.wav", "audio/wav", tinyWav(330));

    const added = await expectStatus<SoundboardCatalog>(
      server,
      "POST",
      "/api/v1/admin/soundboard/sounds",
      { name: " Boing ", emoji: "🎈", volume: 0.4, signedId },
      201,
    );

    const boing = added.sounds.find((sound) => sound.name === "Boing");

    expect(boing).toMatchObject({ emoji: "🎈", volume: 0.4, uploaderId: viewer });

    const edited = await expectStatus<SoundboardCatalog>(
      server,
      "PATCH",
      `/api/v1/admin/soundboard/sounds/${boing?.id}`,
      { name: "Bonk", emoji: null, volume: 1 },
      200,
    );

    expect(edited.sounds.map((sound) => sound.name)).toEqual([
      "Airhorn",
      "Applause",
      "Bonk",
      "Rimshot",
    ]);

    const left = await expectStatus<SoundboardCatalog>(
      server,
      "DELETE",
      `/api/v1/admin/soundboard/sounds/${boing?.id}`,
      null,
      200,
    );

    expect(left.sounds).toHaveLength(3);
  });

  it("refuses as the server words it: name, emoji, volume, file and the limit", async () => {
    const { server } = harness();
    const signedId = await uploaded(server, "boing.wav", "audio/wav", tinyWav(330));
    const image = await uploaded(server, "boing.png", "image/png", tinyWav(330));

    const refused = await send(server, "POST", "/api/v1/admin/soundboard/sounds", {
      name: "airhorn",
      emoji: "two words",
      volume: 2,
      signedId,
    });

    expect(refused.status).toBe(422);
    expect(fieldsOf(refused.json)).toEqual({
      name: ["has already been taken"],
      emoji: ["must be a single Unicode emoji"],
      volume: ["must be between 0.0 and 1.0"],
    });

    const notAudio = await send(server, "POST", "/api/v1/admin/soundboard/sounds", {
      name: "Boing",
      emoji: null,
      volume: null,
      signedId: image,
    });

    expect(fieldsOf(notAudio.json)).toEqual({
      audio: ["must be a readable MP3, Ogg, WAV or audio-only WebM file"],
    });

    const noFile = await send(server, "POST", "/api/v1/admin/soundboard/sounds", {
      name: "Boing",
      emoji: null,
      volume: null,
      signedId: "",
    });

    expect(fieldsOf(noFile.json)).toEqual({ audio: ["isn't an uploaded file"] });
  });

  it("stops at the limit", async () => {
    const { server } = harness();
    const signedId = await uploaded(server, "boing.wav", "audio/wav", tinyWav(330));

    for (let index = 3; index < SOUNDBOARD_CAPACITY; index += 1) {
      await expectStatus(
        server,
        "POST",
        "/api/v1/admin/soundboard/sounds",
        { name: `Sound ${index}`, emoji: null, volume: null, signedId },
        201,
      );
    }

    const full = await send(server, "POST", "/api/v1/admin/soundboard/sounds", {
      name: "One more",
      emoji: null,
      volume: null,
      signedId,
    });

    expect(fieldsOf(full.json)).toEqual({
      base: [`soundboard capacity reached (limit: ${SOUNDBOARD_CAPACITY} sounds)`],
    });
  });
});

describe("mock soundboard plays", () => {
  async function joined(server: MockServer) {
    return expectStatus<HuddleCredentials>(
      server,
      "POST",
      `/api/v1/rooms/${lounge}/huddle`,
      {},
      201,
    );
  }

  it("relays a play to the viewer's call, once a second per tab", async () => {
    const { server, clock } = harness();
    const events = collect(server);
    const { identity } = await joined(server);
    const path = `/api/v1/rooms/${lounge}/huddle/soundboard`;

    await expectStatus(server, "POST", path, { identity, soundId: 1 }, 204);

    const plays = () => events.filter((event) => event.type === "huddle.soundboard");

    expect(plays().map((event) => event.data)).toEqual([
      {
        roomId: lounge,
        soundId: 1,
        userId: viewer,
        identity,
        playedAt: new Date(clock.now()).toISOString(),
      },
    ]);

    const again = await send(server, "POST", path, { identity, soundId: 1 });

    expect(again.status).toBe(429);
    expect(errorOf(again.json).tag).toBe("RateLimited");

    clock.advance(1_000);
    await expectStatus(server, "POST", path, { identity, soundId: 2 }, 204);
    expect(plays()).toHaveLength(2);
  });

  it("refuses a tab outside the call, and a sound not in the catalog", async () => {
    const { server } = harness();
    const path = `/api/v1/rooms/${lounge}/huddle/soundboard`;

    const outside = await send(server, "POST", path, { identity: "nobody", soundId: 1 });

    expect(outside.status).toBe(403);
    expect(errorOf(outside.json).message).toBe("Join the call to play sounds");

    const { identity } = await joined(server);
    const unknown = await send(server, "POST", path, { identity, soundId: 99 });

    expect(unknown.status).toBe(422);
    expect(fieldsOf(unknown.json)).toEqual({ soundId: ["isn't in the soundboard"] });
  });

  it("refuses a play from someone who can't speak", async () => {
    const { server } = harness();
    const { identity } = await joined(server);

    await server.handle({
      method: "POST",
      path: "/__mock/huddle-mute",
      body: { roomId: lounge, userId: viewer, muted: true },
    });

    const muted = await send(server, "POST", `/api/v1/rooms/${lounge}/huddle/soundboard`, {
      identity,
      soundId: 1,
    });

    expect(muted.status).toBe(403);
    expect(errorOf(muted.json).message).toBe("You can't play sounds while you can't speak");
  });

  it("lets a test play a sound as someone else in the call", async () => {
    const { server } = harness();
    const events = collect(server);

    await joined(server);
    await server.handle({
      method: "POST",
      path: "/__mock/soundboard-play",
      body: { roomId: lounge, userId: users.maya, soundId: 2 },
    });

    expect(events.find((event) => event.type === "huddle.soundboard")?.data).toMatchObject({
      soundId: 2,
      userId: users.maya,
      identity: `campfire-participant-maya-${lounge}`,
    });
  });
});
