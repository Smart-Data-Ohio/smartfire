import { type FormEvent, useState } from "react";
import { uuid7 } from "../../lib/uuid7.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { Toggle } from "../../ui/toggle.tsx";
import "./create-poll.css";

/** The server's limits (`CreatePoll`): 2 to 10 options, each up to 200 characters. */
export const MIN_OPTIONS = 2;

export const MAX_OPTIONS = 10;

export const MAX_LABEL = 200;

const HOUR = 3_600_000;

/** When voting stops: never, or after one of these spans. */
const CLOSES = [
  { id: "never", label: "Never", ms: null },
  { id: "hour", label: "In 1 hour", ms: HOUR },
  { id: "day", label: "In 1 day", ms: 24 * HOUR },
  { id: "week", label: "In 1 week", ms: 7 * 24 * HOUR },
] as const;

type CloseId = (typeof CLOSES)[number]["id"];

interface CreatePollDialogProps {
  readonly roomId: number;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** What the question starts as (the text after `/poll`). */
  readonly initialQuestion?: string;
}

/** What's wrong with a draft, by field; both undefined when it can be posted. */
export interface PollProblems {
  readonly question: string | undefined;
  readonly options: string | undefined;
}

const NO_PROBLEMS: PollProblems = { question: undefined, options: undefined };

/** The options as they'll be posted: trimmed, blanks left out. */
export function filledOptions(options: readonly string[]): string[] {
  return options.flatMap((option) => {
    const label = option.trim();

    return label === "" ? [] : [label];
  });
}

/** The draft checked as the server will: a question, 2-10 non-blank options of 200 or less. */
export function pollProblems(question: string, options: readonly string[]): PollProblems {
  const filled = filledOptions(options);
  const long = filled.some((option) => option.length > MAX_LABEL);

  return {
    question: question.trim() === "" ? "Ask a question." : undefined,
    options:
      filled.length < MIN_OPTIONS
        ? "Give at least two options."
        : long
          ? `Keep each option to ${MAX_LABEL} characters.`
          : undefined,
  };
}

/** The last draft posted and the client id it went under. */
interface Sent {
  readonly draft: string;
  readonly clientMessageId: string;
}

/**
 * "Create a poll", from the composer's + menu or `/poll`: the question, 2 to 10 options, single
 * or multiple choice, anonymous or not, and when voting stops. Posting the same draft again
 * after a failure reuses its client id, so a retry after a dropped reply can't post the poll
 * twice; a draft edited since gets a new id, since it's a different poll.
 */
export function CreatePollDialog(props: CreatePollDialogProps) {
  // A fresh form (and client id) for each opening.
  const [opened, setOpened] = useState(0);
  const [wasOpen, setWasOpen] = useState(props.open);

  if (props.open !== wasOpen) {
    setWasOpen(props.open);

    if (props.open) {
      setOpened((count) => count + 1);
    }
  }

  return (
    <Dialog
      open={props.open}
      onOpenChange={props.onOpenChange}
      title="Create a poll"
      description="Everyone in the room can vote."
      size="md"
    >
      <CreatePollForm key={opened} {...props} />
    </Dialog>
  );
}

function CreatePollForm({ roomId, onOpenChange, initialQuestion = "" }: CreatePollDialogProps) {
  const [sent, setSent] = useState<Sent | null>(null);
  const [question, setQuestion] = useState(initialQuestion);
  const [options, setOptions] = useState<readonly string[]>(["", ""]);
  const [multiple, setMultiple] = useState(false);
  const [anonymous, setAnonymous] = useState(false);
  const [closes, setCloses] = useState<CloseId>("never");
  const [problems, setProblems] = useState<PollProblems>(NO_PROBLEMS);
  const [failure, setFailure] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);

  const setOption = (index: number, value: string) => {
    setOptions((current) => current.map((option, at) => (at === index ? value : option)));
    setProblems((current) => ({ ...current, options: undefined }));
  };

  const removeOption = (index: number) => {
    setOptions((current) => current.filter((_, at) => at !== index));
  };

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const found = pollProblems(question, options);

    if (found.question !== undefined || found.options !== undefined) {
      setProblems(found);
      setAttempt((count) => count + 1);

      return;
    }

    const span = CLOSES.find((choice) => choice.id === closes)?.ms ?? null;

    const poll = {
      question: question.trim(),
      options: filledOptions(options),
      multiple,
      anonymous,
    };

    const draft = JSON.stringify({ ...poll, closes });

    const clientMessageId =
      sent !== null && sent.draft === draft ? sent.clientMessageId : uuid7(Date.now());

    setSent({ draft, clientMessageId });
    setBusy(true);
    setFailure(undefined);

    actions.cards
      .createPoll(roomId, {
        clientMessageId,
        ...poll,
        closesAt: span === null ? null : new Date(Date.now() + span).toISOString(),
      })
      .then(
        () => onOpenChange(false),
        (error: Error) => {
          setBusy(false);
          setFailure(error.message);
          setAttempt((count) => count + 1);
        },
      );
  };

  return (
    <form className="create-poll" onSubmit={submit} noValidate>
      <TextField
        label="Question"
        value={question}
        placeholder="What should we have for lunch?"
        maxLength={2000}
        data-autofocus
        error={problems.question}
        attempt={attempt}
        onChange={(event) => {
          setQuestion(event.target.value);
          setProblems((current) => ({ ...current, question: undefined }));
        }}
      />
      <fieldset className="create-poll-options">
        <legend className="create-poll-legend">Options</legend>
        {options.map((option, index) => (
          // Options have no identity but their place while drafting.
          // biome-ignore lint/suspicious/noArrayIndexKey: rows are positional in a draft
          <div className="create-poll-option" key={index}>
            <TextField
              label={`Option ${index + 1}`}
              value={option}
              maxLength={MAX_LABEL}
              onChange={(event) => setOption(index, event.target.value)}
            />
            {options.length > MIN_OPTIONS ? (
              <IconButton
                icon="x"
                label={`Remove option ${index + 1}`}
                size="sm"
                onClick={() => removeOption(index)}
              />
            ) : null}
          </div>
        ))}
        {problems.options === undefined ? null : (
          <p className="create-poll-error" role="alert">
            {problems.options}
          </p>
        )}
        {options.length < MAX_OPTIONS ? (
          <div>
            <Button
              variant="ghost"
              size="sm"
              icon="plus"
              onClick={() => setOptions((current) => [...current, ""])}
            >
              Add option
            </Button>
          </div>
        ) : null}
      </fieldset>
      <div className="create-poll-toggles">
        <Toggle
          checked={multiple}
          onCheckedChange={setMultiple}
          label="Allow multiple choices"
          description="Voters can pick more than one option."
        />
        <Toggle
          checked={anonymous}
          onCheckedChange={setAnonymous}
          label="Anonymous"
          description="Show counts only, not who voted."
        />
      </div>
      <fieldset className="create-poll-closes">
        <legend className="create-poll-legend">Voting closes</legend>
        <div className="create-poll-close-choices">
          {CLOSES.map((choice) => (
            <Button
              key={choice.id}
              variant="pill"
              size="sm"
              aria-pressed={closes === choice.id}
              onClick={() => setCloses(choice.id)}
            >
              {choice.label}
            </Button>
          ))}
        </div>
      </fieldset>
      {failure === undefined ? null : (
        <p className="create-poll-error" role="alert">
          {failure}
        </p>
      )}
      <div className="create-poll-actions">
        <Button variant="secondary" onClick={() => onOpenChange(false)}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" loading={busy} loadingLabel="Posting">
          Post poll
        </Button>
      </div>
    </form>
  );
}
