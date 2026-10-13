import * as LiveKit from "livekit-client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { liveKitTransport } from "./livekit.ts";
import {
  installVideoMedia,
  offlineLiveKit,
  videoPublication,
  videoSender,
  videoTrack,
} from "./sdk-testing.ts";

const sdkUnpublish = LiveKit.LocalParticipant.prototype.unpublishTrack;

beforeEach(() => {
  localStorage.clear();
  installVideoMedia();
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

async function connectedCamera() {
  const { lk, rooms } = offlineLiveKit();

  const capture = vi.fn(async (options?: LiveKit.VideoCaptureOptions) => {
    const track = videoTrack();
    vi.spyOn(track.mediaStreamTrack, "getSettings").mockReturnValue({
      width: options?.resolution?.width ?? 1280,
      height: options?.resolution?.height ?? 720,
      frameRate: options?.resolution?.frameRate ?? 30,
    });

    return track;
  });

  // Publishing crosses the network. The room, participant, track, and publication remain real SDK objects.
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

  const unpublish = vi.spyOn(LiveKit.LocalParticipant.prototype, "unpublishTrack");

  const transport = liveKitTransport({
    ...lk,
    createLocalVideoTrack: capture,
    supportsAV1: () => false,
    supportsVP9: () => false,
  });

  await transport.connect("wss://livekit.test", "token", false, 48_000);
  const room = rooms[0];

  if (room === undefined) throw new Error("SDK room required");

  return { transport, room, capture, publish, unpublish };
}

function cameraPublication(room: LiveKit.Room) {
  const publication = room.localParticipant.getTrackPublication(LiveKit.Track.Source.Camera);

  if (publication?.videoTrack === undefined) throw new Error("Published SDK camera required");

  return { publication, track: publication.videoTrack };
}

function microphonePublication(room: LiveKit.Room) {
  class AudioMediaStub extends EventTarget {
    readonly id = "microphone";
    readonly kind = "audio";
    enabled = true;
    readyState: MediaStreamTrackState = "live";
    getSettings() {
      return { sampleRate: 48_000, channelCount: 1 };
    }
    getConstraints() {
      return {};
    }
    stop() {
      this.readyState = "ended";
    }
  }

  const media = new AudioMediaStub();
  const track: unknown = Reflect.construct(LiveKit.LocalAudioTrack, [media]);

  if (!(track instanceof LiveKit.LocalAudioTrack)) throw new Error("SDK microphone required");
  track.source = LiveKit.Track.Source.Microphone;

  const publication: unknown = Reflect.construct(LiveKit.LocalTrackPublication, [
    LiveKit.Track.Kind.Audio,
    {
      sid: media.id,
      name: "microphone",
      source: LiveKit.Track.sourceToProto(LiveKit.Track.Source.Microphone),
      muted: false,
    },
    track,
  ]);

  if (!(publication instanceof LiveKit.LocalTrackPublication))
    throw new Error("SDK microphone publication required");
  room.localParticipant.trackPublications.set(publication.trackSid, publication);
  room.localParticipant.audioTrackPublications.set(publication.trackSid, publication);

  return { media, track, publication };
}

describe("camera capture and publication", () => {
  it.each([false, true])(
    "stops backup captures before replacement unpublish, including failure=%s",
    async (failed) => {
      const { transport, room, unpublish } = await connectedCamera();

      try {
        await transport.setCamera(true);
        const { track } = cameraPublication(room);
        const backup = track.mediaStreamTrack.clone();
        const sender = videoSender(track.mediaStreamTrack);
        track.sender = sender;
        track.simulcastCodecs.set("vp8", {
          codec: "vp8",
          mediaStreamTrack: backup,
          sender: videoSender(backup),
        });
        Object.assign(room.engine, {
          pcManager: {
            currentState: 0,
            publisher: { getTransceivers: () => [{ sender, direction: "sendonly" }] },
            removeTrack: vi.fn(),
          },
        });
        vi.spyOn(room.engine, "negotiate").mockResolvedValue(undefined);
        const stateAtUnpublish: MediaStreamTrackState[] = [];
        const error = new DOMException("negotiation rejected", "OperationError");
        unpublish.mockImplementationOnce(async function (
          this: LiveKit.LocalParticipant,
          target,
          stop,
        ) {
          stateAtUnpublish.push(backup.readyState);
          const result = await sdkUnpublish.call(this, target, stop);

          if (failed) throw error;

          return result;
        });

        if (failed) await expect(transport.setCameraQuality("1080p60")).rejects.toBe(error);
        else await transport.setCameraQuality("1080p60");
        expect(stateAtUnpublish).toEqual(["ended"]);
        expect(track.simulcastCodecs.size).toBe(0);
        expect(backup.readyState).toBe("ended");
        await transport.setCamera(false);
        expect(backup.readyState).toBe("ended");
      } finally {
        await transport.disconnect();
      }
    },
  );

  it("falls back from independent 1080p and 60fps bounds when the combined capture delivers 30fps", async () => {
    const { transport, room, capture, publish } = await connectedCamera();

    try {
      await transport.setCamera(true);
      capture.mockImplementation(async () => {
        const track = videoTrack();
        vi.spyOn(track.mediaStreamTrack, "getSettings").mockReturnValue({
          width: 1920,
          height: 1080,
          frameRate: 30,
        });

        return track;
      });
      await transport.setCameraQuality("1080p60");
      expect(publish.mock.lastCall?.[1]?.videoEncoding).toEqual({
        maxBitrate: 3_000_000,
        maxFramerate: 30,
      });
      expect(localStorage.getItem("campfire.huddle.cameraQuality")).toBe("1080p30");
      expect(room.options.videoCaptureDefaults).toMatchObject({
        resolution: { width: 1920, height: 1080, frameRate: 30 },
      });
    } finally {
      await transport.disconnect();
    }
  });

  it("starts the default camera with 720p30 capture and the pinned 1.7 Mbps encoding", async () => {
    const { transport, capture, publish } = await connectedCamera();

    try {
      await transport.setCamera(true);
      expect(capture).toHaveBeenCalledExactlyOnceWith({
        resolution: { width: 1280, height: 720, frameRate: 30 },
        frameRate: { ideal: 30 },
      });
      expect(publish.mock.lastCall?.[1]).toMatchObject({
        videoEncoding: { maxBitrate: 1_700_000, maxFramerate: 30 },
        videoCodec: "vp8",
        degradationPreference: "maintain-framerate",
      });
      expect(transport.snapshot().cameraEnabled).toBe(true);
    } finally {
      await transport.disconnect();
    }
  });

  it("replaces an active camera with the selected capture and encoding while releasing the old track", async () => {
    const { transport, room, capture, publish, unpublish } = await connectedCamera();

    try {
      await transport.setCamera(true);
      const { track } = cameraPublication(room);
      const restart = vi.spyOn(track, "restartTrack");
      await transport.setCameraQuality("1080p60");
      expect(capture.mock.lastCall?.[0]).toEqual({
        resolution: { width: 1920, height: 1080, frameRate: 60 },
        frameRate: { ideal: 60 },
      });
      expect(capture).toHaveBeenCalledTimes(2);
      expect(restart).not.toHaveBeenCalled();
      expect(unpublish).toHaveBeenCalledExactlyOnceWith(track, false);
      expect(cameraPublication(room).track).not.toBe(track);
      expect(publish.mock.lastCall?.[1]).toMatchObject({
        videoEncoding: { maxBitrate: 6_000_000, maxFramerate: 60 },
        videoCodec: "vp8",
      });
      expect(room.options.videoCaptureDefaults).toEqual(capture.mock.lastCall?.[0]);
      expect(transport.snapshot().cameraEnabled).toBe(true);
      expect(track.mediaStreamTrack.readyState).toBe("ended");
      expect(cameraPublication(room).track.mediaStreamTrack.readyState).toBe("live");
    } finally {
      await transport.disconnect();
    }
  });

  it("keeps muted camera capture stopped, retains its new constraints on unmute, and leaves the microphone untouched", async () => {
    const { transport, room, capture } = await connectedCamera();

    try {
      await transport.setCamera(true);
      const { track } = cameraPublication(room);
      const microphone = microphonePublication(room);
      const microphoneStop = vi.spyOn(microphone.media, "stop");
      const microphoneRestart = vi.spyOn(microphone.track, "restartTrack");
      const audioDefaults = room.options.audioCaptureDefaults;
      await track.mute();
      const replacement = videoTrack().mediaStreamTrack;
      vi.spyOn(replacement, "getSettings").mockReturnValue({
        width: 1920,
        height: 1080,
        frameRate: 60,
      });
      const afterUnmute = videoTrack().mediaStreamTrack;
      vi.spyOn(replacement, "getConstraints").mockReturnValue({
        width: 1920,
        height: 1080,
        frameRate: { ideal: 60 },
      });
      const apply = vi.spyOn(afterUnmute, "applyConstraints");

      const getUserMedia = vi
        .fn()
        .mockResolvedValueOnce(new MediaStream([replacement]))
        .mockResolvedValueOnce(new MediaStream([afterUnmute]));

      vi.stubGlobal("navigator", {
        userAgent: navigator.userAgent,
        mediaDevices: Object.assign(new EventTarget(), {
          getUserMedia,
          enumerateDevices: async () => [],
        }),
      });
      capture.mockImplementationOnce((options) => LiveKit.createLocalVideoTrack(options));
      await transport.setCameraQuality("1080p60");
      expect(cameraPublication(room).track.isMuted).toBe(true);
      expect(replacement.readyState).toBe("ended");
      expect(transport.snapshot().cameraEnabled).toBe(false);
      expect(transport.snapshot().microphoneEnabled).toBe(true);
      expect(room.localParticipant.getTrackPublication(LiveKit.Track.Source.Microphone)).toBe(
        microphone.publication,
      );
      expect(microphoneStop).not.toHaveBeenCalled();
      expect(microphoneRestart).not.toHaveBeenCalled();
      expect(room.options.audioCaptureDefaults).toBe(audioDefaults);
      await transport.setCamera(true);
      expect(getUserMedia).toHaveBeenCalledTimes(2);
      expect(cameraPublication(room).track.mediaStreamTrack.readyState).toBe("live");
      expect(apply).toHaveBeenCalledWith(
        expect.objectContaining({ width: 1920, height: 1080, frameRate: { ideal: 60 } }),
      );
    } finally {
      await transport.disconnect();
    }
  });

  it("remembers a quality while the camera is off and uses it for the next capture", async () => {
    const { transport, capture, publish } = await connectedCamera();

    try {
      await transport.setCameraQuality("1080p60");
      expect(capture).not.toHaveBeenCalled();
      expect(publish).not.toHaveBeenCalled();
      await transport.setCamera(true);
      expect(capture).toHaveBeenCalledExactlyOnceWith({
        resolution: { width: 1920, height: 1080, frameRate: 60 },
        frameRate: { ideal: 60 },
      });
      expect(publish.mock.lastCall?.[1]?.videoEncoding).toEqual({
        maxBitrate: 6_000_000,
        maxFramerate: 60,
      });
    } finally {
      await transport.disconnect();
    }
  });

  it("retains the original live publication and defaults when replacement capture is denied", async () => {
    const { transport, room, capture, publish, unpublish } = await connectedCamera();

    try {
      await transport.setCamera(true);
      const { publication, track } = cameraPublication(room);
      const defaults = room.options.videoCaptureDefaults;
      const denied = new DOMException("Camera permission denied", "NotAllowedError");
      capture.mockRejectedValueOnce(denied);
      await expect(transport.setCameraQuality("1080p60")).rejects.toBe(denied);
      expect(cameraPublication(room).publication).toBe(publication);
      expect(room.options.videoCaptureDefaults).toEqual(defaults);
      expect(track.mediaStreamTrack.readyState).toBe("live");
      expect(publish).toHaveBeenCalledTimes(1);
      expect(unpublish).not.toHaveBeenCalled();
    } finally {
      await transport.disconnect();
    }
  });

  it("uses the SDK's direct constrained capture instead of its leaky restart applyConstraints path", async () => {
    const { transport, room, capture } = await connectedCamera();

    try {
      await transport.setCamera(true);
      const previous = cameraPublication(room).track;
      const replacement = videoTrack().mediaStreamTrack;
      vi.spyOn(replacement, "getSettings").mockReturnValue({
        width: 1920,
        height: 1080,
        frameRate: 60,
      });

      const apply = vi
        .spyOn(replacement, "applyConstraints")
        .mockRejectedValue(new DOMException("unsupported mode", "OverconstrainedError"));

      const getUserMedia = vi.fn().mockResolvedValue(new MediaStream([replacement]));
      vi.stubGlobal("navigator", {
        userAgent: navigator.userAgent,
        mediaDevices: Object.assign(new EventTarget(), {
          getUserMedia,
          enumerateDevices: async () => [],
        }),
      });
      capture.mockImplementationOnce((options) => LiveKit.createLocalVideoTrack(options));
      await transport.setCameraQuality("1080p60");
      expect(getUserMedia).toHaveBeenCalledExactlyOnceWith(
        expect.objectContaining({
          video: expect.objectContaining({ width: 1920, height: 1080, frameRate: { ideal: 60 } }),
        }),
      );
      expect(apply).not.toHaveBeenCalled();
      expect(previous.mediaStreamTrack.readyState).toBe("ended");
      expect(cameraPublication(room).track.mediaStreamTrack).toBe(replacement);
      await transport.disconnect();
      expect(replacement.readyState).toBe("ended");
    } finally {
      await transport.disconnect();
    }
  });

  it("republishes prior quality after unpublish removed its maps before rejecting", async () => {
    const { transport, room, publish, unpublish, capture } = await connectedCamera();

    try {
      await transport.setCamera(true);
      const { track } = cameraPublication(room);
      const replacement = videoTrack();
      vi.spyOn(replacement.mediaStreamTrack, "getSettings").mockReturnValue({
        width: 1920,
        height: 1080,
        frameRate: 60,
      });
      capture.mockResolvedValueOnce(replacement);
      const failed = new DOMException("negotiation rejected", "OperationError");
      unpublish.mockImplementationOnce(async function (
        this: LiveKit.LocalParticipant,
        target,
        stop,
      ) {
        await sdkUnpublish.call(this, target, stop);
        throw failed;
      });
      await expect(transport.setCameraQuality("1080p60")).rejects.toBe(failed);
      expect(cameraPublication(room).track).toBe(track);
      expect(track.mediaStreamTrack.readyState).toBe("live");
      expect(replacement.mediaStreamTrack.readyState).toBe("ended");
      expect(publish).toHaveBeenCalledTimes(2);
      expect(publish.mock.lastCall?.[1]?.videoEncoding).toEqual({
        maxBitrate: 1_700_000,
        maxFramerate: 30,
      });
    } finally {
      await transport.disconnect();
    }
  });

  it("stops all owned camera captures when restoring the prior publication also fails", async () => {
    const { transport, room, capture, publish } = await connectedCamera();

    try {
      await transport.setCamera(true);
      const { track } = cameraPublication(room);
      const replacement = videoTrack();
      vi.spyOn(replacement.mediaStreamTrack, "getSettings").mockReturnValue({
        width: 1920,
        height: 1080,
        frameRate: 60,
      });
      capture.mockResolvedValueOnce(replacement);
      const failed = new DOMException("VP8 publication failed", "OperationError");
      publish.mockRejectedValueOnce(failed).mockRejectedValueOnce(failed);
      await expect(transport.setCameraQuality("1080p60")).rejects.toBe(failed);
      expect(track.mediaStreamTrack.readyState).toBe("ended");
      expect(replacement.mediaStreamTrack.readyState).toBe("ended");
      expect(transport.snapshot().cameraEnabled).toBe(false);
    } finally {
      await transport.disconnect();
    }
  });

  it("stops a pending replacement capture after leaving without publishing it", async () => {
    const { transport, room, capture, publish, unpublish } = await connectedCamera();
    const pending = Promise.withResolvers<LiveKit.LocalVideoTrack>();
    await transport.setCamera(true);
    const { track } = cameraPublication(room);
    capture.mockImplementationOnce(() => pending.promise);
    const changing = transport.setCameraQuality("1080p60");
    await transport.disconnect();
    const replacement = videoTrack();
    vi.spyOn(replacement.mediaStreamTrack, "getSettings").mockReturnValue({
      width: 1920,
      height: 1080,
      frameRate: 60,
    });
    pending.resolve(replacement);
    await changing;
    expect(publish).toHaveBeenCalledTimes(1);
    expect(unpublish).not.toHaveBeenCalled();
    expect(track.mediaStreamTrack.readyState).toBe("ended");
    expect(replacement.mediaStreamTrack.readyState).toBe("ended");
  });

  it("immediately stops both owned captures when leaving after unpublish with publication pending", async () => {
    const { transport, room, capture, publish } = await connectedCamera();
    await transport.setCamera(true);
    const previous = cameraPublication(room).track;
    const replacement = videoTrack();
    vi.spyOn(replacement.mediaStreamTrack, "getSettings").mockReturnValue({
      width: 1920,
      height: 1080,
      frameRate: 60,
    });
    capture.mockResolvedValueOnce(replacement);
    const begun = Promise.withResolvers<void>();
    const completed = Promise.withResolvers<void>();
    publish.mockImplementationOnce(async () => {
      begun.resolve();
      await completed.promise;

      return videoPublication(replacement);
    });
    const changing = transport.setCameraQuality("1080p60");
    await begun.promise;
    expect(room.localParticipant.getTrackPublication(LiveKit.Track.Source.Camera)).toBeUndefined();
    await transport.disconnect();
    expect(previous.mediaStreamTrack.readyState).toBe("ended");
    expect(replacement.mediaStreamTrack.readyState).toBe("ended");
    completed.resolve();
    await changing;
    expect(transport.snapshot().cameraEnabled).toBe(false);
  });
});
