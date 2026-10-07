import {
  type FormEvent,
  type KeyboardEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { admin } from "../../sync/admin.ts";
import { Button } from "../../ui/button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PaneError } from "../panes/pane-states.tsx";
import { SettingsPage, useBusy } from "../settings/settings-parts.tsx";
import { AdministratorsOnly, adminFailure, needsSudo, useAdmin } from "./admin-parts.tsx";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready" };

/** Where an unsaved edit waits while the classic page confirms the password. */
const DRAFT_KEY = "smartfire.draft.admin-styles";

/** The edit kept across the password round trip, taken (and forgotten) once; `null` when none. */
function takeDraft(): string | null {
  try {
    const draft = sessionStorage.getItem(DRAFT_KEY);

    sessionStorage.removeItem(DRAFT_KEY);

    return draft;
  } catch {
    return null;
  }
}

/** Keeps `css` for the page to restore after the password round trip's full page load. */
function keepDraft(css: string): void {
  try {
    sessionStorage.setItem(DRAFT_KEY, css);
  } catch {
    // Without storage the edit is lost with the page, as on the classic form.
  }
}

/**
 * Custom styles: the workspace's custom CSS, as the classic page edits it. Saving asks for the
 * password when its confirmation has lapsed; Ctrl+Enter (⌘+Enter) saves.
 */
export function StylesSection() {
  const { workspace } = useAdmin();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [css, setCss] = useState("");
  const { busy, track } = useBusy();
  // The kept edit, taken from storage by the first load to land (`undefined` until then), so
  // every later load (a retry, a second mount's) shows it too.
  const kept = useRef<string | null | undefined>(undefined);

  const fetchStyles = useCallback(() => {
    admin.customStyles().then(
      (styles) => {
        if (kept.current === undefined) {
          kept.current = takeDraft();

          if (kept.current !== null) {
            toast({ title: "Your unsaved CSS is back", description: "Save it to keep it." });
          }
        }

        setCss(kept.current ?? styles.css ?? "");
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
          kept.current = null;
          setCss(styles.css ?? "");
          toast({ title: "Custom styles saved", tone: "success" });
        },
        (error: Error) => {
          if (needsSudo(error)) keepDraft(css);

          adminFailure("Couldn't save the custom styles", error);
        },
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
