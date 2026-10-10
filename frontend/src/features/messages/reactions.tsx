import { type CSSProperties, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { Boost } from "../../gen/Boost.ts";
import type { Reaction } from "../../gen/Reaction.ts";
import { EmojiImage } from "../../lib/emoji/emoji-image.tsx";
import { preloadEmojiPicker } from "../../lib/emoji/lazy-emoji-picker.tsx";
import { AnimatedNumber } from "../../motion/animated-number.tsx";
import { prefersReducedMotion } from "../../motion/reduced-motion.ts";
import type { MessageDTO } from "../../store/model.ts";
import { store, useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Tooltip } from "../../ui/tooltip.tsx";
import { takeBurst } from "./burst.ts";
import { removeBoost, toggleReaction } from "./commands.ts";
import { reactionLabel, reactorSummary } from "./reaction-summary.ts";

const PARTICLES = 8;

/** One particle's flight for the like-button burst: a ring of vectors with a little scatter. */
function particleStyle(index: number): CSSProperties {
  const angle = (index / PARTICLES) * Math.PI * 2 + (Math.random() - 0.5) * 0.5;
  const distance = 10 + Math.random() * 8;

  return {
    "--px": `${Math.cos(angle) * distance}px`,
    "--py": `${Math.sin(angle) * distance}px`,
    "--psize": `${0.7 + Math.random() * 0.6}`,
    "--pdelay": `${Math.round(Math.random() * 40)}ms`,
  };
}

function useNames(): (id: number) => string | undefined {
  const users = useStore((state) => state.users);

  return (id) => users[id]?.name;
}

/** Loads the reactors the store doesn't know, the first time someone looks at the tooltip. */
function fetchMissing(ids: readonly number[]): void {
  const known = store.getState().users;

  if (ids.some((id) => known[id] === undefined)) {
    void actions.ensureUsers(ids).catch(() => undefined);
  }
}

interface PillProps {
  readonly message: MessageDTO;
  readonly reaction: Reaction;
  readonly viewerId: number | null;
  readonly fresh: boolean;
  readonly canReact: boolean;
}

/**
 * A reaction pill: the emoji (or icon) and a count that pops when it changes, highlighted when
 * the viewer reacted. Clicking toggles; adding plays the like-button pop and particle burst.
 */
function ReactionPill({ message, reaction, viewerId, fresh, canReact }: PillProps) {
  const mine = viewerId !== null && reaction.reactorIds.includes(viewerId);
  const nameOf = useNames();
  const [burst, setBurst] = useState(false);
  const [particles, setParticles] = useState<readonly CSSProperties[]>([]);
  const timer = useRef(0);

  // The viewer's own add queued a burst: take it once the reaction shows as theirs, whether the
  // click created this pill or it already existed. Before paint, so the pop starts with the pill.
  useLayoutEffect(() => {
    if (mine && takeBurst(message.id, reaction.content)) {
      setParticles(Array.from({ length: PARTICLES }, (_, index) => particleStyle(index)));
      setBurst(true);
    }
  }, [mine, message.id, reaction.content]);

  useEffect(() => {
    if (!burst) {
      return;
    }

    timer.current = window.setTimeout(() => setBurst(false), prefersReducedMotion() ? 0 : 700);

    return () => window.clearTimeout(timer.current);
  }, [burst]);

  const tooltip = reactorSummary(reaction.reactorIds, viewerId, nameOf, reaction.title);

  return (
    <Tooltip content={tooltip} describe={false} placement="top">
      <Button
        variant="pill"
        size="sm"
        className={`reaction t-like${burst ? " is-bursting is-popping" : ""}${fresh ? " enter-chip" : ""}`}
        data-liked={mine}
        aria-pressed={mine}
        aria-label={reactionLabel(reaction.title, reaction.reactorIds.length, mine)}
        disabled={!canReact}
        onPointerEnter={() => fetchMissing(reaction.reactorIds)}
        onFocus={() => fetchMissing(reaction.reactorIds)}
        onClick={() =>
          toggleReaction(message, {
            content: reaction.content,
            title: reaction.title,
            imageUrl: reaction.imageUrl,
          })
        }
      >
        <span className="t-like-icon reaction-glyph" aria-hidden="true">
          {reaction.imageUrl === null ? (
            reaction.content
          ) : (
            <EmojiImage className="reaction-image" src={reaction.imageUrl} draggable={false} />
          )}
        </span>
        <AnimatedNumber value={reaction.reactorIds.length} className="reaction-count" />
        {burst ? (
          <span className="t-like-particles" aria-hidden="true">
            {particles.map((style, index) => (
              // The particles are a fixed ring; their order is their identity.
              // biome-ignore lint/suspicious/noArrayIndexKey: a fixed set of eight decorative dots
              <i key={index} style={style} />
            ))}
          </span>
        ) : null}
      </Button>
    </Tooltip>
  );
}

interface BoostChipProps {
  readonly message: MessageDTO;
  readonly boost: Boost;
  readonly viewerId: number | null;
  readonly fresh: boolean;
}

/** A free-text boost: "nice work · Maya". The viewer's own can be taken back. */
function BoostChip({ message, boost, viewerId, fresh }: BoostChipProps) {
  const booster = useStore((state) => state.users[boost.boosterId]);
  const name = boost.boosterId === viewerId ? "you" : (booster?.name.split(" ")[0] ?? "someone");
  const own = boost.boosterId === viewerId;

  const body = (
    <>
      <Icon name="rocket" size={12} className="boost-icon" />
      <span className="boost-text">{boost.content}</span>
      <span className="boost-by">{name}</span>
    </>
  );

  if (!own) {
    return (
      <span
        className={`boost${fresh ? " enter-chip" : ""}`}
        title={`Boosted by ${booster?.name ?? "someone"}`}
      >
        {body}
      </span>
    );
  }

  return (
    <Tooltip content="Remove your boost" describe={false}>
      <Button
        variant="pill"
        size="sm"
        className={`boost boost-own${fresh ? " enter-chip" : ""}`}
        trailingIcon="x"
        aria-label={`Remove your boost “${boost.content}”`}
        onClick={() => removeBoost(message, boost.id)}
      >
        {body}
      </Button>
    </Tooltip>
  );
}

interface ReactionsRowProps {
  readonly message: MessageDTO;
  readonly viewerId: number | null;
  readonly canReact: boolean;
  /** Opens the emoji picker beside the "+" pill. */
  readonly onAddReaction: (anchor: HTMLElement) => void;
}

/**
 * Under the body: reaction pills, a ghost "+" pill, then boost chips. Pills that arrive after the
 * row mounted scale in on the pop spring; the ones a page brought in don't.
 */
export function ReactionsRow({ message, viewerId, canReact, onAddReaction }: ReactionsRowProps) {
  const [initial] = useState(
    () =>
      new Set([
        ...message.reactions.map((reaction) => `r:${reaction.content}`),
        ...message.boosts.map((boost) => `b:${boost.id}`),
      ]),
  );

  if (message.reactions.length === 0 && message.boosts.length === 0) {
    return null;
  }

  return (
    <div className="reactions">
      {message.reactions.length > 0 ? (
        <div className="reactions-pills">
          {message.reactions.map((reaction) => (
            <ReactionPill
              key={reaction.content}
              message={message}
              reaction={reaction}
              viewerId={viewerId}
              fresh={!initial.has(`r:${reaction.content}`)}
              canReact={canReact}
            />
          ))}
        </div>
      ) : null}
      {message.boosts.length > 0 ? (
        <div className="boosts">
          {message.boosts.map((boost) => (
            <BoostChip
              key={boost.id}
              message={message}
              boost={boost}
              viewerId={viewerId}
              fresh={!initial.has(`b:${boost.id}`)}
            />
          ))}
        </div>
      ) : null}
      {/* Last in the row, so its reserved space doesn't open a gap before the boosts. */}
      {canReact && message.reactions.length > 0 ? (
        <Tooltip content="Add reaction" describe={false}>
          <Button
            variant="pill"
            size="sm"
            className="reaction reaction-add"
            icon="smile-plus"
            aria-label="Add reaction"
            onPointerEnter={preloadEmojiPicker}
            onClick={(event) => onAddReaction(event.currentTarget)}
          />
        </Tooltip>
      ) : null}
    </div>
  );
}
