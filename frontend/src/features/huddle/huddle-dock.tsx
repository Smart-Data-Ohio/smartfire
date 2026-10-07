/**
 * The huddle dock, Discord's "Voice connected" panel: pinned above your own panel at the bottom
 * of the sidebar while in a call (on phones, a bar over the conversation). Status and timer,
 * the room, the microphone (its level glowing round it), deafen, camera, screen share, the call
 * view, settings (devices, noise suppression, connection details) and leave. A failed call keeps
 * the dock up with the reason and Retry until it's closed.
 */
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { Beam } from "../../ui/beam.tsx";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { SpeakingRing } from "../../ui/speaking-ring.tsx";
import { Toggle } from "../../ui/toggle.tsx";
import { callController } from "./call-controller.ts";
import { type CallPhase, livePhase, useCall } from "./call-store.ts";
import { DevicePickers, MeterBar } from "./device-pickers.tsx";
import { formatConnectionStats } from "./engine/stats.ts";
import type { ConnectionQuality } from "./engine/transport.ts";
import "./huddle.css";

const BEAM_MS = 3_000;

const QUALITY_TEXT: Readonly<Record<ConnectionQuality, string>> = {
  excellent: "Excellent connection",
  good: "Good connection",
  poor: "Poor connection",
  lost: "Connection lost",
  unknown: "Connection",
};

const PHASE_TONE: Readonly<Record<CallPhase, string>> = {
  idle: "idle",
  prejoin: "pending",
  connecting: "pending",
  connected: "live",
  reconnecting: "warning",
  failed: "danger",
};

/** mm:ss, then h:mm:ss. */
export function formatElapsed(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = String(total % 60).padStart(2, "0");

  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}`
    : `${minutes}:${seconds}`;
}

/** How long the viewer has been in this call, ticking each second while connected. */
function useElapsed(live: boolean): string | null {
  const startedAt = useCall((state) => state.startedAt);
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (!live) {
      return;
    }

    setNow(Date.now());

    const timer = setInterval(() => setNow(Date.now()), 1000);

    return () => clearInterval(timer);
  }, [live]);

  return live && startedAt !== null ? formatElapsed(now - startedAt) : null;
}

/** A border beam for the first three seconds of a call. */
function useJoinBeam(phase: CallPhase, roomId: number | null): boolean {
  const [beaming, setBeaming] = useState(false);

  useEffect(() => {
    if (phase !== "connected" || roomId === null) {
      return;
    }

    setBeaming(true);

    const timer = setTimeout(() => setBeaming(false), BEAM_MS);

    return () => clearTimeout(timer);
  }, [phase, roomId]);

  return beaming;
}

function SettingsPanel() {
  const devices = useCall((state) => state.devices);
  const selected = useCall((state) => state.selectedDevices);
  const noise = useCall((state) => state.noise);
  const statsOpen = useCall((state) => state.statsOpen);
  const stats = useCall((state) => state.stats);
  const meter = useCall((state) => state.meter);
  const formatted = stats === null ? null : formatConnectionStats(stats);

  return (
    <section className="huddle-dock-settings" aria-label="Call settings">
      <DevicePickers
        devices={devices}
        selected={selected}
        onSelect={(kind, deviceId) => void callController.selectDevice(kind, deviceId)}
        cameraOptional={false}
      />
      <MeterBar level={meter} label="Microphone level" />
      {noise.available ? (
        <Toggle
          checked={noise.enabled}
          disabled={noise.busy}
          onCheckedChange={() => void callController.toggleNoise()}
          label="Noise suppression"
          description="Filters background noise from your microphone"
        />
      ) : null}
      <Button
        variant="ghost"
        size="sm"
        icon="signal"
        aria-expanded={statsOpen}
        onClick={() => callController.toggleStats()}
      >
        {statsOpen ? "Hide connection details" : "Connection details"}
      </Button>
      {statsOpen ? (
        <dl className="huddle-stats">
          {formatted === null ? (
            <div>
              <dt>Measuring…</dt>
            </div>
          ) : (
            <>
              <div>
                <dt>Round trip</dt>
                <dd>{formatted.rtt}</dd>
              </div>
              <div>
                <dt>Packet loss</dt>
                <dd>{formatted.loss}</dd>
              </div>
              <div>
                <dt>Jitter</dt>
                <dd>{formatted.jitter}</dd>
              </div>
              <div>
                <dt>Receiving</dt>
                <dd>{formatted.received}</dd>
              </div>
              <div>
                <dt>Sending</dt>
                <dd>{formatted.sent}</dd>
              </div>
              <div>
                <dt>Route</dt>
                <dd>{formatted.transport}</dd>
              </div>
            </>
          )}
        </dl>
      ) : null}
    </section>
  );
}

function FailedDock() {
  const title = useCall((state) => state.failureTitle);
  const notice = useCall((state) => state.notice);

  return (
    <div className="huddle-dock-failed" role="alert">
      <span className="huddle-dock-title">
        <Icon name="alert" size={14} />
        {title}
      </span>
      {notice === null ? null : <p className="huddle-dock-notice">{notice}</p>}
      <div className="huddle-dock-actions">
        <Button variant="secondary" size="sm" onClick={() => callController.retry()}>
          Retry
        </Button>
        <Button variant="ghost" size="sm" onClick={() => void callController.leave()}>
          Close
        </Button>
      </div>
    </div>
  );
}

interface HuddleDockProps {
  /** The phone bar over the conversation rather than the sidebar panel. */
  readonly compact?: boolean;
}

export function HuddleDock({ compact = false }: HuddleDockProps) {
  const phase = useCall((state) => state.phase);
  const roomId = useCall((state) => state.roomId);
  const roomName = useCall((state) => state.roomName);
  const status = useCall((state) => state.status);
  const notice = useCall((state) => state.notice);
  const quality = useCall((state) => state.quality);
  const reconnectSeconds = useCall((state) => state.reconnectSeconds);
  const canPublish = useCall((state) => state.canPublish);
  const deafened = useCall((state) => state.deafened);
  const busy = useCall((state) => state.busy);
  const microphone = useCall((state) => state.snapshot.microphoneEnabled);
  const camera = useCall((state) => state.snapshot.cameraEnabled);
  const screen = useCall((state) => state.snapshot.screenSharing);
  const playback = useCall((state) => state.snapshot.canPlaybackAudio);
  const meter = useCall((state) => state.meter);
  const devicesOpen = useCall((state) => state.devicesOpen);
  const viewOpen = useCall((state) => state.viewOpen);
  const navigate = useNavigate();
  const viewedRoomId = useParams({ strict: false }).roomId ?? null;
  const streaming = useCall((state) => state.streaming !== null);
  const live = livePhase(phase);
  const elapsed = useElapsed(phase === "connected" || phase === "reconnecting");
  const beaming = useJoinBeam(phase, roomId);

  if (phase === "idle" || phase === "prejoin" || roomId === null) {
    return null;
  }

  const connected = phase === "connected";
  const inRoom = viewedRoomId === roomId;
  const shown = inRoom && viewOpen;

  const statusLine =
    phase === "reconnecting" && reconnectSeconds !== null
      ? `Reconnecting… ${reconnectSeconds > 0 ? `${reconnectSeconds}s` : ""}`.trim()
      : status;

  return (
    <Beam active={beaming} radius={compact ? 0 : 10}>
      <section
        className="huddle-dock"
        data-compact={compact || undefined}
        data-tone={PHASE_TONE[phase]}
        aria-label="Call"
      >
        {phase === "failed" ? (
          <FailedDock />
        ) : (
          <>
            {devicesOpen && !compact ? <SettingsPanel /> : null}
            <div className="huddle-dock-head">
              <span
                className="huddle-dock-signal"
                data-quality={quality}
                title={QUALITY_TEXT[quality]}
              >
                <Icon name={phase === "reconnecting" ? "wifi-off" : "signal"} size={14} />
              </span>
              <span className="huddle-dock-text">
                <span className="huddle-dock-status" role="status">
                  {statusLine}
                  {elapsed === null ? null : (
                    <span className="huddle-dock-elapsed"> · {elapsed}</span>
                  )}
                </span>
                <Link
                  to="/r/$roomId"
                  params={{ roomId }}
                  className="huddle-dock-room"
                  preload={false}
                >
                  {roomName}
                </Link>
              </span>
              {phase === "reconnecting" ? (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => void callController.reconnectNow()}
                >
                  Reconnect
                </Button>
              ) : null}
              <IconButton
                icon="phone-off"
                label="Leave call"
                size="sm"
                className="huddle-dock-leave"
                onClick={() => void callController.leave()}
              />
            </div>
            {notice === null ? null : <p className="huddle-dock-notice">{notice}</p>}
            {playback ? null : (
              <Button
                variant="secondary"
                size="sm"
                icon="volume"
                className="huddle-dock-resume"
                onClick={() => void callController.resumeAudio()}
              >
                Click to hear the call
              </Button>
            )}
            {streaming ? (
              <p className="huddle-dock-live">
                <Icon name="radio" size={12} /> You’re live
              </p>
            ) : null}
            <div className="huddle-dock-controls">
              <SpeakingRing level={meter / 100} speaking={microphone && meter > 8} radius={8}>
                <IconButton
                  icon={microphone ? "mic" : "mic-off"}
                  label={
                    canPublish
                      ? microphone
                        ? "Mute microphone"
                        : "Unmute microphone"
                      : "You can’t speak here"
                  }
                  shortcut={["Ctrl", "Shift", "M"]}
                  size="sm"
                  aria-pressed={!microphone}
                  data-off={!microphone || undefined}
                  disabled={!connected || !canPublish || busy.microphone}
                  onClick={() => void callController.toggleMute()}
                />
              </SpeakingRing>
              <IconButton
                icon={deafened ? "headphone-off" : "headphones"}
                label={deafened ? "Undeafen" : "Deafen"}
                size="sm"
                aria-pressed={deafened}
                data-off={deafened || undefined}
                disabled={!live}
                onClick={() => void callController.toggleDeafen()}
              />
              <IconButton
                icon={camera ? "video" : "video-off"}
                label={camera ? "Turn camera off" : "Turn camera on"}
                size="sm"
                aria-pressed={camera}
                data-on={camera || undefined}
                disabled={!connected || !canPublish || busy.camera}
                onClick={() => void callController.toggleCamera()}
              />
              <IconButton
                icon={screen ? "screen-share-off" : "screen-share"}
                label={screen ? "Stop sharing" : "Share your screen"}
                size="sm"
                aria-pressed={screen}
                data-on={screen || undefined}
                disabled={!connected || !canPublish || busy.screen || streaming}
                onClick={() => void callController.toggleScreenShare()}
              />
              <IconButton
                icon={shown ? "minimize-2" : "maximize-2"}
                label={shown ? "Hide call" : "Show call"}
                size="sm"
                aria-pressed={shown}
                onClick={() => {
                  // The call shows in its own room: from anywhere else, go there.
                  if (!inRoom) {
                    callController.setViewOpen(true);
                    void navigate({ to: "/r/$roomId", params: { roomId } });

                    return;
                  }

                  callController.setViewOpen(!viewOpen);
                }}
              />
              {compact ? null : (
                <IconButton
                  icon="settings"
                  label="Call settings"
                  size="sm"
                  aria-expanded={devicesOpen}
                  disabled={!live}
                  onClick={() => callController.toggleDevices()}
                />
              )}
            </div>
          </>
        )}
      </section>
    </Beam>
  );
}
