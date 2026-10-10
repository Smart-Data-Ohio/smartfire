import { type FormEvent, useEffect, useRef, useState } from "react";
import { LazyEmojiPicker, preloadEmojiPicker } from "../../lib/emoji/lazy-emoji-picker.tsx";
import {
  browserDeps,
  DEFAULT_UPLOAD_LIMIT_BYTES,
  UploadTask,
  uploadSizeError,
} from "../../lib/upload/direct-upload.ts";
import { uuid7 } from "../../lib/uuid7.ts";
import { useStore } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Popover } from "../../ui/popover.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { Toggle } from "../../ui/toggle.tsx";
import { loadCustomIcons } from "../messages/commands.ts";
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
  readonly threadId?: number | null;
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
  const [dirty, setDirty] = useState(false);

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
      description={
        props.threadId == null
          ? "Everyone in the room can vote."
          : "Everyone in the thread can vote."
      }
      size="md"
      dirty={dirty}
    >
      <CreatePollForm key={opened} {...props} onDirty={setDirty} />
    </Dialog>
  );
}

interface CreatePollFormProps extends CreatePollDialogProps {
  /** Whether anything differs from how the form opened. */
  readonly onDirty: (dirty: boolean) => void;
}

type DraftMedia =
  | { readonly kind: "emoji"; readonly content: string }
  | { readonly kind: "image"; readonly file: File }
  | null;

interface DraftOption {
  readonly label: string;
  readonly media: DraftMedia;
}

const IMAGE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp"];

function OptionMediaPicker({
  index,
  media,
  onChange,
  onError,
  limitBytes,
}: {
  readonly index: number;
  readonly media: DraftMedia;
  readonly onChange: (media: DraftMedia) => void;
  readonly onError: (message: string) => void;
  readonly limitBytes: number;
}) {
  const input = useRef<HTMLInputElement>(null);

  return (
    <div className="create-poll-media">
      <Popover
        label={`Emoji for option ${index + 1}`}
        trigger={(props) => (
          <IconButton
            {...props}
            icon="smile"
            label={`Emoji for option ${index + 1}`}
            size="sm"
            onPointerEnter={preloadEmojiPicker}
            onFocus={preloadEmojiPicker}
          />
        )}
      >
        {(close) => (
          <LazyEmojiPicker
            loadCustomIcons={async () =>
              (await loadCustomIcons()).filter(
                (icon) => icon.imageUrl?.startsWith("/icons/") === true,
              )
            }
            onPick={(choice) => {
              close();
              onChange({ kind: "emoji", content: choice.content });
            }}
          />
        )}
      </Popover>
      <IconButton
        icon="image"
        label={`Attach image to option ${index + 1}`}
        size="sm"
        onClick={() => input.current?.click()}
      />
      <input
        ref={input}
        type="file"
        className="visually-hidden"
        aria-label={`Image for option ${index + 1}`}
        accept={IMAGE_TYPES.join(",")}
        onChange={(event) => {
          const file = event.target.files?.[0];
          event.target.value = "";

          if (file === undefined) return;

          if (!IMAGE_TYPES.includes(file.type)) {
            onError("Choose a PNG, JPEG, GIF or WebP image.");

            return;
          }

          const sizeError = uploadSizeError(file, limitBytes);

          if (sizeError !== null) {
            onError(sizeError);

            return;
          }

          onChange({ kind: "image", file });
        }}
      />
      {media === null ? null : (
        <>
          <span className="create-poll-media-name">
            {media.kind === "emoji" ? (
              media.content
            ) : (
              <>
                <Icon name="image" size={14} />
                {media.file.name}
              </>
            )}
          </span>
          <IconButton
            icon="x"
            label={`Remove media from option ${index + 1}`}
            size="sm"
            onClick={() => onChange(null)}
          />
        </>
      )}
    </div>
  );
}

function CreatePollForm({
  roomId,
  threadId = null,
  onOpenChange,
  initialQuestion = "",
  onDirty,
}: CreatePollFormProps) {
  const [sent, setSent] = useState<Sent | null>(null);
  const [startQuestion] = useState(initialQuestion);
  const [question, setQuestion] = useState(initialQuestion);

  const [options, setOptions] = useState<readonly DraftOption[]>([
    { label: "", media: null },
    { label: "", media: null },
  ]);

  const imageLimit = useStore(
    (state) => state.boot?.account.uploadLimitBytes ?? DEFAULT_UPLOAD_LIMIT_BYTES,
  );

  const uploads = useRef(new Set<UploadTask>());
  const uploaded = useRef(new Map<File, string>());
  const live = useRef(true);
  useEffect(() => {
    live.current = true;

    return () => {
      live.current = false;

      for (const task of uploads.current) task.cancel();
    };
  }, []);
  const [multiple, setMultiple] = useState(false);
  const [anonymous, setAnonymous] = useState(false);
  const [closes, setCloses] = useState<CloseId>("never");
  const [problems, setProblems] = useState<PollProblems>(NO_PROBLEMS);
  const [failure, setFailure] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);

  const dirty =
    question !== startQuestion ||
    options.length !== MIN_OPTIONS ||
    options.some((option) => option.label !== "" || option.media !== null) ||
    multiple ||
    anonymous ||
    closes !== "never";

  useEffect(() => onDirty(dirty), [dirty, onDirty]);

  useEffect(() => () => onDirty(false), [onDirty]);

  const setOption = (index: number, value: string) => {
    setOptions((current) =>
      current.map((option, at) => (at === index ? { ...option, label: value } : option)),
    );
    setProblems((current) => ({ ...current, options: undefined }));
  };

  const removeOption = (index: number) => {
    setOptions((current) => current.filter((_, at) => at !== index));
  };

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (busy) return;

    const found = pollProblems(
      question,
      options.map((option) => option.label),
    );

    if (found.question !== undefined || found.options !== undefined) {
      setProblems(found);
      setAttempt((count) => count + 1);

      return;
    }

    const span = CLOSES.find((choice) => choice.id === closes)?.ms ?? null;

    setBusy(true);
    setFailure(undefined);

    try {
      const filled = options.filter((option) => option.label.trim() !== "");

      const optionMedia = await Promise.all(
        filled.map(async (option) => {
          const media = option.media;

          if (media === null) return null;

          if (media.kind === "emoji") return { emoji: media.content };
          const held = uploaded.current.get(media.file);

          if (held !== undefined) return { signedId: held };

          const task = new UploadTask(
            media.file,
            browserDeps(actions.messages.startUpload, imageLimit),
            () => undefined,
          );

          uploads.current.add(task);
          await task.start();
          uploads.current.delete(task);
          const { signedId, error } = task.snapshot;

          if (signedId === null) throw new Error(error ?? "The image upload didn't finish.");
          uploaded.current.set(media.file, signedId);

          return { signedId };
        }),
      );

      if (!live.current) return;

      const poll = {
        threadId,
        question: question.trim(),
        options: filled.map((option) => option.label.trim()),
        multiple,
        anonymous,
      };

      const withMedia = filled.some((option) => option.media !== null)
        ? { ...poll, optionMedia }
        : poll;

      const draft = JSON.stringify({ ...withMedia, closes });

      const clientMessageId =
        sent !== null && sent.draft === draft ? sent.clientMessageId : uuid7(Date.now());

      setSent({ draft, clientMessageId });
      setBusy(true);
      setFailure(undefined);

      await actions.cards
        .createPoll(roomId, {
          clientMessageId,
          ...withMedia,
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
    } catch (error) {
      setBusy(false);
      setFailure(error instanceof Error ? error.message : "The image upload didn't finish.");
    }
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
      <fieldset className="create-poll-options" disabled={busy}>
        <legend className="create-poll-legend">Options</legend>
        {options.map((option, index) => (
          // Options have no identity but their place while drafting.
          // biome-ignore lint/suspicious/noArrayIndexKey: rows are positional in a draft
          <div className="create-poll-option" key={index}>
            <TextField
              label={`Option ${index + 1}`}
              value={option.label}
              maxLength={MAX_LABEL}
              onChange={(event) => setOption(index, event.target.value)}
            />
            <OptionMediaPicker
              index={index}
              media={option.media}
              limitBytes={imageLimit}
              onError={setFailure}
              onChange={(media) => {
                setOptions((current) =>
                  current.map((item, at) => (at === index ? { ...item, media } : item)),
                );
                setFailure(undefined);
              }}
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
              onClick={() => setOptions((current) => [...current, { label: "", media: null }])}
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
      <div className="create-poll-actions" data-dialog-actions>
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
