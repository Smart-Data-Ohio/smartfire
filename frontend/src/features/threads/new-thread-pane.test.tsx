import {
  createMemoryHistory,
  createRootRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { NewThreadPane } from "./new-thread-pane.tsx";
import { messageFixture } from "./test-fixtures.ts";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
  sessionStorage.clear();
});

it("starts a thread with all files from its grouped composer", async () => {
  store.setState({ messages: { 1: messageFixture(1) } });
  const create = vi.spyOn(actions.threads, "create").mockRejectedValue(new Error("Keep the draft"));
  vi.spyOn(actions, "noteActivity").mockImplementation(() => undefined);
  vi.spyOn(actions, "setTyping").mockImplementation(() => undefined);
  vi.spyOn(actions.messages, "startUpload").mockImplementation(async (body) => ({
    signedId: `signed-${body.filename}`,
    uploadUrl: "/put",
  }));
  vi.stubGlobal("matchMedia", (query: string) =>
    Object.assign(new EventTarget(), { matches: true, media: query, onchange: null }),
  );
  vi.stubGlobal(
    "XMLHttpRequest",
    class {
      status = 204;
      upload = { onprogress: null };
      onload: (() => void) | null = null;
      onerror = null;
      onabort = null;
      open() {}
      setRequestHeader() {}
      abort() {}
      send() {
        queueMicrotask(() => this.onload?.());
      }
    },
  );
  const root = createRootRoute({ component: () => <NewThreadPane roomId={4} parentId={1} /> });

  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  const { container } = render(<RouterProvider router={router} />);
  await act(() => router.load());
  const picker = container.querySelector<HTMLInputElement>('input[type="file"]');

  if (picker === null) throw new Error("no file input");
  fireEvent.change(picker, {
    target: {
      files: [
        new File(["one"], "one.txt"),
        new File(["two"], "two.txt"),
        new File(["three"], "three.txt"),
      ],
    },
  });
  await waitFor(() => expect(container.querySelectorAll('[data-phase="done"]')).toHaveLength(3));
  fireEvent.keyDown(screen.getByRole("textbox", { name: "Reply…" }), { key: "Enter" });
  await waitFor(() =>
    expect(create).toHaveBeenCalledWith(
      4,
      1,
      "",
      expect.objectContaining({
        attachmentSignedId: null,
        attachmentSignedIds: ["signed-one.txt", "signed-two.txt", "signed-three.txt"],
      }),
    ),
  );
});

it("holds file adds while the new thread is being created, then adds again after a failure", async () => {
  store.setState({ messages: { 1: messageFixture(1) } });

  let fail: (error: Error) => void = () => undefined;

  const create = vi.spyOn(actions.threads, "create").mockReturnValueOnce(
    new Promise((_, reject) => {
      fail = reject;
    }),
  );

  vi.spyOn(actions, "noteActivity").mockImplementation(() => undefined);
  vi.spyOn(actions, "setTyping").mockImplementation(() => undefined);
  vi.spyOn(actions.messages, "startUpload").mockImplementation(async (body) => ({
    signedId: `signed-${body.filename}`,
    uploadUrl: "/put",
  }));
  vi.stubGlobal("matchMedia", (query: string) =>
    Object.assign(new EventTarget(), { matches: true, media: query, onchange: null }),
  );
  vi.stubGlobal(
    "XMLHttpRequest",
    class {
      status = 204;
      upload = { onprogress: null };
      onload: (() => void) | null = null;
      onerror = null;
      onabort = null;
      open() {}
      setRequestHeader() {}
      abort() {}
      send() {
        queueMicrotask(() => this.onload?.());
      }
    },
  );

  const root = createRootRoute({ component: () => <NewThreadPane roomId={4} parentId={1} /> });

  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });

  const { container } = render(<RouterProvider router={router} />);
  await act(() => router.load());
  const picker = container.querySelector<HTMLInputElement>('input[type="file"]');

  if (picker === null) throw new Error("no file input");

  fireEvent.change(screen.getByRole("textbox", { name: "Reply…" }), {
    target: { value: "Lunch plans" },
  });
  fireEvent.keyDown(screen.getByRole("textbox", { name: "Reply…" }), { key: "Enter" });
  await waitFor(() => expect(create).toHaveBeenCalledTimes(1));

  fireEvent.click(screen.getByRole("button", { name: "Attach and more" }));
  const upload = await screen.findByRole("menuitem", { name: "Upload a file" });
  expect(upload.getAttribute("aria-disabled")).toBe("true");
  fireEvent.change(picker, { target: { files: [new File(["late"], "late.txt")] } });
  expect(screen.queryByText("late.txt")).toBeNull();
  expect(container.querySelector('[data-phase="uploading"], [data-phase="done"]')).toBeNull();

  await act(async () => fail(new Error("Keep the draft")));
  await screen.findByDisplayValue("Lunch plans");

  fireEvent.change(picker, { target: { files: [new File(["again"], "again.txt")] } });
  await screen.findByText("again.txt");
});
