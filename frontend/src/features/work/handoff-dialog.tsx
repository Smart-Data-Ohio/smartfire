import { useMatchRoute, useNavigate } from "@tanstack/react-router";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { WorkDetail } from "../../gen/WorkDetail.ts";
import { parseBoardSearch } from "../../lib/board-search.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { toast } from "../../ui/toast-store.ts";
import { UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import {
  checkHandoff,
  EMPTY_HANDOFF,
  HANDOFF_ITEM_LIMIT,
  type HandoffDraft,
  type HandoffErrors,
  type HandoffField,
  SUMMARY_LIMIT,
  serverHandoffErrors,
} from "./handoff-form.ts";
import { pushedOverState, useClosePushedOver } from "./pushed-over.ts";
import { WorkTextArea } from "./work-textarea.tsx";

/**
 * The handoff dialog follows `/app/r/$roomId/t/$threadId/handoff` (a board's query stays put).
 * An in-app open (the work bar, a board post) pushes that URL with {@link pushedOverState}, and
 * closing steps back, so the thread entry underneath is the one that was already there. A direct
 * arrival (the classic URL's redirect, a deep link) has no such flag, and closing replaces this
 * entry with the thread. Cancel and a successful handoff both close this way.
 */
export function useHandoffRoute(threadId: number) {
  const navigate = useNavigate();
  const matchRoute = useMatchRoute();
  const closeOver = useClosePushedOver();
  const roomId = useStore((state) => state.threads[threadId]?.roomId ?? null);
  const open = matchRoute({ to: "/r/$roomId/t/$threadId/handoff", includeSearch: false }) !== false;

  const replaceWithThread = () => {
    if (roomId === null) {
      return;
    }

    void navigate({
      to: "/r/$roomId/t/$threadId",
      params: { roomId, threadId },
      search: parseBoardSearch,
      replace: true,
    });
  };

  return {
    open,
    openHandoff: () => {
      if (roomId === null) {
        return;
      }

      void navigate({
        to: "/r/$roomId/t/$threadId/handoff",
        params: { roomId, threadId },
        search: parseBoardSearch,
        state: pushedOverState(),
      });
    },
    closeHandoff: () => closeOver(replaceWithThread),
  };
}

interface HandoffDialogProps {
  readonly threadId: number;
  readonly threadName: string;
  readonly work: WorkDetail;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/** The field order: after a server refusal, focus moves to the first of these that failed. */
const HANDOFF_FIELDS = ["receiver", "summary", "links", "openQuestions"] as const;

type HandoffFocus = HandoffField | "alert";

/** The first named field, or the form alert when the refusal names none. */
function handoffFocus(errors: HandoffErrors, alert: string | null): HandoffFocus | null {
  const field = HANDOFF_FIELDS.find((name) => errors[name] !== undefined);

  if (field !== undefined) {
    return field;
  }

  return alert === null ? null : "alert";
}

/** One agent to choose: its face, name, and provider and description when the server gave them. */
function ReceiverOption({
  name,
  agentId,
  userId,
  detail,
  checked,
  first,
  onChoose,
}: {
  readonly name: string;
  readonly agentId: number;
  readonly userId: number;
  readonly detail: string | null;
  readonly checked: boolean;
  /** The dialog opens on the first option. */
  readonly first: boolean;
  readonly onChoose: (agentId: number) => void;
}) {
  return (
    <label className="work-receiver" data-checked={checked || undefined}>
      <input
        type="radio"
        name="receiver"
        value={agentId}
        checked={checked}
        data-autofocus={first || undefined}
        onChange={() => onChoose(agentId)}
      />
      <UserAvatar userId={userId} size={24} decorative />
      <span className="work-receiver-text">
        <span className="work-receiver-name">{name}</span>
        {detail === null ? null : <span className="work-receiver-detail">{detail}</span>}
      </span>
    </label>
  );
}

/**
 * "Hand off “{name}”": picks the receiving agent from `handoffReceivers`, with a summary, links
 * and open questions. Checks what the server checks before sending. A refusal that names a field
 * is shown on that field and takes focus there; one that doesn't (an untracked thread, a lost
 * connection) is the form's alert, which takes focus instead. A reply that lands after the
 * dialog was dismissed or unmounted is ignored, so it can't step back again or toast late.
 */
export function HandoffDialog({
  threadId,
  threadName,
  work,
  open,
  onOpenChange,
}: HandoffDialogProps) {
  const users = useStore((state) => state.users);
  const ids = useId();
  const [draft, setDraft] = useState<HandoffDraft>(EMPTY_HANDOFF);
  const [start, setStart] = useState<HandoffDraft>(EMPTY_HANDOFF);
  const [errors, setErrors] = useState<HandoffErrors>({});
  const [refusal, setRefusal] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  // False until an opening is applied, so a dialog that mounts already open (a direct arrival)
  // still starts a fresh draft and chooses the only agent.
  const [wasOpen, setWasOpen] = useState(false);
  // Bumped when this opening ends (dismissed or unmounted). A reply from
  // before that no longer matches, so it can't close the dialog again.
  const attempt = useRef(0);
  const formRef = useRef<HTMLFormElement>(null);
  const [focus, setFocus] = useState<HandoffFocus | null>(null);

  useEffect(() => {
    return () => {
      attempt.current += 1;
    };
  }, []);

  useLayoutEffect(() => {
    if (focus === null) {
      return;
    }

    const form = formRef.current;

    if (form !== null) {
      const element =
        focus === "alert"
          ? form.querySelector<HTMLElement>('[role="alert"]')
          : form.querySelector<HTMLElement>(`[name="${focus}"]`);

      element?.focus();
    }

    setFocus(null);
  }, [focus]);

  // Each opening starts afresh, with the only agent chosen when there's just one.
  if (open !== wasOpen) {
    setWasOpen(open);

    if (!open) {
      attempt.current += 1;
      setFocus(null);
    }

    if (open) {
      const only = work.handoffReceivers.length === 1 ? work.handoffReceivers[0] : undefined;

      const fresh = { ...EMPTY_HANDOFF, receiverAgentId: only?.agentId ?? null };

      setDraft(fresh);
      setStart(fresh);
      setErrors({});
      setRefusal(null);
      setSending(false);
    }
  }

  const change = (patch: Partial<HandoffDraft>, field: keyof HandoffErrors) => {
    setDraft((current) => ({ ...current, ...patch }));
    setErrors(({ [field]: _gone, ...others }) => others);
  };

  const nameOf = (userId: number) => users[userId]?.name ?? UNKNOWN_NAME;

  const requestClose = () => {
    attempt.current += 1;
    onOpenChange(false);
  };

  const submit = () => {
    const checked = checkHandoff(draft);

    if ("errors" in checked) {
      setErrors(checked.errors);

      return;
    }

    const receiver = work.handoffReceivers.find(
      (option) => option.agentId === checked.body.receiverAgentId,
    );

    const name = receiver === undefined ? UNKNOWN_NAME : nameOf(receiver.userId);

    setSending(true);
    setRefusal(null);
    const mine = attempt.current;

    actions.work.handOff(threadId, checked.body).then(
      () => {
        if (attempt.current !== mine) {
          return;
        }

        setSending(false);
        onOpenChange(false);
        toast({ title: `Handed off to ${name}`, tone: "success" });
      },
      (error: Error) => {
        if (attempt.current !== mine) {
          return;
        }

        const found = serverHandoffErrors(error);

        setFocus(handoffFocus(found.errors, found.alert));
        setSending(false);
        setErrors(found.errors);
        setRefusal(found.alert);
      },
    );
  };

  const formId = `${ids}-form`;

  const dirty =
    draft.receiverAgentId !== start.receiverAgentId ||
    draft.summary !== start.summary ||
    draft.links !== start.links ||
    draft.openQuestions !== start.openQuestions;

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) {
          requestClose();
        }
      }}
      title={`Hand off “${threadName}”`}
      description="Ownership moves to the receiving agent, the handoff is recorded in Work history, and the agent gets the context package below through its event feed."
      size="md"
      dirty={dirty}
      footer={
        <>
          <Button variant="secondary" onClick={requestClose}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon="send"
            type="submit"
            form={formId}
            loading={sending}
            loadingLabel="Handing off"
          >
            Hand off
          </Button>
        </>
      }
    >
      <form
        ref={formRef}
        id={formId}
        className="work-handoff"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        {refusal === null ? null : (
          <p className="work-form-refusal" role="alert" tabIndex={-1}>
            {refusal}
          </p>
        )}
        <fieldset
          className="work-receivers"
          aria-describedby={errors.receiver === undefined ? undefined : `${ids}-receiver-error`}
          aria-invalid={errors.receiver === undefined ? undefined : true}
        >
          <legend className="work-field-label">Receiving agent</legend>
          {work.handoffReceivers.map((option, index) => {
            const candidate = work.ownerCandidates.find((entry) => entry.userId === option.userId);

            const detail = [candidate?.provider, candidate?.description]
              .filter((part) => part != null && part !== "")
              .join(" · ");

            return (
              <ReceiverOption
                key={option.agentId}
                name={nameOf(option.userId)}
                agentId={option.agentId}
                userId={option.userId}
                detail={detail === "" ? null : detail}
                checked={draft.receiverAgentId === option.agentId}
                first={index === 0}
                onChoose={(agentId) => change({ receiverAgentId: agentId }, "receiver")}
              />
            );
          })}
          {errors.receiver === undefined ? null : (
            <p id={`${ids}-receiver-error`} className="work-field-error">
              {errors.receiver}
            </p>
          )}
        </fieldset>
        <WorkTextArea
          label="Summary"
          name="summary"
          placeholder="Where the work stands and what is next…"
          maxLength={SUMMARY_LIMIT}
          rows={4}
          value={draft.summary}
          error={errors.summary}
          onChange={(event) => change({ summary: event.target.value }, "summary")}
        />
        <WorkTextArea
          label={`Links (one URL per line, up to ${HANDOFF_ITEM_LIMIT})`}
          name="links"
          placeholder="https://example.com/spec"
          rows={2}
          value={draft.links}
          error={errors.links}
          onChange={(event) => change({ links: event.target.value }, "links")}
        />
        <WorkTextArea
          label={`Open questions (one per line, up to ${HANDOFF_ITEM_LIMIT})`}
          name="openQuestions"
          placeholder="Which approach did we agree on?"
          rows={2}
          value={draft.openQuestions}
          error={errors.openQuestions}
          onChange={(event) => change({ openQuestions: event.target.value }, "openQuestions")}
        />
      </form>
    </Dialog>
  );
}
