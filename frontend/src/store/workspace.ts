import type { Workspace } from "../gen/Workspace.ts";
import type { WorkspaceBranding } from "../gen/WorkspaceBranding.ts";
import type { State } from "./state.ts";

/** The branding a `Workspace` reply carries: the stock logo counts as none. */
export function brandingOf(workspace: Workspace): WorkspaceBranding {
  return {
    name: workspace.name,
    logoUrl: workspace.logoAttached ? workspace.logoUrl : null,
    logoStillUrl: workspace.logoAttached ? workspace.logoStillUrl : null,
    bannerUrl: workspace.bannerUrl,
    bannerStillUrl: workspace.bannerStillUrl,
  };
}

const FIELDS = ["name", "logoUrl", "logoStillUrl", "bannerUrl", "bannerStillUrl"] as const;

export function setWorkspaceStyles(state: State, css: string | null): State {
  if (state.boot === null || state.boot.customStyles === css) return state;

  return { ...state, boot: { ...state.boot, customStyles: css } };
}

/**
 * The workspace's name, logo and banner as `workspace.updated` (or an admin's own save) gives
 * them. Before boot there's nothing to brand; an unchanged branding keeps the state.
 */
export function setWorkspaceBranding(state: State, branding: WorkspaceBranding): State {
  const { boot } = state;

  if (boot === null || FIELDS.every((field) => boot.account[field] === branding[field])) {
    return state;
  }

  return { ...state, boot: { ...boot, account: { ...branding } } };
}
