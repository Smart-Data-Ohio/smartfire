import { Link, Outlet } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { Workspace } from "../../gen/Workspace.ts";
import { admin } from "../../sync/admin.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Skeleton } from "../../ui/skeleton.tsx";
import { PaneError } from "../panes/pane-states.tsx";
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
 * as the settings are. The workspace loads once; it says whether the viewer may administer.
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
    <div className="settings">
      <header className="settings-header">
        <Link to="/" className="settings-back" aria-label="Back to conversations">
          <Icon name="chevron-left" size={20} />
        </Link>
        <Icon name="home" size={18} className="settings-header-icon" />
        <span className="settings-header-title text-title">Workspace</span>
      </header>
      <div className="settings-body">
        <nav className="settings-nav" aria-label="Workspace sections">
          <ul>
            {sections.map((section) => (
              <li key={section.key}>
                <Link
                  to={section.path}
                  className="settings-nav-link"
                  activeOptions={{ exact: true }}
                  activeProps={{ "aria-current": "page" }}
                >
                  <Icon name={section.icon} size={16} />
                  <span>{section.label}</span>
                </Link>
              </li>
            ))}
          </ul>
        </nav>
        <div className="settings-content">
          {load.status === "loading" ? <AdminSkeleton /> : null}
          {load.status === "error" ? <PaneError message={load.message} onRetry={reload} /> : null}
          {state === null ? null : (
            <AdminContext value={state}>
              <Outlet />
            </AdminContext>
          )}
        </div>
      </div>
    </div>
  );
}
