import { cleanup, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import fixture from "../../../e2e/mock/fixtures/server-template.json?raw";
import { admin, templates } from "../../sync/admin.ts";
import { installMockNetwork } from "../../test/mock-network.ts";
import { AdminContext } from "./admin-parts.tsx";
import { TemplatesSection } from "./templates-section.tsx";
import { renderInRouter } from "./testing.tsx";

const network = installMockNetwork();

beforeEach(async () => {
  await fetch("/__mock/reset", { method: "POST" });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

afterAll(() => network.restore());

async function renderSection(canAdminister = true) {
  const workspace = { ...(await admin.workspace()), canAdminister };

  return renderInRouter(
    <AdminContext value={{ workspace, replace: () => {} }}>
      <TemplatesSection />
    </AdminContext>,
    "templates",
  );
}

async function pick(contents = fixture) {
  const user = userEvent.setup();

  await user.upload(
    screen.getByLabelText("Template file"),
    new File([contents], "workspace.json", { type: "application/json" }),
  );

  return user;
}

describe("the templates section", () => {
  it("previews the structure without writes, shows renamed rooms and settings, then imports on confirmation", async () => {
    await renderSection();
    const before = await templates.export();
    const user = await pick();
    const preview = await screen.findByRole("region", { name: "Template preview" });

    expect(within(preview).getByText("Room general will be created as general (2)")).toBeTruthy();
    expect(within(preview).getByText(/1 categories and 3 rooms/)).toBeTruthy();
    expect(await templates.export()).toEqual(before);
    await user.click(within(preview).getByText("Town hall (stage)"));
    expect(within(preview).getByText("96000 bps")).toBeTruthy();
    expect(within(preview).getByText("20")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Confirm import" }));
    expect(await screen.findByText("Created 1 categories and 3 rooms.")).toBeTruthy();
    const after = await templates.export();

    expect(after.categories.at(-1)?.rooms.map((room) => room.name)).toEqual([
      "general (2)",
      "Town hall",
    ]);
    expect(after.rooms.find((room) => room.name === "Ideas")?.board).toEqual({
      tags: [{ name: "Idea", emoji: "💡" }],
      tagsRequired: true,
      defaultTag: 0,
    });
  });

  it("shows validation errors and never enables confirmation for malformed, unsupported or oversized files", async () => {
    await renderSection();
    const user = await pick("{");

    await screen.findByRole("alert");
    expect(screen.getByRole("button", { name: "Confirm import" })).toHaveProperty("disabled", true);
    await user.upload(
      screen.getByLabelText("Template file"),
      new File([fixture.replace('"version": 1', '"version": 9')], "v9.json", {
        type: "application/json",
      }),
    );
    expect(
      await screen.findByText("Unsupported template version 9. Expected version 1."),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Confirm import" })).toHaveProperty("disabled", true);
    await user.upload(
      screen.getByLabelText("Template file"),
      new File([" ".repeat(1024 * 1024 + 1)], "big.json", { type: "application/json" }),
    );
    expect(await screen.findByRole("alert")).toHaveProperty(
      "textContent",
      "Template files must be at most 1 MiB.",
    );
    expect(screen.queryByRole("button", { name: "Confirm import" })).toBeNull();
  });

  it("cancels a preview and can preview the same file again", async () => {
    await renderSection();
    const user = await pick();

    await screen.findByRole("region", { name: "Template preview" });
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("region", { name: "Template preview" })).toBeNull();
    await pick();
    expect(await screen.findByRole("button", { name: "Confirm import" })).toHaveProperty(
      "disabled",
      false,
    );
  });

  it("shows a failed confirmation and requires a new preview", async () => {
    await renderSection();
    const user = await pick();

    await screen.findByRole("region", { name: "Template preview" });
    // Another admin imports while this preview is open.
    const other = await templates.preview(fixture);

    if (other.template === null) throw new Error("Expected a valid preview");
    await templates.import(fixture, other.template);
    await user.click(screen.getByRole("button", { name: "Confirm import" }));
    expect(await screen.findByRole("alert")).toHaveProperty(
      "textContent",
      "Workspace structure changed. Preview this file again before importing.",
    );
    expect(screen.queryByRole("button", { name: "Confirm import" })).toBeNull();
  });

  it("reports export success and a network failure", async () => {
    await renderSection();
    const user = userEvent.setup();
    const makeUrl = vi.fn(() => "blob:template");
    const previousCreate = URL.createObjectURL;
    const previousRevoke = URL.revokeObjectURL;

    URL.createObjectURL = makeUrl;
    URL.revokeObjectURL = () => {};

    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});

    try {
      await user.click(screen.getByRole("button", { name: "Export template" }));
      await screen.findByText("Template downloaded.");
      expect(makeUrl).toHaveBeenCalledOnce();
      expect(click).toHaveBeenCalledOnce();
      vi.spyOn(templates, "export").mockRejectedValueOnce(new Error("Connection failed"));
      await user.click(screen.getByRole("button", { name: "Export template" }));
      expect(await screen.findByRole("alert")).toHaveProperty("textContent", "Connection failed");
      await waitFor(() =>
        expect(screen.getByRole("button", { name: "Export template" })).toHaveProperty(
          "disabled",
          false,
        ),
      );
    } finally {
      URL.createObjectURL = previousCreate;
      URL.revokeObjectURL = previousRevoke;
    }
  });

  it("hides the flow from non-admins", async () => {
    await renderSection(false);
    expect(screen.queryByRole("button", { name: "Export template" })).toBeNull();
    expect(screen.queryByLabelText("Template file")).toBeNull();
  });
});
