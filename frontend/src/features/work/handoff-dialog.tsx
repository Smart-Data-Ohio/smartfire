import { useMatchRoute, useNavigate } from "@tanstack/react-router";
import { useId, useState } from "react";
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
  SUMMARY_LIMIT,
} from "./handoff-form.ts";
import { WorkTextArea } from "./work-textarea.tsx";

/**
 * The handoff dialog follows `/app/r/$roomId/t/$threadId/handoff` (a board's query stays put).
 * Opening pushes that URL; closing replaces it with the thread, so a direct visit and Cancel
 * both land on the thread.
 */
export function useHandoffRoute(threadId: number) {
  const navigate = useNavigate();
  const matchRoute = useMatchRoute();
  const roomId = useStore((state) => state.threads[threadId]?.roomId ?? null);
  const open = matchRoute({ to: "/r/$roomId/t/$threadId/handoff", includeSearch: false }) !== false;

  const go = (
    to: "/r/$roomId/t/$threadId" | "/r/$roomId/t/$threadId/handoff",
    replace: boolean,
  ) => {
    if (roomId === null) {
      return;
    }

    void navigate({ to, params: { roomId, threadId }, search: parseBoardSearch, replace });
  };

  return {
    open,
    openHandoff: () => go("/r/$roomId/t/$threadId/handoff", false),
    closeHandoff: () => go("/r/$roomId/t/$threadId", true),
  };
}

interface HandoffDialogProps {
  readonly threadId: number;
  readonly threadName: string;
  readonly work: WorkDetail;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
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
 * and open questions. Checks what the server checks before sending, and shows the server's
 * refusal in the form when it has the last word.
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
  const [wasOpen, setWasOpen] = useState(open);

  // Each opening starts afresh, with the only agent chosen when there's just one.
  if (open !== wasOpen) {
    setWasOpen(open);

    if (open) {
      const only = work.handoffReceivers.length === 1 ? work.handoffReceivers[0] : undefined;

      const fresh = { ...EMPTY_HANDOFF, receiverAgentId: only?.agentId ?? null };

      setDraft(fresh);
      setStart(fresh);
      setErrors({});
      setRefusal(null);
    }
  }

  const change = (patch: Partial<HandoffDraft>, field: keyof HandoffErrors) => {
    setDraft((current) => ({ ...current, ...patch }));
    setErrors(({ [field]: _gone, ...others }) => others);
  };

  const nameOf = (userId: number) => users[userId]?.name ?? UNKNOWN_NAME;

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
    actions.work.handOff(threadId, checked.body).then(
      () => {
        setSending(false);
        onOpenChange(false);
        toast({ title: `Handed off to ${name}`, tone: "success" });
      },
      (error: Error) => {
        setSending(false);
        setRefusal(error.message);
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
      onOpenChange={onOpenChange}
      title={`Hand off “${threadName}”`}
      description="Ownership moves to the receiving agent, the handoff is recorded in Work history, and the agent gets the context package below through its event feed."
      size="md"
      dirty={dirty}
      footer={
        <>
          <Button variant="secondary" onClick={() => onOpenChange(false)}>
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
        id={formId}
        className="work-handoff"
        noValidate
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        {refusal === null ? null : (
          <p className="work-form-refusal" role="alert">
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
          placeholder="Where the work stands and what is next…"
          maxLength={SUMMARY_LIMIT}
          rows={4}
          value={draft.summary}
          error={errors.summary}
          onChange={(event) => change({ summary: event.target.value }, "summary")}
        />
        <WorkTextArea
          label={`Links (one URL per line, up to ${HANDOFF_ITEM_LIMIT})`}
          placeholder="https://example.com/spec"
          rows={2}
          value={draft.links}
          error={errors.links}
          onChange={(event) => change({ links: event.target.value }, "links")}
        />
        <WorkTextArea
          label={`Open questions (one per line, up to ${HANDOFF_ITEM_LIMIT})`}
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
