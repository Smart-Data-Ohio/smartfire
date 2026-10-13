import * as LiveKit from "livekit-client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { liveKitTransport } from "./livekit.ts";
import { VIDEO_ADAPTIVE_STREAM } from "./video-quality.ts";

afterEach(() => vi.restoreAllMocks());

class ObservedPublication extends LiveKit.RemoteTrackPublication {
  resize(width: number, height: number): void {
    this.handleVideoDimensionsChange({ width, height });
  }
}

function publication(): ObservedPublication {
  const value: unknown = Reflect.construct(ObservedPublication, [
    LiveKit.Track.Kind.Video,
    {
      sid: "screen",
      name: "screen",
      type: 1,
      source: 3,
      muted: false,
      width: 1920,
      height: 1080,
      layers: [
        { quality: LiveKit.VideoQuality.LOW, width: 960, height: 540 },
        { quality: LiveKit.VideoQuality.HIGH, width: 1920, height: 1080 },
      ],
    },
    true,
  ]);

  if (!(value instanceof ObservedPublication)) throw new Error("SDK publication missing");

  return value;
}

describe("received video quality", () => {
  it.each([1, 2, 3])("requests real display pixels at DPR %i", async (ratio) => {
    vi.useFakeTimers();
    vi.stubGlobal("devicePixelRatio", ratio);

    const track: unknown = Reflect.construct(LiveKit.RemoteVideoTrack, [
      { id: "video", kind: "video", getSettings: () => ({}) },
      "screen",
      {},
      VIDEO_ADAPTIVE_STREAM,
    ]);

    if (!(track instanceof LiveKit.RemoteVideoTrack)) throw new Error("SDK video missing");

    const changed = vi.fn();

    const element = {
      element: {},
      width: () => 640,
      height: () => 360,
      visible: true,
      pictureInPicture: false,
      visibilityChangedAt: undefined,
      observe: vi.fn(),
      stopObserving: vi.fn(),
    };

    track.on(LiveKit.TrackEvent.VideoDimensionsChanged, changed);

    try {
      track.observeElementInfo(element);
      await vi.advanceTimersByTimeAsync(500);
      expect(changed.mock.lastCall?.[0]).toEqual({ width: 640 * ratio, height: 360 * ratio });
    } finally {
      track.stopObservingElementInfo(element);
      vi.unstubAllGlobals();
      vi.useRealTimers();
    }
  });

  it("forces focused HIGH across resizes, then restores the latest adaptive size", async () => {
    const rooms: LiveKit.Room[] = [];

    class OfflineRoom extends LiveKit.Room {
      constructor(options?: Partial<LiveKit.RoomOptions>) {
        super(options);
        this.connect = () => Promise.resolve();
        this.disconnect = () => Promise.resolve();
        rooms.push(this);
      }
    }

    const transport = liveKitTransport({ ...LiveKit, Room: OfflineRoom });
    await transport.connect("wss://livekit.test", "token", false, 48_000);
    const room = rooms[0];

    if (room === undefined) throw new Error("SDK room missing");
    expect(room.options.adaptiveStream).toEqual({ pixelDensity: "screen" });

    const remote = new LiveKit.RemoteParticipant(room.engine.client, "remote", "remote");
    const screen = publication();
    remote.trackPublications.set(screen.trackSid, screen);
    room.remoteParticipants.set(remote.identity, remote);
    const settings = vi.fn();
    screen.on(LiveKit.TrackEvent.UpdateSettings, settings);

    try {
      screen.resize(640, 360);
      transport.showScreen("screen", true);
      expect(settings.mock.lastCall?.[0]).toMatchObject({
        quality: LiveKit.VideoQuality.HIGH,
        width: 0,
        height: 0,
        disabled: false,
      });
      screen.resize(800, 450);
      expect(settings.mock.lastCall?.[0]).toMatchObject({
        quality: LiveKit.VideoQuality.HIGH,
        width: 0,
        height: 0,
      });
      transport.showScreen("screen", false);
      expect(settings.mock.lastCall?.[0]).toMatchObject({ width: 800, height: 450 });

      transport.setViewerQuality("screen", "high");
      expect(settings.mock.lastCall?.[0]).toMatchObject({ width: 0, height: 0 });
      screen.resize(1600, 900);
      transport.setViewerQuality("screen", "low");
      expect(settings.mock.lastCall?.[0]).toMatchObject({
        quality: LiveKit.VideoQuality.LOW,
        width: 0,
        height: 0,
      });
      transport.setViewerQuality("screen", "auto");
      expect(settings.mock.lastCall?.[0]).toMatchObject({ width: 1600, height: 900 });

      const original = screen.emitTrackUpdate;
      transport.showScreen("screen", true);
      room.emit(LiveKit.RoomEvent.TrackUnpublished, screen, remote);
      expect(screen.emitTrackUpdate).toBe(original);
    } finally {
      await transport.disconnect();
    }
  });
});
