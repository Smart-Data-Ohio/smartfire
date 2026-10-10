/**
 * The thread pane's work details: the result (Markdown, rendered as the server's `resultHtml`),
 * the agent's steps, and the history, newest first.
 */
import { useEffect, useRef, useState } from "react";
import type { WorkDetail } from "../../gen/WorkDetail.ts";
import type { WorkHistoryEntry } from "../../gen/WorkHistoryEntry.ts";
import { inlineMentions } from "../../lib/body-html.ts";
import { formatFull } from "../../lib/time.ts";
import { SuccessCheck } from "../../motion/success-check.tsx";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { useSpoilerReveal } from "../messages/spoilers.ts";
import { timeAgo } from "../threads/thread-format.ts";
import { useNow } from "../threads/use-now.ts";
import { FORMER_MEMBER, historyIcon, historyText } from "./work-format.ts";
import { WorkSteps } from "./work-steps.tsx";
import { WorkTextArea } from "./work-textarea.tsx";

/** The server's limit on a result's Markdown. */
export const RESULT_LIMIT = 20_000;

/** How long the saved tick stays. */
const SAVED_MS = 1600;

/** A history actor's name: "Former member" when the account is gone or unknown here. */
function useActorName(actorId: number | null): string {
  const name = useStore((state) => (actorId === null ? undefined : state.users[actorId]?.name));

  return name ?? FORMER_MEMBER;
}

/** "Updated 2 hours ago by Maya". */
function ResultMeta({
  updatedAt,
  updatedById,
}: {
  readonly updatedAt: string;
  readonly updatedById: number | null;
}) {
  const now = useNow();
  const name = useActorName(updatedById);

  return (
    <p className="work-meta">
      Updated{" "}
      <time dateTime={updatedAt} title={formatFull(updatedAt)}>
        {timeAgo(updatedAt, now)}
      </time>{" "}
      by {name}
    </p>
  );
}

interface WorkResultProps {
  readonly threadId: number;
  readonly work: WorkDetail;
  readonly updatedAt: string | null;
  readonly canEdit: boolean;
  readonly announce: (text: string) => void;
}

/**
 * The result: what was recorded, who last changed it and when; whoever manages the work edits
 * the Markdown in place (Save, or Ctrl+Enter; Cancel, or Escape). A blank result clears it.
 */
export function WorkResult({ threadId, work, updatedAt, canEdit, announce }: WorkResultProps) {
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const editRef = useRef<HTMLButtonElement | null>(null);
  const returnFocus = useRef(false);
  const resultRef = useSpoilerReveal(work.resultHtml ?? "");

  // Back from the editor, focus returns to the button that opened it.
  useEffect(() => {
    if (!editing && returnFocus.current) {
      returnFocus.current = false;
      editRef.current?.focus();
    }
  }, [editing]);

  useEffect(() => {
    if (!saved) {
      return;
    }

    const timer = window.setTimeout(() => setSaved(false), SAVED_MS);

    return () => window.clearTimeout(timer);
  }, [saved]);

  const start = () => {
    setText(work.resultMarkdown ?? "");
    setError(undefined);
    setEditing(true);
  };

  const stop = () => {
    returnFocus.current = true;
    setEditing(false);
  };

  const save = () => {
    if (text.length > RESULT_LIMIT) {
      setError(`Result markdown is too long (maximum is ${RESULT_LIMIT} characters)`);

      return;
    }

    const markdown = text.trim() === "" ? null : text;

    setSaving(true);
    actions.work.saveResult(threadId, markdown).then(
      () => {
        setSaving(false);
        setSaved(true);
        announce(markdown === null ? "Result cleared" : "Result saved");
        stop();
      },
      (failure: Error) => {
        setSaving(false);
        setError(failure.message);
      },
    );
  };

  return (
    <section className="work-section" aria-labelledby={`work-result-${threadId}`}>
      <div className="work-section-head">
        <h3 id={`work-result-${threadId}`} className="work-section-title">
          Result
        </h3>
        {/* Mounted only while shown: under reduced motion the recipe shows it whatever its state. */}
        {saved ? <SuccessCheck size={14} className="work-saved" /> : null}
        {canEdit && !editing ? (
          <Button ref={editRef} variant="ghost" size="sm" icon="pencil" onClick={start}>
            {work.resultMarkdown === null ? "Add result" : "Edit result"}
          </Button>
        ) : null}
      </div>
      {editing ? (
        <form
          className="work-result-form"
          onSubmit={(event) => {
            event.preventDefault();
            save();
          }}
        >
          <WorkTextArea
            label="Result"
            hideLabel
            placeholder="Record the outcome in Markdown…"
            rows={4}
            value={text}
            error={error}
            autoFocus
            onChange={(event) => {
              setText(event.target.value);
              setError(undefined);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
                event.preventDefault();
                event.currentTarget.form?.requestSubmit();
              } else if (event.key === "Escape") {
                event.preventDefault();
                event.stopPropagation();
                stop();
              }
            }}
          />
          <div className="work-form-actions">
            <Button variant="secondary" size="sm" onClick={stop}>
              Cancel
            </Button>
            <Button variant="primary" size="sm" type="submit" loading={saving}>
              Save result
            </Button>
          </div>
        </form>
      ) : work.resultHtml === null ? (
        <p className="work-empty">No result recorded yet.</p>
      ) : (
        <div
          ref={resultRef}
          className="message-body work-result-body"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: resultHtml is the server's sanitizer output (crates/richtext), as message bodies are
          dangerouslySetInnerHTML={{ __html: inlineMentions(work.resultHtml) }}
        />
      )}
      {updatedAt === null || editing ? null : (
        <ResultMeta updatedAt={updatedAt} updatedById={work.resultUpdatedById} />
      )}
    </section>
  );
}

/** One history line: the actor, what changed, and when. */
function HistoryItem({ entry, now }: { readonly entry: WorkHistoryEntry; readonly now: number }) {
  const actor = useActorName(entry.actorId);

  return (
    <li className="work-history-item" data-kind={entry.kind}>
      <span className="work-history-icon" aria-hidden="true">
        <Icon name={historyIcon(entry.kind)} size={12} />
      </span>
      <span className="work-history-text">
        <strong className="work-history-actor">{actor}</strong> {historyText(entry)}
      </span>
      <time
        className="work-history-time"
        dateTime={entry.createdAt}
        title={formatFull(entry.createdAt)}
      >
        {timeAgo(entry.createdAt, now)}
      </time>
    </li>
  );
}

/** The work's history, newest first: updates, assignments, handoffs and results. */
export function WorkHistory({
  threadId,
  history,
}: {
  readonly threadId: number;
  readonly history: readonly WorkHistoryEntry[];
}) {
  const now = useNow();

  return (
    <section className="work-section" aria-labelledby={`work-history-${threadId}`}>
      <h3 id={`work-history-${threadId}`} className="work-section-title">
        History
      </h3>
      {history.length === 0 ? (
        <p className="work-empty">No changes recorded yet.</p>
      ) : (
        <ol className="work-history">
          {history.map((entry) => (
            <HistoryItem key={entry.id} entry={entry} now={now} />
          ))}
        </ol>
      )}
    </section>
  );
}

/** The agent's steps, when it has reported any. */
export function WorkStepsSection({
  threadId,
  work,
}: {
  readonly threadId: number;
  readonly work: WorkDetail;
}) {
  if (work.steps.length === 0) {
    return null;
  }

  return (
    <section className="work-section" aria-labelledby={`work-steps-${threadId}`}>
      <h3 id={`work-steps-${threadId}`} className="work-section-title">
        Steps ({work.steps.length})
      </h3>
      <WorkSteps steps={work.steps} />
    </section>
  );
}
