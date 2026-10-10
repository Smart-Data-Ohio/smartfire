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
  joinUrl: "https://chat.example/join/test",
  canAdminister: true,
  restrictRoomCreationToAdministrators: false,
  uploadLimitBytes: 100 * 1024 * 1024,
  version: "test",
  description: "",
  vanitySlug: null,
  vanityUrl: null,
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

it("saves a trimmed description and a vanity slug with an invite preview", async () => {
  const save = vi.spyOn(admin, "updateWorkspace").mockResolvedValue(workspace);
  render(
    <AdminContext value={{ workspace, replace: () => {} }}>
      <WorkspaceSection />
    </AdminContext>,
  );
  fireEvent.change(screen.getByRole("textbox", { name: "Description" }), {
    target: { value: "  Our community  " },
  });
  fireEvent.click(screen.getByRole("button", { name: "Save description" }));
  await waitFor(() => expect(save).toHaveBeenCalledWith({ description: "Our community" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Vanity slug" }), {
    target: { value: "smart-data" },
  });
  expect(screen.getByRole("textbox", { name: "Vanity invite URL" })).toHaveProperty(
    "value",
    "https://chat.example/join/smart-data",
  );
  fireEvent.click(screen.getByRole("button", { name: "Save vanity slug" }));
  await waitFor(() => expect(save).toHaveBeenCalledWith({ vanitySlug: "smart-data" }));
});

it("validates reserved words, charset, boundaries and lengths before saving a slug", () => {
  render(
    <AdminContext value={{ workspace, replace: () => {} }}>
      <WorkspaceSection />
    </AdminContext>,
  );
  const input = screen.getByRole("textbox", { name: "Vanity slug" });

  for (const value of ["app", "api", "ab", "a".repeat(33), "UPPER", "a_b", "-abc", "abc-"]) {
    fireEvent.change(input, { target: { value } });
    expect(input.getAttribute("aria-invalid")).toBe("true");
    expect(screen.getByRole("button", { name: "Save vanity slug" })).toHaveProperty(
      "disabled",
      true,
    );
  }

  fireEvent.change(input, { target: { value: "" } });
  expect(screen.getByRole("button", { name: "Save vanity slug" })).toHaveProperty(
    "disabled",
    false,
  );
});

it("shows a member the workspace About text without editing controls", () => {
  render(
    <AdminContext
      value={{
        workspace: { ...workspace, canAdminister: false, description: "R&D <plain text>" },
        replace: () => {},
      }}
    >
      <WorkspaceSection />
    </AdminContext>,
  );
  const about = screen.getByRole("region", { name: "About Smartfire" });
  expect(about.textContent).toContain("R&D <plain text>");
  expect(about.querySelector("plain")).toBeNull();
  expect(screen.queryByRole("textbox", { name: "Description" })).toBeNull();
  expect(screen.queryByRole("textbox", { name: "Vanity slug" })).toBeNull();
});

it("rejects HTML and oversized descriptions, but accepts 300 Unicode characters", () => {
  render(
    <AdminContext value={{ workspace, replace: () => {} }}>
      <WorkspaceSection />
    </AdminContext>,
  );
  const input = screen.getByRole("textbox", { name: "Description" });
  const button = screen.getByRole("button", { name: "Save description" });

  for (const value of ["<b>About</b>", "x".repeat(301), "bad\u0000text"]) {
    fireEvent.change(input, { target: { value } });
    expect(input.getAttribute("aria-invalid")).toBe("true");
    expect(button).toHaveProperty("disabled", true);
  }

  fireEvent.change(input, { target: { value: "😀".repeat(300) } });
  expect(button).toHaveProperty("disabled", false);
});

it("copies a saved vanity invite and disables copying a changed slug", async () => {
  const copy = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal("navigator", { clipboard: { writeText: copy } });

  const saved = {
    ...workspace,
    vanitySlug: "smart-data",
    vanityUrl: "https://chat.example/join/smart-data",
  };

  render(
    <AdminContext value={{ workspace: saved, replace: () => {} }}>
      <WorkspaceSection />
    </AdminContext>,
  );
  fireEvent.click(screen.getByRole("button", { name: "Copy vanity invite" }));
  await waitFor(() => expect(copy).toHaveBeenCalledWith("https://chat.example/join/smart-data"));
  fireEvent.change(screen.getByRole("textbox", { name: "Vanity slug" }), {
    target: { value: "another-slug" },
  });
  expect(screen.getByRole("button", { name: "Copy vanity invite" })).toHaveProperty(
    "disabled",
    true,
  );
});

it("shows the empty About state to members", () => {
  render(
    <AdminContext value={{ workspace: { ...workspace, canAdminister: false }, replace: () => {} }}>
      <WorkspaceSection />
    </AdminContext>,
  );
  expect(screen.getByRole("region", { name: "About Smartfire" }).textContent).toContain(
    "No description yet.",
  );
});
