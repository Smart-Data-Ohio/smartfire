import { useEffect, useId, useRef, useState } from "react";
import type { BoardPostForm } from "../../gen/BoardPostForm.ts";
import type { BoardTag } from "../../gen/BoardTag.ts";
import type { BoardTagCatalog } from "../../gen/BoardTagCatalog.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { uuid7 } from "../../lib/uuid7.ts";
import { useStore } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { AttachmentTray } from "../composer/attachments/attachment-tray.tsx";
import {
  fileOptions,
  MAX_FILES,
  pastedName,
  useAttachments,
} from "../composer/attachments/use-attachments.ts";
import { isAgent } from "../people/people.ts";
import { MAX_TAGS, parseTags, WORK_STATUS_LABEL, WORK_STATUSES } from "./board-format.ts";
import { BoardTagPicker } from "./board-tag-picker.tsx";
import {
  addedDefault,
  catalogTag,
  defaultTag,
  EMPTY_CATALOG,
  initialSelection,
  keptSelection,
  postTagNames,
  postTagsProblem,
  REQUIRED_TAG_MESSAGE,
  toggleSelection,
} from "./board-tags.ts";
import { useBoardTagCatalog } from "./use-board-tags.ts";

type Fields = Readonly<Record<string, readonly string[]>>;

const NO_FIELDS: Fields = {};

const FIELD_LABEL = new Map([
  ["name", "Title"],
  ["message", "Brief"],
  ["ownerId", "Owner"],
  ["tags", "Tags"],
]);

/** The server's refusal of a post without a curated tag on a board that requires one. */
const MISSING_CATALOG_TAG = "must include a catalog tag";

/**
 * The server's messages ("can't be blank") as sentences under their field ("Title can't be
 * blank."). A missing required tag goes under the board's tag chips, in the form's own words.
 */
function sentences(fields: Fields): Fields {
  const named: [string, readonly string[]][] = [];

  for (const [field, messages] of Object.entries(fields)) {
    const missing = field === "tags" && messages.includes(MISSING_CATALOG_TAG);
    const rest = missing ? messages.filter((message) => message !== MISSING_CATALOG_TAG) : messages;

    if (missing) named.push(["catalog", [REQUIRED_TAG_MESSAGE]]);

    if (rest.length > 0) {
      named.push([field, rest.map((message) => `${FIELD_LABEL.get(field) ?? "This"} ${message}.`)]);
    }
  }

  return Object.fromEntries(named);
}

function fieldMessage(fields: Fields, field: string): string | undefined {
  const messages = fields[field] ?? [];

  return messages.length === 0 ? undefined : messages.join(" ");
}

/**
 * What the form can tell before asking the server: a missing or long title, a required board
 * tag left out, too many tags or a bad free-text one.
 */
function localProblems(
  name: string,
  catalog: Omit<BoardTagCatalog, "roomId">,
  tags: readonly string[],
): Fields {
  const found: [string, readonly string[]][] = [];
  const tagProblem = postTagsProblem(catalog, tags);

  if (name === "") {
    found.push(["name", ["Give the post a title."]]);
  } else if (name.length > 100) {
    found.push(["name", ["Keep the title to 100 characters."]]);
  }

  if (tagProblem !== undefined) {
    found.push([tagProblem.field, [tagProblem.message]]);
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

function blankApartFromStatus(draft: Draft): boolean {
  return (
    draft.name === EMPTY.name &&
    draft.brief === EMPTY.brief &&
    draft.ownerId === EMPTY.ownerId &&
    draft.tags === EMPTY.tags
  );
}

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
  total,
  catalog,
  onAdd,
}: {
  readonly suggestions: readonly string[];
  readonly chosen: readonly string[];
  /** Every tag the post would get, curated ones included: five is the most. */
  readonly total: number;
  /** The board's curated tags have their own chips, so they aren't suggested here. */
  readonly catalog: readonly BoardTag[];
  readonly onAdd: (tag: string) => void;
}) {
  const left = suggestions.filter(
    (tag) => !chosen.includes(tag) && catalogTag(catalog, tag) === undefined,
  );

  if (left.length === 0 || total >= MAX_TAGS) {
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
  readonly initialStatus: WorkStatus;
  readonly onClose: () => void;
  readonly onCreated: (threadId: number) => void;
}

/**
 * "New post" (classic `channel_threads#new` on a board): a title, an optional Markdown brief that
 * becomes the first message, the status (the column's when opened from one), an owner (Unassigned by default)
 * and up to five tags: the board's curated ones as chips (its default picked to start, at least
 * one needed when the board requires it), then free-text ones as before. The chips follow the
 * catalog live. Creating it opens the post in the right pane.
 */
export function NewPostDialog({
  roomId,
  open,
  initialStatus,
  onClose,
  onCreated,
}: NewPostDialogProps) {
  const roomName = useStore((state) => state.rooms[roomId]?.detail?.displayName ?? null);
  const [draft, setDraft] = useState<Draft>(EMPTY);
  // The picked curated tags; `null` until the viewer touches a chip, so the default (which may
  // arrive or change while the dialog is open) stays picked until then.
  const [picked, setPicked] = useState<readonly number[] | null>(null);
  const catalog = useBoardTagCatalog(roomId) ?? EMPTY_CATALOG;
  const firstChipRef = useRef<HTMLButtonElement | null>(null);
  const [form, setForm] = useState<BoardPostForm | null>(null);
  const [fields, setFields] = useState<Fields>(NO_FIELDS);
  const [attempt, setAttempt] = useState(0);
  const [saving, setSaving] = useState(false);
  const briefId = useId();
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const attachments = useAttachments(MAX_FILES);
  // One opening of the dialog: a reply that lands after Cancel, a reopen or a room change belongs
  // to an opening that's gone, and must neither navigate nor touch the new draft.
  const openingRef = useRef(0);
  // The opening's retry identity: every attempt to create this post sends it, so a retry after a
  // lost reply answers the post the first attempt made instead of making a second.
  const clientIdRef = useRef(uuid7(Date.now()));
  const savingRef = useRef(false);

  useEffect(() => {
    if (!open) {
      for (const file of attachments.files) attachments.remove(file.id);
    }
  }, [open, attachments]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: opening or a new room is the trigger
  useEffect(() => {
    openingRef.current += 1;
    clientIdRef.current = uuid7(Date.now());
    savingRef.current = false;
    setSaving(false);
    setPicked(null);

    // Unmounting ends the opening too: the room pane is keyed by room, so a room change unmounts.
    return () => {
      openingRef.current += 1;
    };
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

  useEffect(() => {
    if (open) {
      setDraft((previous) =>
        blankApartFromStatus(previous) ? { ...previous, status: initialStatus } : previous,
      );
    }
  }, [open, initialStatus]);

  const set = (patch: Partial<Draft>) => setDraft((previous) => ({ ...previous, ...patch }));
  const freeTags = parseTags(draft.tags);
  const startSelection = initialSelection(catalog);
  const selection = picked === null ? startSelection : keptSelection(picked, catalog.tags);
  const tags = postTagNames(catalog.tags, selection, freeTags);

  const dirty =
    !blankApartFromStatus(draft) ||
    draft.status !== initialStatus ||
    selection.join(",") !== startSelection.join(",") ||
    attachments.files.length > 0;

  const fail = (next: Fields) => {
    setFields(next);
    setAttempt((count) => count + 1);
  };

  const addFiles = (files: readonly File[]) => {
    if (savingRef.current) return;

    const overflow = attachments.add(files);

    if (overflow > 0) {
      toast({
        title: `A message holds up to ${MAX_FILES} files`,
        description: `${overflow === 1 ? "1 file wasn't" : `${overflow} files weren't`} added. Create this post, then add the rest in a reply.`,
      });
    }
  };

  const submit = () => {
    if (savingRef.current) {
      return;
    }

    const name = draft.name.trim();
    const problems = localProblems(name, catalog, tags);

    if (Object.keys(problems).length > 0) {
      fail(problems);

      // A missing required tag is the one problem the title's autofocus doesn't point at.
      if (problems.name === undefined && problems.catalog !== undefined) {
        firstChipRef.current?.focus();
      }

      return;
    }

    if (!attachments.ready) {
      toast({
        title: attachments.failed ? "A file didn't upload" : "Wait for the files to upload",
        description: attachments.failed
          ? "Retry or remove it before creating the post."
          : "Create the post once all uploads finish.",
        tone: "danger",
      });

      return;
    }

    const files = attachments.files;
    const uploaded = fileOptions(files);
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
        attachmentSignedId: uploaded.attachmentSignedId ?? null,
        attachmentSignedIds: uploaded.attachmentSignedIds ?? [],
        attachmentSpoilers: uploaded.attachmentSpoilers ?? [],
        clientId: clientIdRef.current,
      })
      .then(
        (detail) => {
          if (!current()) {
            return;
          }

          attachments.clearSent(files);
          setDraft(EMPTY);
          setPicked(null);
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
      for (const file of attachments.files) attachments.remove(file.id);
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
      dirty={dirty}
      footer={
        <>
          <Button variant="secondary" onClick={() => close(false)}>
            Cancel
          </Button>
          <Button variant="primary" loading={saving} disabled={!attachments.ready} onClick={submit}>
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
            onPaste={(event) => {
              const files = [...event.clipboardData.files];

              if (files.length > 0) {
                event.preventDefault();
                addFiles(files.map((file) => pastedName(file)));
              }
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                event.preventDefault();
                submit();
              }
            }}
          />
          <AttachmentTray
            files={attachments.files}
            onRemove={attachments.remove}
            onRetry={attachments.retry}
            onToggleSpoiler={attachments.toggleSpoiler}
          />
          <IconButton
            icon="plus"
            label="Upload files"
            size="sm"
            disabled={saving}
            onClick={() => fileInputRef.current?.click()}
          />
          <input
            ref={fileInputRef}
            type="file"
            multiple
            hidden
            tabIndex={-1}
            aria-hidden="true"
            disabled={saving}
            onChange={(event) => {
              addFiles([...(event.target.files ?? [])]);
              event.target.value = "";
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
        {catalog.tags.length > 0 ? (
          <BoardTagPicker
            tags={catalog.tags}
            selected={selection}
            required={catalog.tagsRequired}
            defaultName={defaultTag(catalog)?.name}
            added={addedDefault(catalog, tags)?.name}
            error={fieldMessage(fields, "catalog")}
            attempt={attempt}
            firstRef={firstChipRef}
            onToggle={(id) => {
              setPicked(toggleSelection(selection, id, catalog.tags));
              setFields((current) => {
                const { catalog: _cleared, ...rest } = current;

                return rest;
              });
            }}
          />
        ) : null}
        <TextField
          label={catalog.tags.length > 0 ? "More tags" : "Tags"}
          hint={`Optional. Up to ${MAX_TAGS} in all, separated by commas.`}
          placeholder="bug, api"
          value={draft.tags}
          error={fieldMessage(fields, "tags")}
          attempt={attempt}
          onChange={(event) => set({ tags: event.target.value })}
        />
        <TagSuggestions
          suggestions={form?.tagSuggestions ?? []}
          chosen={freeTags}
          total={tags.length}
          catalog={catalog.tags}
          onAdd={(tag) => set({ tags: [...freeTags, tag].join(", ") })}
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
