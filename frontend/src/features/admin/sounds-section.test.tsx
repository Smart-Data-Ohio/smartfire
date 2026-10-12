import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SoundboardCatalog } from "../../gen/SoundboardCatalog.ts";
import type { SoundboardSound } from "../../gen/SoundboardSound.ts";
import type { Workspace } from "../../gen/Workspace.ts";
import { soundboard } from "../../sync/huddles.ts";
import { ActionError } from "../../sync/run.ts";
import { resetSoundboard, soundboardStore } from "../huddle/soundboard-store.ts";
import {
  SOUND_FILE_TYPES,
  soundCapacity,
  soundFieldError,
  soundVolumeLabel,
} from "./admin-format.ts";
import { AdminContext } from "./admin-parts.tsx";
import { SoundsSection } from "./sounds-section.tsx";

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
  stageAudienceBudget: 300,
  version: "test",
  description: "",
  vanitySlug: null,
  vanityUrl: null,
};

function sound(id: number, name: string, emoji: string | null, volume = 1): SoundboardSound {
  return {
    id,
    name,
    emoji,
    volume,
    uploaderId: 1,
    audioUrl: `/blobs/${name.toLowerCase()}.wav`,
    createdAt: "2026-10-11T12:00:00Z",
    updatedAt: "2026-10-11T12:00:00Z",
  };
}

const RIMSHOT = sound(2, "Rimshot", null, 0.6);

const CATALOG: SoundboardCatalog = {
  sounds: [sound(1, "Airhorn", "📯"), RIMSHOT],
  capacity: 48,
};

/** Every clip the section started, and whether it is still playing. */
const clips: { src: string; volume: number; paused: boolean }[] = [];

class FakeAudio extends EventTarget {
  readonly record: { src: string; volume: number; paused: boolean };

  constructor(src: string) {
    super();
    this.record = { src, volume: 1, paused: true };
    clips.push(this.record);
  }

  set volume(value: number) {
    this.record.volume = value;
  }

  play(): Promise<void> {
    this.record.paused = false;

    return Promise.resolve();
  }

  pause(): void {
    this.record.paused = true;
  }
}

beforeEach(() => {
  clips.length = 0;
  resetSoundboard();
  vi.stubGlobal("Audio", FakeAudio);
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function renderSection(canAdminister = true) {
  return render(
    <AdminContext value={{ workspace: { ...workspace, canAdminister }, replace: () => {} }}>
      <SoundsSection />
    </AdminContext>,
  );
}

function row(name: string): HTMLElement {
  const item = screen.getByText(name).closest("li");

  if (item === null) throw new Error(`No row for ${name}`);

  return item;
}

function submit(name: string) {
  fireEvent.change(screen.getByRole("textbox", { name: "Name" }), { target: { value: name } });
  fireEvent.click(screen.getByRole("button", { name: "Add sound" }));
}

describe("the soundboard section", () => {
  it("lists each sound with its emoji and volume, and counts them against the limit", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue(CATALOG);
    renderSection();

    expect(await screen.findByText("Sounds: 2 of 48 used.")).toBeTruthy();
    expect(within(row("Airhorn")).getByText("📯")).toBeTruthy();
    expect(within(row("Rimshot")).getByText("🔊")).toBeTruthy();
    expect(within(row("Rimshot")).getByText("Volume 60%")).toBeTruthy();
    expect(screen.getByLabelText("Audio file").getAttribute("accept")).toBe(SOUND_FILE_TYPES);
  });

  it("says when there are no sounds yet", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue({ sounds: [], capacity: 48 });
    renderSection();

    expect(await screen.findByText("No sounds yet. Add one above.")).toBeTruthy();
  });

  it("is for administrators only", () => {
    const catalog = vi.spyOn(soundboard, "catalog").mockResolvedValue(CATALOG);

    renderSection(false);

    expect(screen.getByText("Only administrators can see this page.")).toBeTruthy();
    expect(catalog).not.toHaveBeenCalled();
  });

  it("shows a full soundboard plainly and stops taking uploads", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue({ ...CATALOG, capacity: 2 });
    renderSection();

    expect(
      await screen.findByText(
        "Sounds: 2 of 2 used. The soundboard is full: delete a sound to add another.",
      ),
    ).toBeTruthy();
    expect(
      screen
        .getByText("The soundboard is full. Delete a sound to add another.")
        .getAttribute("role"),
    ).toBe("status");
    expect(screen.getByRole("button", { name: "Add sound" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByLabelText("Audio file").hasAttribute("disabled")).toBe(true);
  });

  it("shows the server's refusals under their fields, and the limit above the form", async () => {
    const catalog = vi.spyOn(soundboard, "catalog").mockResolvedValue(CATALOG);

    vi.spyOn(soundboard, "create").mockRejectedValue(
      new ActionError("Validation", "Validation failed", {
        name: ["has already been taken"],
        audio: ["isn't an uploaded file"],
        base: ["soundboard capacity reached (limit: 48 sounds)"],
      }),
    );
    renderSection();
    await screen.findByText("Sounds: 2 of 48 used.");
    submit("Airhorn");

    await screen.findByText("Name has already been taken.");
    expect(screen.getByText("Choose an audio file to upload.")).toBeTruthy();
    expect(
      screen.getByText(
        "The soundboard already has 48 sounds, its limit. Delete one to add another.",
      ),
    ).toBeTruthy();
    // A refusal refetches the catalog: another administrator may have filled it.
    await waitFor(() => expect(catalog).toHaveBeenCalledTimes(2));
  });

  it("previews a sound at its volume, and stops it when pressed again", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue(CATALOG);
    renderSection();
    await screen.findByText("Rimshot");

    fireEvent.click(screen.getByRole("button", { name: "Preview Rimshot" }));
    expect(clips).toEqual([{ src: "/blobs/rimshot.wav", volume: 0.6, paused: false }]);

    fireEvent.click(screen.getByRole("button", { name: "Stop Rimshot" }));
    expect(clips[0]?.paused).toBe(true);
    expect(screen.getByRole("button", { name: "Preview Rimshot" })).toBeTruthy();
  });

  it("edits a sound's name, emoji and volume, and every picker sees the new catalog", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue(CATALOG);

    const renamed = { ...CATALOG, sounds: [sound(1, "Foghorn", "🚢", 0.5), RIMSHOT] };
    const update = vi.spyOn(soundboard, "update").mockResolvedValue(renamed);

    renderSection();
    await screen.findByText("Airhorn");
    fireEvent.click(screen.getByRole("button", { name: "Edit Airhorn" }));

    const dialog = await screen.findByRole("dialog");

    fireEvent.change(within(dialog).getByRole("textbox", { name: "Name" }), {
      target: { value: "Foghorn" },
    });
    fireEvent.change(within(dialog).getByRole("textbox", { name: /Emoji/ }), {
      target: { value: "🚢" },
    });
    fireEvent.change(within(dialog).getByRole("slider"), { target: { value: "50" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));

    await screen.findByText("Foghorn");
    expect(update).toHaveBeenCalledExactlyOnceWith(1, {
      name: "Foghorn",
      emoji: "🚢",
      volume: 0.5,
    });

    const held = soundboardStore.getState().load;

    expect(held.status === "ready" ? held.catalog.sounds[0]?.name : null).toBe("Foghorn");
  });

  it("deletes a sound after asking", async () => {
    vi.spyOn(soundboard, "catalog").mockResolvedValue(CATALOG);

    const destroy = vi
      .spyOn(soundboard, "destroy")
      .mockResolvedValue({ ...CATALOG, sounds: [RIMSHOT] });

    renderSection();
    await screen.findByText("Airhorn");
    fireEvent.click(screen.getByRole("button", { name: "Delete Airhorn" }));

    const confirm = await screen.findByRole("alertdialog");

    expect(within(confirm).getByText("Delete Airhorn?")).toBeTruthy();
    fireEvent.click(within(confirm).getByRole("button", { name: "Delete" }));

    await waitFor(() => expect(screen.queryByText("Airhorn")).toBeNull());
    expect(destroy).toHaveBeenCalledExactlyOnceWith(1);
  });
});

describe("soundboard words", () => {
  it("count sounds of the limit and say when it is full", () => {
    expect(soundCapacity(0, 48)).toBe("Sounds: 0 of 48 used.");
    expect(soundCapacity(48, 48)).toBe(
      "Sounds: 48 of 48 used. The soundboard is full: delete a sound to add another.",
    );
  });

  it("name the field in front of the server's message", () => {
    expect(soundFieldError("name", "must be between 1 and 32 characters")).toBe(
      "Name must be between 1 and 32 characters.",
    );
    expect(soundFieldError("audio", "must be between 1 byte and 512 KB")).toBe(
      "Audio file must be between 1 byte and 512 KB.",
    );
    expect(soundFieldError("emoji", "must be a single Unicode emoji")).toBe(
      "Emoji must be a single Unicode emoji.",
    );
    expect(soundFieldError("other", "went wrong")).toBe("went wrong");
    expect(soundFieldError("name", undefined)).toBeUndefined();
  });

  it("show the volume as a percentage", () => {
    expect(soundVolumeLabel(0.6)).toBe("60%");
    expect(soundVolumeLabel(2)).toBe("100%");
  });
});
