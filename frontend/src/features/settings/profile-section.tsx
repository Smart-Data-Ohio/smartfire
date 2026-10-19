import { type FormEvent, useState } from "react";
import { settings as settingsActions } from "../../sync/settings.ts";
import { Avatar } from "../../ui/avatar.tsx";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { USER_AVATAR_SLOT, USER_BANNER_SLOT } from "../admin/profile-format.ts";
import { ProfileSlot, type Shown, useImageSlot } from "../admin/profile-slot.tsx";
import { ProfileHero } from "../people/profile-banner.tsx";
import { fieldError } from "./settings-format.ts";
import {
  FieldError,
  fieldsOf,
  SettingsPage,
  toastFailure,
  useSettings,
} from "./settings-parts.tsx";

type Fields = Readonly<Record<string, readonly string[]>>;

interface Draft {
  readonly name: string;
  readonly nickname: string;
  readonly pronouns: string;
  readonly bio: string;
}

const avatarProblem = (file: File) => (file.type.startsWith("image/") ? null : "Choose an image.");

function ProfilePreview({
  userId,
  draft,
  savedName,
  avatar,
  banner,
}: {
  readonly userId: number;
  readonly draft: Draft;
  readonly savedName: string;
  readonly avatar: Shown;
  readonly banner: Shown;
}) {
  const accountName = draft.name.trim() || savedName;
  const nickname = draft.nickname.trim();
  const name = nickname || accountName;
  const pronouns = draft.pronouns.trim();
  const bio = draft.bio.trim();
  const aside = [nickname === "" ? "" : accountName, pronouns].filter((part) => part !== "");

  return (
    <figure className="profile-editor-card" aria-label="Profile preview">
      <ProfileHero
        userId={userId}
        name={name}
        url={banner.url}
        stillUrl={banner.stillUrl}
        file={banner.file ?? null}
        size="card"
      >
        <Avatar
          key={avatar.url}
          name={name}
          userId={userId}
          {...(avatar.url === null ? {} : { src: avatar.url })}
          size={64}
          decorative
        />
      </ProfileHero>
      <figcaption className="profile-editor-identity">
        <span className="profile-editor-name">{name}</span>
        {aside.length === 0 ? null : (
          <span className="profile-editor-aside">{aside.join(" · ")}</span>
        )}
        {bio === "" ? null : <span className="profile-editor-bio">{bio}</span>}
      </figcaption>
    </figure>
  );
}

function ProfileEditor({ draft }: { readonly draft: Draft }) {
  const { settings, replace } = useSettings();
  const { profile } = settings;

  const avatar = useImageSlot({
    save: settingsActions.setAvatar,
    remove: settingsActions.removeAvatar,
    onChange: replace,
    savedTitle: "Avatar updated",
    problem: avatarProblem,
  });

  const banner = useImageSlot({
    save: settingsActions.setBanner,
    remove: settingsActions.removeBanner,
    onChange: replace,
    savedTitle: `${USER_BANNER_SLOT.title} updated`,
  });

  const savedAvatar: Shown = {
    url: profile.avatarAttached ? profile.avatarUrl : null,
    stillUrl: null,
  };

  const savedBanner: Shown = { url: profile.bannerUrl, stillUrl: profile.bannerStillUrl };

  return (
    <div className="profile-editor">
      <div className="profile-editor-layout">
        <ProfilePreview
          userId={profile.userId}
          draft={draft}
          savedName={profile.name}
          avatar={avatar.shown({ url: profile.avatarUrl, stillUrl: null })}
          banner={banner.shown(savedBanner)}
        />
        <div className="profile-editor-slots">
          <ProfileSlot
            slot={USER_AVATAR_SLOT}
            accept="image/*"
            image={savedAvatar}
            pending={avatar.pending}
            error={avatar.error}
            onFile={avatar.upload}
            onRemove={avatar.remove}
          />
          <ProfileSlot
            slot={USER_BANNER_SLOT}
            image={savedBanner}
            pending={banner.pending}
            error={banner.error}
            onFile={banner.upload}
            onRemove={banner.remove}
          />
        </div>
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
  const [nickname, setNickname] = useState(profile.nickname ?? "");
  const [pronouns, setPronouns] = useState(profile.pronouns ?? "");
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
    nickname !== (profile.nickname ?? "") ||
    pronouns !== (profile.pronouns ?? "") ||
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
        pronouns: pronouns === (profile.pronouns ?? "") ? null : pronouns,
        nickname: nickname === (profile.nickname ?? "") ? null : nickname,
      })
      .then(
        (next) => {
          replace(next);
          setFields({});
          setCurrentPassword("");
          setPassword("");
          setName(next.profile.name);
          setNickname(next.profile.nickname ?? "");
          setPronouns(next.profile.pronouns ?? "");
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
      <ProfileEditor draft={{ name, nickname, pronouns, bio }} />
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
          label="Display nickname"
          hint="Optional. Leave blank to use your name."
          value={nickname}
          maxLength={32}
          autoComplete="nickname"
          error={fieldError(fields, "nickname", "Display nickname")}
          attempt={attempt}
          onChange={(event) => setNickname(event.target.value)}
        />
        <TextField
          label="Pronouns"
          value={pronouns}
          maxLength={40}
          placeholder="e.g. they/them"
          error={fieldError(fields, "pronouns", "Pronouns")}
          attempt={attempt}
          onChange={(event) => setPronouns(event.target.value)}
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
    </SettingsPage>
  );
}
