import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Workspace } from "../../gen/Workspace.ts";
import type { WorkspaceIconList } from "../../gen/WorkspaceIconList.ts";
import { admin } from "../../sync/admin.ts";
import { ActionError } from "../../sync/run.ts";
import { animatedCapacity, iconFileError } from "./admin-format.ts";
import { AdminContext } from "./admin-parts.tsx";
import { IconsSection } from "./icons-section.tsx";

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

const LIST: WorkspaceIconList = {
  icons: [
    {
      id: 1,
      name: "dance",
      title: "Dance",
      creatorName: "Riel",
      imageUrl: "/icons/dance",
      animated: true,
      stillUrl: "/icons/dance?still=1",
    },
    {
      id: 2,
      name: "ohio",
      title: "Ohio",
      creatorName: "Riel",
      imageUrl: "/icons/ohio",
      animated: false,
      stillUrl: "/icons/ohio",
    },
  ],
  animatedLimit: 250,
  animatedUsage: 1,
};

beforeEach(() => {
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
});

afterEach(() => {
  cleanup();
  delete document.documentElement.dataset.motion;
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function renderSection() {
  return render(
    <AdminContext value={{ workspace, replace: () => {} }}>
      <IconsSection />
    </AdminContext>,
  );
}

function row(name: string): HTMLElement {
  const label = screen.getByText(`:${name}:`);
  const item = label.closest("li");

  if (item === null) throw new Error(`No row for :${name}:`);

  return item;
}

/** Fills in the form and submits it; with no file chosen, the write goes out without one. */
function submit(name: string) {
  fireEvent.change(screen.getByRole("textbox", { name: "Name" }), { target: { value: name } });
  fireEvent.change(screen.getByRole("textbox", { name: "Title" }), { target: { value: "Dance" } });
  fireEvent.click(screen.getByRole("button", { name: "Upload icon" }));
}

describe("the workspace icons section", () => {
  it("takes animated GIF and WebP files", async () => {
    vi.spyOn(admin, "icons").mockResolvedValue(LIST);
    renderSection();

    const input = await screen.findByLabelText("Icon file");

    expect(input.getAttribute("accept")).toBe("image/svg+xml,image/png,image/gif,image/webp");
  });

  it("shows animated capacity as usage of limit, and which icons are animated", async () => {
    vi.spyOn(admin, "icons").mockResolvedValue(LIST);
    renderSection();

    expect(await screen.findByText("Animated icons: 1 of 250 used.")).toBeTruthy();
    expect(within(row("dance")).getByText("Animated")).toBeTruthy();
    expect(within(row("ohio")).queryByText("Animated")).toBeNull();
    expect(row("dance").querySelector("img")?.getAttribute("src")).toBe("/icons/dance");
  });

  it("shows an animated icon's first frame under reduced motion", async () => {
    document.documentElement.dataset.motion = "reduce";
    vi.spyOn(admin, "icons").mockResolvedValue(LIST);
    renderSection();

    await screen.findByText("Animated icons: 1 of 250 used.");

    expect(row("dance").querySelector("img")?.getAttribute("src")).toBe("/icons/dance?still=1");
    expect(row("ohio").querySelector("img")?.getAttribute("src")).toBe("/icons/ohio");
  });

  it("says when the limit is reached", async () => {
    vi.spyOn(admin, "icons").mockResolvedValue({ ...LIST, animatedLimit: 1 });
    renderSection();

    expect(
      await screen.findByText(
        "Animated icons: 1 of 1 used. The limit is reached; static icons are unlimited.",
      ),
    ).toBeTruthy();
  });

  it("shows a capacity refusal under the file, and refreshes the counts", async () => {
    const icons = vi
      .spyOn(admin, "icons")
      .mockResolvedValueOnce(LIST)
      .mockResolvedValueOnce({ ...LIST, animatedUsage: 250 });

    const refusal = "animated emoji capacity reached (limit: 250)";

    vi.spyOn(admin, "createIcon").mockRejectedValue(
      new ActionError("Validation", `Image ${refusal}`, { image: [refusal] }),
    );
    renderSection();
    await screen.findByText("Animated icons: 1 of 250 used.");
    submit("party");

    const alert = await screen.findByRole("alert");

    expect(alert.textContent).toBe(
      "The workspace already has 250 animated icons, its limit. Delete one, or upload a still SVG or PNG.",
    );
    await waitFor(() => expect(icons).toHaveBeenCalledTimes(2));
    expect(
      await screen.findByText(
        "Animated icons: 250 of 250 used. The limit is reached; static icons are unlimited.",
      ),
    ).toBeTruthy();
  });

  it("shows any other refusal of the file as the server words it", async () => {
    vi.spyOn(admin, "icons").mockResolvedValue(LIST);
    vi.spyOn(admin, "createIcon").mockRejectedValue(
      new ActionError("Validation", "Image must contain at most 100 frames", {
        image: ["must contain at most 100 frames"],
      }),
    );
    renderSection();
    await screen.findByText("Animated icons: 1 of 250 used.");
    submit("party");

    expect((await screen.findByRole("alert")).textContent).toBe("must contain at most 100 frames");
  });
});

describe("icon capacity words", () => {
  it("count usage of limit", () => {
    expect(animatedCapacity(0, 250)).toBe("Animated icons: 0 of 250 used.");
    expect(animatedCapacity(3, 3)).toBe(
      "Animated icons: 3 of 3 used. The limit is reached; static icons are unlimited.",
    );
  });

  it("say the capacity refusal plainly and pass anything else through", () => {
    expect(iconFileError("animated emoji capacity reached (limit: 5)")).toBe(
      "The workspace already has 5 animated icons, its limit. Delete one, or upload a still SVG or PNG.",
    );
    expect(iconFileError("must be smaller than 256 KB")).toBe("must be smaller than 256 KB");
    expect(iconFileError(undefined)).toBeUndefined();
  });
});
