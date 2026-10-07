import { type FormEvent, useRef, useState } from "react";
import type { Workspace } from "../../gen/Workspace.ts";
import { browserDeps, UploadTask } from "../../lib/upload/direct-upload.ts";
import { admin } from "../../sync/admin.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";
import { SettingsGroup, SettingsPage, useBusy } from "../settings/settings-parts.tsx";
import { adminFailure, useAdmin } from "./admin-parts.tsx";

/** The logo, with Upload and (when one is attached) Remove for administrators. */
function Logo() {
  const { workspace, replace } = useAdmin();
  const input = useRef<HTMLInputElement | null>(null);
  const { busy, track } = useBusy();

  const upload = (file: File) => {
    const task = new UploadTask(file, browserDeps(actions.messages.startUpload), () => undefined);

    void track(
      "logo",
      task
        .start()
        .then(() => {
          const { phase, signedId, error } = task.snapshot;

          if (phase !== "done" || signedId === null) {
            throw new Error(error ?? "The upload didn't finish.");
          }

          return admin.setLogo(signedId);
        })
        .then(replace, (error: Error) => adminFailure("Couldn't update the logo", error)),
    );
  };

  const remove = () => {
    void track(
      "logo",
      admin
        .removeLogo()
        .then(replace, (error: Error) => adminFailure("Couldn't remove the logo", error)),
    );
  };

  return (
    <div className="settings-avatar">
      <img
        className="admin-logo"
        src={workspace.logoUrl}
        alt={`${workspace.name} logo`}
        width={80}
        height={80}
      />
      {workspace.canAdminister ? (
        <div className="settings-avatar-actions">
          <input
            ref={input}
            type="file"
            accept="image/*"
            hidden
            aria-label="Upload logo"
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
            loading={busy("logo")}
            disabled={busy("logo")}
            onClick={() => input.current?.click()}
          >
            Upload logo
          </Button>
          {workspace.logoAttached ? (
            <Button variant="ghost" size="sm" icon="trash" disabled={busy("logo")} onClick={remove}>
              Remove
            </Button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

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
 * Workspace: the classic account page's top half. Everyone sees the logo, the name and the join
 * link; administrators also rename the workspace, change its logo, decide who may create rooms
 * and swap the join link for a new one.
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
      <SettingsGroup title="Logo">
        <Logo />
      </SettingsGroup>
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
      </SettingsGroup>
    </SettingsPage>
  );
}
