import { type FormEvent, useState } from "react";
import type { Workspace } from "../../gen/Workspace.ts";
import { admin } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";
import { SettingsGroup, SettingsPage, useBusy } from "../settings/settings-parts.tsx";
import { adminFailure, useAdmin } from "./admin-parts.tsx";
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
