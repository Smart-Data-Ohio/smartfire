import type { getBrowser, ScalabilityMode } from "livekit-client";

export type PreferredVideoCodec = "av1" | "vp9" | "vp8";

interface EncoderQuery {
  readonly width: number;
  readonly height: number;
  readonly bitrate: number;
  readonly framerate: number;
  readonly scalabilityMode: ScalabilityMode;
  readonly advancedAllowed: boolean;
  readonly supportsAV1: () => boolean;
  readonly supportsVP9: () => boolean;
  readonly getCapabilities?: () => RTCRtpCapabilities | null;
  readonly encodingInfo?: (
    configuration: MediaEncodingConfiguration,
  ) => Promise<MediaCapabilitiesInfo>;
}

/** The SDK requires Chromium RID semantics and a connected server newer than 1.13.6. */
export function supportsVideoSimulcast(
  browser: ReturnType<typeof getBrowser>,
  serverVersion: string | undefined,
): boolean {
  if (
    browser?.name !== "Chrome" ||
    browser.os === "iOS" ||
    !(Number(browser.version.split(".")[0]) >= 113)
  ) {
    return false;
  }

  const [major = 0, minor = 0, patch = 0] = serverVersion?.split(".").map(Number) ?? [];

  return major > 1 || (major === 1 && (minor > 13 || (minor === 13 && patch > 6)));
}

/** Probe at publication time. Failed/missing APIs cannot make codec selection fail a call. */
export async function selectVideoCodec(query: EncoderQuery): Promise<PreferredVideoCodec> {
  if (!query.advancedAllowed) return "vp8";
  let capabilities: RTCRtpCapabilities | null;

  try {
    capabilities = query.getCapabilities?.() ?? RTCRtpSender.getCapabilities("video");
  } catch {
    return "vp8";
  }

  const advertised = new Set(capabilities?.codecs.map((codec) => codec.mimeType.toLowerCase()));

  for (const codec of ["av1", "vp9"] as const) {
    try {
      const allowed = codec === "av1" ? query.supportsAV1() : query.supportsVP9();

      if (!allowed || !advertised.has(`video/${codec}`)) continue;

      const probe =
        query.encodingInfo ??
        navigator.mediaCapabilities?.encodingInfo?.bind(navigator.mediaCapabilities);

      if (probe !== undefined) {
        let timeout: ReturnType<typeof setTimeout> | undefined;

        try {
          const info = await Promise.race([
            probe({
              type: "webrtc",
              video: {
                contentType: `video/${codec.toUpperCase()}`,
                width: query.width,
                height: query.height,
                bitrate: query.bitrate,
                framerate: query.framerate,
                scalabilityMode: query.scalabilityMode,
              },
            }),
            new Promise<null>((resolve) => {
              timeout = setTimeout(() => resolve(null), 750);
            }),
          ]);

          if (info?.supported === false) continue;
        } catch {
          // WebRTC encodingInfo is absent or incomplete in some browsers; sender capabilities remain authoritative.
        } finally {
          clearTimeout(timeout);
        }
      }

      return codec;
    } catch {
      // The SDK support predicate itself can fail on an older browser.
    }
  }

  return "vp8";
}

/** Retry encoder/negotiation failures, never permission denials or an ended connection. */
export function parseVideoCodecError(cause: unknown): "encoder" | "other" {
  const retry =
    (cause instanceof Error || cause instanceof DOMException) &&
    ([
      "NotSupportedError",
      "OperationError",
      "InvalidModificationError",
      "NegotiationError",
    ].includes(cause.name) ||
      (cause.name === "PublishTrackError" && /codec|encoder|encoding/i.test(cause.message)));

  return retry ? "encoder" : "other";
}
