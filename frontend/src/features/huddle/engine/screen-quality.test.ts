import { ScreenSharePresets, VideoPreset } from "livekit-client";
import { describe, expect, it } from "vitest";
import type { StreamQuality } from "../../../gen/StreamQuality.ts";
import {
  applyScreenContentHint,
  DEFAULT_SHARE_QUALITY,
  knownStreamQuality,
  STREAM_QUALITIES,
  screenCaptureAttempts,
  screenPublishOptions,
} from "./screen-quality.ts";

describe("screen capture", () => {
  it("requests each profile's capture size and frame rate, with constraint fallback", () => {
    const profiles = [
      { quality: "720p15", width: 1280, height: 720, frameRate: 15 },
      { quality: "1080p15", width: 1920, height: 1080, frameRate: 15 },
      { quality: "1080p30", width: 1920, height: 1080, frameRate: 30 },
    ] as const;

    for (const { quality, ...resolution } of profiles) {
      const attempts = screenCaptureAttempts(quality);
      const text = { contentHint: "text", surfaceSwitching: "include", systemAudio: "exclude" };

      expect(attempts).toEqual([
        { ...text, audio: true, resolution },
        { ...text, audio: false, resolution },
        { ...text, audio: false, resolution: { width: 0, height: 0 } },
      ]);
    }
  });

  it("asks for 1920×1080 at 60 fps for 1080p60, then gives up the constraints", () => {
    const resolution = { width: 1920, height: 1080, frameRate: 60 };
    const motion = { contentHint: "motion", surfaceSwitching: "include", systemAudio: "exclude" };

    expect(screenCaptureAttempts("1080p60")).toEqual([
      { ...motion, audio: true, resolution },
      { ...motion, audio: false, resolution },
      { ...motion, audio: false, resolution: { width: 0, height: 0 } },
    ]);
  });

  it("keeps Safari 17 and iOS 17+ unconstrained for every profile", () => {
    const browsers = [
      { name: "Safari", version: "17.6", os: "macOS" },
      { name: "Chrome", version: "130", os: "iOS", osVersion: "18.0" },
    ] as const;

    for (const browser of browsers) {
      for (const quality of STREAM_QUALITIES) {
        expect(
          screenCaptureAttempts(quality, browser).map((attempt) => attempt.resolution),
        ).toEqual([undefined, undefined]);
      }
    }

    expect(
      screenCaptureAttempts("1080p15", { name: "Safari", version: "18.4" })[0]?.resolution,
    ).toEqual({ width: 1920, height: 1080, frameRate: 15 });
  });

  it("uses text where accepted, detail when ignored or rejected, and motion for 60 fps", () => {
    const track = { contentHint: "detail" };
    applyScreenContentHint(track, "1080p15");
    expect(track.contentHint).toBe("text");
    applyScreenContentHint(track, "1080p60");
    expect(track.contentHint).toBe("motion");

    for (const rejects of [false, true]) {
      let hint = "";

      const unsupported = {
        get contentHint() {
          return hint;
        },
        set contentHint(value: string) {
          if (value === "text") {
            if (rejects) throw new TypeError("unsupported hint");
          } else {
            hint = value;
          }
        },
      };

      applyScreenContentHint(unsupported, "720p15");
      expect(unsupported.contentHint).toBe("detail");
    }
  });
});

describe("screen encoding", () => {
  it("keeps the SDK's screen-share presets for the classic three", () => {
    const presets: Record<Exclude<StreamQuality, "1080p60">, VideoPreset> = {
      "720p15": ScreenSharePresets.h720fps15,
      "1080p15": ScreenSharePresets.h1080fps15,
      "1080p30": ScreenSharePresets.h1080fps30,
    };

    for (const [quality, preset] of Object.entries(presets)) {
      const options = screenPublishOptions(knownStreamQuality(quality) ?? "720p15", VideoPreset);

      expect(options).toEqual({
        dtx: false,
        screenShareEncoding: preset.encoding,
        degradationPreference: "maintain-resolution",
      });
    }
  });

  it("encodes 1080p60 at 60 fps and 8 Mbps over a 720p/30 simulcast layer, keeping frames", () => {
    const options = screenPublishOptions("1080p60", VideoPreset);

    expect(options.dtx).toBe(false);
    expect(options.screenShareEncoding).toEqual({
      maxBitrate: 8_000_000,
      maxFramerate: 60,
      priority: "medium",
    });
    expect(options.degradationPreference).toBe("maintain-framerate");

    const layers = options.screenShareSimulcastLayers ?? [];

    expect(layers).toHaveLength(1);
    expect(layers[0]).toBeInstanceOf(VideoPreset);
    expect(layers[0]?.width).toBe(1280);
    expect(layers[0]?.height).toBe(720);
    expect(layers[0]?.encoding).toEqual(ScreenSharePresets.h720fps30.encoding);
  });

  it("hands out a fresh encoding each time, so the SDK can't edit the table", () => {
    const first = screenPublishOptions("1080p60", VideoPreset).screenShareEncoding;

    expect(first).not.toBe(screenPublishOptions("1080p60", VideoPreset).screenShareEncoding);
  });
});

describe("qualities", () => {
  it("lists 1080p60 last and keeps 1080p15 as the share default", () => {
    expect(STREAM_QUALITIES).toEqual(["720p15", "1080p15", "1080p30", "1080p60"]);
    expect(DEFAULT_SHARE_QUALITY).toBe("1080p15");
  });

  it("reads only the qualities it knows", () => {
    expect(knownStreamQuality("1080p60")).toBe("1080p60");
    expect(knownStreamQuality("4k60")).toBeNull();
    expect(knownStreamQuality(null)).toBeNull();
  });
});
