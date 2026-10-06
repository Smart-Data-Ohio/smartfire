import { describe, expect, it } from "vitest";
import { streamVideoIdOf } from "./call-store.ts";
import type { CallParticipant } from "./engine/transport.ts";

function participant(identity: string, local: boolean, screenId: string | null): CallParticipant {
  return {
    identity,
    name: identity,
    local,
    speaking: false,
    audioLevel: 0,
    microphoneMuted: false,
    quality: "good",
    cameraId: null,
    screenId,
  };
}

const ME = participant("me", true, "screen-me");

const MAYA = participant("maya", false, "screen-maya");

const JONAH = participant("jonah", false, null);

describe("streamVideoIdOf", () => {
  it("is this tab's own share while it streams", () => {
    expect(streamVideoIdOf([ME, MAYA], true, "maya")).toBe("screen-me");
  });

  it("is the live presenter's share otherwise", () => {
    expect(streamVideoIdOf([ME, MAYA], false, "maya")).toBe("screen-maya");
  });

  it("is nothing without a presenter, or while the presenter isn't sharing or here", () => {
    expect(streamVideoIdOf([ME, MAYA], false, null)).toBeNull();
    expect(streamVideoIdOf([ME, JONAH], false, "jonah")).toBeNull();
    expect(streamVideoIdOf([ME], false, "maya")).toBeNull();
  });
});
