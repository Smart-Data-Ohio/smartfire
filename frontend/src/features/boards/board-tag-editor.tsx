import {
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import type { BoardTag } from "../../gen/BoardTag.ts";
import type { BoardTagCatalog } from "../../gen/BoardTagCatalog.ts";
import { LazyEmojiPicker, preloadEmojiPicker } from "../../lib/emoji/lazy-emoji-picker.tsx";
import { store, useStore } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Menu, MenuItem, MenuSeparator } from "../../ui/menu.tsx";
import { Popover } from "../../ui/popover.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";
import { useAnnouncer } from "../destinations/live-region.tsx";
import {
  deleteConsequences,
  MAX_BOARD_TAG_NAME,
  MAX_BOARD_TAGS,
  moveId,
  postsUsing,
  tagNameProblem,
} from "./board-tags.ts";
import { refreshBoardTags, useBoardTagCatalog } from "./use-board-tags.ts";
import { useLiveFlip } from "./use-live-flip.ts";
import "./boards.css";

type Load = "loading" | "ready" | "error";

/** A refusal's words for the field it names ("Name has already been taken."), else its message. */
function refusalText(error: Error): string {
  if (error instanceof ActionError) {
    const named = [
      ...(error.fields.name ?? []).map((message) => `Name ${message}.`),
      ...(error.fields.tags ?? []).map((message) => `Tags ${message}.`),
    ];

    if (named.length > 0) return named.join(" ");
  }

  return error.message;
}

/**
 * A tag's emoji as a square well (the room icon's, smaller): the emoji, or a faint smile until
 * one is picked; the shared picker opens in a popover, with "Remove emoji" once there is one.
 */
function EmojiWell({
  emoji,
  label,
  disabled = false,
  onPick,
}: {
  readonly emoji: string | null;
  readonly label: string;
  readonly disabled?: boolean;
  readonly onPick: (emoji: string | null) => void;
}) {
  return (
    <Popover
      label={label}
      placement="bottom-start"
      trigger={(props) => (
        <button
          {...props}
          type="button"
          className="tag-editor-emoji"
          aria-label={emoji === null ? label : `${label} (${emoji})`}
          data-empty={emoji === null || undefined}
          disabled={disabled}
          onPointerEnter={preloadEmojiPicker}
          onFocus={preloadEmojiPicker}
        >
          {emoji === null ? <Icon name="smile-plus" size={16} /> : <span>{emoji}</span>}
        </button>
      )}
    >
      {(close) => (
        <div className="room-icon-popover">
          <LazyEmojiPicker
            onPick={(choice) => {
              close();

              // Tags carry plain emoji: a workspace icon's `:name:` has no image outside messages.
              if (choice.imageUrl === null) onPick(choice.content);
            }}
          />
          {emoji === null ? null : (
            <div className="room-icon-popover-footer">
              <Button
                variant="ghost"
                size="sm"
                icon="x"
                onClick={() => {
                  close();
                  onPick(null);
                }}
              >
                Remove emoji
              </Button>
            </div>
          )}
        </div>
      )}
    </Popover>
  );
}

function RenameField({
  tag,
  tags,
  onSave,
  onCancel,
}: {
  readonly tag: BoardTag;
  readonly tags: readonly BoardTag[];
  readonly onSave: (name: string) => Promise<BoardTagCatalog>;
  readonly onCancel: () => void;
}) {
  const [name, setName] = useState(tag.name);
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);

  const save = () => {
    if (busy) return;

    if (name.trim() === tag.name) {
      onCancel();

      return;
    }

    const problem = tagNameProblem(tags, name, tag.id);

    if (problem !== undefined) {
      setError(problem);
      setAttempt((count) => count + 1);

      return;
    }

    setBusy(true);
    onSave(name.trim()).then(onCancel, (failure: Error) => {
      setBusy(false);
      setError(refusalText(failure));
      setAttempt((count) => count + 1);
    });
  };

  return (
    <div className="tag-editor-rename">
      <TextField
        label={`Rename ${tag.name}`}
        className="tag-editor-rename-input"
        value={name}
        maxLength={MAX_BOARD_TAG_NAME + 10}
        autoComplete="off"
        error={error}
        attempt={attempt}
        autoFocus
        disabled={busy}
        onChange={(event) => {
          setName(event.target.value);
          setError(undefined);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            save();
          } else if (event.key === "Escape") {
            // Escape leaves the rename, not the settings dialog around it.
            event.preventDefault();
            event.stopPropagation();
            onCancel();
          }
        }}
      />
      <div className="tag-editor-rename-actions">
        <Button variant="secondary" size="sm" onClick={onCancel} disabled={busy}>
          Cancel
        </Button>
        <Button variant="primary" size="sm" loading={busy} onClick={save}>
          Save
        </Button>
      </div>
    </div>
  );
}

/** How far each key moves a focused handle's tag (Home and End: to either end). */
const HANDLE_STEPS = new Map([
  ["ArrowUp", -1],
  ["ArrowDown", 1],
  ["Home", Number.NEGATIVE_INFINITY],
  ["End", Number.POSITIVE_INFINITY],
]);

interface Drag {
  readonly id: number;
  readonly pointerId: number;
  /** The order when the drag began, put back if it's cancelled. */
  readonly before: readonly number[];
}

function EditorSkeleton() {
  return (
    <div className="tag-editor" aria-hidden="true">
      {[56, 72, 44].map((width) => (
        <div key={width} className="tag-editor-row">
          <Skeleton width={32} height={32} radius="md" />
          <Skeleton width={`${width}%`} height={12} />
        </div>
      ))}
    </div>
  );
}

interface PolicyPatch {
  readonly tagsRequired?: boolean;
  readonly defaultBoardTagId?: number | null;
}

/**
 * A board's curated tags (Discord's forum tags), for its creator and administrators: add one
 * with an emoji, rename it, change its emoji, drag it (or move it with the arrow keys) into
 * place, delete it after a confirmation that says what it does to posts, and set the policy:
 * whether new posts need a tag, and which one they start with. Every change saves at once, in
 * the order made, and the catalog follows anyone else's changes live.
 */
export function BoardTagEditor({ roomId }: { readonly roomId: number }) {
  const held = useBoardTagCatalog(roomId);
  const counts = useStore((state) => state.boards[roomId]?.tagCounts);
  const [load, setLoad] = useState<Load>(held === undefined ? "loading" : "ready");
  const [retry, setRetry] = useState(0);
  const [order, setOrder] = useState<readonly number[] | null>(null);
  const [pending, setPending] = useState(0);
  const [renaming, setRenaming] = useState<number | null>(null);
  const [deleting, setDeleting] = useState<BoardTag | null>(null);
  const [removing, setRemoving] = useState(false);
  const [dragging, setDragging] = useState<number | null>(null);
  const [newName, setNewName] = useState("");
  const [newEmoji, setNewEmoji] = useState<string | null>(null);
  const [addError, setAddError] = useState<string | undefined>(undefined);
  const [addAttempt, setAddAttempt] = useState(0);
  const [adding, setAdding] = useState(false);
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const pendingRef = useRef(0);
  const drag = useRef<Drag | null>(null);
  const listRef = useRef<HTMLOListElement | null>(null);
  const newNameRef = useRef<HTMLInputElement | null>(null);
  /** The last queued write's answer, while more are queued behind it. */
  const answered = useRef<BoardTagCatalog | undefined>(undefined);
  /** Policy changes made but not yet answered, shown over the catalog meanwhile. */
  const [policyDraft, setPolicyDraft] = useState<PolicyPatch>({});
  /** The row the delete confirmation was opened from, for focus to land near when it closes. */
  const confirmedAt = useRef(0);
  const handles = useRef(new Map<number, HTMLButtonElement>());
  const { announce, region } = useAnnouncer("assertive");
  const instructionsId = useId();
  const defaultId = useId();

  // biome-ignore lint/correctness/useExhaustiveDependencies: retry is the Try again trigger
  useEffect(() => {
    let live = true;

    actions.boards.tagCatalog(roomId).then(
      () => {
        if (live) setLoad("ready");
      },
      () => {
        if (live) setLoad((current) => (current === "ready" ? current : "error"));
      },
    );

    return () => {
      live = false;
    };
  }, [roomId, retry]);

  const catalog: BoardTagCatalog | undefined = held;
  const tags = catalog?.tags ?? [];
  const byId = new Map(tags.map((tag) => [tag.id, tag]));
  // The viewer's own reorder shows at once; the server's order takes over once it has answered.
  const ids = (order ?? tags.map((tag) => tag.id)).filter((id) => byId.has(id));

  for (const tag of tags) if (!ids.includes(tag.id)) ids.push(tag.id);

  const shown = ids.flatMap((id) => {
    const tag = byId.get(id);

    return tag === undefined ? [] : [tag];
  });

  useLiveFlip(listRef, ids.join(","), String(roomId));

  /**
   * Runs `write` after every change made before it, so the server sees them in order. It gets
   * the catalog as of the change before it (that write's answer, else the store's), so a field
   * it doesn't change goes out as the server now has it, not as it was when the change was made.
   */
  const enqueue = (
    write: (base: BoardTagCatalog | undefined) => Promise<BoardTagCatalog>,
  ): Promise<BoardTagCatalog> => {
    pendingRef.current += 1;
    setPending((count) => count + 1);

    const result = queue.current.then(() =>
      write(answered.current ?? store.getState().boardTagCatalogs[roomId]).then((answer) => {
        answered.current = answer;

        return answer;
      }),
    );

    queue.current = result
      .catch(() => undefined)
      .finally(() => {
        pendingRef.current -= 1;
        setPending((count) => count - 1);

        if (pendingRef.current === 0) {
          answered.current = undefined;
          setOrder(null);
          setPolicyDraft({});
        }
      });

    return result;
  };

  const failed = (title: string) => (failure: Error) => {
    toast({ title, description: refusalText(failure), tone: "danger" });
    // Whatever the change was, show the catalog as the server now has it.
    answered.current = undefined;
    setOrder(null);
    setPolicyDraft({});
    refreshBoardTags(roomId);
  };

  const saveOrder = (next: readonly number[]) => {
    enqueue(() => actions.boards.reorderTags(roomId, next)).catch(
      failed("Couldn't save the new order"),
    );
  };

  const move = (id: number, by: number) => {
    const from = ids.indexOf(id);
    const to = Math.max(0, Math.min(ids.length - 1, from + by));

    if (from === -1 || from === to) return;

    const next = moveId(ids, from, to);
    const tag = byId.get(id);

    setOrder(next);
    saveOrder(next);
    announce(`${tag?.name ?? "Tag"} moved to position ${to + 1} of ${next.length}.`);
    // The row re-renders in its new place; its handle keeps the focus.
    requestAnimationFrame(() => handles.current.get(id)?.focus());
  };

  const onHandleKey = (event: KeyboardEvent<HTMLButtonElement>, id: number) => {
    const step = HANDLE_STEPS.get(event.key);

    if (step === undefined) return;

    event.preventDefault();
    move(id, step);
  };

  const onPointerDown = (event: ReactPointerEvent<HTMLButtonElement>, id: number) => {
    if (event.button !== 0 || drag.current !== null) return;

    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { id, pointerId: event.pointerId, before: ids };
    setDragging(id);
  };

  const onPointerMove = (event: ReactPointerEvent<HTMLButtonElement>) => {
    const active = drag.current;
    const list = listRef.current;

    if (active === null || active.pointerId !== event.pointerId || list === null) return;

    // The dragged row goes before the first other row whose middle is below the pointer.
    const rows = [...list.querySelectorAll<HTMLElement>("[data-tag-id]")].filter(
      (row) => Number(row.dataset.tagId) !== active.id,
    );

    const below = rows.findIndex((row) => {
      const box = row.getBoundingClientRect();

      return event.clientY < box.top + box.height / 2;
    });

    const target = below === -1 ? rows.length : below;
    const from = ids.indexOf(active.id);

    if (from !== target) setOrder(moveId(ids, from, target));
  };

  const endDrag = (event: ReactPointerEvent<HTMLButtonElement>, cancelled: boolean) => {
    const active = drag.current;

    if (active === null || active.pointerId !== event.pointerId) return;

    drag.current = null;
    setDragging(null);

    if (cancelled) {
      setOrder(null);

      return;
    }

    if (ids.join(",") !== active.before.join(",")) {
      const tag = byId.get(active.id);

      saveOrder(ids);
      announce(`${tag?.name ?? "Tag"} moved to position ${ids.indexOf(active.id) + 1}.`);
    } else {
      setOrder(null);
    }
  };

  const saveTag = (tag: BoardTag, patch: { name?: string; emoji?: string | null }) =>
    enqueue((base) => {
      // An earlier queued rename or emoji change has landed by now: keep it.
      const now = base?.tags.find((held) => held.id === tag.id) ?? tag;

      return actions.boards.updateTag(roomId, tag.id, {
        name: patch.name ?? now.name,
        emoji: patch.emoji === undefined ? now.emoji : patch.emoji,
      });
    });

  const savePolicy = (patch: PolicyPatch) => {
    if (catalog === undefined) return;

    setPolicyDraft((draft) => ({ ...draft, ...patch }));
    enqueue((base = catalog) =>
      actions.boards.updateTagPolicy(roomId, {
        tagsRequired: patch.tagsRequired ?? base.tagsRequired,
        defaultBoardTagId:
          patch.defaultBoardTagId === undefined ? base.defaultBoardTagId : patch.defaultBoardTagId,
      }),
    ).catch(failed("Couldn't save the tag rules"));
  };

  const add = () => {
    if (adding) return;

    const problem = tagNameProblem(tags, newName);

    if (problem !== undefined) {
      setAddError(problem);
      setAddAttempt((count) => count + 1);

      return;
    }

    const name = newName.trim();

    setAdding(true);
    enqueue(() => actions.boards.createTag(roomId, { name, emoji: newEmoji })).then(
      () => {
        setAdding(false);
        setNewName("");
        setNewEmoji(null);
        setAddError(undefined);
        announce(`Added ${name}.`);
      },
      (failure: Error) => {
        setAdding(false);
        setAddError(refusalText(failure));
        setAddAttempt((count) => count + 1);
      },
    );
  };

  const remove = () => {
    if (deleting === null || catalog === undefined) return;

    const tag = deleting;

    setRemoving(true);
    // The server lifts a requirement the last tag held up in the same write, and answers with it.
    enqueue(() => actions.boards.deleteTag(roomId, tag.id))
      .then(() => {
        setRemoving(false);
        setDeleting(null);
        announce(`Deleted ${tag.name}.`);
      })
      .catch((failure: Error) => {
        setRemoving(false);
        setDeleting(null);
        failed(`Couldn't delete ${tag.name}`)(failure);
      });
  };

  if (catalog === undefined) {
    return load === "error" ? (
      <div className="room-form-load-error">
        <p className="picker-note picker-error" role="alert">
          Couldn't load the board's tags.
        </p>
        <Button
          size="sm"
          onClick={() => {
            setLoad("loading");
            setRetry((count) => count + 1);
          }}
        >
          Try again
        </Button>
      </div>
    ) : (
      <EditorSkeleton />
    );
  }

  const full = tags.length >= MAX_BOARD_TAGS;
  const shownRequired = policyDraft.tagsRequired ?? catalog.tagsRequired;

  const shownDefault =
    policyDraft.defaultBoardTagId === undefined
      ? catalog.defaultBoardTagId
      : policyDraft.defaultBoardTagId;

  const consequences =
    deleting === null
      ? null
      : deleteConsequences(catalog, deleting, postsUsing(counts, deleting.name));

  return (
    <div className="tag-editor" data-room-integration="">
      <p className="room-integration-note">
        Tags people pick when they start a post, shown with their emoji on the board and in its
        filter. Posts keep any other tags they already have.
      </p>
      <p id={instructionsId} className="visually-hidden">
        Drag the handle, or focus it and press the up or down arrow, to move a tag.
      </p>
      {shown.length === 0 ? (
        <p className="tag-editor-empty">
          <Icon name="hash" size={16} />
          No tags yet. Add the first below.
        </p>
      ) : (
        <ol className="tag-editor-list" ref={listRef} aria-label="Board tags">
          {shown.map((tag, index) => {
            const isDefault = shownDefault === tag.id;

            return (
              <li
                key={tag.id}
                className="tag-editor-row"
                data-flip={tag.id}
                data-tag-id={tag.id}
                data-dragging={dragging === tag.id || undefined}
              >
                <button
                  type="button"
                  className="tag-editor-handle"
                  aria-label={`Move ${tag.name}, position ${index + 1} of ${shown.length}`}
                  aria-describedby={instructionsId}
                  ref={(element) => {
                    if (element === null) handles.current.delete(tag.id);
                    else handles.current.set(tag.id, element);
                  }}
                  onKeyDown={(event) => onHandleKey(event, tag.id)}
                  onPointerDown={(event) => onPointerDown(event, tag.id)}
                  onPointerMove={onPointerMove}
                  onPointerUp={(event) => endDrag(event, false)}
                  onPointerCancel={(event) => endDrag(event, true)}
                >
                  <Icon name="grip-vertical" size={16} />
                </button>
                <EmojiWell
                  emoji={tag.emoji}
                  label={`Emoji for ${tag.name}`}
                  onPick={(emoji) => {
                    saveTag(tag, { emoji }).catch(failed(`Couldn't change ${tag.name}'s emoji`));
                  }}
                />
                {renaming === tag.id ? (
                  <RenameField
                    tag={tag}
                    tags={tags}
                    onSave={(name) => saveTag(tag, { name })}
                    onCancel={() => {
                      setRenaming(null);
                      requestAnimationFrame(() => handles.current.get(tag.id)?.focus());
                    }}
                  />
                ) : (
                  <span className="tag-editor-name">
                    <span className="tag-editor-label">{tag.name}</span>
                    {isDefault ? <span className="tag-editor-badge">Default</span> : null}
                  </span>
                )}
                {renaming === tag.id ? null : (
                  <Menu
                    placement="bottom-end"
                    label={`${tag.name} options`}
                    trigger={(props) => (
                      <IconButton
                        {...props}
                        icon="more"
                        label={`${tag.name} options`}
                        className="tag-editor-more"
                      />
                    )}
                  >
                    <MenuItem icon="pencil" onSelect={() => setRenaming(tag.id)}>
                      Rename
                    </MenuItem>
                    <MenuItem
                      icon={isDefault ? "star-off" : "star"}
                      onSelect={() => savePolicy({ defaultBoardTagId: isDefault ? null : tag.id })}
                    >
                      {isDefault ? "Stop starting posts with it" : "Start new posts with it"}
                    </MenuItem>
                    <MenuItem disabled={index === 0} onSelect={() => move(tag.id, -1)}>
                      Move up
                    </MenuItem>
                    <MenuItem
                      icon="arrow-down"
                      disabled={index === shown.length - 1}
                      onSelect={() => move(tag.id, 1)}
                    >
                      Move down
                    </MenuItem>
                    <MenuSeparator />
                    <MenuItem
                      icon="trash"
                      tone="danger"
                      onSelect={() => {
                        confirmedAt.current = index;
                        setDeleting(tag);
                      }}
                    >
                      Delete…
                    </MenuItem>
                  </Menu>
                )}
              </li>
            );
          })}
        </ol>
      )}
      <section className="tag-editor-add" aria-label="Add a tag">
        <EmojiWell
          emoji={newEmoji}
          label="Emoji for the new tag"
          disabled={full}
          onPick={setNewEmoji}
        />
        <TextField
          ref={newNameRef}
          label="New tag"
          className="tag-editor-add-input"
          placeholder={full ? `${MAX_BOARD_TAGS} tags is the most` : "Bug, Idea, Shipped…"}
          value={newName}
          maxLength={MAX_BOARD_TAG_NAME + 10}
          autoComplete="off"
          disabled={full}
          error={addError}
          attempt={addAttempt}
          hint={`${tags.length} of ${MAX_BOARD_TAGS}`}
          onChange={(event) => {
            setNewName(event.target.value);
            setAddError(undefined);
          }}
          onKeyDown={(event) => {
            // Enter adds the tag; it mustn't submit the settings form around it.
            if (event.key === "Enter") {
              event.preventDefault();
              add();
            }
          }}
        />
        <Button
          variant="secondary"
          icon="plus"
          className="tag-editor-add-button"
          disabled={full}
          loading={adding}
          onClick={add}
        >
          Add tag
        </Button>
      </section>
      <section className="tag-editor-policy" aria-label="Tag rules">
        <Toggle
          checked={shownRequired}
          disabled={tags.length === 0 && !shownRequired}
          onCheckedChange={(tagsRequired) => savePolicy({ tagsRequired })}
          label="Require a tag on new posts"
          description={
            tags.length === 0
              ? "Add a tag first."
              : "Every new post needs at least one of these tags."
          }
        />
        <div className="field tag-editor-default">
          <label className="field-label" htmlFor={defaultId}>
            Start new posts with
          </label>
          <select
            id={defaultId}
            className="input board-select"
            value={shownDefault ?? ""}
            disabled={tags.length === 0}
            aria-describedby={`${defaultId}-hint`}
            onChange={(event) =>
              savePolicy({
                defaultBoardTagId: event.target.value === "" ? null : Number(event.target.value),
              })
            }
          >
            <option value="">No tag</option>
            {tags.map((tag) => (
              <option key={tag.id} value={tag.id}>
                {tag.emoji === null ? tag.name : `${tag.emoji} ${tag.name}`}
              </option>
            ))}
          </select>
          <p id={`${defaultId}-hint`} className="field-hint">
            Picked to start with on every new post.
            {shownRequired ? " A post sent without a tag gets it." : ""}
          </p>
        </div>
        {pending > 0 ? (
          <span className="tag-editor-saving" role="status">
            Saving…
          </span>
        ) : null}
      </section>
      {region}
      <Dialog
        open={deleting !== null}
        onOpenChange={(next) => (removing || next ? undefined : setDeleting(null))}
        role="alertdialog"
        size="sm"
        returnFocus={() => {
          // The menu that opened this is gone, and after a delete its row too: the handle now in
          // that place (the next row's, or the last), else the New tag field.
          const id = ids[Math.min(confirmedAt.current, ids.length - 1)];

          return (id === undefined ? undefined : handles.current.get(id)) ?? newNameRef.current;
        }}
        title={`Delete “${deleting?.name ?? "this tag"}”?`}
        description={
          consequences === null ? undefined : (
            <span className="tag-editor-consequences">
              {consequences.lines.map((line) => (
                <span key={line}>{line}</span>
              ))}
              <span>This can't be undone.</span>
            </span>
          )
        }
        footer={
          <>
            <Button variant="secondary" onClick={() => setDeleting(null)} data-autofocus>
              Keep it
            </Button>
            <Button variant="danger" loading={removing} loadingLabel="Deleting…" onClick={remove}>
              Delete tag
            </Button>
          </>
        }
      />
    </div>
  );
}
