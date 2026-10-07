import { type FormEvent, type KeyboardEvent, useCallback, useEffect, useState } from "react";
import { admin } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError } from "../panes/pane-states.tsx";
import { SettingsPage, useBusy } from "../settings/settings-parts.tsx";
import { AdministratorsOnly, adminFailure, useAdmin } from "./admin-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready" };

/**
 * Custom styles: the workspace's custom CSS, as the classic page edits it. Saving asks for the
 * password when its confirmation has lapsed; Ctrl+Enter (⌘+Enter) saves.
 */
export function StylesSection() {
  const { workspace } = useAdmin();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [css, setCss] = useState("");
  const { busy, track } = useBusy();

  const fetchStyles = useCallback(() => {
    admin.customStyles().then(
      (styles) => {
        setCss(styles.css ?? "");
        setLoad({ status: "ready" });
      },
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(() => {
    if (workspace.canAdminister) fetchStyles();
  }, [fetchStyles, workspace.canAdminister]);

  if (!workspace.canAdminister) {
    return <AdministratorsOnly />;
  }

  const reload = () => {
    setLoad({ status: "loading" });
    fetchStyles();
  };

  const save = () => {
    void track(
      "save",
      admin.updateCustomStyles(css).then(
        (styles) => {
          setCss(styles.css ?? "");
          toast({ title: "Custom styles saved", tone: "success" });
        },
        (error: Error) => adminFailure("Couldn't save the custom styles", error),
      ),
    );
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    save();
  };

  const shortcut = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      save();
    }
  };

  return (
    <SettingsPage
      title="Custom CSS"
      description="Add custom CSS styles. Use caution: you could break things."
    >
      {load.status === "loading" ? <p className="text-muted">Loading…</p> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {load.status === "ready" ? (
        <form className="settings-form" onSubmit={submit}>
          <label className="visually-hidden" htmlFor="admin-custom-styles">
            Custom CSS
          </label>
          <textarea
            id="admin-custom-styles"
            className="input settings-textarea admin-code"
            rows={16}
            value={css}
            placeholder="Add CSS styles…"
            autoComplete="off"
            spellCheck={false}
            autoCorrect="off"
            autoCapitalize="off"
            onChange={(event) => setCss(event.target.value)}
            onKeyDown={shortcut}
          />
          <div className="settings-actions">
            <Button type="submit" variant="primary" loading={busy("save")} disabled={busy("save")}>
              Save changes
            </Button>
          </div>
        </form>
      ) : null}
    </SettingsPage>
  );
}
