import { type DragEvent, useEffect, useId, useRef, useState } from "react";
import type { Workspace } from "../../gen/Workspace.ts";
import { browserDeps, UploadTask } from "../../lib/upload/direct-upload.ts";
import { useReducedMotion } from "../../motion/reduced-motion.ts";
import { admin } from "../../sync/admin.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import { adminFailure, needsSudo, useAdmin } from "./admin-parts.tsx";
import { mayAnimate, useFirstFrame } from "./first-frame.ts";
import {
  imageProblem,
  PROFILE_FORMATS,
  PROFILE_IMAGE_TYPES,
  PROFILE_SLOTS,
  type ProfileImageKind,
  uploadPercent,
} from "./profile-format.ts";

/** A slot's upload in flight: the chosen file shown at once, and how far its bytes have got. */
interface Pending {
  readonly preview: string;
  /** The chosen file, for a still of its first frame when motion is reduced. */
  readonly file: File | null;
  readonly percent: number;
  readonly saving: boolean;
}

/** The logo or banner the profile shows: the server's, or the file being uploaded in its place. */
interface Shown {
  readonly url: string | null;
  readonly stillUrl: string | null;
  /** A file being uploaded in its place: it has no still from the server yet. */
  readonly file?: File | null;
}

function serverImage(workspace: Workspace, kind: ProfileImageKind): Shown {
  if (kind === "logo") {
    return workspace.logoAttached
      ? { url: workspace.logoUrl, stillUrl: workspace.logoStillUrl }
      : { url: null, stillUrl: null };
  }

  return { url: workspace.bannerUrl, stillUrl: workspace.bannerStillUrl };
}

function initialsOf(name: string): string {
  return name
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((word) => [...word][0] ?? "")
    .join("")
    .toUpperCase();
}

/**
 * The image as it should show here: animated unless motion is reduced. Under reduced motion a
 * file still uploading shows its first frame, drawn here (nothing until it's drawn).
 */
function useShownSrc(image: Shown): string | null {
  const reduced = useReducedMotion();
  const file = image.file ?? null;
  const drawn = useFirstFrame(file, reduced && file !== null && mayAnimate(file));

  if (!reduced) {
    return image.url;
  }

  if (file !== null && mayAnimate(file)) {
    return drawn;
  }

  return image.stillUrl ?? image.url;
}

/**
 * The profile as members will see it: the banner at the top of the sidebar with the workspace's
 * name over it, and the icon as the rail shows it. Files being uploaded show here at once.
 */
function ProfilePreview({
  name,
  logo,
  banner,
}: {
  readonly name: string;
  readonly logo: Shown;
  readonly banner: Shown;
}) {
  const logoSrc = useShownSrc(logo);
  const shownBanner = useShownSrc(banner);
  // An image that fails to load leaves the plain header, as the sidebar does.
  const [brokenBanner, setBrokenBanner] = useState<string | null>(null);
  const bannerSrc = shownBanner === brokenBanner ? null : shownBanner;

  return (
    <figure className="profile-preview" aria-label="Preview">
      <div className="profile-preview-rail">
        <span className="profile-preview-icon" data-logo={logoSrc !== null || undefined}>
          {logoSrc === null ? (
            initialsOf(name)
          ) : (
            <img src={logoSrc} alt="" width={40} height={40} draggable={false} />
          )}
        </span>
        <span className="profile-preview-dot" aria-hidden="true" />
        <span className="profile-preview-dot" aria-hidden="true" />
      </div>
      <div className="profile-preview-sidebar">
        <div className="profile-preview-banner" data-empty={bannerSrc === null || undefined}>
          {bannerSrc === null ? null : (
            <img
              className="profile-preview-banner-image"
              src={bannerSrc}
              alt=""
              draggable={false}
              onError={() => setBrokenBanner(bannerSrc)}
            />
          )}
          <span className="profile-preview-name">
            <span className="profile-preview-name-text">{name}</span>
            <Icon name="chevron-down" size={14} />
          </span>
        </div>
        <span className="profile-preview-lines" aria-hidden="true">
          <span />
          <span />
          <span />
          <span />
        </span>
      </div>
      <figcaption className="visually-hidden">
        {`${name}: ${logoSrc === null ? "initials" : "icon"} in the rail, ${bannerSrc === null ? "no banner" : "banner"} behind the name`}
      </figcaption>
    </figure>
  );
}

interface SlotProps {
  readonly kind: ProfileImageKind;
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
function ProfileSlot({ kind, image, pending, error, onFile, onRemove }: SlotProps) {
  const slot = PROFILE_SLOTS[kind];
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
      data-kind={kind}
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
      <div className="profile-slot-thumb" data-empty={src === null || undefined}>
        {src === null ? (
          <Icon name="image" size={20} />
        ) : (
          <img src={src} alt={`Current ${slot.noun}`} draggable={false} />
        )}
      </div>
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
            accept={PROFILE_IMAGE_TYPES.join(",")}
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

const SET = { logo: admin.setLogo, banner: admin.setBanner } as const;

const REMOVE = { logo: admin.removeLogo, banner: admin.removeBanner } as const;

/** What a slot is doing: an upload in flight, and the last refusal. */
interface SlotState {
  readonly pending: Pending | null;
  readonly error: string | null;
}

const IDLE: SlotState = { pending: null, error: null };

/**
 * The workspace profile, Discord's server profile: an icon for the rail and a banner for the top of
 * the sidebar, with a preview of both. Administrators upload, replace and remove them; everyone
 * else sees the preview.
 */
export function WorkspaceProfile() {
  const { workspace, replace } = useAdmin();
  const previews = useRef(new Set<string>());

  const [slots, setSlots] = useState<Record<ProfileImageKind, SlotState>>({
    logo: IDLE,
    banner: IDLE,
  });

  useEffect(() => {
    const urls = previews.current;

    return () => {
      for (const url of urls) {
        URL.revokeObjectURL(url);
      }
    };
  }, []);

  const set = (kind: ProfileImageKind, next: SlotState) =>
    setSlots((current) => ({ ...current, [kind]: next }));

  // A refusal shows under its slot; a lapsed password confirmation goes to the classic page.
  const refused = (kind: ProfileImageKind, error: Error) => {
    if (needsSudo(error)) {
      set(kind, IDLE);
      adminFailure(`Couldn't change the ${PROFILE_SLOTS[kind].noun}`, error);

      return;
    }

    set(kind, { pending: null, error: error.message });
  };

  const release = (preview: string) => {
    URL.revokeObjectURL(preview);
    previews.current.delete(preview);
  };

  const upload = (kind: ProfileImageKind, file: File) => {
    const problem = imageProblem(file);

    if (problem !== null) {
      set(kind, { pending: null, error: problem });

      return;
    }

    const preview = URL.createObjectURL(file);

    previews.current.add(preview);
    set(kind, { pending: { preview, file, percent: 0, saving: false }, error: null });

    const task = new UploadTask(file, browserDeps(actions.messages.startUpload), (snapshot) => {
      if (snapshot.phase === "uploading") {
        set(kind, {
          pending: {
            preview,
            file,
            percent: uploadPercent(snapshot.loaded, snapshot.total),
            saving: false,
          },
          error: null,
        });
      }
    });

    void task
      .start()
      .then(() => {
        const { phase, signedId, error } = task.snapshot;

        if (phase !== "done" || signedId === null) {
          throw new Error(error ?? "The upload didn't finish.");
        }

        set(kind, { pending: { preview, file, percent: 100, saving: true }, error: null });

        return SET[kind](signedId);
      })
      .then(
        (next) => {
          replace(next);
          set(kind, IDLE);
          toast({ title: `${PROFILE_SLOTS[kind].title} updated`, tone: "success" });
        },
        (error: Error) => refused(kind, error),
      )
      .finally(() => release(preview));
  };

  const remove = (kind: ProfileImageKind) => {
    set(kind, { pending: { preview: "", file: null, percent: 100, saving: true }, error: null });
    REMOVE[kind]().then(
      (next) => {
        replace(next);
        set(kind, IDLE);
      },
      (error: Error) => refused(kind, error),
    );
  };

  const shown = (kind: ProfileImageKind): Shown => {
    const pending = slots[kind].pending;

    return pending === null || pending.preview === ""
      ? serverImage(workspace, kind)
      : { url: pending.preview, stillUrl: null, file: pending.file };
  };

  return (
    <div className="profile">
      <ProfilePreview name={workspace.name} logo={shown("logo")} banner={shown("banner")} />
      {workspace.canAdminister ? (
        <div className="profile-slots">
          {(["logo", "banner"] as const).map((kind) => (
            <ProfileSlot
              key={kind}
              kind={kind}
              image={serverImage(workspace, kind)}
              pending={slots[kind].pending}
              error={slots[kind].error}
              onFile={(file) => upload(kind, file)}
              onRemove={() => remove(kind)}
            />
          ))}
          <p className="profile-formats text-meta">{PROFILE_FORMATS}</p>
        </div>
      ) : null}
    </div>
  );
}
