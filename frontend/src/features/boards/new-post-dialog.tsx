import { useEffect, useId, useRef, useState } from "react";
import type { BoardPostForm } from "../../gen/BoardPostForm.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { uuid7 } from "../../lib/uuid7.ts";
import { useStore } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { isAgent } from "../people/people.ts";
import {
  MAX_TAGS,
  parseTags,
  tagsProblem,
  WORK_STATUS_LABEL,
  WORK_STATUSES,
} from "./board-format.ts";

type Fields = Readonly<Record<string, readonly string[]>>;

const NO_FIELDS: Fields = {};

const FIELD_LABEL = new Map([
  ["name", "Title"],
  ["message", "Brief"],
  ["ownerId", "Owner"],
  ["tags", "Tags"],
]);

/** The server's messages ("can't be blank") as sentences under their field ("Title can't be blank."). */
function sentences(fields: Fields): Fields {
  return Object.fromEntries(
    Object.entries(fields).map(([field, messages]) => [
      field,
      messages.map((message) => `${FIELD_LABEL.get(field) ?? "This"} ${message}.`),
    ]),
  );
}

function fieldMessage(fields: Fields, field: string): string | undefined {
  const messages = fields[field] ?? [];

  return messages.length === 0 ? undefined : messages.join(" ");
}

/** What the form can tell before asking the server: a missing or long title, bad tags. */
function localProblems(name: string, tags: readonly string[]): Fields {
  const found: [string, readonly string[]][] = [];
  const tagProblem = tagsProblem(tags);

  if (name === "") {
    found.push(["name", ["Give the post a title."]]);
  } else if (name.length > 100) {
    found.push(["name", ["Keep the title to 100 characters."]]);
  }

  if (tagProblem !== undefined) {
    found.push(["tags", [tagProblem]]);
  }

  return Object.fromEntries(found);
}

interface Draft {
  readonly name: string;
  readonly brief: string;
  readonly status: WorkStatus;
  /** "" is unassigned. */
  readonly ownerId: string;
  readonly tags: string;
}

const EMPTY: Draft = { name: "", brief: "", status: "planned", ownerId: "", tags: "" };

/** The owner picker: Unassigned, then the board's people, then its agents, as the classic form. */
function OwnerSelect({
  form,
  value,
  error,
  onChange,
}: {
  readonly form: BoardPostForm | null;
  readonly value: string;
  readonly error: string | undefined;
  readonly onChange: (value: string) => void;
}) {
  const id = useId();
  const users = useStore((state) => state.users);
  const candidates = form?.ownerCandidates ?? [];
  const people = candidates.filter((candidate) => !isAgent(users[candidate.userId]));
  const agents = candidates.filter((candidate) => isAgent(users[candidate.userId]));
  const name = (userId: number) => users[userId]?.name ?? "Someone";

  return (
    <div className={`field${error === undefined ? "" : " is-error"}`}>
      <label className="field-label" htmlFor={id}>
        Owner
      </label>
      <select
        id={id}
        className="input board-select"
        value={value}
        disabled={form === null}
        aria-invalid={error === undefined ? undefined : true}
        onChange={(event) => onChange(event.target.value)}
      >
        <option value="">Unassigned</option>
        {people.length > 0 ? (
          <optgroup label="Members">
            {people.map((candidate) => (
              <option key={candidate.userId} value={candidate.userId}>
                {name(candidate.userId)}
              </option>
            ))}
          </optgroup>
        ) : null}
        {agents.length > 0 ? (
          <optgroup label="Agents">
            {agents.map((candidate) => (
              <option key={candidate.userId} value={candidate.userId}>
                {name(candidate.userId)}
              </option>
            ))}
          </optgroup>
        ) : null}
      </select>
      <p className="field-error t-error-msg" aria-live="polite">
        {error ?? ""}
      </p>
    </div>
  );
}

/** Tags already on the board, one click to add each to the field. */
function TagSuggestions({
  suggestions,
  chosen,
  onAdd,
}: {
  readonly suggestions: readonly string[];
  readonly chosen: readonly string[];
  readonly onAdd: (tag: string) => void;
}) {
  const left = suggestions.filter((tag) => !chosen.includes(tag));

  if (left.length === 0 || chosen.length >= MAX_TAGS) {
    return null;
  }

  return (
    <ul className="board-suggestions" aria-label="Tags on this board">
      {left.slice(0, 12).map((tag) => (
        <li key={tag}>
          <button type="button" className="board-suggestion" onClick={() => onAdd(tag)}>
            {tag}
          </button>
        </li>
      ))}
    </ul>
  );
}

interface NewPostDialogProps {
  readonly roomId: number;
  readonly open: boolean;
  readonly onClose: () => void;
  readonly onCreated: (threadId: number) => void;
}

/**
 * "New post" (classic `channel_threads#new` on a board): a title, an optional Markdown brief that
 * becomes the first message, the status (Planned by default), an owner (Unassigned by default)
 * and up to five tags. Creating it opens the post in the right pane.
 */
export function NewPostDialog({ roomId, open, onClose, onCreated }: NewPostDialogProps) {
  const roomName = useStore((state) => state.rooms[roomId]?.detail?.displayName ?? null);
  const [draft, setDraft] = useState<Draft>(EMPTY);
  const [form, setForm] = useState<BoardPostForm | null>(null);
  const [fields, setFields] = useState<Fields>(NO_FIELDS);
  const [attempt, setAttempt] = useState(0);
  const [saving, setSaving] = useState(false);
  const briefId = useId();
  // One opening of the dialog: a reply that lands after Cancel, a reopen or a room change belongs
  // to an opening that's gone, and must neither navigate nor touch the new draft.
  const openingRef = useRef(0);
  // The opening's retry identity: every attempt to create this post sends it, so a retry after a
  // lost reply answers the post the first attempt made instead of making a second.
  const clientIdRef = useRef(uuid7(Date.now()));
  const savingRef = useRef(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: opening or a new room is the trigger
  useEffect(() => {
    openingRef.current += 1;
    clientIdRef.current = uuid7(Date.now());
    savingRef.current = false;
    setSaving(false);
  }, [open, roomId]);

  useEffect(() => {
    if (!open) {
      return;
    }

    let current = true;

    actions.boards.postForm(roomId).then(
      (loaded) => {
        if (current) {
          setForm(loaded);
        }
      },
      (error: Error) => {
        if (current) {
          toast({
            title: "Couldn't load the post form",
            description: error.message,
            tone: "danger",
          });
        }
      },
    );

    return () => {
      current = false;
    };
  }, [open, roomId]);

  const set = (patch: Partial<Draft>) => setDraft((previous) => ({ ...previous, ...patch }));
  const tags = parseTags(draft.tags);

  const fail = (next: Fields) => {
    setFields(next);
    setAttempt((count) => count + 1);
  };

  const submit = () => {
    if (savingRef.current) {
      return;
    }

    const name = draft.name.trim();
    const problems = localProblems(name, tags);

    if (Object.keys(problems).length > 0) {
      fail(problems);

      return;
    }

    const opening = openingRef.current;
    const current = () => openingRef.current === opening;

    savingRef.current = true;
    setSaving(true);
    actions.boards
      .createPost(roomId, {
        name,
        status: draft.status,
        ownerId: draft.ownerId === "" ? null : Number(draft.ownerId),
        tags,
        brief: draft.brief,
        clientId: clientIdRef.current,
      })
      .then(
        (detail) => {
          if (!current()) {
            return;
          }

          setDraft(EMPTY);
          setFields(NO_FIELDS);
          onCreated(detail.thread.id);
        },
        (error: Error) => {
          if (!current()) {
            return;
          }

          const named = error instanceof ActionError ? sentences(error.fields) : NO_FIELDS;

          fail(named);

          if (Object.keys(named).length === 0) {
            toast({
              title: "Couldn't create the post",
              description: error.message,
              tone: "danger",
            });
          }
        },
      )
      .finally(() => {
        if (current()) {
          savingRef.current = false;
          setSaving(false);
        }
      });
  };

  const close = (next: boolean) => {
    if (!next) {
      setFields(NO_FIELDS);
      onClose();
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={close}
      title="New post"
      description={roomName === null ? undefined : `In ${roomName}`}
      footer={
        <>
          <Button variant="secondary" onClick={() => close(false)}>
            Cancel
          </Button>
          <Button variant="primary" loading={saving} onClick={submit}>
            Create post
          </Button>
        </>
      }
    >
      <form
        className="board-form"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <TextField
          label="Title"
          placeholder="What needs doing?"
          value={draft.name}
          maxLength={100}
          error={fieldMessage(fields, "name")}
          attempt={attempt}
          data-autofocus
          onChange={(event) => set({ name: event.target.value })}
        />
        <div className={`field${fieldMessage(fields, "message") === undefined ? "" : " is-error"}`}>
          <label className="field-label" htmlFor={briefId}>
            Brief <span className="field-optional">optional, Markdown</span>
          </label>
          <textarea
            id={briefId}
            className="input board-brief"
            placeholder="Write the brief for whoever picks this up…"
            rows={5}
            value={draft.brief}
            onChange={(event) => set({ brief: event.target.value })}
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                event.preventDefault();
                submit();
              }
            }}
          />
          <p className="field-error t-error-msg" aria-live="polite">
            {fieldMessage(fields, "message") ?? ""}
          </p>
        </div>
        <div className="board-form-row">
          <StatusSelect value={draft.status} onChange={(status) => set({ status })} />
          <OwnerSelect
            form={form}
            value={draft.ownerId}
            error={fieldMessage(fields, "ownerId")}
            onChange={(ownerId) => set({ ownerId })}
          />
        </div>
        <TextField
          label="Tags"
          hint={`Optional. Up to ${MAX_TAGS}, separated by commas.`}
          placeholder="bug, api"
          value={draft.tags}
          error={fieldMessage(fields, "tags")}
          attempt={attempt}
          onChange={(event) => set({ tags: event.target.value })}
        />
        <TagSuggestions
          suggestions={form?.tagSuggestions ?? []}
          chosen={tags}
          onAdd={(tag) => set({ tags: [...tags, tag].join(", ") })}
        />
      </form>
    </Dialog>
  );
}

function StatusSelect({
  value,
  onChange,
}: {
  readonly value: WorkStatus;
  readonly onChange: (value: WorkStatus) => void;
}) {
  const id = useId();

  return (
    <div className="field">
      <label className="field-label" htmlFor={id}>
        Status
      </label>
      <select
        id={id}
        className="input board-select"
        value={value}
        onChange={(event) => {
          const picked = WORK_STATUSES.find((status) => status === event.target.value);

          if (picked !== undefined) {
            onChange(picked);
          }
        }}
      >
        {WORK_STATUSES.map((status) => (
          <option key={status} value={status}>
            {WORK_STATUS_LABEL[status]}
          </option>
        ))}
      </select>
      <p className="field-error t-error-msg" aria-hidden="true" />
    </div>
  );
}
