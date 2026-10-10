import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { Workspace } from "../../gen/Workspace.ts";
import { admin } from "../../sync/admin.ts";
import { AdminContext } from "./admin-parts.tsx";
import { WorkspaceSection } from "./workspace-section.tsx";

const workspace: Workspace = {
  name: "Smartfire",
  logoUrl: "/logo",
  logoStillUrl: null,
  bannerUrl: null,
  bannerStillUrl: null,
  logoAttached: false,
  joinUrl: "/join/test",
  canAdminister: true,
  restrictRoomCreationToAdministrators: false,
  uploadLimitBytes: 100 * 1024 * 1024,
  version: "test",
};

beforeEach(() => {
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("lets an administrator save the upload limit in MB", async () => {
  const next = { ...workspace, uploadLimitBytes: 125 * 1024 * 1024 };
  const save = vi.spyOn(admin, "updateWorkspace").mockResolvedValue(next);
  const replace = vi.fn();
  render(
    <AdminContext value={{ workspace, replace }}>
      <WorkspaceSection />
    </AdminContext>,
  );
  const input = screen.getByRole("spinbutton", { name: "Maximum file size (MB)" });
  expect(input).toHaveProperty("value", "100");
  fireEvent.change(input, { target: { value: "125" } });
  fireEvent.click(screen.getByRole("button", { name: "Save upload limit" }));
  await waitFor(() => expect(save).toHaveBeenCalledWith({ uploadLimitBytes: 125 * 1024 * 1024 }));
  await waitFor(() => expect(replace).toHaveBeenCalledWith(next));
});

it("shows the limit to members without giving them an edit control", () => {
  render(
    <AdminContext value={{ workspace: { ...workspace, canAdminister: false }, replace: () => {} }}>
      <WorkspaceSection />
    </AdminContext>,
  );
  expect(screen.queryByRole("spinbutton", { name: "Maximum file size (MB)" })).toBeNull();
  expect(screen.getByText("Up to 100 MB per file.")).toBeTruthy();
});
