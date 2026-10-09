import { Outlet } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { Workspace } from "../../gen/Workspace.ts";
import { admin } from "../../sync/admin.ts";
import { Skeleton } from "../../ui/skeleton.tsx";
import { PaneError } from "../panes/pane-states.tsx";
import { SectionsLayout } from "../settings/sections-layout.tsx";
import { visibleSections } from "./admin-format.ts";
import { AdminContext } from "./admin-parts.tsx";
import "../panes/panes.css";
import "../settings/settings.css";
import "./admin.css";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly workspace: Workspace };

function AdminSkeleton() {
  return (
    <div className="settings-page" role="status" aria-busy="true" aria-label="Loading workspace">
      <Skeleton width={180} height={22} />
      {[0, 1].map((group) => (
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
 * `/app/admin`: the classic account pages as sections (the workspace, its people, and for
 * administrators the workspace icons, custom styles, audit log and integration health), laid out
 * as the settings are (a list, then the section, on phones). The workspace loads once; it says
 * whether the viewer may administer.
 */
export function AdminView() {
  const [load, setLoad] = useState<Load>({ status: "loading" });

  const fetchWorkspace = useCallback(() => {
    admin.workspace().then(
      (workspace) => setLoad({ status: "ready", workspace }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchWorkspace, [fetchWorkspace]);

  const reload = () => {
    setLoad({ status: "loading" });
    fetchWorkspace();
  };

  const state = useMemo(
    () =>
      load.status === "ready"
        ? {
            workspace: load.workspace,
            replace: (workspace: Workspace) => setLoad({ status: "ready", workspace }),
          }
        : null,
    [load],
  );

  const sections = visibleSections(state?.workspace.canAdminister ?? false);

  return (
    <SectionsLayout
      root="/admin"
      title="Workspace"
      icon="home"
      navLabel="Workspace sections"
      sections={sections}
    >
      {load.status === "loading" ? <AdminSkeleton /> : null}
      {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
      {state === null ? null : (
        <AdminContext value={state}>
          <Outlet />
        </AdminContext>
      )}
    </SectionsLayout>
  );
}
