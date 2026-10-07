import { type FormEvent, useEffect, useId, useState } from "react";
import { AnimatedNumber } from "../../motion/animated-number.tsx";
import { pollView } from "../../store/cards.ts";
import type { MessageDTO } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Tooltip } from "../../ui/tooltip.tsx";
import { useViewerId } from "../messages/use-message.ts";
import { UNKNOWN_NAME } from "../people/people.ts";
import { AvatarGroup } from "../threads/avatar-group.tsx";
import { useNow } from "../threads/use-now.ts";
import { closesLabel, percent } from "./format.ts";

type Poll = NonNullable<MessageDTO["poll"]>;

type PollOption = Poll["options"][number];

/** The most voters an option shows as faces. */
const FACES = 3;

function voteError(error: Error): void {
  toast({ title: "Couldn't record your vote", description: error.message, tone: "danger" });
}

/** "Single choice · Anonymous · Closes in 2 hours": what kind of poll it is. */
function PollMeta({
  poll,
  closed,
  now,
}: {
  readonly poll: Poll;
  readonly closed: boolean;
  readonly now: number;
}) {
  const parts = [poll.multiple ? "Multiple choice" : "Single choice"];

  if (poll.anonymous) {
    parts.push("Anonymous");
  }

  return (
    <div className="poll-meta">
      <span className="poll-kind">
        <Icon name="chart-bar" size={14} />
        Poll
      </span>
      <span className="poll-meta-text">{parts.join(" · ")}</span>
      {closed ? (
        <span className="poll-state" data-state="closed">
          <Icon name="lock" size={12} />
          Closed
        </span>
      ) : poll.closesAt === null ? null : (
        <span className="poll-state" data-state="closing">
          <Icon name="timer" size={12} />
          {closesLabel(poll.closesAt, now)}
        </span>
      )}
    </div>
  );
}

/** Who chose an option, as up to three faces with every name in the tooltip. */
function Voters({ voterIds }: { readonly voterIds: readonly number[] }) {
  const names = useStore((state) =>
    voterIds.map((id) => state.users[id]?.name ?? UNKNOWN_NAME).join(", "),
  );

  if (voterIds.length === 0) {
    return null;
  }

  return (
    <Tooltip content={names}>
      <span className="poll-voters">
        <AvatarGroup userIds={voterIds} size={18} max={FACES} />
        {voterIds.length > FACES ? (
          <span className="poll-voters-more" aria-hidden="true">
            +{voterIds.length - FACES}
          </span>
        ) : null}
        <span className="visually-hidden">Voted: {names}</span>
      </span>
    </Tooltip>
  );
}

interface ResultRowProps {
  readonly option: PollOption;
  readonly total: number;
  readonly mine: boolean;
  readonly leading: boolean;
  readonly anonymous: boolean;
}

/** One option's result: its share as a bar that grows into place, the count, the voters. */
function ResultRow({ option, total, mine, leading, anonymous }: ResultRowProps) {
  const share = percent(option.votes, total);

  return (
    <li className="poll-result" data-mine={mine || undefined} data-leading={leading || undefined}>
      <span
        className="poll-bar t-resize"
        style={{ "--poll-share": `${share}%` }}
        aria-hidden="true"
      />
      <span className="poll-result-label">
        {mine ? <Icon name="circle-check" size={14} className="poll-mine" /> : null}
        <span className="poll-result-text">{option.label}</span>
        {mine ? <span className="visually-hidden">(your vote)</span> : null}
      </span>
      <span className="poll-result-figures">
        {anonymous ? null : <Voters voterIds={option.voterIds} />}
        <span className="poll-count tabular" aria-hidden="true">
          <AnimatedNumber value={share} format={(value) => `${value}%`} />
        </span>
        <span className="visually-hidden">
          {`${option.votes} ${option.votes === 1 ? "vote" : "votes"}, ${share}%`}
        </span>
      </span>
    </li>
  );
}

interface ChoiceListProps {
  readonly poll: Poll;
  readonly initial: readonly number[];
  readonly onVote: (optionIds: readonly number[]) => void;
  readonly onCancel: (() => void) | null;
}

/**
 * Choosing, as a form like classic's: a radio group in a single-choice poll (the arrow keys move
 * between options) and checkboxes in a multiple-choice one, each in a fieldset, then Vote.
 */
function ChoiceList({ poll, initial, onVote, onCancel }: ChoiceListProps) {
  const name = useId();
  const [picked, setPicked] = useState<readonly number[]>(initial);

  const pick = (optionId: number, checked: boolean) =>
    setPicked((current) => {
      if (!poll.multiple) {
        return [optionId];
      }

      const others = current.filter((id) => id !== optionId);

      return checked ? [...others, optionId] : others;
    });

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (picked.length > 0) {
      onVote(picked);
    }
  };

  return (
    <form className="poll-choices" onSubmit={submit}>
      <fieldset className="poll-options" data-multiple={poll.multiple || undefined}>
        <legend className="visually-hidden">
          {poll.multiple ? "Cast your vote: choose one or more" : "Cast your vote"}
        </legend>
        {poll.options.map((option) =>
          poll.multiple ? (
            <div key={option.id} className="poll-option" data-checkbox="">
              <Checkbox
                checked={picked.includes(option.id)}
                label={option.label}
                onCheckedChange={(checked) => pick(option.id, checked)}
              />
            </div>
          ) : (
            <label key={option.id} className="poll-option">
              <span className="poll-radio">
                <input
                  type="radio"
                  className="poll-radio-input"
                  name={name}
                  value={option.id}
                  checked={picked.includes(option.id)}
                  onChange={() => pick(option.id, true)}
                />
              </span>
              <span className="poll-option-label">{option.label}</span>
            </label>
          ),
        )}
      </fieldset>
      <div className="poll-actions">
        <Button type="submit" variant="primary" size="sm" disabled={picked.length === 0}>
          Vote
        </Button>
        {onCancel === null ? null : (
          <Button variant="ghost" size="sm" onClick={onCancel}>
            Cancel
          </Button>
        )}
      </div>
    </form>
  );
}

interface LoadErrorProps {
  readonly retrying: boolean;
  readonly onRetry: () => void;
}

/**
 * The viewer's results couldn't be fetched (an anonymous poll needs them to know the viewer's own
 * vote): say so, and offer to try again. The button stays, busy, while the retry is on its way.
 */
function LoadError({ retrying, onRetry }: LoadErrorProps) {
  return (
    <div className="poll-load-error">
      <div className="card-error">
        <Icon name="circle-alert" size={14} />
        <span>Couldn't load this poll's results.</span>
      </div>
      <div>
        <Button
          variant="secondary"
          size="sm"
          icon="refresh-cw"
          loading={retrying}
          loadingLabel="Loading"
          onClick={onRetry}
        >
          Try again
        </Button>
      </div>
    </div>
  );
}

/**
 * A poll under its question: pick an option (or tick several) and vote, then the results as bars that grow into
 * place; change or take back a vote while it's open; final results once closed (the client
 * closes it itself when `closesAt` passes). Anonymous polls show counts only, and fetch the
 * viewer's own vote, which their `voterIds` can't tell.
 */
export function PollCard({ message, poll }: { readonly message: MessageDTO; readonly poll: Poll }) {
  const viewerId = useViewerId() ?? 0;
  const now = useNow();
  const ballot = useStore((state) => state.cards.ballots[poll.id]);
  const pending = useStore((state) => state.cards.pendingVotes[poll.id]);
  const load = useStore((state) => state.cards.pollLoads[poll.id] ?? "idle");
  const [changing, setChanging] = useState(false);
  const [retried, setRetried] = useState(false);
  const view = pollView(poll, ballot, pending, viewerId, now);
  const unknown = view.myOptionIds === null;

  const voterIds = poll.options.flatMap((option) => option.voterIds);
  const voterKey = voterIds.join(",");

  useEffect(() => {
    if (voterKey !== "") {
      actions.ensureUsers(voterKey.split(",").map(Number)).catch(() => undefined);
    }
  }, [voterKey]);

  useEffect(() => {
    if (unknown && load === "idle") {
      actions.cards.loadPoll(message.roomId, poll.id).catch(() => undefined);
    }
  }, [unknown, load, message.roomId, poll.id]);

  const retry = () => {
    setRetried(true);
    actions.cards.loadPoll(message.roomId, poll.id).catch(() => undefined);
  };

  const vote = (optionIds: readonly number[]) => {
    setChanging(false);
    actions.cards.vote(message.roomId, poll.id, optionIds).catch(voteError);
  };

  const mine = view.myOptionIds ?? [];
  const voted = mine.length > 0;
  const choosing = !view.closed && (changing || !voted);
  const top = Math.max(0, ...view.poll.options.map((option) => option.votes));
  const total = view.poll.options.reduce((sum, option) => sum + option.votes, 0);
  // A failed fetch stays failed (the effect above asks only from idle) until Try again; the
  // error stays up, its button busy, while that retry is on its way.
  const failed = unknown && !view.closed && (load === "error" || (retried && load === "loading"));

  return (
    <section className="card poll-card" data-closed={view.closed || undefined} aria-label="Poll">
      <PollMeta poll={view.poll} closed={view.closed} now={now} />
      {failed ? (
        <LoadError retrying={load === "loading"} onRetry={retry} />
      ) : (
        <SkeletonReveal
          loading={unknown && !view.closed}
          skeleton={
            <div className="poll-skeleton">
              {view.poll.options.map((option) => (
                <Skeleton key={option.id} height={34} radius="md" />
              ))}
            </div>
          }
        >
          {choosing ? (
            <ChoiceList
              poll={view.poll}
              initial={mine}
              onVote={vote}
              onCancel={changing ? () => setChanging(false) : null}
            />
          ) : (
            <ul className="poll-results">
              {view.poll.options.map((option) => (
                <ResultRow
                  key={option.id}
                  option={option}
                  total={total}
                  mine={mine.includes(option.id)}
                  leading={view.closed && top > 0 && option.votes === top}
                  anonymous={view.poll.anonymous}
                />
              ))}
            </ul>
          )}
        </SkeletonReveal>
      )}
      <footer className="poll-footer">
        <span className="poll-total tabular">
          <AnimatedNumber value={view.poll.totalVotes} />{" "}
          {view.poll.totalVotes === 1 ? "vote" : "votes"}
          {view.closed ? " · Final results" : voted && !choosing ? " · You voted" : null}
        </span>
        {!view.closed && voted && !choosing ? (
          <span className="poll-footer-actions">
            <Button variant="ghost" size="sm" onClick={() => setChanging(true)}>
              Change vote
            </Button>
            <Button variant="ghost" size="sm" onClick={() => vote([])}>
              Retract
            </Button>
          </span>
        ) : null}
      </footer>
    </section>
  );
}
