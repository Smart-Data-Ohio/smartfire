import { useEffect, useId, useRef, useState } from "react";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import { threadTitle } from "../threads/thread-format.ts";
import {
  EMPTY_DRAFT,
  type HandoffDraft,
  type HandoffField,
  type HandoffProblems,
  handoffBody,
  localProblems,
  SUMMARY_MAX,
  serverProblems,
} from "./handoff-form.ts";
import "./handoff.css";

/** The summary's counter shows from here on, so it says something only near the limit. */
const COUNTER_FROM = SUMMARY_MAX - 200;

/** The field order: the first one in trouble takes focus after a refused submit. */
const ORDER: readonly HandoffField[] = ["receiverAgentId", "summary", "links", "openQuestions"];

interface AreaFieldProps {
  readonly label: string;
  readonly hint?: string;
  readonly value: string;
  readonly error: string | undefined;
  readonly placeholder: string;
  readonly rows: number;
  readonly disabled: boolean;
  readonly maxLength?: number;
  readonly className?: string;
  readonly counter?: string | null;
  readonly counterOver?: boolean;
  readonly name: HandoffField;
  /** Takes focus when the dialog opens. */
  readonly autofocus?: boolean;
  readonly onChange: (value: string) => void;
  readonly onSubmit: () => void;
}

/** A labelled textarea with a hint, an error under it and, optionally, a counter by its label. */
function AreaField({
  label,
  hint,
  value,
  error,
  placeholder,
  rows,
  disabled,
  maxLength,
  className,
  counter = null,
  counterOver = false,
  name,
  autofocus = false,
  onChange,
  onSubmit,
}: AreaFieldProps) {
  const id = useId();
  const invalid = error !== undefined;

  return (
    <div className={`field t-input-wrap${invalid ? " is-error" : ""}`}>
      <div className="handoff-label-row">
        <label className="field-label" htmlFor={id}>
          {label}
        </label>
        {counter === null ? null : (
          <span className="handoff-counter" data-over={counterOver || undefined} aria-hidden="true">
            {counter}
          </span>
        )}
      </div>
      <textarea
        id={id}
        name={name}
        className={`input handoff-textarea${className === undefined ? "" : ` ${className}`}${
          invalid ? " is-error" : ""
        }`}
        value={value}
        rows={rows}
        placeholder={placeholder}
        disabled={disabled}
        data-autofocus={autofocus || undefined}
        {...(maxLength === undefined ? {} : { maxLength })}
        aria-invalid={invalid || undefined}
        aria-describedby={`${id}-${invalid ? "error" : "hint"}`}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
            event.preventDefault();
            onSubmit();
          }
        }}
      />
      {hint === undefined || invalid ? null : (
        <p id={`${id}-hint`} className="field-hint">
          {hint}
        </p>
      )}
      <p id={`${id}-error`} className="field-error t-error-msg" aria-live="polite">
        {error ?? ""}
      </p>
    </div>
  );
}

interface HandoffDialogProps {
  readonly threadId: number;
  readonly open: boolean;
  /** Back to the thread: after Cancel, a handoff, or when this work can't be handed off. */
  readonly onClose: () => void;
}

/**
 * "Hand off" (classic `threads/work/handoffs#new`): ownership of tracked work moves to an agent in
 * the room, with a summary of where it stands, links and open questions as its context package.
 * The receivers are the thread detail's `handoffReceivers`; the reply installs the updated thread
 * and says who has it now. Someone who can't manage the work (or a thread that isn't tracked) is
 * sent back to the thread.
 */
export function HandoffDialog({ threadId, open, onClose }: HandoffDialogProps) {
  const thread = useStore((state) => state.threads[threadId]);
  const paneStatus = useStore((state) => state.threadPanes[threadId]?.status ?? "loading");
  const canManage = useStore((state) => state.threadPanes[threadId]?.permissions?.canManageWork);
  const receivers = useStore((state) => state.threadPanes[threadId]?.work?.handoffReceivers);
  const users = useStore((state) => state.users);
  const [draft, setDraft] = useState<HandoffDraft>(EMPTY_DRAFT);
  const [problems, setProblems] = useState<HandoffProblems>({});
  const [saving, setSaving] = useState(false);
  const formRef = useRef<HTMLFormElement | null>(null);
  const selectId = useId();
  // One opening of the dialog: a reply landing after it closed must not touch the next draft.
  const openingRef = useRef(0);
  const savingRef = useRef(false);
  // Sent back once per opening, however often the store changes before the route does.
  const refusedRef = useRef(false);
  const tracked = thread === undefined ? null : thread.work !== null;
  const ready = paneStatus === "ready" && receivers !== undefined;
  const choices = receivers ?? [];
  const onlyChoice = choices.length === 1 ? String(choices[0]?.agentId) : null;

  // biome-ignore lint/correctness/useExhaustiveDependencies: each opening starts a fresh draft
  useEffect(() => {
    openingRef.current += 1;
    savingRef.current = false;
    refusedRef.current = false;
    setSaving(false);
    setDraft(EMPTY_DRAFT);
    setProblems({});

    return () => {
      openingRef.current += 1;
    };
  }, [open, threadId]);

  // A lone receiver is the only answer: choose it.
  useEffect(() => {
    if (open && onlyChoice !== null) {
      setDraft((previous) =>
        previous.receiverAgentId === "" ? { ...previous, receiverAgentId: onlyChoice } : previous,
      );
    }
  }, [open, onlyChoice]);

  // Work this viewer can't hand off, opened by its URL: say why and go back to the thread.
  useEffect(() => {
    if (!open || savingRef.current || refusedRef.current || paneStatus !== "ready") {
      return;
    }

    const refusal =
      tracked === false
        ? "This thread isn't tracked as work"
        : canManage === false
          ? "You cannot manage work in this thread"
          : null;

    if (refusal !== null) {
      refusedRef.current = true;
      toast({ title: refusal, tone: "danger" });
      onClose();
    }
  }, [open, paneStatus, tracked, canManage, onClose]);

  const set = (patch: Partial<HandoffDraft>) => {
    setDraft((previous) => ({ ...previous, ...patch }));

    // Editing a field clears its error, and the form's alert.
    setProblems((previous) =>
      Object.fromEntries(
        Object.entries(previous).filter(([field]) => field !== "base" && !(field in patch)),
      ),
    );
  };

  const focusFirst = (found: HandoffProblems) => {
    const field = ORDER.find((each) => found[each] !== undefined);

    if (field !== undefined) {
      requestAnimationFrame(() =>
        formRef.current?.querySelector<HTMLElement>(`[name="${field}"]`)?.focus(),
      );
    }
  };

  const submit = () => {
    if (savingRef.current || !ready) {
      return;
    }

    const local = localProblems(draft);

    if (Object.keys(local).length > 0) {
      setProblems(local);
      focusFirst(local);

      return;
    }

    const opening = openingRef.current;
    const current = () => openingRef.current === opening;
    const body = handoffBody(draft);
    const receiver = choices.find((choice) => choice.agentId === body.receiverAgentId);

    const name =
      receiver === undefined ? "the agent" : (users[receiver.userId]?.name ?? "the agent");

    savingRef.current = true;
    setSaving(true);
    setProblems({});
    actions.work.handoff(threadId, body).then(
      () => {
        if (!current()) {
          return;
        }

        toast({ title: `Work handed off to ${name}.`, tone: "success" });
        onClose();
      },
      (error: Error) => {
        if (!current()) {
          return;
        }

        const found = serverProblems(error);

        savingRef.current = false;
        setSaving(false);
        setProblems(found);
        focusFirst(found);
      },
    );
  };

  const close = (next: boolean) => {
    if (!next) {
      onClose();
    }
  };

  const title = thread === undefined ? "Hand off work" : `Hand off “${threadTitle(thread)}”`;
  const length = [...draft.summary].length;
  const noReceivers = ready && choices.length === 0;

  return (
    <Dialog
      open={open}
      onOpenChange={close}
      title={title}
      description="Ownership moves to the receiving agent, the handoff is recorded in Work history, and the agent gets the context package below through its event feed."
      footer={
        <>
          <Button variant="secondary" onClick={() => close(false)}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon="send"
            loading={saving}
            loadingLabel="Handing off…"
            disabled={!ready || noReceivers}
            onClick={submit}
          >
            Hand off
          </Button>
        </>
      }
    >
      <form
        ref={formRef}
        className="handoff-form"
        aria-label="Handoff"
        aria-busy={saving || undefined}
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        {problems.base === undefined ? null : (
          <p className="handoff-alert" role="alert">
            <Icon name="alert" size={14} />
            {problems.base}
          </p>
        )}
        <div
          className={`field t-input-wrap${problems.receiverAgentId === undefined ? "" : " is-error"}`}
        >
          <label className="field-label" htmlFor={selectId}>
            Receiving agent
          </label>
          <select
            id={selectId}
            name="receiverAgentId"
            className={`input handoff-select${problems.receiverAgentId === undefined ? "" : " is-error"}`}
            value={draft.receiverAgentId}
            disabled={saving || !ready || noReceivers}
            required
            data-autofocus={onlyChoice === null ? true : undefined}
            aria-invalid={problems.receiverAgentId === undefined ? undefined : true}
            aria-describedby={`${selectId}-${problems.receiverAgentId === undefined ? "hint" : "error"}`}
            onChange={(event) => set({ receiverAgentId: event.target.value })}
          >
            <option value="">{ready ? "Choose an agent" : "Loading agents…"}</option>
            {choices.map((choice) => (
              <option key={choice.agentId} value={choice.agentId}>
                {users[choice.userId]?.name ?? "Agent"}
              </option>
            ))}
          </select>
          {noReceivers && problems.receiverAgentId === undefined ? (
            <p id={`${selectId}-hint`} className="field-hint">
              No agent here can take this work. An agent needs to be in this room and allowed to
              post, manage threads and read messages.
            </p>
          ) : null}
          <p id={`${selectId}-error`} className="field-error t-error-msg" aria-live="polite">
            {problems.receiverAgentId ?? ""}
          </p>
        </div>
        <AreaField
          name="summary"
          label="Summary"
          value={draft.summary}
          error={problems.summary}
          placeholder="Where the work stands and what is next…"
          rows={6}
          maxLength={SUMMARY_MAX}
          className="handoff-summary"
          autofocus={onlyChoice !== null}
          counter={length >= COUNTER_FROM ? `${length} / ${SUMMARY_MAX}` : null}
          counterOver={length >= SUMMARY_MAX}
          disabled={saving}
          onChange={(summary) => set({ summary })}
          onSubmit={submit}
        />
        <AreaField
          name="links"
          label="Links"
          hint="One URL per line, up to 10."
          value={draft.links}
          error={problems.links}
          placeholder="https://example.com/spec"
          rows={3}
          disabled={saving}
          onChange={(links) => set({ links })}
          onSubmit={submit}
        />
        <AreaField
          name="openQuestions"
          label="Open questions"
          hint="One per line, up to 10."
          value={draft.openQuestions}
          error={problems.openQuestions}
          placeholder="Which approach did we agree on?"
          rows={3}
          disabled={saving}
          onChange={(openQuestions) => set({ openQuestions })}
          onSubmit={submit}
        />
      </form>
    </Dialog>
  );
}
