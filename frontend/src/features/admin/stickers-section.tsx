import { type FormEvent, useEffect, useRef, useState } from "react";
import type { Sticker } from "../../gen/Sticker.ts";
import type { StickerCatalog } from "../../gen/StickerCatalog.ts";
import { EmojiImage } from "../../lib/emoji/emoji-image.tsx";
import { admin } from "../../sync/admin.ts";
import { ActionError } from "../../sync/run.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import {
  loadStickerCatalog,
  replaceStickerCatalog,
  useStickerLoad,
} from "../composer/sticker-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import {
  FieldError,
  fieldsOf,
  SettingsGroup,
  SettingsPage,
  useBusy,
} from "../settings/settings-parts.tsx";
import { stickerCapacity, stickerFieldError, stickerLimitMessage } from "./admin-format.ts";
import {
  AdministratorsOnly,
  adminFailure,
  uploaded,
  useAdmin,
  useRowFocus,
} from "./admin-parts.tsx";
import { STICKER_FILE_TYPES, stickerFileProblem } from "./sticker-file.ts";

type Fields = Readonly<Record<string, readonly string[]>>;

/** The forms' own fields: their messages show under them. */
const FORM_FIELDS = new Set(["name", "description", "tags", "image"]);

/** Messages about anything else, for the line above the form. */
function elsewhere(fields: Fields): string | undefined {
  const messages = Object.entries(fields)
    .filter(([key]) => !FORM_FIELDS.has(key))
    .flatMap(([key, each]) => each.map((message) => stickerFieldError(key, message) ?? message));

  return messages.length === 0 ? undefined : messages.join(" ");
}

/** The limit a `StickerLimitReached` refusal names, else the catalog's own. */
function refusedLimit(error: ActionError, capacity: number | null): number | null {
  const named = /limit: (\d+)/.exec(error.message)?.[1];

  return named === undefined ? capacity : Number(named);
}

/** Name, description and tags: the fields a new sticker and an edit share. */
function StickerFields({
  name,
  description,
  tags,
  fields,
  disabled = false,
  onName,
  onDescription,
  onTags,
}: {
  readonly name: string;
  readonly description: string;
  readonly tags: string;
  readonly fields: Fields;
  readonly disabled?: boolean;
  readonly onName: (name: string) => void;
  readonly onDescription: (description: string) => void;
  readonly onTags: (tags: string) => void;
}) {
  const error = (key: string) => stickerFieldError(key, fields[key]?.[0]);

  return (
    <>
      <TextField
        label="Name"
        hint="2–30 characters"
        value={name}
        autoComplete="off"
        placeholder="Party parrot"
        maxLength={60}
        disabled={disabled}
        error={error("name")}
        onChange={(event) => onName(event.target.value)}
      />
      <TextField
        label="Description"
        hint="Optional"
        value={description}
        autoComplete="off"
        placeholder="For celebrations"
        disabled={disabled}
        error={error("description")}
        onChange={(event) => onDescription(event.target.value)}
      />
      <TextField
        label="Tags"
        hint="Words or an emoji the picker's search finds it by"
        value={tags}
        autoComplete="off"
        placeholder="🎉 party"
        disabled={disabled}
        error={error("tags")}
        onChange={(event) => onTags(event.target.value)}
      />
    </>
  );
}

interface NewStickerProps {
  readonly full: boolean;
  readonly capacity: number | null;
  readonly onSaved: (catalog: StickerCatalog) => void;
  /** A refused upload: the count may have moved (another administrator's upload, say). */
  readonly onRefused: () => void;
}

/**
 * The upload form: name, description, tags and the image, saved together. The image is checked
 * here first (type, size and dimensions), so a wrong file never uploads.
 */
function NewSticker({ full, capacity, onSaved, onRefused }: NewStickerProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [tags, setTags] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [fileProblem, setFileProblem] = useState<string | null>(null);
  const [fields, setFields] = useState<Fields>({});
  const [limit, setLimit] = useState<string | null>(null);
  const input = useRef<HTMLInputElement | null>(null);
  const { busy, track } = useBusy();

  const choices = useRef(0);

  const choose = (chosen: File | null) => {
    const choice = ++choices.current;

    setFile(chosen);
    setFileProblem(null);

    if (chosen !== null) {
      void stickerFileProblem(chosen).then((problem) => {
        // A later choice wins over this one's check.
        if (choice === choices.current) setFileProblem(problem);
      });
    }
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setFields({});
    setLimit(null);

    // With no file chosen the write goes out without one, and the server says so under the field.
    const save = async () => {
      const problem = file === null ? null : await stickerFileProblem(file);

      if (problem !== null) {
        setFileProblem(problem);

        return null;
      }

      const signedId = file === null ? null : await uploaded(file);

      return admin.createSticker({ name, description, tags, signedId });
    };

    void track(
      "create",
      save().then(
        (catalog) => {
          if (catalog === null) {
            return;
          }

          onSaved(catalog);
          setName("");
          setDescription("");
          setTags("");
          setFile(null);

          if (input.current !== null) {
            input.current.value = "";
          }

          toast({ title: "Sticker added", tone: "success" });
        },
        (error: Error) => {
          const named = fieldsOf(error);

          if (error instanceof ActionError && error.tag === "StickerLimitReached") {
            const most = refusedLimit(error, capacity);

            setLimit(most === null ? error.message : stickerLimitMessage(most));
            onRefused();
          } else if (Object.keys(named).length > 0) {
            setFields(named);
            onRefused();
          } else {
            adminFailure("Couldn't add the sticker", error);
          }
        },
      ),
    );
  };

  return (
    <form className="settings-form" onSubmit={submit} aria-label="Add a sticker">
      {full ? (
        <p className="settings-callout text-muted" role="status">
          The sticker catalog is full. Delete a sticker to add another.
        </p>
      ) : null}
      <FieldError message={limit ?? elsewhere(fields)} />
      <StickerFields
        name={name}
        description={description}
        tags={tags}
        fields={fields}
        disabled={full}
        onName={setName}
        onDescription={setDescription}
        onTags={setTags}
      />
      <div className="settings-field">
        <label className="settings-label" htmlFor="admin-sticker-file">
          Image
        </label>
        <input
          ref={input}
          id="admin-sticker-file"
          className="input"
          type="file"
          accept={STICKER_FILE_TYPES}
          disabled={full}
          aria-invalid={fileProblem !== null || undefined}
          onChange={(event) => choose(event.target.files?.[0] ?? null)}
        />
        <p className="settings-hint text-faint">
          PNG, APNG or GIF, at most 512 KB and 320 × 320 pixels
        </p>
        <FieldError message={fileProblem ?? stickerFieldError("image", fields.image?.[0])} />
      </div>
      <div className="settings-actions">
        <Button
          type="submit"
          variant="primary"
          icon="cloud-upload"
          loading={busy("create")}
          disabled={full || busy("create") || fileProblem !== null}
        >
          Add sticker
        </Button>
      </div>
    </form>
  );
}

/** The edit dialog's form: the sticker's name, description and tags (its image stays). */
function EditSticker({
  sticker,
  onClose,
  onSaved,
}: {
  readonly sticker: Sticker;
  readonly onClose: () => void;
  readonly onSaved: (catalog: StickerCatalog) => void;
}) {
  const [name, setName] = useState(sticker.name);
  const [description, setDescription] = useState(sticker.description);
  const [tags, setTags] = useState(sticker.tags);
  const [fields, setFields] = useState<Fields>({});
  const { busy, track } = useBusy();

  const dirty =
    name !== sticker.name || description !== sticker.description || tags !== sticker.tags;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setFields({});

    void track(
      "save",
      admin.updateSticker(sticker.id, { name, description, tags }).then(
        (catalog) => {
          onSaved(catalog);
          onClose();
          toast({ title: "Sticker saved", tone: "success" });
        },
        (error: Error) => {
          const named = fieldsOf(error);

          if (Object.keys(named).length > 0) {
            setFields(named);
          } else {
            adminFailure("Couldn't save the sticker", error);
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
      title={`Edit ${sticker.name}`}
      dirty={dirty}
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            form="admin-sticker-edit"
            variant="primary"
            loading={busy("save")}
            disabled={busy("save")}
          >
            Save
          </Button>
        </>
      }
    >
      <form id="admin-sticker-edit" className="settings-form" onSubmit={submit}>
        <FieldError message={elsewhere(fields)} />
        <StickerFields
          name={name}
          description={description}
          tags={tags}
          fields={fields}
          onName={setName}
          onDescription={setDescription}
          onTags={setTags}
        />
      </form>
    </Dialog>
  );
}

/**
 * The workspace's stickers, Discord's: images members send on their own in a message. Upload one
 * (a name, an optional description, search tags, and a PNG, APNG or GIF of at most 512 KB and
 * 320 × 320), edit its words or delete it; a deleted sticker's messages keep a placeholder with
 * its name. The workspace holds a limited number, shown as usage of the limit. Animated stickers
 * play here, on their first frame under reduced motion.
 */
export function StickersSection() {
  const { workspace } = useAdmin();
  const load = useStickerLoad();
  const [editing, setEditing] = useState<Sticker | null>(null);
  const [doomed, setDoomed] = useState<Sticker | null>(null);
  const { busy, track } = useBusy();
  const container = useRowFocus();

  useEffect(() => {
    if (workspace.canAdminister) void loadStickerCatalog(true).catch(() => undefined);
  }, [workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const refetch = () => void loadStickerCatalog(true).catch(() => undefined);

  const destroy = () => {
    const sticker = doomed;

    setDoomed(null);

    if (sticker === null) {
      return;
    }

    void track(
      `sticker-${sticker.id}`,
      admin
        .destroySticker(sticker.id)
        .then(replaceStickerCatalog, (error: Error) =>
          adminFailure(`Couldn't delete ${sticker.name}`, error),
        ),
    );
  };

  const catalog = load.status === "ready" ? load.catalog : null;
  const full = catalog !== null && catalog.stickers.length >= catalog.capacity;

  return (
    <SettingsPage title="Stickers" description="Images members can send on their own in a message.">
      <SettingsGroup
        title="Add a sticker"
        description={
          catalog === null ? undefined : stickerCapacity(catalog.stickers.length, catalog.capacity)
        }
      >
        <NewSticker
          full={full}
          capacity={catalog?.capacity ?? null}
          onSaved={replaceStickerCatalog}
          onRefused={refetch}
        />
      </SettingsGroup>
      <SettingsGroup title="Stickers">
        <div ref={container} tabIndex={-1} className="admin-focus-root">
          {load.status === "idle" || load.status === "loading" ? (
            <PaneListSkeleton rows={3} />
          ) : null}
          {load.status === "error" ? <PaneError message={load.message} onRetry={refetch} /> : null}
          {catalog !== null && catalog.stickers.length === 0 ? (
            <p className="text-muted">No stickers yet. Add one above.</p>
          ) : null}
          {catalog !== null && catalog.stickers.length > 0 ? (
            <ul className="settings-list" aria-label="Stickers">
              {catalog.stickers.map((sticker) => (
                <li
                  key={sticker.id}
                  className="settings-list-row admin-sticker-row"
                  data-row={sticker.id}
                  tabIndex={-1}
                >
                  <EmojiImage
                    className="admin-sticker-image"
                    src={sticker.imageUrl}
                    still={sticker.stillUrl}
                    width={48}
                    height={48}
                    loading="lazy"
                  />
                  <span className="settings-list-main">
                    <strong className="admin-sticker-name">
                      {sticker.name}
                      {sticker.animated ? <span className="settings-badge">Animated</span> : null}
                    </strong>
                    {sticker.description === "" ? null : (
                      <span className="text-muted admin-sticker-text">{sticker.description}</span>
                    )}
                    {sticker.tags === "" ? null : (
                      <span className="text-faint admin-sticker-text">Tags: {sticker.tags}</span>
                    )}
                  </span>
                  <span className="admin-sticker-actions">
                    <Button
                      variant="secondary"
                      size="sm"
                      icon="pencil"
                      data-row-control="edit"
                      disabled={busy(`sticker-${sticker.id}`)}
                      onClick={() => setEditing(sticker)}
                    >
                      <span className="visually-hidden">Edit {sticker.name}</span>
                    </Button>
                    <Button
                      variant="danger"
                      size="sm"
                      icon="trash"
                      data-row-control="delete"
                      disabled={busy(`sticker-${sticker.id}`)}
                      onClick={() => setDoomed(sticker)}
                    >
                      <span className="visually-hidden">Delete {sticker.name}</span>
                    </Button>
                  </span>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      </SettingsGroup>
      {editing === null ? null : (
        <EditSticker
          key={editing.id}
          sticker={editing}
          onClose={() => setEditing(null)}
          onSaved={replaceStickerCatalog}
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
        description="It leaves the picker. Messages that sent it show a placeholder with its name."
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
