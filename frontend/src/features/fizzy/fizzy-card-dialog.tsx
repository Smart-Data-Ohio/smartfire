import { Link } from "@tanstack/react-router";
import { type RefObject, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { CreatedFizzyCard } from "../../gen/CreatedFizzyCard.ts";
import type { FizzyMessageCardForm } from "../../gen/FizzyMessageCardForm.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog, focusOnOpen } from "../../ui/dialog.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { TextField } from "../../ui/text-field.tsx";
import {
  type CreateCard,
  classifyFailure,
  createBody,
  DESCRIPTION_MAX_LENGTH,
  type FieldErrors,
  type FizzyDraft,
  type FizzyField,
  type FizzyMessageScope,
  firstInvalid,
  hasErrors,
  initialDraft,
  type LoadedForm,
  localErrors,
  type ReadForm,
  readForm,
  TITLE_MAX_LENGTH,
  VALIDATION_SUMMARY,
} from "./fizzy-card-model.ts";
import "./fizzy.css";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly loaded: LoadedForm };

/** Where the dialog stands once the form is read. */
type Stage =
  | { readonly kind: "editing" }
  /** No usable connection: the source and the way to connect, with the reason when there's one. */
  | { readonly kind: "disconnected"; readonly reason: string | null }
  /** The card exists but its reply didn't post: show the card, never create it again. */
  | {
      readonly kind: "created";
      readonly message: string;
      readonly card: { readonly number: string; readonly url: string };
    };

export interface FizzyCardDialogProps {
  readonly scope: FizzyMessageScope;
  readonly open: boolean;
  readonly onClose: () => void;
  /** The card was created and its reply posted (the reply is already in the store). */
  readonly onCreated: (created: CreatedFizzyCard) => void;
  /** Where focus goes on close when the menu that opened the dialog is gone. */
  readonly returnFocus?: () => HTMLElement | null;
  /** Prefer `returnFocus` over the opener (a direct entry has no real opener). */
  readonly returnFocusFirst?: boolean;
  /** Reads the form; the runtime's action unless a test passes its own. */
  readonly read?: ReadForm;
  /** Creates the card; the runtime's action unless a test passes its own. */
  readonly create?: CreateCard;
}

const readWithRuntime: ReadForm = (scope) => actions.cards.fizzyForm(scope);

const createWithRuntime: CreateCard = (scope, body) => actions.cards.createFizzy(scope, body);

/** The element `ref` holds, when focus has nowhere useful to be (gone, or outside the dialog). */
function focusIfStranded(element: HTMLElement | null): void {
  if (element === null) {
    return;
  }

  const dialog = element.closest("dialog");
  const active = document.activeElement;

  if (
    active === null ||
    active === document.body ||
    !active.isConnected ||
    (dialog !== null && !dialog.contains(active))
  ) {
    element.focus();
  }
}

/**
 * "Create Fizzy card" on a message (the classic form page): the source, a board, a title and a
 * description prefilled from the message, created as the viewer in their Fizzy account, and then a
 * reply in the conversation linking the new card. Without a connection it shows the source and
 * the way to connect. A create is sent once per click and never retried: Fizzy may already hold
 * the card.
 */
export default function FizzyCardDialog({
  scope,
  open,
  onClose,
  onCreated,
  returnFocus,
  returnFocusFirst = false,
  read = readWithRuntime,
  create = createWithRuntime,
}: FizzyCardDialogProps) {
  const formId = useId();
  const boardId = useId();
  const descriptionId = useId();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [tries, setTries] = useState(0);
  const [draft, setDraft] = useState<FizzyDraft | null>(null);
  const [stage, setStage] = useState<Stage>({ kind: "editing" });
  const [errors, setErrors] = useState<FieldErrors>({});
  const [problem, setProblem] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const boardRef = useRef<HTMLSelectElement | null>(null);
  const titleRef = useRef<HTMLInputElement | null>(null);
  const closeRef = useRef<HTMLButtonElement | null>(null);
  const connectRef = useRef<HTMLAnchorElement | null>(null);
  const form = load.status === "ready" ? load.loaded.form : null;
  const editable = form !== null && draft !== null && stage.kind !== "disconnected";
  const start = form === null ? null : initialDraft(form);

  const dirty =
    draft !== null &&
    start !== null &&
    (draft.boardId !== start.boardId ||
      draft.title !== start.title ||
      draft.description !== start.description);

  // The overlay mounts the dialog afresh for each opening, so this reads the form once per
  // opening; a retry (`tries`) reads it again.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `scope` and `read` are fixed per opening
  useEffect(() => {
    if (!open) {
      return;
    }

    let live = true;

    setLoad({ status: "loading" });

    readForm(read, scope).then(
      (loaded) => {
        if (!live) return;

        setLoad({ status: "ready", loaded });
        setDraft(initialDraft(loaded.form));
        setStage(
          loaded.form.connected
            ? { kind: "editing" }
            : { kind: "disconnected", reason: loaded.notice },
        );
      },
      (failure: Error) => {
        if (live) setLoad({ status: "error", message: failure.message });
      },
    );

    return () => {
      live = false;
    };
  }, [open, tries]);

  // The dialog focused its close button while the form loaded: the board takes focus now.
  const fieldsShown = form?.connected === true;

  useLayoutEffect(() => {
    const select = boardRef.current;

    if (!fieldsShown || select === null) {
      return;
    }

    const active = document.activeElement;
    const inField = active instanceof Element && active.closest(".fz-form") !== null;

    if (!inField) {
      focusOnOpen(select);
    }
  }, [fieldsShown]);

  // The submit button went away with the form (disconnected) or the footer (created).
  useLayoutEffect(() => {
    if (stage.kind === "created") {
      focusIfStranded(closeRef.current);
    } else if (stage.kind === "disconnected") {
      focusIfStranded(connectRef.current);
    }
  }, [stage.kind]);

  const focusField = (field: FizzyField | null) => {
    if (field === "boardId") {
      boardRef.current?.focus();
    } else if (field === "title") {
      titleRef.current?.focus();
    }
  };

  const edit = (patch: Partial<FizzyDraft>) => {
    setDraft((current) => (current === null ? current : { ...current, ...patch }));

    const touched = Object.keys(patch);

    setErrors((current) =>
      Object.fromEntries(Object.entries(current).filter(([key]) => !touched.includes(key))),
    );
  };

  const invalid = (next: FieldErrors, summary: string) => {
    setErrors(next);
    setProblem(summary);
    setAttempt((count) => count + 1);
    focusField(firstInvalid(next));
  };

  const submit = () => {
    if (busy || draft === null || stage.kind !== "editing") {
      return;
    }

    const local = localErrors(draft);

    if (hasErrors(local)) {
      invalid(local, VALIDATION_SUMMARY);

      return;
    }

    setBusy(true);
    setProblem(null);

    create(scope, createBody(draft)).then(
      (created) => {
        setBusy(false);
        onCreated(created);
      },
      (failure: Error) => {
        setBusy(false);

        const outcome = classifyFailure(failure);

        switch (outcome.kind) {
          case "validation":
            invalid(outcome.errors, outcome.summary);

            return;
          case "disconnected":
            setStage({ kind: "disconnected", reason: outcome.message });

            return;
          case "created":
            setStage({ kind: "created", message: outcome.message, card: outcome.card });

            return;
          case "problem":
            setProblem(outcome.message);

            return;
        }
      },
    );
  };

  /** Esc, the close button and the backdrop wait for a create in flight: its outcome must show. */
  const requestClose = () => {
    if (!busy) onClose();
  };

  const footer = (
    <Footer
      stage={stage.kind}
      formId={formId}
      editable={editable}
      busy={busy}
      closeRef={closeRef}
      connectRef={connectRef}
      onClose={onClose}
      onCancel={requestClose}
    />
  );

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) requestClose();
      }}
      title="Create Fizzy card"
      description={
        form === null ? undefined : (
          <span className="fz-eyebrow">
            Create Fizzy card in <strong>{form.roomDisplayName}</strong>
          </span>
        )
      }
      size="md"
      dirty={dirty}
      footer={footer}
      {...(returnFocus === undefined ? {} : { returnFocus })}
      returnFocusFirst={returnFocusFirst}
    >
      {form === null || draft === null ? (
        <Pending load={load} onRetry={() => setTries((count) => count + 1)} />
      ) : (
        <div className="fz-dialog">
          <Source form={form} />
          {stage.kind === "disconnected" ? (
            <Disconnected reason={stage.reason} />
          ) : (
            <form
              id={formId}
              className="fz-form"
              noValidate
              onSubmit={(event) => {
                event.preventDefault();
                submit();
              }}
            >
              {problem === null ? null : (
                <div className="fz-problem enter-fade" role="alert">
                  <Icon name="circle-alert" size={16} />
                  <p>{problem}</p>
                </div>
              )}
              {stage.kind === "created" ? (
                <div className="fz-problem fz-created enter-fade" role="alert">
                  <Icon name="circle-alert" size={16} />
                  <div>
                    <p>{stage.message}</p>
                    <a href={stage.card.url} target="_blank" rel="noopener noreferrer">
                      Open Fizzy card #{stage.card.number}
                      <Icon name="external-link" size={12} />
                    </a>
                  </div>
                </div>
              ) : null}
              <fieldset className="fz-fields" disabled={stage.kind === "created"}>
                <div className={`field${errors.boardId === undefined ? "" : " is-error"}`}>
                  <label className="field-label" htmlFor={boardId}>
                    Board
                  </label>
                  <select
                    ref={boardRef}
                    id={boardId}
                    className="input fz-select"
                    value={draft.boardId}
                    required
                    aria-invalid={errors.boardId === undefined ? undefined : true}
                    aria-describedby={errors.boardId === undefined ? undefined : `${boardId}-error`}
                    onChange={(event) => edit({ boardId: event.target.value })}
                  >
                    <option value="">Choose a board</option>
                    {form.boards.map((board) => (
                      <option key={board.id} value={board.id}>
                        {board.name}
                      </option>
                    ))}
                  </select>
                  {errors.boardId === undefined ? null : (
                    <p id={`${boardId}-error`} className="field-error" role="alert">
                      {errors.boardId}
                    </p>
                  )}
                </div>
                <TextField
                  ref={titleRef}
                  label="Title"
                  value={draft.title}
                  placeholder="Card title"
                  autoComplete="off"
                  maxLength={TITLE_MAX_LENGTH}
                  required
                  error={errors.title}
                  attempt={attempt}
                  onChange={(event) => edit({ title: event.target.value })}
                />
                <div className="field">
                  <label className="field-label" htmlFor={descriptionId}>
                    Description
                  </label>
                  <textarea
                    id={descriptionId}
                    className="input fz-description"
                    value={draft.description}
                    rows={8}
                    maxLength={DESCRIPTION_MAX_LENGTH}
                    onChange={(event) => edit({ description: event.target.value })}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                        event.currentTarget.form?.requestSubmit();
                      }
                    }}
                  />
                </div>
              </fieldset>
              <p className="fz-note">
                Creates the card as {form.fizzyUserName} in {form.accountName}, then posts a reply
                here with the new card.
              </p>
            </form>
          )}
        </div>
      )}
    </Dialog>
  );
}

interface FooterProps {
  readonly stage: Stage["kind"];
  readonly formId: string;
  readonly editable: boolean;
  readonly busy: boolean;
  readonly closeRef: RefObject<HTMLButtonElement | null>;
  readonly connectRef: RefObject<HTMLAnchorElement | null>;
  readonly onClose: () => void;
  /** Cancel while editing: it waits for a create in flight. */
  readonly onCancel: () => void;
}

/** Create while editing; the way to connect when disconnected; only Close once the card exists. */
function Footer({
  stage,
  formId,
  editable,
  busy,
  closeRef,
  connectRef,
  onClose,
  onCancel,
}: FooterProps) {
  switch (stage) {
    case "created":
      return (
        <Button ref={closeRef} variant="primary" onClick={onClose}>
          Close
        </Button>
      );
    case "disconnected":
      return (
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Link
            ref={connectRef}
            to="/settings/integrations"
            hash="integration-fizzy"
            className="button"
            data-variant="primary"
            data-size="md"
          >
            Connect Fizzy on your profile
          </Link>
        </>
      );
    case "editing":
      return (
        <>
          <Button variant="secondary" onClick={onCancel}>
            Cancel
          </Button>
          <Button
            type="submit"
            form={formId}
            variant="primary"
            disabled={!editable}
            loading={busy}
            loadingLabel="Creating card…"
          >
            Create card
          </Button>
        </>
      );
  }
}

/** The message the card comes from, as classic quotes it. */
function Source({ form }: { readonly form: FizzyMessageCardForm }) {
  return (
    <blockquote className="fz-source">
      <p>{form.excerpt}</p>
      <cite>— {form.authorName}</cite>
    </blockquote>
  );
}

function Disconnected({ reason }: { readonly reason: string | null }) {
  return (
    <div className="fz-disconnected enter-fade" role="alert">
      <Icon name="lock" size={16} />
      <div>
        {reason === null ? null : <p className="fz-disconnected-reason">{reason}</p>}
        <p>
          Connect your Fizzy account first: card previews and creation use your own Fizzy access.
        </p>
      </div>
    </div>
  );
}

function Pending({ load, onRetry }: { readonly load: Load; readonly onRetry: () => void }) {
  if (load.status === "error") {
    return (
      <div className="fz-load-error" role="alert">
        <p>Couldn't load the form: {load.message}</p>
        <Button variant="secondary" size="sm" icon="refresh-cw" onClick={onRetry}>
          Try again
        </Button>
      </div>
    );
  }

  return (
    <div className="fz-skeleton" role="status" aria-busy="true" aria-label="Loading the form">
      <Skeleton width="100%" height={56} radius="md" />
      <Skeleton width="20%" height={12} />
      <Skeleton width="100%" height={36} radius="md" />
      <Skeleton width="20%" height={12} />
      <Skeleton width="100%" height={36} radius="md" />
      <Skeleton width="100%" height={120} radius="md" />
    </div>
  );
}
