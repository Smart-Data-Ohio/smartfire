import { type FormEvent, useRef, useState } from "react";
import { browserDeps, UploadTask } from "../../lib/upload/direct-upload.ts";
import { actions } from "../../sync/runtime.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { Avatar } from "../../ui/avatar.tsx";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { classicPage, fieldError } from "./settings-format.ts";
import {
  ClassicLink,
  FieldError,
  fieldsOf,
  SettingsGroup,
  SettingsPage,
  toastFailure,
  useSettings,
} from "./settings-parts.tsx";

type Fields = Readonly<Record<string, readonly string[]>>;

/** The avatar: the picture, a way to upload a new one, and Remove when one is attached. */
function AvatarPicker() {
  const { settings, replace } = useSettings();
  const input = useRef<HTMLInputElement | null>(null);
  const [busy, setBusy] = useState<"uploading" | "removing" | null>(null);
  const { profile } = settings;

  const upload = (file: File) => {
    setBusy("uploading");

    const task = new UploadTask(file, browserDeps(actions.messages.startUpload), () => undefined);

    task
      .start()
      .then(() => {
        const { phase, signedId, error } = task.snapshot;

        if (phase !== "done" || signedId === null) {
          throw new Error(error ?? "The upload didn't finish.");
        }

        return settingsActions.setAvatar(signedId);
      })
      .then(
        (next) => {
          replace(next);
          toast({ title: "Avatar updated", tone: "success" });
        },
        (error: Error) => toastFailure("Couldn't update your avatar", error),
      )
      .finally(() => setBusy(null));
  };

  const remove = () => {
    setBusy("removing");
    settingsActions
      .removeAvatar()
      .then(replace, (error: Error) => toastFailure("Couldn't remove your avatar", error))
      .finally(() => setBusy(null));
  };

  return (
    <div className="settings-avatar">
      <Avatar
        key={profile.avatarUrl}
        name={profile.name}
        userId={profile.userId}
        src={profile.avatarUrl}
        size={80}
      />
      <div className="settings-avatar-actions">
        <input
          ref={input}
          type="file"
          accept="image/*"
          hidden
          aria-label="Upload avatar"
          onChange={(event) => {
            const file = event.target.files?.[0];

            event.target.value = "";

            if (file !== undefined) {
              upload(file);
            }
          }}
        />
        <Button
          variant="secondary"
          size="sm"
          icon="cloud-upload"
          loading={busy === "uploading"}
          loadingLabel="Uploading…"
          disabled={busy !== null}
          onClick={() => input.current?.click()}
        >
          Upload photo
        </Button>
        {profile.avatarAttached ? (
          <Button
            variant="ghost"
            size="sm"
            icon="trash"
            loading={busy === "removing"}
            disabled={busy !== null}
            onClick={remove}
          >
            Remove
          </Button>
        ) : null}
      </div>
    </div>
  );
}

/**
 * Profile: avatar, name, email (with the current password when it changes and the account has
 * one), a new password, bio and GitHub username, saved together as the classic form does.
 */
export function ProfileSection() {
  const { settings, replace } = useSettings();
  const { profile } = settings;
  const [name, setName] = useState(profile.name);
  const [email, setEmail] = useState(profile.emailAddress ?? "");
  const [currentPassword, setCurrentPassword] = useState("");
  const [password, setPassword] = useState("");
  const [bio, setBio] = useState(profile.bio ?? "");
  const [github, setGithub] = useState(profile.githubLogin ?? "");
  const [fields, setFields] = useState<Fields>({});
  const [attempt, setAttempt] = useState(0);
  const [saving, setSaving] = useState(false);

  const emailChanged = email !== (profile.emailAddress ?? "");
  const githubChanged = !profile.githubVerified && github !== (profile.githubLogin ?? "");

  const changed =
    name !== profile.name ||
    emailChanged ||
    password !== "" ||
    bio !== (profile.bio ?? "") ||
    githubChanged;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    settingsActions
      .updateProfile({
        name: name === profile.name ? null : name,
        emailAddress: emailChanged ? email : null,
        currentPassword: emailChanged && profile.hasPassword ? currentPassword : null,
        password: password === "" ? null : password,
        bio: bio === (profile.bio ?? "") ? null : bio,
        githubLogin: githubChanged ? github : null,
      })
      .then(
        (next) => {
          replace(next);
          setFields({});
          setCurrentPassword("");
          setPassword("");
          setName(next.profile.name);
          setEmail(next.profile.emailAddress ?? "");
          setBio(next.profile.bio ?? "");
          setGithub(next.profile.githubLogin ?? "");
          toast({ title: "Profile saved", tone: "success" });
        },
        (error: Error) => {
          const named = fieldsOf(error);

          setFields(named);
          setAttempt((count) => count + 1);

          if (Object.keys(named).length === 0) {
            toastFailure("Couldn't save your profile", error);
          }
        },
      )
      .finally(() => setSaving(false));
  };

  return (
    <SettingsPage title="Profile" description="How you appear to everyone in the workspace.">
      <AvatarPicker />
      <form className="settings-form" onSubmit={submit} noValidate>
        <TextField
          label="Name"
          value={name}
          required
          autoComplete="name"
          placeholder="Enter your name"
          error={fieldError(fields, "name", "Name")}
          attempt={attempt}
          onChange={(event) => setName(event.target.value)}
        />
        <TextField
          label="Email address"
          type="email"
          value={email}
          autoComplete="username"
          placeholder="Enter your email address"
          error={fieldError(fields, "emailAddress", "Email address")}
          attempt={attempt}
          onChange={(event) => setEmail(event.target.value)}
        />
        {profile.hasPassword && emailChanged ? (
          <TextField
            label="Current password"
            hint="Required to change your email address."
            type="password"
            value={currentPassword}
            maxLength={72}
            autoComplete="current-password"
            error={fieldError(fields, "currentPassword", "Current password")}
            attempt={attempt}
            onChange={(event) => setCurrentPassword(event.target.value)}
          />
        ) : null}
        <TextField
          label="New password"
          hint="Leave blank to keep your current password."
          type="password"
          value={password}
          maxLength={72}
          autoComplete="new-password"
          error={fieldError(fields, "password", "Password")}
          attempt={attempt}
          onChange={(event) => setPassword(event.target.value)}
        />
        <div className="settings-field">
          <label htmlFor="settings-bio" className="settings-label">
            Bio
          </label>
          <textarea
            id="settings-bio"
            className="input settings-textarea"
            value={bio}
            rows={3}
            maxLength={200}
            placeholder="A few words about yourself…"
            onChange={(event) => setBio(event.target.value)}
          />
          <FieldError message={fieldError(fields, "bio", "Bio")} />
        </div>
        <TextField
          label="GitHub username"
          hint={
            profile.githubVerified
              ? "Set by your linked GitHub account. Disconnect GitHub to edit it."
              : "Link your GitHub username to receive review requests in your activity inbox."
          }
          value={github}
          disabled={profile.githubVerified}
          autoComplete="off"
          placeholder="GitHub username"
          error={fieldError(fields, "githubLogin", "GitHub username")}
          attempt={attempt}
          onChange={(event) => setGithub(event.target.value)}
        />
        <div className="settings-actions">
          <Button type="submit" loading={saving} disabled={!changed}>
            Save profile
          </Button>
        </div>
      </form>
      <SettingsGroup title="More on the classic page">
        <p>Rooms you're in, two-step verification and signing in on another device.</p>
        <ClassicLink href={classicPage(settings.integrations.managePath)}>
          Open the classic profile
        </ClassicLink>
      </SettingsGroup>
    </SettingsPage>
  );
}
