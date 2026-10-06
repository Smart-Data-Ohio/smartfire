/**
 * The call over the conversation: a tile per person in the call (their camera, or their avatar
 * in a speaking ring) and a tile per screen share. A share expands into theater mode (and from
 * there to full screen); a stage stream expands itself for viewers and offers its quality. Only
 * the three loudest speakers get voice-glow (the dock's microphone is the fourth); the rest keep
 * the plain ring.
 */
import { useEffect, useRef } from "react";
import { useStore } from "../../store/store.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { SpeakingRing } from "../../ui/speaking-ring.tsx";
import { UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { callController } from "./call-controller.ts";
import { useCall } from "./call-store.ts";
import { loadParticipantMuted, loadStreamQuality } from "./engine/preferences.ts";
import type { CallParticipant, ViewerQuality } from "./engine/transport.ts";
import { ParticipantMenu } from "./participant-menu.tsx";

const GLOWS = 3;

const QUALITY_LABELS: Readonly<Record<ViewerQuality, string>> = {
  auto: "Auto",
  low: "Data saver",
  high: "Best quality",
};

const VIEWER_QUALITIES: readonly ViewerQuality[] = ["auto", "high", "low"];

/** A LiveKit video track in a <video>, attached for as long as it's shown. */
function VideoTrack({
  videoId,
  label,
  screen,
  expanded,
}: {
  readonly videoId: string;
  readonly label: string;
  readonly screen: boolean;
  readonly expanded: boolean;
}) {
  const ref = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const element = ref.current;

    if (element === null) {
      return;
    }

    callController.attachVideo(videoId, element);

    return () => callController.detachVideo(videoId, element);
  }, [videoId]);

  // A share asks for a layer that matches the size it's shown at (device pixels).
  useEffect(() => {
    const element = ref.current;

    if (!screen || element === null || typeof ResizeObserver === "undefined") {
      return;
    }

    const observer = new ResizeObserver(([entry]) => {
      if (entry === undefined) {
        return;
      }

      const ratio = window.devicePixelRatio || 1;

      callController.screenShown(
        videoId,
        Math.round(entry.contentRect.width * ratio),
        Math.round(entry.contentRect.height * ratio),
        expanded,
      );
    });

    observer.observe(element);

    return () => observer.disconnect();
  }, [videoId, screen, expanded]);

  return (
    <video
      ref={ref}
      className="call-video"
      data-screen={screen || undefined}
      autoPlay
      muted
      playsInline
      aria-label={label}
    />
  );
}

/** The person behind a LiveKit identity (the viewer's own tile is theirs). */
function useParticipantUser(participant: CallParticipant): number | null {
  const viewerId = useStore((state) => state.me?.user.id ?? null);

  useStore((state) => state.huddles);

  return participant.local ? viewerId : callController.userIdFor(participant.identity);
}

function ParticipantTile({
  participant,
  glow,
}: {
  readonly participant: CallParticipant;
  readonly glow: boolean;
}) {
  const userId = useParticipantUser(participant);

  const name = useStore((state) =>
    userId === null ? participant.name : (state.users[userId]?.name ?? participant.name),
  );

  const display = name === "" ? UNKNOWN_NAME : name;
  // Re-read on every snapshot: the controller republishes the snapshot after a local mute.
  const mutedForMe = userId !== null && !participant.local && loadParticipantMuted(userId);

  const tile = (
    <div
      className="call-tile"
      data-speaking={participant.speaking || undefined}
      data-camera={participant.cameraId !== null || undefined}
    >
      {participant.cameraId === null ? (
        <SpeakingRing
          level={participant.audioLevel}
          speaking={participant.speaking}
          glow={glow}
          radius={14}
        >
          {userId === null ? (
            <span className="call-tile-initial">{display.slice(0, 1)}</span>
          ) : (
            <UserAvatar userId={userId} size={56} decorative />
          )}
        </SpeakingRing>
      ) : (
        <VideoTrack
          videoId={participant.cameraId}
          label={`${display}’s camera`}
          screen={false}
          expanded={false}
        />
      )}
      <span className="call-tile-name">
        {participant.microphoneMuted ? <Icon name="mic-off" size={12} /> : null}
        {mutedForMe ? <Icon name="volume-x" size={12} /> : null}
        {participant.quality === "poor" || participant.quality === "lost" ? (
          <Icon name="wifi-off" size={12} />
        ) : null}
        <span className="call-tile-label">
          {display}
          {participant.local ? " (you)" : ""}
        </span>
      </span>
    </div>
  );

  if (userId === null) {
    return tile;
  }

  return (
    <ParticipantMenu
      userId={userId}
      trigger={(props) => (
        <button
          {...props}
          type="button"
          className="call-tile-button"
          aria-label={`${display}${participant.speaking ? ", speaking" : ""}`}
        >
          {tile}
        </button>
      )}
    />
  );
}

function ShareTile({
  participant,
  expanded,
  stream,
}: {
  readonly participant: CallParticipant;
  readonly expanded: boolean;
  readonly stream: boolean;
}) {
  const screenId = participant.screenId;
  const userId = useParticipantUser(participant);

  const name = useStore((state) =>
    userId === null ? participant.name : (state.users[userId]?.name ?? participant.name),
  );

  const watching = useCall((state) => Math.max(0, state.snapshot.participants.length - 1));
  const frame = useRef<HTMLDivElement>(null);

  if (screenId === null) {
    return null;
  }

  const label = participant.local ? "Your screen" : `${name}’s screen`;

  return (
    <figure
      ref={frame}
      className="call-share"
      data-expanded={expanded || undefined}
      data-stream={stream || undefined}
    >
      <VideoTrack videoId={screenId} label={label} screen expanded={expanded} />
      <figcaption className="call-share-bar">
        <span className="call-share-label">
          {stream ? (
            <span className="call-live-badge">
              <Icon name="radio" size={12} /> Live
            </span>
          ) : (
            <Icon name="screen-share" size={14} />
          )}
          {label}
          {stream ? <span className="call-share-watching">{watching} watching</span> : null}
        </span>
        {stream && !participant.local ? (
          <select
            className="huddle-select call-share-quality"
            aria-label="Stream quality"
            defaultValue={loadStreamQuality()}
            onChange={(event) => {
              const quality = VIEWER_QUALITIES.find(
                (candidate) => candidate === event.currentTarget.value,
              );

              if (quality !== undefined) {
                callController.setViewerQuality(quality);
              }
            }}
          >
            {VIEWER_QUALITIES.map((quality) => (
              <option key={quality} value={quality}>
                {QUALITY_LABELS[quality]}
              </option>
            ))}
          </select>
        ) : null}
        {expanded ? (
          <IconButton
            icon="maximize-2"
            label="Full screen"
            size="sm"
            onClick={() => void frame.current?.requestFullscreen?.().catch(() => undefined)}
          />
        ) : null}
        <IconButton
          icon={expanded ? "minimize-2" : "maximize-2"}
          label={expanded ? "Exit theater mode" : "Theater mode"}
          size="sm"
          onClick={() => callController.expand(expanded ? null : screenId)}
        />
      </figcaption>
    </figure>
  );
}

/** The loudest few speakers, by identity. */
function loudest(participants: readonly CallParticipant[]): ReadonlySet<string> {
  return new Set(
    participants
      .filter((participant) => participant.speaking)
      .toSorted((a, b) => b.audioLevel - a.audioLevel)
      .slice(0, GLOWS)
      .map((participant) => participant.identity),
  );
}

/** The call, in its own room's conversation (the dock covers every other room). */
export function CallView({ roomId }: { readonly roomId: number }) {
  const here = useCall((state) => state.roomId === roomId);
  const phase = useCall((state) => state.phase);
  const viewOpen = useCall((state) => state.viewOpen);
  const roomName = useCall((state) => state.roomName);
  const participants = useCall((state) => state.snapshot.participants);
  const expandedVideoId = useCall((state) => state.expandedVideoId);
  // Subscribed for the stream's identity: the presenter's share is the stream.
  useStore((state) => state.stages);
  useCall((state) => state.streaming);

  if (
    !here ||
    !viewOpen ||
    !(phase === "connecting" || phase === "connected" || phase === "reconnecting")
  ) {
    return null;
  }

  const streamVideoId = callController.streamVideoId();
  const shares = participants.filter((participant) => participant.screenId !== null);
  const expanded = shares.find((participant) => participant.screenId === expandedVideoId);
  const glowing = loudest(participants);

  return (
    <section
      className="call-view"
      data-theater={expanded === undefined ? undefined : true}
      aria-label="Call"
    >
      <header className="call-view-head">
        <Icon name="audio-lines" size={16} />
        <h2 className="call-view-title">{roomName}</h2>
        <span className="call-view-count">
          {participants.length} {participants.length === 1 ? "person" : "people"}
        </span>
        <IconButton
          icon="minimize-2"
          label="Hide call"
          size="sm"
          className="call-view-hide"
          onClick={() => callController.setViewOpen(false)}
        />
      </header>
      {phase === "connecting" && participants.length === 0 ? (
        <p className="call-view-connecting">Connecting…</p>
      ) : null}
      {expanded === undefined ? null : (
        <ShareTile participant={expanded} expanded stream={expanded.screenId === streamVideoId} />
      )}
      <div className="call-grid" data-count={Math.min(participants.length + shares.length, 9)}>
        {shares
          .filter((participant) => participant !== expanded)
          .map((participant) => (
            <ShareTile
              key={`share-${participant.identity}`}
              participant={participant}
              expanded={false}
              stream={participant.screenId === streamVideoId}
            />
          ))}
        {participants.map((participant) => (
          <ParticipantTile
            key={participant.identity}
            participant={participant}
            glow={glowing.has(participant.identity)}
          />
        ))}
      </div>
    </section>
  );
}
