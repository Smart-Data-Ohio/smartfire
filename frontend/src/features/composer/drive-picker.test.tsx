import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { DriveShare } from "../../gen/DriveShare.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { DRIVE_SEARCH_DEBOUNCE_MS } from "./drive-picker.ts";
import { DrivePicker } from "./drive-picker.tsx";

const file = {
  id: "1RoadmapQ4draft",
  name: "Q4 roadmap",
  kind: "document",
  modifiedAt: null,
  owner: "Maya Okafor",
  url: "https://docs.google.com/document/d/1RoadmapQ4draft/edit",
};

const maya = { id: 2, name: "Maya Okafor", email: "maya@37signals.com" };

const jonah = { id: 3, name: "Jonah Lindqvist", email: "jonah@37signals.com" };

const partial: DriveShare = {
  outcome: "shared",
  fileId: file.id,
  blocked: null,
  changedIds: [],
  recipients: [],
  results: [
    { recipient: maya, status: "granted", reason: null },
    { recipient: jonah, status: "failed", reason: "denied" },
  ],
};

beforeEach(() => {
  vi.spyOn(actions.drive, "preparePicker").mockResolvedValue({
    choose: async () => null,
    dispose: () => {},
  });
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();

  for (const record of [...toastSnapshot()]) {
    removeToast(record.id);
  }
});

function picker(open: boolean) {
  return (
    <DrivePicker
      roomId={1}
      attachedFileIds={[]}
      open={open}
      onOpenChange={() => undefined}
      onAttach={() => undefined}
    />
  );
}

describe("drive search debounce", () => {
  it("cancels the timer when the query changes or the picker closes", async () => {
    vi.useFakeTimers();
    const search = vi.spyOn(actions.drive, "search").mockResolvedValue({ files: [] });
    const view = render(picker(true));

    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(search).toHaveBeenCalledTimes(1);
    expect(search).toHaveBeenCalledWith("");

    const input = screen.getByRole("combobox", { name: "Search Drive files" });

    fireEvent.change(input, { target: { value: "old" } });
    fireEvent.change(input, { target: { value: "new" } });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(DRIVE_SEARCH_DEBOUNCE_MS);
    });

    expect(search).not.toHaveBeenCalledWith("old");
    expect(search).toHaveBeenCalledWith("new");
    expect(search).toHaveBeenCalledTimes(2);

    fireEvent.change(input, { target: { value: "stale" } });
    view.rerender(picker(false));

    await act(async () => {
      await vi.advanceTimersByTimeAsync(DRIVE_SEARCH_DEBOUNCE_MS);
    });

    expect(search).not.toHaveBeenCalledWith("stale");
    expect(search).toHaveBeenCalledTimes(2);
  });
});

describe("a grant in flight", () => {
  it("cannot be dismissed, and a late result is toasted once the picker is gone", async () => {
    vi.spyOn(actions.drive, "search").mockResolvedValue({ files: [file] });
    vi.spyOn(actions.drive, "recipients").mockResolvedValue({ recipients: [maya, jonah] });

    let finish: (share: DriveShare) => void = () => undefined;

    vi.spyOn(actions.drive, "share").mockImplementation(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );

    Element.prototype.scrollIntoView = () => undefined;

    const attached = vi.fn();
    const user = userEvent.setup();

    const view = render(
      <DrivePicker
        roomId={1}
        attachedFileIds={[]}
        open
        onOpenChange={() => undefined}
        onAttach={attached}
      />,
    );

    await user.click(await screen.findByRole("option", { name: /Q4 roadmap/ }));
    await user.click(await screen.findByRole("checkbox", { name: /Maya Okafor/ }));
    await user.click(screen.getByRole("button", { name: "Grant view access and attach" }));

    const dialog = screen.getByRole("alertdialog", { name: "Share a Drive file" });
    const grant = screen.getByRole("button", { name: "Grant view access and attach" });

    // SAFETY: Cancel is a <button>, so `disabled` is the attribute.
    const cancel = screen.getByRole("button", { name: "Cancel" }) as HTMLButtonElement;

    // SAFETY: Attach only is a <button>, so `disabled` is the attribute.
    const attachOnly = screen.getByRole("button", { name: "Attach only" }) as HTMLButtonElement;

    expect(cancel.disabled).toBe(true);
    expect(attachOnly.disabled).toBe(true);
    expect(screen.queryByRole("button", { name: "Close" })).toBeNull();
    expect(grant.getAttribute("aria-busy")).toBe("true");

    fireEvent.keyDown(dialog, { key: "Escape" });
    fireEvent.click(dialog);

    expect(screen.getByRole("alertdialog", { name: "Share a Drive file" })).toBeTruthy();
    expect(toastSnapshot()).toEqual([]);

    view.unmount();

    await act(async () => {
      finish(partial);
      await Promise.resolve();
    });

    expect(attached).toHaveBeenCalledWith(
      expect.objectContaining({ id: file.id, name: "Q4 roadmap" }),
    );

    await waitFor(() => {
      expect(toastSnapshot()).toEqual([
        expect.objectContaining({
          title: "1 of 2 recipients have access.",
          description:
            "Maya Okafor: granted view access; Jonah Lindqvist: not granted (refused by Google)",
        }),
      ]);
    });
  });
});

describe("Google Picker", () => {
  it("reviews a file that the stored grant's search cannot find", async () => {
    vi.spyOn(actions.drive, "search").mockResolvedValue({ files: [] });
    vi.spyOn(actions.drive, "recipients").mockResolvedValue({ recipients: [maya] });

    const choose = vi.fn(async () => ({
      id: file.id,
      name: file.name,
      kind: file.kind,
      url: file.url,
    }));

    const dispose = vi.fn();
    vi.mocked(actions.drive.preparePicker).mockResolvedValue({ choose, dispose });
    const attached = vi.fn();
    const user = userEvent.setup();

    const view = render(
      <DrivePicker
        roomId={1}
        attachedFileIds={[]}
        open
        onOpenChange={() => {}}
        onAttach={attached}
      />,
    );

    await user.click(await screen.findByRole("button", { name: "Choose from Google Drive" }));
    await screen.findByRole("dialog", { name: "Share a Drive file" });
    expect(choose).toHaveBeenCalledOnce();
    expect(screen.queryByRole("option")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Attach only" }));
    expect(attached).toHaveBeenCalledExactlyOnceWith({
      id: file.id,
      name: file.name,
      kind: file.kind,
      url: file.url,
    });
    view.unmount();
    expect(dispose).toHaveBeenCalledOnce();
  });

  it("keeps Enable Drive previews when Picker config returns the empty 404", async () => {
    vi.spyOn(actions.drive, "search").mockResolvedValue({ files: [] });
    vi.mocked(actions.drive.preparePicker).mockRejectedValue(new ActionError("NotFound", "404"));
    render(picker(true));
    expect(await screen.findByRole("button", { name: "Enable Drive previews" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Choose from Google Drive" })).toBeNull();
  });
});

describe("recipient loading", () => {
  it("shows a load failure and retries before enabling sharing", async () => {
    vi.spyOn(actions.drive, "search").mockResolvedValue({ files: [file] });
    const first = Promise.withResolvers<{ recipients: (typeof maya)[] }>();

    const load = vi
      .spyOn(actions.drive, "recipients")
      .mockReturnValueOnce(first.promise)
      .mockResolvedValue({ recipients: [maya] });

    const share = vi.spyOn(actions.drive, "share").mockResolvedValue({ ...partial, results: [] });
    const user = userEvent.setup();
    render(picker(true));
    await user.click(await screen.findByRole("option", { name: /Q4 roadmap/ }));
    expect(screen.getByText("Loading recipients…")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Grant view access and attach" })).toHaveProperty(
      "disabled",
      true,
    );
    await act(async () => first.reject(new ActionError("NetworkError", "Connection lost")));
    expect(await screen.findByRole("alert")).toHaveProperty(
      "textContent",
      expect.stringContaining("Couldn't load recipients: Connection lost"),
    );
    expect(share).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Retry" }));
    await user.click(await screen.findByRole("checkbox", { name: /Maya Okafor/ }));
    expect(load).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("button", { name: "Grant view access and attach" })).toHaveProperty(
      "disabled",
      false,
    );
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
