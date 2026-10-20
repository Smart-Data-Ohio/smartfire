import { type DragEvent, useEffect, useId, useRef, useState } from "react";
import {
  browserDeps,
  DEFAULT_UPLOAD_LIMIT_BYTES,
  UploadTask,
} from "../../lib/upload/direct-upload.ts";
import { useReducedMotion } from "../../motion/reduced-motion.ts";
import { store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import { stoppedAtConfirmation } from "../auth/confirmation.ts";
import { useFirstFrame } from "./first-frame.ts";
import {
  imageProblem,
  PROFILE_IMAGE_TYPES,
  type ProfileImageKind,
  uploadPercent,
} from "./profile-format.ts";
import "./profile-slot.css";

/** A slot's upload in flight: the chosen file shown at once, and how far its bytes have got. */
export interface Pending {
  readonly preview: string;
  /** The chosen file, for a still of its first frame when motion is reduced. */
  readonly file: File | null;
  readonly percent: number;
  readonly saving: boolean;
}

/** The image a slot shows: the server's, or the file being uploaded in its place. */
export interface Shown {
  readonly url: string | null;
  readonly stillUrl: string | null;
  /** A file being uploaded in its place: it has no still from the server yet. */
  readonly file?: File | null;
}

/**
 * The image as it should show here: animated unless motion is reduced. Under reduced motion a
 * file still uploading shows its first frame, drawn here (nothing until it's drawn): any of the
 * accepted types may move, a PNG included (APNG), so every one is drawn.
 */
export function useShownSrc(image: Shown): string | null {
  const reduced = useReducedMotion();
  const file = image.file ?? null;
  const drawn = useFirstFrame(file, reduced && file !== null);

  if (!reduced) {
    return image.url;
  }

  return file === null ? (image.stillUrl ?? image.url) : drawn;
}

/** What a slot is called and the size it asks for. */
export interface SlotCopy {
  readonly title: string;
  readonly noun: string;
  readonly hint: string;
}

interface SlotProps {
  readonly thumbnail?: ProfileImageKind;
  readonly slot: SlotCopy;
  readonly accept?: string;
  readonly image: Shown;
  readonly pending: Pending | null;
  readonly error: string | null;
  readonly onFile: (file: File) => void;
  readonly onRemove: () => void;
}

/**
 * One image's controls: drop a file on it or choose one, replace it, or remove it. The upload's
 * progress and any refusal show in place.
 */
export function ProfileSlot({
  thumbnail,
  slot,
  accept = PROFILE_IMAGE_TYPES.join(","),
  image,
  pending,
  error,
  onFile,
  onRemove,
}: SlotProps) {
  const input = useRef<HTMLInputElement | null>(null);
  const [over, setOver] = useState(false);
  const hintId = useId();
  const errorId = useId();
  const src = useShownSrc(image);
  const busy = pending !== null;

  const accepts = (event: DragEvent) => [...event.dataTransfer.types].includes("Files");

  const onDragOver = (event: DragEvent<HTMLDivElement>) => {
    if (busy || !accepts(event)) {
      return;
    }

    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";
    setOver(true);
  };

  const onDrop = (event: DragEvent<HTMLDivElement>) => {
    setOver(false);

    if (busy || !accepts(event)) {
      return;
    }

    event.preventDefault();

    const file = event.dataTransfer.files[0];

    if (file !== undefined) {
      onFile(file);
    }
  };

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: a drop target for mice; the buttons inside are the keyboard's way
    <div
      className="profile-slot"
      data-kind={thumbnail}
      data-over={over || undefined}
      data-busy={busy || undefined}
      onDragEnter={onDragOver}
      onDragOver={onDragOver}
      onDragLeave={(event) => {
        const into = event.relatedTarget;

        if (!(into instanceof Node && event.currentTarget.contains(into))) {
          setOver(false);
        }
      }}
      onDrop={onDrop}
    >
      {thumbnail === undefined ? null : (
        <div className="profile-slot-thumb" data-empty={src === null || undefined}>
          {src === null ? (
            <Icon name="image" size={20} />
          ) : (
            <img src={src} alt={`Current ${slot.noun}`} draggable={false} />
          )}
        </div>
      )}
      <div className="profile-slot-body">
        <h3 className="profile-slot-title">{slot.title}</h3>
        <p id={hintId} className="profile-slot-hint text-meta">
          {over ? `Drop to use it as the ${slot.noun}` : slot.hint}
        </p>
        {pending === null ? null : (
          <div className="profile-slot-progress">
            <progress
              max={100}
              value={pending.saving ? undefined : pending.percent}
              aria-label={`Uploading the ${slot.noun}`}
            />
            <span className="text-meta" aria-live="polite">
              {pending.saving ? "Saving…" : `${pending.percent}%`}
            </span>
          </div>
        )}
        {error === null ? null : (
          <p id={errorId} className="profile-slot-error" role="alert">
            <Icon name="circle-alert" size={14} />
            {error}
          </p>
        )}
        <div className="profile-slot-actions">
          <input
            ref={input}
            type="file"
            accept={accept}
            hidden
            aria-label={`Choose ${slot.noun} image`}
            onChange={(event) => {
              const file = event.target.files?.[0];

              event.target.value = "";

              if (file !== undefined) {
                onFile(file);
              }
            }}
          />
          <Button
            variant="secondary"
            size="sm"
            icon="cloud-upload"
            loading={busy}
            disabled={busy}
            aria-describedby={error === null ? hintId : `${hintId} ${errorId}`}
            onClick={() => input.current?.click()}
          >
            {image.url === null ? `Upload ${slot.noun}` : `Replace ${slot.noun}`}
          </Button>
          {image.url === null ? null : (
            <Button variant="ghost" size="sm" icon="trash" disabled={busy} onClick={onRemove}>
              {`Remove ${slot.noun}`}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

/** What a slot is doing: an upload in flight, and the last refusal. */
interface SlotState {
  readonly pending: Pending | null;
  readonly error: string | null;
}

const IDLE: SlotState = { pending: null, error: null };

/** One image slot's writes: what a finished upload is saved with, and how it's removed. */
interface ImageSlotOptions<T> {
  readonly save: (signedId: string) => Promise<T>;
  readonly remove: () => Promise<T>;
  /** Takes the reply of a save or removal. */
  readonly onChange: (next: T) => void;
  /** The toast once a new image is saved. */
  readonly savedTitle: string;
  readonly problem?: (file: File) => string | null;
}

/** A slot's state and actions, for [`ProfileSlot`]. */
export interface ImageSlot {
  readonly pending: Pending | null;
  readonly error: string | null;
  readonly upload: (file: File) => void;
  readonly remove: () => void;
  /** What shows in its place: the file being uploaded, else the server's image. */
  readonly shown: (server: Shown) => Shown;
}

/**
 * One image slot's upload and removal: the file is checked, shown at once, sent as a direct
 * upload with its progress, then saved. A refusal shows under the slot; one that stopped at its
 * confirmation just puts it back.
 */
export function useImageSlot<T>({
  save,
  remove,
  onChange,
  savedTitle,
  problem = imageProblem,
}: ImageSlotOptions<T>): ImageSlot {
  const [state, setState] = useState<SlotState>(IDLE);
  const previews = useRef(new Set<string>());

  useEffect(() => {
    const urls = previews.current;

    return () => {
      for (const url of urls) {
        URL.revokeObjectURL(url);
      }
    };
  }, []);

  const refused = (error: Error) => {
    setState(stoppedAtConfirmation(error) ? IDLE : { pending: null, error: error.message });
  };

  const release = (preview: string) => {
    URL.revokeObjectURL(preview);
    previews.current.delete(preview);
  };

  const upload = (file: File) => {
    const refusal = problem(file);

    if (refusal !== null) {
      setState({ pending: null, error: refusal });

      return;
    }

    const preview = URL.createObjectURL(file);

    previews.current.add(preview);
    setState({ pending: { preview, file, percent: 0, saving: false }, error: null });

    const task = new UploadTask(
      file,
      browserDeps(
        actions.messages.startUpload,
        store.getState().boot?.account.uploadLimitBytes ?? DEFAULT_UPLOAD_LIMIT_BYTES,
      ),
      (snapshot) => {
        if (snapshot.phase === "uploading") {
          setState({
            pending: {
              preview,
              file,
              percent: uploadPercent(snapshot.loaded, snapshot.total),
              saving: false,
            },
            error: null,
          });
        }
      },
    );

    void task
      .start()
      .then(() => {
        const { phase, signedId, error } = task.snapshot;

        if (phase !== "done" || signedId === null) {
          throw new Error(error ?? "The upload didn't finish.");
        }

        setState({ pending: { preview, file, percent: 100, saving: true }, error: null });

        return save(signedId);
      })
      .then((next) => {
        onChange(next);
        setState(IDLE);
        toast({ title: savedTitle, tone: "success" });
      }, refused)
      .finally(() => release(preview));
  };

  const removeImage = () => {
    setState({ pending: { preview: "", file: null, percent: 100, saving: true }, error: null });
    remove().then((next) => {
      onChange(next);
      setState(IDLE);
    }, refused);
  };

  const shown = (server: Shown): Shown => {
    const pending = state.pending;

    return pending === null || pending.preview === ""
      ? server
      : { url: pending.preview, stillUrl: null, file: pending.file };
  };

  return { pending: state.pending, error: state.error, upload, remove: removeImage, shown };
}
