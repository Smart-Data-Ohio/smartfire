import { describe, expect, it } from "vitest";
import type { Workspace } from "../gen/Workspace.ts";
import type { WorkspaceBranding } from "../gen/WorkspaceBranding.ts";
import type { Boot } from "./model.ts";
import { applyEvents } from "./reducers.ts";
import { initialState, type State } from "./state.ts";
import { brandingOf, setWorkspaceBranding } from "./workspace.ts";

const BOOT: Boot = {
  user: { id: 1, name: "Riel", avatarUrl: "/users/1/avatar" },
  account: {
    name: "Smart Data",
    logoUrl: null,
    logoStillUrl: null,
    bannerUrl: null,
    bannerStillUrl: null,
  },
  theme: "system",
  textSize: "default",
  cableUrl: "/cable",
  version: "test",
  revision: null,
};

const BRANDED: WorkspaceBranding = {
  name: "Smart Data Labs",
  logoUrl: "/account/logo?v=2&animated=1",
  logoStillUrl: "/account/logo?v=2",
  bannerUrl: "/account/banner?v=7",
  bannerStillUrl: null,
};

const booted = (): State => ({ ...initialState, boot: BOOT });

function workspace(extra: Partial<Workspace> = {}): Workspace {
  return {
    name: "Smart Data",
    logoUrl: "/account/logo?v=1",
    logoAttached: false,
    logoStillUrl: null,
    bannerUrl: null,
    bannerStillUrl: null,
    joinUrl: "http://127.0.0.1/join/abc",
    canAdminister: true,
    restrictRoomCreationToAdministrators: false,
    version: "2.0.0",
    ...extra,
  };
}

describe("workspace branding", () => {
  it("takes the name, logo and banner from workspace.updated", () => {
    const state = applyEvents(
      booted(),
      [{ seq: 1, topic: "user", type: "workspace.updated", data: BRANDED }],
      0,
    );

    expect(state.boot?.account).toEqual(BRANDED);

    expect(state.boot?.user).toBe(BOOT.user);
  });

  it("goes back to initials and the plain header when both images are removed", () => {
    const branded = setWorkspaceBranding(booted(), BRANDED);

    const cleared = setWorkspaceBranding(branded, {
      ...BRANDED,
      logoUrl: null,
      logoStillUrl: null,
      bannerUrl: null,
    });

    expect(cleared.boot?.account).toMatchObject({
      logoUrl: null,
      logoStillUrl: null,
      bannerUrl: null,
    });
  });

  it("keeps the state when nothing changed, and before boot", () => {
    const branded = setWorkspaceBranding(booted(), BRANDED);

    expect(setWorkspaceBranding(branded, { ...BRANDED })).toBe(branded);

    expect(setWorkspaceBranding(initialState, BRANDED)).toBe(initialState);
  });

  it("reads a workspace reply's stock logo as no logo", () => {
    expect(brandingOf(workspace())).toEqual({
      name: "Smart Data",
      logoUrl: null,
      logoStillUrl: null,
      bannerUrl: null,
      bannerStillUrl: null,
    });

    expect(
      brandingOf(
        workspace({
          logoAttached: true,
          logoUrl: "/account/logo?v=3&animated=1",
          logoStillUrl: "/account/logo?v=3",
          bannerUrl: "/account/banner?v=9",
        }),
      ),
    ).toEqual({
      name: "Smart Data",
      logoUrl: "/account/logo?v=3&animated=1",
      logoStillUrl: "/account/logo?v=3",
      bannerUrl: "/account/banner?v=9",
      bannerStillUrl: null,
    });
  });
});
