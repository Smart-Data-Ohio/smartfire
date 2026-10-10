import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { initialState } from "../../../store/state.ts";
import { store } from "../../../store/store.ts";
import { actions } from "../../../sync/runtime.ts";
import { toastSnapshot } from "../../../ui/toast-store.ts";
import { fileOptions, MAX_FILES, type TrayFile, useAttachments } from "./use-attachments.ts";

/** An XHR whose PUT always lands. */
class LandingXhr {
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
}

const file = (name: string, type = "text/plain") => new File(["bytes"], name, { type });

beforeEach(() => {
  vi.stubGlobal("XMLHttpRequest", LandingXhr);
  vi.spyOn(actions.messages, "startUpload").mockImplementation((body) =>
    Promise.resolve({ signedId: `signed-${body.filename}`, uploadUrl: "/put" }),
  );
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  store.setState(initialState, true);
});

describe("the attachment tray", () => {
  it("uploads each added file on its own and is ready once all are done", async () => {
    const { result } = renderHook(() => useAttachments(MAX_FILES));

    act(() => {
      result.current.add([file("a.txt"), file("b.txt"), file("c.txt")]);
    });

    expect(result.current.files.map((entry) => entry.file.name)).toEqual([
      "a.txt",
      "b.txt",
      "c.txt",
    ]);
    expect(result.current.ready).toBe(false);

    await waitFor(() => expect(result.current.ready).toBe(true));
    expect(result.current.files.map((entry) => entry.snapshot.signedId)).toEqual([
      "signed-a.txt",
      "signed-b.txt",
      "signed-c.txt",
    ]);
  });

  it("removes one file and keeps the rest", async () => {
    const { result } = renderHook(() => useAttachments(MAX_FILES));

    act(() => {
      result.current.add([file("keep.txt"), file("drop.txt")]);
    });

    const dropped = result.current.files[1]?.id ?? "";

    act(() => result.current.remove(dropped));

    expect(result.current.files.map((entry) => entry.file.name)).toEqual(["keep.txt"]);
    await waitFor(() => expect(result.current.ready).toBe(true));
  });

  it("marks a failed upload, blocks readiness, and retries it", async () => {
    const start = vi
      .spyOn(actions.messages, "startUpload")
      .mockRejectedValueOnce(new Error("Network down"))
      .mockResolvedValue({ signedId: "signed-again", uploadUrl: "/put" });

    const { result } = renderHook(() => useAttachments(MAX_FILES));

    act(() => {
      result.current.add([file("flaky.txt")]);
    });

    await waitFor(() => expect(result.current.failed).toBe(true));
    expect(result.current.ready).toBe(false);

    act(() => result.current.retry(result.current.files[0]?.id ?? ""));

    await waitFor(() => expect(result.current.ready).toBe(true));
    expect(result.current.failed).toBe(false);
    expect(result.current.files[0]?.snapshot.signedId).toBe("signed-again");
    expect(start).toHaveBeenCalledTimes(2);
  });

  it("takes at most the cap and reports how many didn't fit", () => {
    const { result } = renderHook(() => useAttachments(MAX_FILES));
    let overflow = 0;

    act(() => {
      overflow = result.current.add(
        Array.from({ length: MAX_FILES - 2 }, (_, index) => file(`first-${index}.txt`)),
      );
    });

    expect(overflow).toBe(0);

    act(() => {
      overflow = result.current.add([file("x.txt"), file("y.txt"), file("z.txt")]);
    });

    expect(overflow).toBe(1);
    expect(result.current.files).toHaveLength(MAX_FILES);
    expect(result.current.files.at(-1)?.file.name).toBe("y.txt");
  });

  it("refuses a file above the workspace limit and keeps the others", () => {
    store.setState({
      boot: {
        user: { id: 1, name: "David", avatarUrl: "/avatar" },
        account: {
          name: "Smartfire",
          logoUrl: null,
          logoStillUrl: null,
          bannerUrl: null,
          bannerStillUrl: null,
          uploadLimitBytes: 1024 * 1024,
        },
        customStyles: null,
        theme: "system",
        textSize: "default",
        cableUrl: "/cable",
        serviceWorkerUrl: null,
        version: "test",
        revision: null,
        appearancePreferences: null,
      },
    });

    const large = file("huge.bin");

    Object.defineProperty(large, "size", { value: 1024 * 1024 + 1 });

    const { result } = renderHook(() => useAttachments(MAX_FILES));

    act(() => {
      result.current.add([file("small.txt"), large]);
    });

    expect(result.current.files.map((entry) => entry.file.name)).toEqual(["small.txt"]);
    expect(toastSnapshot().at(-1)).toMatchObject({
      title: "File too large",
      description: '"huge.bin" exceeds the 1 MB upload limit.',
    });
  });
});

describe("a send's file options", () => {
  const done = (name: string, type = "text/plain"): TrayFile => ({
    id: name,
    file: file(name, type),
    previewUrl: type.startsWith("image/") ? `blob:${name}` : null,
    snapshot: { phase: "done", loaded: 5, total: 5, signedId: `signed-${name}`, error: null },
  });

  it("keeps a single file in the legacy slot", () => {
    expect(fileOptions([done("solo.pdf", "application/pdf")])).toEqual({
      attachmentSignedId: "signed-solo.pdf",
      attachment: {
        filename: "solo.pdf",
        contentType: "application/pdf",
        byteSize: 5,
        previewUrl: null,
      },
    });
    expect(fileOptions([])).toEqual({ attachmentSignedId: null, attachment: null });
  });

  it("groups several files as attachmentSignedIds, in tray order", () => {
    const options = fileOptions([done("a.png", "image/png"), done("b.txt"), done("c.txt")]);

    expect(options.attachmentSignedId).toBeNull();
    expect(options.attachmentSignedIds).toEqual(["signed-a.png", "signed-b.txt", "signed-c.txt"]);
    expect(options.attachments?.map((entry) => entry.filename)).toEqual([
      "a.png",
      "b.txt",
      "c.txt",
    ]);
    expect(options.attachments?.[0]?.previewUrl).toBe("blob:a.png");
  });
});
