import { cleanup, fireEvent, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { WelcomeAdmin } from "../../gen/WelcomeAdmin.ts";
import { admin } from "../../sync/admin.ts";
import { ActionError } from "../../sync/run.ts";
import { renderInRouter } from "./testing.tsx";
import { WelcomeEditor } from "./welcome-editor.tsx";

const loaded: WelcomeAdmin = {
  settings: {
    message: "Hello",
    rooms: [],
    guide: [{ title: "Hours", emoji: null, roomId: null, body: "Nine to five" }],
  },
  choices: [
    { id: 1, name: "general" },
    { id: 2, name: "design" },
  ],
};

beforeEach(() => {
  // Focus moves once React has moved the rows: the next frame.
  vi.stubGlobal("requestAnimationFrame", (run: () => void) => setTimeout(run, 0));
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("suggests a room with a description and saves the welcome, keeping the saved guide", async () => {
  vi.spyOn(admin, "welcome").mockResolvedValue(loaded);

  const save = vi.spyOn(admin, "saveWelcome").mockImplementation(async (settings) => ({
    ...loaded,
    settings,
  }));

  await renderInRouter(<WelcomeEditor />);

  const group = await screen.findByRole("region", { name: "Welcome screen" });
  expect(within(group).getByRole("textbox", { name: "Welcome message" })).toHaveProperty(
    "value",
    "Hello",
  );

  fireEvent.click(within(group).getByRole("button", { name: "Add a room" }));
  const row = within(group).getByRole("group", { name: "Suggested room 1" });
  const save1 = within(group).getByRole("button", { name: "Save welcome" });
  // Nothing chosen yet: the row says so and Save waits.
  expect(within(row).getByText("Choose a room.")).toBeTruthy();
  expect(save1).toHaveProperty("disabled", true);

  fireEvent.change(within(row).getByRole("combobox", { name: "Room" }), {
    target: { value: "2" },
  });
  fireEvent.change(within(row).getByRole("textbox", { name: "Emoji" }), {
    target: { value: "🎨" },
  });
  fireEvent.change(within(row).getByRole("textbox", { name: "Description" }), {
    target: { value: " Where the pixels live " },
  });
  fireEvent.click(save1);

  await waitFor(() =>
    expect(save).toHaveBeenCalledWith({
      message: "Hello",
      rooms: [{ roomId: 2, emoji: "🎨", description: "Where the pixels live" }],
      guide: loaded.settings.guide,
    }),
  );
});

it("refuses a word for an emoji before saving", async () => {
  vi.spyOn(admin, "welcome").mockResolvedValue(loaded);
  await renderInRouter(<WelcomeEditor />);

  const group = await screen.findByRole("region", { name: "Welcome screen" });
  fireEvent.click(within(group).getByRole("button", { name: "Add a room" }));
  const row = within(group).getByRole("group", { name: "Suggested room 1" });
  fireEvent.change(within(row).getByRole("combobox", { name: "Room" }), {
    target: { value: "1" },
  });
  fireEvent.change(within(row).getByRole("textbox", { name: "Emoji" }), {
    target: { value: "wave" },
  });

  expect(within(row).getByText("Use a single emoji, or leave it empty.")).toBeTruthy();
  expect(within(group).getByRole("button", { name: "Save welcome" })).toHaveProperty(
    "disabled",
    true,
  );
});

it("adds, reorders and removes guide entries, saving them in order", async () => {
  vi.spyOn(admin, "welcome").mockResolvedValue(loaded);

  const save = vi.spyOn(admin, "saveWelcome").mockImplementation(async (settings) => ({
    ...loaded,
    settings,
  }));

  await renderInRouter(<WelcomeEditor />);

  const group = await screen.findByRole("region", { name: "Server guide" });
  fireEvent.click(within(group).getByRole("button", { name: "Add an entry" }));
  const second = within(group).getByRole("group", { name: "Guide entry 2" });
  fireEvent.change(within(second).getByRole("textbox", { name: "Title" }), {
    target: { value: "Start here" },
  });
  fireEvent.click(within(second).getByRole("radio", { name: "A room link" }));
  fireEvent.change(within(second).getByRole("combobox", { name: "Room" }), {
    target: { value: "1" },
  });
  fireEvent.click(within(group).getByRole("button", { name: "Move entry 2 up" }));
  // Focus stays on the moved entry's control, now the first entry's.
  await waitFor(() =>
    expect(document.activeElement).toBe(
      within(group).getByRole("button", { name: "Move entry 1 down" }),
    ),
  );

  fireEvent.click(within(group).getByRole("button", { name: "Save guide" }));
  await waitFor(() =>
    expect(save).toHaveBeenLastCalledWith({
      ...loaded.settings,
      guide: [
        { title: "Start here", emoji: null, roomId: 1, body: null },
        { title: "Hours", emoji: null, roomId: null, body: "Nine to five" },
      ],
    }),
  );

  fireEvent.click(within(group).getByRole("button", { name: "Remove entry 1" }));
  expect(within(group).queryByRole("group", { name: "Guide entry 2" })).toBeNull();
});

it("shows the server's refusal beside the form", async () => {
  vi.spyOn(admin, "welcome").mockResolvedValue(loaded);
  vi.spyOn(admin, "saveWelcome").mockRejectedValue(
    new ActionError("Validation", "Validation failed", {
      rooms: ["#1: isn't a room you can suggest"],
    }),
  );
  await renderInRouter(<WelcomeEditor />);

  const group = await screen.findByRole("region", { name: "Welcome screen" });
  fireEvent.click(within(group).getByRole("button", { name: "Save welcome" }));

  expect((await within(group).findByRole("alert")).textContent).toBe(
    "#1: isn't a room you can suggest",
  );
});
