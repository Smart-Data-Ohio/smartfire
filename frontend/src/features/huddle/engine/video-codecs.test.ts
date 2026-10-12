import * as LiveKit from "livekit-client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { liveKitTransport } from "./livekit.ts";
import { installVideoMedia, offlineLiveKit, videoPublication, videoTrack } from "./sdk-testing.ts";
import { parseVideoCodecError, selectVideoCodec, supportsVideoSimulcast } from "./video-codecs.ts";

function query(codecs: readonly string[]) {
  return {
    width: 1280,
    height: 720,
    bitrate: 1_700_000,
    framerate: 30,
    scalabilityMode: "L1T3",
    advancedAllowed: true,
    supportsAV1: () => true,
    supportsVP9: () => true,
    getCapabilities: () => ({
      headerExtensions: [],
      codecs: codecs.map((codec) => ({ mimeType: `video/${codec}`, clockRate: 90_000 })),
    }),
  } as const;
}

describe("video encoder selection", () => {
  it.each([
    [["AV1", "VP9", "VP8"], "av1"],
    [["VP9", "VP8"], "vp9"],
    [["VP8", "H264"], "vp8"],
  ] as const)("uses the best advertised encoder in %j", async (codecs, codec) => {
    expect(await selectVideoCodec(query(codecs))).toBe(codec);
  });
  it("uses VP9 when encodingInfo rejects AV1 and ignores power efficiency", async () => {
    const encodingInfo = vi.fn(async (configuration: MediaEncodingConfiguration) => ({
      supported: configuration.video?.contentType === "video/VP9",
      smooth: false,
      powerEfficient: false,
    }));

    expect(await selectVideoCodec({ ...query(["AV1", "VP9", "VP8"]), encodingInfo })).toBe("vp9");
    expect(encodingInfo).toHaveBeenCalledTimes(2);
  });
  it("respects SDK browser exclusions and keeps text shares on VP8 without RID support", async () => {
    expect(
      await selectVideoCodec({
        ...query(["AV1", "VP9", "VP8"]),
        supportsAV1: () => false,
        supportsVP9: () => false,
      }),
    ).toBe("vp8");
    expect(
      await selectVideoCodec({ ...query(["AV1", "VP9", "VP8"]), advancedAllowed: false }),
    ).toBe("vp8");
  });
  it("tolerates unavailable capability APIs and rejected encodingInfo", async () => {
    expect(
      await selectVideoCodec({
        ...query([]),
        getCapabilities: () => {
          throw new TypeError("old browser");
        },
      }),
    ).toBe("vp8");
    expect(
      await selectVideoCodec({
        ...query(["AV1", "VP8"]),
        encodingInfo: async () => {
          throw new TypeError("WebRTC query unavailable");
        },
      }),
    ).toBe("av1");
  });
  it("allows RID simulcast only on supported browser and server versions", () => {
    expect(supportsVideoSimulcast({ name: "Chrome", version: "113" }, "1.13.7")).toBe(true);
    expect(supportsVideoSimulcast({ name: "Chrome", version: "112" }, "1.13.7")).toBe(false);
    expect(supportsVideoSimulcast({ name: "Chrome", version: "147" }, "1.13.6")).toBe(false);
    expect(supportsVideoSimulcast({ name: "Chrome", version: "147", os: "iOS" }, "1.13.7")).toBe(
      false,
    );
    expect(supportsVideoSimulcast({ name: "Safari", version: "18" }, "1.13.7")).toBe(false);
    expect(supportsVideoSimulcast({ name: "Chrome", version: "unknown" }, "1.13.7")).toBe(false);
    expect(supportsVideoSimulcast({ name: "Chrome", version: "147" }, undefined)).toBe(false);
  });
  it("does not treat denied permissions or lost connections as codec failures", () => {
    expect(parseVideoCodecError(new DOMException("unsupported encoder", "NotSupportedError"))).toBe(
      "encoder",
    );
    expect(parseVideoCodecError(new DOMException("denied", "NotAllowedError"))).toBe("other");
    expect(
      parseVideoCodecError(Object.assign(new Error("lost"), { name: "ConnectionError" })),
    ).toBe("other");
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("prefers AV1 with a VP8 backup when the camera encoder supports it", async () => {
  installVideoMedia();
  vi.stubGlobal("RTCRtpSender", {
    getCapabilities: () => ({
      codecs: ["AV1", "VP9", "VP8"].map((codec) => ({
        mimeType: `video/${codec}`,
        clockRate: 90_000,
      })),
    }),
  });
  const options: LiveKit.TrackPublishOptions[] = [];
  vi.spyOn(LiveKit.LocalParticipant.prototype, "setCameraEnabled").mockImplementation(
    async (_enabled, _capture, publish) => {
      if (publish !== undefined) options.push(publish);

      return undefined;
    },
  );
  vi.spyOn(LiveKit.LocalParticipant.prototype, "publishTrack").mockImplementation(
    async (track, publish) => {
      if (!(track instanceof LiveKit.LocalTrack)) throw new Error("SDK track required");

      if (publish !== undefined) options.push(publish);

      return videoPublication(track);
    },
  );
  const { lk } = offlineLiveKit();

  const transport = liveKitTransport({
    ...lk,
    createLocalVideoTrack: async () => videoTrack(),
    supportsAV1: () => true,
    supportsVP9: () => true,
    getBrowser: () => ({ name: "Chrome", version: "147" }),
  });

  try {
    await transport.connect("wss://livekit.test", "token", false, 48_000);
    await transport.setCamera(true);
    expect(options.at(-1)).toMatchObject({
      videoCodec: "av1",
      backupCodec: { codec: "vp8" },
      backupCodecPolicy: LiveKit.BackupCodecPolicy.SIMULCAST,
      simulcast: true,
    });
  } finally {
    await transport.disconnect();
  }
});

async function publishingRoom(serverVersion = "1.13.7") {
  installVideoMedia();
  vi.stubGlobal("RTCRtpSender", { getCapabilities: query(["AV1", "VP9", "VP8"]).getCapabilities });
  vi.stubGlobal("navigator", {
    userAgent: navigator.userAgent,
    onLine: true,
    mediaDevices: Object.assign(new EventTarget(), {
      getDisplayMedia: vi.fn(),
      enumerateDevices: async () => [],
    }),
  });
  const capture = vi.fn(async () => videoTrack());

  const screenCapture = vi
    .spyOn(LiveKit.LocalParticipant.prototype, "createScreenTracks")
    .mockImplementation(async () => {
      const track = videoTrack();
      track.source = LiveKit.Track.Source.ScreenShare;

      return [track];
    });

  const publish = vi
    .spyOn(LiveKit.LocalParticipant.prototype, "publishTrack")
    .mockImplementation(async function (this: LiveKit.LocalParticipant, track, options) {
      if (!(track instanceof LiveKit.LocalTrack)) throw new Error("SDK track required");
      const publication = videoPublication(track);

      if (options !== undefined) publication.options = options;
      this.trackPublications.set(publication.trackSid, publication);
      this.videoTrackPublications.set(publication.trackSid, publication);

      return publication;
    });

  const { lk, rooms } = offlineLiveKit();

  const transport = liveKitTransport({
    ...lk,
    createLocalVideoTrack: capture,
    supportsAV1: () => true,
    supportsVP9: () => true,
    getBrowser: () => ({ name: "Chrome", version: "147" }),
  });

  await transport.connect("wss://livekit.test", "token", false, 48_000);
  const room = rooms[0];

  if (room === undefined) throw new Error("SDK room missing");
  vi.spyOn(room.engine, "serverVersion", "get").mockReturnValue(serverVersion);

  const reconnect = vi.spyOn(room.engine, "reconnect").mockImplementation(() => {
    queueMicrotask(() => room.engine.emit("restarted"));
  });

  return { transport, room, publish, capture, screenCapture, reconnect };
}

describe("publishing with codec compatibility", () => {
  it("applies the selected 1080p60 camera encoding to the VP8 compatibility sender", async () => {
    const { transport, capture, publish } = await publishingRoom();
    capture.mockImplementation(async () => {
      const track = videoTrack();
      vi.spyOn(track.mediaStreamTrack, "getSettings").mockReturnValue({
        width: 1920,
        height: 1080,
        frameRate: 60,
      });

      return track;
    });

    try {
      await transport.setCameraQuality("1080p60");
      await transport.setCamera(true);
      expect(publish.mock.lastCall?.[1]?.backupCodec).toEqual({
        codec: "vp8",
        encoding: { maxBitrate: 6_000_000, maxFramerate: 60 },
      });
    } finally {
      await transport.disconnect();
    }
  });

  it("preserves an active stage screen through reconnect unpublish and republish", async () => {
    const { transport, room, publish, reconnect } = await publishingRoom();
    const ended = vi.fn();

    const unsubscribe = transport.subscribe((event) => {
      if (event.type === "screen-ended") ended();
    });

    try {
      await transport.setScreenShare(true, "1080p15");

      const previousPublication = room.localParticipant.getTrackPublication(
        LiveKit.Track.Source.ScreenShare,
      );

      const screen = previousPublication?.videoTrack;

      if (previousPublication === undefined || screen === undefined)
        throw new Error("screen missing");
      const backup = screen.mediaStreamTrack.clone();
      screen.simulcastCodecs.set("vp8", { codec: "vp8", mediaStreamTrack: backup });
      const sequence: string[] = [];
      reconnect.mockImplementation(() => {
        void (async () => {
          sequence.push("reconnecting");
          room.emit(LiveKit.RoomEvent.Reconnecting);
          await room.localParticipant.unpublishTrack(screen, false);
          room.emit(
            LiveKit.RoomEvent.LocalTrackUnpublished,
            previousPublication,
            room.localParticipant,
          );
          sequence.push("screen-unpublished");
          const publication = await room.localParticipant.publishTrack(screen);
          room.emit(LiveKit.RoomEvent.LocalTrackPublished, publication, room.localParticipant);
          sequence.push("screen-republished");
          room.engine.emit("restarted");
          room.emit(LiveKit.RoomEvent.Reconnected);
          sequence.push("reconnected");
        })();
      });
      publish.mockRejectedValueOnce(new DOMException("encoder unavailable", "NotSupportedError"));
      await transport.setCamera(true);
      expect(sequence).toEqual([
        "reconnecting",
        "screen-unpublished",
        "screen-republished",
        "reconnected",
      ]);
      expect(transport.snapshot().screenSharing).toBe(true);
      expect(backup.readyState).toBe("ended");
      expect(ended).not.toHaveBeenCalled();

      const publishedScreen = room.localParticipant.getTrackPublication(
        LiveKit.Track.Source.ScreenShare,
      );

      await transport.setScreenShare(false);

      if (publishedScreen === undefined) throw new Error("screen missing after reconnect");
      room.emit(LiveKit.RoomEvent.LocalTrackUnpublished, publishedScreen, room.localParticipant);
      expect(ended).toHaveBeenCalledTimes(1);
    } finally {
      unsubscribe();
      await transport.disconnect();
    }
  });

  it.each([false, true])(
    "forwards a genuine screen end during codec recovery, native ended=%s",
    async (nativeEnded) => {
      const { transport, room, publish, reconnect } = await publishingRoom();
      const ended = vi.fn();

      const unsubscribe = transport.subscribe((event) => {
        if (event.type === "screen-ended") ended();
      });

      try {
        await transport.setScreenShare(true, "1080p15");

        const publication = room.localParticipant.getTrackPublication(
          LiveKit.Track.Source.ScreenShare,
        );

        const screen = publication?.videoTrack;

        if (publication === undefined || screen === undefined) throw new Error("screen missing");
        reconnect.mockImplementation(() => {
          void (async () => {
            room.emit(LiveKit.RoomEvent.Reconnecting);
            await room.localParticipant.unpublishTrack(screen, nativeEnded);
            room.emit(LiveKit.RoomEvent.LocalTrackUnpublished, publication, room.localParticipant);
            room.engine.emit("restarted");
            room.emit(LiveKit.RoomEvent.Reconnected);
          })();
        });
        publish.mockRejectedValueOnce(new DOMException("encoder unavailable", "NotSupportedError"));
        await transport.setCamera(true);
        expect(ended).toHaveBeenCalledTimes(1);
        expect(screen.mediaStreamTrack.readyState).toBe("ended");
        expect(transport.snapshot().screenSharing).toBe(false);
      } finally {
        unsubscribe();
        await transport.disconnect();
      }
    },
  );

  it("retries a rejected primary encoder as VP8 using the same camera capture", async () => {
    const { transport, room, publish, capture, reconnect } = await publishingRoom();
    const rejected = vi.fn();
    publish.mockImplementationOnce(async (track) => {
      rejected(track);
      throw new DOMException("encoder unavailable", "NotSupportedError");
    });

    try {
      await transport.setCamera(true);
      expect(capture).toHaveBeenCalledTimes(1);
      expect(reconnect).toHaveBeenCalledTimes(1);
      expect(publish.mock.calls.map(([, options]) => options?.videoCodec)).toEqual(["av1", "vp8"]);
      expect(publish.mock.lastCall?.[1]?.backupCodec).toBe(false);
      expect(transport.snapshot().cameraEnabled).toBe(true);
      const failed = rejected.mock.lastCall?.[0];

      const published = room.localParticipant.getTrackPublication(
        LiveKit.Track.Source.Camera,
      )?.track;

      expect(published?.mediaStreamTrack.id).not.toBe(failed?.mediaStreamTrack.id);
      expect(published?.mediaStreamTrack.readyState).toBe("live");
      expect(failed?.mediaStreamTrack.readyState).toBe("ended");
    } finally {
      await transport.disconnect();
    }
  });
  it.each(["1.13.7", "1.13.6"])("keeps text sharp with connected server %s", async (version) => {
    const { transport, publish, screenCapture } = await publishingRoom(version);

    try {
      await transport.setScreenShare(true, "1080p15");
      const options = publish.mock.lastCall?.[1];
      const track = publish.mock.lastCall?.[0];
      expect(options).toMatchObject({
        videoCodec: version === "1.13.7" ? "av1" : "vp8",
        degradationPreference: "maintain-resolution",
      });
      expect(options?.scalabilityMode).toBe(version === "1.13.7" ? "L1T3" : undefined);
      expect(track instanceof LiveKit.LocalVideoTrack && track.mediaStreamTrack.contentHint).toBe(
        "text",
      );
      expect(screenCapture).toHaveBeenCalledTimes(1);
    } finally {
      await transport.disconnect();
    }
  });
  it("does not reopen screen capture when a primary codec fails", async () => {
    const { transport, publish, screenCapture } = await publishingRoom();
    publish.mockRejectedValueOnce(new DOMException("encoder unavailable", "NotSupportedError"));

    try {
      await transport.setScreenShare(true, "1080p15");
      expect(screenCapture).toHaveBeenCalledTimes(1);
      expect(publish.mock.calls.map(([, options]) => options?.videoCodec)).toEqual(["av1", "vp8"]);
      const track = publish.mock.lastCall?.[0];
      expect(track instanceof LiveKit.LocalVideoTrack && track.mediaStreamTrack.contentHint).toBe(
        "text",
      );
    } finally {
      await transport.disconnect();
    }
  });
  it("does not retry a permissions failure as another codec", async () => {
    const { transport, publish, capture } = await publishingRoom();
    publish.mockRejectedValueOnce(new DOMException("denied", "NotAllowedError"));

    try {
      await expect(transport.setCamera(true)).rejects.toMatchObject({ name: "NotAllowedError" });
      expect(publish).toHaveBeenCalledTimes(1);
      expect(capture).toHaveBeenCalledTimes(1);
    } finally {
      await transport.disconnect();
    }
  });
  it("waits for server-session cleanup before retrying and cancels after leaving", async () => {
    const { transport, room, publish, reconnect } = await publishingRoom();
    const closingListeners = room.engine.listenerCount("closing");
    const begun = Promise.withResolvers<void>();
    reconnect.mockImplementation(() => begun.resolve());
    publish.mockRejectedValueOnce(new DOMException("encoder unavailable", "NotSupportedError"));
    const publishing = transport.setCamera(true);
    const failure = expect(publishing).rejects.toThrow("call-ended");
    await begun.promise;
    expect(publish).toHaveBeenCalledTimes(1);
    await transport.disconnect();
    room.engine.emit("restarted");
    await failure;
    expect(publish).toHaveBeenCalledTimes(1);
    expect(room.engine.listenerCount("closing")).toBe(closingListeners);
  });
  it("stops a late VP8 publication after the caller leaves during retry", async () => {
    const { transport, publish } = await publishingRoom();
    const begun = Promise.withResolvers<LiveKit.LocalVideoTrack>();
    const completed = Promise.withResolvers<void>();
    publish.mockRejectedValueOnce(new DOMException("encoder unavailable", "NotSupportedError"));
    publish.mockImplementationOnce(async (track) => {
      if (!(track instanceof LiveKit.LocalVideoTrack)) throw new Error("SDK video missing");
      begun.resolve(track);
      await completed.promise;

      return videoPublication(track);
    });
    const publishing = transport.setCamera(true);
    const failure = expect(publishing).rejects.toThrow("call-ended");
    const retry = await begun.promise;
    await transport.disconnect();
    expect(retry.mediaStreamTrack.readyState).toBe("ended");
    completed.resolve();
    await failure;
    expect(retry.mediaStreamTrack.readyState).toBe("ended");
    expect(publish).toHaveBeenCalledTimes(2);
  });
});
