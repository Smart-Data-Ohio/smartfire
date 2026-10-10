import { type FormEvent, useEffect, useId, useRef, useState } from "react";
import { AnimatedNumber } from "../../motion/animated-number.tsx";
import { useReducedMotion } from "../../motion/reduced-motion.ts";
import { pollView } from "../../store/cards.ts";
import type { MessageDTO } from "../../store/model.ts";
import { store, useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import { useAnnouncer } from "../destinations/live-region.tsx";
import { useViewerId } from "../messages/use-message.ts";
import { UNKNOWN_NAME } from "../people/people.ts";
import { AvatarGroup } from "../threads/avatar-group.tsx";
import { useNow } from "../threads/use-now.ts";
import { closesLabel, percent } from "./format.ts";

type Poll = NonNullable<MessageDTO["poll"]>;

type PollOption = Poll["options"][number];

function OptionLabel({ option }: { readonly option: PollOption }) {
  const reduced = useReducedMotion();
  const media = option.media;

  const custom =
    media?.kind === "emoji" && media.content.startsWith(":") && media.content.endsWith(":")
      ? media.content.slice(1, -1)
      : null;

  return (
    <span className="poll-option-content">
      {media?.kind === "image" ? (
        <img
          className="poll-option-image"
          src={reduced ? (media.stillUrl ?? media.url) : media.url}
          alt={option.label}
          loading="lazy"
        />
      ) : null}
      {media?.kind === "emoji" ? (
        <span className="poll-option-emoji" aria-hidden="true">
          {custom === null ? (
            media.content
          ) : (
            <img src={`/icons/${encodeURIComponent(custom)}${reduced ? "?still=1" : ""}`} alt="" />
          )}
        </span>
      ) : null}
      <span>{option.label}</span>
    </span>
  );
}

/** The most voters an option shows as faces. */
const FACES = 3;

/** Where focus goes once the control that had it is replaced: the options, or Change vote. */
type FocusTarget = "choices" | "change";

/** Each target's candidates, best first; the first one on screen takes focus. */
const FOCUS_ORDER: Readonly<Record<FocusTarget, readonly string[]>> = {
  choices: [".poll-choices input:checked", ".poll-choices input", "[data-poll-change]"],
  change: ["[data-poll-change]", ".poll-choices input:checked", ".poll-choices input"],
};

function focusable(card: HTMLElement, target: FocusTarget): HTMLElement | null {
  for (const selector of FOCUS_ORDER[target]) {
    const found = card.querySelector<HTMLElement>(selector);

    if (found !== null) {
      return found;
    }
  }

  return null;
}

/** "Tacos 60%, Pizza 40%": a poll's results read aloud. */
function resultsText(poll: Poll): string {
  const total = poll.options.reduce((sum, option) => sum + option.votes, 0);

  return poll.options
    .map((option) => `${option.label} ${percent(option.votes, total)}%`)
    .join(", ");
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
        <span className="poll-result-text">
          <OptionLabel option={option} />
        </span>
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
                label={<OptionLabel option={option} />}
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
              <span className="poll-option-label">
                <OptionLabel option={option} />
              </span>
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
 * A poll under its question: pick an option (or tick several) and vote, then the results as bars
 * that grow into place; change or take back a vote while it's open; final results once closed
 * (the client closes it itself when `closesAt` passes). Anonymous polls show counts only, and
 * fetch the viewer's own vote, which their `voterIds` can't tell.
 *
 * Voting, changing and retracting swap the controls, so focus moves on to the control that
 * replaces the one that had it (the chosen option, or Change vote), and one live region per card
 * says what happened.
 */
export function PollCard({ message, poll }: { readonly message: MessageDTO; readonly poll: Poll }) {
  const viewerId = useViewerId() ?? 0;
  const administrator = useStore((state) => state.me?.user.role === "administrator");
  const now = useNow();
  const ballot = useStore((state) => state.cards.ballots[poll.id]);
  const pending = useStore((state) => state.cards.pendingVotes[poll.id]);
  const load = useStore((state) => state.cards.pollLoads[poll.id] ?? "idle");
  const [changing, setChanging] = useState(false);
  const [retried, setRetried] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [ending, setEnding] = useState(false);
  const [focusNext, setFocusNext] = useState<FocusTarget | null>(null);
  const cardRef = useRef<HTMLElement | null>(null);
  const { announce, region } = useAnnouncer();
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

  const skeleton = unknown && !view.closed;

  // Runs after every render while a move is due, until the target is on screen (the results may
  // still be loading). Focus moves only if it was in the card or fell to the page when the
  // control that had it went: it never pulls the viewer back from somewhere else.
  useEffect(() => {
    const card = cardRef.current;

    if (focusNext === null || card === null) {
      return;
    }

    const target = focusable(card, focusNext);

    if (target === null && skeleton) {
      return;
    }

    const active = document.activeElement;

    if (target !== null && (active === null || active === document.body || card.contains(active))) {
      target.focus();
    }

    setFocusNext(null);
  });

  const retry = () => {
    setRetried(true);
    setFocusNext("choices");
    actions.cards.loadPoll(message.roomId, poll.id).catch(() => undefined);
  };

  const vote = (optionIds: readonly number[]) => {
    const retracting = optionIds.length === 0;

    const labels = poll.options.flatMap((option) =>
      optionIds.includes(option.id) ? [option.label] : [],
    );

    setChanging(false);
    setFailure(null);
    setFocusNext(retracting ? "choices" : "change");

    actions.cards.vote(message.roomId, poll.id, optionIds).then(
      () => {
        const landed = store.getState().messages[message.id]?.poll ?? poll;

        announce(
          retracting
            ? "Vote retracted."
            : `Voted for ${labels.join(", ")}. Results: ${resultsText(landed)}.`,
        );
      },
      (error: Error) => {
        const text = `Couldn't record your vote. ${error.message}`;

        setFailure(text);
        setFocusNext("choices");
        announce(text);
      },
    );
  };

  const change = () => {
    setChanging(true);
    setFocusNext("choices");
  };

  const end = () => {
    setEnding(true);
    setFailure(null);
    actions.cards.endPoll(message.roomId, poll.id).then(
      () => {
        setEnding(false);
        const landed = store.getState().messages[message.id]?.poll ?? poll;
        announce(`Poll ended. Final results: ${resultsText(landed)}.`);
      },
      (error: Error) => {
        setEnding(false);
        const text = `Couldn't end this poll. ${error.message}`;
        setFailure(text);
        announce(text);
      },
    );
  };

  const cancel = () => {
    setChanging(false);
    setFocusNext("change");
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
    <section
      ref={cardRef}
      className="card poll-card"
      data-closed={view.closed || undefined}
      aria-label="Poll"
    >
      <PollMeta poll={view.poll} closed={view.closed} now={now} />
      {failed ? (
        <LoadError retrying={load === "loading"} onRetry={retry} />
      ) : (
        <SkeletonReveal
          loading={skeleton}
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
              onCancel={changing ? cancel : null}
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
      {failure === null ? null : (
        <div className="card-error">
          <Icon name="circle-alert" size={14} />
          <span>{failure}</span>
        </div>
      )}
      <footer className="poll-footer">
        <span className="poll-total tabular">
          <AnimatedNumber value={view.poll.totalVotes} />{" "}
          {view.poll.totalVotes === 1 ? "vote" : "votes"}
          {view.closed ? " · Final results" : voted && !choosing ? " · You voted" : null}
        </span>
        {!view.closed ? (
          <span className="poll-footer-actions">
            {voted && !choosing ? (
              <>
                <Button variant="ghost" size="sm" data-poll-change="" onClick={change}>
                  Change vote
                </Button>
                <Button variant="ghost" size="sm" onClick={() => vote([])}>
                  Retract
                </Button>
              </>
            ) : null}
            {viewerId === message.creatorId || administrator ? (
              <Button
                variant="ghost"
                size="sm"
                loading={ending}
                loadingLabel="Ending"
                onClick={end}
              >
                End poll now
              </Button>
            ) : null}
          </span>
        ) : null}
      </footer>
      {region}
    </section>
  );
}
