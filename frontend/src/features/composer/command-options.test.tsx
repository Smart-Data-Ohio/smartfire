import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SlashCommand } from "../../gen/SlashCommand.ts";
import { emptyTimeline, initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { composerActions } from "../../sync/composer-actions.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { toastSnapshot } from "../../ui/toast-store.ts";
import { clearCommandCache } from "./autocomplete/suggestions.ts";
import { Composer } from "./composer.tsx";
import { draftKey, readDraft } from "./draft.ts";

const ROOM = 12;

const DEPLOY: SlashCommand = {
  name: "deploy",
  description: "Ship a service",
  argHint: "",
  takesArguments: true,
  agentName: "Ember",
  optionSchema: {
    subcommands: [
      {
        name: "run",
        description: "Start a deploy",
        options: [
          {
            name: "service",
            description: "The service to deploy",
            type: "string",
            required: true,
            min_length: 2,
            max_length: 40,
          },
          {
            name: "environment",
            description: "Where it goes",
            type: "string",
            required: true,
            choices: [
              { name: "Staging", value: "staging" },
              { name: "Production", value: "production" },
            ],
          },
          {
            name: "replicas",
            description: "How many copies",
            type: "integer",
            required: false,
            min: 1,
            max: 10,
          },
        ],
      },
    ],
  },
};

const REMIND: SlashCommand = {
  name: "remind",
  description: "Set a reminder",
  argHint: "<when> <text>",
  takesArguments: true,
  agentName: null,
  optionSchema: null,
};

beforeEach(() => {
  Element.prototype.scrollIntoView = () => undefined;
  sessionStorage.clear();
  clearCommandCache();
  store.setState(
    { ...initialState, timelines: { [ROOM]: { ...emptyTimeline, status: "ready" } } },
    true,
  );
  vi.stubGlobal("matchMedia", (query: string) =>
    Object.assign(new EventTarget(), { matches: true, media: query, onchange: null }),
  );
  vi.spyOn(actions, "noteActivity").mockImplementation(() => undefined);
  vi.spyOn(actions, "setTyping").mockImplementation(() => undefined);
  vi.spyOn(actions.scheduled, "load").mockResolvedValue(undefined);
  vi.spyOn(composerActions, "slashCommands").mockResolvedValue([REMIND, DEPLOY]);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

function textbox(): HTMLElement {
  return screen.getByRole("textbox", { name: "Message" });
}

/** Types `/dep`, picks /deploy, then `run`: the service field has focus. */
async function openDeploy(): Promise<void> {
  render(<Composer roomId={ROOM} placeholder="Message" />);

  const input = textbox();

  fireEvent.focus(input);
  fireEvent.change(input, { target: { value: "/dep" } });
  await screen.findByRole("option", { name: /\/deploy <run>/ });
  fireEvent.keyDown(input, { key: "Enter" });

  const subcommand = await screen.findByRole("combobox", { name: "subcommand" });

  await waitFor(() => expect(document.activeElement).toBe(subcommand));
  expect(screen.getByRole("listbox", { name: "Subcommands" })).toBeTruthy();
  expect(subcommand.getAttribute("aria-expanded")).toBe("true");
  fireEvent.keyDown(subcommand, { key: "Enter" });
  await waitFor(() =>
    expect(document.activeElement).toBe(screen.getByRole("textbox", { name: "service" })),
  );
}

function field(name: string): HTMLElement {
  return screen.getByRole(name === "environment" ? "combobox" : "textbox", { name });
}

describe("a typed command's fields", () => {
  it("moves with Tab and Enter, validates as it goes, and sends the typed values", async () => {
    const run = vi
      .spyOn(composerActions, "runSlashCommand")
      .mockResolvedValue({ status: "ephemeral", message: "Sent to Ember" });

    await openDeploy();

    const send = screen.getByRole("button", { name: "Send message" });

    expect(send).toHaveProperty("disabled", true);
    expect(screen.queryByRole("textbox", { name: "Message" })).toBeNull();

    fireEvent.change(field("service"), { target: { value: "a" } });
    fireEvent.keyDown(field("service"), { key: "Tab" });
    await waitFor(() => expect(document.activeElement).toBe(field("environment")));

    // The left field shows its problem, wired to the input.
    const service = field("service");
    const error = screen.getByText("Must be 2 to 40 characters.");

    expect(service.getAttribute("aria-invalid")).toBe("true");
    expect(service.getAttribute("aria-describedby")).toContain(error.id);

    // Backspace in an empty field goes back to the one before it.
    fireEvent.keyDown(field("environment"), { key: "Backspace" });
    await waitFor(() => expect(document.activeElement).toBe(field("service")));
    fireEvent.change(field("service"), { target: { value: "api" } });
    expect(field("service").getAttribute("aria-invalid")).toBeNull();
    fireEvent.keyDown(field("service"), { key: "Enter" });
    await waitFor(() => expect(document.activeElement).toBe(field("environment")));

    // A choice picks from its listbox: arrows, then Enter.
    expect(screen.getByRole("listbox", { name: "Choices" })).toBeTruthy();
    fireEvent.keyDown(field("environment"), { key: "ArrowDown" });
    fireEvent.keyDown(field("environment"), { key: "Enter" });
    expect(field("environment")).toHaveProperty("value", "Production");
    expect(send).toHaveProperty("disabled", false);

    // An optional option joins from the row below, and is checked like the rest.
    fireEvent.click(screen.getByRole("button", { name: "Add replicas" }));
    await waitFor(() => expect(document.activeElement).toBe(field("replicas")));
    fireEvent.change(field("replicas"), { target: { value: "11" } });
    expect(send).toHaveProperty("disabled", true);
    fireEvent.keyDown(field("replicas"), { key: "Enter" });
    expect(run).not.toHaveBeenCalled();
    expect(screen.getByText("Must be 1 to 10.")).toBeTruthy();
    fireEvent.change(field("replicas"), { target: { value: "3" } });
    fireEvent.keyDown(field("replicas"), { key: "Enter" });

    await waitFor(() =>
      expect(run).toHaveBeenCalledWith(
        ROOM,
        "/deploy run service:api environment:production replicas:3",
        null,
        { subcommand: "run", values: { service: "api", environment: "production", replicas: 3 } },
      ),
    );
    await waitFor(() => expect(document.activeElement).toBe(textbox()));
    expect(toastSnapshot().at(-1)?.title).toBe("Sent to Ember");
  });

  it("puts a 422's field errors back on the fields they name", async () => {
    vi.spyOn(composerActions, "runSlashCommand").mockRejectedValue(
      new ActionError("Validation", "Invalid slash command options", {
        "options.values.service": ["length is out of range"],
      }),
    );

    await openDeploy();
    fireEvent.change(field("service"), { target: { value: "api" } });
    fireEvent.keyDown(field("service"), { key: "Tab" });
    fireEvent.change(field("environment"), { target: { value: "staging" } });
    fireEvent.keyDown(field("environment"), { key: "Escape" });
    fireEvent.click(screen.getByRole("button", { name: "Send message" }));

    const error = await screen.findByText("Length is out of range.");

    expect(field("service").getAttribute("aria-invalid")).toBe("true");
    expect(field("service").getAttribute("aria-describedby")).toContain(error.id);
    await waitFor(() => expect(document.activeElement).toBe(field("service")));
    expect(field("environment")).toHaveProperty("value", "staging");

    fireEvent.change(field("service"), { target: { value: "web" } });
    expect(screen.queryByText("Length is out of range.")).toBeNull();
  });

  it("keeps the generic handling for other failures, with the fields put back", async () => {
    vi.spyOn(composerActions, "runSlashCommand").mockRejectedValue(
      new ActionError("NetworkError", "Offline"),
    );

    await openDeploy();
    fireEvent.change(field("service"), { target: { value: "api" } });
    fireEvent.keyDown(field("service"), { key: "Tab" });
    fireEvent.change(field("environment"), { target: { value: "Staging" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Send message" }));
    });

    await waitFor(() => expect(toastSnapshot().at(-1)?.title).toBe("That command didn't run"));
    expect(toastSnapshot().at(-1)?.description).toBe("Offline");
    expect(field("service")).toHaveProperty("value", "api");
  });

  it("turns back into plain text on Escape, and opens again from it on Enter", async () => {
    const run = vi.spyOn(composerActions, "runSlashCommand");

    await openDeploy();
    fireEvent.change(field("service"), { target: { value: "my api" } });
    fireEvent.keyDown(field("service"), { key: "Escape" });

    await waitFor(() => expect(textbox()).toHaveProperty("value", '/deploy run service:"my api" '));
    expect(document.activeElement).toBe(textbox());

    fireEvent.keyDown(textbox(), { key: "Enter" });
    await waitFor(() => expect(field("service")).toHaveProperty("value", "my api"));
    expect(run).not.toHaveBeenCalled();
  });

  it("goes back to plain text on Backspace before the first field", async () => {
    render(<Composer roomId={ROOM} placeholder="Message" />);
    fireEvent.focus(textbox());
    fireEvent.change(textbox(), { target: { value: "/dep" } });
    await screen.findByRole("option", { name: /\/deploy/ });
    fireEvent.keyDown(textbox(), { key: "Tab" });

    const subcommand = await screen.findByRole("combobox", { name: "subcommand" });

    fireEvent.keyDown(subcommand, { key: "Backspace" });
    await waitFor(() => expect(textbox()).toHaveProperty("value", "/deploy "));
  });

  it("runs a typed command written out in full without opening the fields", async () => {
    const run = vi
      .spyOn(composerActions, "runSlashCommand")
      .mockResolvedValue({ status: "ephemeral", message: "Sent to Ember" });

    render(<Composer roomId={ROOM} placeholder="Message" />);
    fireEvent.change(textbox(), {
      target: { value: "/deploy run service:api environment:Staging" },
    });
    fireEvent.keyDown(textbox(), { key: "Enter" });

    await waitFor(() =>
      expect(run).toHaveBeenCalledWith(ROOM, "/deploy run service:api environment:staging", null, {
        subcommand: "run",
        values: { service: "api", environment: "staging" },
      }),
    );
  });

  it("leaves commands without a schema as free text", async () => {
    const run = vi
      .spyOn(composerActions, "runSlashCommand")
      .mockResolvedValue({ status: "ephemeral", message: "Set" });

    render(<Composer roomId={ROOM} placeholder="Message" />);
    fireEvent.focus(textbox());
    fireEvent.change(textbox(), { target: { value: "/rem" } });
    await screen.findByRole("option", { name: /\/remind/ });
    fireEvent.keyDown(textbox(), { key: "Enter" });
    expect(textbox()).toHaveProperty("value", "/remind ");
    expect(screen.queryByRole("group")).toBeNull();

    fireEvent.change(textbox(), { target: { value: "/remind in 5m stretch" } });
    fireEvent.keyDown(textbox(), { key: "Enter" });
    await waitFor(() => expect(run).toHaveBeenCalledWith(ROOM, "/remind in 5m stretch", null));
  });
});

/** Fills /deploy's required fields from `openDeploy`: service `api`, environment Production. */
async function fillDeploy(): Promise<void> {
  fireEvent.change(field("service"), { target: { value: "api" } });
  fireEvent.keyDown(field("service"), { key: "Tab" });
  await waitFor(() => expect(document.activeElement).toBe(field("environment")));
  fireEvent.keyDown(field("environment"), { key: "ArrowDown" });
  fireEvent.keyDown(field("environment"), { key: "Enter" });
}

/** A promise settled from outside, to hold a lookup in flight. */
function deferred<T>() {
  let settle: (value: T) => void = () => undefined;

  const promise = new Promise<T>((resolve) => {
    settle = resolve;
  });

  return { promise, settle };
}

describe("a typed command's fields across time", () => {
  it("keeps the fields as the conversation's draft, and opens them again without the focus", async () => {
    await openDeploy();
    await fillDeploy();
    fireEvent.click(screen.getByRole("button", { name: "Add replicas" }));
    fireEvent.change(field("replicas"), { target: { value: "3" } });

    const key = draftKey(ROOM, null);

    await waitFor(() =>
      expect(readDraft(key)).toBe("/deploy run service:api environment:production replicas:3"),
    );

    // Away and back (or a reload): the same fields, with what they held.
    cleanup();
    render(<Composer roomId={ROOM} placeholder="Message" />);

    await waitFor(() => expect(field("service")).toHaveProperty("value", "api"));
    expect(field("environment")).toHaveProperty("value", "Production");
    expect(field("replicas")).toHaveProperty("value", "3");
    expect(screen.queryByRole("textbox", { name: "Message" })).toBeNull();
    expect(document.activeElement).toBe(document.body);

    // Escape: the draft is text again.
    fireEvent.keyDown(field("service"), { key: "Escape" });
    await waitFor(() =>
      expect(textbox()).toHaveProperty(
        "value",
        "/deploy run service:api environment:production replicas:3 ",
      ),
    );
    cleanup();
    render(<Composer roomId={ROOM} placeholder="Message" />);
    expect(textbox()).toHaveProperty(
      "value",
      "/deploy run service:api environment:production replicas:3 ",
    );
  });

  it("keeps a thread's fields apart from the room's", async () => {
    render(<Composer roomId={ROOM} threadId={88} placeholder="Message" />);
    fireEvent.change(textbox(), { target: { value: "/deploy run service:web" } });
    fireEvent.keyDown(textbox(), { key: "Enter" });
    await waitFor(() => expect(field("service")).toHaveProperty("value", "web"));
    await waitFor(() => expect(readDraft(draftKey(ROOM, 88))).toBe("/deploy run service:web"));
    expect(readDraft(draftKey(ROOM, null))).toBe("");

    cleanup();
    render(<Composer roomId={ROOM} threadId={88} placeholder="Message" />);
    await waitFor(() => expect(field("service")).toHaveProperty("value", "web"));
  });

  it("doesn't cover newer text with fields from a lookup that was still loading", async () => {
    const lookup = deferred<readonly SlashCommand[]>();

    vi.spyOn(composerActions, "slashCommands").mockReturnValue(lookup.promise);

    const run = vi.spyOn(composerActions, "runSlashCommand");

    render(<Composer roomId={ROOM} placeholder="Message" />);
    fireEvent.change(textbox(), { target: { value: "/deploy run" } });
    fireEvent.keyDown(textbox(), { key: "Enter" });
    fireEvent.change(textbox(), { target: { value: "Something else entirely" } });

    await act(async () => {
      lookup.settle([REMIND, DEPLOY]);
      await lookup.promise;
    });

    expect(screen.queryByRole("group", { name: "Options for /deploy" })).toBeNull();
    expect(textbox()).toHaveProperty("value", "Something else entirely");
    expect(run).not.toHaveBeenCalled();
  });

  it("doesn't put refused fields back over newer text either", async () => {
    const lookup = deferred<readonly SlashCommand[]>();

    vi.spyOn(composerActions, "slashCommands").mockReturnValue(lookup.promise);
    vi.spyOn(composerActions, "runSlashCommand").mockRejectedValue(
      new ActionError("Validation", "Invalid slash command options", {
        "options.values.service": ["length is out of range"],
      }),
    );

    render(<Composer roomId={ROOM} placeholder="Message" />);
    fireEvent.change(textbox(), {
      target: { value: "/deploy run service:api environment:staging" },
    });
    fireEvent.keyDown(textbox(), { key: "Enter" });
    fireEvent.change(textbox(), { target: { value: "Something else entirely" } });

    await act(async () => {
      lookup.settle([REMIND, DEPLOY]);
      await lookup.promise;
    });

    await waitFor(() => expect(toastSnapshot().at(-1)?.title).toBe("That command didn't run"));
    expect(screen.queryByRole("group", { name: "Options for /deploy" })).toBeNull();
    expect(textbox()).toHaveProperty("value", "Something else entirely");
  });

  it("refuses a typed line's misspelt option until it's removed", async () => {
    const run = vi
      .spyOn(composerActions, "runSlashCommand")
      .mockResolvedValue({ status: "ephemeral", message: "Sent to Ember" });

    render(<Composer roomId={ROOM} placeholder="Message" />);
    fireEvent.change(textbox(), {
      target: { value: "/deploy run service:api environment:staging replica:10" },
    });
    fireEvent.keyDown(textbox(), { key: "Enter" });

    const alert = await screen.findByRole("alert");

    expect(alert.textContent).toContain("Not an option here: replica:10.");
    expect(run).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Send message" })).toHaveProperty("disabled", true);

    fireEvent.click(screen.getByRole("button", { name: "Remove replica:10" }));
    expect(screen.queryByRole("alert")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Send message" }));
    await waitFor(() =>
      expect(run).toHaveBeenCalledWith(ROOM, "/deploy run service:api environment:staging", null, {
        subcommand: "run",
        values: { service: "api", environment: "staging" },
      }),
    );
  });

  it("reads a changed schema again after a refusal, and moves the fields onto it", async () => {
    const regional: SlashCommand = {
      ...DEPLOY,
      optionSchema: {
        subcommands: [
          {
            name: "run",
            description: "Start a deploy",
            options: [
              {
                name: "service",
                description: "The service to deploy",
                type: "string",
                required: true,
              },
              {
                name: "region",
                description: "Where it runs",
                type: "string",
                required: true,
              },
            ],
          },
        ],
      },
    };

    const lists = vi
      .spyOn(composerActions, "slashCommands")
      .mockResolvedValueOnce([REMIND, DEPLOY])
      .mockResolvedValue([REMIND, regional]);

    vi.spyOn(composerActions, "runSlashCommand").mockRejectedValue(
      new ActionError("Validation", "Invalid slash command options", {
        "options.values.region": ["is required"],
        "options.values.environment": ["is unknown"],
      }),
    );

    await openDeploy();
    await fillDeploy();
    fireEvent.click(screen.getByRole("button", { name: "Send message" }));

    // The new schema's field takes its error; the old option is left over, on the alert.
    await waitFor(() => expect(field("region").getAttribute("aria-invalid")).toBe("true"));
    expect(screen.getByText("Is required.")).toBeTruthy();
    expect(field("service")).toHaveProperty("value", "api");
    expect(screen.getByRole("alert").textContent).toContain(
      "Not an option here: environment:production.",
    );
    expect(lists).toHaveBeenCalledTimes(2);
  });
});
