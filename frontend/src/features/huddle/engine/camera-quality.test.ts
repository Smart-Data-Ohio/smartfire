import { VideoPreset, VideoPresets } from "livekit-client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  CAMERA_QUALITIES,
  cameraCaptureOptions,
  cameraPublishOptions,
  supportedCameraQualities,
  cameraQualityFromSettings,
  DEFAULT_CAMERA_QUALITY,
  knownCameraQuality,
} from "./camera-quality.ts";
import {
  CAMERA_QUALITY_STORAGE_KEY,
  loadCameraQuality,
  storeCameraQuality,
} from "./preferences.ts";

beforeEach(() => localStorage.clear());

afterEach(() => vi.restoreAllMocks());

describe("camera capture profiles", () => {
  it("defaults to an advisory 720p capture at 30 fps", () => {
    expect(DEFAULT_CAMERA_QUALITY).toBe("720p30");
    expect(cameraCaptureOptions(DEFAULT_CAMERA_QUALITY)).toEqual({
      resolution: { width: 1280, height: 720, frameRate: 30 },
      frameRate: { ideal: 30 },
    });
  });

  it("requests full HD at 30 or 60 fps without a mandatory frame rate", () => {
    expect(cameraCaptureOptions("1080p30")).toEqual({
      resolution: { width: 1920, height: 1080, frameRate: 30 },
      frameRate: { ideal: 30 },
    });
    expect(cameraCaptureOptions("1080p60", "camera-usb")).toEqual({
      resolution: { width: 1920, height: 1080, frameRate: 60 },
      frameRate: { ideal: 60 },
      deviceId: { ideal: "camera-usb" },
    });
    expect(cameraCaptureOptions("720p30", "").deviceId).toBeUndefined();
  });
});

describe("camera encoding profiles", () => {
  it("keeps the pinned SDK's 720p and 1080p camera encodings", () => {
    const hd = cameraPublishOptions("720p30", VideoPreset);
    const fullHd = cameraPublishOptions("1080p30", VideoPreset);

    expect(hd.videoEncoding).toEqual(VideoPresets.h720.encoding);
    expect(fullHd.videoEncoding).toEqual(VideoPresets.h1080.encoding);
    expect(hd.simulcast).toBe(true);
    expect(hd.videoSimulcastLayers).toEqual([VideoPresets.h180, VideoPresets.h360]);
    expect(fullHd.videoSimulcastLayers).toEqual([VideoPresets.h360, VideoPresets.h720]);
  });

  it("encodes full HD at 60 fps and 6 Mbps with lower layers for small views", () => {
    const options = cameraPublishOptions("1080p60", VideoPreset);

    expect(options.videoEncoding).toEqual({ maxBitrate: 6_000_000, maxFramerate: 60 });
    expect(options.simulcast).toBe(true);
    expect(options.degradationPreference).toBe("maintain-framerate");
    expect(options.videoSimulcastLayers).toEqual([VideoPresets.h360, VideoPresets.h720]);
    expect(options.videoSimulcastLayers?.[1]).toBeInstanceOf(VideoPreset);
    expect(options.screenShareEncoding).toBeUndefined();
  });

  it("does not let an SDK mutation change later publish or capture options", () => {
    const first = cameraPublishOptions("1080p60", VideoPreset);
    const firstCapture = cameraCaptureOptions("1080p60");

    if (first.videoEncoding !== undefined) first.videoEncoding.maxBitrate = 1;
    const layer = first.videoSimulcastLayers?.[1];

    if (layer !== undefined) layer.encoding.maxBitrate = 1;

    if (firstCapture.resolution !== undefined) firstCapture.resolution.width = 1;

    expect(cameraPublishOptions("1080p60", VideoPreset).videoEncoding?.maxBitrate).toBe(6_000_000);
    expect(
      cameraPublishOptions("1080p60", VideoPreset).videoSimulcastLayers?.[1]?.encoding.maxBitrate,
    ).toBe(1_700_000);
    expect(cameraCaptureOptions("1080p60").resolution?.width).toBe(1920);
  });
});

describe("camera capability bounds", () => {
  it.each([
    [{ width: 1920, height: 1080, frameRate: 30 }, "1080p30"],
    [{ width: 1280, height: 720, frameRate: 60 }, "720p30"],
    [{ width: 1920, height: 1080, frameRate: 59.94 }, "1080p60"],
    [{}, "720p30"],
  ] as const)("checks width, height and fps together from %j", (settings, quality) => {
    expect(cameraQualityFromSettings("1080p60", settings)).toBe(quality);
  });

  it("keeps 720p as the fallback for unknown and smaller cameras", () => {
    expect(supportedCameraQualities(null)).toEqual(["720p30"]);
    expect(supportedCameraQualities({})).toEqual(["720p30"]);
    expect(
      supportedCameraQualities({
        width: { min: 160, max: 640 },
        height: { min: 90, max: 480 },
        frameRate: { min: 1, max: 15 },
      }),
    ).toEqual(["720p30"]);
  });

  it("allows full HD at 30 fps and adds 60 fps only with a sufficient frame-rate bound", () => {
    const size = { width: { min: 160, max: 1920 }, height: { min: 90, max: 1080 } };

    expect(supportedCameraQualities({ ...size, frameRate: { min: 1, max: 30 } })).toEqual([
      "720p30",
      "1080p30",
    ]);
    expect(supportedCameraQualities({ ...size, frameRate: { min: 1, max: 60 } })).toEqual(
      CAMERA_QUALITIES,
    );
    expect(supportedCameraQualities(size)).toEqual(["720p30"]);
  });

  it("requires both size bounds even when the frame-rate bound is high", () => {
    expect(
      supportedCameraQualities({
        width: { min: 160, max: 1280 },
        height: { min: 90, max: 1080 },
        frameRate: { min: 1, max: 60 },
      }),
    ).toEqual(["720p30"]);
    expect(
      supportedCameraQualities({
        width: { min: 160, max: 1920 },
        height: { min: 90, max: 720 },
        frameRate: { min: 1, max: 60 },
      }),
    ).toEqual(["720p30"]);
  });
});

describe("camera preference", () => {
  it("reads only camera qualities and defaults when absent or corrupt", () => {
    expect(knownCameraQuality("720p30")).toBe("720p30");
    expect(knownCameraQuality("1080p60")).toBe("1080p60");
    expect(knownCameraQuality("1080p15")).toBeNull();
    expect(knownCameraQuality(null)).toBeNull();
    expect(loadCameraQuality()).toBe("720p30");
    localStorage.setItem(CAMERA_QUALITY_STORAGE_KEY, "4k60");
    expect(loadCameraQuality()).toBe("720p30");
  });

  it("remembers a chosen camera quality independently of the screen-share preference", () => {
    localStorage.setItem("campfire.huddle.shareQuality", "1080p15");
    storeCameraQuality("1080p60");

    expect(loadCameraQuality()).toBe("1080p60");
    expect(localStorage.getItem(CAMERA_QUALITY_STORAGE_KEY)).toBe("1080p60");
    expect(localStorage.getItem("campfire.huddle.shareQuality")).toBe("1080p15");
  });

  it("falls back and tolerates refused browser storage", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new DOMException("Storage is blocked", "SecurityError");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new DOMException("Storage is blocked", "SecurityError");
    });

    expect(loadCameraQuality()).toBe("720p30");
    expect(() => storeCameraQuality("1080p30")).not.toThrow();
  });
});
