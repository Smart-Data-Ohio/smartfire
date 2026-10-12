/**
 * The in-call soundboard, Discord's: a button among the call controls opens the workspace's
 * sounds (emoji and name) beside it, or as a bottom sheet on a touch phone. Pressing one plays it
 * to everyone in the call (soundboard-player.ts). The panel also holds this browser's soundboard
 * volume and the switch that turns soundboard sounds off: then the button stays, saying so, and
 * the panel explains it and offers the switch back.
 */
import { useEffect, useId, useState } from "react";
import { useStore } from "../../store/store.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Popover } from "../../ui/popover.tsx";
import { Toggle } from "../../ui/toggle.tsx";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { livePhase, useCall } from "./call-store.ts";
import { playToCall, SENDER_INTERVAL_MS } from "./soundboard-player.ts";
import {
  loadSoundboardCatalog,
  setSoundboardEnabled,
  setSoundboardVolume,
  useSoundboard,
} from "./soundboard-store.ts";

/** What a sound without an emoji shows. */
const NO_EMOJI = "🔊";

/** The volume and the opt-out: in the panel, and in the Calls settings. */
export function SoundboardPreferences() {
  const volume = useSoundboard((state) => state.volume);
  const enabled = useSoundboard((state) => state.enabled);
  const id = useId();

  return (
    <div className="soundboard-preferences">
      {/* The label names the slider itself: wrapped round both, it would name the output. */}
      <div className="huddle-volume">
        <span className="huddle-volume-label">
          <label htmlFor={id}>Soundboard volume</label> <output htmlFor={id}>{volume}%</output>
        </span>
        <input
          id={id}
          type="range"
          min={0}
          max={100}
          step={5}
          value={volume}
          disabled={!enabled}
          onChange={(event) => setSoundboardVolume(Number(event.currentTarget.value))}
        />
      </div>
      <Toggle
        checked={enabled}
        onCheckedChange={setSoundboardEnabled}
        label="Play soundboard sounds"
        description="Off: you won’t hear sounds in calls or be able to play them"
      />
    </div>
  );
}

function SoundGrid() {
  const load = useSoundboard((state) => state.load);
  const enabled = useSoundboard((state) => state.enabled);
  const canPublish = useCall((state) => state.canPublish);
  const live = useCall((state) => livePhase(state.phase));
  const administrator = useStore((state) => state.me?.user.role === "administrator");
  const [cooling, setCooling] = useState(false);

  useEffect(() => {
    void loadSoundboardCatalog().catch(() => undefined);
  }, []);

  useEffect(() => {
    if (!cooling) {
      return;
    }

    const timer = setTimeout(() => setCooling(false), SENDER_INTERVAL_MS);

    return () => clearTimeout(timer);
  }, [cooling]);

  if (load.status === "idle" || load.status === "loading") {
    return <PaneListSkeleton rows={3} square={24} />;
  }

  if (load.status === "error") {
    return (
      <PaneError
        message="Couldn’t load the sounds"
        onRetry={() => void loadSoundboardCatalog(true).catch(() => undefined)}
      />
    );
  }

  const { sounds } = load.catalog;

  if (sounds.length === 0) {
    return (
      <p className="soundboard-note">
        No sounds yet.
        {administrator
          ? " Add some under Workspace, Soundboard."
          : " An administrator can add some."}
      </p>
    );
  }

  const playable = enabled && canPublish && live;

  return (
    <>
      {enabled && !canPublish ? (
        <p className="soundboard-note">You can’t play sounds while you can’t speak here.</p>
      ) : null}
      <ul className="soundboard-grid" aria-label="Sounds" data-cooling={cooling || undefined}>
        {sounds.map((sound) => (
          <li key={sound.id}>
            <button
              type="button"
              className="soundboard-sound"
              disabled={!playable}
              onClick={() => {
                if (playToCall(sound.id)) {
                  setCooling(true);
                }
              }}
            >
              <span className="soundboard-emoji" aria-hidden="true">
                {sound.emoji ?? NO_EMOJI}
              </span>
              <span className="soundboard-name">{sound.name}</span>
            </button>
          </li>
        ))}
      </ul>
    </>
  );
}

/** The panel: the sounds, then the preferences. */
export function SoundboardPanel() {
  const enabled = useSoundboard((state) => state.enabled);

  return (
    <div className="soundboard">
      <h2 className="soundboard-title">Soundboard</h2>
      {enabled ? null : (
        <p className="soundboard-note" role="status">
          Soundboard sounds are off. Turn them on below to hear sounds and play them.
        </p>
      )}
      <SoundGrid />
      <SoundboardPreferences />
    </div>
  );
}

/**
 * The soundboard's button: an icon among the dock's controls, or (`wide`) a labelled button, as
 * the phone's call view shows it.
 */
export function SoundboardButton({ wide = false }: { readonly wide?: boolean }) {
  const enabled = useSoundboard((state) => state.enabled);
  const live = useCall((state) => livePhase(state.phase));
  const label = enabled ? "Soundboard" : "Soundboard (sounds off)";

  return (
    <Popover
      label="Soundboard"
      placement="top"
      trigger={(props) =>
        wide ? (
          <Button
            {...props}
            variant="secondary"
            icon="music"
            className="soundboard-wide"
            data-off={!enabled || undefined}
            disabled={!live}
          >
            {label}
          </Button>
        ) : (
          <IconButton
            {...props}
            icon="music"
            label={label}
            size="sm"
            data-off={!enabled || undefined}
            disabled={!live}
          />
        )
      }
    >
      <SoundboardPanel />
    </Popover>
  );
}
