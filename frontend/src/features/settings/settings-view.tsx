import { Outlet } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { Settings } from "../../gen/Settings.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { Skeleton } from "../../ui/skeleton.tsx";
import { PaneError } from "../panes/pane-states.tsx";
import { SectionsLayout } from "./sections-layout.tsx";
import { SECTIONS } from "./settings-format.ts";
import { SettingsContext } from "./settings-parts.tsx";
import "../panes/panes.css";
import "./settings.css";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly settings: Settings };

function SettingsSkeleton() {
  return (
    <div className="settings-page" role="status" aria-busy="true" aria-label="Loading settings">
      <Skeleton width={180} height={22} />
      {[0, 1, 2].map((group) => (
        <div key={group} className="settings-group">
          <Skeleton width={120} height={14} />
          <Skeleton width="80%" height={12} />
          <Skeleton width="60%" height={32} radius="md" />
        </div>
      ))}
    </div>
  );
}

/**
 * `/app/settings`: the classic profile page as sections (profile, status, notifications, rooms,
 * appearance, calls, security, sessions, push devices, integrations), a nav on the left and the chosen
 * section on the right; on phones the nav is the page and each section is pushed over it
 * (SectionsLayout). The page loads once; each section writes its own part and takes the server's
 * answer back.
 */
export function SettingsView() {
  const [load, setLoad] = useState<Load>({ status: "loading" });

  const fetchSettings = useCallback(() => {
    settingsActions.load().then(
      (settings) => setLoad({ status: "ready", settings }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchSettings, [fetchSettings]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchSettings();
  };

  const state = useMemo(
    () =>
      load.status === "ready"
        ? {
            settings: load.settings,
            replace: (settings: Settings) => setLoad({ status: "ready", settings }),
            update: (change: (current: Settings) => Settings) =>
              setLoad((current) =>
                current.status === "ready"
                  ? { status: "ready", settings: change(current.settings) }
                  : current,
              ),
          }
        : null,
    [load],
  );

  return (
    <SectionsLayout
      root="/settings"
      title="Settings"
      icon="settings"
      navLabel="Settings sections"
      sections={SECTIONS}
    >
      {load.status === "loading" ? <SettingsSkeleton /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {state === null ? null : (
        <SettingsContext value={state}>
          <Outlet />
        </SettingsContext>
      )}
    </SectionsLayout>
  );
}
