import { type FormEvent, useId, useState } from "react";
import type { Workspace } from "../../gen/Workspace.ts";
import { admin } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";
import { SettingsGroup, SettingsPage, useBusy } from "../settings/settings-parts.tsx";
import { adminFailure, useAdmin } from "./admin-parts.tsx";
import { descriptionError, vanitySlugError } from "./workspace-identity.ts";
import { WorkspaceProfile } from "./workspace-profile.tsx";

/** The name, saved on submit as the classic form saves it. */
function NameForm({ workspace }: { readonly workspace: Workspace }) {
  const { replace } = useAdmin();
  const [name, setName] = useState(workspace.name);
  const { busy, track } = useBusy();

  const submit = (event: FormEvent) => {
    event.preventDefault();
    void track(
      "name",
      admin.updateWorkspace({ name }).then(
        (next) => {
          replace(next);
          setName(next.name);
          toast({ title: "Workspace saved", tone: "success" });
        },
        (error: Error) => adminFailure("Couldn't save the workspace", error),
      ),
    );
  };

  return (
    <form className="settings-form settings-inline" onSubmit={submit}>
      <TextField
        label="Name"
        value={name}
        autoComplete="off"
        placeholder="Name this account"
        onChange={(event) => setName(event.target.value)}
      />
      <Button type="submit" variant="primary" loading={busy("name")} disabled={busy("name")}>
        Save
      </Button>
    </form>
  );
}

function UploadLimitForm({ workspace }: { readonly workspace: Workspace }) {
  const { replace } = useAdmin();
  const [limit, setLimit] = useState(String(workspace.uploadLimitBytes / (1024 * 1024)));
  const { busy, track } = useBusy();

  const submit = (event: FormEvent) => {
    event.preventDefault();
    void track(
      "uploads",
      admin.updateWorkspace({ uploadLimitBytes: Number(limit) * 1024 * 1024 }).then(
        (next) => {
          replace(next);
          toast({ title: "Upload limit saved", tone: "success" });
        },
        (error: Error) => adminFailure("Couldn't save the upload limit", error),
      ),
    );
  };

  return (
    <form className="settings-form settings-inline" onSubmit={submit}>
      <TextField
        label="Maximum file size (MB)"
        type="number"
        min="1"
        max="8589934591"
        step="1"
        required
        hint="Per file. 1 MB = 1,048,576 bytes."
        value={limit}
        onChange={(event) => setLimit(event.target.value)}
      />
      <Button type="submit" variant="primary" loading={busy("uploads")} disabled={busy("uploads")}>
        Save upload limit
      </Button>
    </form>
  );
}

function DescriptionForm({ workspace }: { readonly workspace: Workspace }) {
  const { replace } = useAdmin();
  const [description, setDescription] = useState(workspace.description);
  const { busy, track } = useBusy();
  const id = useId();
  const error = descriptionError(description);

  const submit = (event: FormEvent) => {
    event.preventDefault();

    if (error !== undefined) return;

    void track(
      "description",
      admin.updateWorkspace({ description: description.trim() }).then(
        (next) => {
          replace(next);
          toast({ title: "Description saved", tone: "success" });
        },
        (error: Error) => adminFailure("Couldn't save the description", error),
      ),
    );
  };

  return (
    <form className="settings-form" onSubmit={submit}>
      <div className="field">
        <label className="field-label" htmlFor={id}>
          Description
        </label>
        <textarea
          id={id}
          className="input settings-textarea"
          rows={3}
          value={description}
          aria-invalid={error !== undefined || undefined}
          aria-describedby={`${id}-hint`}
          onChange={(event) => setDescription(event.target.value)}
        />
        <p
          id={`${id}-hint`}
          className={error === undefined ? "field-hint" : "field-error"}
          aria-live="polite"
        >
          {error ?? "Plain text, up to 300 characters. Shown to members and on the join page."}
        </p>
      </div>
      <Button
        type="submit"
        variant="primary"
        loading={busy("description")}
        disabled={busy("description") || error !== undefined}
      >
        Save description
      </Button>
    </form>
  );
}

function VanityForm({ workspace }: { readonly workspace: Workspace }) {
  const { replace } = useAdmin();
  const [slug, setSlug] = useState(workspace.vanitySlug ?? "");
  const { busy, track } = useBusy();
  const error = vanitySlugError(slug);

  const inviteUrl =
    slug.trim() === "" || error !== undefined
      ? null
      : new URL(`/join/${slug.trim()}`, new URL(workspace.joinUrl, window.location.origin)).href;

  const saved = slug.trim() === workspace.vanitySlug;

  const submit = (event: FormEvent) => {
    event.preventDefault();

    if (error !== undefined) return;

    void track(
      "vanity",
      admin.updateWorkspace({ vanitySlug: slug.trim() }).then(
        (next) => {
          replace(next);
          toast({ title: "Vanity invite saved", tone: "success" });
        },
        (error: Error) => adminFailure("Couldn't save the vanity invite", error),
      ),
    );
  };

  const copy = () => {
    if (inviteUrl === null || !saved) return;
    void navigator.clipboard.writeText(inviteUrl).then(
      () => toast({ title: "Vanity invite copied", tone: "success" }),
      () => toast({ title: "Couldn't copy to the clipboard", tone: "danger" }),
    );
  };

  return (
    <form className="settings-form" onSubmit={submit}>
      <TextField
        label="Vanity slug"
        value={slug}
        error={error}
        hint="Optional. Clear it to remove the vanity invite. Resetting the join link keeps it working."
        onChange={(event) => setSlug(event.target.value)}
      />
      {inviteUrl === null ? null : (
        <TextField
          label="Vanity invite URL"
          value={inviteUrl}
          readOnly
          hint={saved ? "Ready to share." : "Save this slug before sharing."}
          onFocus={(event) => event.target.select()}
        />
      )}
      <div className="settings-actions">
        <Button
          type="submit"
          variant="primary"
          loading={busy("vanity")}
          disabled={busy("vanity") || error !== undefined}
        >
          Save vanity slug
        </Button>
        {inviteUrl === null ? null : (
          <Button variant="secondary" icon="copy" disabled={!saved} onClick={copy}>
            Copy vanity invite
          </Button>
        )}
      </div>
    </form>
  );
}

/** The join link, to copy or share; administrators can swap it for a new one. */
function JoinLink() {
  const { workspace, replace } = useAdmin();
  const { busy, track } = useBusy();

  const copy = () => {
    void navigator.clipboard.writeText(workspace.joinUrl).then(
      () => toast({ title: "Join link copied", tone: "success" }),
      () => toast({ title: "Couldn't copy to the clipboard", tone: "danger" }),
    );
  };

  const reset = () => {
    void track(
      "join",
      admin.resetJoinCode().then(
        (next) => {
          replace(next);
          toast({ title: "New join link ready", tone: "success" });
        },
        (error: Error) => adminFailure("Couldn't make a new join link", error),
      ),
    );
  };

  return (
    <div className="settings-form">
      <TextField
        label="Share to invite more people"
        value={workspace.joinUrl}
        readOnly
        onFocus={(event) => event.target.select()}
      />
      <div className="settings-actions">
        <Button variant="secondary" icon="copy" onClick={copy}>
          Copy join link
        </Button>
        {workspace.canAdminister ? (
          <Button
            variant="ghost"
            icon="rotate-ccw"
            loading={busy("join")}
            disabled={busy("join")}
            onClick={reset}
          >
            New join link
          </Button>
        ) : null}
      </div>
    </div>
  );
}

/**
 * Workspace: the classic account page's top half. Everyone sees the profile (icon and banner),
 * the name and the join link; administrators also rename the workspace, change its icon and
 * banner, decide who may create rooms and swap the join link for a new one.
 */
export function WorkspaceSection() {
  const { workspace, replace } = useAdmin();
  const { busy, track } = useBusy();

  const restrict = (restrictRoomCreationToAdministrators: boolean) => {
    void track(
      "restrict",
      admin
        .updateWorkspace({ restrictRoomCreationToAdministrators })
        .then(replace, (error: Error) => adminFailure("Couldn't save the workspace", error)),
    );
  };

  return (
    <SettingsPage
      title={workspace.canAdminister ? "Workspace" : workspace.name}
      description={`Smartfire™ version ${workspace.version}`}
    >
      <SettingsGroup
        title="Workspace profile"
        description="The icon and banner everyone sees in the rail and at the top of the sidebar."
      >
        <WorkspaceProfile />
        <section aria-label={`About ${workspace.name}`} className="workspace-about">
          <h3 className="text-title">About</h3>
          <p>{workspace.description === "" ? "No description yet." : workspace.description}</p>
        </section>
      </SettingsGroup>
      {workspace.canAdminister ? (
        <SettingsGroup title="Description">
          <DescriptionForm key={workspace.description} workspace={workspace} />
        </SettingsGroup>
      ) : null}
      {workspace.canAdminister ? (
        <SettingsGroup title="Name">
          <NameForm key={workspace.name} workspace={workspace} />
        </SettingsGroup>
      ) : null}
      {workspace.canAdminister ? (
        <SettingsGroup title="Rooms">
          <Toggle
            checked={workspace.restrictRoomCreationToAdministrators}
            disabled={busy("restrict")}
            label="Must be admin to create new rooms"
            onCheckedChange={restrict}
          />
        </SettingsGroup>
      ) : null}
      <SettingsGroup title="Invite">
        <JoinLink />
        {workspace.canAdminister ? (
          <VanityForm key={workspace.vanitySlug} workspace={workspace} />
        ) : null}
      </SettingsGroup>
      <SettingsGroup
        title="Uploads"
        description={`Up to ${workspace.uploadLimitBytes / (1024 * 1024)} MB per file.`}
      >
        {workspace.canAdminister ? (
          <UploadLimitForm key={workspace.uploadLimitBytes} workspace={workspace} />
        ) : null}
      </SettingsGroup>
    </SettingsPage>
  );
}
