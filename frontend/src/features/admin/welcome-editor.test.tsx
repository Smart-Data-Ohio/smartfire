import { cleanup, fireEvent, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { WelcomeAdmin } from "../../gen/WelcomeAdmin.ts";
import { admin } from "../../sync/admin.ts";
import { ActionError } from "../../sync/run.ts";
import { renderInRouter } from "./testing.tsx";
import { WelcomeEditor } from "./welcome-editor.tsx";

const hours = { title: "Hours", emoji: null, roomId: null, keep: null, body: "Nine to five" };

const loaded: WelcomeAdmin = {
  message: "Hello",
  rooms: [],
  guide: [hours],
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

it("suggests a room with a description and saves the welcome section alone", async () => {
  vi.spyOn(admin, "welcome").mockResolvedValue(loaded);

  const save = vi.spyOn(admin, "saveWelcome").mockImplementation(async (update) => ({
    ...loaded,
    ...update,
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
      rooms: [{ roomId: 2, keep: null, emoji: "🎨", description: "Where the pixels live" }],
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

  const save = vi.spyOn(admin, "saveWelcome").mockImplementation(async (update) => ({
    ...loaded,
    ...update,
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
      guide: [{ title: "Start here", emoji: null, roomId: 1, keep: null, body: null }, hours],
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

/** A promise and the function that settles it. */
function deferred<T>() {
  let resolve: (value: T) => void = () => undefined;

  const promise = new Promise<T>((settle) => {
    resolve = settle;
  });

  return { promise, resolve };
}

it("saves the welcome and the guide at once without either undoing the other", async () => {
  vi.spyOn(admin, "welcome").mockResolvedValue(loaded);

  const replies = [deferred<WelcomeAdmin>(), deferred<WelcomeAdmin>()];

  const save = vi
    .spyOn(admin, "saveWelcome")
    .mockImplementationOnce(() => replies[0]?.promise ?? Promise.reject(new Error("unused")))
    .mockImplementationOnce(() => replies[1]?.promise ?? Promise.reject(new Error("unused")));

  await renderInRouter(<WelcomeEditor />);

  const welcome = await screen.findByRole("region", { name: "Welcome screen" });
  const guide = screen.getByRole("region", { name: "Server guide" });

  fireEvent.change(within(welcome).getByRole("textbox", { name: "Welcome message" }), {
    target: { value: "Hi there" },
  });
  fireEvent.change(within(guide).getByRole("textbox", { name: "Title" }), {
    target: { value: "Opening hours" },
  });
  fireEvent.click(within(welcome).getByRole("button", { name: "Save welcome" }));
  fireEvent.click(within(guide).getByRole("button", { name: "Save guide" }));

  // Each save carries its own section only, so the server keeps whichever lands first.
  await waitFor(() => expect(save).toHaveBeenCalledTimes(2));
  expect(save.mock.calls[0]?.[0]).toEqual({ message: "Hi there", rooms: [] });
  expect(save.mock.calls[1]?.[0]).toEqual({ guide: [{ ...hours, title: "Opening hours" }] });

  // The answers land in the other order; each form keeps what it saved.
  const stored: WelcomeAdmin = {
    ...loaded,
    message: "Hi there",
    guide: [{ ...hours, title: "Opening hours" }],
  };

  replies[1]?.resolve(stored);
  replies[0]?.resolve({ ...stored, guide: loaded.guide });

  await waitFor(() =>
    expect(within(guide).getByRole("button", { name: "Save guide" })).toHaveProperty(
      "disabled",
      false,
    ),
  );
  await waitFor(() =>
    expect(within(welcome).getByRole("button", { name: "Save welcome" })).toHaveProperty(
      "disabled",
      false,
    ),
  );
  expect(within(welcome).getByRole("textbox", { name: "Welcome message" })).toHaveProperty(
    "value",
    "Hi there",
  );
  expect(within(guide).getByRole("textbox", { name: "Title" })).toHaveProperty(
    "value",
    "Opening hours",
  );
});

it("keeps a saved room this administrator can't see, unnamed, as its token", async () => {
  vi.spyOn(admin, "welcome").mockResolvedValue({
    ...loaded,
    rooms: [{ roomId: null, keep: "sealed", emoji: null, description: "Ask to join" }],
  });

  const save = vi.spyOn(admin, "saveWelcome").mockImplementation(async (update) => ({
    ...loaded,
    ...update,
  }));

  await renderInRouter(<WelcomeEditor />);

  const group = await screen.findByRole("region", { name: "Welcome screen" });
  const row = within(group).getByRole("group", { name: "Suggested room 1" });
  const picker = within(row).getByRole("combobox", { name: "Room" });

  expect(within(picker).getByRole("option", { selected: true }).textContent).toBe(
    "A room you can't see",
  );
  fireEvent.change(within(row).getByRole("textbox", { name: "Description" }), {
    target: { value: "Ask Jason to join" },
  });
  fireEvent.click(within(group).getByRole("button", { name: "Save welcome" }));
  await waitFor(() =>
    expect(save).toHaveBeenLastCalledWith({
      message: "Hello",
      rooms: [{ roomId: null, keep: "sealed", emoji: null, description: "Ask Jason to join" }],
    }),
  );

  // Picking a room it can see replaces the hidden one; the token goes.
  await waitFor(() =>
    expect(within(group).getByRole("button", { name: "Save welcome" })).toHaveProperty(
      "disabled",
      false,
    ),
  );

  const again = within(group).getByRole("group", { name: "Suggested room 1" });

  fireEvent.change(within(again).getByRole("combobox", { name: "Room" }), {
    target: { value: "1" },
  });
  expect(within(again).queryByRole("option", { name: "A room you can't see" })).toBeNull();
  fireEvent.click(within(group).getByRole("button", { name: "Save welcome" }));
  await waitFor(() =>
    expect(save).toHaveBeenLastCalledWith({
      message: "Hello",
      rooms: [{ roomId: 1, keep: null, emoji: null, description: "Ask Jason to join" }],
    }),
  );
});
