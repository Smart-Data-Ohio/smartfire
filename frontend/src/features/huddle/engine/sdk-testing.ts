import * as LiveKit from "livekit-client";
import { vi } from "vitest";

export class VideoMediaStub extends EventTarget {
  readonly id = crypto.randomUUID();
  readonly kind: "video" | "audio";
  constructor(kind: "video" | "audio" = "video") {
    super();
    this.kind = kind;
  }
  enabled = true;
  contentHint = "";
  readyState: MediaStreamTrackState = "live";
  getSettings() {
    return { width: 1280, height: 720, frameRate: 30 };
  }
  getConstraints() {
    return {};
  }
  async applyConstraints() {}
  getCapabilities() {
    return { width: { max: 1920 }, height: { max: 1080 }, frameRate: { max: 60 } };
  }
  clone() {
    return new VideoMediaStub(this.kind);
  }
  stop() {
    this.readyState = "ended";
  }
}

export function installVideoMedia(): void {
  class MediaStreamStub extends EventTarget {
    readonly tracks: MediaStreamTrack[];
    constructor(tracks: MediaStreamTrack[] = []) {
      super();
      this.tracks = tracks;
    }
    getTracks() {
      return this.tracks;
    }
    getVideoTracks() {
      return this.tracks.filter((track) => track.kind === "video");
    }
    getAudioTracks() {
      return this.tracks.filter((track) => track.kind === "audio");
    }
  }

  vi.stubGlobal("MediaStream", MediaStreamStub);
  vi.stubGlobal("MediaStreamTrack", VideoMediaStub);
}

export function videoTrack(): LiveKit.LocalVideoTrack {
  const track: unknown = Reflect.construct(LiveKit.LocalVideoTrack, [
    new VideoMediaStub(),
    undefined,
    false,
  ]);

  if (!(track instanceof LiveKit.LocalVideoTrack)) throw new Error("SDK video track missing");
  track.source = LiveKit.Track.Source.Camera;

  return track;
}

export function videoPublication(track: LiveKit.LocalTrack): LiveKit.LocalTrackPublication {
  const publication: unknown = Reflect.construct(LiveKit.LocalTrackPublication, [
    track.kind,
    {
      sid: track.mediaStreamTrack.id,
      name: "video",
      source: LiveKit.Track.sourceToProto(track.source),
      muted: track.isMuted,
      width: 1280,
      height: 720,
    },
    track,
  ]);

  if (!(publication instanceof LiveKit.LocalTrackPublication))
    throw new Error("SDK publication missing");

  return publication;
}

export function screenAudioTrack(): LiveKit.LocalTrack {
  const track: unknown = Reflect.construct(LiveKit.LocalAudioTrack, [
    new VideoMediaStub("audio"),
    undefined,
    false,
  ]);

  if (!(track instanceof LiveKit.LocalTrack) || track.kind !== LiveKit.Track.Kind.Audio)
    throw new Error("SDK audio track missing");
  track.source = LiveKit.Track.Source.ScreenShareAudio;

  return track;
}

export function offlineLiveKit() {
  const rooms: LiveKit.Room[] = [];

  class OfflineRoom extends LiveKit.Room {
    constructor(options?: Partial<LiveKit.RoomOptions>) {
      super(options);
      this.connect = () => Promise.resolve();
      this.disconnect = () => Promise.resolve();
      rooms.push(this);
    }
  }

  return { lk: { ...LiveKit, Room: OfflineRoom }, rooms };
}

export function videoSender(track: MediaStreamTrack | null): RTCRtpSender {
  return {
    track,
    dtmf: null,
    transport: null,
    transform: null,
    getStats: vi.fn(async () => new Map()),
    getParameters: () => ({
      transactionId: "video",
      encodings: [{}],
      codecs: [],
      headerExtensions: [],
      rtcp: {},
    }),
    replaceTrack: vi.fn(async () => undefined),
    setParameters: vi.fn(async () => undefined),
    setStreams: vi.fn(),
  };
}
