import { useNavigate } from "@tanstack/react-router";
import { type ChangeEvent, type FormEvent, useEffect, useId, useState } from "react";
import { type AuthNext, auth, inlineSignedOutBoot } from "../../sync/auth.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { pageExit, signedOutPageFor } from "./auth-navigation.ts";
import {
  AuthHeader,
  AuthScreen,
  refusalMessage,
  TranslateButton,
  useDocumentTitle,
  useFollowNext,
  useRestoredFromCache,
} from "./auth-parts.tsx";
import { EMAIL_TRANSLATIONS, NAME_TRANSLATIONS, PASSWORD_TRANSLATIONS } from "./translations.ts";

const TITLE = "Set up Smartfire";

const AVATAR_LABEL = "Add your avatar";

type Opening =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready" };

/** Whether the form can open, and a way to ask again after a failure. */
interface FirstRunOpening {
  readonly opening: Opening;
  readonly retry: () => void;
}

/**
 * `/app/first_run`: the retained first run (crates/retained_pages/templates/first_runs/show.html)
 * in the SPA. A new instance's first account, which administers the workspace: an avatar, a name,
 * an email address and a password. Once the workspace exists the page goes home, as the retained
 * one redirects; setting up signs the administrator in and enters the app.
 */
export function FirstRunPage() {
  const titleId = useId();
  const { opening, retry } = useFirstRunOpening();

  useDocumentTitle(TITLE);

  if (opening.status === "ready") return <FirstRunForm />;

  return (
    <AuthScreen labelledBy={titleId} busy={opening.status === "loading"}>
      <AuthHeader titleId={titleId} title={TITLE} />
      {opening.status === "error" ? (
        <div className="auth-view-stack" role="alert">
          <p className="auth-view-lede">{opening.message}</p>
          <Button variant="primary" size="lg" className="auth-view-wide" onClick={retry}>
            Try again
          </Button>
        </div>
      ) : (
        <p className="auth-view-wait" role="status">
          <Spinner />
          One moment…
        </p>
      )}
    </AuthScreen>
  );
}

/**
 * Whether the form can open. The Rust shell only draws this page while there is no workspace, so
 * its inlined boot (or the sign-in page's, which came here because first run is pending) opens it
 * at once; otherwise (the Vite dev page) the server is asked, and a set-up workspace goes home.
 */
function useFirstRunOpening(): FirstRunOpening {
  const navigate = useNavigate();

  const [opening, setOpening] = useState<Opening>(() =>
    inlineSignedOutBoot()?.firstRunPending === true ? { status: "ready" } : { status: "loading" },
  );

  useEffect(() => {
    if (opening.status !== "loading") return;

    let live = true;

    auth.firstRunState().then(
      (state) => {
        if (!live) return;

        if (state.kind === "pending") {
          setOpening({ status: "ready" });

          return;
        }

        const page = signedOutPageFor(state.location);

        if (page === null) {
          pageExit.replace(state.location);
        } else {
          void navigate({ to: page, replace: true });
        }
      },
      (failure: Error) => {
        if (live) setOpening({ status: "error", message: failure.message });
      },
    );

    return () => {
      live = false;
    };
  }, [opening.status, navigate]);

  return { opening, retry: () => setOpening({ status: "loading" }) };
}

function FirstRunForm() {
  const titleId = useId();
  const follow = useFollowNext();
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [avatar, setAvatar] = useState<File | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useRestoredFromCache(() => setBusy(false));

  const refuse = (message: string) => {
    setError(message);
    setBusy(false);
  };

  const answered = (next: AuthNext) => {
    if (next.kind === "error") {
      refuse(refusalMessage(next.fieldErrors, "base"));

      return;
    }

    // Busy stays on while the page leaves for the app, so a second press can't send it again.
    follow(next);
  };

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (busy) return;

    setBusy(true);
    setError("");
    auth
      .firstRun({ name, emailAddress: email, password }, avatar)
      .then(answered, (failure: Error) => refuse(failure.message));
  };

  return (
    <AuthScreen labelledBy={titleId}>
      <AuthHeader
        titleId={titleId}
        title={TITLE}
        lede="Create the first account. It administers the workspace."
      />

      <form className="auth-view-form" onSubmit={submit} aria-labelledby={titleId}>
        <AvatarPicker file={avatar} onPick={setAvatar} />
        <TextField
          label="Name"
          labelAccessory={<TranslateButton field="Name" translations={NAME_TRANSLATIONS} />}
          name="name"
          required
          autoFocus
          autoComplete="name"
          data-1p-ignore
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
        <TextField
          label="Email address"
          labelAccessory={
            <TranslateButton field="Email address" translations={EMAIL_TRANSLATIONS} />
          }
          type="email"
          name="email_address"
          required
          autoComplete="username"
          placeholder="you@example.com"
          value={email}
          onChange={(event) => setEmail(event.target.value)}
        />
        <TextField
          label="Password"
          labelAccessory={<TranslateButton field="Password" translations={PASSWORD_TRANSLATIONS} />}
          type="password"
          name="password"
          required
          autoComplete="new-password"
          maxLength={72}
          value={password}
          onChange={(event) => setPassword(event.target.value)}
        />
        <Button
          type="submit"
          variant="primary"
          size="lg"
          className="auth-view-wide"
          loading={busy}
          loadingLabel="Creating account…"
        >
          Create account
        </Button>
        {/* A line kept for a refusal or a failure, so nothing moves when one arrives. */}
        <p className="auth-view-form-error" role="alert">
          {error}
        </p>
      </form>
    </AuthScreen>
  );
}

/**
 * The round avatar picker the retained form draws: a person until a picture is chosen, then the
 * picture, with a camera badge. Under it, a line that names the choice.
 */
function AvatarPicker({
  file,
  onPick,
}: {
  readonly file: File | null;
  readonly onPick: (file: File | null) => void;
}) {
  const noteId = useId();
  const preview = useObjectUrl(file);

  const picked = (event: ChangeEvent<HTMLInputElement>) => {
    onPick(event.target.files?.[0] ?? null);
  };

  return (
    <div className="auth-view-avatar-field">
      <label className="auth-view-avatar" title={AVATAR_LABEL}>
        {preview === null ? (
          <Icon name="user" size={32} />
        ) : (
          <img className="auth-view-avatar-preview" src={preview} alt="" />
        )}
        <span className="auth-view-avatar-badge">
          <Icon name="camera" size={16} />
        </span>
        <input
          type="file"
          name="avatar"
          accept="image/*"
          className="visually-hidden"
          aria-label={AVATAR_LABEL}
          aria-describedby={noteId}
          onChange={picked}
        />
      </label>
      <p id={noteId} className="auth-view-avatar-note">
        {file?.name ?? "Optional"}
      </p>
    </div>
  );
}

/** An object URL for `file` while it is chosen, revoked when it changes or the page goes. */
function useObjectUrl(file: File | null): string | null {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    if (file === null) {
      setUrl(null);

      return;
    }

    const created = URL.createObjectURL(file);

    setUrl(created);

    return () => URL.revokeObjectURL(created);
  }, [file]);

  return url;
}
