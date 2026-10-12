import { type FormEvent, useEffect, useRef, useState } from "react";
import type { SoundboardCatalog } from "../../gen/SoundboardCatalog.ts";
import type { SoundboardSound } from "../../gen/SoundboardSound.ts";
import { soundboard } from "../../sync/huddles.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import {
  loadSoundboardCatalog,
  replaceSoundboardCatalog,
  useSoundboard,
} from "../huddle/soundboard-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import {
  FieldError,
  fieldsOf,
  SettingsGroup,
  SettingsPage,
  useBusy,
} from "../settings/settings-parts.tsx";
import {
  SOUND_FILE_TYPES,
  soundCapacity,
  soundFieldError,
  soundVolumeLabel,
} from "./admin-format.ts";
import {
  AdministratorsOnly,
  adminFailure,
  uploaded,
  useAdmin,
  useRowFocus,
} from "./admin-parts.tsx";

type Fields = Readonly<Record<string, readonly string[]>>;

/** What a sound without an emoji shows. */
const NO_EMOJI = "🔊";

/** The forms' own fields: their messages show under them. */
const FORM_FIELDS = new Set(["name", "emoji", "volume", "audio"]);

/** Messages about anything else (the capacity, say), for the line above the form. */
function elsewhere(fields: Fields): string | undefined {
  const messages = Object.entries(fields)
    .filter(([key]) => !FORM_FIELDS.has(key))
    .flatMap(([key, each]) => each.map((message) => soundFieldError(key, message) ?? message));

  return messages.length === 0 ? undefined : messages.join(" ");
}

/** A blank emoji clears it. */
function emojiOrNull(emoji: string): string | null {
  const trimmed = emoji.trim();

  return trimmed === "" ? null : trimmed;
}

/** Name, emoji and default volume: the fields a new sound and an edit share. */
function SoundFields({
  idPrefix,
  name,
  emoji,
  volume,
  fields,
  disabled = false,
  onName,
  onEmoji,
  onVolume,
}: {
  readonly idPrefix: string;
  readonly name: string;
  readonly emoji: string;
  /** 0–100. */
  readonly volume: number;
  readonly fields: Fields;
  readonly disabled?: boolean;
  readonly onName: (name: string) => void;
  readonly onEmoji: (emoji: string) => void;
  readonly onVolume: (volume: number) => void;
}) {
  const error = (key: string) => soundFieldError(key, fields[key]?.[0]);

  return (
    <>
      <div className="settings-inline admin-sound-names">
        <TextField
          label="Name"
          hint="1–32 characters"
          value={name}
          autoComplete="off"
          placeholder="Airhorn"
          maxLength={64}
          disabled={disabled}
          error={error("name")}
          onChange={(event) => onName(event.target.value)}
        />
        <TextField
          label="Emoji"
          hint="Optional"
          value={emoji}
          autoComplete="off"
          placeholder="📯"
          className="admin-sound-emoji-field"
          disabled={disabled}
          error={error("emoji")}
          onChange={(event) => onEmoji(event.target.value)}
        />
      </div>
      <div className="settings-field">
        <label className="settings-label" htmlFor={`${idPrefix}-volume`}>
          Default volume <output htmlFor={`${idPrefix}-volume`}>{volume}%</output>
        </label>
        <input
          id={`${idPrefix}-volume`}
          className="admin-sound-volume"
          type="range"
          min={0}
          max={100}
          step={5}
          value={volume}
          disabled={disabled}
          onChange={(event) => onVolume(Number(event.currentTarget.value))}
        />
        <FieldError message={error("volume")} />
      </div>
    </>
  );
}

interface NewSoundProps {
  readonly full: boolean;
  readonly onSaved: (catalog: SoundboardCatalog) => void;
  /** A refused upload: the count may have moved (another administrator's upload, say). */
  readonly onRefused: () => void;
}

/** The upload form: name, emoji, default volume and the file, saved together. */
function NewSound({ full, onSaved, onRefused }: NewSoundProps) {
  const [name, setName] = useState("");
  const [emoji, setEmoji] = useState("");
  const [volume, setVolume] = useState(100);
  const [file, setFile] = useState<File | null>(null);
  const [fields, setFields] = useState<Fields>({});
  const input = useRef<HTMLInputElement | null>(null);
  const { busy, track } = useBusy();

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setFields({});

    // With no file chosen the write goes out without one, and the server says so under the field.
    const save = async () => {
      const signedId = file === null ? "" : await uploaded(file);

      return soundboard.create({ name, emoji: emojiOrNull(emoji), volume: volume / 100, signedId });
    };

    void track(
      "create",
      save().then(
        (catalog) => {
          onSaved(catalog);
          setName("");
          setEmoji("");
          setVolume(100);
          setFile(null);

          if (input.current !== null) {
            input.current.value = "";
          }

          toast({ title: "Sound added", tone: "success" });
        },
        (error: Error) => {
          const named = fieldsOf(error);

          if (Object.keys(named).length > 0) {
            setFields(named);
            onRefused();
          } else {
            adminFailure("Couldn't add the sound", error);
          }
        },
      ),
    );
  };

  return (
    <form className="settings-form" onSubmit={submit} aria-label="Add a sound">
      {full ? (
        <p className="settings-callout text-muted" role="status">
          The soundboard is full. Delete a sound to add another.
        </p>
      ) : null}
      <FieldError message={elsewhere(fields)} />
      <SoundFields
        idPrefix="admin-sound-new"
        name={name}
        emoji={emoji}
        volume={volume}
        fields={fields}
        disabled={full}
        onName={setName}
        onEmoji={setEmoji}
        onVolume={setVolume}
      />
      <div className="settings-field">
        <label className="settings-label" htmlFor="admin-sound-file">
          Audio file
        </label>
        <input
          ref={input}
          id="admin-sound-file"
          className="input"
          type="file"
          accept={SOUND_FILE_TYPES}
          disabled={full}
          onChange={(event) => setFile(event.target.files?.[0] ?? null)}
        />
        <p className="settings-hint text-faint">MP3, Ogg, WAV or audio-only WebM, at most 512 KB</p>
        <FieldError message={soundFieldError("audio", fields.audio?.[0])} />
      </div>
      <div className="settings-actions">
        <Button
          type="submit"
          variant="primary"
          icon="cloud-upload"
          loading={busy("create")}
          disabled={full || busy("create")}
        >
          Add sound
        </Button>
      </div>
    </form>
  );
}

/** The edit dialog's form: the sound's name, emoji and default volume. */
function EditSound({
  sound,
  onClose,
  onSaved,
}: {
  readonly sound: SoundboardSound;
  readonly onClose: () => void;
  readonly onSaved: (catalog: SoundboardCatalog) => void;
}) {
  const [name, setName] = useState(sound.name);
  const [emoji, setEmoji] = useState(sound.emoji ?? "");
  const [volume, setVolume] = useState(Math.round(sound.volume * 100));
  const [fields, setFields] = useState<Fields>({});
  const { busy, track } = useBusy();

  const dirty =
    name !== sound.name ||
    emojiOrNull(emoji) !== sound.emoji ||
    volume !== Math.round(sound.volume * 100);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setFields({});

    void track(
      "save",
      soundboard.update(sound.id, { name, emoji: emojiOrNull(emoji), volume: volume / 100 }).then(
        (catalog) => {
          onSaved(catalog);
          onClose();
          toast({ title: "Sound saved", tone: "success" });
        },
        (error: Error) => {
          const named = fieldsOf(error);

          if (Object.keys(named).length > 0) {
            setFields(named);
          } else {
            adminFailure("Couldn't save the sound", error);
          }
        },
      ),
    );
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      title={`Edit ${sound.name}`}
      dirty={dirty}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            form="admin-sound-edit"
            variant="primary"
            loading={busy("save")}
            disabled={busy("save")}
          >
            Save
          </Button>
        </>
      }
    >
      <form id="admin-sound-edit" className="settings-form" onSubmit={submit}>
        <FieldError message={elsewhere(fields)} />
        <SoundFields
          idPrefix="admin-sound-edit"
          name={name}
          emoji={emoji}
          volume={volume}
          fields={fields}
          onName={setName}
          onEmoji={setEmoji}
          onVolume={setVolume}
        />
      </form>
    </Dialog>
  );
}

/** Plays one sound at a time, at its default volume: pressing it again (or another) stops it. */
function usePreview() {
  const [playing, setPlaying] = useState<number | null>(null);
  const audio = useRef<HTMLAudioElement | null>(null);

  const stop = () => {
    audio.current?.pause();
    audio.current = null;
    setPlaying(null);
  };

  useEffect(() => () => audio.current?.pause(), []);

  const toggle = (sound: SoundboardSound) => {
    const again = playing === sound.id;

    stop();

    if (again) {
      return;
    }

    const clip = new Audio(sound.audioUrl);

    clip.volume = Math.max(0, Math.min(1, sound.volume));

    const ended = () => {
      if (audio.current === clip) {
        audio.current = null;
        setPlaying(null);
      }
    };

    clip.addEventListener("ended", ended);
    clip.addEventListener("error", ended);
    audio.current = clip;
    setPlaying(sound.id);
    clip.play().catch(ended);
  };

  return { playing, toggle };
}

/**
 * The voice soundboard, Discord's: the sounds members play into calls. Upload one (a name, an
 * optional emoji, a default volume and an MP3, Ogg, WAV or WebM file of at most 512 KB), preview,
 * edit or delete one. The workspace holds a limited number, shown as usage of the limit; at the
 * limit the form says the soundboard is full.
 */
export function SoundsSection() {
  const { workspace } = useAdmin();
  const load = useSoundboard((state) => state.load);
  const [editing, setEditing] = useState<SoundboardSound | null>(null);
  const [doomed, setDoomed] = useState<SoundboardSound | null>(null);
  const { busy, track } = useBusy();
  const { playing, toggle } = usePreview();
  const container = useRowFocus();

  useEffect(() => {
    if (workspace.canAdminister) void loadSoundboardCatalog(true).catch(() => undefined);
  }, [workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const refetch = () => void loadSoundboardCatalog(true).catch(() => undefined);

  const destroy = () => {
    const sound = doomed;

    setDoomed(null);

    if (sound === null) {
      return;
    }

    void track(
      `sound-${sound.id}`,
      soundboard
        .destroy(sound.id)
        .then(replaceSoundboardCatalog, (error: Error) =>
          adminFailure(`Couldn't delete ${sound.name}`, error),
        ),
    );
  };

  const catalog = load.status === "ready" ? load.catalog : null;
  const full = catalog !== null && catalog.sounds.length >= catalog.capacity;

  return (
    <SettingsPage
      title="Soundboard"
      description="Short sounds members can play to everyone in a call."
    >
      <SettingsGroup
        title="Add a sound"
        description={
          catalog === null ? undefined : soundCapacity(catalog.sounds.length, catalog.capacity)
        }
      >
        <NewSound full={full} onSaved={replaceSoundboardCatalog} onRefused={refetch} />
      </SettingsGroup>
      <SettingsGroup title="Sounds">
        <div ref={container} tabIndex={-1} className="admin-focus-root">
          {load.status === "idle" || load.status === "loading" ? (
            <PaneListSkeleton rows={3} />
          ) : null}
          {load.status === "error" ? <PaneError message={load.message} onRetry={refetch} /> : null}
          {catalog !== null && catalog.sounds.length === 0 ? (
            <p className="text-muted">No sounds yet. Add one above.</p>
          ) : null}
          {catalog !== null && catalog.sounds.length > 0 ? (
            <ul className="settings-list" aria-label="Sounds">
              {catalog.sounds.map((sound) => (
                <li
                  key={sound.id}
                  className="settings-list-row admin-sound-row"
                  data-row={sound.id}
                  tabIndex={-1}
                >
                  <span className="admin-sound-emoji" aria-hidden="true">
                    {sound.emoji ?? NO_EMOJI}
                  </span>
                  <span className="settings-list-main">
                    <strong className="admin-sound-name">{sound.name}</strong>
                    <span className="text-faint">Volume {soundVolumeLabel(sound.volume)}</span>
                  </span>
                  <span className="admin-sound-actions">
                    <Button
                      variant="secondary"
                      size="sm"
                      icon={playing === sound.id ? "x" : "play"}
                      aria-pressed={playing === sound.id}
                      data-row-control="preview"
                      onClick={() => toggle(sound)}
                    >
                      <span className="visually-hidden">
                        {playing === sound.id ? "Stop" : "Preview"} {sound.name}
                      </span>
                    </Button>
                    <Button
                      variant="secondary"
                      size="sm"
                      icon="pencil"
                      data-row-control="edit"
                      disabled={busy(`sound-${sound.id}`)}
                      onClick={() => setEditing(sound)}
                    >
                      <span className="visually-hidden">Edit {sound.name}</span>
                    </Button>
                    <Button
                      variant="danger"
                      size="sm"
                      icon="trash"
                      data-row-control="delete"
                      disabled={busy(`sound-${sound.id}`)}
                      onClick={() => setDoomed(sound)}
                    >
                      <span className="visually-hidden">Delete {sound.name}</span>
                    </Button>
                  </span>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      </SettingsGroup>
      {editing === null ? null : (
        <EditSound
          key={editing.id}
          sound={editing}
          onClose={() => setEditing(null)}
          onSaved={replaceSoundboardCatalog}
        />
      )}
      <Dialog
        open={doomed !== null}
        onOpenChange={(open) => {
          if (!open) setDoomed(null);
        }}
        role="alertdialog"
        size="sm"
        title={doomed === null ? "" : `Delete ${doomed.name}?`}
        description="It leaves every call's soundboard."
        footer={
          <>
            <Button variant="secondary" onClick={() => setDoomed(null)} data-autofocus>
              Cancel
            </Button>
            <Button variant="danger" onClick={destroy}>
              Delete
            </Button>
          </>
        }
      />
    </SettingsPage>
  );
}
