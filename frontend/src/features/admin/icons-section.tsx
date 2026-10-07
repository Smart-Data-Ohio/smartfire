import { type FormEvent, useCallback, useEffect, useRef, useState } from "react";
import type { WorkspaceIcon } from "../../gen/WorkspaceIcon.ts";
import { admin } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import {
  FieldError,
  fieldsOf,
  SettingsGroup,
  SettingsPage,
  useBusy,
} from "../settings/settings-parts.tsx";
import { ICON_NAME_HINT } from "./admin-format.ts";
import {
  AdministratorsOnly,
  adminFailure,
  uploaded,
  useAdmin,
  useRowFocus,
} from "./admin-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly icons: readonly WorkspaceIcon[] };

type Fields = Readonly<Record<string, readonly string[]>>;

/** The form's own fields: their messages show under them. */
const FORM_FIELDS = new Set(["name", "title", "image"]);

/** Messages about anything else, joined, for the line above the form. */
function elsewhere(fields: Fields): string | undefined {
  const messages = Object.entries(fields)
    .filter(([key]) => !FORM_FIELDS.has(key))
    .flatMap(([, each]) => each);

  return messages.length === 0 ? undefined : messages.join(" ");
}

/** The upload form: name, title and the file, saved together as the classic form does. */
function NewIcon({ onSaved }: { readonly onSaved: (icons: readonly WorkspaceIcon[]) => void }) {
  const [name, setName] = useState("");
  const [title, setTitle] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [fields, setFields] = useState<Fields>({});
  const input = useRef<HTMLInputElement | null>(null);
  const { busy, track } = useBusy();

  const submit = (event: FormEvent) => {
    event.preventDefault();
    setFields({});

    const save = async () => {
      const signedId = file === null ? null : await uploaded(file);

      return admin.createIcon({ name, title, signedId });
    };

    void track(
      "create",
      save().then(
        ({ icons }) => {
          onSaved(icons);
          setName("");
          setTitle("");
          setFile(null);

          if (input.current !== null) {
            input.current.value = "";
          }

          toast({ title: "Icon uploaded", tone: "success" });
        },
        (error: Error) => {
          const named = fieldsOf(error);

          if (Object.keys(named).length > 0) {
            setFields(named);
          } else {
            adminFailure("Couldn't upload the icon", error);
          }
        },
      ),
    );
  };

  const first = (key: string) => fields[key]?.[0];

  return (
    <form className="settings-form" onSubmit={submit}>
      <FieldError message={elsewhere(fields)} />
      <div className="settings-inline admin-icon-names">
        <TextField
          label="Name"
          hint={ICON_NAME_HINT}
          value={name}
          autoComplete="off"
          placeholder="acme"
          error={first("name")}
          onChange={(event) => setName(event.target.value)}
        />
        <TextField
          label="Title"
          value={title}
          autoComplete="off"
          placeholder="Acme Corp"
          error={first("title")}
          onChange={(event) => setTitle(event.target.value)}
        />
      </div>
      <div className="settings-field">
        <label className="settings-label" htmlFor="admin-icon-file">
          Icon file
        </label>
        <input
          ref={input}
          id="admin-icon-file"
          className="input"
          type="file"
          accept="image/svg+xml,image/png"
          onChange={(event) => setFile(event.target.files?.[0] ?? null)}
        />
        <p className="settings-hint text-faint">
          SVG or square PNG (at least 64px), at most 256 KB
        </p>
        <FieldError message={first("image")} />
      </div>
      <div className="settings-actions">
        <Button
          type="submit"
          variant="primary"
          icon="cloud-upload"
          loading={busy("create")}
          disabled={busy("create")}
        >
          Upload icon
        </Button>
      </div>
    </form>
  );
}

/**
 * Workspace icons: the `:shortcodes:` members can use in messages and reactions. Upload one (name,
 * title, SVG or PNG) or delete one; a deleted icon's messages show the literal shortcode.
 */
export function IconsSection() {
  const { workspace } = useAdmin();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [doomed, setDoomed] = useState<WorkspaceIcon | null>(null);
  const { busy, track } = useBusy();
  const container = useRowFocus();

  const fetchIcons = useCallback(() => {
    admin.icons().then(
      ({ icons }) => setLoad({ status: "ready", icons }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(() => {
    if (workspace.canAdminister) fetchIcons();
  }, [fetchIcons, workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const reload = () => {
    setLoad({ status: "loading" });
    fetchIcons();
  };

  const shown = (icons: readonly WorkspaceIcon[]) => setLoad({ status: "ready", icons });

  const destroy = () => {
    const icon = doomed;

    setDoomed(null);

    if (icon === null) {
      return;
    }

    void track(
      `icon-${icon.id}`,
      admin.destroyIcon(icon.id).then(
        ({ icons }) => {
          // The deleted row was where the dialog handed focus back; the next row takes it.
          shown(icons);
        },
        (error: Error) => adminFailure(`Couldn't delete :${icon.name}:`, error),
      ),
    );
  };

  return (
    <SettingsPage
      title="Workspace icons"
      description={
        <>
          Upload SVG or PNG icons members can use as <code>:shortcodes:</code> in messages and
          reactions.
        </>
      }
    >
      <SettingsGroup title="Upload an icon">
        <NewIcon onSaved={shown} />
      </SettingsGroup>
      <SettingsGroup title="Icons">
        <div ref={container} tabIndex={-1} className="admin-focus-root">
          {load.status === "loading" ? <PaneListSkeleton rows={3} /> : null}
          {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
          {load.status === "ready" && load.icons.length === 0 ? (
            <p className="text-muted">No workspace icons yet.</p>
          ) : null}
          {load.status === "ready" && load.icons.length > 0 ? (
            <ul className="settings-list">
              {load.icons.map((icon) => (
                <li key={icon.id} className="settings-list-row" data-row={icon.id} tabIndex={-1}>
                  <img
                    className="admin-icon-image"
                    src={icon.imageUrl}
                    alt=""
                    width={32}
                    height={32}
                    loading="lazy"
                  />
                  <span className="settings-list-main">
                    <strong>
                      {icon.title} <code>:{icon.name}:</code>
                    </strong>
                    <span className="text-faint">Uploaded by {icon.creatorName}</span>
                  </span>
                  <Button
                    variant="danger"
                    size="sm"
                    icon="trash"
                    data-row-control="delete"
                    disabled={busy(`icon-${icon.id}`)}
                    onClick={() => setDoomed(icon)}
                  >
                    <span className="visually-hidden">Delete :{icon.name}:</span>
                  </Button>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      </SettingsGroup>
      <Dialog
        open={doomed !== null}
        onOpenChange={(open) => {
          if (!open) setDoomed(null);
        }}
        role="alertdialog"
        size="sm"
        title={doomed === null ? "" : `Delete :${doomed.name}:?`}
        description="Messages using it will show the literal shortcode."
        footer={
          <>
            <Button variant="secondary" onClick={() => setDoomed(null)} data-autofocus>
              Cancel
            </Button>
            <Button variant="danger" onClick={destroy}>
              Delete
            </Button>
          </>
        }
      />
    </SettingsPage>
  );
}
