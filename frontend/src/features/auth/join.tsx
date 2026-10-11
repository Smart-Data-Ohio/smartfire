import { Link, useParams } from "@tanstack/react-router";
import { type ChangeEvent, type FormEvent, useEffect, useId, useRef, useState } from "react";
import type { JoinPage as JoinPageData } from "../../gen/JoinPage.ts";
import {
  auth,
  inlineSignedOutBoot,
  type JoinNext,
  type JoinTarget,
  type SignedOutBootData,
} from "../../sync/auth.ts";
import { Button, Spinner } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { TextField } from "../../ui/text-field.tsx";
import {
  AuthHeader,
  AuthHelp,
  AuthScreen,
  refusalMessage,
  TranslateButton,
  useDocumentTitle,
  useFollowNext,
  useRestoredFromCache,
  useSignedOutBoot,
} from "./auth-parts.tsx";
import { EMAIL_TRANSLATIONS, NAME_TRANSLATIONS, PASSWORD_TRANSLATIONS } from "./translations.ts";

/** `/app/join/:joinCode`: joining with the workspace's join code. */
export function JoinCodeRoute() {
  const { joinCode } = useParams({ from: "/join/$joinCode" });

  return <JoinPage key={joinCode} target={{ via: "code", code: joinCode }} />;
}

/** `/app/invite/:token`: joining with a workspace invite. */
export function InviteRoute() {
  const { token } = useParams({ from: "/invite/$token" });

  return <JoinPage key={token} target={{ via: "invite", token }} />;
}

type Load =
  | { readonly status: "loading" }
  | { readonly status: "failed"; readonly message: string }
  | { readonly status: "notFound" }
  | { readonly status: "ready"; readonly page: JoinPageData };

/**
 * The retained join page (crates/retained_pages/templates/users/new.html) in the SPA: the
 * workspace's name, logo and description over an avatar, name, email address and password, or why
 * an invite can no longer be used; then sign-in for people who already have an account, and whom
 * to ask for help. Joining signs the new account in and enters the app.
 */
export function JoinPage({ target }: { readonly target: JoinTarget }) {
  const titleId = useId();
  const follow = useFollowNext();
  const { load: bootLoad } = useSignedOutBoot();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [inline] = useState(() => inlineSignedOutBoot());
  const asked = useRef(false);

  const answered = (next: JoinNext) => {
    switch (next.kind) {
      case "join":
      case "inviteInvalid":
        setLoad({ status: "ready", page: next });

        return;
      // A wrong join code: the retained page's empty 404.
      case "error":
        setLoad({ status: "notFound" });

        return;
      default:
        // Already signed in: on to the app, as the retained page redirects.
        follow(next);
    }
  };

  const ask = () => {
    setLoad({ status: "loading" });
    auth
      .joinPage(target)
      .then(answered, (error: Error) => setLoad({ status: "failed", message: error.message }));
  };

  // Once per page: development's double effect would ask twice.
  // biome-ignore lint/correctness/useExhaustiveDependencies: keyed on the code or token
  useEffect(() => {
    if (asked.current) return;

    asked.current = true;
    ask();
  }, []);

  const workspace = load.status === "ready" ? load.page.workspace : inline?.workspace;
  const title = joinTitle(workspace?.name ?? null);
  const boot = bootLoad.status === "ready" ? bootLoad.boot : null;

  useDocumentTitle(title);

  if (load.status === "ready") {
    return load.page.kind === "join" ? (
      <JoinForm target={target} page={load.page} boot={boot} />
    ) : (
      <DeadInvite page={load.page} boot={boot} />
    );
  }

  return (
    <AuthScreen
      labelledBy={titleId}
      busy={load.status === "loading"}
      below={
        boot?.helpContact === null || boot === null || load.status === "loading" ? undefined : (
          <AuthHelp contact={boot.helpContact} version={boot.version} />
        )
      }
    >
      <AuthHeader titleId={titleId} title={title} logoUrl={workspace?.logoUrl ?? null}>
        {load.status === "notFound" ? (
          <p className="auth-view-lede" role="alert">
            This join link isn't valid.
          </p>
        ) : null}
      </AuthHeader>
      <PendingBody load={load} retry={ask} />
    </AuthScreen>
  );
}

/** The card's body before there is a form: waiting, a failure to retry, or a wrong code. */
function PendingBody({
  load,
  retry,
}: {
  readonly load: Exclude<Load, { readonly status: "ready" }>;
  readonly retry: () => void;
}) {
  switch (load.status) {
    case "loading":
      return (
        <p className="auth-view-wait" role="status">
          <Spinner />
          One moment…
        </p>
      );

    case "failed":
      return (
        <div className="auth-view-stack" role="alert">
          <p className="auth-view-lede">{load.message}</p>
          <Button variant="primary" size="lg" className="auth-view-wide" onClick={retry}>
            Try again
          </Button>
        </div>
      );

    case "notFound":
      return (
        <>
          <p className="auth-view-note">
            Check the link, or ask a workspace administrator for a new one.
          </p>
          <SignInNote />
        </>
      );
  }
}

/** "Join Harbor", as the retained page's heading. */
function joinTitle(name: string | null): string {
  return name === null ? "Join" : `Join ${name}`;
}

/** Under every state: the way to the SPA's sign-in page. */
function SignInNote() {
  return (
    <p className="auth-view-note">
      Already have an account? <Link to="/session/new">Sign in</Link>
    </p>
  );
}

type Field = "name" | "emailAddress" | "password";

type FieldErrors = Partial<Record<Field, string>>;

const FIELDS: readonly Field[] = ["name", "emailAddress", "password"];

/** What the retained form's `required` and `type="email"` fields ask for, said per field. */
function validate(values: Record<Field, string>, emailValid: boolean): FieldErrors {
  const errors: FieldErrors = {};

  if (values.name.trim() === "") errors.name = "Enter your name.";

  if (values.emailAddress.trim() === "") {
    errors.emailAddress = "Enter your email address.";
  } else if (!emailValid) {
    errors.emailAddress = "Enter an email address like you@example.com.";
  }

  if (values.password === "") errors.password = "Enter a password.";

  return errors;
}

function JoinForm({
  target,
  page,
  boot,
}: {
  readonly target: JoinTarget;
  readonly page: Extract<JoinPageData, { kind: "join" }>;
  readonly boot: SignedOutBootData | null;
}) {
  const titleId = useId();
  const follow = useFollowNext();

  const refs = {
    name: useRef<HTMLInputElement | null>(null),
    emailAddress: useRef<HTMLInputElement | null>(null),
    password: useRef<HTMLInputElement | null>(null),
  };

  const [values, setValues] = useState<Record<Field, string>>({
    name: "",
    emailAddress: "",
    password: "",
  });

  const [avatar, setAvatar] = useState<File | null>(null);
  const [errors, setErrors] = useState<FieldErrors>({});
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const [dead, setDead] = useState<Extract<JoinPageData, { kind: "inviteInvalid" }> | null>(null);

  // Back from the app (or after leaving for sign-in) restores the page as it was left: usable,
  // and sending nothing by itself.
  useRestoredFromCache(() => setBusy(false));

  const refuse = (next: FieldErrors) => {
    setErrors(next);
    setAttempt((count) => count + 1);
    setBusy(false);

    const first = FIELDS.find((field) => next[field] !== undefined);

    if (first !== undefined) refs[first].current?.focus();
  };

  const answered = (next: JoinNext) => {
    switch (next.kind) {
      case "error": {
        const { fieldErrors } = next;
        const named: FieldErrors = {};

        for (const field of FIELDS) {
          const message = fieldErrors[field]?.[0];

          if (message !== undefined) named[field] = message;
        }

        // Anything not about one field (a malformed request, an access gate) goes with the
        // password, which is typed last, as the sign-in page says it.
        if (Object.keys(named).length === 0) named.password = refusalMessage(fieldErrors, "base");

        refuse(named);

        return;
      }

      // The invite died while the form was open (its last use was taken): say so instead.
      case "inviteInvalid":
        setDead(next);

        return;
      case "join":
        setBusy(false);

        return;
      default:
        // Busy stays on while the page leaves, so a second press can't create it twice.
        follow(next);
    }
  };

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (busy) return;

    const emailValid = refs.emailAddress.current?.validity.typeMismatch !== true;
    const invalid = validate(values, emailValid);

    if (Object.keys(invalid).length > 0) {
      refuse(invalid);

      return;
    }

    setBusy(true);
    auth
      .join(target, values, avatar)
      .then(answered, (failure: Error) => refuse({ password: failure.message }));
  };

  const change = (field: Field) => (event: ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;

    setValues((current) => ({ ...current, [field]: value }));

    if (errors[field] !== undefined) {
      setErrors((current) => ({ ...current, [field]: undefined }));
    }
  };

  if (dead !== null) {
    return <DeadInvite page={dead} boot={boot} />;
  }

  const { workspace } = page;
  const contact = page.helpContact;

  return (
    <AuthScreen
      labelledBy={titleId}
      below={
        contact === null || boot === null ? undefined : (
          <AuthHelp contact={contact} version={boot.version} />
        )
      }
    >
      <AuthHeader titleId={titleId} title={joinTitle(workspace.name)} logoUrl={workspace.logoUrl}>
        {workspace.description === "" ? null : (
          <p className="auth-view-description">{workspace.description}</p>
        )}
        <p className="auth-view-lede">Create your account to start chatting.</p>
      </AuthHeader>

      <form className="auth-view-form" onSubmit={submit} aria-labelledby={titleId} noValidate>
        <AvatarPicker file={avatar} onChange={setAvatar} disabled={busy} />
        <TextField
          ref={refs.name}
          label="Name"
          labelAccessory={<TranslateButton field="Name" translations={NAME_TRANSLATIONS} />}
          name="name"
          required
          autoFocus
          autoComplete="name"
          data-1p-ignore
          value={values.name}
          onChange={change("name")}
          error={errors.name}
          attempt={attempt}
        />
        <TextField
          ref={refs.emailAddress}
          label="Email address"
          labelAccessory={
            <TranslateButton field="Email address" translations={EMAIL_TRANSLATIONS} />
          }
          type="email"
          name="email_address"
          required
          autoComplete="username"
          placeholder="you@example.com"
          value={values.emailAddress}
          onChange={change("emailAddress")}
          error={errors.emailAddress}
          attempt={attempt}
        />
        <TextField
          ref={refs.password}
          label="Password"
          labelAccessory={<TranslateButton field="Password" translations={PASSWORD_TRANSLATIONS} />}
          type="password"
          name="password"
          required
          autoComplete="new-password"
          maxLength={72}
          value={values.password}
          onChange={change("password")}
          error={errors.password}
          attempt={attempt}
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
      </form>

      <SignInNote />
    </AuthScreen>
  );
}

/** An invite that died between showing the form and sending it. */
function DeadInvite({
  page,
  boot,
}: {
  readonly page: Extract<JoinPageData, { kind: "inviteInvalid" }>;
  readonly boot: SignedOutBootData | null;
}) {
  const titleId = useId();
  const contact = page.helpContact;

  return (
    <AuthScreen
      labelledBy={titleId}
      below={
        contact === null || boot === null ? undefined : (
          <AuthHelp contact={contact} version={boot.version} />
        )
      }
    >
      <AuthHeader
        titleId={titleId}
        title={joinTitle(page.workspace.name)}
        logoUrl={page.workspace.logoUrl}
      >
        <p className="auth-view-lede" role="alert">
          This invite is no longer valid.
        </p>
      </AuthHeader>
      <p className="auth-view-note">
        {page.reason} Ask a workspace administrator for a new invite.
      </p>
      <SignInNote />
    </AuthScreen>
  );
}

/**
 * The round picture over the form: a person until a file is chosen, then that picture, before
 * anything is uploaded (the retained auth.js preview). The file input stays in the tab order under
 * the circle, which shows its focus.
 */
function AvatarPicker({
  file,
  onChange,
  disabled,
}: {
  readonly file: File | null;
  readonly onChange: (file: File | null) => void;
  readonly disabled: boolean;
}) {
  const [preview, setPreview] = useState<string | null>(null);

  useEffect(() => {
    if (file === null) {
      setPreview(null);

      return;
    }

    const url = URL.createObjectURL(file);

    setPreview(url);

    return () => URL.revokeObjectURL(url);
  }, [file]);

  const label = file === null ? "Upload avatar" : "Change avatar";

  return (
    <label className="auth-view-avatar" title={label}>
      {preview === null ? (
        <Icon name="user" size={32} />
      ) : (
        <img className="auth-view-avatar-preview" src={preview} alt="" />
      )}
      <span className="auth-view-avatar-badge" aria-hidden="true">
        <Icon name="camera" size={16} />
      </span>
      <input
        type="file"
        name="avatar"
        accept="image/*"
        className="visually-hidden"
        disabled={disabled}
        onChange={(event) => onChange(event.target.files?.[0] ?? null)}
      />
      <span className="visually-hidden">
        {file === null ? label : `${label}, ${file.name} chosen`}
      </span>
    </label>
  );
}
