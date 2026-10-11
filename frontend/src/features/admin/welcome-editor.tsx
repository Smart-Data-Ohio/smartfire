import { useLocation } from "@tanstack/react-router";
import { type FormEvent, type ReactNode, useEffect, useId, useRef, useState } from "react";
import type { WelcomeAdmin } from "../../gen/WelcomeAdmin.ts";
import type { WelcomeRoomChoice } from "../../gen/WelcomeRoomChoice.ts";
import { admin } from "../../sync/admin.ts";
import { ActionError } from "../../sync/run.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { toast } from "../../ui/toast-store.ts";
import { SettingsGroup, useBusy } from "../settings/settings-parts.tsx";
import { adminFailure } from "./admin-parts.tsx";
import {
  BODY_MAX,
  blankEntry,
  blankRoom,
  DESCRIPTION_MAX,
  type EntryDraft,
  entryDrafts,
  entryErrors,
  GUIDE_MAX,
  guideUpdateOf,
  hasErrors,
  length,
  MESSAGE_MAX,
  messageError,
  moved,
  ROOMS_MAX,
  type RoomDraft,
  type RowErrors,
  roomDrafts,
  roomErrors,
  TITLE_MAX,
  welcomeUpdateOf,
} from "./welcome-draft.ts";
import "./welcome-editor.css";

/** A field's hint, or its error in its place; announced as it changes. */
function FieldNote({
  id,
  error,
  children,
}: {
  readonly id: string;
  readonly error: string | undefined;
  readonly children?: ReactNode;
}) {
  return (
    <p id={id} className={error === undefined ? "field-hint" : "field-error"} aria-live="polite">
      {error ?? children}
    </p>
  );
}

/** A labelled one-line input with its hint or error. */
function Line({
  label,
  value,
  onChange,
  error,
  hint,
  maxLength,
  className,
}: {
  readonly label: string;
  readonly value: string;
  readonly onChange: (value: string) => void;
  readonly error: string | undefined;
  readonly hint?: ReactNode;
  readonly maxLength?: number;
  readonly className?: string;
}) {
  const id = useId();

  return (
    <div className={className === undefined ? "field" : `field ${className}`}>
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        className="input"
        value={value}
        maxLength={maxLength}
        autoComplete="off"
        aria-invalid={error !== undefined || undefined}
        aria-describedby={`${id}-note`}
        onChange={(event) => onChange(event.target.value)}
      />
      <FieldNote id={`${id}-note`} error={error}>
        {hint}
      </FieldNote>
    </div>
  );
}

/** A labelled textarea with a character count, or its error. */
function Paragraph({
  label,
  value,
  onChange,
  error,
  max,
  hint,
  rows = 4,
}: {
  readonly label: string;
  readonly value: string;
  readonly onChange: (value: string) => void;
  readonly error: string | undefined;
  readonly max: number;
  readonly hint: string;
  readonly rows?: number;
}) {
  const id = useId();

  return (
    <div className="field">
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <textarea
        id={id}
        className="input settings-textarea"
        rows={rows}
        value={value}
        aria-invalid={error !== undefined || undefined}
        aria-describedby={`${id}-note`}
        onChange={(event) => onChange(event.target.value)}
      />
      <FieldNote id={`${id}-note`} error={error}>
        {hint} {length(value.trim()).toLocaleString()}/{max.toLocaleString()}
      </FieldNote>
    </div>
  );
}

/** The picker's value for a saved room this administrator can't see. */
const KEPT = "kept";

/**
 * The room picker: the rooms this administrator can suggest, by name. A saved room they can't
 * see (`kept`: the server sent a token, not its id) stays chosen, unnamed, until they pick
 * another.
 */
function RoomPicker({
  label,
  value,
  kept,
  choices,
  onChange,
  error,
}: {
  readonly label: string;
  readonly value: number | null;
  readonly kept: boolean;
  readonly choices: readonly WelcomeRoomChoice[];
  readonly onChange: (roomId: number | null) => void;
  readonly error: string | undefined;
}) {
  const id = useId();
  const hidden = value === null && kept;

  return (
    <div className="field welcome-editor-room">
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <select
        id={id}
        className="input settings-select"
        value={hidden ? KEPT : value === null ? "" : String(value)}
        aria-invalid={error !== undefined || undefined}
        aria-describedby={`${id}-note`}
        onChange={(event) => {
          if (event.target.value === KEPT) {
            return;
          }

          const picked = choices.find((choice) => String(choice.id) === event.target.value);

          onChange(picked?.id ?? null);
        }}
      >
        <option value="">Choose a room…</option>
        {hidden ? <option value={KEPT}>A room you can't see</option> : null}
        {choices.map((choice) => (
          <option key={choice.id} value={String(choice.id)}>
            #{choice.name}
          </option>
        ))}
      </select>
      <FieldNote id={`${id}-note`} error={error} />
    </div>
  );
}

/** Up, down and remove for row `index` of `count`; `noun` names it ("room 2"). */
function RowTools({
  noun,
  index,
  count,
  onMove,
  onRemove,
}: {
  readonly noun: string;
  readonly index: number;
  readonly count: number;
  readonly onMove: (to: number) => void;
  readonly onRemove: () => void;
}) {
  return (
    <div className="welcome-editor-tools">
      <IconButton
        icon="arrow-up"
        size="sm"
        label={`Move ${noun} up`}
        data-tool="up"
        disabled={index === 0}
        onClick={() => onMove(index - 1)}
      />
      <IconButton
        icon="arrow-down"
        size="sm"
        label={`Move ${noun} down`}
        data-tool="down"
        disabled={index === count - 1}
        onClick={() => onMove(index + 1)}
      />
      <IconButton icon="trash" size="sm" label={`Remove ${noun}`} onClick={onRemove} />
    </div>
  );
}

/** Focus, once React has moved the rows, onto `selector` inside `root` (else `fallback`). */
function focusAfter(root: HTMLElement | null, selector: string, fallback: string): void {
  requestAnimationFrame(() => {
    const target =
      root?.querySelector<HTMLElement>(`${selector}:not(:disabled)`) ??
      root?.querySelector<HTMLElement>(fallback);

    target?.focus();
  });
}

/** Which validation messages the server sent for `field`, as one line. */
function serverError(error: Error, field: string): string | null {
  if (!(error instanceof ActionError)) {
    return null;
  }

  const messages = error.fields[field] ?? [];

  return messages.length === 0 ? null : messages.join(" ");
}

function WelcomeForm({
  loaded,
  onSaved,
}: {
  readonly loaded: WelcomeAdmin;
  readonly onSaved: (next: WelcomeAdmin) => void;
}) {
  const { choices } = loaded;
  const [message, setMessage] = useState(loaded.message);
  const [rooms, setRooms] = useState<RoomDraft[]>(() => roomDrafts(loaded.rooms));
  const [refused, setRefused] = useState<string | null>(null);
  const { busy, track } = useBusy();
  const listRef = useRef<HTMLDivElement>(null);
  const errors = roomErrors(rooms);
  const messageProblem = messageError(message);
  const invalid = messageProblem !== undefined || hasErrors(errors);

  const change = (key: number, patch: Partial<Omit<RoomDraft, "key">>) =>
    setRooms((current) => current.map((room) => (room.key === key ? { ...room, ...patch } : room)));

  const submit = (event: FormEvent) => {
    event.preventDefault();

    if (invalid) {
      return;
    }

    setRefused(null);
    void track(
      "welcome",
      // The welcome's section only: a guide saved meanwhile stays as saved.
      admin.saveWelcome(welcomeUpdateOf(message, rooms)).then(
        (next) => {
          onSaved(next);
          toast({ title: "Welcome saved", tone: "success" });
        },
        (error: Error) => {
          const named = serverError(error, "message") ?? serverError(error, "rooms");

          if (named === null) {
            adminFailure("Couldn't save the welcome", error);
          } else {
            setRefused(named);
          }
        },
      ),
    );
  };

  return (
    <form className="settings-form welcome-editor" onSubmit={submit} noValidate>
      <Paragraph
        label="Welcome message"
        value={message}
        onChange={setMessage}
        error={messageProblem}
        max={MESSAGE_MAX}
        hint="Markdown, shown at the top of Home until each person dismisses it."
      />
      <fieldset className="welcome-editor-set">
        <legend className="field-label">Suggested rooms</legend>
        <p className="field-hint">
          Up to {ROOMS_MAX}. People see only the rooms they can open or join.
        </p>
        <div ref={listRef} className="welcome-editor-rows">
          {rooms.map((room, index) => {
            const rowErrors: RowErrors = errors[index] ?? {};
            const noun = `room ${index + 1}`;

            return (
              <fieldset key={room.key} className="welcome-editor-row" data-row={room.key}>
                <legend className="welcome-editor-legend">Suggested room {index + 1}</legend>
                <RoomPicker
                  label="Room"
                  value={room.roomId}
                  kept={room.keep !== null}
                  choices={choices}
                  error={rowErrors.room}
                  onChange={(roomId) => change(room.key, { roomId, keep: null })}
                />
                <Line
                  label="Emoji"
                  className="welcome-editor-emoji"
                  value={room.emoji}
                  maxLength={16}
                  error={rowErrors.emoji}
                  onChange={(emoji) => change(room.key, { emoji })}
                />
                <Line
                  label="Description"
                  className="welcome-editor-grow"
                  value={room.description}
                  error={rowErrors.description}
                  hint={`${length(room.description.trim())}/${DESCRIPTION_MAX}`}
                  onChange={(description) => change(room.key, { description })}
                />
                <RowTools
                  noun={noun}
                  index={index}
                  count={rooms.length}
                  onMove={(to) => {
                    setRooms((current) => moved(current, index, to));
                    focusAfter(
                      listRef.current,
                      `[data-row="${room.key}"] [data-tool="${to < index ? "up" : "down"}"]`,
                      // At the top or bottom that button is disabled: the other one, then.
                      `[data-row="${room.key}"] [data-tool="${to < index ? "down" : "up"}"]`,
                    );
                  }}
                  onRemove={() => {
                    const neighbour = rooms[index + 1] ?? rooms[index - 1];

                    setRooms((current) =>
                      current.filter((candidate) => candidate.key !== room.key),
                    );
                    focusAfter(
                      listRef.current?.parentElement ?? null,
                      neighbour === undefined
                        ? "[data-add]"
                        : `[data-row="${neighbour.key}"] select`,
                      "[data-add]",
                    );
                  }}
                />
              </fieldset>
            );
          })}
        </div>
        <Button
          type="button"
          variant="secondary"
          size="sm"
          icon="plus"
          data-add=""
          disabled={rooms.length >= ROOMS_MAX}
          onClick={() => {
            const added = blankRoom();

            setRooms((current) => [...current, added]);
            focusAfter(listRef.current, `[data-row="${added.key}"] select`, "select");
          }}
        >
          Add a room
        </Button>
      </fieldset>
      {refused === null ? null : (
        <p className="field-error" role="alert">
          {refused}
        </p>
      )}
      <Button
        type="submit"
        variant="primary"
        loading={busy("welcome")}
        disabled={busy("welcome") || invalid}
      >
        Save welcome
      </Button>
    </form>
  );
}

function GuideForm({
  loaded,
  onSaved,
}: {
  readonly loaded: WelcomeAdmin;
  readonly onSaved: (next: WelcomeAdmin) => void;
}) {
  const { choices } = loaded;
  const [entries, setEntries] = useState<EntryDraft[]>(() => entryDrafts(loaded.guide));
  const [refused, setRefused] = useState<string | null>(null);
  const { busy, track } = useBusy();
  const listRef = useRef<HTMLDivElement>(null);
  const errors = entryErrors(entries);
  const invalid = hasErrors(errors);

  const change = (key: number, patch: Partial<Omit<EntryDraft, "key">>) =>
    setEntries((current) =>
      current.map((entry) => (entry.key === key ? { ...entry, ...patch } : entry)),
    );

  const submit = (event: FormEvent) => {
    event.preventDefault();

    if (invalid) {
      return;
    }

    setRefused(null);
    void track(
      "guide",
      // The guide's section only: a welcome saved meanwhile stays as saved.
      admin.saveWelcome(guideUpdateOf(entries)).then(
        (next) => {
          onSaved(next);
          toast({ title: "Server guide saved", tone: "success" });
        },
        (error: Error) => {
          const named = serverError(error, "guide");

          if (named === null) {
            adminFailure("Couldn't save the server guide", error);
          } else {
            setRefused(named);
          }
        },
      ),
    );
  };

  return (
    <form className="settings-form welcome-editor" onSubmit={submit} noValidate>
      <div ref={listRef} className="welcome-editor-rows">
        {entries.map((entry, index) => {
          const rowErrors: RowErrors = errors[index] ?? {};
          const noun = `entry ${index + 1}`;

          return (
            <fieldset key={entry.key} className="welcome-editor-row" data-row={entry.key}>
              <legend className="welcome-editor-legend">Guide entry {index + 1}</legend>
              <Line
                label="Title"
                className="welcome-editor-grow"
                value={entry.title}
                error={rowErrors.title}
                hint={`${length(entry.title.trim())}/${TITLE_MAX}`}
                onChange={(title) => change(entry.key, { title })}
              />
              <Line
                label="Emoji"
                className="welcome-editor-emoji"
                value={entry.emoji}
                maxLength={16}
                error={rowErrors.emoji}
                onChange={(emoji) => change(entry.key, { emoji })}
              />
              <EntryKind
                value={entry.kind}
                onChange={(kind) => change(entry.key, { kind })}
                index={index}
              />
              {entry.kind === "room" ? (
                <RoomPicker
                  label="Room"
                  value={entry.roomId}
                  kept={entry.keep !== null}
                  choices={choices}
                  error={rowErrors.room}
                  onChange={(roomId) => change(entry.key, { roomId, keep: null })}
                />
              ) : (
                <div className="welcome-editor-wide">
                  <Paragraph
                    label="Note"
                    value={entry.body}
                    rows={3}
                    onChange={(body) => change(entry.key, { body })}
                    error={rowErrors.body}
                    max={BODY_MAX}
                    hint="Markdown."
                  />
                </div>
              )}
              <RowTools
                noun={noun}
                index={index}
                count={entries.length}
                onMove={(to) => {
                  setEntries((current) => moved(current, index, to));
                  focusAfter(
                    listRef.current,
                    `[data-row="${entry.key}"] [data-tool="${to < index ? "up" : "down"}"]`,
                    // At the top or bottom that button is disabled: the other one, then.
                    `[data-row="${entry.key}"] [data-tool="${to < index ? "down" : "up"}"]`,
                  );
                }}
                onRemove={() => {
                  const neighbour = entries[index + 1] ?? entries[index - 1];

                  setEntries((current) =>
                    current.filter((candidate) => candidate.key !== entry.key),
                  );
                  focusAfter(
                    listRef.current?.parentElement ?? null,
                    neighbour === undefined ? "[data-add]" : `[data-row="${neighbour.key}"] input`,
                    "[data-add]",
                  );
                }}
              />
            </fieldset>
          );
        })}
      </div>
      <Button
        type="button"
        variant="secondary"
        size="sm"
        icon="plus"
        data-add=""
        disabled={entries.length >= GUIDE_MAX}
        onClick={() => {
          const added = blankEntry();

          setEntries((current) => [...current, added]);
          focusAfter(listRef.current, `[data-row="${added.key}"] input`, "input");
        }}
      >
        Add an entry
      </Button>
      {refused === null ? null : (
        <p className="field-error" role="alert">
          {refused}
        </p>
      )}
      <Button
        type="submit"
        variant="primary"
        loading={busy("guide")}
        disabled={busy("guide") || invalid}
      >
        Save guide
      </Button>
    </form>
  );
}

/** A guide entry links a room or holds a note: two native radios. */
function EntryKind({
  value,
  onChange,
  index,
}: {
  readonly value: EntryDraft["kind"];
  readonly onChange: (kind: EntryDraft["kind"]) => void;
  readonly index: number;
}) {
  const name = useId();

  return (
    <div className="welcome-editor-kind" role="radiogroup" aria-label={`Entry ${index + 1} holds`}>
      <label className="welcome-editor-radio">
        <input
          type="radio"
          name={name}
          checked={value === "note"}
          onChange={() => onChange("note")}
        />
        A note
      </label>
      <label className="welcome-editor-radio">
        <input
          type="radio"
          name={name}
          checked={value === "room"}
          onChange={() => onChange("room")}
        />
        A room link
      </label>
    </div>
  );
}

/** The answer each form last saved, which it starts again from; `null` before its first save. */
interface FormStarts {
  readonly welcome: WelcomeAdmin | null;
  readonly guide: WelcomeAdmin | null;
}

/**
 * The welcome card and the server guide, for administrators: two groups in the workspace
 * settings, each saving only its own section, so saving one never puts back an older copy of the
 * other (even with both saves in flight). Loaded once on mount; a save answers with the stored
 * copy, which the form that saved starts again from.
 */
export function WelcomeEditor() {
  const [loaded, setLoaded] = useState<WelcomeAdmin | null>(null);
  const [failed, setFailed] = useState(false);

  // A form starts again from the answer to its own save (not whichever answer landed last); the
  // other keeps its edits.
  const [starts, setStarts] = useState<FormStarts>({ welcome: null, guide: null });
  const [generations, setGenerations] = useState({ welcome: 0, guide: 0 });
  const { hash } = useLocation();

  useEffect(() => {
    let live = true;

    admin.welcome().then(
      (next) => {
        if (live) {
          setLoaded(next);
        }
      },
      () => {
        if (live) {
          setFailed(true);
        }
      },
    );

    return () => {
      live = false;
    };
  }, []);

  // A link to `#welcome` or `#guide` (the card's and the guide's Edit) lands once it's loaded.
  useEffect(() => {
    if (loaded !== null && (hash === "welcome" || hash === "guide")) {
      document.getElementById(hash)?.scrollIntoView({ block: "start" });
    }
  }, [loaded, hash]);

  const saved = (form: "welcome" | "guide") => (next: WelcomeAdmin) => {
    setLoaded(next);
    setStarts((current) => ({ ...current, [form]: next }));
    setGenerations((current) => ({ ...current, [form]: current[form] + 1 }));
  };

  const body = (form: (loaded: WelcomeAdmin) => ReactNode, start: WelcomeAdmin | null) =>
    loaded === null ? (
      failed ? (
        <p className="text-muted">Couldn't load these settings.</p>
      ) : (
        <Skeleton width={240} height={14} />
      )
    ) : (
      form({ ...(start ?? loaded), choices: loaded.choices })
    );

  return (
    <>
      <SettingsGroup
        id="welcome"
        title="Welcome screen"
        description="A message and up to five suggested rooms at the top of Home, for everyone."
      >
        {body(
          (current) => (
            <WelcomeForm key={generations.welcome} loaded={current} onSaved={saved("welcome")} />
          ),
          starts.welcome,
        )}
      </SettingsGroup>
      <SettingsGroup
        id="guide"
        title="Server guide"
        description="Up to ten entries, each a room link or a short note, on the Server guide page."
      >
        {body(
          (current) => (
            <GuideForm key={generations.guide} loaded={current} onSaved={saved("guide")} />
          ),
          starts.guide,
        )}
      </SettingsGroup>
    </>
  );
}
